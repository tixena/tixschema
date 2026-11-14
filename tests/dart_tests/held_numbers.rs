//! The Dart expression the emitter writes around the read and the write of a number of an integer
//! type narrower than a Dart `int`, which holds it to the type's range.

#[cfg(feature = "serde")]
use super::dart_integer_read::integer_read;

/// `number`, held to the range `least` to `most`; one written `{expr} as int` is the read of
/// `expr`.
#[cfg(feature = "serde")]
pub fn held(number: &str, least: &str, most: &str) -> String {
    let expected = match (least, most) {
        ("0", "255") => "u8",
        ("0", "4294967295") => "u32",
        _ => "an integer",
    };
    let read = number
        .strip_suffix(" as int")
        .map_or_else(|| number.to_owned(), |expr| integer_read(expr, expected));
    format!(
        "(switch ({read}) {{ final number when number >= {least} && number <= {most} => number, \
         final number => throw FormatException('expected an integer from {least} to {most}, found \
         $number') }})"
    )
}

/// `number`, held to the range of a `u8`.
#[cfg(feature = "serde")]
pub fn u8_held(number: &str) -> String {
    held(number, "0", "255")
}

/// `number`, held to the range of a `u32`.
#[cfg(feature = "serde")]
pub fn u32_held(number: &str) -> String {
    held(number, "0", "4294967295")
}
