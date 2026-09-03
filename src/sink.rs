//! Where the log goes, and how it gets there.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// The rule between one entry and the next.
const SEPARATOR_LINE: &str = "-----------------------------------------------------------";

/// The directory the log files are written into.
///
/// Resolved once, on the first write, from the feature flags: the target directory under
/// `output_to_target`, the workspace root's `.debug` under `use_workspace`, and the
/// current directory's `.debug` otherwise.
pub fn debug_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = determine_debug_dir();
        if let Err(e) = fs::create_dir_all(&dir) {
            eprintln!("odebug: could not create {}: {e}", dir.display());
        }
        dir
    })
}

fn determine_debug_dir() -> PathBuf {
    #[cfg(feature = "output_to_target")]
    {
        find_target_dir().map_or_else(
            || {
                eprintln!("odebug: no target directory found, falling back to the default");
                default_debug_dir()
            },
            |dir| dir.join("odebug"),
        )
    }

    #[cfg(not(feature = "output_to_target"))]
    {
        default_debug_dir()
    }
}

fn default_debug_dir() -> PathBuf {
    #[cfg(feature = "use_workspace")]
    {
        find_workspace_root().map_or_else(
            || {
                eprintln!("odebug: no workspace root found, falling back to the current directory");
                std::env::current_dir().unwrap_or_default().join(".debug")
            },
            |root| root.join(".debug"),
        )
    }

    #[cfg(not(feature = "use_workspace"))]
    {
        std::env::current_dir().unwrap_or_default().join(".debug")
    }
}

#[cfg(feature = "output_to_target")]
fn find_target_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        return Some(PathBuf::from(dir));
    }

    #[cfg(feature = "use_workspace")]
    if let Some(root) = find_workspace_root() {
        return Some(root.join("target"));
    }

    Some(std::env::current_dir().ok()?.join("target"))
}

/// Walks up from the current directory looking for the manifest that declares a workspace.
///
/// The declaration is matched as a line of its own rather than as a substring. Searching
/// for `[workspace]` anywhere in the text also matches `[workspace.dependencies]`, which
/// a member crate carries, and a member is not the root.
#[cfg(feature = "use_workspace")]
fn find_workspace_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let manifest = dir.join("Cargo.toml");
        if manifest.is_file() {
            if let Ok(text) = fs::read_to_string(&manifest) {
                if declares_a_workspace(&text) {
                    return Some(dir);
                }
            }
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Whether a manifest's text opens a `[workspace]` table.
///
/// Split out so it can be tested against the shapes that used to fool it.
#[cfg(feature = "use_workspace")]
pub(crate) fn declares_a_workspace(manifest: &str) -> bool {
    manifest
        .lines()
        .map(str::trim)
        .any(|line| line == "[workspace]")
}

/// The open log files, keyed by name.
///
/// Kept open because opening the file per line comes out a few times slower than writing
/// through `write_to_debug_file` with the handle already here, measured in
/// `benches/write.rs`, and a build that logs a few thousand lines spends that difference
/// inside the compiler, where it is felt. The absolute figures move a lot with the machine
/// so none are quoted; the bench keeps every alternative as an arm and answers for the
/// machine it runs on. The arm that only keeps a handle and writes reads faster still, but
/// no caller can obtain that, since the lock and the map lookup are part of the cost.
///
/// Unbuffered on purpose. A buffer takes another order of magnitude or two off and loses
/// whatever hasn't been flushed when the process dies, which for a debug log is the moment
/// the contents matter most. The `buffered` feature is there for logging in bulk from
/// something that will exit tidily.
static FILES: Mutex<Option<HashMap<String, Sink>>> = Mutex::new(None);

#[cfg(not(feature = "buffered"))]
type Sink = File;
#[cfg(feature = "buffered")]
type Sink = std::io::BufWriter<File>;

#[cfg(not(feature = "buffered"))]
fn wrap(file: File) -> Sink {
    file
}
#[cfg(feature = "buffered")]
fn wrap(file: File) -> Sink {
    std::io::BufWriter::new(file)
}

/// Writes an entry to a log file, with an optional header and context.
///
/// The first write to a given name in a process truncates the file, so a log describes one
/// run rather than accumulating across them. Later writes in the same process append.
///
/// # Errors
///
/// Returns the underlying `io::Error` if the file cannot be opened or written.
///
/// # Examples
///
/// ```
/// # use odebug::write_to_debug_file;
/// write_to_debug_file(
///     "debug.log",
///     "Something happened",
///     Some("INFO"),
///     Some("main.rs:42")
/// ).expect("Failed to write to log");
/// ```
pub fn write_to_debug_file(
    filename: &str,
    content: &str,
    header: Option<&str>,
    context: Option<&str>,
) -> std::io::Result<()> {
    let mut guard = FILES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let files = guard.get_or_insert_with(HashMap::new);

    let sink = match files.get_mut(filename) {
        Some(sink) => sink,
        None => {
            let path = debug_dir().join(filename);
            // Truncating here rather than removing keeps any handle a reader already has
            // pointed at the same file.
            let file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&path)?;
            files.entry(filename.to_owned()).or_insert(wrap(file))
        },
    };

    match (header, context) {
        (Some(header), Some(context)) => {
            writeln!(sink, "\n{SEPARATOR_LINE}")?;
            writeln!(sink, "> {header} ({context})")?;
            writeln!(sink, "{SEPARATOR_LINE}")?;
            writeln!(sink, "{content}")
        },
        (Some(header), None) => {
            writeln!(sink, "\n{SEPARATOR_LINE}")?;
            writeln!(sink, "> {header}")?;
            writeln!(sink, "{SEPARATOR_LINE}")?;
            writeln!(sink, "{content}")
        },
        (None, Some(context)) => {
            writeln!(sink, "\n{SEPARATOR_LINE}")?;
            writeln!(sink, "> [at {context}]")?;
            writeln!(sink, "{SEPARATOR_LINE}")?;
            writeln!(sink, "{content}")
        },
        (None, None) => writeln!(sink, "\n{content}"),
    }
}

/// Pushes anything still held in memory out to the files.
///
/// Without the `buffered` feature every write has already gone to the operating system and
/// this does nothing, so calling it is harmless either way. With it, this is what makes
/// the log complete, and a process that will not exit through its own end should call it.
///
/// # Errors
///
/// Returns the first `io::Error` any of the open files reports.
pub fn flush() -> std::io::Result<()> {
    let mut guard = FILES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(files) = guard.as_mut() {
        for sink in files.values_mut() {
            sink.flush()?;
        }
    }
    Ok(())
}

/// Forgets every open file, so the next write to a name truncates again.
#[cfg(test)]
pub(crate) fn reset() {
    let mut guard = FILES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *guard = None;
}

/// The file writer, as a [`Sink`].
///
/// The default destination, and the one a procedural macro wants, given that it runs
/// inside the compiler and has no stdout anybody reads. Installed on the first entry
/// unless something else already is, so nothing has to be set up to use `odebug!`, and
/// replaced by installing another sink.
///
/// [`Sink`]: crate::Sink
#[derive(Debug, Clone, Copy)]
pub struct FileSink;

impl notko::sink::Emit<crate::Entry<'_>> for FileSink {
    type Err = crate::Error;

    fn emit(&self, entry: crate::Entry<'_>) -> notko::Outcome<(), Self::Err> {
        // Formatted here rather than at the call site, because that is the difference this
        // sink's having an allocator buys: the entry arrives as `Arguments` and this is
        // where somewhere-to-put-it exists.
        let content = entry.content.to_string();
        let origin = entry.origin.to_string();

        match write_to_debug_file(entry.target, &content, entry.header, Some(&origin)) {
            Ok(()) => notko::Outcome::Ok(()),
            Err(e) => {
                eprintln!("odebug: could not write the log: {e}");
                notko::Outcome::Err(crate::Error)
            },
        }
    }
}

impl crate::Sink for FileSink {
    fn flush(&self) {
        // Reported rather than discarded. Every other failure in this crate reaches stderr,
        // and a flush that silently fails is the one that loses entries: `buffered` holds
        // them until this runs, so a refusal here is data gone rather than data delayed.
        if let Err(e) = flush() {
            eprintln!("odebug: could not flush the log: {e}");
        }
    }
}

/// Installs [`FileSink`] unless something is already installed.
///
/// Called by the macro before every entry, which is once per entry and a load and a branch
/// when a sink is already there. Doing it lazily rather than at startup is what keeps this
/// crate free of any initialisation a consumer has to remember.
pub fn install_default_sink() {
    if crate::sink().is_none() {
        static HOLDER: &dyn crate::Sink = &FileSink;
        crate::install_sink_ref(&HOLDER);
    }
}
