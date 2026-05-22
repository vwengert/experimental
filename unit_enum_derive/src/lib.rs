use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Expr, ExprLit, Fields, Lit, Meta, MetaList,
    MetaNameValue,
};

#[proc_macro_derive(UnitEnum, attributes(unit, serde))]
pub fn derive_unit_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = input.ident;

    let data_enum = match input.data {
        Data::Enum(data) => data,
        _ => {
            return syn::Error::new_spanned(enum_name, "UnitEnum can only be derived for enums")
                .to_compile_error()
                .into()
        }
    };

    let mut as_str_arms = Vec::new();
    let mut factor_arms = Vec::new();
    let mut try_from_arms = Vec::new();

    for variant in data_enum.variants {
        if !matches!(variant.fields, Fields::Unit) {
            return syn::Error::new_spanned(variant, "UnitEnum supports only unit enum variants")
                .to_compile_error()
                .into();
        }

        let variant_ident = variant.ident;
        let unit_meta = extract_unit_meta(&variant.attrs);
        let unit_literal = unit_meta
            .rename
            .unwrap_or_else(|| variant_ident.to_string());
        let unit_factor = unit_meta.factor.unwrap_or(1.0f64);

        as_str_arms.push(quote! {
            Self::#variant_ident => #unit_literal,
        });

        factor_arms.push(quote! {
            Self::#variant_ident => #unit_factor,
        });

        try_from_arms.push(quote! {
            #unit_literal => ::core::result::Result::Ok(Self::#variant_ident),
        });
    }

    let expanded = quote! {
        impl #enum_name {
            pub fn as_str(self) -> &'static str {
                match self {
                    #(#as_str_arms)*
                }
            }

            pub fn factor(self) -> f64 {
                match self {
                    #(#factor_arms)*
                }
            }

            pub fn convert_value(value: f64, from: Self, to: Self) -> f64 {
                value * from.factor() / to.factor()
            }
        }

        impl ::core::convert::TryFrom<&str> for #enum_name {
            type Error = ::std::string::String;

            fn try_from(value: &str) -> ::core::result::Result<Self, Self::Error> {
                match value {
                    #(#try_from_arms)*
                    _ => ::core::result::Result::Err(
                        ::std::format!("Unsupported unit '{}'", value)
                    ),
                }
            }
        }

        impl UnitConvertible for #enum_name {
            fn unit_factor(self) -> f64 {
                match self {
                    #(#factor_arms)*
                }
            }
        }
    };

    expanded.into()
}

#[derive(Default)]
struct UnitMeta {
    rename: Option<String>,
    factor: Option<f64>,
}

fn extract_unit_meta(attrs: &[Attribute]) -> UnitMeta {
    let mut unit_meta = UnitMeta::default();

    for attr in attrs {
        if attr.path().is_ident("unit") {
            if let Ok(meta_list) = attr.meta.require_list() {
                apply_unit_list(meta_list, &mut unit_meta);
            }
        }

        if attr.path().is_ident("serde") {
            if let Ok(meta_list) = attr.meta.require_list() {
                if unit_meta.rename.is_none() {
                    unit_meta.rename = extract_serde_rename(meta_list);
                }
            }
        }
    }

    unit_meta
}

fn apply_unit_list(meta: &MetaList, out: &mut UnitMeta) {
    let nested: syn::punctuated::Punctuated<Meta, syn::Token![,]> =
        match meta.parse_args_with(syn::punctuated::Punctuated::parse_terminated) {
            Ok(value) => value,
            Err(_) => return,
        };

    for node in nested {
        if let Meta::NameValue(MetaNameValue { path, value, .. }) = node {
            if path.is_ident("rename") {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(lit_str),
                    ..
                }) = &value
                {
                    out.rename = Some(lit_str.value());
                }
            }

            if path.is_ident("factor") {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Float(lit_float),
                    ..
                }) = &value
                {
                    if let Ok(parsed) = lit_float.base10_parse::<f64>() {
                        out.factor = Some(parsed);
                    }
                } else if let Expr::Lit(ExprLit {
                    lit: Lit::Int(lit_int),
                    ..
                }) = &value
                {
                    if let Ok(parsed) = lit_int.base10_parse::<f64>() {
                        out.factor = Some(parsed);
                    }
                }
            }
        }
    }
}

fn extract_literal_from_list(meta: &MetaList) -> Option<String> {
    let nested: syn::punctuated::Punctuated<Meta, syn::Token![,]> = meta
        .parse_args_with(syn::punctuated::Punctuated::parse_terminated)
        .ok()?;

    for node in nested {
        if let Meta::NameValue(MetaNameValue {
            path,
            value:
                Expr::Lit(ExprLit {
                    lit: Lit::Str(lit_str),
                    ..
                }),
            ..
        }) = node
        {
            if path.is_ident("rename") {
                return Some(lit_str.value());
            }
        }
    }

    None
}

fn extract_serde_rename(meta: &MetaList) -> Option<String> {
    extract_literal_from_list(meta)
}
