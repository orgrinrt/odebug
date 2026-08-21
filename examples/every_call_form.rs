//! Every shape `odebug!` accepts, and where each one puts what it was given.
//!
//! An entry has three pieces the call can decide: the file it goes to, an optional header,
//! and the content. The forms below are the ways of naming them, from the shortest to the
//! most explicit, and they exist because a debug line is written in a hurry and read once.
//!
//! Files land in `.debug/` beside the workspace root under the default features. This
//! example prints where that is and then lists what it wrote, so the output is checkable
//! rather than a claim.

use odebug::odebug;

fn main() {
    let value = 41;
    let message = String::from("held in a variable");

    // The default file, which is `debug.log`.
    odebug!("the shortest form");
    odebug!("formatted, so a value can go in: {}", value + 1);

    // A file named by a string, on the left of `=>`.
    odebug!("parser.log" => "a line for the parser's own file");
    odebug!("parser.log" => "and a formatted one: {}", value);

    // A file named by an identifier, with a header after `::`. The header is the label an
    // entry gets in the file, which is what makes a log skimmable.
    odebug!(resolver::Bindings("what the resolver bound"));
    odebug!(resolver::Bindings("with a value: {}", value));

    // The same, with one half left out. An identifier and no header, then a header on the
    // default file.
    odebug!(resolver::("no header, still the resolver's file"));
    odebug!(::Timing("a header on the default file"));

    // The method-ish forms, which read left to right and take expressions rather than
    // identifiers. `to_file` and `with_header` compose in either presence.
    odebug!("a literal".to_file("codegen.log"));
    odebug!("a literal".with_header("NOTE"));
    odebug!("a literal".to_file("codegen.log").with_header("NOTE"));

    // The same three, with the content in a variable rather than written inline.
    odebug!(message.to_file("codegen.log"));
    odebug!(message.with_header("NOTE"));
    odebug!(message.to_file("codegen.log").with_header("NOTE"));

    // Nothing above returns anything or can fail from the caller's side. A refusal reaches
    // stderr and the program carries on, because a debug log that halts what it is debugging
    // has stopped being a debug log.
    // Nothing above returns anything or can fail from the caller's side. A refusal reaches
    // stderr and the program carries on, because a debug log that halts what it is debugging
    // has stopped being a debug log.
    let dir = odebug::debug_dir();
    println!("wrote into {}", dir.display());

    // Named rather than listed, because the directory is shared: the test suite writes into
    // it too, and a listing would report whatever happened to be there.
    for file in ["debug.log", "parser.log", "resolver.log", "codegen.log"] {
        let path = dir.join(file);
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        println!("  {file}: {size} bytes");
    }
}
