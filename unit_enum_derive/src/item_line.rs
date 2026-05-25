use proc_macro::{Delimiter, Group, TokenStream, TokenTree};

use crate::shared::{
    compile_error, is_punct, parse_attr, parse_attr_prefix, parse_named_decl, parse_string_literal,
    split_by_top_level_comma, string_lit, Attr,
};

pub fn expand_derive_item_line_struct(input: TokenStream) -> TokenStream {
    let (outer_attrs, struct_name, body) = match parse_struct_decl(input) {
        Ok(parsed) => parsed,
        Err(msg) => return compile_error(&msg),
    };

    let element_name = string_lit(
        &extract_item_line_element_name(&outer_attrs).unwrap_or_else(|| struct_name.clone()),
    );

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

#[derive(Default)]
struct ItemFieldMeta {
    name: Option<String>,
    ty: Option<String>,
    unit: Option<String>,
}

struct ParsedField {
    name: String,
    meta: ItemFieldMeta,
}

fn parse_struct_decl(input: TokenStream) -> Result<(Vec<Attr>, String, Group), String> {
    let tokens: Vec<TokenTree> = input.into_iter().collect();
    let (struct_index, struct_name, body) = parse_named_decl(
        &tokens,
        "struct",
        "ItemLineStruct can only be derived for structs",
        "ItemLineStruct supports only structs with named fields",
    )?;

    let mut attrs = Vec::new();
    let mut iter = tokens[..struct_index].iter();

    while let Some(token) = iter.next() {
        if is_punct(token, '#') {
            let Some(next_token) = iter.next() else {
                return Err("Malformed attribute syntax".to_string());
            };

            let group = match next_token {
                TokenTree::Group(group) if group.delimiter() == Delimiter::Bracket => group,
                _ => return Err("Malformed attribute syntax".to_string()),
            };
            attrs.push(parse_attr(group)?);
        }
    }

    Ok((attrs, struct_name, body))
}

fn parse_fields(stream: TokenStream) -> Result<Vec<ParsedField>, String> {
    let mut out = Vec::new();

    for segment in split_by_top_level_comma(stream) {
        if segment.is_empty() {
            continue;
        }

        let (attrs, mut idx) = parse_attr_prefix(&segment)?;
        if idx >= segment.len() {
            continue;
        }

        if let TokenTree::Ident(ident) = &segment[idx] {
            if ident.to_string() == "pub" {
                idx += 1;
                if idx < segment.len() {
                    if let TokenTree::Group(group) = &segment[idx] {
                        if group.delimiter() == Delimiter::Parenthesis {
                            idx += 1;
                        }
                    }
                }
            }
        }

        if idx >= segment.len() {
            continue;
        }

        let field_name = match &segment[idx] {
            TokenTree::Ident(ident) => ident.to_string(),
            _ => return Err("ItemLineStruct supports only structs with named fields".to_string()),
        };

        idx += 1;
        if idx >= segment.len() || !is_punct(&segment[idx], ':') {
            return Err("ItemLineStruct supports only structs with named fields".to_string());
        }

        let meta = extract_item_field_meta(&attrs);
        out.push(ParsedField {
            name: field_name,
            meta,
        });
    }
    Ok(out)
}

fn extract_item_line_element_name(attrs: &[Attr]) -> Option<String> {
    let mut out = None;

    for attr in attrs {
        if attr.name != "item_line" {
            continue;
        }

        for (key, value) in &attr.args {
            if key == "element" {
                out = parse_string_literal(value);
            }
        }
    }

    out
}

fn extract_item_field_meta(attrs: &[Attr]) -> ItemFieldMeta {
    let mut out = ItemFieldMeta::default();

    for attr in attrs {
        if attr.name != "item_field" {
            continue;
        }

        for (key, value) in &attr.args {
            if key == "ty" {
                out.ty = parse_string_literal(value);
            }
            if key == "name" {
                out.name = parse_string_literal(value);
            }
            if key == "unit" {
                out.unit = parse_string_literal(value);
            }
        }
    }

    out
}
