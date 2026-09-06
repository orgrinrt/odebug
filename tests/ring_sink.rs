//! `Ring` holds the tail of the log in a fixed array, formatted in place.
//!
//! The claims: an entry lands in it whole and in the file writer's shape, the oldest bytes
//! go first once it is full, reading does not consume, and several threads writing at once
//! neither interleave inside a line nor lose one. The sink is used both installed, through
//! the macro, and directly, through `Emit`, because the two reach it by different routes.

use odebug::{odebug, Emit, Entry, Outcome, Ring};

/// Builds an entry the way the macro does, for driving a ring directly.
fn emit_into<const N: usize>(ring: &Ring<N>, target: &str, header: Option<&str>, content: &str) {
    let entry = Entry {
        target,
        content: format_args!("{content}"),
        header,
        origin: format_args!("{}:{}", file!(), line!()),
    };
    assert!(matches!(ring.emit(entry), Outcome::Ok(())));
}

fn text<const N: usize>(ring: &Ring<N>) -> String {
    let mut out = [0u8; 1 << 16];
    let n = ring.read_into(&mut out);
    String::from_utf8_lossy(&out[..n]).into_owned()
}

#[test]
fn a_fresh_ring_holds_nothing() {
    let ring: Ring<64> = Ring::new();
    assert!(ring.is_empty());
    assert_eq!(ring.len(), 0);
    assert_eq!(ring.capacity(), 64);
    let mut out = [0u8; 8];
    assert_eq!(ring.read_into(&mut out), 0);
}

#[test]
fn an_entry_lands_whole_and_in_shape() {
    let ring: Ring<256> = Ring::new();
    emit_into(&ring, "parser.log", Some("Tokens"), "12 of them");
    let held = text(&ring);
    assert!(
        held.starts_with("[parser.log][Tokens] 12 of them ("),
        "{held:?}"
    );
    assert!(
        held.contains("ring_sink.rs:"),
        "the origin is in the line: {held:?}"
    );
    assert!(held.ends_with(")\n"), "one line, terminated: {held:?}");
    assert_eq!(ring.len(), held.len());
}

#[test]
fn an_entry_without_a_header_has_no_header_bracket() {
    let ring: Ring<256> = Ring::new();
    emit_into(&ring, "debug.log", None, "plain");
    let held = text(&ring);
    assert!(held.starts_with("[debug.log] plain ("), "{held:?}");
}

#[test]
fn reading_does_not_consume() {
    let ring: Ring<256> = Ring::new();
    emit_into(&ring, "debug.log", None, "kept");
    assert_eq!(text(&ring), text(&ring));
    assert!(!ring.is_empty());
}

#[test]
fn a_short_buffer_gets_the_oldest_bytes() {
    let ring: Ring<256> = Ring::new();
    emit_into(&ring, "debug.log", None, "abcdef");
    let mut out = [0u8; 5];
    assert_eq!(ring.read_into(&mut out), 5);
    assert_eq!(&out, b"[debu");
}

#[test]
fn once_full_the_oldest_bytes_go_first() {
    // Small enough that one entry does not fit, so what survives is the tail of it.
    let ring: Ring<16> = Ring::new();
    emit_into(&ring, "debug.log", None, "0123456789");
    let held = text(&ring);
    assert_eq!(ring.len(), 16, "full, and no fuller");
    assert!(
        held.ends_with(")\n"),
        "the end of the entry is what survives: {held:?}"
    );
    assert!(
        !held.starts_with("[debug"),
        "the start went first: {held:?}"
    );

    // A second entry pushes the first out entirely, and the ring wraps around its array
    // on the way, which is where a copy in two segments earns its keep.
    emit_into(&ring, "x.log", None, "second");
    let held = text(&ring);
    assert_eq!(ring.len(), 16);
    assert!(held.ends_with(")\n"), "{held:?}");
    assert!(
        !held.contains("0123456789"),
        "the first entry was pushed out: {held:?}"
    );
}

#[test]
fn a_write_longer_than_the_ring_keeps_its_last_bytes() {
    let ring: Ring<8> = Ring::new();
    emit_into(
        &ring,
        "t",
        None,
        "the quick brown fox jumps over the lazy dog",
    );
    assert_eq!(ring.len(), 8);
    let held = text(&ring);
    assert!(held.ends_with(")\n"), "{held:?}");
}

#[test]
fn clear_forgets_everything_and_the_ring_is_usable_after() {
    let ring: Ring<64> = Ring::new();
    emit_into(&ring, "t", None, "before");
    ring.clear();
    assert!(ring.is_empty());
    emit_into(&ring, "t", None, "after");
    let held = text(&ring);
    assert!(
        held.contains("after") && !held.contains("before"),
        "{held:?}"
    );
}

#[test]
fn a_zero_capacity_ring_accepts_entries_and_holds_none() {
    // The degenerate size, which the arithmetic has to survive rather than divide by.
    let ring: Ring<0> = Ring::new();
    emit_into(&ring, "t", None, "dropped");
    assert!(ring.is_empty());
    let mut out = [0u8; 4];
    assert_eq!(ring.read_into(&mut out), 0);
}

#[test]
#[cfg(not(feature = "no_std"))]
fn contents_reads_the_same_bytes_as_read_into() {
    let ring: Ring<128> = Ring::new();
    emit_into(&ring, "a.log", Some("H"), "one");
    emit_into(&ring, "b.log", None, "two");
    assert_eq!(ring.contents(), text(&ring));
}

#[test]
fn threads_writing_at_once_keep_every_line_whole() {
    // Big enough for every line, since the point is that none is lost or torn, and a ring
    // that filled would drop the oldest by design rather than by fault.
    static RING: Ring<32768> = Ring::new();
    std::thread::scope(|s| {
        for t in 0..8u8 {
            s.spawn(move || {
                for i in 0..20 {
                    emit_into(&RING, "t.log", None, &format!("thread {t} line {i} end"));
                }
            });
        }
    });
    let held = text(&RING);
    let lines: Vec<&str> = held.lines().collect();
    assert_eq!(lines.len(), 160, "every line arrived:\n{held}");
    for line in &lines {
        assert!(
            line.starts_with("[t.log] thread ") && line.contains(" end ("),
            "a line was interleaved with another: {line:?}"
        );
    }
}

/// The macro reaches an installed ring like any other sink.
///
/// This test is last in the file by name and holds the only installation, since a sink is
/// global to the process and the other tests here drive their rings directly.
#[test]
fn z_the_macro_reaches_an_installed_ring() {
    static LOG: Ring<512> = Ring::new();
    static HOLDER: &dyn odebug::Sink = &LOG;
    odebug::install_sink_ref(&HOLDER);

    odebug!("through the macro");
    odebug!(parser::Tokens("{} of them", 12));

    let held = text(&LOG);
    assert!(held.contains("[debug.log] through the macro ("), "{held}");
    assert!(held.contains("[parser.log][Tokens] 12 of them ("), "{held}");
}
