//! The emitted clients put through a real runtime, with a real message object.

#![cfg(all(feature = "serde", feature = "zod", feature = "typescript"))]

extern crate alloc;

#[cfg(test)]
#[macro_use]
#[path = "service_schema_emitted_client_tests/tests.rs"]
mod tests;

/// The `http_rest` dispatcher for `ThumbnailClientService`, the Rust twin the `bytes` body kind
/// is measured against — in a module of its own, the placement every dispatcher macro needs.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/thumbnail_http_rest_transport.rs"]
mod thumbnail_http_rest_transport;

/// The `http_rest` dispatcher for `ContentClientService`, the Rust twin the `stream` body kind
/// is measured against.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/content_http_rest_transport.rs"]
mod content_http_rest_transport;

/// The `http_rest` dispatcher for `UploadDocumentClientService`, the Rust twin the `multipart`
/// body kind is measured against.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/upload_document_http_rest_transport.rs"]
mod upload_document_http_rest_transport;

/// The `http_rest` dispatcher for `EchoClientService`, the Rust twin the `header_in` echo group
/// is measured against.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/echo_http_rest_transport.rs"]
mod echo_http_rest_transport;

/// The `http_rest` dispatcher for `PulseClientService`, the Rust twin the bodyless-empty-message
/// group is measured against.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/pulse_http_rest_transport.rs"]
mod pulse_http_rest_transport;

/// The `http_rest` dispatcher for `LookupClientService`, the Rust twin the reserved-word group's
/// requests are read by.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/lookup_http_rest_transport.rs"]
mod lookup_http_rest_transport;

/// The `ws_rpc` dispatcher and client for `StampClientService`, the Rust twins the headers groups
/// are measured against.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/stamp_ws_rpc_transport.rs"]
mod stamp_ws_rpc_transport;

#[cfg(test)]
#[path = "service_schema_emitted_client_tests/stamp_ws_rpc_client.rs"]
mod stamp_ws_rpc_client;

#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_node.rs"]
mod run_node;

#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_node_ws_headers.rs"]
mod run_node_ws_headers;

#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_node_ws_server.rs"]
mod run_node_ws_server;

/// The emitted `http_rest` server: the seven design requests, the reader forms, the query-read
/// message, and the three body kinds compared against the Rust twins above.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_node_http_service.rs"]
mod run_node_http_service;

#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart.rs"]
mod run_dart;

#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_ws.rs"]
mod run_dart_ws;

#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_ws_headers.rs"]
mod run_dart_ws_headers;

/// The `validate()` of each emitted Dart type, run beside Rust's own.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_validate.rs"]
mod run_dart_validate;

/// The read of each emitted Dart type, which refuses what its `validate()` refuses.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_read.rs"]
mod run_dart_read;

/// A service named after reserved words, called through each mobile client and read by the Rust
/// dispatcher.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_reserved.rs"]
mod run_reserved;

#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift.rs"]
mod run_swift;

#[cfg(test)]
#[cfg(slow_tests)]
#[path = "service_schema_emitted_client_tests/run_swift_ws_headers.rs"]
mod run_swift_ws_headers;

/// The emitted `validate()` of each shape, run and compared with the Rust one.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_validate.rs"]
mod run_swift_validate;

#[cfg(all(test, feature = "swift"))]
#[path = "service_schema_emitted_client_tests/swift_validate_models.rs"]
mod swift_validate_models;

/// The emitted read of each shape, run and compared with the Rust `validate()` and the Rust read.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_read.rs"]
mod run_swift_read;

#[cfg(all(test, feature = "swift"))]
#[path = "service_schema_emitted_client_tests/swift_read_models.rs"]
mod swift_read_models;

#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin.rs"]
mod run_kotlin;

#[cfg(test)]
#[cfg(slow_tests)]
#[path = "service_schema_emitted_client_tests/run_kotlin_ws_headers.rs"]
mod run_kotlin_ws_headers;

/// `validate()` of the emitted Kotlin types, run and held to what Rust's answers.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_validate.rs"]
mod run_kotlin_validate;

/// The read of the emitted Kotlin types, which refuses what `validate()` refuses, and the fault
/// each client answers for such a reply.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_read.rs"]
mod run_kotlin_read;

#[cfg(test)]
#[path = "service_schema_emitted_client_tests/runtime.rs"]
mod runtime;

/// The models of the `preprocess` tests: the table the Rust read and the Zod schemas are held to.
#[cfg(all(test, any(feature = "dart", feature = "swift")))]
#[path = "pattern_preprocess_tests/models.rs"]
mod preprocess_models;

#[cfg(all(test, feature = "swift"))]
#[path = "service_schema_emitted_client_tests/swift_preprocess_models.rs"]
mod swift_preprocess_models;

/// The emitted read of a type that names a `preprocess`, run and held to that table.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_preprocess.rs"]
mod run_swift_preprocess;

/// The read of each emitted Dart type that names a `preprocess`. Declared last: its models take
/// names the modules above declare, and an expansion reads a type by its name.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_preprocess.rs"]
mod run_dart_preprocess;

/// `preprocess` in the emitted Kotlin types: the functions a type and its members name, called
/// by the read ahead of what it reads. Its models take names no other module of this binary
/// declares.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_preprocess.rs"]
mod run_kotlin_preprocess;

/// The models a key under `#[serde(default)]` is written on, and what a serde read prints for
/// each payload of the table every surface is held to.
#[cfg(all(test, any(feature = "dart", feature = "swift", feature = "kotlin")))]
#[path = "serde_default_tests/models.rs"]
mod serde_default_models;

/// A key under `#[serde(default)]` the payload leaves out, read by the emitted Dart types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_serde_default.rs"]
mod run_dart_serde_default;

/// A key under `#[serde(default)]` the payload leaves out, read by the emitted Swift types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_serde_default.rs"]
mod run_swift_serde_default;

/// A std sequence other than `Vec`, read and written by the emitted Swift types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_sequences.rs"]
mod run_swift_sequences;

/// A key under `#[serde(default)]` the payload leaves out, read by the emitted Kotlin types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_serde_default.rs"]
mod run_kotlin_serde_default;

/// A tuple and a map under a key Swift wraps, held by a type that declares a parameter, read and
/// written by the emitted Swift types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_generic_helpers.rs"]
mod run_swift_generic_helpers;

/// A date held by a type that declares a parameter, read and written by the emitted Swift types.
#[cfg(test)]
#[cfg(feature = "chrono")]
#[path = "service_schema_emitted_client_tests/run_swift_generic_dates.rs"]
mod run_swift_generic_dates;

/// A brand whose value Swift converts, a map under a key Swift wraps or a date, read and written by
/// the emitted Swift types.
#[cfg(test)]
#[cfg(feature = "chrono")]
#[path = "service_schema_emitted_client_tests/run_swift_brand_reads.rs"]
mod run_swift_brand_reads;

/// The models a map under a key Swift holds as its text is written on.
#[cfg(all(test, feature = "swift", feature = "chrono"))]
#[path = "swift_tests/text_map_keys_models.rs"]
mod text_map_keys_models;

/// A map under a key Swift holds as the text serde writes for it, read and written by the emitted
/// Swift types.
#[cfg(test)]
#[cfg(feature = "chrono")]
#[path = "service_schema_emitted_client_tests/run_swift_text_map_keys.rs"]
mod run_swift_text_map_keys;

/// The models a map keyed by a date is written on.
#[cfg(all(test, feature = "swift", feature = "chrono"))]
#[path = "swift_tests/date_map_keys_models.rs"]
mod date_map_keys_models;

/// A map keyed by a date, read and written by the emitted Swift types.
#[cfg(test)]
#[cfg(feature = "chrono")]
#[path = "service_schema_emitted_client_tests/run_swift_date_map_keys.rs"]
mod run_swift_date_map_keys;

/// The models a date held in a slot, as a map's value and as a type argument is written on.
#[cfg(all(test, feature = "swift", feature = "chrono"))]
#[path = "swift_tests/held_dates_models.rs"]
mod held_dates_models;

/// A date held in a slot, as a map's value and as a type argument, read and written by the emitted
/// Swift types.
#[cfg(test)]
#[cfg(feature = "chrono")]
#[path = "service_schema_emitted_client_tests/run_swift_held_dates.rs"]
mod run_swift_held_dates;

/// The models a map under a key Swift wraps, held in a slot and as a map's value, is written on.
#[cfg(all(test, feature = "swift"))]
#[path = "swift_tests/wrapped_maps_models.rs"]
mod wrapped_maps_models;

/// A map under a key Swift wraps, held in a slot and as a map's value, read and written by the
/// emitted Swift types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_wrapped_maps.rs"]
mod run_swift_wrapped_maps;

/// The models a tuple struct that declares one slot is written on.
#[cfg(all(test, feature = "swift"))]
#[path = "swift_tests/lone_slots_models.rs"]
mod lone_slots_models;

/// A tuple struct that declares one slot, which serde writes as the slot's value, read and written
/// by the emitted Swift types wherever it is held.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_lone_slots.rs"]
mod run_swift_lone_slots;

/// The models a type that declares a parameter and publishes under another name is written on.
#[cfg(all(test, feature = "swift"))]
#[path = "swift_tests/renamed_generics_models.rs"]
mod renamed_generics_models;

/// A type that declares a parameter and publishes under another name, read and written by the
/// emitted Swift types through each of its names.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_renamed_generics.rs"]
mod run_swift_renamed_generics;

/// The models a map under a key Swift wraps, in a list, in a list of optionals, in an optional list
/// and in an optional, is written on.
#[cfg(all(test, feature = "swift"))]
#[path = "swift_tests/listed_maps_models.rs"]
mod listed_maps_models;

/// A map under a key Swift wraps, in a list, in a list of optionals, in an optional list and in an
/// optional, read and written by the emitted Swift types wherever it is held.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_listed_maps.rs"]
mod run_swift_listed_maps;

/// The models the four std ranges are written on, and the payloads serde writes and refuses.
#[cfg(all(test, any(feature = "dart", feature = "swift", feature = "kotlin")))]
#[path = "std_range_tests/models.rs"]
mod range_models;

/// The payloads each client reads a range from, and what the serde read prints of each.
#[cfg(all(test, any(feature = "dart", feature = "swift", feature = "kotlin")))]
#[path = "service_schema_emitted_client_tests/range_rows.rs"]
mod range_rows;

/// The four std ranges, read and written by the emitted Dart types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_ranges.rs"]
mod run_dart_ranges;

/// The four std ranges, read and written by the emitted Swift types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_ranges.rs"]
mod run_swift_ranges;

/// The four std ranges, read and written by the emitted Kotlin types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_ranges.rs"]
mod run_kotlin_ranges;

/// The models a std `Result` is written on, and the payloads serde writes and refuses.
#[cfg(all(test, any(feature = "dart", feature = "swift", feature = "kotlin")))]
#[path = "std_result_tests/models.rs"]
mod result_models;

/// The payloads each client reads a `Result` from, and what the serde read prints of each.
#[cfg(all(test, any(feature = "dart", feature = "swift", feature = "kotlin")))]
#[path = "service_schema_emitted_client_tests/result_rows.rs"]
mod result_rows;

/// The std `Result`, read and written by the emitted Dart types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_results.rs"]
mod run_dart_results;

/// The std `Result`, read and written by the emitted Swift types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_results.rs"]
mod run_swift_results;

/// The std `Result`, read and written by the emitted Kotlin types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_results.rs"]
mod run_kotlin_results;

/// The models a set is written on, and the payloads that repeat an item of one.
#[cfg(all(test, any(feature = "dart", feature = "swift", feature = "kotlin")))]
#[path = "set_tests/models.rs"]
mod set_models;

/// A set that repeats an item, refused by the read of the emitted Dart types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_sets.rs"]
mod run_dart_sets;

/// A set that repeats an item, refused by the read of the emitted Swift types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_sets.rs"]
mod run_swift_sets;

/// A set that repeats an item, refused by the read of the emitted Kotlin types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_sets.rs"]
mod run_kotlin_sets;

/// The models an enum with a variant of several slots is written on, under each tagging.
#[cfg(all(test, feature = "kotlin"))]
#[path = "kotlin_tests/tuple_variants_models.rs"]
mod tuple_variants_models;

/// An enum with a variant of several slots, read and written by the emitted Kotlin types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_tuple_variants.rs"]
mod run_kotlin_tuple_variants;

// The transport's expansion reaches each service through `$crate`, which is this binary's root:
// every trait and its schema module are named here for it to resolve.
#[cfg(all(test, feature = "dart"))]
use run_dart_preprocess::{IntakeClientService, intake_client_service_schema};
#[cfg(all(test, feature = "dart"))]
use run_dart_read::{ParcelClientService, parcel_client_service_schema};
#[cfg(test)]
use tests::{
    ArchiveClientService, ContentClientService, ConversationClientService, EchoClientService,
    GateClientService, LabelClientService, LookupClientService, MediaClientService,
    PulseClientService, SealClientService, SearchClientService, StampClientService,
    ThumbnailClientService, UploadDocumentClientService, VaultClientService,
    archive_client_service_schema, content_client_service_schema,
    conversation_client_service_schema, echo_client_service_schema, gate_client_service_schema,
    label_client_service_schema, lookup_client_service_schema, media_client_service_schema,
    pulse_client_service_schema, seal_client_service_schema, search_client_service_schema,
    stamp_client_service_schema, thumbnail_client_service_schema,
    upload_document_client_service_schema, vault_client_service_schema,
};
#[cfg(all(test, feature = "dart"))]
use tests::{ShelfClientService, shelf_client_service_schema};
