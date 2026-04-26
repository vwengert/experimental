use domain::dto::timesteps_dto::Timestep;
// use domain::models::elements::Schemas;
use domain::models::unified_model::UnifiedModel;
use domain::utility::persistence;

fn main() {
    // let schemas = Schemas::load_default();
    // println!("Default schemas loaded:");
    // println!("{:#?}", schemas);

    // // Test loading lists.json with embedded schemas
    // println!("\n--- Testing lists.json with embedded schemas ---");
    // match persistence::load_validated("lists.json") {
    //     Ok(data) => {
    //         println!("✓ Successfully loaded lists.json!");
    //         println!("  Lists: {}", data.lists.len());
    //     }
    //     Err(e) => {
    //         eprintln!("✗ Failed to load: ĵ}", e);
    //     }
    // }

    let timesteps: Vec<Timestep> =
        persistence::load::<Vec<Timestep>>("/workspaces/experimental/domain/assets/timesteps.json")
            .unwrap();

    // Multiply the number of objects by 10 for each timestep
    // for timestep in &mut timesteps {
    //     let original_objects = timestep.objects.clone();
    //     for _ in 0..9 {
    //         timestep.objects.extend(original_objects.clone());
    //     }
    //     timestep.num_objects = timestep.objects.len();
    // }

    // Multiply the number of timesteps 500 times and adjust timestep numbers
    // let original_timesteps = timesteps.clone();
    // timesteps.clear();
    // for i in 0..500 {
    //     for timestep in &original_timesteps {
    //         let mut new_timestep = timestep.clone();
    //         new_timestep.timestep =
    //             (i as f64 * 0.3 + timestep.timestep as f64 * 0.3).round() / 10.0;
    //         timesteps.push(new_timestep);
    //     }
    // }

    // Save the updated timesteps back to the file using save_json utility
    // persistence::save_json(
    //     "/workspaces/experimental/domain/assets/timesteps.json",
    //     &timesteps,
    // )
    // .unwrap();

    println!("Updated timesteps saved successfully.");

    let unified_model: UnifiedModel = timesteps.into();
    println!("Remodeling to Unified Model done");
    let _timesteps: Vec<Timestep> = unified_model.into();
    println!("Remodeling back to Timesteps done");
}
