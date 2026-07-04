use std::f64::consts::PI;

// ==========================================
// 1. Telemetry and Flight Data Core
// ==========================================
#[derive(Debug, Clone)]
pub struct FlightData {
    pub heading: f64,   // Radians (0 to 2*PI)
    pub roll: f64,      // Radians (-PI to +PI)
    pub speed: f64,     // Meters per second (velocity)
    pub roll_rate: f64, // Radians per second (roll speed)
    pub max_roll: f64,  // Radians (Maximum performance envelope boundary)
}

impl FlightData {
    // Calculates standard atmospheric aerodynamics turn rate updates
    fn apply_turn_physics(&mut self, dt: f64) {
        if self.roll.abs() > 0.001 {
            // Aircraft Turn Rate (rad/s) = (gravity * tan(roll)) / airspeed
            let turn_rate = (9.81 * self.roll.tan()) / self.speed;
            self.heading = (self.heading + turn_rate * dt) % (2.0 * PI);
        }
    }
}

// ==========================================
// 2. State Machine Framework Structure
// ==========================================
#[derive(Debug)]
pub enum FlightState {
    Level(LevelFlight),
    RollingIn(RollingIn),
    Turning(Turning),
    RollingOut(RollingOut),
}

pub trait StateStrategy {
    // Consumes self, processes local steps, and shifts ownership onward
    fn update(self, dt: f64) -> FlightState;
}

// ==========================================
// 3. Concrete State Definitions & Implementation
// ==========================================

#[derive(Debug)]
pub struct LevelFlight {
    pub data: FlightData,
}

impl StateStrategy for LevelFlight {
    fn update(mut self, _dt: f64) -> FlightState {
        self.data.roll = 0.0; // Enforce wings level
        FlightState::Level(self)
    }
}

#[derive(Debug)]
pub struct RollingIn {
    pub data: FlightData,
    pub target_roll: f64,
    pub target_heading: f64,
}

impl StateStrategy for RollingIn {
    fn update(mut self, dt: f64) -> FlightState {
        // Look ahead to check if the target will be matched even during roll configuration
        let final_predicted_heading = simulate_full_maneuver_steps(
            self.data.roll,
            self.data.heading,
            self.data.speed,
            self.data.roll_rate,
            dt,
        );

        let heading_diff = target_heading_diff(self.target_heading, final_predicted_heading);

        if heading_diff <= 0.0 {
            println!("[Transition] Look-ahead matched target mid-roll. Rolling out early.");
            return FlightState::RollingOut(RollingOut { data: self.data });
        }

        self.data.apply_turn_physics(dt);
        let step = self.data.roll_rate * dt;
        let diff = self.target_roll - self.data.roll;

        if diff.abs() <= step {
            self.data.roll = self.target_roll;
            println!(
                "[Transition] Dynamic target bank reached ({:.1}°). Holding turn.",
                self.data.roll.to_degrees()
            );
            FlightState::Turning(Turning {
                data: self.data,
                target_heading: self.target_heading,
            })
        } else {
            self.data.roll += step * diff.signum();
            FlightState::RollingIn(self)
        }
    }
}

#[derive(Debug)]
pub struct Turning {
    pub data: FlightData,
    pub target_heading: f64,
}

impl StateStrategy for Turning {
    fn update(mut self, dt: f64) -> FlightState {
        // Look ahead check BEFORE changing the active heading coordinates
        let final_predicted_heading = simulate_full_maneuver_steps(
            self.data.roll,
            self.data.heading,
            self.data.speed,
            self.data.roll_rate,
            dt,
        );

        let heading_diff = target_heading_diff(self.target_heading, final_predicted_heading);

        if heading_diff <= 0.0 {
            println!("[Transition] Look-ahead intercepted target. Moving to RollingOut.");
            return FlightState::RollingOut(RollingOut { data: self.data });
        }

        self.data.apply_turn_physics(dt);
        FlightState::Turning(self)
    }
}

#[derive(Debug)]
pub struct RollingOut {
    pub data: FlightData,
}

impl StateStrategy for RollingOut {
    fn update(mut self, dt: f64) -> FlightState {
        self.data.apply_turn_physics(dt);
        let step = self.data.roll_rate * dt;

        if self.data.roll.abs() <= step {
            self.data.roll = 0.0;
            println!("[Transition] Wings level! Back to Level Flight.");
            FlightState::Level(LevelFlight { data: self.data })
        } else {
            self.data.roll -= step * self.data.roll.signum();
            FlightState::RollingOut(self)
        }
    }
}

// ==========================================
// 4. Shared Analytical Prediction Utilities
// ==========================================
fn simulate_full_maneuver_steps(
    mut sim_roll: f64,
    mut sim_heading: f64,
    speed: f64,
    roll_rate: f64,
    dt: f64,
) -> f64 {
    // Step loop replicating the exact future path execution of RollingOut phase ticks
    while sim_roll.abs() > 0.001 {
        let turn_rate = (9.81 * sim_roll.tan()) / speed;
        sim_heading = (sim_heading + turn_rate * dt) % (2.0 * PI);

        let step = roll_rate * dt;
        if sim_roll.abs() <= step {
            sim_roll = 0.0;
        } else {
            sim_roll -= step * sim_roll.signum();
        }
    }
    sim_heading
}

fn target_heading_diff(target: f64, current: f64) -> f64 {
    let mut diff = target - current;
    while diff <= -PI {
        diff += 2.0 * PI;
    }
    while diff > PI {
        diff -= 2.0 * PI;
    }
    diff
}

// ==========================================
// 5. The Context Management Structure
// ==========================================
pub struct Aircraft {
    pub state: Option<FlightState>,
}

impl Aircraft {
    pub fn new(speed: f64) -> Self {
        let initial_data = FlightData {
            heading: 0.0,
            roll: 0.0,
            speed,
            roll_rate: 20.0_f64.to_radians(), // 20° per second
            max_roll: 80.0_f64.to_radians(),  // Maximum envelope up to 80°
        };

        Self {
            state: Some(FlightState::Level(LevelFlight { data: initial_data })),
        }
    }

    pub fn command_turn(&mut self, turn_angle_deg: f64) {
        if let Some(FlightState::Level(level_state)) = self.state.take() {
            let change_rad = turn_angle_deg.to_radians();
            let target_heading = (level_state.data.heading + change_rad) % (2.0 * PI);
            let roll_direction = if turn_angle_deg >= 0.0 { 1.0 } else { -1.0 };

            // Dynamic bank mapping calculation:
            // Small turns generate shallow banks, large turns map right to the 80° ceiling limit.
            let proportional_gain = 1.5;
            let calculated_roll = change_rad.abs() * proportional_gain;
            let target_roll = calculated_roll.min(level_state.data.max_roll) * roll_direction;

            println!(
                "\n[Command] Turning {}°. Calculated dynamic target bank angle: {:.1}°",
                turn_angle_deg,
                target_roll.to_degrees().abs()
            );

            self.state = Some(FlightState::RollingIn(RollingIn {
                data: level_state.data,
                target_roll,
                target_heading,
            }));
        }
    }

    pub fn tick(&mut self, dt: f64) {
        if let Some(current_state) = self.state.take() {
            let next_state = match current_state {
                FlightState::Level(s) => s.update(dt),
                FlightState::RollingIn(s) => s.update(dt),
                FlightState::Turning(s) => s.update(dt),
                FlightState::RollingOut(s) => s.update(dt),
            };
            self.state = Some(next_state);
        }
    }

    pub fn telemetry(&self) -> &FlightData {
        match self.state.as_ref().unwrap() {
            FlightState::Level(s) => &s.data,
            FlightState::RollingIn(s) => &s.data,
            FlightState::Turning(s) => &s.data,
            FlightState::RollingOut(s) => &s.data,
        }
    }
}

// ==========================================
// 6. Simulation Entrypoint Execution
// ==========================================
fn main() {
    // Run Test 1: Executing a small 3-degree turn adjustment
    run_flight_simulation(3.0);

    // Run Test 2: Executing a wide 45-degree tactical turn adjustment
    run_flight_simulation(45.0);
}

fn run_flight_simulation(turn_size: f64) {
    let mut plane = Aircraft::new(100.0); // 100 meters per second airspeed
    let dt = 0.02; // 0.5 second telemetry tick resolution

    // Establish normal tracking level flight for 2 seconds
    plane.tick(dt);
    plane.tick(dt);

    plane.command_turn(turn_size);

    loop {
        plane.tick(dt);

        let data = plane.telemetry();
        let state_label = match plane.state.as_ref().unwrap() {
            FlightState::Level(_) => "Level Flight",
            FlightState::RollingIn(_) => "Rolling In  ",
            FlightState::Turning(_) => "Turning     ",
            FlightState::RollingOut(_) => "Rolling Out ",
        };

        println!(
            "State: {} | Roll: {:>5.1}° | Heading: {:>5.1}°",
            state_label,
            data.roll.to_degrees(),
            data.heading.to_degrees()
        );

        if let Some(FlightState::Level(_)) = plane.state {
            println!("--------------------------------------------------");
            break;
        }
    }
}
