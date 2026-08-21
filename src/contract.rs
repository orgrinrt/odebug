//! Where an entry goes, as a contract rather than a decision.
//!
//! This crate writes to files because that is what a procedural macro needs: it runs inside
//! the compiler, where `println!` goes somewhere nobody is reading. That is a good default
//! and a poor requirement. A crate without `std` has no files to write to, and one on a
//! microcontroller has a serial port, a ring buffer in RAM, or a semihosting channel, and
//! none of those is a path.
//!
//! So the destination is a [`Sink`], the file writer is one implementation of it, and a
//! consumer that has somewhere else to put an entry says so once.
//!
//! An entry arrives as [`core::fmt::Arguments`] rather than as a formatted string, which is
//! what `format_args!` produces and what costs nothing to build. A sink with somewhere to
//! write formats straight into it; a sink with an allocator may build a `String` if it
//! prefers. Neither choice is made here.

use core::fmt;
use core::sync::atomic::{AtomicPtr, Ordering};

/// Where a debug entry goes.
///
/// One method, because there is one thing to do with an entry. What a sink does with the
/// four pieces is its business: the file writer separates entries with a rule and prefixes
/// each with its origin, and a sink writing to a serial port may reasonably drop everything
/// but the content.
pub trait Sink: Sync {
    /// Writes one entry.
    ///
    /// `target` is what the call named, which the file writer treats as a filename and
    /// another sink may treat as a channel, a topic, or nothing at all. `header` is the
    /// optional label the call gave, and `origin` is the file and line it came from.
    ///
    /// Returns `Err` when the entry could not be written. The macro reports a failure and
    /// carries on, because a debug log that halts the program it is debugging has stopped
    /// being a debug log.
    fn write_entry(
        &self,
        target: &str,
        content: fmt::Arguments<'_>,
        header: Option<&str>,
        origin: fmt::Arguments<'_>,
    ) -> Result<(), Error>;

    /// Flushes whatever is held, if anything is.
    ///
    /// The default does nothing, which is right for a sink that writes through.
    fn flush(&self) {}
}

/// What a sink could not do.
///
/// Deliberately without a payload. A debug log's failure is reported and moved past, so
/// nothing downstream branches on why, and carrying an `io::Error` here would put `std` in
/// the contract that exists to be free of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the debug entry could not be written")
    }
}

/// A thin pointer to a `&'static dyn Sink`, or null when nothing has been installed.
///
/// A pointer to the reference rather than the reference itself, because a `&dyn Sink` is two
/// words and an `AtomicPtr` holds one. Splitting it across two atomics would work and would
/// need an argument about which half is read first and a transmute the language does not
/// formally guarantee; a thin pointer to a static holding the pair needs neither.
///
/// [`install_sink!`] declares that holder, which is why the caller never sees it.
static SINK: AtomicPtr<&'static dyn Sink> = AtomicPtr::new(core::ptr::null_mut());

/// Installs the sink every entry goes to, replacing whatever was there.
///
/// ```
/// use core::fmt;
/// use odebug::{install_sink, Error, Sink};
///
/// struct Discard;
///
/// impl Sink for Discard {
///     fn write_entry(
///         &self,
///         _target: &str,
///         _content: fmt::Arguments<'_>,
///         _header: Option<&str>,
///         _origin: fmt::Arguments<'_>,
///     ) -> Result<(), Error> {
///         Ok(())
///     }
/// }
///
/// install_sink!(Discard);
/// assert!(odebug::sink().is_some());
/// ```
///
/// The argument is a value, not a reference: the macro declares the static that holds it,
/// so the value has to be constructible in a `static` initialiser. A sink that needs
/// something at runtime holds it behind whatever interior mutability suits, which is the
/// same shape the file writer here uses.
#[macro_export]
macro_rules! install_sink {
    ($sink:expr) => {{
        static __ODEBUG_SINK_VALUE: &'static dyn $crate::Sink = &$sink;
        $crate::install_sink_ref(&__ODEBUG_SINK_VALUE);
    }};
}

/// Installs a sink from a reference to a reference. Called by [`install_sink!`].
///
/// Public because the macro expands in the caller's crate and has to name it. There is
/// nothing wrong with calling it directly if you already have the holder.
pub fn install_sink_ref(holder: &'static &'static dyn Sink) {
    SINK.store(
        // Casting away the shared reference to store it. Nothing ever writes through the
        // resulting pointer; `sink` reads it back as shared.
        core::ptr::from_ref(holder).cast_mut(),
        Ordering::Release,
    );
}

/// The installed sink, or `None` when nothing has been installed.
#[must_use]
pub fn sink() -> Option<&'static dyn Sink> {
    let holder = SINK.load(Ordering::Acquire);
    if holder.is_null() {
        return None;
    }

    // SAFETY: the pointer came from a `&'static &'static dyn Sink` in `install_sink_ref`,
    // which is the only writer, so it is aligned, initialised, and valid forever. The read
    // is of a shared reference, and nothing writes through the pointer.
    Some(*unsafe { &*holder })
}

/// Writes one entry to the installed sink.
///
/// The single place the macro reaches a sink. Returns `Err` when there is no sink or when
/// the sink refused, and the macro reports either and carries on.
///
/// # Errors
///
/// When no sink is installed, or the installed one could not write the entry.
pub fn emit(
    target: &str,
    content: fmt::Arguments<'_>,
    header: Option<&str>,
    origin: fmt::Arguments<'_>,
) -> Result<(), Error> {
    sink().ok_or(Error)?.write_entry(target, content, header, origin)
}
