# `odebug`

<div align="center" style="text-align: center;">

[![GitHub Stars](https://img.shields.io/github/stars/orgrinrt/odebug.svg)](https://github.com/orgrinrt/odebug/stargazers)
[![Crates.io](https://img.shields.io/crates/v/odebug)](https://crates.io/crates/odebug)
[![docs.rs](https://img.shields.io/docsrs/odebug)](https://docs.rs/odebug)
[![GitHub Issues](https://img.shields.io/github/issues/orgrinrt/odebug.svg)](https://github.com/orgrinrt/odebug/issues)
![License](https://img.shields.io/github/license/orgrinrt/odebug?color=%23009689)

> A debug logging macro that writes to text files, for proc-macro compilation especially. Ships with the file writer and a sink contract for putting entries somewhere else.

</div>

`odebug!` takes a message the way `println!` does and writes it into a file instead, with the
file and line it was called from attached to the entry, so an entry says where it came from as
well as what happened. Inside a procedural macro that matters more than usual, since the
macro runs inside the compiler and whatever gets printed there lands somewhere nobody is
looking, though it's just as usable from a build script or a test where stdout is being eaten
by something else.

The files go under `target/odebug` in the default build, one per name the call gave, and each
of them is truncated on the first write of a run, so what's in there describes the last build
and not the twenty before it. Nothing needs setting up, as the file writer installs itself on
the first entry. In a release build the whole invocation expands to nothing (the arguments
included, so those aren't evaluated either), unless the `always_log` feature says otherwise.

Where an entry goes is a contract, though, and the file writer is one implementation of it.
A crate without `std` has no files to write to, and a test would rather have the entries in
memory where it can assert on them, so the destination is a `Sink`, the file writer is the
one that ships, and the `no_std` feature compiles that one out and leaves the macro and the
contract. The contract itself is [`notko`](https://crates.io/crates/notko)'s
`Emit`, re-exported here, which is also the one dependency this crate carries.

## Usage

```bash
cargo add odebug
```

The default file is `debug.log`, and the shortest form is the message with its format
arguments after it, exactly as `format!` takes them:

```rust
use odebug::odebug;

let tokens = 12;
odebug!("expansion started");
odebug!("{} tokens in, and the first is {:?}", tokens, "struct");
```

A file of its own and a header come either as a path, where the identifier before `::` names
the file (`.log` gets appended) and the one after it is the header written above the entry,
or as a chain on the message:

```rust
use odebug::odebug;

odebug!(parser::Bindings("what the parser bound"));
odebug!(parser::("the parser's file, no header"));
odebug!(::Timing("a header, in debug.log"));
odebug!("out.log" => "a file named as a string, formatted too: {}", 1);
odebug!("the same, chained".to_file("out.log").with_header("NOTE"));
```

Do note that the chained forms are matched by the macro rather than being method calls on
anything, so the message in that position is a string literal or a plain binding (a `String`
in a variable is fine, it goes through `Display`), and an expression there doesn't match any
of the arms. The file and the header are ordinary expressions.

Where the files end up is decided by two default features. `output_to_target` puts them in
`target/odebug`, honouring `CARGO_TARGET_DIR` where it's set, and without it they go to a
`.debug` directory instead. `use_workspace` makes that the workspace root's `.debug` rather
than the current directory's, the root being the manifest that opens a `[workspace]` table on
a line of its own, so a member crate carrying `[workspace.dependencies]` isn't mistaken for
it. `odebug::debug_dir()` answers with whichever one got resolved.

Every write reaches the operating system before the call returns. The `buffered` feature
keeps each file open behind a buffer instead, which is a good deal faster and loses whatever
is still in the buffer if the process dies, and a process dying is rather often the moment a
debug log is wanted for. With it on, `odebug::flush()` before exiting is what makes the log
complete; without it the call is harmless and does nothing.

## Example

Here's a derive macro that's misbehaving on some input and not on others, which is the usual
shape of it. The whole input goes into `derive.log` under a header, so a broken invocation
can be compared against one that worked, then one line per token as they get walked, and a
summary at the end. In the real crate this sits under `#[proc_macro_derive(Row)]`; the
attribute is left off here because it needs the proc-macro crate type to compile at all.

```rust,no_run
extern crate proc_macro;

use odebug::odebug;
use proc_macro::{Delimiter, TokenStream, TokenTree};

fn derive_row(input: TokenStream) -> TokenStream {
    // the input as one entry, headed, so it's easy to find in the file
    odebug!(derive::Input("{}", input));

    let mut fields = 0;
    for tree in input.clone() {
        // one line per token, in derive.log and with no header
        odebug!(derive::("{:?}", tree));
        if let TokenTree::Group(group) = &tree {
            if group.delimiter() == Delimiter::Brace {
                // a field per comma, near enough for a debug count
                fields = group.stream().into_iter().filter(|t| matches!(t, TokenTree::Punct(p) if p.as_char() == ',')).count() + 1;
            }
        }
    }

    // a header on the default file, so the summary is not lost among the tokens
    odebug!(::Summary("{} fields counted", fields));

    "impl Row for Unit {}".parse().expect("a literal impl parses")
}
```

After a build, `target/odebug/derive.log` opens on the input, set off by a rule and carrying
the header and the call site, and `debug.log` holds the summary the same way:

```text

-----------------------------------------------------------
> Input (src/lib.rs:9)
-----------------------------------------------------------
struct Row { id : u32, name : String }
```

An entry with no header still gets the call site, as `> [at src/lib.rs:14]`, and a plain
message with neither is written as is, on a line of its own.

## Motivation

Debugging complex code flows, especially in proc-macros, can be challenging, often feeling
like the usual tools are limited or unhelpful. Print statements often get lost in compiler
output or don't work at all in certain contexts. Stepping through code with a debugger can be
tedious and time-consuming with proc macros, especially when dealing with large codebases and
complex expansions. It's also so very easy to end up in an all-inclusive stepping tour through
the `syn`, `quote`, and `proc_macro2` crates.

`odebug` writes values, expressions, token streams and similar to files at chosen points
instead, and each entry carries the file and line it came from without being asked. After the
build, the log says what happened, and as it's a file it's still there once the compiler has
exited.

Logging during expansion means a build can write thousands of lines, so the per-line cost is
about the whole cost of it. The file is kept open between writes for that reason, and the
crate's own bench (`benches/write.rs`, which keeps every alternative as an arm of its own)
has the shipped path coming out a few times faster per line than opening the file for every
write, which is the obvious way to do it, with a buffer on top taking another order of
magnitude or two off. That last one is what the `buffered` feature turns on, and the default
stays unbuffered for the reason given above. The absolute numbers move a lot with the
machine and with whatever else it happens to be doing, so none are quoted here; `cargo bench`
answers for the machine it runs on.

## Extras

### Status

Early days still, so the api hasn't settled and the next release can move things out from
under whatever was written against this one, the sink contract in particular. Every release
is tagged and the log between two tags is what moved. I'd caution against building anything
serious on it, though a debug log is rarely that.

It builds on stable from 1.85 onwards, which is what the `rust-version` in the manifest says,
and needs no nightly anywhere.

### Cargo features

| Feature | Default | Effect |
|---|---|---|
| `use_workspace` | on | Resolves the `.debug` directory from the workspace root rather than the current directory. |
| `output_to_target` | on | Writes under `target/odebug`, honouring `CARGO_TARGET_DIR`, instead of `.debug`. |
| `always_log` | off | Logs in release builds too. Without it the macro expands to nothing outside a debug build. |
| `buffered` | off | Keeps each file open behind a buffer. Faster, and loses what is unflushed if the process dies. |
| `no_std` | off | Compiles the file writer out, along with the flags above that only say where it writes. What's left is the macro and the `Sink` contract. |
| `no_alloc` | off | Implies `no_std`. Adds nothing and takes nothing away, and says that the path from a call to a sink allocates nowhere. |

`no_alloc` switches nothing, since the macro builds no `String` under any selection; an
entry is a `core::fmt::Arguments` and the sink formats it into wherever it writes, so the
feature is there to say so where a consumer wants that stated. The `test_suite_*` features in the manifest belong to the test matrix and do nothing
for a consumer.

### Sinks

A sink is an `Emit<Entry<'_>>` with `Error` as its error, plus an empty `impl Sink` line.
`Emit` is `notko`'s and re-exported, so implementing one names this crate only:

```rust
use odebug::{install_sink, Emit, Entry, Error, Outcome, Sink};

struct Discard;

impl Emit<Entry<'_>> for Discard {
    type Err = Error;

    fn emit(&self, _entry: Entry<'_>) -> Outcome<(), Self::Err> {
        Outcome::Ok(())
    }
}

impl Sink for Discard {}

install_sink!(Discard);
```

`emit` takes `&self` because the sink is installed once and reached from anywhere, so nobody
holds it exclusively, and it is fallible because a write fails for reasons the caller neither
caused nor can act on. An entry's `content` and `origin` arrive as `core::fmt::Arguments`,
which is what `format_args!` produces, so a sink with a fixed buffer formats straight into
it and no `String` exists on the way. The `target` is the file name the writer would have
used and `header` the label the call gave, and a sink decides what either means; one writing
to a serial port might keep neither.

`install_sink!` takes a value and declares the static holding it, so the value has to be
constructible in a static initialiser, and a sink that needs something at runtime keeps it
behind whatever interior mutability suits. Installing one replaces the file writer rather
than adding to it, and the writer installs itself only where nothing else has. `Sink` has a
`flush` that does nothing by default, which is why the empty impl line isn't given as a
blanket impl: a sink holding something wants to override it, and a type can't override a
method of an impl it didn't write. The `examples/` directory has a working in-memory sink,
and another file that goes through every form the macro takes.

### Limitations

There are no levels and no timestamps, and probably won't be, as the crate is
for reading one build's log after the fact rather than for running a service. An entry goes
where the call said and that's about all the structure there is.

`Error` carries no payload on purpose, so nothing downstream can branch on why a write
failed; the file writer prints the underlying `io::Error` to stderr and moves on. Under
`no_std` with no sink installed an entry is dropped without a word, since there is nowhere to
report it to either.

A file is truncated by the first write of a process, and a workspace build runs one compiler
process per crate, so two proc-macro crates logging into the same name at the same time will
write over each other. Naming the file per crate, with the path form or `=>`, is the way
around that.

## Support

Feel free to contribute! If unsure about wasting work, the best practice is to throw in an issue describing what you'd do, and only then commit to writing a big PR, because chances are, it might not be something that belongs here. However, forks are always a valid choice and we'd encourage everyone to experiment and have their own takes on this. When doing this, do mind the license(s) though!

Whether you use this project, have learned something from it, or just like it, please consider supporting it by buying me a coffee, so I can dedicate more time on open-source projects like this :)

<a href="https://buymeacoffee.com/orgrinrt" target="_blank"><img src="https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png" alt="Buy Me A Coffee" style="height: auto !important;width: auto !important;" ></a>

## License

> The project is licensed under the **Mozilla Public License 2.0**.

`SPDX-License-Identifier: MPL-2.0`

> You can check out the full license [here](https://github.com/orgrinrt/odebug/blob/main/LICENSE)
