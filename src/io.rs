use crate::domain::{Element, ElementSchemas};
use std::fs;
use std::path::Path;

pub fn load_elements(path: impl AsRef<Path>) -> Result<Vec<Element>, std::io::Error> {
    let text = fs::read_to_string(path)?;
    let elements: Vec<Element> = serde_json::from_str(&text)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    Ok(elements)
}

pub fn save_elements(path: impl AsRef<Path>, elements: &[Element]) -> Result<(), std::io::Error> {
    let text = serde_json::to_string_pretty(elements)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    fs::write(path, text)?;
    Ok(())
}

pub fn load_schemas(path: impl AsRef<Path>) -> Result<ElementSchemas, std::io::Error> {
    let text = fs::read_to_string(path)?;
    let schemas: ElementSchemas = serde_json::from_str(&text)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    Ok(schemas)
}

pub fn save_schemas(path: impl AsRef<Path>, schemas: &ElementSchemas) -> Result<(), std::io::Error> {
    let text = serde_json::to_string_pretty(schemas)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    fs::write(path, text)?;
    Ok(())
}
