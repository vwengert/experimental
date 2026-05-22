use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Expr, ExprLit, Fields, Lit, Meta, MetaNameValue};

pub fn expand_derive_item_line_struct(input: DeriveInput) -> TokenStream {
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
            }
        },
        _ => {
            return syn::Error::new_spanned(
                struct_name,
                "ItemLineStruct can only be derived for structs",
            )
            .to_compile_error()
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
                return syn::Error::new_spanned(field, "Named field expected").to_compile_error();
            }
        };
        let field_name = field_ident.to_string();

        let field_meta = match extract_item_field_meta(&field.attrs) {
            Ok(meta) => meta,
            Err(error) => return error.to_compile_error(),
        };

        let schema_name = field_meta
            .name
            .as_deref()
            .unwrap_or(field_name.as_str())
            .to_string();

        let ty_token = match field_meta.ty.as_deref() {
            Some("Float") => quote! { ValueType::Float },
            Some("Int") => quote! { ValueType::Int },
            Some("Str") => quote! { ValueType::Str },
            Some(other) => {
                return syn::Error::new_spanned(
                    field_ident,
                    format!("Unsupported item_field ty '{other}'. Supported: Float, Int, Str"),
                )
                .to_compile_error()
            }
            None => return syn::Error::new_spanned(
                field_ident,
                "Missing item_field ty. Example: #[item_field(ty = \"Float\", unit = \"length\")] or #[item_field(ty = \"Str\")]",
            )
            .to_compile_error(),
        };

        let value_parser = match field_meta.ty.as_deref() {
            Some("Float") => quote! { parse_float_value(line, #schema_name)? },
            Some("Int") => quote! { parse_int_value(line, #schema_name)? },
            Some("Str") => quote! { parse_string_value(line, #schema_name)? },
            _ => unreachable!(),
        };

        if let Some(unit_name) = field_meta.unit.as_deref() {
            let unit_name = unit_name.to_string();
            let unit_parser = match unit_name.as_str() {
                "length" => {
                    needs_length_group_check = true;
                    quote! { parse_length_unit(line, schemas, #schema_name)? }
                }
                other => {
                    return syn::Error::new_spanned(
                        field_ident,
                        format!(
                            "Unsupported unit group '{other}'. Currently only 'length' is supported"
                        ),
                    )
                    .to_compile_error()
                }
            };

            validate_arms.push(quote! {
                validate_field(schema.field(#schema_name), #schema_name, #ty_token, #unit_name)?;
            });

            assign_arms.push(quote! {
                #field_ident: ValueWithUnit {
                    value: #value_parser,
                    unit: #unit_parser,
                }
            });
        } else {
            validate_arms.push(quote! {
                validate_field_without_unit(schema.field(#schema_name), #schema_name, #ty_token)?;
            });

            assign_arms.push(quote! {
                #field_ident: #value_parser
            });
        }
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

    quote! {
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
    }
}

#[derive(Default)]
struct ItemFieldMeta {
    name: Option<String>,
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
                if path.is_ident("name") {
                    out.name = Some(lit_str.value());
                }
                if path.is_ident("unit") {
                    out.unit = Some(lit_str.value());
                }
            }
        }
    }

    Ok(out)
}
