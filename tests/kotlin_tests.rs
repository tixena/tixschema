//! The test binary of `kotlin_tests/`.

extern crate alloc;

#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "kotlin_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "kotlin_tests/validate.rs"]
mod validate;

/// The two serializers of each type: the read alone, and the type's own, which judges.
#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "kotlin_tests/read.rs"]
mod read;

/// `preprocess`: the functions a type and its members name, as the read of each calls them.
#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "kotlin_tests/preprocess.rs"]
mod preprocess;

/// The models an enum with a variant of several slots is written on, under each tagging, and a
/// tuple in each place a writer names its wrapper more than once.
#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "kotlin_tests/tuple_variants_models.rs"]
mod tuple_variants_models;

/// An enum with a variant of several slots, its slots written and read by one tuple wrapper, and
/// one wrapper per tuple shape in each place a writer names a tuple more than once.
#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "kotlin_tests/tuple_variants.rs"]
mod tuple_variants;

/// A map flattened as the one member of its struct, optional or not and keyed by a plain enum, a
/// flattened struct beside a member of its own, an optional member beside a flattened one with its
/// twin under `nullable`, and a member flattened after a struct or beside a member of its holder's
/// own. Declared after the module that declares the member they flatten.
#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "kotlin_tests/flattened_models.rs"]
mod flattened_models;

/// An optional flattened member whose type the emitter recorded no keys for: a struct declared
/// below its holder with a flattened member of its own, and an enum under each tagging but serde's
/// own.
#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "kotlin_tests/unrecorded_options_models.rs"]
mod unrecorded_options_models;

/// An optional flattened member that names a function: a struct the emitter recorded keys for,
/// under a function that answers the payload as it was handed and under one that answers `null`,
/// and an enum under a tag alone.
#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "kotlin_tests/corrected_options_models.rs"]
mod corrected_options_models;

/// A member serde reads and never writes, in a struct of its own, beside a flattened member and in
/// an enum's variant under each tagging that writes a variant's named members.
#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "service_schema_emitted_client_tests/unwritten_models.rs"]
mod unwritten_models;

/// What the Kotlin emitter writes and reads of a member serde never writes.
#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "kotlin_tests/unwritten.rs"]
mod unwritten;

/// The models a name serde reads beside the one it writes is read under.
#[cfg(test)]
#[cfg(all(feature = "kotlin", feature = "serde"))]
#[path = "kotlin_tests/aliased_models.rs"]
mod aliased_models;

/// A name serde reads beside the one it writes: a variant's alias and a member's.
#[cfg(test)]
#[cfg(all(feature = "kotlin", feature = "serde"))]
#[path = "kotlin_tests/aliased.rs"]
mod aliased;
