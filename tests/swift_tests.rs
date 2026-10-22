//! The test binary of `swift_tests/`.

extern crate alloc;

#[cfg(test)]
#[cfg(feature = "swift")]
#[path = "swift_tests/tests.rs"]
mod tests;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/validate.rs"]
mod validate;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/read.rs"]
mod read;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/preprocess.rs"]
mod preprocess;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/sequences.rs"]
mod sequences;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/generic_variants.rs"]
mod generic_variants;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/generic_helpers.rs"]
mod generic_helpers;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde", feature = "chrono"))]
#[path = "swift_tests/generic_dates.rs"]
mod generic_dates;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/brand_reads.rs"]
mod brand_reads;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde", feature = "chrono"))]
#[path = "swift_tests/brand_date_reads.rs"]
mod brand_date_reads;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde", feature = "chrono"))]
#[path = "swift_tests/text_map_keys_models.rs"]
mod text_map_keys_models;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde", feature = "chrono"))]
#[path = "swift_tests/text_map_keys.rs"]
mod text_map_keys;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde", feature = "chrono"))]
#[path = "swift_tests/date_map_keys_models.rs"]
mod date_map_keys_models;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde", feature = "chrono"))]
#[path = "swift_tests/date_map_keys.rs"]
mod date_map_keys;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/renamed_generics_models.rs"]
mod renamed_generics_models;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/renamed_generics.rs"]
mod renamed_generics;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde", feature = "chrono"))]
#[path = "swift_tests/held_dates_models.rs"]
mod held_dates_models;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde", feature = "chrono"))]
#[path = "swift_tests/held_dates.rs"]
mod held_dates;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/wrapped_maps_models.rs"]
mod wrapped_maps_models;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/wrapped_maps.rs"]
mod wrapped_maps;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/lone_slots_models.rs"]
mod lone_slots_models;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/lone_slots.rs"]
mod lone_slots;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/listed_maps_models.rs"]
mod listed_maps_models;

#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/listed_maps.rs"]
mod listed_maps;

/// A member serde reads and never writes, in a struct of its own, beside a flattened member and in
/// an enum's variant under each tagging that writes a variant's named members.
#[cfg(test)]
#[cfg(feature = "swift")]
#[path = "service_schema_emitted_client_tests/unwritten_models.rs"]
mod unwritten_models;

/// What the Swift emitter writes and reads of a member serde never writes.
#[cfg(test)]
#[cfg(feature = "swift")]
#[path = "swift_tests/unwritten.rs"]
mod unwritten;

/// A map that is the one member of its object, under a key Swift wraps, of models, of dates and
/// naming a function, and a member that names a function at each place a member can be handed
/// fewer keys than its object holds.
#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/flattened_models.rs"]
mod flattened_models;

/// What the Swift emitter reads and writes of a flattened member.
#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/flattened.rs"]
mod flattened;

/// A flattened `Option` beside a member of its holder's own, alone in its holder and in a variant
/// under a tag key, of a struct of two keys, of a struct declared below its holder, of a model
/// `validate()` walks, of a struct that reads the keys no member around it took, read from what a
/// function answers, and of an enum of each tagging and one declared below its holder, and a
/// struct flattened bare beside a member of its holder's own.
#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/flattened_options_models.rs"]
mod flattened_options_models;

/// An optional flattened member that names a function: a struct under a function that answers the
/// payload as it was handed and under one that answers `null`, and an enum under a tag alone.
#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "kotlin_tests/corrected_options_models.rs"]
mod corrected_options_models;

/// What the Swift emitter writes and reads of a flattened `Option`.
#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/flattened_options.rs"]
mod flattened_options;

/// The models a name serde reads beside the one it writes is read under.
#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "kotlin_tests/aliased_models.rs"]
mod aliased_models;

/// A member under an alias in each way the Swift read reads a member under its key.
#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/aliased_members_models.rs"]
mod aliased_members_models;

/// What the Swift emitter reads of a variant and a member under the `alias` serde reads them
/// under.
#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/aliased.rs"]
mod aliased;

/// Models whose names on the wire hold a character a Swift string literal reads apart from text.
#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "dart_tests/quoted_models.rs"]
mod quoted_models;

/// What the Swift emitter writes of a name on the wire that holds such a character.
#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/quoted.rs"]
mod quoted;
