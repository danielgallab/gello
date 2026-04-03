use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::interpreter::Value;

/// A reference-counted environment for lexical scoping and closures
pub type EnvRef = Rc<RefCell<Environment>>;

/// A scoped variable store with parent chain for lexical scoping
#[derive(Debug, Clone)]
pub struct Environment {
    values: HashMap<String, Value>,
    parent: Option<EnvRef>,
}

impl Environment {
    /// Create a new root environment
    pub fn new() -> EnvRef {
        Rc::new(RefCell::new(Environment {
            values: HashMap::new(),
            parent: None,
        }))
    }

    /// Create a child environment with the given parent
    pub fn new_child(parent: EnvRef) -> EnvRef {
        Rc::new(RefCell::new(Environment {
            values: HashMap::new(),
            parent: Some(parent),
        }))
    }

    /// Get a value by name, walking up the parent chain if needed
    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(val) = self.values.get(name) {
            Some(val.clone())
        } else if let Some(ref parent) = self.parent {
            parent.borrow().get(name)
        } else {
            None
        }
    }

    /// Set a value in the current scope only (for let bindings)
    pub fn set(&mut self, name: &str, val: Value) {
        self.values.insert(name.to_string(), val);
    }

    /// Assign to an existing binding, walking up to find it
    /// Returns false if the variable was not found
    pub fn assign(&mut self, name: &str, val: Value) -> bool {
        if self.values.contains_key(name) {
            self.values.insert(name.to_string(), val);
            true
        } else if let Some(ref parent) = self.parent {
            parent.borrow_mut().assign(name, val)
        } else {
            false
        }
    }
}

