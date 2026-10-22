//! The test binary of `dart_tests/`.

extern crate alloc;

#[cfg(test)]
#[cfg(feature = "dart")]
#[path = "dart_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(all(
    feature = "dart",
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[path = "dart_tests/bounded_brands.rs"]
mod bounded_brands;

#[cfg(test)]
#[cfg(all(feature = "dart", feature = "serde"))]
#[path = "dart_tests/validate.rs"]
mod validate;

#[cfg(test)]
#[cfg(all(feature = "dart", feature = "serde"))]
#[path = "dart_tests/read.rs"]
mod read;

#[cfg(test)]
#[cfg(all(feature = "dart", feature = "serde"))]
#[path = "dart_tests/preprocess.rs"]
mod preprocess;

/// A member serde reads and never writes, in a struct of its own, beside a flattened member and in
/// an enum's variant under each tagging that writes a variant's named members.
#[cfg(test)]
#[cfg(feature = "dart")]
#[path = "service_schema_emitted_client_tests/unwritten_models.rs"]
mod unwritten_models;

/// What the Dart emitter writes and reads of a member serde never writes.
#[cfg(test)]
#[cfg(feature = "dart")]
#[path = "dart_tests/unwritten.rs"]
mod unwritten;

/// A map that is the one member of its object, optional too, and a member that names a function
/// flattened at each place a member can be handed fewer keys than its object holds.
#[cfg(test)]
#[cfg(all(feature = "dart", feature = "serde"))]
#[path = "dart_tests/flattened_models.rs"]
mod flattened_models;

/// What the Dart emitter reads of a flattened member, at its place among the flattened members.
#[cfg(test)]
#[cfg(all(feature = "dart", feature = "serde"))]
#[path = "dart_tests/flattened.rs"]
mod flattened;

/// The models a name serde reads beside the one it writes is read under.
#[cfg(test)]
#[cfg(all(feature = "dart", feature = "serde"))]
#[path = "kotlin_tests/aliased_models.rs"]
mod aliased_models;

/// The enums of those models flattened, each beside a member of its holder's own. Declared after
/// the module that declares the enums.
#[cfg(test)]
#[cfg(all(feature = "dart", feature = "serde"))]
#[path = "dart_tests/flattened_aliased_models.rs"]
mod flattened_aliased_models;

/// What the Dart emitter reads of a variant and a member under the `alias` serde reads them under.
#[cfg(test)]
#[cfg(all(feature = "dart", feature = "serde"))]
#[path = "dart_tests/aliased.rs"]
mod aliased;

/// Models whose names on the wire hold a character a Dart string literal reads apart from text.
#[cfg(test)]
#[cfg(all(feature = "dart", feature = "serde"))]
#[path = "dart_tests/quoted_models.rs"]
mod quoted_models;

/// What the Dart emitter writes of a name on the wire that holds such a character.
#[cfg(test)]
#[cfg(all(feature = "dart", feature = "serde"))]
#[path = "dart_tests/quoted.rs"]
mod quoted;
