// use std::sync::{Arc, Mutex};
// use std::thread;

// 1. The Slint GUI Definition
// slint::include_modules!();

// // 1. The Observer Trait
// trait CalculationObserver: Send + Sync {
//     fn on_calculation_complete(&self, result: String);
// }

// // 2. Your External Data Struct
// struct AppController {
//     saved_result_data: String,
//     ui_handle: slint::Weak<AppWindow>,
// }

// impl AppController {
//     fn new(ui_handle: slint::Weak<AppWindow>) -> Self {
//         Self {
//             saved_result_data: String::new(),
//             ui_handle,
//         }
//     }

//     fn calculate(self_arc: Arc<Mutex<Self>>) {
//         // Clone the Arc reference to move it into the thread
//         let observer_clone = Arc::clone(&self_arc);

//         // 3. Spawn the calculation thread directly
//         thread::spawn(move || {
//             println!("Thread started: Calculating...");

//             // Do your hard work or data generation directly here
//             thread::sleep(std::time::Duration::from_secs(2));
//             let result = "Success: 42 without channels!".to_string();

//             // Direct function call right here when done!
//             observer_clone.on_calculation_complete(result);
//         });
//     }
// }

// // 4. Implement the Observer on the thread-safe wrapper
// impl CalculationObserver for Arc<Mutex<AppController>> {
//     fn on_calculation_complete(&self, result: String) {
//         println!("Observer called directly by the working thread!");

//         let ui_weak = {
//             // Lock briefly, write the data into the struct outside the GUI
//             let mut controller = self.lock().unwrap();
//             controller.saved_result_data = result.clone();

//             // Extract the weak handle out of the lock
//             controller.ui_handle.clone()
//         };

//         // Push data directly to the GUI view using Slint's thread-safe helper
//         ui_weak.upgrade_in_event_loop(move |ui| {
//             ui.set_gui_data(result.into());
//         });
//     }
// }

// fn main() -> Result<(), slint::PlatformError> {
//     let main_window = AppWindow::new()?;
//     let controller = Arc::new(Mutex::new(AppController::new(main_window.as_weak())));

//     let controller_clone = Arc::clone(&controller);
//     main_window.on_start_calculation_clicked(move || {
//         AppController::calculate(Arc::clone(&controller_clone));
//     });

//     main_window.run()
// }
use std::f64::consts::PI;

const DT: f64 = 0.02; // Zeitschritt in Sekunden
const G: f64 = 9.80665; // Erdbeschleunigung m/s^2
const MAX_ROLL_RATE: f64 = 120.0 * (std::f64::consts::PI / 180.0);

#[derive(Debug, Clone, Copy)]
struct Vector3D {
    n: f64, // North
    e: f64, // East
    d: f64, // Down
}

#[derive(Debug, Clone, Copy)]
struct Attitude {
    heading: f64,    // Psi (in Radiant)
    bank: f64,       // Phi (in Radiant)
    climb_dive: f64, // Theta (in Radiant)
}

#[derive(Debug, Clone, Copy)]
struct JetState {
    position: Vector3D,
    velocity: Vector3D,
    acceleration: Vector3D,
    attitude: Attitude,
}

// Hilfsfunktion: Normiert einen Winkel auf [-PI, PI]
fn normalize_angle(angle: f64) -> f64 {
    let mut norm = angle % (2.0 * PI);
    if norm > PI {
        norm -= 2.0 * PI;
    } else if norm < -PI {
        norm += 2.0 * PI;
    }
    norm
}

fn simulate_turn_step(state: &mut JetState, target_heading: f64, desired_acc: f64) -> bool {
    let v_horiz = (state.velocity.n.powi(2) + state.velocity.e.powi(2)).sqrt();
    if v_horiz < 0.1 {
        return true;
    }

    let heading_diff = normalize_angle(target_heading - state.attitude.heading);

    // 1. Ziel-Bank-Angle für die gewünschte Beschleunigung berechnen
    // Wenn nahe am Zielheading, wollen wir wieder aufrichten (Target Bank = 0)
    let is_close_to_target = heading_diff.abs() < 0.01;
    let target_bank = if is_close_to_target {
        0.0
    } else {
        (desired_acc / G).atan() * heading_diff.signum()
    };

    // 2. Bank Angle schrittweise über Rollrate anpassen
    let bank_diff = target_bank - state.attitude.bank;
    let max_delta_bank = MAX_ROLL_RATE * DT;

    if bank_diff.abs() <= max_delta_bank {
        state.attitude.bank = target_bank;
    } else {
        state.attitude.bank += max_delta_bank * bank_diff.signum();
    }

    // 3. Reale Zentripetalbeschleunigung aus AKTUELLEM Bank Angle ableiten
    // Physik: a_c = g * tan(bank)
    let actual_centripetal_acc = G * state.attitude.bank.tan();

    // 4. Tatsächliche Drehrate berechnen (omega = a_c / v)
    let actual_omega = actual_centripetal_acc / v_horiz;
    let mut delta_heading = actual_omega * DT;

    // Verhindern, dass wir über das Zielhading herausschießen
    if heading_diff.abs() <= delta_heading.abs() && heading_diff.signum() == delta_heading.signum()
    {
        delta_heading = heading_diff;
    }

    // 5. Kinematik-Werte mit realer Beschleunigung berechnen
    let turn_direction = state.attitude.bank.signum();
    if state.attitude.bank.abs() > 0.001 {
        // Beschleunigung wirkt senkrecht zum aktuellen Heading
        state.acceleration.n =
            -actual_centripetal_acc.abs() * state.attitude.heading.sin() * turn_direction;
        state.acceleration.e =
            actual_centripetal_acc.abs() * state.attitude.heading.cos() * turn_direction;
    } else {
        state.acceleration.n = 0.0;
        state.acceleration.e = 0.0;
    }
    state.acceleration.d = 0.0;

    // 6. Integration
    state.attitude.heading = normalize_angle(state.attitude.heading + delta_heading);

    state.velocity.n += state.acceleration.n * DT;
    state.velocity.e += state.acceleration.e * DT;
    state.position.n += state.velocity.n * DT;
    state.position.e += state.velocity.e * DT;

    // Das Manöver ist fertig, wenn das Heading erreicht UND der Jet wieder aufgerichtet ist
    is_close_to_target && state.attitude.bank.abs() < 0.01
}

fn main() {
    // Initialer Zustand (z.B. Flug nach Norden mit 200 m/s)
    let mut jet = JetState {
        position: Vector3D {
            n: 0.0,
            e: 0.0,
            d: -5000.0,
        }, // 5000m Höhe (Down ist negativ)
        velocity: Vector3D {
            n: 200.0,
            e: 0.0,
            d: 0.0,
        }, // 200 m/s Richtung Norden
        acceleration: Vector3D {
            n: 0.0,
            e: 0.0,
            d: 0.0,
        },
        attitude: Attitude {
            heading: 0.0, // 0 Radiant = Norden
            bank: 0.0,
            climb_dive: 0.0,
        },
    };

    // Ziel: Flug nach Osten (90 Grad = PI / 2) mit einer Turnrate (Beschleunigung) von 40 m/s² (~4G Kurve)
    let target_heading = PI / 2.0;
    let turn_acceleration = 40.0;

    println!("Starte Simulation des Kurvenflugs...");
    let mut time = 0.0;

    while time < 10.0 {
        // Sicherheits-Timeout 10 Sekunden
        let finished = simulate_turn_step(&mut jet, target_heading, turn_acceleration);
        time += DT;

        println!(
            "Zeit: {:.2}s | Pos: (N: {:.1}, E: {:.1}) | Velocity: (N: {:.1}, E: {:.1}, D: {:.1}) | Acceleration: (N: {:.1}, E: {:.1}, D: {:.1}) | Heading: {:.1}° | Bank: {:.1}°",
            time,
            jet.position.n,
            jet.position.e,
            jet.velocity.n,
            jet.velocity.e,
            jet.velocity.d,
            jet.acceleration.n,
            jet.acceleration.e,
            jet.acceleration.d,
            jet.attitude.heading.to_degrees(),
            jet.attitude.bank.to_degrees()
        );

        if finished {
            println!("Zielheading erreicht nach {:.2} Sekunden!", time);
            break;
        }
    }
}
