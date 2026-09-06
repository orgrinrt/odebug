//! Entries on stderr rather than in a file.
//!
//! A procedural macro's stderr goes somewhere nobody reads, which is why the file writer is
//! the default. A build script, a test or an ordinary binary has one somebody does read, and
//! `Stderr` puts the entries there as they happen, in the same shape the file writer gives
//! them so the two read alike.

#[cfg(not(feature = "no_std"))]
use odebug::{install_sink, odebug, Stderr};

// The sink writes through `std::io::stderr`, which `no_std` has none of. cargo builds every
// example under every feature selection, so the gate is on `main`.
#[cfg(feature = "no_std")]
fn main() {
    println!("this example needs stderr, which `no_std` has none of");
}

#[cfg(not(feature = "no_std"))]
fn main() {
    install_sink!(Stderr);

    odebug!("a plain line");
    odebug!(parser::Tokens("{} of them", 12));
    odebug!("a line".to_file("codegen.log").with_header("NOTE"));

    // Installing a sink replaces the file writer rather than adding to it, so the debug
    // directory the writer would have used has nothing new in it from the three above.
    println!(
        "the three entries went to stderr, and nothing was written under {}",
        odebug::debug_dir().display()
    );
}
