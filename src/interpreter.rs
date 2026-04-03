use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::ast::{BinaryOp, Expr, Stmt, UnaryOp};
use crate::environment::{EnvRef, Environment};

/// A reference-counted array for mutation semantics
pub type ArrayRef = Rc<RefCell<Vec<Value>>>;

/// Runtime values in Gello
#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    Str(String),
    Bool(bool),
    Null,
    Function {
        params: Vec<String>,
        body: Vec<Stmt>,
        closure: EnvRef,
    },
    Array(ArrayRef),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => {
                // Print whole numbers without decimals
                if n.fract() == 0.0 && n.is_finite() {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            Value::Str(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Null => write!(f, "null"),
            Value::Function { params, .. } => {
                write!(f, "<fn({})>", params.join(", "))
            }
            Value::Array(arr) => {
                let elements: Vec<String> = arr.borrow().iter().map(|v| v.to_string()).collect();
                write!(f, "[{}]", elements.join(", "))
            }
        }
    }
}

/// Result of executing a statement - allows early return propagation
enum StmtResult {
    None,
    Return(Value),
}

/// Tree-walking interpreter for Gello
pub struct Interpreter {
    env: EnvRef,
    output: Vec<String>,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            env: Environment::new(),
            output: Vec::new(),
        }
    }

    /// Run a list of statements
    pub fn run(&mut self, stmts: Vec<Stmt>) -> Result<(), String> {
        for stmt in stmts {
            self.execute(&stmt)?;
        }
        Ok(())
    }

    /// Clear the output buffer and return its contents
    pub fn take_output(&mut self) -> Vec<String> {
        std::mem::take(&mut self.output)
    }

    /// Get the output buffer contents without clearing
    pub fn get_output(&self) -> &[String] {
        &self.output
    }

    /// Clear the output buffer
    pub fn clear_output(&mut self) {
        self.output.clear();
    }

    /// Execute a single statement
    fn execute(&mut self, stmt: &Stmt) -> Result<StmtResult, String> {
        match stmt {
            Stmt::Let { name, value } => {
                let val = self.evaluate(value)?;
                self.env.borrow_mut().set(name, val);
                Ok(StmtResult::None)
            }

            Stmt::Print(expr) => {
                let val = self.evaluate(expr)?;
                self.output.push(format!("{}", val));
                Ok(StmtResult::None)
            }

            Stmt::Expression(expr) => {
                self.evaluate(expr)?;
                Ok(StmtResult::None)
            }

            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                let cond_val = self.evaluate(condition)?;
                if self.is_truthy(&cond_val) {
                    // Create child environment for then block
                    let child = Environment::new_child(Rc::clone(&self.env));
                    let old_env = std::mem::replace(&mut self.env, child);

                    let result = self.execute_block(then_block)?;

                    // Restore parent environment
                    self.env = old_env;

                    if let StmtResult::Return(_) = result {
                        return Ok(result);
                    }
                } else if let Some(else_stmts) = else_block {
                    // Create child environment for else block
                    let child = Environment::new_child(Rc::clone(&self.env));
                    let old_env = std::mem::replace(&mut self.env, child);

                    let result = self.execute_block(else_stmts)?;

                    // Restore parent environment
                    self.env = old_env;

                    if let StmtResult::Return(_) = result {
                        return Ok(result);
                    }
                }
                Ok(StmtResult::None)
            }

            Stmt::While { condition, body } => {
                loop {
                    let cond_val = self.evaluate(condition)?;
                    if !self.is_truthy(&cond_val) {
                        break;
                    }

                    // Create fresh child environment each iteration
                    let child = Environment::new_child(Rc::clone(&self.env));
                    let old_env = std::mem::replace(&mut self.env, child);

                    let result = self.execute_block(body)?;

                    // Restore parent environment
                    self.env = old_env;

                    if let StmtResult::Return(_) = result {
                        return Ok(result);
                    }
                }
                Ok(StmtResult::None)
            }

            Stmt::Fn { name, params, body } => {
                // Capture the current environment as the closure
                let func = Value::Function {
                    params: params.clone(),
                    body: body.clone(),
                    closure: Rc::clone(&self.env),
                };
                self.env.borrow_mut().set(name, func);
                Ok(StmtResult::None)
            }

            Stmt::Return(expr) => {
                let val = self.evaluate(expr)?;
                Ok(StmtResult::Return(val))
            }
        }
    }

    /// Execute a block of statements, returning early on Return
    fn execute_block(&mut self, stmts: &[Stmt]) -> Result<StmtResult, String> {
        for stmt in stmts {
            let result = self.execute(stmt)?;
            if let StmtResult::Return(_) = result {
                return Ok(result);
            }
        }
        Ok(StmtResult::None)
    }

    /// Evaluate an expression to a value
    fn evaluate(&mut self, expr: &Expr) -> Result<Value, String> {
        match expr {
            Expr::Number(n) => Ok(Value::Number(*n)),
            Expr::StringLit(s) => Ok(Value::Str(s.clone())),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Null => Ok(Value::Null),

            Expr::Identifier(name) => self
                .env
                .borrow()
                .get(name)
                .ok_or_else(|| format!("Undefined variable '{}'", name)),

            Expr::Unary { op, right } => {
                let val = self.evaluate(right)?;
                match op {
                    UnaryOp::Neg => match val {
                        Value::Number(n) => Ok(Value::Number(-n)),
                        _ => Err("Cannot negate non-number".to_string()),
                    },
                    UnaryOp::Not => Ok(Value::Bool(!self.is_truthy(&val))),
                }
            }

            Expr::Binary { left, op, right } => {
                let left_val = self.evaluate(left)?;
                let right_val = self.evaluate(right)?;
                self.eval_binary(op, left_val, right_val)
            }

            Expr::Assign { name, value } => {
                let val = self.evaluate(value)?;
                if self.env.borrow_mut().assign(name, val.clone()) {
                    Ok(val)
                } else {
                    Err(format!("Undefined variable '{}'", name))
                }
            }

            Expr::Lambda { params, body } => {
                // Capture the current environment as the closure
                Ok(Value::Function {
                    params: params.clone(),
                    body: body.clone(),
                    closure: Rc::clone(&self.env),
                })
            }

            Expr::Array(elements) => {
                let mut values = Vec::new();
                for elem in elements {
                    values.push(self.evaluate(elem)?);
                }
                Ok(Value::Array(Rc::new(RefCell::new(values))))
            }

            Expr::Index { array, index } => {
                let arr_val = self.evaluate(array)?;
                let idx_val = self.evaluate(index)?;

                match (&arr_val, &idx_val) {
                    (Value::Array(arr), Value::Number(n)) => {
                        let idx = *n as usize;
                        let borrowed = arr.borrow();
                        if idx < borrowed.len() {
                            Ok(borrowed[idx].clone())
                        } else {
                            Err(format!("Index {} out of bounds", idx))
                        }
                    }
                    (Value::Array(_), _) => Err("Array index must be a number".to_string()),
                    _ => Err("Cannot index non-array".to_string()),
                }
            }

            Expr::Call { callee, args } => {
                // Handle built-in functions first
                if let Expr::Identifier(name) = callee.as_ref() {
                    if name == "push" {
                        return self.builtin_push(args);
                    }
                }

                // Evaluate the callee expression to get the function value
                let func = self.evaluate(callee)?;

                match func {
                    Value::Function {
                        params,
                        body,
                        closure,
                    } => {
                        if args.len() != params.len() {
                            return Err(format!(
                                "Expected {} arguments but got {}",
                                params.len(),
                                args.len()
                            ));
                        }

                        // Evaluate arguments in the current environment
                        let mut arg_vals = Vec::new();
                        for arg in args {
                            arg_vals.push(self.evaluate(arg)?);
                        }

                        // Create child environment from the CLOSURE (lexical scoping)
                        let func_env = Environment::new_child(closure);

                        // Bind parameters to arguments
                        for (param, val) in params.iter().zip(arg_vals) {
                            func_env.borrow_mut().set(param, val);
                        }

                        // Save current environment and switch to function environment
                        let old_env = std::mem::replace(&mut self.env, func_env);

                        // Execute function body
                        let result = self.execute_block(&body);

                        // Restore caller's environment
                        self.env = old_env;

                        match result {
                            Ok(StmtResult::Return(val)) => Ok(val),
                            Ok(StmtResult::None) => Ok(Value::Null),
                            Err(e) => Err(e),
                        }
                    }
                    _ => Err("Cannot call non-function".to_string()),
                }
            }
        }
    }

    /// Built-in push function: push(array, value)
    fn builtin_push(&mut self, args: &[Expr]) -> Result<Value, String> {
        if args.len() != 2 {
            return Err(format!("push expects 2 arguments, got {}", args.len()));
        }

        let arr_val = self.evaluate(&args[0])?;
        let val = self.evaluate(&args[1])?;

        match arr_val {
            Value::Array(arr) => {
                arr.borrow_mut().push(val);
                Ok(Value::Null)
            }
            _ => Err("First argument to push must be an array".to_string()),
        }
    }

    /// Evaluate a binary operation
    fn eval_binary(&self, op: &BinaryOp, left: Value, right: Value) -> Result<Value, String> {
        match op {
            BinaryOp::Add => match (&left, &right) {
                (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
                (Value::Str(a), Value::Str(b)) => Ok(Value::Str(format!("{}{}", a, b))),
                _ => Err("Cannot add these types".to_string()),
            },

            BinaryOp::Sub => match (&left, &right) {
                (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a - b)),
                _ => Err("Cannot subtract non-numbers".to_string()),
            },

            BinaryOp::Mul => match (&left, &right) {
                (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a * b)),
                _ => Err("Cannot multiply non-numbers".to_string()),
            },

            BinaryOp::Div => match (&left, &right) {
                (Value::Number(a), Value::Number(b)) => {
                    if *b == 0.0 {
                        Err("Division by zero".to_string())
                    } else {
                        Ok(Value::Number(a / b))
                    }
                }
                _ => Err("Cannot divide non-numbers".to_string()),
            },

            BinaryOp::Mod => match (&left, &right) {
                (Value::Number(a), Value::Number(b)) => {
                    if *b == 0.0 {
                        Err("Modulo by zero".to_string())
                    } else {
                        Ok(Value::Number(a % b))
                    }
                }
                _ => Err("Cannot modulo non-numbers".to_string()),
            },

            BinaryOp::Eq => Ok(Value::Bool(self.values_equal(&left, &right))),
            BinaryOp::Ne => Ok(Value::Bool(!self.values_equal(&left, &right))),

            BinaryOp::Lt => match (&left, &right) {
                (Value::Number(a), Value::Number(b)) => Ok(Value::Bool(a < b)),
                _ => Err("Cannot compare non-numbers with '<'".to_string()),
            },

            BinaryOp::Le => match (&left, &right) {
                (Value::Number(a), Value::Number(b)) => Ok(Value::Bool(a <= b)),
                _ => Err("Cannot compare non-numbers with '<='".to_string()),
            },

            BinaryOp::Gt => match (&left, &right) {
                (Value::Number(a), Value::Number(b)) => Ok(Value::Bool(a > b)),
                _ => Err("Cannot compare non-numbers with '>'".to_string()),
            },

            BinaryOp::Ge => match (&left, &right) {
                (Value::Number(a), Value::Number(b)) => Ok(Value::Bool(a >= b)),
                _ => Err("Cannot compare non-numbers with '>='".to_string()),
            },

            BinaryOp::And => {
                Ok(Value::Bool(self.is_truthy(&left) && self.is_truthy(&right)))
            }

            BinaryOp::Or => {
                Ok(Value::Bool(self.is_truthy(&left) || self.is_truthy(&right)))
            }
        }
    }

    /// Check if a value is truthy
    fn is_truthy(&self, val: &Value) -> bool {
        match val {
            Value::Bool(b) => *b,
            Value::Null => false,
            Value::Number(n) => *n != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::Function { .. } => true,
            Value::Array(_) => true,
        }
    }

    /// Check if two values are equal
    fn values_equal(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Number(x), Value::Number(y)) => (x - y).abs() < f64::EPSILON,
            (Value::Str(x), Value::Str(y)) => x == y,
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::Null, Value::Null) => true,
            _ => false,
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}
