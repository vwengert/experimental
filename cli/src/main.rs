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
        },
        Err(e) => {
            eprintln!("✗ Failed to load: {}", e);
        }
    }
}