use crate::expression::Expression;
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Instant;
pub trait Logger {
    fn log(&self, msg: &str);
}
pub struct ConsoleLogger;
impl Logger for ConsoleLogger {
    fn log(&self, msg: &str) {
        println!("[LOG] {}", msg);
    }
}

pub struct LoggingExpression {
    inner: Box<dyn Expression>,
    logger: Box<dyn Logger>,
}
impl LoggingExpression {
    pub fn new(inner: Box<dyn Expression>, logger: Box<dyn Logger>) -> Self {
        Self { inner, logger }
    }
}
impl Expression for LoggingExpression {
    fn evaluate(&self, vars: &HashMap<String, f64>) -> Result<f64, String> {
        self.logger
            .log(&format!("Evaluating: {}", self.inner.to_string()));
        let result = self.inner.evaluate(vars);
        match &result {
            Ok(val) => self.logger.log(&format!("Result: {}", val)),
            Err(err) => self.logger.log(&format!("Error: {}", err)),
        }
        result
    }
    fn to_string(&self) -> String {
        self.inner.to_string()
    }
    fn precedence(&self) -> u8 {
        self.inner.precedence()
    }
}

pub struct TimingExpression {
    inner: Box<dyn Expression>,
}
impl TimingExpression {
    pub fn new(inner: Box<dyn Expression>) -> Self {
        Self { inner }
    }
}
impl Expression for TimingExpression {
    fn evaluate(&self, vars: &HashMap<String, f64>) -> Result<f64, String> {
        let start = Instant::now();
        let result = self.inner.evaluate(vars);
        let duration = start.elapsed();
        println!("Evaluation took: {:?}", duration);
        result
    }
    fn to_string(&self) -> String {
        self.inner.to_string()
    }
    fn precedence(&self) -> u8 {
        self.inner.precedence()
    }
}

pub struct CachingExpression {
    inner: Box<dyn Expression>,
    last_result: RefCell<Option<f64>>,
}
impl CachingExpression {
    pub fn new(inner: Box<dyn Expression>) -> Self {
        Self {
            inner,
            last_result: RefCell::new(None),
        }
    }
    pub fn invalidate_cache(&self) {
        *self.last_result.borrow_mut() = None;
    }
}
impl Expression for CachingExpression {
    fn evaluate(&self, vars: &HashMap<String, f64>) -> Result<f64, String> {
        if let Some(result) = *self.last_result.borrow() {
            return Ok(result);
        }
        let result = self.inner.evaluate(vars)?;
        *self.last_result.borrow_mut() = Some(result);
        Ok(result)
    }
    fn to_string(&self) -> String {
        self.inner.to_string()
    }
    fn precedence(&self) -> u8 {
        self.inner.precedence()
    }
}
