use jsonsss::domain::ElementSchemas;
use jsonsss::io::json_read_string;

fn main() {
    let schemas = include_str!("./schemas.json");
    println!("{:#?}", schemas);
    let schemas = json_read_string::<ElementSchemas>(schemas);
    println!("Loaded {:?} schemas", schemas);
}