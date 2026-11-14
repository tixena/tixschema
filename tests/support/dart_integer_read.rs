//! The Dart text the emitter reads an integer with: a float refused in serde's words, any other
//! value cast to an `int`.

/// The read of the integer `expr` of the Rust type `expected`, `int` written as `int_name`.
pub fn integer_read_named(expr: &str, expected: &str, int_name: &str) -> String {
    [
        ("@EXPR@", expr),
        ("@DOUBLE@", "double"),
        ("@INT@", int_name),
        ("@NUMBER@", "number"),
        ("@SPELLED@", "spelled"),
        ("@HELD@", "held"),
        ("@EXPECTED@", expected),
        ("@OUT@", "number out of range"),
    ]
    .into_iter()
    .fold(
        include_str!("../../src/features/dart/integer_read.txt").to_owned(),
        |text, (from, to)| text.replace(from, to),
    )
}

/// The read of the integer `expr` of the Rust type `expected`.
pub fn integer_read(expr: &str, expected: &str) -> String {
    integer_read_named(expr, expected, "int")
}
