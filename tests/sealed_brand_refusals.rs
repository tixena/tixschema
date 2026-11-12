//! What rustc prints for each way of building, editing or declaring a brand that the seal closes.
//! Each case in `sealed_brand_refusals/` is a crate of its own, held to the `.stderr` file beside
//! it.

// Every `.stderr` was written with every feature on, the build the gate runs.
#![cfg(all(
    feature = "serde",
    feature = "zod",
    feature = "typescript",
    feature = "dart",
    feature = "swift",
    feature = "kotlin",
    feature = "mongodb",
    feature = "chrono",
    feature = "indexmap"
))]

#[cfg(test)]
mod tests {
    use trybuild::TestCases;

    #[test]
    fn each_case_fails_with_the_errors_written_beside_it() {
        TestCases::new().compile_fail("tests/sealed_brand_refusals/*.rs");
    }
}
