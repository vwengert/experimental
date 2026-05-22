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

#[proc_macro_derive(ItemLineStruct, attributes(item_line, item_field))]
pub fn derive_item_line_struct(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = input.ident;

    let fields = match input.data {
        Data::Struct(data_struct) => match data_struct.fields {
            Fields::Named(fields) => fields.named,
            _ => {
                return syn::Error::new_spanned(
                    struct_name,
                    "ItemLineStruct supports only structs with named fields",
                )
                .to_compile_error()
                .into()
            }
        },
        _ => {
            return syn::Error::new_spanned(
                struct_name,
                "ItemLineStruct can only be derived for structs",
            )
            .to_compile_error()
            .into()
        }
    };

    let element_name =
        extract_item_line_element_name(&input.attrs).unwrap_or_else(|| struct_name.to_string());

    let mut validate_arms = Vec::new();
    let mut assign_arms = Vec::new();
    let mut needs_length_group_check = false;

    for field in fields {
        let field_ident = match field.ident {
            Some(ident) => ident,
            None => {
                return syn::Error::new_spanned(field, "Named field expected")
                    .to_compile_error()
                    .into()
            }
        };
        let field_name = field_ident.to_string();

        let field_meta = match extract_item_field_meta(&field.attrs) {
            Ok(meta) => meta,
            Err(error) => return error.to_compile_error().into(),
        };

        let ty_token = match field_meta.ty.as_deref() {
            Some("Float") => quote! { ValueType::Float },
            Some("Int") => quote! { ValueType::Int },
            Some(other) => {
                return syn::Error::new_spanned(
                    field_ident,
                    format!("Unsupported item_field ty '{other}'. Supported: Float, Int"),
                )
                .to_compile_error()
                .into()
            }
            None => return syn::Error::new_spanned(
                field_ident,
                "Missing item_field ty. Example: #[item_field(ty = \"Float\", unit = \"length\")]",
            )
            .to_compile_error()
            .into(),
        };

        let unit_name = match field_meta.unit.as_deref() {
            Some(unit) => unit.to_string(),
            None => {
                return syn::Error::new_spanned(
                    field_ident,
                    "Missing item_field unit. Example: #[item_field(ty = \"Float\", unit = \"length\")]",
                )
                .to_compile_error()
                .into()
            }
        };

        let value_parser = match field_meta.ty.as_deref() {
            Some("Float") => quote! { parse_float_value(line, #field_name)? },
            Some("Int") => quote! { parse_int_value(line, #field_name)? },
            _ => unreachable!(),
        };

        let unit_parser =
            match unit_name.as_str() {
                "length" => {
                    needs_length_group_check = true;
                    quote! { parse_length_unit(line, schemas, #field_name)? }
                }
                other => return syn::Error::new_spanned(
                    field_ident,
                    format!(
                        "Unsupported unit group '{other}'. Currently only 'length' is supported"
                    ),
                )
                .to_compile_error()
                .into(),
            };

        validate_arms.push(quote! {
            validate_field(schema.field(#field_name), #field_name, #ty_token, #unit_name)?;
        });

        assign_arms.push(quote! {
            #field_ident: ValueWithUnit {
                value: #value_parser,
                unit: #unit_parser,
            }
        });
    }

    let unit_group_check = if needs_length_group_check {
        quote! {
            if !schemas.units.contains_key("length") {
                return Err(ItemLineConversionError::MissingLengthUnitGroup);
            }
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        impl #struct_name {
            pub fn try_from_item_line(
                line: &crate::models::model::ItemLine,
                schemas: &crate::models::elements::Schemas,
            ) -> Result<Self, crate::models::error::item_line_conversion_error::ItemLineConversionError> {
                let schema = schemas
                    .schema_for(#element_name)
                    .ok_or(ItemLineConversionError::MissingContainerSchema)?;

                #(#validate_arms)*
                #unit_group_check

                if line.title != #element_name {
                    return Err(ItemLineConversionError::WrongElementType {
                        expected: #element_name,
                        found: line.title.clone(),
                    });
                }

                Ok(Self {
                    #(#assign_arms,)*
                })
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

#[derive(Default)]
struct ItemFieldMeta {
    ty: Option<String>,
    unit: Option<String>,
}

fn extract_item_line_element_name(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if !attr.path().is_ident("item_line") {
            continue;
        }

        let meta_list = attr.meta.require_list().ok()?;
        let nested: syn::punctuated::Punctuated<Meta, syn::Token![,]> = meta_list
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
                if path.is_ident("element") {
                    return Some(lit_str.value());
                }
            }
        }
    }

    None
}

fn extract_item_field_meta(attrs: &[Attribute]) -> Result<ItemFieldMeta, syn::Error> {
    let mut out = ItemFieldMeta::default();

    for attr in attrs {
        if !attr.path().is_ident("item_field") {
            continue;
        }

        let meta_list = attr.meta.require_list()?;
        let nested: syn::punctuated::Punctuated<Meta, syn::Token![,]> =
            meta_list.parse_args_with(syn::punctuated::Punctuated::parse_terminated)?;

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
                if path.is_ident("ty") {
                    out.ty = Some(lit_str.value());
                }
                if path.is_ident("unit") {
                    out.unit = Some(lit_str.value());
                }
            }
        }
    }

    Ok(out)
}
