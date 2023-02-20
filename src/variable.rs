use crate::expr;
use std::rc::Rc;

use std::collections::HashMap;

#[derive(Debug)]
pub enum Value {
    String(String),
    Array(Vec<String>),
    Function(Box<expr::Command>)
}



#[derive(Debug)]
pub struct Variable {
    value: Option<Value>,
}

impl Variable {
    pub fn new(value: Option<Value>) -> Variable {
        Variable { value }
    }

    #[inline]
    pub fn value(&self) -> &Option<Value> {
        &self.value
    }

    pub fn as_str(&self) -> &str {
        match &self.value {
            Some(Value::String(value)) => value,
            Some(Value::Function(_)) => "(function)",
            Some(Value::Array(elems)) => match elems.get(0) {
                Some(elem) => elem.as_str(),
                _ => "",
            },
            None => "",
        }
    }
}

#[derive(Debug)]
pub struct Variables {
    vars: HashMap<String, Rc<Variable>>,
}

impl Variables {
    pub fn new() -> Variables {
        Variables {
        vars: HashMap::new(),
        }
    }

    pub fn define(&mut self, key: &str) {
        self.vars.insert(key.into(),Rc::new(Variable::new(None)));
    }
    
    pub fn set(&mut self, key: &str, value: Value) {
        self.vars.insert(key.into(),Rc::new(Variable::new(Some(value))));
    }

    pub fn remove(&mut self, key: &str) -> Option<Rc<Variable>> {
        self.vars.remove(key)
    }

    pub fn get(&self, key: &str) -> Option<Rc<Variable>> {
        self.vars.get(key).cloned()
    }

    pub fn get_func_args(&self) -> Vec<Rc<Variable>> {
        let mut args = Vec::new();
        for i in 1.. {
            if let Some(var) = self.get(&i.to_string()) {
                args.push(var.clone());
            }
            else {
                break;
            }
        }
        args
    }
    
    // for getting $1, $2, $3, etc
    pub fn get_func_args_string(&self) -> Vec<String> {
        let mut args = Vec::new();
        for var in self.get_func_args() {
            if let Some(Value::String(value)) = var.value() {
                args.push(value.clone());
            }
        }

        args
    }
    
    // for setting $1, $2, $3, etc
    pub fn set_func_args(&mut self, args: &[String]) {
        for (i, arg) in args.iter().enumerate() {
            self.set(&(i+1).to_string(), Value::String(arg.clone()));
        }
    }

    pub fn set_nth_func_arg(&mut self, index: usize, value:Value) {
        self.set(&index.to_string(), value)
    }

    pub fn remove_nth_func_arg(&mut self, index: usize) -> Option<Rc<Variable>> {
        self.remove(&index.to_string())
    }

    pub fn get_nth_func_arg(&mut self, index: usize) -> Option<Rc<Variable>> {
        self.get(&index.to_string())
    }

    pub fn num_func_args(&self) -> usize {
        let mut num_args = 0;
        for i in 1..=9 {
            if self.get(&i.to_string()).is_none() {
                break;
            }

            num_args += 1;
        }
        num_args
    }

}
