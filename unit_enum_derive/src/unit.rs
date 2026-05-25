use proc_macro::{Group, TokenStream, TokenTree};

use crate::shared::{
    compile_error, parse_attr_prefix, parse_named_decl, parse_string_literal,
    split_by_top_level_comma, string_lit,
};

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

fn parse_enum_decl(input: TokenStream) -> Result<(String, Group), String> {
    let tokens: Vec<TokenTree> = input.into_iter().collect();
    let (_, enum_name, body) = parse_named_decl(
        &tokens,
        "enum",
        "UnitEnum can only be derived for enums",
        "UnitEnum can only be derived for enums",
    )?;
    Ok((enum_name, body))
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
