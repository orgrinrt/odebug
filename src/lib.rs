use once_cell::sync::Lazy;
use std::collections::HashSet;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

pub static DEBUG_DIR: Lazy<PathBuf> = Lazy::new(|| {
    let debug_dir = determine_debug_dir();
    fs::create_dir_all(&debug_dir).unwrap_or_else(|e| {
        eprintln!("Failed to create debug directory: {}", e);
    });
    debug_dir
});

/// Determines the appropriate debug directory based on feature flags
fn determine_debug_dir() -> PathBuf {
    #[cfg(feature = "output_to_target")]
    {
        find_target_dir()
            .map(|dir| dir.join("odebug"))
            .unwrap_or_else(|| {
                eprintln!(
                    "Warning: Could not find target directory, falling back to default location"
                );
                default_debug_dir()
            })
    }

    #[cfg(not(feature = "output_to_target"))]
    {
        default_debug_dir()
    }
}

/// Returns the default debug directory (either workspace root or current dir)/.debug
fn default_debug_dir() -> PathBuf {
    #[cfg(feature = "use_workspace")]
    {
        find_workspace_root()
            .map(|root| root.join(".debug"))
            .unwrap_or_else(|| {
                eprintln!(
                    "Warning: Could not find workspace root, falling back to current directory"
                );
                std::env::current_dir().unwrap_or_default().join(".debug")
            })
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
    {
        if let Some(ws_root) = find_workspace_root() {
            return Some(ws_root.join("target"));
        }
    }

    let current = std::env::current_dir().ok()?;
    Some(current.join("target"))
}

#[cfg(feature = "use_workspace")]
fn find_workspace_root() -> Option<PathBuf> {
    let mut current_dir = env::current_dir().ok()?;
    loop {
        let cargo_toml_path = current_dir.join("Cargo.toml");
        if cargo_toml_path.exists() {
            if let Ok(content) = fs::read_to_string(&cargo_toml_path) {
                if content.contains("[workspace]") {
                    return Some(current_dir);
                }
            }
        }
        if !current_dir.pop() {
            break;
        }
    }
    None
}

const SEPARATOR_LINE: &str = "-----------------------------------------------------------";

static INITIALIZED_FILES: Lazy<std::sync::Mutex<HashSet<String>>> =
    Lazy::new(|| std::sync::Mutex::new(HashSet::new()));

pub fn write_to_debug_file(
    filename: &str,
    content: &str,
    header: Option<&str>,
    context: Option<&str>,
) -> std::io::Result<()> {
    let _ = fs::create_dir_all(&*DEBUG_DIR);

    let path = DEBUG_DIR.join(filename);

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

    // buffered writer for better performance
    let file = OpenOptions::new().create(true).append(true).open(&path)?;
    let mut writer = std::io::BufWriter::new(file);

    match (header, context) {
        (Some(header), Some(context)) => {
            writeln!(writer, "\n{0}", SEPARATOR_LINE)?;
            writeln!(writer, "> {0} ({1})", header, context)?;
            writeln!(writer, "{0}", SEPARATOR_LINE)?;
            writeln!(writer, "{0}", content)?;
        },
        (Some(header), None) => {
            writeln!(writer, "\n{0}", SEPARATOR_LINE)?;
            writeln!(writer, "> {0}", header)?;
            writeln!(writer, "{0}", SEPARATOR_LINE)?;
            writeln!(writer, "{0}", content)?;
        },
        (None, Some(context)) => {
            writeln!(writer, "\n{0}", SEPARATOR_LINE)?;
            writeln!(writer, "> [at {0}]", context)?;
            writeln!(writer, "{0}", SEPARATOR_LINE)?;
            writeln!(writer, "{0}", content)?;
        },
        (None, None) => {
            writeln!(writer, "\n{0}", content)?;
        },
    }

    writer.flush()?;

    Ok(())
}

#[macro_export]
macro_rules! odebug {
    ($($args:tt)*) => {
        #[cfg(any(debug_assertions, feature = "always_log"))]
        {
            $crate::__internal_debug_macro!($($args)*)
        }
    };
}

#[macro_export]
macro_rules! __internal_debug_macro {
    // path-like syntax with file and header
    ($file:ident::$header:ident($content:expr)) => {{
        let context = format!("{}:{}", file!(), line!());
        $crate::write_to_debug_file(
            &format!("{}.log", stringify!($file)),
            &$content.to_string(),
            Some(stringify!($header)),
            Some(&context)
        ).unwrap_or_else(|e| eprintln!("Failed to write debug log: {}", e))
    }};

    // path-like syntax with file and header, formatted content
    ($file:ident::$header:ident($fmt:expr, $($arg:tt)+)) => {{
        let context = format!("{}:{}", file!(), line!());
        let content = format!($fmt, $($arg)+);
        $crate::write_to_debug_file(
            &format!("{}.log", stringify!($file)),
            &content,
            Some(stringify!($header)),
            Some(&context)
        ).unwrap_or_else(|e| eprintln!("Failed to write debug log: {}", e))
    }};

    // path-like syntax with just file
    ($file:ident::($content:expr)) => {{
        let context = format!("{}:{}", file!(), line!());
        $crate::write_to_debug_file(
            &format!("{}.log", stringify!($file)),
            &$content.to_string(),
            None,
            Some(&context)
        ).unwrap_or_else(|e| eprintln!("Failed to write debug log: {}", e))
    }};

    // path-like syntax with just file, formatted content
    ($file:ident::($fmt:expr, $($arg:tt)+)) => {{
        let context = format!("{}:{}", file!(), line!());
        let content = format!($fmt, $($arg)+);
        $crate::write_to_debug_file(
            &format!("{}.log", stringify!($file)),
            &content,
            None,
            Some(&context)
        ).unwrap_or_else(|e| eprintln!("Failed to write debug log: {}", e))
    }};

    // just header syntax
    (::$header:ident($content:expr)) => {{
        let context = format!("{}:{}", file!(), line!());
        $crate::write_to_debug_file(
            "debug.log",
            &$content.to_string(),
            Some(stringify!($header)),
            Some(&context)
        ).unwrap_or_else(|e| eprintln!("Failed to write debug log: {}", e))
    }};

    // just header syntax with formatted content
    (::$header:ident($fmt:expr, $($arg:tt)+)) => {{
        let context = format!("{}:{}", file!(), line!());
        let content = format!($fmt, $($arg)+);
        $crate::write_to_debug_file(
            "debug.log",
            &content,
            Some(stringify!($header)),
            Some(&context)
        ).unwrap_or_else(|e| eprintln!("Failed to write debug log: {}", e))
    }};

    // string literal filename support (keeping => syntax)
    ($file:expr => $content:expr) => {{
        let context = format!("{}:{}", file!(), line!());
        $crate::write_to_debug_file(
            $file,
            &$content.to_string(),
            None,
            Some(&context)
        ).unwrap_or_else(|e| eprintln!("Failed to write debug log: {}", e))
    }};

    // string literal filename with formatted content
    ($file:expr => $fmt:expr, $($arg:tt)+) => {{
        let context = format!("{}:{}", file!(), line!());
        let content = format!($fmt, $($arg)*);
        $crate::write_to_debug_file(
            $file,
            &content,
            None,
            Some(&context)
        ).unwrap_or_else(|e| eprintln!("Failed to write debug log: {}", e))
    }};

    // method chaining for literals
    ($content:literal.to_file($file:expr)) => {{
        let context = format!("{}:{}", file!(), line!());
        $crate::write_to_debug_file(
            $file,
            &$content.to_string(),
            None,
            Some(&context)
        ).unwrap_or_else(|e| eprintln!("Failed to write debug log: {}", e))
    }};

    ($content:literal.with_header($header:expr)) => {{
        let context = format!("{}:{}", file!(), line!());
        $crate::write_to_debug_file(
            "debug.log",
            &$content.to_string(),
            Some(&$header.to_string()),
            Some(&context)
        ).unwrap_or_else(|e| eprintln!("Failed to write debug log: {}", e))
    }};

    // combined method chaining for literals
    ($content:literal.to_file($file:expr).with_header($header:expr)) => {{
        let context = format!("{}:{}", file!(), line!());
        $crate::write_to_debug_file(
            $file,
            &$content.to_string(),
            Some(&$header.to_string()),
            Some(&context)
        ).unwrap_or_else(|e| eprintln!("Failed to write debug log: {}", e))
    }};

    // method chaining for identifiers
    ($content:ident.to_file($file:expr)) => {{
        let context = format!("{}:{}", file!(), line!());
        $crate::write_to_debug_file(
            $file,
            &$content.to_string(),
            None,
            Some(&context)
        ).unwrap_or_else(|e| eprintln!("Failed to write debug log: {}", e))
    }};

    ($content:ident.with_header($header:expr)) => {{
        let context = format!("{}:{}", file!(), line!());
        $crate::write_to_debug_file(
            "debug.log",
            &$content.to_string(),
            Some(&$header.to_string()),
            Some(&context)
        ).unwrap_or_else(|e| eprintln!("Failed to write debug log: {}", e))
    }};

    ($content:ident.to_file($file:expr).with_header($header:expr)) => {{
        let context = format!("{}:{}", file!(), line!());
        $crate::write_to_debug_file(
            $file,
            &$content.to_string(),
            Some(&$header.to_string()),
            Some(&context)
        ).unwrap_or_else(|e| eprintln!("Failed to write debug log: {}", e))
    }};

    // simple content (default file, no header)
    ($content:expr) => {{
        let context = format!("{}:{}", file!(), line!());
        $crate::write_to_debug_file(
            "debug.log",
            &$content.to_string(),
            None,
            Some(&context)
        ).unwrap_or_else(|e| eprintln!("Failed to write debug log: {}", e))
    }};

    // format string (default file, no header)
    ($fmt:expr, $($arg:tt)+) => {{
        let context = format!("{}:{}", file!(), line!());
        let content = format!($fmt, $($arg)+);
        $crate::write_to_debug_file(
            "debug.log",
            &content,
            None,
            Some(&context)
        ).unwrap_or_else(|e| eprintln!("Failed to write debug log: {}", e))
    }};
}

#[cfg(test)]
mod tests {
    use once_cell::sync::Lazy;
    use std::fs;
    use std::path::Path;
    use std::sync::Mutex;

    static TEST_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    fn cleanup_test_logs() {
        let debug_dir = crate::DEBUG_DIR.as_path();
        let files = ["debug.log", "custom.log", "test.log"];
        for file in files {
            let _ = fs::remove_file(debug_dir.join(file));
        }
    }

    #[test]
    fn test_default_variants() {
        let _guard = TEST_MUTEX.lock().unwrap();
        cleanup_test_logs();

        // Test format string variant
        odebug!("Test value: {}", 42);

        // Test plain content variant
        odebug!("Plain message");

        // Test header and content variant (now using path syntax)
        odebug!(::TestHeader("Test content"));

        // Verify file was created
        let path = crate::DEBUG_DIR.join("debug.log");
        assert!(Path::new(&path).exists(), "debug.log should exist");

        // Verify file content
        let content = fs::read_to_string(path).unwrap();
        let expected_values = ["Test value: 42", "Plain message", "TestHeader", "Test content"];

        for expected in expected_values {
            assert!(
                content.contains(expected),
                "Log should contain: '{}'",
                expected
            );
        }
    }

    #[test]
    fn test_custom_filename_variants() {
        let _guard = TEST_MUTEX.lock().unwrap();
        cleanup_test_logs();

        // Test all custom filename variants with the new syntax
        odebug!(custom::("Test value: {}", 42));
        odebug!(custom::("Plain message"));
        odebug!(custom::TestHeader("Test content"));
        odebug!("custom.log" => "Alternative content");

        // Verify file was created
        let path = crate::DEBUG_DIR.join("custom.log");
        assert!(Path::new(&path).exists(), "custom.log should exist");

        // Verify file content
        let content = fs::read_to_string(path).unwrap();
        let expected_values = [
            "Test value: 42",
            "Plain message",
            "TestHeader",
            "Test content",
            "Alternative content",
        ];

        for expected in expected_values {
            assert!(
                content.contains(expected),
                "Log should contain: '{}'",
                expected
            );
        }
    }

    #[test]
    fn test_string_literal_filename_variants() {
        let _guard = TEST_MUTEX.lock().unwrap();
        cleanup_test_logs();

        // Test string filename variants with => syntax
        odebug!("test.log" => "Test value: {}", 42);
        odebug!("test.log" => "Plain message");
        odebug!("test.log" => "Test content");

        // Verify file was created
        let path = crate::DEBUG_DIR.join("test.log");
        assert!(Path::new(&path).exists(), "test.log should exist");

        // Verify file content
        let content = fs::read_to_string(path).unwrap();
        let expected_values = ["Test value: 42", "Plain message", "Test content"];

        for expected in expected_values {
            assert!(
                content.contains(expected),
                "Log should contain: '{}'",
                expected
            );
        }
    }

    #[test]
    fn test_literal_method_chaining() {
        let _guard = TEST_MUTEX.lock().unwrap();
        cleanup_test_logs();

        // Test literal method chaining
        odebug!("Message".to_file("chain.log"));
        odebug!("Message".with_header("Test Header"));
        odebug!("Message".to_file("chain.log").with_header("Combined"));

        // Verify files were created
        let debug_path = crate::DEBUG_DIR.join("debug.log");
        let chain_path = crate::DEBUG_DIR.join("chain.log");

        assert!(Path::new(&debug_path).exists(), "debug.log should exist");
        assert!(Path::new(&chain_path).exists(), "chain.log should exist");

        // Verify content
        let debug_content = fs::read_to_string(debug_path).unwrap();
        let chain_content = fs::read_to_string(chain_path).unwrap();

        assert!(
            debug_content.contains("Test Header"),
            "debug.log should contain the header"
        );
        assert!(
            chain_content.contains("Message"),
            "chain.log should contain the message"
        );
        assert!(
            chain_content.contains("Combined"),
            "chain.log should contain the combined header"
        );
    }

    #[test]
    fn test_identifier_method_chaining() {
        let _guard = TEST_MUTEX.lock().unwrap();
        cleanup_test_logs();

        // Create variables to test identifier chaining
        let message = "Variable message".to_string();
        let header = "Variable header".to_string();

        // Test identifier method chaining
        odebug!(message.to_file("var.log"));
        odebug!(message.with_header(header));
        odebug!(message.to_file("var.log").with_header("Combined"));

        // Verify files were created
        let debug_path = crate::DEBUG_DIR.join("debug.log");
        let var_path = crate::DEBUG_DIR.join("var.log");

        assert!(Path::new(&debug_path).exists(), "debug.log should exist");
        assert!(Path::new(&var_path).exists(), "var.log should exist");

        // Verify content
        let debug_content = fs::read_to_string(debug_path).unwrap();
        let var_content = fs::read_to_string(var_path).unwrap();

        assert!(
            debug_content.contains("Variable header"),
            "debug.log should contain the variable header"
        );
        assert!(
            var_content.contains("Variable message"),
            "var.log should contain the variable message"
        );
        assert!(
            var_content.contains("Combined"),
            "var.log should contain the combined header"
        );
    }
}

#[cfg(test)]
mod feature_tests {
    use super::*;

    fn get_debug_dir_path() -> PathBuf {
        determine_debug_dir()
    }

    #[test]
    fn test_debug_dir_location() {
        let dir = get_debug_dir_path();
        println!("Debug directory would be: {}", dir.display());

        #[cfg(feature = "output_to_target")]
        {
            // Check if CARGO_TARGET_DIR is set
            if let Ok(target_dir) = std::env::var("CARGO_TARGET_DIR") {
                // If set, our path should start with it and end with "odebug"
                assert!(
                    dir.to_string_lossy().starts_with(&target_dir),
                    "Should use CARGO_TARGET_DIR when set"
                );
            } else {
                // Otherwise check for the default pattern
                assert!(
                    dir.to_string_lossy().contains("target/odebug")
                        || dir.to_string_lossy().contains("target\\odebug"),
                    "Default path should contain 'target/odebug'"
                );
            }

            // Always verify it ends with "odebug" regardless of the base path
            assert!(
                dir.file_name().map_or(false, |name| name == "odebug"),
                "Path should end with 'odebug' directory"
            );
        }

        // Keep the rest of your existing test cases for other features
        #[cfg(all(not(feature = "output_to_target"), feature = "use_workspace"))]
        {
            assert!(
                dir.ends_with(".debug"),
                "With use_workspace enabled, path should end with '.debug'"
            );
        }

        #[cfg(all(not(feature = "output_to_target"), not(feature = "use_workspace")))]
        {
            let expected = std::env::current_dir().unwrap_or_default().join(".debug");
            assert_eq!(dir, expected, "Default path should be current_dir/.debug");
        }
    }

    // Test with environment variable
    #[test]
    #[cfg(feature = "output_to_target")]
    fn test_target_dir_env_var() {
        let test_dir = "/tmp/test_target_dir";
        std::env::set_var("CARGO_TARGET_DIR", test_dir);

        let dir = get_debug_dir_path();

        assert!(
            dir.starts_with(test_dir),
            "Should use CARGO_TARGET_DIR when set"
        );

        std::env::remove_var("CARGO_TARGET_DIR");
    }
}
