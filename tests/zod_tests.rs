//! Tests for Zod v4 schema generation — the `zod` feature's validation schemas from Rust types,
//! covering primitives, optional fields, arrays, maps, nested structs, plain and discriminated
//! union enums, TypeScript integration, serde renames, and `ObjectId` support.

#[cfg(test)]
#[cfg(all(feature = "serde", feature = "zod"))]
#[path = "zod_tests/tests.rs"]
mod tests;
