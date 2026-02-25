use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};

pub fn load<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> Result<T, std::io::Error> {
    let text = fs::read_to_string(path)?;
    json_read_string(text.as_str())
}

pub fn save<T: Serialize>(path: impl AsRef<Path>, data: &T) -> Result<(), std::io::Error> {
    let text = serde_json::to_string_pretty(data)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    fs::write(path, text)?;
    Ok(())
}

pub fn json_read_string<T: for<'de> Deserialize<'de>>(json_str: &str) -> Result<T, std::io::Error> {
    serde_json::from_str(json_str)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}