use proc_macro::{Group, TokenStream, TokenTree};

use crate::shared::{
    compile_error, find_attrs, optional_attr_value, parse_named_decl, split_by_top_level_comma,
    string_lit, strip_attr_prefix, Attr,
};

#[derive(Default)]
struct UnitVariantMeta {
    rename: Option<String>,
    factor: Option<f64>,
}

#[derive(Default)]
struct VariantMeta {
    name: String,
    rename: Option<String>,
    factor: Option<f64>,
}

impl TryFrom<(String, &[Attr])> for VariantMeta {
    type Error = String;

    fn try_from((name, attrs): (String, &[Attr])) -> Result<Self, Self::Error> {
        let unit_meta = UnitVariantMeta::try_from(attrs)?;
        Ok(Self {
            name,
            rename: unit_meta.rename,
            factor: unit_meta.factor,
        })
    }
}

impl TryFrom<&[Attr]> for UnitVariantMeta {
    type Error = String;

    fn try_from(attrs: &[Attr]) -> Result<Self, Self::Error> {
        let mut out = UnitVariantMeta::default();

        for attr in find_attrs(attrs, "unit") {
            if let Some(value) = optional_attr_value(attr, "rename")? {
                out.rename = Some(value);
            }
            if let Some(value) = optional_attr_value(attr, "factor")? {
                out.factor = Some(value);
            }
        }

        Ok(out)
    }
}

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
        let unit_literal = string_lit(&variant.rename.unwrap_or_else(|| variant.name.clone()));
        let unit_factor = variant.factor.unwrap_or(1.0);

        as_str_arms.push_str(&format!("Self::{} => {},", variant.name, unit_literal));
        factor_arms.push_str(&format!("Self::{} => {}f64,", variant.name, unit_factor));
        try_from_arms.push_str(&format!(
            "{} => ::core::result::Result::Ok(Self::{}),",
            unit_literal, variant.name
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

        let (attrs, rest) = strip_attr_prefix(&segment)?;
        if rest.is_empty() {
            continue;
        }

        let variant_name = match &rest[0] {
            TokenTree::Ident(ident) => ident.to_string(),
            _ => continue,
        };

        if rest.len() > 1 && matches!(&rest[1], TokenTree::Group(_)) {
            return Err("UnitEnum supports only unit enum variants".to_string());
        }

        out.push(VariantMeta::try_from((variant_name, attrs.as_slice()))?);
    }

    Ok(out)
}
