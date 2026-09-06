//! A sink that keeps the last so many bytes of what it was given, in memory, without `std`.
//!
//! What a consumer with no filesystem has is a fixed amount of RAM, so this is the sink for
//! it: a ring of `N` bytes, an entry formatted straight into the ring, and the oldest bytes
//! overwritten once it is full. It is also what a test wants, since the entries are right
//! there to assert on rather than in a file somewhere.
//!
//! The formatting goes through [`core::fmt::Write`] into the ring itself, so no `String`
//! exists at any point on the way, which is the property [`Entry`] carrying
//! [`core::fmt::Arguments`] is for.

use core::cell::UnsafeCell;
use core::fmt::{self, Write};
use core::sync::atomic::{AtomicBool, Ordering};

use notko::sink::Emit;

use crate::{Entry, Error, Sink};

/// The bytes, and where the next one goes.
struct State<const N: usize> {
    bytes: [u8; N],
    /// Where the next byte is written.
    head: usize,
    /// How many bytes are held, which is at most `N`.
    held: usize,
}

impl<const N: usize> State<N> {
    /// Where the oldest held byte sits.
    ///
    /// A zero-capacity ring holds nothing and has nowhere for it to sit, and the modulo
    /// would divide by zero saying so.
    const fn tail(&self) -> usize {
        if N == 0 {
            0
        } else {
            (self.head + N - self.held) % N
        }
    }

    /// Appends bytes, keeping the last `N` of what has ever been written.
    fn push(&mut self, mut bytes: &[u8]) {
        if N == 0 {
            return;
        }
        // Only the last `N` bytes of an oversized write can survive, so the rest is not
        // copied only to be overwritten in the same call.
        if bytes.len() > N {
            bytes = &bytes[bytes.len() - N..];
        }
        // At most two segments: up to the end of the array, and from its start.
        let first = (N - self.head).min(bytes.len());
        self.bytes[self.head..self.head + first].copy_from_slice(&bytes[..first]);
        let rest = bytes.len() - first;
        self.bytes[..rest].copy_from_slice(&bytes[first..]);
        self.head = (self.head + bytes.len()) % N;
        self.held = (self.held + bytes.len()).min(N);
    }
}

impl<const N: usize> Write for State<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push(s.as_bytes());
        Ok(())
    }
}

/// A sink holding the last `N` bytes of what it was given.
///
/// Each entry is written as `[target][header] content (origin)` on a line of its own, the
/// header bracket left out where the call gave none. Once the ring is full the oldest bytes
/// go first, so what it holds is the tail of the log rather than its head, which is the
/// half a crash is explained by.
///
/// It is `const`-constructible, so it sits in a `static` and [`install_sink!`](crate::install_sink)
/// takes it directly. A spin lock guards it, since a sink is reached from every thread at
/// once and `core` has nothing that parks; the critical section is one entry's formatting.
///
/// ```
/// use odebug::{install_sink, odebug, Ring};
///
/// static LOG: odebug::Ring<256> = odebug::Ring::new();
///
/// static HOLDER: &dyn odebug::Sink = &LOG;
/// odebug::install_sink_ref(&HOLDER);
///
/// odebug!(parser::Tokens("{} of them", 12));
///
/// let mut out = [0u8; 256];
/// let n = LOG.read_into(&mut out);
/// let text = core::str::from_utf8(&out[.. n]).unwrap();
/// assert!(text.contains("[parser.log][Tokens] 12 of them"));
/// ```
pub struct Ring<const N: usize> {
    busy: AtomicBool,
    state: UnsafeCell<State<N>>,
}

// SAFETY: the state is only ever reached through `with`, which holds `busy` for the
// duration, so no two threads hold a reference into it at once.
unsafe impl<const N: usize> Sync for Ring<N> {}

/// Releases the lock on the way out, panic or not.
struct Release<'a>(&'a AtomicBool);

impl Drop for Release<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl<const N: usize> Default for Ring<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Ring<N> {
    /// An empty ring.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            busy: AtomicBool::new(false),
            state: UnsafeCell::new(State {
                bytes: [0; N],
                head: 0,
                held: 0,
            }),
        }
    }

    /// How many bytes this can hold, which is `N`.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        N
    }

    fn with<R>(&self, f: impl FnOnce(&mut State<N>) -> R) -> R {
        while self
            .busy
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            core::hint::spin_loop();
        }
        let _release = Release(&self.busy);
        // SAFETY: `busy` is held until `_release` drops, so this is the only reference into
        // the state for as long as it lives.
        f(unsafe { &mut *self.state.get() })
    }

    /// How many bytes are held, which is at most the capacity.
    #[must_use]
    pub fn len(&self) -> usize {
        self.with(|state| state.held)
    }

    /// Whether nothing has been written since the last clear.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Forgets everything held.
    pub fn clear(&self) {
        self.with(|state| {
            state.head = 0;
            state.held = 0;
        });
    }

    /// Copies what is held into `out`, oldest byte first, and returns how many bytes that
    /// was.
    ///
    /// An `out` shorter than what is held gets the oldest bytes and the rest stay in the
    /// ring; nothing is consumed by reading. An entry that was cut by the ring filling can
    /// start mid-character, so `core::str::from_utf8` on the result is not guaranteed to
    /// succeed at the very start; `from_utf8_lossy` where `std` is available, or skipping
    /// to the first newline, are the two usual answers.
    pub fn read_into(&self, out: &mut [u8]) -> usize {
        self.with(|state| {
            let count = state.held.min(out.len());
            let tail = state.tail();
            let first = (N - tail).min(count);
            out[..first].copy_from_slice(&state.bytes[tail..tail + first]);
            out[first..count].copy_from_slice(&state.bytes[..count - first]);
            count
        })
    }

    /// What is held, as text, with anything a cut entry left undecodable replaced.
    #[cfg(not(feature = "no_std"))]
    #[must_use]
    pub fn contents(&self) -> std::string::String {
        self.with(|state| {
            let tail = state.tail();
            let mut bytes = std::vec::Vec::with_capacity(state.held);
            let first = (N - tail).min(state.held);
            bytes.extend_from_slice(&state.bytes[tail..tail + first]);
            bytes.extend_from_slice(&state.bytes[..state.held - first]);
            std::string::String::from_utf8_lossy(&bytes).into_owned()
        })
    }
}

impl<const N: usize> Emit<Entry<'_>> for Ring<N> {
    type Err = Error;

    fn emit(&self, entry: Entry<'_>) -> notko::Outcome<(), Self::Err> {
        // Straight into the ring. `State`'s `write_str` cannot fail, so the `fmt::Result`
        // here only ever reports a `Display` impl in the entry refusing, which is the one
        // failure a sink with nowhere to fail has.
        let written = self.with(|state| {
            write!(state, "[{}]", entry.target)?;
            if let Some(header) = entry.header {
                write!(state, "[{header}]")?;
            }
            writeln!(state, " {} ({})", entry.content, entry.origin)
        });
        match written {
            Ok(()) => notko::Outcome::Ok(()),
            Err(_) => notko::Outcome::Err(Error),
        }
    }
}

impl<const N: usize> Sink for Ring<N> {}
