use std::{
  cell::RefCell,
  collections::HashMap,
  rc::Rc,
};

use petgraph::{
  Graph,
  graph::NodeIndex,
};
use proc_macro2::TokenStream as NewStream;
use quote::quote;
use syn::{
  Data,
  DataStruct,
  DeriveInput,
  Expr,
  Field,
  Ident,
  Lit,
  Meta,
  MetaList,
  MetaNameValue,
  Token,
  punctuated::Punctuated,
};

const DROP_ATTR: &str = "drop";
const DROP_ID: &str = "id";
const DROP_BEFORE: &str = "before";

struct DropField {
  ident: Ident,
  canonical: usize,
}

macro_rules! res_ret {
  ($expr:expr) => {
    match $expr {
      Ok(v) => v,
      Err(e) => return e,
    }
  };
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

fn ensure_named(field: &Field) -> Result<&Ident, NewStream> {
  if let Some(ident) = &field.ident {
    Ok(ident)
  } else {
    Err(quote! {
        compile_error!("DropOrder can only be derived for structs with named fields");
    })
  }
}

fn process_field_named(
  this_id: NodeIndex,
  field: Rc<RefCell<DropField>>,
  meta: &MetaNameValue,
  lookup: &HashMap<Ident, NodeIndex>,
) -> Result<(), NewStream> {
  let mut lfield = field.borrow_mut();

  if meta.path.is_ident(DROP_ID) {
    match &meta.value {
      Expr::Lit(expr_lit) => {
        if let Lit::Str(lit_str) = &expr_lit.lit {
          lfield.ident = Ident::new(&lit_str.value(), lit_str.span());
        } else {
          return Err(quote! { compile_error!("Expected string literal for drop id"); });
        }
      }
      _ => return Err(quote! { compile_error!("Expected string literal for drop id"); }),
    }
  }

  if meta.path.is_ident(DROP_BEFORE) {
    // accepted forms:
    // #[drop(before = "fieldname")]
    // #[drop(before = ["fieldname1", "fieldname2"])]
  }

  Ok(())
}

fn process_field_attr(
  this_id: NodeIndex,
  field: Rc<RefCell<DropField>>,
  list: &MetaList,
  lookup: &HashMap<Ident, NodeIndex>,
) -> Result<(), NewStream> {
  if !list.path.is_ident(DROP_ATTR) {
    return Ok(());
  }

  let meta_list = match list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated) {
    Ok(p) => p,
    Err(_) => {
      return Err(quote! {
        compile_error!("Malformed attribute parameters");
      });
    }
  };

  for item in meta_list {
    match item {
      Meta::Path(_) => return Err(quote! { compile_error!("Malformed attribute parameters"); }),
      Meta::List(_) => return Err(quote! { compile_error!("Malformed attribute parameters"); }),
      Meta::NameValue(nv) => process_field_named(this_id, field.clone(), &nv, lookup)?,
    }
  }

  Ok(())
}

fn process_field(
  canonical: usize,
  field: &Field,
  graph: &mut Graph<Rc<RefCell<DropField>>, ()>,
  lookup: &mut HashMap<Ident, NodeIndex>,
) -> Result<(), NewStream> {
  let ident = ensure_named(field)?;

  let dropf = DropField {
    ident: ident.clone(),
    canonical,
  };

  let dropf_wrapped = Rc::new(RefCell::new(dropf));
  let this_id = graph.add_node(dropf_wrapped.clone());

  for attr in &field.attrs {
    match &attr.meta {
      Meta::Path(_) => {} // #[drop]
      Meta::List(meta_list) => {
        process_field_attr(this_id, dropf_wrapped.clone(), meta_list, &lookup)?
      }
      Meta::NameValue(_) => {}
    }
  }

  lookup.insert(ident.clone(), graph.node_indices().last().unwrap());
  Ok(())
}

pub fn expand_drop(ast: DeriveInput) -> NewStream {
  let struct_data = res_ret!(ensure_struct(&ast));

  let mut graph = Graph::<Rc<RefCell<DropField>>, ()>::new();
  let mut lookup = HashMap::<Ident, NodeIndex>::new();

  for (i, field) in struct_data.fields.iter().enumerate() {
    res_ret!(process_field(i, field, &mut graph, &mut lookup));
  }

  quote! {}
}
