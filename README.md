# odebug

<div style="text-align: center;">

[![GitHub Stars](https://img.shields.io/github/stars/orgrinrt/odebug.svg)](https://github.com/orgrinrt/odebug/stargazers)
[![Crates.io Total Downloads](https://img.shields.io/crates/d/odebug)](https://crates.io/crates/odebug)
[![GitHub Issues](https://img.shields.io/github/issues/orgrinrt/odebug.svg)](https://github.com/orgrinrt/odebug/issues)
[![Current Version](https://img.shields.io/badge/version-0.1.0-red.svg)](https://github.com/orgrinrt/odebug)

> Debug logging utility that writes to text files, practical especially during proc-macro compilation.

</div>

## Features

- Macro-based API for logging information to files
- Configurable output location (project root, workspace root, or target directory)
- Works during proc-macro compilation, where print output is hard to capture
- No dependencies besides `once_cell`
- No runtime overhead when not building for debug (unless `always_log` feature is enabled)

## Usage

```rust
use odebug::odebug;

let some_value = 42;

// basic logging to the default debug.log file
odebug!("Simple message");
odebug!("Formatted message: value = {}", some_value);

// logging to a custom file (string literal syntax)
odebug!("test.log" => "This goes to test.log");
odebug!("test.log" => "Formatted: {}", some_value);

// path-based syntax: the first ident names the file ("custom.log" below),
// the second becomes a header above the entry
odebug!(custom::Header("A message with a header"));
odebug!(custom::Header("Formatted message: {}", some_value));
// file only (no header), or header only (default debug.log file)
odebug!(custom::("No header"));
odebug!(::Header("Header in debug.log"));

// alternative to above, method chaining syntax, works for string literals and idents
odebug!("My message".to_file("custom.log"));
odebug!("My message".with_header("IMPORTANT"));
odebug!("My message".to_file("custom.log").with_header("DEBUG"));

// also works with variables, but they are not evaluated as expressions,
// rather only as idents to use internally (some caveats for usage)
let msg = format!("Dynamic content: {}", some_value);
odebug!(msg.to_file("dynamic.log").with_header("VARIABLE"));
```

## Configuration

The crate can be configured with feature flags:

- `use_workspace` (default): Resolves paths from the workspace root instead of the current directory (the workspace `target` directory, or the workspace root's `.debug` directory when `output_to_target` is disabled)
- `output_to_target` (default): Places log files in the `target/odebug` directory (honours `CARGO_TARGET_DIR`) instead of the legacy `.debug` directory
- `always_log`: Always logs to the file, even if debug_assertions are disabled

## The Problem

Debugging complex code flows, especially in proc-macros, can be challenging, often feeling like the usual tools in your toolbox are limited or unhelpful. Print statements often get lost in compiler output or don't work at all in certain contexts. Stepping through code with a debugger can be tedious and time-consuming with proc macros, especially when dealing with large codebases and complex expansions. It's also so very easy to end up in an all-inclusive stepping tour through the
`syn`, `quote`, and `proc_macro2` crates.

`odebug` provides a simple way to log values, expressions, token streams and similar to files at specific points in your code, fairly ergonomically. After execution, you can examine these logs to understand what happened during compilation or runtime, with the specific file name and the line number for reference automatically collected and included.

## Support

Whether you use this project, have learned something from it, or just like it, please consider supporting it by buying me a coffee, so I can dedicate more time on open-source projects like this :)

<a href="https://buymeacoffee.com/orgrinrt" target="_blank"><img src="https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png" alt="Buy Me A Coffee" style="height: auto !important;width: auto !important;" ></a>

## License

> You can check out the full license [here](https://github.com/orgrinrt/odebug/blob/main/LICENSE)

This project is licensed under the terms of the **MIT** license.
