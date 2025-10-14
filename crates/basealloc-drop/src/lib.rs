use proc_macro::TokenStream as OldStream;
use syn::{
  DeriveInput,
  parse_macro_input,
};

mod expand;
mod graph;
mod parser;

#[proc_macro_derive(DropOrder, attributes(drop))]
pub fn derive_drop_order(input: OldStream) -> OldStream {
  let ast = parse_macro_input!(input as DeriveInput);
  let generated = expand::expand_drop(ast);
  OldStream::from(generated)
}
