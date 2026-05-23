use proc_macro::{Delimiter, Group, TokenStream, TokenTree};

pub fn expand_derive_unit_enum(input: TokenStream) -> TokenStream {
    let (enum_name, body) = match parse_enum_decl(input) {
        Ok(parsed) => parsed,
        Err(msg) => return compile_error(&msg),
    };

    let variants = match parse_variants(body.stream()) {
        Ok(value) => value,
        Err(msg) => return compile_error(&msg),
    };

    let mut as_str_arms = String::new();
    let mut factor_arms = String::new();
    let mut try_from_arms = String::new();

    for variant in variants {
        let unit_literal = variant.rename.unwrap_or_else(|| variant.name.clone());
        let unit_factor = variant.factor.unwrap_or(1.0);

        as_str_arms.push_str(&format!(
            "Self::{} => {},",
            variant.name,
            string_lit(&unit_literal)
        ));
        factor_arms.push_str(&format!("Self::{} => {}f64,", variant.name, unit_factor));
        try_from_arms.push_str(&format!(
            "{} => ::core::result::Result::Ok(Self::{}),",
            string_lit(&unit_literal),
            variant.name
        ));
    }

    let expanded = format!(
        "
impl {enum_name} {{
    pub fn as_str(&self) -> &'static str {{
        match self {{
            {as_str_arms}
        }}
    }}

    pub fn factor(&self) -> f64 {{
        match self {{
            {factor_arms}
        }}
    }}

    pub fn convert_value(value: f64, from: Self, to: Self) -> f64 {{
        value * from.factor() / to.factor()
    }}
}}

impl ::core::convert::TryFrom<&str> for {enum_name} {{
    type Error = ::std::string::String;

    fn try_from(value: &str) -> ::core::result::Result<Self, Self::Error> {{
        match value {{
            {try_from_arms}
            _ => ::core::result::Result::Err(::std::format!(\"Unsupported unit '{{}}'\", value)),
        }}
    }}
}}

impl UnitConvertible for {enum_name} {{
    fn unit_factor(self) -> f64 {{
        match self {{
            {factor_arms}
        }}
    }}
}}
"
    );

    match expanded.parse() {
        Ok(tokens) => tokens,
        Err(_) => compile_error("Failed to generate UnitEnum implementation"),
    }
}

#[derive(Default)]
struct VariantMeta {
    name: String,
    rename: Option<String>,
    factor: Option<f64>,
}

#[derive(Default)]
struct Attr {
    name: String,
    args: Vec<(String, String)>,
}

fn parse_enum_decl(input: TokenStream) -> Result<(String, Group), String> {
    let tokens: Vec<TokenTree> = input.into_iter().collect();
    let mut i = 0usize;

    while i < tokens.len() {
        if let TokenTree::Ident(ident) = &tokens[i] {
            if ident.to_string() == "enum" {
                if i + 1 >= tokens.len() {
                    return Err("UnitEnum can only be derived for enums".to_string());
                }

                let enum_name = match &tokens[i + 1] {
                    TokenTree::Ident(name) => name.to_string(),
                    _ => return Err("UnitEnum can only be derived for enums".to_string()),
                };

                let mut j = i + 2;
                while j < tokens.len() {
                    if let TokenTree::Group(group) = &tokens[j] {
                        if group.delimiter() == Delimiter::Brace {
                            return Ok((enum_name, group.clone()));
                        }
                    }
                    j += 1;
                }
            }
        }
        i += 1;
    }

    Err("UnitEnum can only be derived for enums".to_string())
}

fn parse_variants(stream: TokenStream) -> Result<Vec<VariantMeta>, String> {
    let mut out = Vec::new();

    for segment in split_by_top_level_comma(stream) {
        if segment.is_empty() {
            continue;
        }

        let (attrs, idx) = parse_attr_prefix(&segment)?;
        if idx >= segment.len() {
            continue;
        }

        let variant_name = match &segment[idx] {
            TokenTree::Ident(ident) => ident.to_string(),
            _ => continue,
        };

        if idx + 1 < segment.len() {
            if let TokenTree::Group(_) = segment[idx + 1] {
                return Err("UnitEnum supports only unit enum variants".to_string());
            }
        }

        let mut variant = VariantMeta {
            name: variant_name,
            rename: None,
            factor: None,
        };

        for attr in attrs {
            if attr.name != "unit" {
                continue;
            }

            for (key, value) in attr.args {
                if key == "rename" {
                    variant.rename = parse_string_literal(&value);
                }
                if key == "factor" {
                    variant.factor = value.parse::<f64>().ok();
                }
            }
        }

        out.push(variant);
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
