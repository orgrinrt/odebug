# odebug

<div style="text-align: center;">

[![GitHub Stars](https://img.shields.io/github/stars/orgrinrt/odebug.svg)](https://github.com/orgrinrt/odebug/stargazers)
[![Crates.io Total Downloads](https://img.shields.io/crates/d/odebug)](https://crates.io/crates/odebug)
[![GitHub Issues](https://img.shields.io/github/issues/orgrinrt/odebug.svg)](https://github.com/orgrinrt/odebug/issues)
[![Current Version](https://img.shields.io/badge/version-0.0.1-red.svg)](https://github.com/orgrinrt/odebug)

> Simple and flexible debug logging utility that allows simple and practical logging to a text file especially during proc-macro compilation.

</div>

## Features

- Simple macro-based API for logging information to files
- Configurable output location (project root, workspace root, or target directory)
- Works great for debugging proc-macros
- No dependencies besides `once_cell`
- No runtime overhead when not building for debug (unless `always_log` feature is enabled)

## Usage

```rust
use odebug::odebug;

// basic logging to default debug.log file
odebug!("Simple message");
odebug!("Formatted message: value = {}", some_value);

// logging to a custom file (legacy syntax)
odebug!("test.log" => "This goes to test.log");

// path-based syntax has hierarchical formatting
// below, the file name is derived from the path (first node) = "custom.log"
odebug!(custom::nested::headers("A message with two headers, one for each level"));
// or alternatively, a string literal can be used to explicitly specify the file name
odebug!("explicit.log"::specific::outfile("Message with explicit file name and fmt {}", foo));

// alternative to above, method chaining syntax, works for string literals and idents
// can be used with any type that implements `ToString`
odebug!("My message".to_file("custom.log"));
odebug!("My message".with_header("IMPORTANT"));
odebug!("My message".to_file("custom.log").with_header("DEBUG"));

// also works with variables, but they are not evaluated as expressions,
// rather only as idents to use internally (some caveats for usage)
let msg = format!("Dynamic content: {}", value);
odebug!(msg.to_file("dynamic.log").with_header("VARIABLE"));
```

## Configuration

The crate can be configured with feature flags:

- `use_workspace` (default): Places log files in workspace root's `.debug` directory if in a workspace
- `output_to_target` (default): Places log files in `target/odebug` directory instead of the legacy `root/.debug` directory
- `always_log`: Always logs to the file, even if debug_assertions are disabled

## The Problem

Debugging complex code flows, especially in proc-macros, can be challenging, often feeling like the usual tools in your toolbox are limited or unhelpful. Print statements often get lost in compiler output or don't work at all in certain contexts. Stepping through code with a debugger can be tedious and time-consuming with proc macros, especially when dealing with large codebases and complex expansions. It's also so very easy to end up in an all-inclusive stepping tour through the
`syn`, `quote`, and `proc_macro2` crates.

`odebug` provides a simple way to log values, expressions, token streams and similar to files at specific points in your code, fairly ergonomically. After execution, you can examine these logs to understand what happened during compilation or runtime, with the specific file name and the line number for reference automatically collected and included.

## Support

Whether you use this project, have learned something from it, or just like it, please consider supporting it by buying me a coffee, so I can dedicate more time on open-source projects like this :)

<a href="https://buymeacoffee.com/orgrinrt" target="_blank"><img src="https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png" alt="Buy Me A Coffee" style="height: auto !important;width: auto !important;" ></a>

## License

> You can check out the full license [here](https://github.com/orgrinrt/odebug/blob/master/LICENSE)

This project is licensed under the terms of the **MIT** license.
