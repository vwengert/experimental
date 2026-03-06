use domain::schema::Schemas;
use domain::io;
use domain::domain::ItemData;

fn main() {
    let schemas = Schemas::load_default();
    println!("Default schemas loaded:");
    println!("{:#?}", schemas);

    // Test loading lists.json with embedded schemas
    println!("\n--- Testing lists.json with embedded schemas ---");
    match io::load_validated::<ItemData>("lists.json") {
        Ok(data) => {
            println!("✓ Successfully loaded lists.json!");
            println!("  Lists: {}", data.lists.len());
            println!("  Units: {}", data.units.len());
            println!("  Elements: {}", data.elements.len());

            // Show the schemas from the loaded data
            let loaded_schemas = data.get_schemas();
            println!("\nSchemas from loaded data:");
            println!("  Units: {:?}", loaded_schemas.units.keys().collect::<Vec<_>>());
            println!("  Elements: {:?}", loaded_schemas.elements.keys().collect::<Vec<_>>());
        },
        Err(e) => {
            eprintln!("✗ Failed to load: {}", e);
        }
    }
}