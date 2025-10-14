use std::{
  cell::RefCell,
  collections::HashMap,
  rc::Rc,
};

use petgraph::{
  graph::NodeIndex,
  Graph,
};
use proc_macro2::TokenStream as NewStream;
use quote::quote;
use syn::{
  Field,
  Ident,
  Meta,
};

use crate::parser::{
  parse_drop_attribute,
  Directive,
  EdgeTarget,
};

struct EdgeRequest {
  from: NodeIndex,
  target: EdgeTarget,
  before: bool,
}

pub struct DropGraph {
  graph: Graph<Rc<RefCell<Ident>>, ()>,
  ident_idx: HashMap<String, NodeIndex>,
  canon_idx: HashMap<usize, NodeIndex>,
  pending: Vec<EdgeRequest>,
}

impl DropGraph {
  pub fn new() -> Self {
    Self {
      graph: Graph::new(),
      ident_idx: HashMap::new(),
      canon_idx: HashMap::new(),
      pending: Vec::new(),
    }
  }

  pub fn add_field(&mut self, canonical: usize, field: &Field) -> Result<(), NewStream> {
    let ident = ensure_named(field)?;

    let node = Rc::new(RefCell::new(ident.clone()));
    let node_id = self.graph.add_node(node.clone());

    self.ident_idx.insert(ident.to_string(), node_id);
    self.canon_idx.insert(canonical, node_id);

    for attr in &field.attrs {
      match &attr.meta {
        Meta::Path(_) => {}
        Meta::List(list) => {
          for directive in parse_drop_attribute(list)? {
            self.apply_directive(node_id, node.clone(), directive)?;
          }
        }
        Meta::NameValue(_) => {}
      }
    }

    Ok(())
  }

  pub fn finalize(&mut self) -> Result<(), NewStream> {
    while let Some(edge) = self.pending.pop() {
      if let Some(target_idx) = self.resolve_target(&edge.target) {
        self.apply_dependency(edge.from, target_idx, edge.before);
      } else {
        return Err(quote! { compile_error!("Unresolved drop dependency reference"); });
      }
    }

    Ok(())
  }

  fn apply_directive(
    &mut self,
    node_id: NodeIndex,
    node: Rc<RefCell<Ident>>,
    directive: Directive,
  ) -> Result<(), NewStream> {
    match directive {
      Directive::Rename(new_ident) => self.rename_node(node_id, node, new_ident),
      Directive::Before(targets) => {
        self.register_targets(node_id, targets, true);
        Ok(())
      }
      Directive::After(targets) => {
        self.register_targets(node_id, targets, false);
        Ok(())
      }
    }
  }

  fn rename_node(
    &mut self,
    node_id: NodeIndex,
    node: Rc<RefCell<Ident>>,
    new_ident: Ident,
  ) -> Result<(), NewStream> {
    let old_key = node.borrow().to_string();
    let new_key = new_ident.to_string();

    self.ident_idx.remove(&old_key);
    self.ident_idx.insert(new_key, node_id);
    *node.borrow_mut() = new_ident;

    Ok(())
  }

  fn register_targets(&mut self, from: NodeIndex, targets: Vec<EdgeTarget>, before: bool) {
    for target in targets {
      self.enqueue_dependency(from, target, before);
    }
  }

  fn enqueue_dependency(&mut self, from: NodeIndex, target: EdgeTarget, before: bool) {
    if let Some(to) = self.resolve_target(&target) {
      self.apply_dependency(from, to, before);
    } else {
      self.pending.push(EdgeRequest {
        from,
        target,
        before,
      });
    }
  }

  fn resolve_target(&self, target: &EdgeTarget) -> Option<NodeIndex> {
    match target {
      EdgeTarget::Alias(alias) => self.ident_idx.get(alias).copied(),
      EdgeTarget::Index(idx) => self.canon_idx.get(idx).copied(),
    }
  }

  fn apply_dependency(&mut self, from: NodeIndex, to: NodeIndex, before: bool) {
    if before {
      self.graph.add_edge(from, to, ());
    } else {
      self.graph.add_edge(to, from, ());
    }
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
