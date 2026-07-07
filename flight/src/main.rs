use std::f64::consts::PI;

// ==========================================
// 1. Tactical Telemetry & Jet Data Core
// ==========================================
#[derive(Debug, Clone)]
pub struct FlightData {
    pub heading: f64,   // Radians (0 to 2*PI)
    pub roll: f64,      // Radians (-PI to +PI)
    pub pitch: f64,     // Radians (-PI/2 to +PI/2)
    pub altitude: f64,  // Meters
    pub speed: f64,     // Meters per second (Velocity)
    pub current_g: f64, // Aktuell anliegende G-Kraft (1.0 = normaler Geradeausflug)

    // PARAMETER: Kampfjet-spezifische Limits und Raten
    pub roll_rate: f64, // Radians per second
    pub max_roll: f64,  // Radians
    pub max_pitch: f64, // Radians
    pub max_g: f64,     // Maximal zulässige G-Kraft des Jets (z.B. 9.0 G)
}

impl FlightData {
    // Jet-Physik: Nutzt die horizontale G-Komponente für die Wende
    pub fn apply_jet_physics(&mut self, dt: f64) {
        // 1. Kurvenphysik via horizontaler G-Komponente
        if self.roll.abs() > 0.001 && self.current_g > 1.0 {
            let horizontal_speed = self.speed * self.pitch.cos();

            // Die horizontale Kraft, die den Jet in die Kurve zieht
            let horizontal_g = self.current_g * self.roll.sin().abs();
            let turn_rate = (9.81 * horizontal_g) / horizontal_speed.max(1.0);

            if self.roll >= 0.0 {
                self.heading = (self.heading + turn_rate * dt) % (2.0 * PI);
            } else {
                self.heading = (self.heading - turn_rate * dt) % (2.0 * PI);
                if self.heading < 0.0 {
                    self.heading += 2.0 * PI;
                }
            }
        }

        // 2. Höhenphysik via Pitch-Winkel
        let vertical_velocity = self.speed * self.pitch.sin();
        self.altitude += vertical_velocity * dt;
    }
}

// ==========================================
// 2. Tactical Sub-State Framework
// ==========================================
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TacticalRollState {
    Level,
    RapidRolling { target_roll: f64 },
    BankEstablished,
    RollingOut,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum TacticalPitchState {
    Level,
    PitchingIn { target_pitch: f64 },
    GLimiting,
    PitchingOut,
}

#[derive(Debug)]
pub enum FlightState {
    Navigating {
        data: FlightData,
        roll_state: TacticalRollState,
        pitch_state: TacticalPitchState,
        target_heading: f64,
        target_altitude: f64,
        requested_g: f64,
    },
}

pub trait StateStrategy {
    fn update(self, dt: f64) -> FlightState;
}

// ==========================================
// 3. Tactical Jet Look-Ahead Utilities
// ==========================================
pub fn simulate_tactical_steps(
    mut sim_roll: f64,
    mut sim_heading: f64,
    mut sim_pitch: f64,
    mut sim_alt: f64,
    speed: f64,
    roll_rate: f64,
    g_load: f64,
    target_alt: f64,
    sim_roll_state: TacticalRollState,
    sim_pitch_state: TacticalPitchState,
    dt: f64,
) -> (f64, f64) {
    let roll_step = roll_rate * dt;
    let pitch_change_rate = (9.81 * (g_load - 1.0)) / speed;

    // Simuliert exakt die Schritte des echten Zustandsautomaten vorab
    for _ in 0..200 {
        // Max 200 Zeitschritte Vorschau (10 Sekunden Flugweg)
        let mut current_g = 1.0;

        // Pitch-Sub-Simulation
        if sim_pitch_state == TacticalPitchState::GLimiting
            || sim_pitch_state == TacticalPitchState::PitchingOut
        {
            current_g = g_load;
        }

        // Roll-Sub-Simulation
        if sim_roll_state == TacticalRollState::BankEstablished
            && sim_pitch_state == TacticalPitchState::Level
        {
            current_g = g_load;
        }

        // 1. Horizontale G-Komponente anwenden
        if sim_roll.abs() > 0.001 && current_g > 1.0 {
            let horizontal_speed = speed * sim_pitch.cos();
            let horizontal_g = current_g * sim_roll.sin().abs();
            let turn_rate = (9.81 * horizontal_g) / horizontal_speed.max(1.0);

            if sim_roll >= 0.0 {
                sim_heading = (sim_heading + turn_rate * dt) % (2.0 * PI);
            } else {
                sim_heading = (sim_heading - turn_rate * dt) % (2.0 * PI);
                if sim_heading < 0.0 {
                    sim_heading += 2.0 * PI;
                }
            }
        }

        // 2. Vertikale Bewegung anwenden
        let v_vert = speed * sim_pitch.sin();
        sim_alt += v_vert * dt;

        // Schrittweiser Abbau der Querneigung (Rolling Out)
        if sim_roll.abs() <= roll_step {
            sim_roll = 0.0;
        } else {
            sim_roll -= roll_step * sim_roll.signum();
        }

        // Schrittweiser Abbau des Pitches (Pitching Out)
        if sim_pitch.abs() <= (pitch_change_rate * dt) {
            sim_pitch = 0.0;
        } else {
            sim_pitch -= (pitch_change_rate * dt) * sim_pitch.signum();
        }

        if sim_roll.abs() <= 0.001 && sim_pitch.abs() <= 0.001 {
            break;
        }
    }
    (sim_heading, sim_alt)
}

pub fn target_heading_diff(target: f64, current: f64) -> f64 {
    let mut diff = (target - current).abs();
    if diff > PI {
        diff = 2.0 * PI - diff;
    }
    diff
}
// ==========================================
// 4. State Strategy Implementation
// ==========================================
impl StateStrategy for FlightState {
    fn update(self, dt: f64) -> FlightState {
        let FlightState::Navigating {
            mut data,
            mut roll_state,
            mut pitch_state,
            target_heading,
            target_altitude,
            requested_g,
        } = self;

        let tactical_g_limit = requested_g.min(data.max_g);

        // ----------------------------------------------------
        // A. JET PITCH STATE MACHINE (Höhenregelung via G-Load)
        // ----------------------------------------------------
        let alt_diff = target_altitude - data.altitude;

        // Exaktes Look-Ahead: Berechnet den Bremsweg des Pitches
        let (_, predicted_alt) = simulate_tactical_steps(
            data.roll,
            data.heading,
            data.pitch,
            data.altitude,
            data.speed,
            data.roll_rate,
            tactical_g_limit,
            target_altitude,
            roll_state,
            pitch_state,
            dt,
        );

        let look_ahead_triggered_pitch = (alt_diff > 0.0 && predicted_alt >= target_altitude)
            || (alt_diff < 0.0 && predicted_alt <= target_altitude);

        match pitch_state {
            TacticalPitchState::Level => {
                data.current_g = 1.0;
                if alt_diff.abs() > 2.0 {
                    let dir = alt_diff.signum();
                    let target = data.max_pitch * dir;
                    pitch_state = TacticalPitchState::GLimiting;
                }
            }
            TacticalPitchState::GLimiting => {
                data.current_g = tactical_g_limit;
                let pitch_change_rate = (9.81 * (data.current_g - 1.0)) / data.speed;
                let target_pitch = data.max_pitch * alt_diff.signum();

                if look_ahead_triggered_pitch {
                    pitch_state = TacticalPitchState::PitchingOut;
                } else {
                    let diff = target_pitch - data.pitch;
                    if diff.abs() <= (pitch_change_rate * dt) {
                        data.pitch = target_pitch;
                        pitch_state = TacticalPitchState::PitchingIn { target_pitch };
                    } else {
                        data.pitch += (pitch_change_rate * dt) * diff.signum();
                    }
                }
            }
            TacticalPitchState::PitchingIn { target_pitch } => {
                data.current_g = 1.0;
                if look_ahead_triggered_pitch {
                    pitch_state = TacticalPitchState::PitchingOut;
                }
            }
            TacticalPitchState::PitchingOut => {
                data.current_g = tactical_g_limit;
                let pitch_change_rate = (9.81 * (data.current_g - 1.0)) / data.speed;

                if data.pitch.abs() <= (pitch_change_rate * dt) || alt_diff.abs() < 0.5 {
                    data.pitch = 0.0;
                    data.current_g = 1.0;
                    pitch_state = TacticalPitchState::Level;
                } else {
                    data.pitch -= (pitch_change_rate * dt) * data.pitch.signum();
                }
            }
        }

        // ----------------------------------------------------
        // B. JET ROLL STATE MACHINE (Hochgeschwindigkeits-Wende)
        // ----------------------------------------------------
        let roll_step = data.roll_rate * dt;
        let (predicted_heading, _) = simulate_tactical_steps(
            data.roll,
            data.heading,
            data.pitch,
            data.altitude,
            data.speed,
            data.roll_rate,
            tactical_g_limit,
            target_altitude,
            roll_state,
            pitch_state,
            dt,
        );
        let heading_diff = target_heading_diff(target_heading, predicted_heading);
        let look_ahead_triggered_roll = heading_diff <= 0.005;

        match roll_state {
            TacticalRollState::Level => {
                data.roll = 0.0;
            }
            TacticalRollState::RapidRolling { target_roll } => {
                if look_ahead_triggered_roll {
                    roll_state = TacticalRollState::RollingOut;
                } else {
                    let diff = target_roll - data.roll;
                    if diff.abs() <= roll_step {
                        data.roll = target_roll;
                        roll_state = TacticalRollState::BankEstablished;
                    } else {
                        data.roll += roll_step * diff.signum();
                    }
                }
            }
            TacticalRollState::BankEstablished => {
                if pitch_state == TacticalPitchState::Level {
                    data.current_g = tactical_g_limit;
                }
                if look_ahead_triggered_roll {
                    roll_state = TacticalRollState::RollingOut;
                }
            }
            TacticalRollState::RollingOut => {
                if data.roll.abs() <= roll_step {
                    data.roll = 0.0;
                    let current_error = target_heading_diff(target_heading, data.heading);
                    if current_error < 0.01 {
                        roll_state = TacticalRollState::Level;
                    }
                } else {
                    data.roll -= roll_step * data.roll.signum();
                }
            }
        }

        // Berechnete Flugphysik anwenden
        data.apply_jet_physics(dt);

        FlightState::Navigating {
            data,
            roll_state,
            pitch_state,
            target_heading,
            target_altitude,
            requested_g,
        }
    }
}

// ==========================================
// 5. The Context Management Structure
// ==========================================
pub struct Aircraft {
    pub state: Option<FlightState>,
}

impl Aircraft {
    pub fn new(speed: f64, initial_altitude: f64) -> Self {
        let initial_data = FlightData {
            heading: 0.0,
            roll: 0.0,
            pitch: 0.0,
            altitude: initial_altitude,
            speed,
            roll_rate: 200.0_f64.to_radians(), // Blitzschnelles Anrollen
            max_roll: 65.0_f64.to_radians(),
            max_pitch: 35.0_f64.to_radians(),
            max_g: 9.0, // Hardcoded Flugzeuglimit: 9G
            current_g: 1.0,
        };

        Self {
            state: Some(FlightState::Navigating {
                data: initial_data,
                roll_state: TacticalRollState::Level,
                pitch_state: TacticalPitchState::Level,
                target_heading: 0.0,
                target_altitude: initial_altitude,
                requested_g: 1.0,
            }),
        }
    }

    pub fn command_tactical_maneuver(
        &mut self,
        turn_angle_deg: f64,
        target_altitude: f64,
        target_g_load: f64,
    ) {
        if let Some(FlightState::Navigating { mut data, .. }) = self.state.take() {
            let change_rad = turn_angle_deg.to_radians();
            let mut target_heading = (data.heading + change_rad) % (2.0 * PI);
            if target_heading < 0.0 {
                target_heading += 2.0 * PI;
            }

            let roll_direction = if turn_angle_deg >= 0.0 { 1.0 } else { -1.0 };
            let target_roll = data.max_roll * roll_direction;

            println!(
                "\n[TACTICAL COMMAND] Kursänderung: {}° | Zielhöhe: {}m | Angeforderte Last: {:.1} G (Max: {} G)",
                turn_angle_deg, target_altitude, target_g_load, data.max_g
            );

            self.state = Some(FlightState::Navigating {
                data,
                roll_state: TacticalRollState::RapidRolling { target_roll },
                pitch_state: TacticalPitchState::Level,
                target_heading,
                target_altitude,
                requested_g: target_g_load,
            });
        }
    }

    pub fn tick(&mut self, dt: f64) {
        if let Some(current_state) = self.state.take() {
            self.state = Some(current_state.update(dt));
        }
    }

    pub fn telemetry(&self) -> &FlightData {
        match self.state.as_ref().unwrap() {
            FlightState::Navigating { data, .. } => data,
        }
    }
}

// ==========================================
// 6. Simulation Entrypoint Execution
// ==========================================
fn main() {
    let mut jet = Aircraft::new(250.0, 2000.0);
    let dt = 0.02;
    let mut timer = 0.0;

    jet.tick(dt);

    // BEFEHL: Fahre eine 60-Grad-Kurve und steige um 300 Meter mit 6.0 G.
    jet.command_tactical_maneuver(6.0, 2010.0, 6.0);

    loop {
        jet.tick(dt);
        timer += dt;

        let data = jet.telemetry();

        println!(
            "Zeit: {:>4.2}s | G-Load: {:>3.1} G | Roll: {:>5.1}° | Pitch: {:>5.1}° | Kurs: {:>5.1}° | Höhe: {:>6.1}m",
            timer,
            data.current_g,
            data.roll.to_degrees(),
            data.pitch.to_degrees(),
            data.heading.to_degrees(),
            data.altitude
        );

        if let Some(FlightState::Navigating {
            roll_state: TacticalRollState::Level,
            pitch_state: TacticalPitchState::Level,
            ..
        }) = jet.state
        {
            println!(
                "[Maneuver Complete] Zielkurs und Zielhöhe mittels Fly-By-Wire perfekt stabilisiert."
            );
            break;
        }

        if timer > 15.0 {
            break;
        }
    }
}
