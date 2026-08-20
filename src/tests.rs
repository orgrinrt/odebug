//! Every documented form is written and then read back from the file it claims to write.
//!
//! The tests share one process, one set of log files and one set of open handles, so they
//! run under a lock rather than in parallel. Each takes the lock and resets the handle
//! table, which makes the next write to a name truncate, so a test reads only its own
//! output.

use crate::{debug_dir, odebug, write_to_debug_file};
use std::fs;
use std::sync::{Mutex, MutexGuard};

static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    let guard = SERIAL.lock().unwrap_or_else(|poisoned| {
        SERIAL.clear_poison();
        poisoned.into_inner()
    });
    crate::sink::reset();
    guard
}

/// What a named log file holds right now.
fn read(name: &str) -> String {
    crate::flush().expect("the log flushes");
    fs::read_to_string(debug_dir().join(name))
        .unwrap_or_else(|e| panic!("{name} should exist and be readable: {e}"))
}

#[test]
fn plain_content_goes_to_the_default_file() {
    let _g = serial();
    odebug!("a plain message");
    assert!(read("debug.log").contains("a plain message"));
}

#[test]
fn format_arguments_are_applied() {
    let _g = serial();
    odebug!("a value: {}", 42);
    odebug!("two values: {} and {}", 1, 2);
    let log = read("debug.log");
    assert!(log.contains("a value: 42"), "the argument reaches the file, not the format string");
    assert!(log.contains("two values: 1 and 2"));
}

#[test]
fn a_header_without_a_file_goes_to_the_default_file() {
    let _g = serial();
    odebug!(::Marker("content under a header"));
    let log = read("debug.log");
    assert!(log.contains("Marker"));
    assert!(log.contains("content under a header"));
}

#[test]
fn a_named_file_is_used_and_gets_the_log_suffix() {
    let _g = serial();
    odebug!(parser::("from the parser"));
    assert!(read("parser.log").contains("from the parser"));
}

#[test]
fn a_named_file_and_header_together() {
    let _g = serial();
    odebug!(parser::Trace("traced"));
    let log = read("parser.log");
    assert!(log.contains("Trace"));
    assert!(log.contains("traced"));
}

#[test]
fn a_named_file_takes_format_arguments() {
    let _g = serial();
    odebug!(parser::("token {} of {}", 3, 9));
    odebug!(parser::Trace("token {} of {}", 4, 9));
    let log = read("parser.log");
    assert!(log.contains("token 3 of 9"));
    assert!(log.contains("token 4 of 9"));
}

#[test]
fn a_string_filename_with_the_arrow_form() {
    let _g = serial();
    odebug!("explicit.log" => "written by name");
    odebug!("explicit.log" => "and formatted: {}", 7);
    let log = read("explicit.log");
    assert!(log.contains("written by name"));
    assert!(log.contains("and formatted: 7"));
}

#[test]
fn chaining_on_a_literal() {
    let _g = serial();
    odebug!("to a file".to_file("chain.log"));
    odebug!("with a header".with_header("Chained"));
    odebug!("both".to_file("chain.log").with_header("Combined"));
    let chain = read("chain.log");
    assert!(chain.contains("to a file"));
    assert!(chain.contains("Combined"));
    assert!(chain.contains("both"));
    let default = read("debug.log");
    assert!(default.contains("Chained"));
    assert!(default.contains("with a header"));
}

#[test]
fn chaining_on_a_binding() {
    let _g = serial();
    let message = String::from("from a binding");
    let header = String::from("BindingHeader");
    odebug!(message.to_file("binding.log"));
    odebug!(message.with_header(header));
    odebug!(message.to_file("binding.log").with_header("Combined"));
    assert!(read("binding.log").contains("from a binding"));
    assert!(read("debug.log").contains("BindingHeader"));
}

#[test]
fn every_entry_records_where_it_came_from() {
    let _g = serial();
    odebug!("located");
    let log = read("debug.log");
    assert!(
        log.contains("src/tests.rs:"),
        "an entry carries its own file and line, which is most of why this crate exists.\n{log}"
    );
}

#[test]
fn a_header_is_set_off_by_a_rule() {
    let _g = serial();
    odebug!(::Shape("body"));
    let log = read("debug.log");
    let lines: Vec<&str> = log.lines().filter(|l| !l.is_empty()).collect();
    assert!(lines[0].starts_with("---"), "a rule opens the entry, got {:?}", lines[0]);
    assert!(lines[1].starts_with("> Shape ("), "then the header and context, got {:?}", lines[1]);
    assert!(lines[2].starts_with("---"), "then a rule, got {:?}", lines[2]);
    assert_eq!(lines[3], "body", "then the content");
}

#[test]
fn the_first_write_of_a_run_replaces_what_was_there() {
    let _g = serial();
    odebug!("run.log" => "from an earlier run");
    assert!(read("run.log").contains("from an earlier run"));

    // A fresh process is what `reset` stands in for: it forgets the open handles, so the
    // next write to this name opens it anew.
    crate::sink::reset();
    odebug!("run.log" => "from this run");

    let log = read("run.log");
    assert!(log.contains("from this run"));
    assert!(
        !log.contains("from an earlier run"),
        "a log describes one run, so the first write of a run truncates.\n{log}"
    );
}

#[test]
fn later_writes_in_one_run_append() {
    let _g = serial();
    odebug!("append.log" => "first");
    odebug!("append.log" => "second");
    let log = read("append.log");
    assert!(log.contains("first") && log.contains("second"), "both survive:\n{log}");
}

#[test]
fn writing_from_several_threads_keeps_every_line() {
    let _g = serial();
    const LINES: usize = 64;
    std::thread::scope(|s| {
        for n in 0..LINES {
            s.spawn(move || {
                write_to_debug_file("threads.log", &format!("line {n}"), None, None)
                    .expect("a threaded write");
            });
        }
    });
    let log = read("threads.log");
    for n in 0..LINES {
        assert!(log.contains(&format!("line {n}")), "line {n} went missing under threads");
    }
}

#[test]
fn the_public_api_reports_a_failure_rather_than_panicking() {
    let _g = serial();
    // A name with a path separator in it does not resolve to a file inside the debug
    // directory, so this is the failure path a caller can actually reach.
    let result = write_to_debug_file("no/such/directory/x.log", "content", None, None);
    assert!(result.is_err(), "an unwritable name is an error, not a panic");
}

#[cfg(feature = "use_workspace")]
mod workspace_detection {
    use crate::sink::declares_a_workspace;

    #[test]
    fn a_root_manifest_is_recognised() {
        assert!(declares_a_workspace("[workspace]\nmembers = []\n"));
        assert!(declares_a_workspace("[package]\nname = \"x\"\n\n[workspace]\n"));
        assert!(declares_a_workspace("  [workspace]  \n"), "leading space is still the table");
    }

    #[test]
    fn a_member_manifest_is_not() {
        // The case that used to fool it. A member crate inheriting from its root carries
        // `[workspace.dependencies]`, and a substring search reads that as a root, so the
        // first member walked past would have been called the workspace.
        assert!(!declares_a_workspace(
            "[package]\nname = \"member\"\n\n[dependencies]\nserde.workspace = true\n"
        ));
        assert!(!declares_a_workspace("[workspace.dependencies]\nserde = \"1\"\n"));
        assert!(!declares_a_workspace("[workspace.package]\nversion = \"1.0\"\n"));
        assert!(!declares_a_workspace("# [workspace] is commented out\n"));
        assert!(!declares_a_workspace("description = \"see [workspace] for more\"\n"));
    }
}

mod directory_choice {
    use crate::debug_dir;

    #[test]
    fn the_directory_matches_the_features_that_chose_it() {
        let dir = debug_dir();
        let shown = dir.to_string_lossy();

        #[cfg(feature = "output_to_target")]
        assert!(
            shown.contains("target/odebug") || shown.contains("target\\odebug"),
            "with output_to_target the log belongs under the target directory, got {shown}"
        );

        #[cfg(all(not(feature = "output_to_target"), feature = "use_workspace"))]
        assert!(
            dir.ends_with(".debug"),
            "with use_workspace the log belongs in the workspace's .debug, got {shown}"
        );

        #[cfg(all(not(feature = "output_to_target"), not(feature = "use_workspace")))]
        assert_eq!(
            dir,
            std::env::current_dir().unwrap_or_default().join(".debug"),
            "with neither, the log belongs beside the caller"
        );

        assert!(dir.is_dir(), "the directory is made, not just named");
    }
}

/// The macro's own feature belongs to this crate, not to the caller's.
///
/// `odebug!` used to expand to `#[cfg(any(debug_assertions, feature = "always_log"))]`, and
/// an expansion is compiled in the calling crate, so that condition asked whether the
/// *caller* had a feature called `always_log`. Callers do not; it is odebug's. The feature
/// therefore did nothing for anybody, and rustc reported
/// `unexpected cfg condition value: always_log` once per call site: twenty-eight of them in
/// one consumer.
///
/// There is no way to observe a `cfg` from another crate at runtime, so what this asserts
/// is the half that is observable: with the feature on, logging happens whatever the
/// caller's build profile is.
#[test]
#[cfg(feature = "always_log")]
fn always_log_writes_regardless_of_the_build_profile() {
    let _g = serial();
    odebug!("always.log" => "written under always_log");
    assert!(read("always.log").contains("written under always_log"));
}

#[test]
#[cfg(all(not(feature = "always_log"), debug_assertions))]
fn without_always_log_a_debug_build_still_writes() {
    let _g = serial();
    odebug!("debugonly.log" => "written in a debug build");
    assert!(read("debugonly.log").contains("written in a debug build"));
}
