//! What rustc prints for a type whose flattened members serde would not read back where they are
//! declared, or as strict as they are: a member that reads every key and refuses one ahead of
//! another, two such members, a member beside its holder's own tag, a holder its author closed
//! over one, a map beside another member, two sources of one key, a key one member reads that a
//! later one writes, an `Option` serde reads back as `Some`, an untagged variant that reads every
//! object ahead of another, a filling that breaks one of these, the placements the Dart and Swift
//! reads cannot see through, and each of them for a member declared below its holder, in another
//! crate or never expanded. Each case in `flatten_order_refusals/` is a crate of its own, held to
//! the `.stderr` file beside it.

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
        TestCases::new().compile_fail("tests/flatten_order_refusals/*.rs");
    }
}
