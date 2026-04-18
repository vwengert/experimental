use serde::{Deserialize, Serialize};

use crate::models::model::{ItemData, ItemLine, ItemList, ItemSet};

pub const DEFAULT_LISTS_TITLE: &str = "Lists";
pub const DEFAULT_LISTS_DESCRIPTION: &str = "Saved lists data";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListsFileDto {
    pub title: String,
    pub description: String,
    pub properties: ListsFilePropertiesDto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListsFilePropertiesDto {
    pub lists: Vec<ItemListDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemListDto {
    pub name: String,
    pub lines: Vec<ItemLineDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemLineDto {
    pub element: String,
    pub data: Vec<ItemSetDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemSetDto {
    pub key: String,
    pub value: String,
    pub unit: String,
}

impl From<ListsFileDto> for ItemData {
    fn from(value: ListsFileDto) -> Self {
        Self {
            lists: value.properties.lists.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<&ItemData> for ListsFileDto {
    fn from(value: &ItemData) -> Self {
        Self {
            title: DEFAULT_LISTS_TITLE.into(),
            description: DEFAULT_LISTS_DESCRIPTION.into(),
            properties: ListsFilePropertiesDto {
                lists: value.lists.iter().map(Into::into).collect(),
            },
        }
    }
}

impl From<ItemListDto> for ItemList {
    fn from(value: ItemListDto) -> Self {
        Self {
            name: value.name,
            lines: value.lines.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<&ItemList> for ItemListDto {
    fn from(value: &ItemList) -> Self {
        Self {
            name: value.name.clone(),
            lines: value.lines.iter().map(Into::into).collect(),
        }
    }
}

impl From<ItemLineDto> for ItemLine {
    fn from(value: ItemLineDto) -> Self {
        Self {
            title: value.element,
            data: value.data.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<&ItemLine> for ItemLineDto {
    fn from(value: &ItemLine) -> Self {
        Self {
            element: value.title.clone(),
            data: value.data.iter().map(Into::into).collect(),
        }
    }
}

impl From<ItemSetDto> for ItemSet {
    fn from(value: ItemSetDto) -> Self {
        Self {
            key: value.key,
            value: value.value,
            unit: value.unit,
        }
    }
}

impl From<&ItemSet> for ItemSetDto {
    fn from(value: &ItemSet) -> Self {
        Self {
            key: value.key.clone(),
            value: value.value.clone(),
            unit: value.unit.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_wrapped_lists_file_to_domain() {
        let dto = ListsFileDto {
            title: DEFAULT_LISTS_TITLE.into(),
            description: DEFAULT_LISTS_DESCRIPTION.into(),
            properties: ListsFilePropertiesDto {
                lists: vec![ItemListDto {
                    name: "own".into(),
                    lines: vec![ItemLineDto {
                        element: "Button".into(),
                        data: vec![ItemSetDto {
                            key: "label".into(),
                            value: "ok".into(),
                            unit: String::new(),
                        }],
                    }],
                }],
            },
        };

        let data: ItemData = dto.into();
        assert_eq!(data.lists.len(), 1);
        assert_eq!(data.lists[0].lines[0].title, "Button");
    }

    #[test]
    fn maps_domain_to_wrapped_lists_file() {
        let data = ItemData {
            lists: vec![ItemList {
                name: "own".into(),
                lines: vec![ItemLine {
                    title: "Container".into(),
                    data: vec![ItemSet {
                        key: "width".into(),
                        value: "2".into(),
                        unit: "px".into(),
                    }],
                }],
            }],
        };

        let dto = ListsFileDto::from(&data);
        assert_eq!(dto.title, DEFAULT_LISTS_TITLE);
        assert_eq!(dto.properties.lists[0].lines[0].data[0].unit, "px");
    }

}

