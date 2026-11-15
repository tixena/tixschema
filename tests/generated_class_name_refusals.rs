//! What rustc prints for a type whose published name is the name of a refusal or draft class that
//! another type writes in the Dart, Swift and Kotlin texts, declared before it or after it, and
//! for a renamed type whose refusal class under its Rust name is taken. Each case in
//! `generated_class_name_refusals/` is a crate of its own, held to the `.stderr` file beside it,
//! the same under each client feature.

#![cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]

#[cfg(test)]
mod tests {
    use trybuild::TestCases;

    #[test]
    fn each_case_fails_with_the_errors_written_beside_it() {
        TestCases::new().compile_fail("tests/generated_class_name_refusals/*.rs");
    }
}
