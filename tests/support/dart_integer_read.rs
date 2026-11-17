//! The Dart text the emitter reads an integer with: the digits of an integer a `u64` holds above an
//! `i64` and a float refused in serde's words, any other value cast to an `int`.

include!("../../src/features/dart/integer_read.rs");

/// The read of the integer `expr` of the Rust type `expected`, `int` written as `int_name`.
pub fn integer_read_named(expr: &str, expected: &str, int_name: &str) -> String {
    let beyond = format!("invalid value: integer `${{number}}`, expected {expected}");
    [
        ("@EXPR@", expr),
        ("@DOUBLE@", "double"),
        ("@INT@", int_name),
        ("@NUMBER@", "number"),
        ("@SPELLED@", "spelled"),
        ("@HELD@", "held"),
        ("@EXPECTED@", expected),
        ("@BEYOND@", &beyond),
        ("@OUT@", "number out of range"),
    ]
    .into_iter()
    .fold(INTEGER_READ.to_owned(), |text, (from, to)| {
        text.replace(from, to)
    })
}

/// The read of the integer `expr` of the Rust type `expected`.
pub fn integer_read(expr: &str, expected: &str) -> String {
    integer_read_named(expr, expected, "int")
}
