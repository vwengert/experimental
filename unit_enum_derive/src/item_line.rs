use proc_macro::{Delimiter, Group, TokenStream, TokenTree};

use crate::shared::{
    compile_error, find_attr, find_attrs, is_punct, optional_attr_value, parse_named_decl,
    split_by_top_level_comma, string_lit, strip_attr_prefix, Attr,
};

struct ParsedField {
    name: String,
    meta: ItemFieldMeta,
}

#[derive(Default)]
struct ItemLineMeta {
    element: Option<String>,
}

#[derive(Default)]
struct ItemFieldMeta {
    name: Option<String>,
    ty: Option<String>,
    unit: Option<String>,
}

impl TryFrom<&[Attr]> for ItemLineMeta {
    type Error = String;

    fn try_from(attrs: &[Attr]) -> Result<Self, Self::Error> {
        let mut out = ItemLineMeta::default();

        if let Some(attr) = find_attr(attrs, "item_line") {
            out.element = optional_attr_value(attr, "element")?;
        }

        Ok(out)
    }
}

impl TryFrom<&[Attr]> for ItemFieldMeta {
    type Error = String;

    fn try_from(attrs: &[Attr]) -> Result<Self, Self::Error> {
        let mut out = ItemFieldMeta::default();

        for attr in find_attrs(attrs, "item_field") {
            if let Some(value) = optional_attr_value(attr, "ty")? {
                out.ty = Some(value);
            }
            if let Some(value) = optional_attr_value(attr, "name")? {
                out.name = Some(value);
            }
            if let Some(value) = optional_attr_value(attr, "unit")? {
                out.unit = Some(value);
            }
        }

        Ok(out)
    }
}

pub fn expand_derive_item_line_struct(input: TokenStream) -> TokenStream {
    let (outer_attrs, struct_name, body) = match parse_struct_decl(input) {
        Ok(parsed) => parsed,
        Err(msg) => return compile_error(&msg),
    };

    let line_meta = match ItemLineMeta::try_from(outer_attrs.as_slice()) {
        Ok(meta) => meta,
        Err(msg) => return compile_error(&msg),
    };

    let element_name = string_lit(&line_meta.element.unwrap_or_else(|| struct_name.clone()));

    let fields = match parse_fields(body.stream()) {
        Ok(value) => value,
        Err(msg) => return compile_error(&msg),
    };

    let mut validate_arms = String::new();
    let mut assign_arms = String::new();

    for field in fields {
        let field_meta = field.meta;
        let schema_name = string_lit(&field_meta.name.unwrap_or_else(|| field.name.clone()));

        let ty_token = match field_meta.ty.as_deref() {
            Some("Float") => "ValueType::Float",
            Some("Int") => "ValueType::Int",
            Some("Str") => "ValueType::Str",
            Some(other) => {
                return compile_error(&format!(
                    "Unsupported item_field ty '{other}'. Supported: Float, Int, Str"
                ));
            }
            None => {
                return compile_error(
                    "Missing item_field ty. Example: #[item_field(ty = \"Float\", unit = \"length\")] or #[item_field(ty = \"Str\")]",
                );
            }
        };

        let value_parser = match field_meta.ty.as_deref() {
            Some("Float") => format!("parse_float_value(line, {})?", schema_name),
            Some("Int") => format!("parse_int_value(line, {})?", schema_name),
            Some("Str") => format!("parse_string_value(line, {})?", schema_name),
            _ => unreachable!(),
        };

        if let Some(unit_name) = field_meta.unit {
            let unit_parser = format!(
                "crate::utility::parse::parse_{}_unit(line, schemas, {})?",
                unit_name, schema_name
            );

            validate_arms.push_str(&format!(
                "validate_field(schema.field({}), {}, {}, {})?;",
                schema_name,
                schema_name,
                ty_token,
                string_lit(&unit_name)
            ));

            assign_arms.push_str(&format!(
                "{}: ValueWithUnit {{ value: {}, unit: {} }},",
                field.name, value_parser, unit_parser
            ));
        } else {
            validate_arms.push_str(&format!(
                "validate_field_without_unit(schema.field({}), {}, {})?;",
                schema_name, schema_name, ty_token
            ));

            assign_arms.push_str(&format!("{}: {},", field.name, value_parser));
        }
    }

    let expanded = format!(
        "
impl {struct_name} {{
    pub fn try_from_item_line(
        line: &crate::models::model::ItemLine,
        schemas: &crate::models::elements::Schemas,
    ) -> Result<Self, crate::models::error::item_line_conversion_error::ItemLineConversionError> {{
        let schema = schemas
            .schema_for({element_name})
            .ok_or(ItemLineConversionError::MissingContainerSchema)?;

        {validate_arms}

        if line.title != {element_name} {{
            return Err(ItemLineConversionError::WrongElementType {{
                expected: {element_name},
                found: line.title.clone(),
            }});
        }}

        Ok(Self {{
            {assign_arms}
        }})
    }}
}}
"
    );

    match expanded.parse() {
        Ok(tokens) => tokens,
        Err(_) => compile_error("Failed to generate ItemLineStruct implementation"),
    }
}

fn parse_struct_decl(input: TokenStream) -> Result<(Vec<Attr>, String, Group), String> {
    let tokens: Vec<TokenTree> = input.into_iter().collect();
    let (struct_index, struct_name, body) = parse_named_decl(
        &tokens,
        "struct",
        "ItemLineStruct can only be derived for structs",
        "ItemLineStruct supports only structs with named fields",
    )?;

    let (attrs, _) = strip_attr_prefix(&tokens[..struct_index])?;

    Ok((attrs, struct_name, body))
}

fn parse_fields(stream: TokenStream) -> Result<Vec<ParsedField>, String> {
    let mut out = Vec::new();

    for segment in split_by_top_level_comma(stream) {
        if segment.is_empty() {
            continue;
        }

        let (attrs, rest) = strip_attr_prefix(&segment)?;
        if rest.is_empty() {
            continue;
        }

        let mut idx = 0usize;

        if let TokenTree::Ident(ident) = &rest[idx] {
            if ident.to_string() == "pub" {
                idx += 1;
                if idx < rest.len() {
                    if let TokenTree::Group(group) = &rest[idx] {
                        if group.delimiter() == Delimiter::Parenthesis {
                            idx += 1;
                        }
                    }
                }
            }
        }

        if idx >= rest.len() {
            continue;
        }

        let field_name = match &rest[idx] {
            TokenTree::Ident(ident) => ident.to_string(),
            _ => return Err("ItemLineStruct supports only structs with named fields".to_string()),
        };

        idx += 1;
        if idx >= rest.len() || !is_punct(&rest[idx], ':') {
            return Err("ItemLineStruct supports only structs with named fields".to_string());
        }

        let meta = ItemFieldMeta::try_from(attrs.as_slice())?;
        out.push(ParsedField {
            name: field_name,
            meta,
        });
    }
    Ok(out)
}
