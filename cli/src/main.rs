use jsonsss::io::load_schemas;

fn main() {
    let schemas = load_schemas("schemas.json").unwrap();
    println!("Loaded {:?} schemas", schemas);
}