//! An entry reaches a sink the consumer wrote, with nothing allocated on the way.
//!
//! The file writer is one destination and this crate's default, not its contract. What a
//! `#![no_std]` consumer has is a serial port, a ring buffer in RAM, or a semihosting
//! channel, and none of those is a path, so the destination is a `Sink` and the file writer
//! is one implementation of it.
//!
//! The sink here writes into a fixed array, which is what "nothing allocated" looks like
//! when you have to prove it rather than claim it: an entry that went through a `String`
//! anywhere could not land in this.

use core::cell::UnsafeCell;
use core::fmt::{self, Write};
use core::sync::atomic::{AtomicUsize, Ordering};

use odebug::{odebug, Emit, Entry, Error, Outcome, Sink};

/// How much of an entry this keeps.
const CAPACITY: usize = 512;

/// A sink that writes into a fixed buffer and nowhere else.
///
/// Deliberately awkward in exactly the way an embedded one is: no allocator, no `std`, and
/// a destination whose size was decided before the program ran.
struct Buffer {
    /// The bytes, behind a cell because `write_entry` takes `&self`.
    ///
    /// A real one would use a critical section or a lock; this test is single-threaded and
    /// says so rather than pretending the cell is enough.
    bytes: UnsafeCell<[u8; CAPACITY]>,
    /// How much has been written.
    used:  AtomicUsize,
}

// SAFETY: this test drives the sink from one thread. `Sink` requires `Sync` because a sink
// is installed globally and could be reached from anywhere; nothing here does.
unsafe impl Sync for Buffer {}

impl Buffer {
    const fn new() -> Self {
        Self { bytes: UnsafeCell::new([0; CAPACITY]), used: AtomicUsize::new(0) }
    }

    /// What has been written so far.
    fn contents(&self) -> String {
        let used = self.used.load(Ordering::Acquire);
        // SAFETY: single-threaded, as above, and `used` never exceeds `CAPACITY`.
        let bytes = unsafe { &*self.bytes.get() };
        String::from_utf8_lossy(&bytes[.. used]).into_owned()
    }

    fn clear(&self) {
        self.used.store(0, Ordering::Release);
    }
}

/// The `core::fmt::Write` half, which is how an `Arguments` gets rendered without a heap.
struct Cursor<'a>(&'a Buffer);

impl Write for Cursor<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let used = self.0.used.load(Ordering::Acquire);
        let end = (used + s.len()).min(CAPACITY);
        if end > used {
            // SAFETY: single-threaded, and `end` is clamped to the capacity.
            let bytes = unsafe { &mut *self.0.bytes.get() };
            bytes[used .. end].copy_from_slice(&s.as_bytes()[.. end - used]);
            self.0.used.store(end, Ordering::Release);
        }
        Ok(())
    }
}

impl Emit<Entry<'_>> for Buffer {
    type Err = Error;

    fn emit(&self, entry: Entry<'_>) -> Outcome<(), Self::Err> {
        let mut cursor = Cursor(self);

        // Formatted straight into the buffer. The entry arrived carrying `Arguments`, so
        // there is no intermediate string anywhere: this is the whole reason `Entry` holds
        // `Arguments` rather than a `&str`.
        let written = (|| {
            write!(cursor, "[{}]", entry.target)?;
            if let Some(header) = entry.header {
                write!(cursor, "[{header}]")?;
            }
            writeln!(cursor, " {} ({})", entry.content, entry.origin)
        })();

        match written {
            Ok(()) => Outcome::Ok(()),
            Err(_) => Outcome::Err(Error),
        }
    }
}

// One empty line, because `Emit` above is where the work is. It is not blanket-implemented:
// a type cannot override a method of an impl it did not write, and `flush` is a method a sink
// holding something wants to override.
impl Sink for Buffer {}

static BUFFER: Buffer = Buffer::new();

/// One at a time.
///
/// The sink is installed globally, which is what a debug log is, so tests sharing it cannot
/// run at once: the second would clear the buffer under the first and both would read a
/// mixture. Found by writing them without this and watching entries from one test appear in
/// another's assertion.
static ONE_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Installs the buffer and clears it, so each test starts from nothing.
///
/// Returns the guard that keeps the other tests out, which the caller holds for the length
/// of its own body.
fn install() -> std::sync::MutexGuard<'static, ()> {
    let guard = ONE_AT_A_TIME.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    static HOLDER: &dyn Sink = &BUFFER;
    odebug::install_sink_ref(&HOLDER);
    BUFFER.clear();
    guard
}

#[test]
fn an_entry_reaches_a_sink_the_consumer_wrote() {
    let _guard = install();

    odebug!("a plain message");

    let written = BUFFER.contents();
    assert!(written.contains("a plain message"), "the entry did not arrive:\n{written}");
    assert!(written.contains("[debug.log]"), "the target did not arrive:\n{written}");
    assert!(written.contains("no_std_sink.rs"), "the origin did not arrive:\n{written}");
}

#[test]
fn every_form_reaches_it_too() {
    let _guard = install();

    // The forms differ only in which target and header they choose, so all of them have to
    // arrive at the same sink. A form still calling the file writer directly would write a
    // file and leave this buffer empty.
    odebug!("formatted: {} and {}", 1, 2);
    odebug!(::Header("with a header"));
    odebug!(parser::("to another target"));
    odebug!("out.log" => "to a named target");

    let written = BUFFER.contents();
    for expected in [
        "formatted: 1 and 2",
        "[Header] with a header",
        "[parser.log]",
        "to another target",
        "[out.log]",
        "to a named target",
    ] {
        assert!(written.contains(expected), "missing {expected:?} in:\n{written}");
    }
}

#[test]
fn installing_a_sink_replaces_whatever_was_there() {
    let _guard = install();

    // The file writer installs itself on the first entry unless something is already
    // installed, so a consumer that installs its own keeps it. Without that, the first
    // `odebug!` would silently take the destination back.
    odebug!("first");
    let after_first = BUFFER.contents();
    assert!(after_first.contains("first"), "the consumer's sink was displaced:\n{after_first}");

    odebug!("second");
    let after_second = BUFFER.contents();
    assert!(after_second.contains("first"), "the buffer was reset:\n{after_second}");
    assert!(after_second.contains("second"), "the second entry did not arrive:\n{after_second}");
}

#[test]
fn the_entry_carries_the_line_it_came_from() {
    let _guard = install();

    // Every entry is prefixed with its origin, which is the point of a debug log written
    // from inside a compiler: without it an entry says what happened and not where.
    // The next line, which is where the call is.
    let line = line!() + 1;
    odebug!("locate me");

    let written = BUFFER.contents();
    assert!(
        written.contains(&format!("no_std_sink.rs:{line}")),
        "the origin is wrong or missing, expected line {line}:\n{written}",
    );
}
