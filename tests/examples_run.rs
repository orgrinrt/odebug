//! Every example builds and runs, and prints something.
//!
//! Both examples here are load-bearing documentation rather than illustration. One names
//! every shape the macro accepts, so an arm that stops matching shows up as an example that
//! stops building; the other implements the contract the way a consumer would, so a change to
//! that contract breaks it before it breaks anybody's crate.
//!
//! The empty-output assertion catches an example whose `main` was left as a stub, which is
//! the one failure building cannot see.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// The examples, read from the directory rather than listed, so one added without being named
/// here still runs.
fn examples() -> Vec<String> {
    let dir = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/examples"));
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("examples/ exists")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "rs"))
        .filter_map(|path| path.file_stem().map(|s| s.to_string_lossy().to_string()))
        .collect();
    names.sort();
    names
}

#[test]
fn every_example_runs_and_says_something() {
    let names = examples();
    assert!(
        names.len() >= 2,
        "found {} examples, which is fewer than the directory is supposed to hold. An example \
         deleted stops being checked, and nothing else would report it.",
        names.len()
    );

    for name in &names {
        let output = Command::new(env!("CARGO"))
            .args(["run", "--quiet", "--example", name])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .env(
                "CARGO_TARGET_DIR",
                concat!(env!("CARGO_MANIFEST_DIR"), "/target/examples-run"),
            )
            .output()
            .expect("cargo runs");

        assert!(
            output.status.success(),
            "`cargo run --example {name}` failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !output.stdout.is_empty(),
            "`{name}` ran and printed nothing, so whatever it was showing is not being shown"
        );
    }
}

#[test]
fn the_own_sink_example_keeps_its_entries_out_of_a_file() {
    // The example claims the three entries it logs reach its own sink and no file. That is
    // the property the contract exists for, and a claim in a comment is not a check: a sink
    // that failed to install would leave the default writer in place, the entries would go to
    // `.debug/` as usual, and the example's output would be identical apart from the list it
    // prints being empty.
    let output = Command::new(env!("CARGO"))
        .args(["run", "--quiet", "--example", "your_own_sink"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env(
            "CARGO_TARGET_DIR",
            concat!(env!("CARGO_MANIFEST_DIR"), "/target/examples-run"),
        )
        .output()
        .expect("cargo runs");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("the first entry")
            && stdout.contains("12 of them")
            && stdout.contains("[codegen.log][NOTE]"),
        "the installed sink did not receive all three entries, so either it was not installed \
         or the entries went somewhere else:\n{stdout}"
    );
}
