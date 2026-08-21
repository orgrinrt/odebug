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
//! [`Sink`] is not a contract this crate invents. Receiving an item is
//! [`notko::sink::Emit`], which is where a stack-wide contract belongs and where anything
//! else needing one will look. What this crate adds on top of it is [`Entry`], which names
//! the four pieces of a debug entry, and the global install below.
//!
//! An entry arrives as [`core::fmt::Arguments`] rather than as a formatted string, which is
//! what `format_args!` produces and what costs nothing to build. A sink with somewhere to
//! write formats straight into it; a sink with an allocator may build a `String` if it
//! prefers. Neither choice is made here.

use core::fmt;
use core::sync::atomic::{AtomicPtr, Ordering};

use notko::sink::Emit;

/// One debug entry, as a value.
///
/// Four pieces rather than a formatted line, because a sink knows better than this crate what
/// to do with them: the file writer treats `target` as a filename and separates entries with
/// a rule, and one writing to a serial port may reasonably keep `content` and drop the rest.
///
/// `content` and `origin` are [`core::fmt::Arguments`] rather than strings, which is what
/// `format_args!` produces and what costs nothing to build. A sink with somewhere to write
/// formats straight into it; one with an allocator may build a `String` if it prefers.
/// Neither choice is made here, and no entry allocates on the way.
#[derive(Clone, Copy)]
pub struct Entry<'a> {
    /// What the call named. A filename to the file writer, a channel or a topic elsewhere.
    pub target: &'a str,
    /// The entry itself.
    pub content: fmt::Arguments<'a>,
    /// The optional label the call gave.
    pub header: Option<&'a str>,
    /// The file and line the call came from.
    pub origin: fmt::Arguments<'a>,
}

/// Where a debug entry goes.
///
/// A [`notko::sink::Emit`] of [`Entry`] that reports [`Error`], plus a flush. The `Emit` is
/// where the work is; this trait exists so the global holder below has one name to store, and
/// so `flush` has somewhere to live.
///
/// Implementing it is one empty line, `impl Sink for MySink {}`, unless the sink holds
/// something and wants `flush` to mean something. A blanket impl would save that line and
/// take `flush` away with it, since a type cannot override a method of an impl it did not
/// write.
pub trait Sink: for<'a> Emit<Entry<'a>, Err = Error> + Sync {
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
/// use odebug::{install_sink, Emit, Entry, Error, Outcome};
///
/// struct Discard;
///
/// impl Emit<Entry<'_>> for Discard {
///     type Err = Error;
///
///     fn emit(&self, _entry: Entry<'_>) -> Outcome<(), Self::Err> {
///         Outcome::Ok(())
///     }
/// }
///
/// impl odebug::Sink for Discard {}
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
    let entry = Entry {
        target,
        content,
        header,
        origin,
    };
    // `Outcome` is notko's, and this crate's own surface reports `Result`, because a consumer
    // implementing a sink meets `Outcome` there and a consumer calling the macro meets
    // neither. Converting here keeps notko out of the macro's expansion.
    match sink().ok_or(Error)?.emit(entry) {
        notko::Outcome::Ok(()) => Ok(()),
        notko::Outcome::Err(error) => Err(error),
    }
}
