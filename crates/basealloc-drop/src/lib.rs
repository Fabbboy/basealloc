/*
from:

struct SomeBs {
  causesSegfault: Segfaulter, // owns the whole memory SomeBs is in
  leaksMemory: MemoryLeaker,
  isVeryUnsafe: UnsafeThing,
}

to:

#[derive(DropOrder)]
struct SomeBs {
  #[drop(id = "causesSegfault")] // rename field from field name
  causesSegfault: Segfaulter, // owns the whole memory SomeBs is in
  #[drop(after = "causesSegfault")] // say something should drop after another field
  leaksMemory: MemoryLeaker,
  #[drop(before = "leaksMemory")] // say something should drop before another field
  isVeryUnsafe: UnsafeThing,
}


*/

use proc_macro::TokenStream as OldStream;
use syn::{
  DeriveInput,
  parse_macro_input,
};

mod expand;

#[proc_macro_derive(DropOrder, attributes(drop))]
pub fn derive_drop_order(input: OldStream) -> OldStream {
  let ast = parse_macro_input!(input as DeriveInput);
  let generated = expand::expand_drop(ast);
  OldStream::from(generated)
}
