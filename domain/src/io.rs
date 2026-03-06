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

/// Validates a JSON string against the built-in lists.schema.json.
/// Returns Ok(()) if valid, or an error with validation details.
pub fn validate_lists_json(json_str: &str) -> Result<(), String> {
    let schema_str = include_str!("lists.schema.json");
    let schema_value: serde_json::Value = serde_json::from_str(schema_str)
        .map_err(|e| format!("Failed to parse schema: {}", e))?;

    let compiled = jsonschema::validator_for(&schema_value)
        .map_err(|e| format!("Failed to compile schema: {}", e))?;

    let instance: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    if compiled.is_valid(&instance) {
        Ok(())
    } else {
        // Collect detailed validation errors
        let error_iter = compiled.iter_errors(&instance);
        let mut error_messages = Vec::new();
        for error in error_iter {
            error_messages.push(format!("  • Path '{}': {}", error.instance_path, error));
        }

        if error_messages.is_empty() {
            Err("Schema validation failed: JSON does not match the expected schema for lists".to_string())
        } else {
            Err(format!("Schema validation failed:\n{}", error_messages.join("\n")))
        }
    }
}

/// Loads and validates a JSON file against the lists schema.
pub fn load_validated<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> Result<T, std::io::Error> {
    let text = fs::read_to_string(path)?;

    // Validate against schema first
    validate_lists_json(&text)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    // Then deserialize
    json_read_string(text.as_str())
}
