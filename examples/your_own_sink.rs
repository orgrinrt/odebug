//! Putting entries somewhere other than a file, by implementing the contract.
//!
//! The file writer is a good default and a poor requirement. A crate without `std` has no
//! files to write to, one on a microcontroller has a serial port or a ring buffer in RAM, and
//! a test wants the entries in memory where it can assert on them.
//!
//! So the destination is a contract. It is [`notko::sink::Emit`], re-exported here, rather
//! than anything this crate invented: receiving an item is a stack-wide shape, and a crate
//! that writes its own is a crate nobody else's code composes with.
//!
//! Two things make it that shape rather than the more familiar `Push`:
//!
//! - `&self`, because the sink is installed once and reached from anywhere, so nobody holds
//!   it exclusively and `&mut self` is not on offer.
//! - Fallible, because a write fails for reasons the caller neither caused nor can act on.
//!
//! An entry arrives as [`odebug::Entry`], whose `content` and `origin` are
//! `core::fmt::Arguments`. That is what `format_args!` produces and it costs nothing to
//! build: the sink below formats straight into its own buffer, and no `String` exists at any
//! point on the way. A sink with an allocator may build one if it prefers, which the file
//! writer does.

use std::sync::Mutex;

use odebug::{odebug, Emit, Entry, Error, Outcome, Sink};

/// Keeps every entry in memory, formatted.
///
/// A `Mutex` because `emit` takes `&self` and this one has something to change. That is the
/// cost of the choice, and it falls on the implementor that made it rather than on every
/// implementor: a sink writing to a port that needs no state pays none of it.
struct InMemory {
    entries: Mutex<Vec<String>>,
}

impl InMemory {
    const fn new() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
        }
    }

    fn take(&self) -> Vec<String> {
        core::mem::take(&mut self.entries.lock().expect("no panic held the lock"))
    }
}

impl Emit<Entry<'_>> for InMemory {
    type Err = Error;

    fn emit(&self, entry: Entry<'_>) -> Outcome<(), Self::Err> {
        // `header` is what the call labelled the entry, and `target` is the file the default
        // writer would have used. A sink decides what either means; this one puts both in the
        // line and a serial-port sink might reasonably keep neither.
        let label = entry.header.unwrap_or("-");
        let line = format!("[{}][{}] {} at {}", entry.target, label, entry.content, entry.origin);

        match self.entries.lock() {
            Ok(mut entries) => {
                entries.push(line);
                Outcome::Ok(())
            },
            // A poisoned lock means a previous entry's thread panicked mid-push. Reporting
            // the refusal is the contract; deciding it is fatal is not this sink's call.
            Err(_) => Outcome::Err(Error),
        }
    }
}

// One empty line, because the `Emit` above is the whole implementation. It is deliberately not
// blanket-implemented: a type cannot override a method of an impl it did not write, and
// `flush` is exactly what a sink holding something wants to override.
impl Sink for InMemory {}

static SINK: InMemory = InMemory::new();

fn main() {
    // `install_sink!` takes a value and declares the static itself. This one already has a
    // static, so it installs the holder directly, which is what the macro expands to.
    static HOLDER: &dyn Sink = &SINK;
    odebug::install_sink_ref(&HOLDER);

    odebug!("the first entry");
    odebug!(parser::Tokens("{} of them", 12));
    odebug!("a line".to_file("codegen.log").with_header("NOTE"));

    println!("the sink received:");
    for entry in SINK.take() {
        println!("  {entry}");
    }

    // Nothing reached a file. Installing a sink replaces the writer rather than adding to it,
    // and the default one is only installed when nothing else has been.
    println!();
    println!("and .debug/ is untouched by any of the three above");
}
