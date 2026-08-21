//! The `odebug!` macro and the forms it accepts.
//!
//! Fifteen arms used to carry a copy each of the same six-line body, differing only in
//! three expressions: which file, which header, and what to write. They are one arm now,
//! and the rest decide only which three things to hand it. That is not tidying: the copies
//! had already drifted, one of them expanding its arguments with `*` where every other
//! used `+`.

/// Writes one entry. Internal.
///
/// The single place any of the forms below reaches the sink, so a change to what an entry
/// looks like happens once.
#[doc(hidden)]
#[macro_export]
macro_rules! __odebug_emit {
    ($file:expr, $header:expr, $content:expr) => {{
        // `format_args!` rather than a built `String`. It costs nothing, and it is what lets
        // a sink with no allocator write the entry: it formats straight into wherever it
        // writes, instead of being handed something already built out of a heap it does not
        // have.
        // The file writer installs itself on the first entry, so nothing has to be set up
        // to use this macro. A consumer that installed its own sink already keeps it: this
        // only fills an empty slot.
        //
        // Called unconditionally, with the feature decided on this crate's side. A
        // `#[cfg(feature = ..)]` written here would be evaluated where the macro expands,
        // which is the consumer's crate, so it would read the consumer's features and find
        // nothing by that name.
        $crate::install_default_sink();

        let _ = $crate::emit(
            $file,
            $content,
            $header,
            ::core::format_args!("{}:{}", ::core::file!(), ::core::line!()),
        );
    }};
}

/// Logs debug information to a file, and compiles to nothing in release builds.
///
/// It writes to files, which is the point: a procedural macro runs inside the compiler,
/// where `println!` goes somewhere nobody is reading. Each entry carries the file and line
/// it came from.
///
/// Active in debug builds, or in any build with the `always_log` feature. Without either,
/// the whole invocation expands to nothing and its arguments are not evaluated.
///
/// # The forms
///
/// | Written | Goes to | Header |
/// |---|---|---|
/// | `odebug!("text")` | `debug.log` | none |
/// | `odebug!("{} and {}", a, b)` | `debug.log` | none |
/// | `odebug!(::Header("text"))` | `debug.log` | `Header` |
/// | `odebug!(parser::("text"))` | `parser.log` | none |
/// | `odebug!(parser::Header("text"))` | `parser.log` | `Header` |
/// | `odebug!("out.log" => "text")` | `out.log` | none |
/// | `odebug!("text".to_file("out.log"))` | `out.log` | none |
/// | `odebug!("text".with_header("H"))` | `debug.log` | `H` |
/// | `odebug!("text".to_file("out.log").with_header("H"))` | `out.log` | `H` |
///
/// Every one of them also takes format arguments, and every one is asserted in the test
/// suite against the file it claims to write.
///
/// # Examples
///
/// ```
/// use odebug::odebug;
/// odebug!("Simple debug message");
/// odebug!("Formatted message: {}", 42);
/// ```
///
/// A file of its own, and a header:
///
/// ```
/// use odebug::odebug;
/// odebug!("custom.log" => "Message in custom file");
/// odebug!(custom::Header("Message with header"));
/// ```
///
/// The same, written as a chain:
///
/// ```
/// use odebug::odebug;
/// odebug!("Debug info".to_file("output.log"));
/// odebug!("Important message".with_header("IMPORTANT"));
/// odebug!("Error details".to_file("errors.log").with_header("ERROR"));
/// ```
// Two definitions, selected by *this* crate's feature rather than the caller's.
//
// The single definition tested `feature = "always_log"` inside the expansion, and an
// expansion is compiled in the calling crate, so the condition asked whether the *caller*
// had declared a feature by that name. Callers do not: `always_log` belongs to odebug. So
// the feature did nothing for anybody, and rustc reported
// `unexpected cfg condition value: always_log` once per call site. One consumer crate
// carried twenty-eight of those.
//
// `debug_assertions` stays inside the expansion on purpose. That one is meant to be the
// caller's, so a debug build of a crate that depends on a release-built odebug still logs.
#[cfg(feature = "always_log")]
#[macro_export]
macro_rules! odebug {
    ($($args:tt)*) => {
        $crate::__odebug_dispatch!($($args)*)
    };
}

#[cfg(not(feature = "always_log"))]
#[macro_export]
macro_rules! odebug {
    ($($args:tt)*) => {
        #[cfg(debug_assertions)]
        {
            $crate::__odebug_dispatch!($($args)*)
        }
    };
}

/// Decides which file, header and content each form means. Internal.
///
/// Order matters. The more particular shapes come first, because the last two arms match
/// anything at all.
#[doc(hidden)]
#[macro_export]
macro_rules! __odebug_dispatch {
    // A file and a header, named as a path: `parser::Trace("...")`.
    ($file:ident::$header:ident($fmt:expr $(, $arg:tt)* $(,)?)) => {
        $crate::__odebug_emit!(
            ::core::concat!(::core::stringify!($file), ".log"),
            Some(stringify!($header)),
            ::core::format_args!($fmt $(, $arg)*)
        )
    };

    // A file with no header: `parser::("...")`.
    ($file:ident::($fmt:expr $(, $arg:tt)* $(,)?)) => {
        $crate::__odebug_emit!(
            ::core::concat!(::core::stringify!($file), ".log"),
            None,
            ::core::format_args!($fmt $(, $arg)*)
        )
    };

    // A header with no file: `::Trace("...")`.
    (::$header:ident($fmt:expr $(, $arg:tt)* $(,)?)) => {
        $crate::__odebug_emit!("debug.log", Some(stringify!($header)), ::core::format_args!($fmt $(, $arg)*))
    };

    // A file named by a string: `"out.log" => "..."`.
    ($file:expr => $fmt:expr $(, $arg:tt)* $(,)?) => {
        $crate::__odebug_emit!($file, None, ::core::format_args!($fmt $(, $arg)*))
    };

    // Chained, on a literal or a binding. Both spellings exist because a `tt` cannot be
    // followed by `.` in a matcher, so the content has to be captured by fragment kind.
    ($content:literal.to_file($file:expr).with_header($header:expr)) => {
        $crate::__odebug_emit!($file, Some(::core::convert::AsRef::<str>::as_ref(&$header)), ::core::format_args!("{}", $content))
    };
    ($content:ident.to_file($file:expr).with_header($header:expr)) => {
        $crate::__odebug_emit!($file, Some(::core::convert::AsRef::<str>::as_ref(&$header)), ::core::format_args!("{}", $content))
    };
    ($content:literal.to_file($file:expr)) => {
        $crate::__odebug_emit!($file, None, ::core::format_args!("{}", $content))
    };
    ($content:ident.to_file($file:expr)) => {
        $crate::__odebug_emit!($file, None, ::core::format_args!("{}", $content))
    };
    ($content:literal.with_header($header:expr)) => {
        $crate::__odebug_emit!("debug.log", Some(::core::convert::AsRef::<str>::as_ref(&$header)), ::core::format_args!("{}", $content))
    };
    ($content:ident.with_header($header:expr)) => {
        $crate::__odebug_emit!("debug.log", Some(::core::convert::AsRef::<str>::as_ref(&$header)), ::core::format_args!("{}", $content))
    };

    // Plain content, with or without format arguments.
    ($fmt:expr, $($arg:tt)+) => {
        $crate::__odebug_emit!("debug.log", None, ::core::format_args!($fmt, $($arg)+))
    };
    ($content:expr) => {
        $crate::__odebug_emit!("debug.log", None, ::core::format_args!("{}", $content))
    };
}
