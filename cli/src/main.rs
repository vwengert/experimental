use jsonsss::domain::Schemas;

fn main() {
    let schemas = Schemas::load_default();
    println!("{:#?}", schemas);
}