//! Every feature selection this crate offers, built, plus what `no_std` really removes.
//!
//! `no_std` here is not a cosmetic flag. It compiles the file writer out, which is the
//! crate's whole default behaviour, and leaves the macro and the `Sink` contract. A
//! selection that never gets built is a claim nobody checked, and this one is easy to break
//! from the `std` side without noticing.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Builds the crate under one feature selection.
fn check(features: &str) -> (bool, String) {
    let mut command = Command::new(env!("CARGO"));
    command
        .args(["check", "--quiet", "--no-default-features"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env(
            "CARGO_TARGET_DIR",
            concat!(env!("CARGO_MANIFEST_DIR"), "/target/feature-matrix"),
        );
    if !features.is_empty() {
        command.args(["--features", features]);
    }
    let output = command.output().expect("cargo runs");
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

/// Builds a throwaway crate whose body is `body`, against this crate at `features`.
fn consumer_compiles(name: &str, features: &str, attrs: &str, body: &str) -> (bool, String) {
    let root = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/target/consumers")).join(name);
    fs::create_dir_all(root.join("src")).expect("the consumer directory");

    let features_list = if features.is_empty() {
        String::new()
    } else {
        features
            .split(',')
            .map(|f| format!("\"{f}\""))
            .collect::<Vec<_>>()
            .join(", ")
    };

    fs::write(
        root.join("Cargo.toml"),
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n\
             [dependencies.odebug]\npath = \"{crate_dir}\"\ndefault-features = false\n\
             features = [{features_list}]\n\n[workspace]\n",
            crate_dir = env!("CARGO_MANIFEST_DIR"),
        ),
    )
    .expect("the consumer manifest");

    fs::write(
        root.join("src").join("lib.rs"),
        format!("{attrs}\n{body}\n"),
    )
    .expect("the consumer source");

    let output = Command::new(env!("CARGO"))
        .args(["check", "--quiet"])
        .current_dir(&root)
        .env(
            "CARGO_TARGET_DIR",
            concat!(env!("CARGO_MANIFEST_DIR"), "/target/consumers/target"),
        )
        .output()
        .expect("cargo runs");

    (
        output.status.success(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    )
}

#[test]
fn every_selection_builds() {
    for features in [
        "",
        "use_workspace",
        "output_to_target",
        "always_log",
        "buffered",
        "use_workspace,output_to_target,always_log,buffered",
        "no_std",
        "no_alloc",
        "no_alloc,no_std",
        // The flags that only say where the file writer puts its files, alongside the
        // selection that removes the file writer. A consumer whose workspace turns one on
        // everywhere lands here, and it must not be a build failure.
        "no_std,use_workspace",
        "no_alloc,output_to_target,buffered",
    ] {
        let (ok, err) = check(features);
        let label = if features.is_empty() { "no features" } else { features };
        assert!(ok, "{label} builds:\n{err}");
    }
}

/// A sink a `#![no_std]` consumer can actually write.
const CONSUMER_SINK: &str = r#"
// One crate, not two. The contract is `notko::sink::Emit`, and this crate re-exports it so a
// consumer writing a sink does not acquire a dependency on notko to spell it.
use odebug::{odebug, Emit, Entry, Error, Outcome, Sink};

pub struct Discard;

impl Emit<Entry<'_>> for Discard {
    type Err = Error;

    fn emit(&self, _entry: Entry<'_>) -> Outcome<(), Self::Err> {
        Outcome::Ok(())
    }
}

impl Sink for Discard {}

pub fn install() {
    static HOLDER: &dyn Sink = &Discard;
    odebug::install_sink_ref(&HOLDER);
}

pub fn log() {
    odebug!("a message");
    odebug!("formatted: {}", 1);
    odebug!(::Header("with a header"));
}
"#;

#[test]
fn a_no_std_consumer_can_install_a_sink_and_log_to_it() {
    // The thing the feature exists for, and the only place it can be seen: the macro
    // expands in the consumer's crate, so whether what it expands to needs `std` is a
    // question about that crate rather than this one.
    let (ok, err) = consumer_compiles("no_std_consumer", "no_alloc", "#![no_std]", CONSUMER_SINK);
    assert!(
        ok,
        "a `#![no_std]` consumer can log through its own sink:\n{err}"
    );
}

#[test]
fn that_consumer_really_is_without_std() {
    // The control. Without it the test above would pass just as well against a consumer
    // where `#![no_std]` did nothing at all.
    let (ok, err) = consumer_compiles(
        "no_std_control",
        "no_alloc",
        "#![no_std]",
        "pub fn reaches() { let _ = std::vec::Vec::<u8>::new(); }",
    );
    assert!(!ok, "a `#![no_std]` consumer must not reach `std::vec`");
    assert!(
        err.contains("std"),
        "the error is about `std` being absent:\n{err}"
    );
}

#[test]
fn the_file_writer_is_absent_under_no_std() {
    // The half a build check cannot reach: a feature that removes nothing passes every
    // positive case above. Under `no_std` there are no files to write to, so the writer and
    // everything naming a path is gone.
    for item in ["odebug::write_to_debug_file", "odebug::debug_dir", "odebug::FileSink"] {
        let body = format!("pub fn reaches() {{ let _ = {item}; }}");
        let (ok, err) = consumer_compiles(
            &format!("absent_{}", item.rsplit(':').next().unwrap()),
            "no_alloc",
            "#![no_std]",
            &body,
        );
        assert!(!ok, "`{item}` must be absent under `no_std`");
        assert!(
            err.contains("odebug"),
            "the error names the crate it is missing from:\n{err}",
        );
    }
}

#[test]
fn the_file_writer_is_present_without_no_std() {
    // The control for the test above, and the reason the default needs no setup at all:
    // with `std` the file writer is there and installs itself on the first entry.
    let body = "pub fn reaches() { let _ = odebug::write_to_debug_file; \
                let _ = odebug::debug_dir; let _ = odebug::FileSink; }";
    let (ok, err) = consumer_compiles("present_file_writer", "use_workspace", "", body);
    assert!(ok, "the file writer is present without `no_std`:\n{err}");
}

#[test]
fn the_sink_suite_actually_runs() {
    // `tests/no_std_sink.rs` drives the contract through a sink of its own. It is not
    // feature-gated, but it shares one global sink with the file writer's own tests, and a
    // suite that stopped compiling reports the same `running 0 tests` as one that was
    // filtered out.
    let output = Command::new(env!("CARGO"))
        .args(["test", "--test", "no_std_sink"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env(
            "CARGO_TARGET_DIR",
            concat!(env!("CARGO_MANIFEST_DIR"), "/target/feature-matrix"),
        )
        .output()
        .expect("cargo runs");

    let report = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "the sink suite passes:\n{report}");

    let ran: usize = report
        .lines()
        .find_map(|line| line.strip_prefix("test result: ok. "))
        .and_then(|rest| rest.split(' ').next())
        .and_then(|count| count.parse().ok())
        .expect("the suite reported a result line");

    assert!(
        ran >= 4,
        "the sink suite ran {ran} cases, where it has at least 4"
    );
}
