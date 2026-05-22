use proc_macro::TokenStream;
use syn::parse_macro_input;

mod item_line;
mod unit;

#[proc_macro_derive(UnitEnum, attributes(unit))]
pub fn derive_unit_enum(input: TokenStream) -> TokenStream {
    unit::expand_derive_unit_enum(parse_macro_input!(input as syn::DeriveInput)).into()
}

#[proc_macro_derive(ItemLineStruct, attributes(item_line, item_field))]
pub fn derive_item_line_struct(input: TokenStream) -> TokenStream {
    item_line::expand_derive_item_line_struct(parse_macro_input!(input as syn::DeriveInput)).into()
}
