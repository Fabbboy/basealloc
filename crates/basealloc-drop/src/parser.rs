use proc_macro2::TokenStream as NewStream;
use quote::quote;
use syn::{
  punctuated::Punctuated,
  Expr,
  Ident,
  Lit,
  Meta,
  MetaList,
  MetaNameValue,
  Token,
};

const DROP_ATTR: &str = "drop";
const DROP_ID: &str = "id";
const DROP_BEFORE: &str = "before";
const DROP_AFTER: &str = "after";

#[derive(Clone)]
pub enum EdgeTarget {
  Alias(String),
  Index(usize),
}

pub enum Directive {
  Rename(Ident),
  Before(Vec<EdgeTarget>),
  After(Vec<EdgeTarget>),
}

pub fn parse_drop_attribute(list: &MetaList) -> Result<Vec<Directive>, NewStream> {
  if !list.path.is_ident(DROP_ATTR) {
    return Ok(Vec::new());
  }

  let meta_items = match list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated) {
    Ok(items) => items,
    Err(_) => {
      return Err(quote! {
        compile_error!("Malformed attribute parameters");
      });
    }
  };

  let mut directives = Vec::new();
  for item in meta_items {
    match item {
      Meta::NameValue(nv) => directives.push(parse_named_value(&nv)?),
      Meta::Path(_) | Meta::List(_) => {
        return Err(quote! { compile_error!("Malformed attribute parameters"); })
      }
    }
  }

  Ok(directives)
}

fn parse_named_value(meta: &MetaNameValue) -> Result<Directive, NewStream> {
  if meta.path.is_ident(DROP_ID) {
    Ok(Directive::Rename(parse_ident_expr(
      &meta.value,
      "Expected identifier literal for drop id",
    )?))
  } else if meta.path.is_ident(DROP_BEFORE) {
    Ok(Directive::Before(parse_targets(&meta.value)?))
  } else if meta.path.is_ident(DROP_AFTER) {
    Ok(Directive::After(parse_targets(&meta.value)?))
  } else {
    Err(quote! { compile_error!("Unsupported drop attribute key"); })
  }
}

fn parse_targets(expr: &Expr) -> Result<Vec<EdgeTarget>, NewStream> {
  match expr {
    Expr::Array(array) => array
      .elems
      .iter()
      .map(parse_target)
      .collect::<Result<Vec<_>, _>>(),
    _ => parse_target(expr).map(|target| vec![target]),
  }
}

fn parse_target(expr: &Expr) -> Result<EdgeTarget, NewStream> {
  match expr {
    Expr::Lit(expr_lit) => parse_literal(&expr_lit.lit),
    Expr::Path(expr_path) => expr_path
      .path
      .get_ident()
      .map(|ident| EdgeTarget::Alias(ident.to_string()))
      .ok_or_else(|| quote! { compile_error!("Expected identifier path for drop dependency"); }),
    Expr::Group(group) => parse_target(&group.expr),
    Expr::Paren(paren) => parse_target(&paren.expr),
    _ => Err(quote! { compile_error!("Unsupported expression for drop dependency"); }),
  }
}

fn parse_literal(lit: &Lit) -> Result<EdgeTarget, NewStream> {
  match lit {
    Lit::Str(lit_str) => Ok(EdgeTarget::Alias(lit_str.value())),
    Lit::Int(lit_int) => lit_int.base10_parse::<usize>().map(EdgeTarget::Index).map_err(|_| {
      quote! { compile_error!("Expected unsigned integer for drop dependency index"); }
    }),
    _ => Err(quote! { compile_error!("Unsupported literal for drop dependency"); }),
  }
}

fn parse_ident_expr(expr: &Expr, error_msg: &str) -> Result<Ident, NewStream> {
  match expr {
    Expr::Lit(expr_lit) => match &expr_lit.lit {
      Lit::Str(lit_str) => syn::parse_str::<Ident>(&lit_str.value()).map_err(|_| {
        quote! { compile_error!(#error_msg); }
      }),
      _ => Err(quote! { compile_error!(#error_msg); }),
    },
    Expr::Path(expr_path) => expr_path
      .path
      .get_ident()
      .cloned()
      .ok_or_else(|| quote! { compile_error!(#error_msg); }),
    Expr::Group(group) => parse_ident_expr(&group.expr, error_msg),
    Expr::Paren(paren) => parse_ident_expr(&paren.expr, error_msg),
    _ => Err(quote! { compile_error!(#error_msg); }),
  }
}
