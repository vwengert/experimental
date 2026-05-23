use proc_macro::{Delimiter, Group, TokenStream, TokenTree};

pub fn expand_derive_item_line_struct(input: TokenStream) -> TokenStream {
    let (outer_attrs, struct_name, body) = match parse_struct_decl(input) {
        Ok(parsed) => parsed,
        Err(msg) => return compile_error(&msg),
    };

    let element_name =
        extract_item_line_element_name(&outer_attrs).unwrap_or_else(|| struct_name.clone());

    let fields = match parse_fields(body.stream()) {
        Ok(value) => value,
        Err(msg) => return compile_error(&msg),
    };

    let mut validate_arms = String::new();
    let mut assign_arms = String::new();
    let mut needs_length_group_check = false;

    for field in fields {
        let field_meta = field.meta;
        let schema_name = field_meta.name.unwrap_or_else(|| field.name.clone());

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
            Some("Float") => format!("parse_float_value(line, {})?", string_lit(&schema_name)),
            Some("Int") => format!("parse_int_value(line, {})?", string_lit(&schema_name)),
            Some("Str") => format!("parse_string_value(line, {})?", string_lit(&schema_name)),
            _ => unreachable!(),
        };

        if let Some(unit_name) = field_meta.unit {
            let unit_parser = match unit_name.as_str() {
                "length" => {
                    needs_length_group_check = true;
                    format!(
                        "parse_length_unit(line, schemas, {})?",
                        string_lit(&schema_name)
                    )
                }
                other => {
                    return compile_error(&format!(
                        "Unsupported unit group '{other}'. Currently only 'length' is supported"
                    ));
                }
            };

            validate_arms.push_str(&format!(
                "validate_field(schema.field({}), {}, {}, {})?;",
                string_lit(&schema_name),
                string_lit(&schema_name),
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
                string_lit(&schema_name),
                string_lit(&schema_name),
                ty_token
            ));

            assign_arms.push_str(&format!("{}: {},", field.name, value_parser));
        }
    }

    let unit_group_check = if needs_length_group_check {
        "if !schemas.units.contains_key(\"length\") { return Err(ItemLineConversionError::MissingLengthUnitGroup); }".to_string()
    } else {
        String::new()
    };

    let expanded = format!(
        "
impl {struct_name} {{
    pub fn try_from_item_line(
        line: &crate::models::model::ItemLine,
        schemas: &crate::models::elements::Schemas,
    ) -> Result<Self, crate::models::error::item_line_conversion_error::ItemLineConversionError> {{
        let schema = schemas
            .schema_for({element_name_lit})
            .ok_or(ItemLineConversionError::MissingContainerSchema)?;

        {validate_arms}
        {unit_group_check}

        if line.title != {element_name_lit} {{
            return Err(ItemLineConversionError::WrongElementType {{
                expected: {element_name_lit},
                found: line.title.clone(),
            }});
        }}

        Ok(Self {{
            {assign_arms}
        }})
    }}
}}
",
        struct_name = struct_name,
        element_name_lit = string_lit(&element_name),
        validate_arms = validate_arms,
        unit_group_check = unit_group_check,
        assign_arms = assign_arms
    );

    match expanded.parse() {
        Ok(tokens) => tokens,
        Err(_) => compile_error("Failed to generate ItemLineStruct implementation"),
    }
}

#[derive(Default)]
struct Attr {
    name: String,
    args: Vec<(String, String)>,
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
    let mut attrs = Vec::new();
    let mut i = 0usize;

    while i < tokens.len() {
        if i + 1 < tokens.len() && is_punct(&tokens[i], '#') {
            let group = match &tokens[i + 1] {
                TokenTree::Group(group) if group.delimiter() == Delimiter::Bracket => group,
                _ => return Err("Malformed attribute syntax".to_string()),
            };
            attrs.push(parse_attr(group)?);
            i += 2;
            continue;
        }

        if let TokenTree::Ident(ident) = &tokens[i] {
            if ident.to_string() == "struct" {
                if i + 1 >= tokens.len() {
                    return Err("ItemLineStruct can only be derived for structs".to_string());
                }

                let struct_name = match &tokens[i + 1] {
                    TokenTree::Ident(name) => name.to_string(),
                    _ => return Err("ItemLineStruct can only be derived for structs".to_string()),
                };

                let mut j = i + 2;
                while j < tokens.len() {
                    if let TokenTree::Group(group) = &tokens[j] {
                        if group.delimiter() == Delimiter::Brace {
                            return Ok((attrs, struct_name, group.clone()));
                        }
                    }
                    j += 1;
                }

                return Err("ItemLineStruct supports only structs with named fields".to_string());
            }
        }

        i += 1;
    }

    Err("ItemLineStruct can only be derived for structs".to_string())
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

fn parse_attr_prefix(tokens: &[TokenTree]) -> Result<(Vec<Attr>, usize), String> {
    let mut attrs = Vec::new();
    let mut i = 0usize;

    while i + 1 < tokens.len() {
        if !is_punct(&tokens[i], '#') {
            break;
        }

        let group = match &tokens[i + 1] {
            TokenTree::Group(group) if group.delimiter() == Delimiter::Bracket => group,
            _ => return Err("Malformed attribute syntax".to_string()),
        };

        attrs.push(parse_attr(group)?);
        i += 2;
    }

    Ok((attrs, i))
}

fn parse_attr(group: &Group) -> Result<Attr, String> {
    let mut tokens: Vec<TokenTree> = group.stream().into_iter().collect();
    if tokens.is_empty() {
        return Err("Malformed attribute syntax".to_string());
    }

    let name = match tokens.remove(0) {
        TokenTree::Ident(ident) => ident.to_string(),
        _ => return Err("Malformed attribute syntax".to_string()),
    };

    let mut args = Vec::new();
    if !tokens.is_empty() {
        if tokens.len() != 1 {
            return Err("Malformed attribute syntax".to_string());
        }

        let arg_group = match &tokens[0] {
            TokenTree::Group(g) if g.delimiter() == Delimiter::Parenthesis => g,
            _ => return Err("Malformed attribute syntax".to_string()),
        };

        args = parse_kv_args(arg_group.stream());
    }

    Ok(Attr { name, args })
}

fn parse_kv_args(stream: TokenStream) -> Vec<(String, String)> {
    let mut out = Vec::new();

    for segment in split_by_top_level_comma(stream) {
        if segment.len() < 3 {
            continue;
        }

        let key = match &segment[0] {
            TokenTree::Ident(ident) => ident.to_string(),
            _ => continue,
        };

        if !is_punct(&segment[1], '=') {
            continue;
        }

        let value = segment[2].to_string();
        out.push((key, value));
    }

    out
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

fn parse_string_literal(raw: &str) -> Option<String> {
    let text = raw.trim();
    if !(text.starts_with('"') && text.ends_with('"')) {
        return None;
    }

    let inner = &text[1..text.len() - 1];
    let mut result = String::with_capacity(inner.len());
    let mut chars = inner.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            result.push(ch);
            continue;
        }

        match chars.next() {
            Some('"') => result.push('"'),
            Some('\\') => result.push('\\'),
            Some('n') => result.push('\n'),
            Some('r') => result.push('\r'),
            Some('t') => result.push('\t'),
            Some(other) => {
                result.push('\\');
                result.push(other);
            }
            None => result.push('\\'),
        }
    }

    Some(result)
}

fn split_by_top_level_comma(stream: TokenStream) -> Vec<Vec<TokenTree>> {
    let mut parts = Vec::new();
    let mut current = Vec::new();
    let mut angle_depth = 0usize;

    for token in stream {
        if let TokenTree::Punct(p) = &token {
            match p.as_char() {
                '<' => angle_depth += 1,
                '>' => {
                    angle_depth = angle_depth.saturating_sub(1);
                }
                ',' if angle_depth == 0 => {
                    parts.push(current);
                    current = Vec::new();
                    continue;
                }
                _ => {}
            }
        }

        if is_punct(&token, ',') && angle_depth == 0 {
            parts.push(current);
            current = Vec::new();
            continue;
        }

        current.push(token);
    }

    parts.push(current);
    parts
}

fn is_punct(token: &TokenTree, ch: char) -> bool {
    matches!(token, TokenTree::Punct(p) if p.as_char() == ch)
}

fn compile_error(message: &str) -> TokenStream {
    let escaped = string_lit(message);
    let src = format!("compile_error!({escaped});");
    src.parse().unwrap_or_default()
}

fn string_lit(value: &str) -> String {
    format!("{:?}", value)
}
