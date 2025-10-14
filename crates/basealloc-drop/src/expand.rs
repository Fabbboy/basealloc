use petgraph::algo::toposort;
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

  let topo = match toposort(drop_graph.graph(), None) {
    Ok(order) => order,
    Err(cycle) => {
      let node = &drop_graph.graph()[cycle.node_id()];
      let node_ident = &*node.borrow();
      return Err(quote! {
        compile_error!(concat!("Cycle detected in drop order involving field: ", stringify!(#node_ident)));
      });
    }
  };

  let drop_statements = topo.iter().map(|&node_idx| {
    let node = &drop_graph.graph()[node_idx];
    let field_ident = &*node.borrow();
    quote! {
      std::ptr::drop_in_place(&mut self.#field_ident);
    }
  });

  let struct_name = &ast.ident;
  let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

  Ok(quote! {
    impl #impl_generics Drop for #struct_name #ty_generics #where_clause {
      fn drop(&mut self) {
        #( #drop_statements )*
      }
    }
  })
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
