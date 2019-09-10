extern crate proc_macro;
extern crate proc_macro2;
#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;
extern crate regex;

mod derive_read;
mod derive_write;

use proc_macro::TokenStream;
use syn::DeriveInput;

/// Derives PCDRecordRead trait on normal struct or tuple struct.
///
/// The field type can be either primitive, array of primitive or [Vec](std::vec::Vec) of primitive.
#[proc_macro_derive(PCDRecordRead, attributes(pcd_rename, pcd_ignore_name))]
pub fn pcd_record_read_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let derive_read_tokens = derive_read::f_pcd_record_read_derive(input)
        .unwrap_or_else(|err| err.to_compile_error().into());
    TokenStream::from(derive_read_tokens)
}

/// Derives PCDRecordWrite trait on normal struct or tuple struct.
///
/// The field type can be either primitive or array of primitive.
#[proc_macro_derive(PCDRecordWrite, attributes(pcd_rename))]
pub fn pcd_record_write_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let derive_write_tokens = derive_write::f_pcd_record_write_derive(input)
        .unwrap_or_else(|err| err.to_compile_error().into());
    TokenStream::from(derive_write_tokens)
}