use crate::ast::{BinaryOp, Expr, Stmt, UnaryOp};
use crate::token::{Token, TokenKind};

/// Recursive descent parser for the Gello language
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    /// Parse the token stream into a list of statements
    pub fn parse(&mut self) -> Result<Vec<Stmt>, String> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            statements.push(self.declaration()?);
        }

        Ok(statements)
    }

    // --- Statement parsing ---

    fn declaration(&mut self) -> Result<Stmt, String> {
        if self.check(&TokenKind::Let) {
            self.advance();
            self.let_statement()
        } else if self.check(&TokenKind::Fn) {
            // Check if this is a named function statement or anonymous lambda
            if self.peek_next_is_identifier() {
                self.advance();
                self.fn_statement()
            } else {
                self.statement()
            }
        } else {
            self.statement()
        }
    }

    fn peek_next_is_identifier(&self) -> bool {
        if self.current + 1 < self.tokens.len() {
            matches!(self.tokens[self.current + 1].kind, TokenKind::Identifier(_))
        } else {
            false
        }
    }

    fn let_statement(&mut self) -> Result<Stmt, String> {
        let name = self.expect_identifier("Expected variable name after 'let'")?;
        self.expect(&TokenKind::Equal, "Expected '=' after variable name")?;
        let value = self.expression()?;
        Ok(Stmt::Let { name, value })
    }

    fn fn_statement(&mut self) -> Result<Stmt, String> {
        let name = self.expect_identifier("Expected function name after 'fn'")?;
        self.expect(&TokenKind::LeftParen, "Expected '(' after function name")?;

        let mut params = Vec::new();
        if !self.check(&TokenKind::RightParen) {
            loop {
                params.push(self.expect_identifier("Expected parameter name")?);
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
        }

        self.expect(&TokenKind::RightParen, "Expected ')' after parameters")?;
        self.expect(&TokenKind::LeftBrace, "Expected '{' before function body")?;
        let body = self.block()?;

        Ok(Stmt::Fn { name, params, body })
    }

    fn statement(&mut self) -> Result<Stmt, String> {
        if self.check(&TokenKind::Print) {
            self.advance();
            let expr = self.expression()?;
            Ok(Stmt::Print(expr))
        } else if self.check(&TokenKind::If) {
            self.advance();
            self.if_statement()
        } else if self.check(&TokenKind::While) {
            self.advance();
            self.while_statement()
        } else if self.check(&TokenKind::Return) {
            self.advance();
            let value = self.expression()?;
            Ok(Stmt::Return(value))
        } else {
            let expr = self.expression()?;
            Ok(Stmt::Expression(expr))
        }
    }

    fn if_statement(&mut self) -> Result<Stmt, String> {
        let condition = self.expression()?;
        self.expect(&TokenKind::LeftBrace, "Expected '{' after if condition")?;
        let then_block = self.block()?;

        let else_block = if self.check(&TokenKind::Else) {
            self.advance();
            self.expect(&TokenKind::LeftBrace, "Expected '{' after 'else'")?;
            Some(self.block()?)
        } else {
            None
        };

        Ok(Stmt::If {
            condition,
            then_block,
            else_block,
        })
    }

    fn while_statement(&mut self) -> Result<Stmt, String> {
        let condition = self.expression()?;
        self.expect(&TokenKind::LeftBrace, "Expected '{' before while body")?;
        let body = self.block()?;

        Ok(Stmt::While { condition, body })
    }

    fn block(&mut self) -> Result<Vec<Stmt>, String> {
        let mut statements = Vec::new();

        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            statements.push(self.declaration()?);
        }

        self.expect(&TokenKind::RightBrace, "Expected '}' after block")?;
        Ok(statements)
    }

    // --- Expression parsing (precedence climbing) ---

    fn expression(&mut self) -> Result<Expr, String> {
        self.assignment()
    }

    fn assignment(&mut self) -> Result<Expr, String> {
        let expr = self.or()?;

        if self.match_token(&TokenKind::Equal) {
            let value = self.assignment()?;

            if let Expr::Identifier(name) = expr {
                return Ok(Expr::Assign {
                    name,
                    value: Box::new(value),
                });
            }

            let token = self.previous();
            return Err(format!(
                "Invalid assignment target at line {}, column {}",
                token.line, token.col
            ));
        }

        Ok(expr)
    }

    fn or(&mut self) -> Result<Expr, String> {
        let mut expr = self.and()?;

        while self.match_token(&TokenKind::PipePipe) {
            let right = self.and()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn and(&mut self) -> Result<Expr, String> {
        let mut expr = self.equality()?;

        while self.match_token(&TokenKind::AmpAmp) {
            let right = self.equality()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn equality(&mut self) -> Result<Expr, String> {
        let mut expr = self.comparison()?;

        loop {
            if self.match_token(&TokenKind::EqualEqual) {
                let right = self.comparison()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::Eq,
                    right: Box::new(right),
                };
            } else if self.match_token(&TokenKind::BangEqual) {
                let right = self.comparison()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::Ne,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn comparison(&mut self) -> Result<Expr, String> {
        let mut expr = self.term()?;

        loop {
            if self.match_token(&TokenKind::Less) {
                let right = self.term()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::Lt,
                    right: Box::new(right),
                };
            } else if self.match_token(&TokenKind::LessEqual) {
                let right = self.term()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::Le,
                    right: Box::new(right),
                };
            } else if self.match_token(&TokenKind::Greater) {
                let right = self.term()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::Gt,
                    right: Box::new(right),
                };
            } else if self.match_token(&TokenKind::GreaterEqual) {
                let right = self.term()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::Ge,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn term(&mut self) -> Result<Expr, String> {
        let mut expr = self.factor()?;

        loop {
            if self.match_token(&TokenKind::Plus) {
                let right = self.factor()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::Add,
                    right: Box::new(right),
                };
            } else if self.match_token(&TokenKind::Minus) {
                let right = self.factor()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::Sub,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn factor(&mut self) -> Result<Expr, String> {
        let mut expr = self.unary()?;

        loop {
            if self.match_token(&TokenKind::Star) {
                let right = self.unary()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::Mul,
                    right: Box::new(right),
                };
            } else if self.match_token(&TokenKind::Slash) {
                let right = self.unary()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::Div,
                    right: Box::new(right),
                };
            } else if self.match_token(&TokenKind::Percent) {
                let right = self.unary()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::Mod,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        if self.match_token(&TokenKind::Bang) {
            let right = self.unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                right: Box::new(right),
            });
        }

        if self.match_token(&TokenKind::Minus) {
            let right = self.unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                right: Box::new(right),
            });
        }

        self.call()
    }

    fn call(&mut self) -> Result<Expr, String> {
        let mut expr = self.primary()?;

        loop {
            if self.match_token(&TokenKind::LeftParen) {
                // Function call
                let mut args = Vec::new();

                if !self.check(&TokenKind::RightParen) {
                    loop {
                        args.push(self.expression()?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                }

                self.expect(&TokenKind::RightParen, "Expected ')' after arguments")?;

                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                };
            } else if self.match_token(&TokenKind::LeftBracket) {
                // Array indexing
                let index = self.expression()?;
                self.expect(&TokenKind::RightBracket, "Expected ']' after index")?;
                expr = Expr::Index {
                    array: Box::new(expr),
                    index: Box::new(index),
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn primary(&mut self) -> Result<Expr, String> {
        let token = self.peek().clone();

        match &token.kind {
            TokenKind::Number(n) => {
                let value = *n;
                self.advance();
                Ok(Expr::Number(value))
            }
            TokenKind::String(s) => {
                let value = s.clone();
                self.advance();
                Ok(Expr::StringLit(value))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Bool(true))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Bool(false))
            }
            TokenKind::Null => {
                self.advance();
                Ok(Expr::Null)
            }
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(Expr::Identifier(name))
            }
            TokenKind::LeftParen => {
                self.advance();
                let expr = self.expression()?;
                self.expect(&TokenKind::RightParen, "Expected ')' after expression")?;
                Ok(expr)
            }
            TokenKind::LeftBracket => {
                // Array literal
                self.advance();
                let mut elements = Vec::new();

                if !self.check(&TokenKind::RightBracket) {
                    loop {
                        elements.push(self.expression()?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                }

                self.expect(&TokenKind::RightBracket, "Expected ']' after array elements")?;
                Ok(Expr::Array(elements))
            }
            TokenKind::Fn => {
                // Anonymous function (lambda): fn(params) { body }
                self.advance();
                self.expect(&TokenKind::LeftParen, "Expected '(' after 'fn'")?;

                let mut params = Vec::new();
                if !self.check(&TokenKind::RightParen) {
                    loop {
                        params.push(self.expect_identifier("Expected parameter name")?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                }

                self.expect(&TokenKind::RightParen, "Expected ')' after parameters")?;
                self.expect(&TokenKind::LeftBrace, "Expected '{' before function body")?;
                let body = self.block()?;

                Ok(Expr::Lambda { params, body })
            }
            _ => Err(format!(
                "Unexpected token '{:?}' at line {}, column {}",
                token.kind, token.line, token.col
            )),
        }
    }

    // --- Helper methods ---

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn check(&self, kind: &TokenKind) -> bool {
        if self.is_at_end() {
            return false;
        }
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: &TokenKind, message: &str) -> Result<&Token, String> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            let token = self.peek();
            Err(format!(
                "{} at line {}, column {}",
                message, token.line, token.col
            ))
        }
    }

    fn expect_identifier(&mut self, message: &str) -> Result<String, String> {
        let token = self.peek().clone();
        if let TokenKind::Identifier(name) = token.kind {
            self.advance();
            Ok(name)
        } else {
            Err(format!(
                "{} at line {}, column {}",
                message, token.line, token.col
            ))
        }
    }
}
