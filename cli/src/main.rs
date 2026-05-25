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
use token::{Function, Operator, Token};

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
}
