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

/// A member serde reads and never writes, in a struct of its own and beside a flattened member.
#[cfg(test)]
#[cfg(feature = "swift")]
#[path = "service_schema_emitted_client_tests/unwritten_models.rs"]
mod unwritten_models;

/// What the Swift emitter writes and reads of a member serde never writes.
#[cfg(test)]
#[cfg(feature = "swift")]
#[path = "swift_tests/unwritten.rs"]
mod unwritten;

/// A map flattened after a struct, ahead of one, beside another map, after a member that takes
/// none of the keys it reads, under a key Swift wraps, of models, of dates and naming a function.
#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/flattened_models.rs"]
mod flattened_models;

/// What the Swift emitter reads and writes of a flattened map, at its place among the flattened
/// members.
#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/flattened.rs"]
mod flattened;
