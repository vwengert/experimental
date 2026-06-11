use std::sync::{Arc, Mutex};
use std::thread;

// 1. The Slint GUI Definition
slint::include_modules!();

// 1. The Observer Trait
trait CalculationObserver: Send + Sync {
    fn on_calculation_complete(&self, result: String);
}

// 2. Your External Data Struct
struct AppController {
    saved_result_data: String,
    ui_handle: slint::Weak<AppWindow>,
}

impl AppController {
    fn new(ui_handle: slint::Weak<AppWindow>) -> Self {
        Self {
            saved_result_data: String::new(),
            ui_handle,
        }
    }

    fn calculate(self_arc: Arc<Mutex<Self>>) {
        // Clone the Arc reference to move it into the thread
        let observer_clone = Arc::clone(&self_arc);

        // 3. Spawn the calculation thread directly
        thread::spawn(move || {
            println!("Thread started: Calculating...");

            // Do your hard work or data generation directly here
            thread::sleep(std::time::Duration::from_secs(2));
            let result = "Success: 42 without channels!".to_string();

            // Direct function call right here when done!
            observer_clone.on_calculation_complete(result);
        });
    }
}

// 4. Implement the Observer on the thread-safe wrapper
impl CalculationObserver for Arc<Mutex<AppController>> {
    fn on_calculation_complete(&self, result: String) {
        println!("Observer called directly by the working thread!");

        let ui_weak = {
            // Lock briefly, write the data into the struct outside the GUI
            let mut controller = self.lock().unwrap();
            controller.saved_result_data = result.clone();

            // Extract the weak handle out of the lock
            controller.ui_handle.clone()
        };

        // Push data directly to the GUI view using Slint's thread-safe helper
        ui_weak.upgrade_in_event_loop(move |ui| {
            ui.set_gui_data(result.into());
        });
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let main_window = AppWindow::new()?;
    let controller = Arc::new(Mutex::new(AppController::new(main_window.as_weak())));

    let controller_clone = Arc::clone(&controller);
    main_window.on_start_calculation_clicked(move || {
        AppController::calculate(Arc::clone(&controller_clone));
    });

    main_window.run()
}
