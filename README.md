# odebug

<div style="text-align: center;">

[![GitHub Stars](https://img.shields.io/github/stars/orgrinrt/odebug.svg)](https://github.com/orgrinrt/odebug/stargazers)
[![Crates.io Total Downloads](https://img.shields.io/crates/d/odebug)](https://crates.io/crates/odebug)
[![GitHub Issues](https://img.shields.io/github/issues/orgrinrt/odebug.svg)](https://github.com/orgrinrt/odebug/issues)
[![Current Version](https://img.shields.io/badge/version-0.2.0-red.svg)](https://github.com/orgrinrt/odebug)

> Debug logging utility that writes to text files, practical especially during proc-macro compilation.

</div>

## Features

- A macro that logs to files, carrying the file and line it was written at
- Configurable output location: project root, workspace root, or the target directory
- Works during proc-macro expansion, where print output goes somewhere nobody reads
- No dependencies at all
- Compiles to nothing outside debug builds, unless `always_log` says otherwise

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

- `use_workspace` (default): resolves paths from the workspace root rather than the
  current directory. The root is the manifest that opens a `[workspace]` table of its own,
  matched as a whole line, so a member crate carrying `[workspace.dependencies]` is not
  mistaken for it.
- `output_to_target` (default): puts the files in `target/odebug`, honouring
  `CARGO_TARGET_DIR`, rather than in `.debug`
- `always_log`: logs in release builds too
- `buffered`: keeps each file open behind a buffer. See below; it is not the default and
  the reason is worth reading before turning it on.
- `no_std`: compiles the file writer out, along with the three flags that only say where it
  puts its files. What is left is the macro and the destination contract. Nothing is installed
  by default, because there is nothing sensible to default to, and an entry written before a
  sink is installed is reported as a refusal and dropped.
- `no_alloc`: implies `no_std`. The macro builds no `String` under any selection, so this adds
  nothing and takes nothing away. It states that the path from a call to a sink allocates
  nowhere.

## Somewhere other than a file

Files are what a procedural macro needs, since it runs inside the compiler where `println!`
goes somewhere nobody is reading. That is a good default and a poor requirement: a crate
without `std` has no files, one on a microcontroller has a serial port or a ring buffer, and a
test wants the entries in memory where it can assert on them.

So the destination is a contract, and it is `notko::sink::Emit` rather than one invented here.
Receiving an item is a stack-wide shape; a crate that writes its own is a crate nobody else's
code composes with. It is re-exported, so implementing a sink names this crate and not notko.

```rust
use odebug::{Emit, Entry, Error, Outcome, Sink};

struct Discard;

impl Emit<Entry<'_>> for Discard {
    type Err = Error;

    fn emit(&self, _entry: Entry<'_>) -> Outcome<(), Self::Err> {
        Outcome::Ok(())
    }
}

impl Sink for Discard {}
```

`&self` because the sink is installed once and reached from anywhere, so nobody holds it
exclusively. Fallible because a write fails for reasons the caller neither caused nor can act
on. An entry's `content` and `origin` are `core::fmt::Arguments`, which is what `format_args!`
produces and costs nothing to build, so a sink formats straight into wherever it writes and no
`String` exists on the way.

`install_sink!(Discard)` puts it in place, and
[`examples/your_own_sink.rs`](examples/your_own_sink.rs) is a working one that keeps entries in
memory. [`examples/every_call_form.rs`](examples/every_call_form.rs) names every shape the
macro accepts.

## What a line costs

Logging during expansion means a build can emit thousands of lines, so the per-line cost
is the whole cost. From `benches/write.rs`, which keeps every alternative as an arm:

| | per line |
|---|---:|
| opening the file each time, as 0.1 shipped | 23.8 µs |
| the same without the redundant `create_dir_all` | 23.3 µs |
| keeping the file open, which is what 0.2 does | **5.0 µs** |
| keeping it open behind a buffer (`buffered`) | 0.2 µs |

The last row is a hundred times faster again and is **not** the default, because a buffer
loses whatever it still holds when the process dies, and a process dying is when a debug
log earns its keep. Without the feature every line has already reached the operating
system by the time the call returns. Turn it on for bulk logging from something that will
exit tidily, and call `odebug::flush()` before it does.

A log file is truncated on the first write of a run, so it describes one run rather than
accumulating across them.

## The Problem

Debugging complex code flows, especially in proc-macros, can be challenging, often feeling like the usual tools in your toolbox are limited or unhelpful. Print statements often get lost in compiler output or don't work at all in certain contexts. Stepping through code with a debugger can be tedious and time-consuming with proc macros, especially when dealing with large codebases and complex expansions. It's also so very easy to end up in an all-inclusive stepping tour through the
`syn`, `quote`, and `proc_macro2` crates.

`odebug` writes values, expressions and token streams to files at chosen points, and each
entry carries the file and line it came from without being asked. After the build, the log
says what happened.

## Support

Whether you use this project, have learned something from it, or just like it, please consider supporting it by buying me a coffee, so I can dedicate more time on open-source projects like this :)

<a href="https://buymeacoffee.com/orgrinrt" target="_blank"><img src="https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png" alt="Buy Me A Coffee" style="height: auto !important;width: auto !important;" ></a>

## License

> You can check out the full license [here](https://github.com/orgrinrt/odebug/blob/main/LICENSE)

This project is licensed under the terms of the **MPL-2.0** license.
