use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone)]
pub enum CalculatorEvent {
    VariableChanged(String, f64),
    ResultCalculated(f64, String),
    ModeChanged(String),
    Error(String),
}

pub trait Observer: Send + Sync {
    fn update(&self, event: &CalculatorEvent);
}
pub trait Subject {
    fn attach(&mut self, observer: Box<dyn Observer>) -> usize;
    fn detach(&mut self, observer_id: usize);
    fn notify(&self, event: &CalculatorEvent);
}

pub struct Observable {
    observers: HashMap<usize, Box<dyn Observer>>,
    next_observer_id: usize,
}

impl Observable {
    pub fn new() -> Self {
        Self {
            observers: HashMap::new(),
            next_observer_id: 0,
        }
    }
}

impl Subject for Observable {
    fn attach(&mut self, observer: Box<dyn Observer>) -> usize {
        let id = self.next_observer_id;
        self.observers.insert(id, observer);
        self.next_observer_id += 1;
        id
    }

    fn detach(&mut self, observer_id: usize) {
        self.observers.remove(&observer_id);
    }

    fn notify(&self, event: &CalculatorEvent) {
        for observer in self.observers.values() {
            observer.update(event);
        }
    }
}

pub struct Dsp {}

impl Display for Dsp {
    fn print(&self, msg: &str) {
        println!("{}", msg);
    }
}
pub trait Display: Send + Sync {
    fn print(&self, msg: &str);
}

pub struct DisplayObserver {
    display: Arc<Mutex<dyn Display>>,
}

impl Observer for DisplayObserver {
    fn update(&self, event: &CalculatorEvent) {
        let msg = match event {
            CalculatorEvent::VariableChanged(name, value) => {
                format!("Variable '{}' changed to {}", name, value)
            }
            CalculatorEvent::ResultCalculated(result, expression) => {
                format!("Result of '{}' is {}", expression, result)
            }
            CalculatorEvent::ModeChanged(mode) => format!("Mode changed to '{}'", mode),
            CalculatorEvent::Error(err) => format!("Error: {}", err),
        };
        self.display.lock().unwrap().print(&msg);
    }
}
fn main() {
    let mut observable = Observable::new();
    let observer = DisplayObserver {
        display: Arc::new(Mutex::new(Dsp {})),
    };
    observable.attach(Box::new(observer));
    observable.notify(&CalculatorEvent::VariableChanged("x".to_string(), 42.0));

    println!("Hello, world!");
}
