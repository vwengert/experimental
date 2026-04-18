use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

use crate::models::model::ItemData;
use crate::dto::lists::ListsFileDto;

pub fn load<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> Result<T, std::io::Error> {
    let text = fs::read_to_string(path)?;
    json_read_string(text.as_str())
}

pub fn save_json<T: Serialize>(path: impl AsRef<Path>, data: &T) -> Result<(), std::io::Error> {
    let text = serde_json::to_string_pretty(data)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    fs::write(path, text)?;
    Ok(())
}

pub fn save(path: impl AsRef<Path>, data: &ItemData) -> Result<(), std::io::Error> {
    let dto = ListsFileDto::from(data);
    save_json(path, &dto)
}

pub fn json_read_string<T: for<'de> Deserialize<'de>>(json_str: &str) -> Result<T, std::io::Error> {
    serde_json::from_str(json_str)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

/// Loads and validates a JSON file against the lists configuration.
pub fn load_validated(path: impl AsRef<Path>) -> Result<ItemData, std::io::Error> {
    let text = fs::read_to_string(path)?;
    let dto: ListsFileDto = json_read_string(text.as_str())?;
    Ok(dto.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::model::{ItemLine, ItemList, ItemSet};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_file_path(name: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("domain-{name}-{unique}.json"))
    }

    #[test]
    fn save_writes_wrapped_lists_file_and_load_validated_reads_it() {
        let path = temp_file_path("wrapped");
        let data = ItemData {
            lists: vec![ItemList {
                name: "own".into(),
                lines: vec![ItemLine {
                    title: "Button".into(),
                    data: vec![ItemSet {
                        key: "label".into(),
                        value: "ok".into(),
                        unit: String::new(),
                    }],
                }],
            }],
        };

        save(&path, &data).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"title\": \"Lists\""));
        assert!(text.contains("\"properties\""));

        let loaded = load_validated(&path).unwrap();
        assert_eq!(loaded.lists.len(), 1);
        assert_eq!(loaded.lists[0].lines[0].title, "Button");

        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_validated_rejects_legacy_bare_lists_file() {
        let path = temp_file_path("legacy");
        let text = r#"{
            "lists": [
                {
                    "name": "own",
                    "lines": []
                }
            ]
        }"#;
        fs::write(&path, text).unwrap();

        match load_validated(&path) {
            Ok(_) => panic!("legacy bare lists format should be rejected"),
            Err(error) => assert_eq!(error.kind(), std::io::ErrorKind::InvalidData),
        }

        let _ = fs::remove_file(path);
    }
}


