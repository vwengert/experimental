use proc_macro::TokenStream;

mod item_line;
mod shared;
mod unit;

#[proc_macro_derive(UnitEnum, attributes(unit))]
pub fn derive_unit_enum(input: TokenStream) -> TokenStream {
    unit::expand_derive_unit_enum(input)
}

#[proc_macro_derive(ItemLineStruct, attributes(item_line, item_field))]
pub fn derive_item_line_struct(input: TokenStream) -> TokenStream {
    item_line::expand_derive_item_line_struct(input)
}
