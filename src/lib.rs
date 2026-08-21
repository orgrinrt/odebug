#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", "README.md"))]
#![cfg_attr(feature = "no_std", no_std)]

// The destination is a contract, and the file writer is one implementation of it. See
// `contract` for why: this crate writes to files because a procedural macro runs inside the
// compiler, where nobody reads stdout, and that is a good default and a poor requirement.
mod contract;
mod macros;
#[cfg(not(feature = "no_std"))]
mod sink;

pub use contract::{emit, install_sink_ref, sink, Error, Sink};

/// Installs the default sink, where there is one.
///
/// Under `no_std` there is not: there are no files to write to, so a consumer installs
/// whatever it has and this does nothing. The macro calls it unconditionally, because a
/// `#[cfg]` in a macro body is evaluated in the crate the macro expands into and would
/// read that crate's features rather than this one's.
#[cfg(feature = "no_std")]
#[doc(hidden)]
#[inline]
pub fn install_default_sink() {}
#[cfg(not(feature = "no_std"))]
pub use sink::{debug_dir, flush, install_default_sink, write_to_debug_file, FileSink};

#[cfg(all(test, not(feature = "no_std")))]
mod tests;
