# `odebug`

[![Crates.io](https://img.shields.io/crates/v/odebug)](https://crates.io/crates/odebug) [![docs.rs](https://img.shields.io/docsrs/odebug)](https://docs.rs/odebug)

> A debug logging macro that writes to text files, made for proc-macro compilation, with a sink contract for sending entries somewhere else.

`odebug!` takes a message the way `println!` does and writes it to a file, with the file and line it was called from attached. Inside a procedural macro that matters, since the macro runs inside the compiler and whatever it prints lands where nobody looks, and it works as well in a build script or a test where stdout is being eaten. Files go under `target/odebug`, one per name the call gave, and each is truncated on the first write of a run so it describes the last build. In a release build the whole call expands to nothing, arguments included, unless the `always_log` feature is on.

## Usage

```bash
cargo add odebug
```

```rust
use odebug::odebug;

odebug!("expansion started");
odebug!("{} tokens in, and the first is {:?}", 12, "struct");
```

The default file is `debug.log`. A path form puts the entry in a file of its own under a header, where the identifier before `::` names the file and the one after it is the header, or the same can be chained on the message.

```rust
use odebug::odebug;

odebug!(parser::Bindings("what the parser bound"));
odebug!(::Timing("a header, in debug.log"));
odebug!("out.log" => "a file named as a string, formatted too: {}", 1);
odebug!("the same, chained".to_file("out.log").with_header("NOTE"));
```

The chained forms are matched by the macro and are not method calls, so the message there is a string literal or a plain binding. Every write reaches the operating system before the call returns, and the `buffered` feature keeps each file behind a buffer instead, which is a good deal faster and loses what is unflushed if the process dies, so `odebug::flush()` before exiting makes the log complete.

## Sinks

Where an entry goes is a contract, and the file writer is the one implementation that ships. A sink is an `Emit<Entry<'_>>` from [`notko`](https://crates.io/crates/notko) plus an empty `impl Sink`, and `install_sink!` installs it in place of the file writer. An entry's content arrives as `core::fmt::Arguments`, so a sink with a fixed buffer formats straight into it. The `examples/` directory has an in-memory sink.

## Features

`use_workspace` and `output_to_target` are on by default and decide where the files land, in `target/odebug` honouring `CARGO_TARGET_DIR`, or in a `.debug` directory at the workspace root. `always_log` keeps the macro in release builds, `buffered` is described above, `no_std` compiles the file writer out and leaves the macro and the contract, and `no_alloc` implies `no_std` and only states that nothing on the way allocates.

## Status

The api hasn't settled and the sink contract in particular may move. It builds on stable from rust 1.85. There are no levels and no timestamps, and a workspace build runs one compiler process per crate, so two proc-macro crates logging to the same file name will overwrite each other, which naming the file per crate avoids.

## License

> The project is licensed under the **Mozilla Public License 2.0**.

`SPDX-License-Identifier: MPL-2.0`

> You can check out the full license [here](https://github.com/orgrinrt/odebug/blob/main/LICENSE)
