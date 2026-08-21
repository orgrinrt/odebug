//! What a logged line costs.
//!
//! The crate is for logging during proc-macro expansion, where a build can emit
//! thousands of lines, so the per-line cost is the whole cost. The shipped path
//! creates the directory, opens the file, wraps it in a `BufWriter`, writes,
//! flushes and closes, once per line.
//!
//! The arms are the alternatives somebody would actually reach for:
//!
//! - `per_call_open` is what the crate shipped.
//! - `no_mkdir` is the same without the redundant `create_dir_all`, since the
//!   directory is already made when the path is first resolved.
//! - `cached_handle` keeps the file open and writes into it.
//! - `cached_buffered` keeps it open behind a buffer, flushed on drop.
//! - `shipped` is the crate's own `write_to_debug_file`, so the headline number is
//!   about the code that ships rather than about a lookalike written here.

// Every arm here writes to a file, so the whole bench needs a filesystem and there is none
// under `no_std`. cargo builds a bench under every feature selection and `required-features`
// names what a target needs rather than what it cannot have, so the gate is per item. The
// crate has no positive `std` feature for this to key on, which is the underlying awkwardness:
// a `std` default that `no_std` is the absence of would make this one attribute.
#[cfg(not(feature = "no_std"))]
use criterion::{criterion_group, criterion_main, Criterion, Throughput};
#[cfg(not(feature = "no_std"))]
use std::collections::HashMap;
#[cfg(not(feature = "no_std"))]
use std::fs::{self, File, OpenOptions};
#[cfg(not(feature = "no_std"))]
use std::hint::black_box;
#[cfg(not(feature = "no_std"))]
use std::io::{BufWriter, Write};
#[cfg(not(feature = "no_std"))]
use std::path::PathBuf;
#[cfg(not(feature = "no_std"))]
use std::sync::Mutex;

#[cfg(not(feature = "no_std"))]
fn dir() -> PathBuf {
    let d = std::env::temp_dir().join("odebug_bench");
    fs::create_dir_all(&d).expect("the bench directory");
    d
}

#[cfg(not(feature = "no_std"))]
fn per_call_open(path: &PathBuf, line: &str) {
    let _ = fs::create_dir_all(path.parent().expect("a parent"));
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("the log file");
    let mut w = BufWriter::new(file);
    writeln!(w, "{line}").expect("a write");
    w.flush().expect("a flush");
}

#[cfg(not(feature = "no_std"))]
fn no_mkdir(path: &PathBuf, line: &str) {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("the log file");
    let mut w = BufWriter::new(file);
    writeln!(w, "{line}").expect("a write");
    w.flush().expect("a flush");
}

#[cfg(not(feature = "no_std"))]
static PLAIN: Mutex<Option<HashMap<PathBuf, File>>> = Mutex::new(None);

#[cfg(not(feature = "no_std"))]
fn cached_handle(path: &PathBuf, line: &str) {
    let mut map = PLAIN.lock().expect("the handle map");
    let files = map.get_or_insert_with(HashMap::new);
    let file = files.entry(path.clone()).or_insert_with(|| {
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("the log file")
    });
    writeln!(file, "{line}").expect("a write");
}

#[cfg(not(feature = "no_std"))]
static BUFFERED: Mutex<Option<HashMap<PathBuf, BufWriter<File>>>> = Mutex::new(None);

#[cfg(not(feature = "no_std"))]
fn cached_buffered(path: &PathBuf, line: &str) {
    let mut map = BUFFERED.lock().expect("the writer map");
    let files = map.get_or_insert_with(HashMap::new);
    let w = files.entry(path.clone()).or_insert_with(|| {
        BufWriter::new(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .expect("the log file"),
        )
    });
    writeln!(w, "{line}").expect("a write");
}

/// The crate's own function.
///
/// It writes into the resolved debug directory rather than into the temporary one the
/// other arms use, so on a machine where those are different filesystems the comparison
/// carries that difference too. Naming it is better than implying the arms are equal in a
/// respect they are not.
#[cfg(not(feature = "no_std"))]
fn shipped(_path: &PathBuf, line: &str) {
    odebug::write_to_debug_file("bench_shipped.log", line, None, None).expect("a write");
}

#[cfg(not(feature = "no_std"))]
fn one_line(c: &mut Criterion) {
    let base = dir();
    let line = "a representative line of expanded tokens, about this long, give or take";
    let mut g = c.benchmark_group("one line");
    g.throughput(Throughput::Elements(1));
    for (name, f) in [
        ("per_call_open", per_call_open as fn(&PathBuf, &str)),
        ("no_mkdir", no_mkdir as fn(&PathBuf, &str)),
        ("cached_handle", cached_handle as fn(&PathBuf, &str)),
        ("cached_buffered", cached_buffered as fn(&PathBuf, &str)),
        ("shipped", shipped as fn(&PathBuf, &str)),
    ] {
        let path = base.join(format!("{name}.log"));
        let _ = fs::remove_file(&path);
        g.bench_function(name, |b| b.iter(|| f(black_box(&path), black_box(line))));
    }
    g.finish();
}

#[cfg(not(feature = "no_std"))]
criterion_group!(benches, one_line);
#[cfg(not(feature = "no_std"))]
criterion_main!(benches);

// criterion_main generates the entry point, and it is gated out above, so `no_std` needs one
// of its own. cargo refuses a bench target without a main whatever the feature selection.
#[cfg(feature = "no_std")]
fn main() {
    println!("these benches write to files, which `no_std` has none of");
}
