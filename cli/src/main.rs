mod builder;
mod config;
mod decorator;
mod expression;
mod factory;
mod token;

use crate::expression::Expression;
use builder::ExpressionBuilder;
use config::CalculatorConfig;
use factory::{NumberToken, ScientificFactory, StandardFactory, TokenFactory};
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use token::{Function, Operator, Token};

pub struct Loop {
    counter: i32,
}

impl Loop {
    fn increment(&mut self) {
        self.counter += 1;
        println!("Incremented counter: {}", self.counter);
    }
}

pub struct Main {
    inner: Arc<Mutex<Loop>>,
    is_running: Arc<AtomicBool>,
}
impl Main {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Loop { counter: 0 })),
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }
    pub fn start_timer(&self, delay: Duration) {
        if self.is_running.swap(true, SeqCst) {
            println!("Timer is already running");
            return;
        }

        let inner_clone = Arc::clone(&self.inner);
        let is_running_clone = Arc::clone(&self.is_running);
        thread::spawn(move || {
            while is_running_clone.load(SeqCst) {
                thread::sleep(delay);
                if !is_running_clone.load(SeqCst) {
                    break;
                }
                if let Ok(mut inner) = inner_clone.lock() {
                    inner.increment()
                }
            }
            println!("Timer thread finished");
        });
    }

    pub fn stop_timer(&self) {
        self.is_running.store(false, SeqCst);
    }
    pub fn get_count(&self) -> i32 {
        self.inner.lock().unwrap().counter
    }
}

pub struct ScientificFunctionExpression {
    operation: Box<dyn Fn(f64) -> f64>,
    arg_expression: Box<dyn Expression>,
    description: String,
}
impl ScientificFunctionExpression {
    pub fn new_sin(arg: Box<dyn Expression>) -> Self {
        let operation = Box::new(move |angle: f64| f64::sin(angle));
        Self {
            operation,
            arg_expression: arg,
            description: "sin".to_string(),
        }
    }
}
impl Expression for ScientificFunctionExpression {
    fn evaluate(&self, variables: &HashMap<String, f64>) -> Result<f64, String> {
        let arg_value = self.arg_expression.evaluate(variables)?;
        Ok((self.operation)(arg_value))
    }
    fn to_string(&self) -> String {
        format!("{} ({})", self.description, self.arg_expression.to_string())
    }
    fn precedence(&self) -> u8 {
        self.arg_expression.precedence()
    }
}

fn main() {
    // Demonstrate Factory Methods
    let num_token = Token::number(42.0);
    let op_token = Token::operator(Operator::Add);
    let func_token = Token::function(Function::Sin);
    let var_token = Token::variable("x");

    println!(
        "Created tokens: {:?}, {:?}, {:?}, {:?}",
        num_token, op_token, func_token, var_token
    );

    // Demonstrate Factory from string
    match Token::from_str("3.14") {
        Ok(token) => println!("Parsed number: {:?}", token),
        Err(e) => println!("Error: {}", e),
    }

    // Demonstrate Abstract Factory
    let standard_factory = StandardFactory;
    let sci_factory = ScientificFactory;

    let standard_num = standard_factory.create_number("123").unwrap();
    let sci_num = sci_factory.create_number("1.23e-4").unwrap();

    println!("Standard number: {}", standard_num.format());
    println!("Scientific number: {}", sci_num.format());

    // Demonstrate Builder pattern
    let expr = ExpressionBuilder::new()
        .number(2.0)
        .operator(Operator::Add)
        .open_paren()
        .number(3.0)
        .operator(Operator::Multiply)
        .number(4.0)
        .close_paren()
        .unwrap() // close_paren returns Result<Self, String>
        .build()
        .unwrap();

    println!("Built expression: {:?}", expr);

    // Demonstrate configuration (alternative to Singleton)
    let default_config = CalculatorConfig::default();
    let sci_config = CalculatorConfig::scientific();

    println!("Default config: {:?}", default_config);
    println!("Scientific config: {:?}", sci_config);

    let my_loop = Main::new();
    my_loop.start_timer(Duration::from_secs(1));

    thread::sleep(Duration::from_secs(10));
    my_loop.stop_timer();
    println!("Loop counter after stop: {}", my_loop.get_count());
    thread::sleep(Duration::from_secs(3));
    my_loop.start_timer(Duration::from_millis(100));
    thread::sleep(Duration::from_secs(12));
    my_loop.stop_timer();
    println!("Loop counter after stop: {}", my_loop.get_count());
}
