//! A variant serde neither writes nor reads, under `#[serde(skip)]` or under `skip_serializing`
//! and `skip_deserializing` together, is described by no surface, under every tagging; a variant
//! serde drops in one direction alone is described as every variant is. A member of a variant
//! serde never reads is read by no hook.

#![cfg(all(
    feature = "serde",
    feature = "typescript",
    feature = "zod",
    feature = "jsonschema"
))]

#[cfg(test)]
#[path = "skipped_variant_tests/models.rs"]
mod models;

#[cfg(test)]
#[path = "std_range_tests/documents.rs"]
mod documents;

#[cfg(test)]
#[path = "skipped_variant_tests/surfaces.rs"]
mod surfaces;

#[cfg(test)]
#[path = "skipped_variant_tests/unread_members.rs"]
mod unread_members;
