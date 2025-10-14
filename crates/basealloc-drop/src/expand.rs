use proc_macro2::TokenStream as NewStream;
use quote::quote;
use syn::{
  Data,
  DataStruct,
  DeriveInput,
};

use crate::graph::DropGraph;

pub fn expand_drop(ast: DeriveInput) -> NewStream {
  match expand_drop_internal(ast) {
    Ok(tokens) => tokens,
    Err(err) => err,
  }
}

fn expand_drop_internal(ast: DeriveInput) -> Result<NewStream, NewStream> {
  let struct_data = ensure_struct(&ast)?;

  let mut drop_graph = DropGraph::new();
  for (i, field) in struct_data.fields.iter().enumerate() {
    drop_graph.add_field(i, field)?;
  }

  drop_graph.finalize()?;

  Ok(quote! {})
}

fn ensure_struct(ast: &DeriveInput) -> Result<&DataStruct, NewStream> {
  if let Data::Struct(s) = &ast.data {
    Ok(s)
  } else {
    Err(quote! {
        compile_error!("DropOrder can only be derived for structs");
    })
  }
}
