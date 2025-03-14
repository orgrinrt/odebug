use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Once;

pub static WORKSPACE_DIR: Lazy<PathBuf> = Lazy::new(|| {
    let output = std::process::Command::new(env!("CARGO"))
        .arg("locate-project")
        .arg("--workspace")
        .arg("--message-format=plain")
        .output()
        .expect("Failed to run cargo locate-project")
        .stdout;
    let cargo_path = Path::new(std::str::from_utf8(&output).expect("Invalid UTF-8").trim());
    cargo_path
        .parent()
        .expect("No parent directory")
        .to_path_buf()
});

static INIT: Once = Once::new();
static DEBUG_DIR: Lazy<PathBuf> = Lazy::new(|| {
    let dir = WORKSPACE_DIR.join(".debug");
    fs::create_dir_all(&dir).expect("Failed to create debug directory");
    dir
});

#[macro_export]
macro_rules! debug_tokens {
    ($name:expr, $tokens:expr) => {
        let content = format!("Token stream: {}", $tokens);
        $crate::write_to_debug_file(&format!("{}_proc_macro.log", $name), &content, None)
            .unwrap_or_else(|e| eprintln!("Failed to write debug log: {}", e));
    };
}

#[macro_export]
macro_rules! debug_proc {
    ($name:expr, $content:expr) => {
        $crate::write_to_debug_file(&format!("{}_proc_macro.log", $name), &$content, None)
            .unwrap_or_else(|e| eprintln!("Failed to write debug log: {}", e));
    };
    ($name:expr, $header:expr, $content:expr) => {
        $crate::write_to_debug_file(
            &format!("{}_proc_macro.log", $name),
            &$content,
            Some($header),
        )
        .unwrap_or_else(|e| eprintln!("Failed to write debug log: {}", e));
    };
}

const SEPARATOR_LINE: &str = "-----------------------------------------------------------";

static INITIALIZED_FILES: Lazy<std::sync::Mutex<HashSet<String>>> =
    Lazy::new(|| std::sync::Mutex::new(HashSet::new()));

pub fn write_to_debug_file(
    filename: &str,
    content: &str,
    header: Option<&str>,
) -> std::io::Result<()> {
    let path = DEBUG_DIR.join(filename);

    // minimize the lock duration by scoping it
    let should_clear = {
        let mut initialized = INITIALIZED_FILES.lock().unwrap();
        if !initialized.contains(filename) {
            initialized.insert(filename.to_string());
            true
        } else {
            false
        }
    };

    if should_clear {
        let _ = fs::remove_file(&path);
    }

    let mut file = OpenOptions::new().create(true).append(true).open(path)?;

    match header {
        Some(header) => writeln!(
            file,
            "\n{0}\n\
             > {1}\n\
             {0}\n\
             {2}",
            SEPARATOR_LINE, header, content
        ),
        None => writeln!(file, "\n{}", content),
    }
}
