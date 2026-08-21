#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", "README.md"))]

mod macros;
mod sink;

pub use sink::{debug_dir, flush, write_to_debug_file};

#[cfg(test)]
mod tests;
