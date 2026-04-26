use domain::dto::timesteps_dto::Timestep;
use domain::models::elements::Schemas;
use domain::models::unified_model::UnifiedModel;
use domain::utility::persistence;

fn main() {
    let schemas = Schemas::load_default();
    println!("Default schemas loaded:");
    println!("{:#?}", schemas);

    // Test loading lists.json with embedded schemas
    println!("\n--- Testing lists.json with embedded schemas ---");
    match persistence::load_validated("lists.json") {
        Ok(data) => {
            println!("✓ Successfully loaded lists.json!");
            println!("  Lists: {}", data.lists.len());
        }
        Err(e) => {
            eprintln!("✗ Failed to load: {}", e);
        }
    }

    let unified_model: UnifiedModel =
        persistence::load::<Vec<Timestep>>("/workspaces/experimental/domain/assets/timesteps.json")
            .unwrap()
            .into();

    dbg!(unified_model);
}
