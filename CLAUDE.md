
# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## BaseAlloc
A memory allocator written in Rust. Inspired by Jemalloc and Mimalloc. Tcache for the win

## Development Commands

- **Build:** `cargo build` or `cargo build --release`
- **Test:** `cargo test` (runs all tests including unit tests in src files)
- **Test specific crate:** `cargo test -p basealloc-sys`
- **Check:** `cargo check` (faster than build, just checks compilation)
- **Clippy:** `cargo clippy` (linting)
- **Format:** `cargo fmt`

## Architecture

This is a workspace with multiple crates:
- `basealloc-sys`: Low-level primitives and system abstractions for memory management
  - `prim.rs`: Core alignment utilities, page size detection, and pointer manipulation functions
  - Uses `#![no_std]` and only `core` crate allowed (no `std`, no `alloc`)
  - Platform-specific page size detection for Linux/macOS via libc, fallback for others
- `basealloc-list`: Intrusive doubly-linked list implementation
  - `HasLink` trait for types that can be linked
  - `Link<T>` struct containing next/prev pointers  
  - `List` utility with insert_before/insert_after/remove operations
  - `ListIter` and `ListDrainer` for iteration (use `ListIter::from(&node)`)
  - Uses `#![cfg_attr(not(test), no_std)]` to allow std in tests for Vec/Debug

The allocator is designed to be standalone without fallback to libc malloc, global alloc, or system alloc.

## Data Structures

**Intrusive Linked Lists (`basealloc-list`):**
- Nodes contain their own link storage via `HasLink` trait
- `List::insert_before(item, at)` - insert item before at
- `List::insert_after(item, at)` - insert item after at  
- `List::remove(item)` - remove item from list
- `ListIter::from(&start_node)` - iterate without removing
- `ListDrainer::from(&start_node)` - iterate and remove each node

- **Radix Tree (`basealloc-rtree`):**
  - `RTree::insert(key, value)` now always stores a concrete `T`; use `remove(key)` to clear entries.
  - Nodes still allocate from a `Bump`; duplicate inserts return `RTreeError::AlreadyPresent`.
  - Benchmarks and tests expect the non-optional insert signature.
  - Keep fanout a power of two; `FANOUT.trailing_zeros()` drives level calculations.
- **Extent Map (`basealloc-alloc::static_`):**
  - Extents are registered one system page at a time via `register_extent`/`unregister_extent` without allocating extra memory.
  - `lookup_arena(addr)` aligns `addr` down to the page boundary and returns the owning `Arena` when present.

## Coding Rules

**Naming and API Design:**
- Use descriptive lifetime and generic names, never meaningless ones like `'a`
- Minimize `pub` visibility - only expose what's necessary
- Keep function names concise - avoid chaining full sentences (e.g. avoid `get_user_name_from_id` or `allocate_segment_from_active_arena`)
- Don't describe obvious context in names (e.g. no need for `from_active_arena` since we wouldn't allocate from inactive ones)

**Safety and Error Handling:**
- Methods using `unsafe` that aren't fully guaranteed safe **MUST** be marked `unsafe`
- Avoid `panic!` except for truly unrecoverable errors - bubble up errors instead
- For raw pointers, prefer `NonNull<T>` over `*mut T` or `*const T` when possible
- Generally prefer references over raw pointers (though raw pointers may be necessary for allocator implementation)

**Performance and Constraints:**
- Mark methods as `const` when possible (constructors, getters, etc.)
- Use `#![no_std]` - only `core` crate allowed, no `std` or `alloc`
- This is **THE** allocator - no fallback to libc malloc, global alloc, or system alloc
- **Critical:** Avoid `String`, `Vec`, or formatting after allocator failure (causes infinite loops)

**Code Quality:**
- **NEVER ignore Clippy warnings** - address ALL warnings, even trivial ones
- Follow Clippy rules: max 7 function arguments, max **25** lines per function, no unnecessary Box/Vec usage (configured in clippy.toml)
- Use rustfmt with project settings: 2-space tabs, 100 char width, vertical imports (rustfmt.toml)
- Fix bugs in implementation, never work around them by changing tests or examples
- Use Rust ergonomics: Option/Result combinators, iterators, closures, pattern matching, From/TryFrom, Into/TryInto, Deref/DerefMut, Index/IndexMut
