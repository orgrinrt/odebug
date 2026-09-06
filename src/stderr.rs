//! A sink that writes to stderr, for the programs that have one.
//!
//! The file writer exists because a procedural macro's stderr goes somewhere nobody reads.
//! A build script, a test, or an ordinary binary has a stderr somebody does read, and an
//! entry there arrives as it happens rather than in a file opened afterwards. Same shape
//! on the line as the file writer gives it, so the two read alike.

use std::io::Write;

use notko::sink::Emit;

use crate::{Entry, Error, Sink};

/// Writes every entry to stderr, in the file writer's shape.
///
/// ```
/// use odebug::{install_sink, odebug, Stderr};
///
/// install_sink!(Stderr);
/// odebug!(::Note("this goes to stderr rather than to a file"));
/// ```
///
/// Stderr is locked for the length of one entry, so entries from several threads do not
/// interleave inside a line. The `target` is written into the header line, since there is
/// no file for it to name.
#[derive(Debug, Clone, Copy, Default)]
pub struct Stderr;

impl Emit<Entry<'_>> for Stderr {
    type Err = Error;

    fn emit(&self, entry: Entry<'_>) -> notko::Outcome<(), Self::Err> {
        let stderr = std::io::stderr();
        let mut lock = stderr.lock();
        let written = crate::sink::write_shaped(
            &mut lock,
            Some(entry.target),
            entry.header,
            Some(entry.origin),
            entry.content,
        );
        match written {
            Ok(()) => notko::Outcome::Ok(()),
            Err(_) => notko::Outcome::Err(Error),
        }
    }
}

impl Sink for Stderr {
    fn flush(&self) {
        let _ = std::io::stderr().flush();
    }
}
