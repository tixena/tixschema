//! The Dart expression the emitter writes around the read and the write of a number of an integer
//! type narrower than a Dart `int`, which holds it to the type's range.

/// `number`, held to the range `least` to `most`.
pub fn held(number: &str, least: &str, most: &str) -> String {
    format!(
        "(switch ({number}) {{ final number when number >= {least} && number <= {most} => number, \
         final number => throw FormatException('expected an integer from {least} to {most}, found \
         $number') }})"
    )
}

/// `number`, held to the range of a `u8`.
pub fn u8_held(number: &str) -> String {
    held(number, "0", "255")
}

/// `number`, held to the range of a `u32`.
pub fn u32_held(number: &str) -> String {
    held(number, "0", "4294967295")
}
