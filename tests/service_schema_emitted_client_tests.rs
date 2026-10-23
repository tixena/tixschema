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

/// Each TypeScript client handed a message in a wider form than serde reads, over each transport.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_node_wire_forms.rs"]
mod run_node_wire_forms;

/// An `i64` through each TypeScript client and dispatcher, as the string of its digits.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_node_digits.rs"]
mod run_node_digits;

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

/// A service whose header names, part names, message names and paths hold a character a mobile
/// client's string literal reads apart from text, or a line break, called through each mobile
/// client.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_quoted_service.rs"]
mod run_quoted_service;

/// A service with operations only `http_rest` carries: no `ws_rpc` client or dispatcher holds them.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_http_alone.rs"]
mod run_http_alone;

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

/// The models a `literal` is written on, and what each payload of their table reads as.
#[cfg(all(test, any(feature = "dart", feature = "swift", feature = "kotlin")))]
#[path = "model_schema_prop_tests/literal_models.rs"]
mod literal_models;

/// A `literal` read by the emitted Dart types, against serde.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_literals.rs"]
mod run_dart_literals;

/// A `literal` read by the emitted Swift types, against serde.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_literals.rs"]
mod run_swift_literals;

/// A `literal` read by the emitted Kotlin types, against serde.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_literals.rs"]
mod run_kotlin_literals;

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

/// The models an enum with a variant of several slots is written on, under each tagging, and a
/// tuple in each place a writer names its wrapper more than once.
#[cfg(all(test, feature = "kotlin"))]
#[path = "kotlin_tests/tuple_variants_models.rs"]
mod tuple_variants_models;

/// An enum with a variant of several slots, and a tuple in each place a writer names its wrapper
/// more than once, read and written by the emitted Kotlin types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_tuple_variants.rs"]
mod run_kotlin_tuple_variants;

/// A map flattened as the one member of its struct, optional or not and keyed by a plain enum, a
/// flattened struct beside a member of its own, an optional member beside a flattened one with its
/// twin under `nullable`, and a member flattened after a struct or beside a member of its holder's
/// own. Declared after the module that declares the member they flatten. The Swift run reads its
/// externally tagged enums flattened where both clients are built.
#[cfg(all(test, feature = "kotlin"))]
#[path = "kotlin_tests/flattened_models.rs"]
mod flattened_models;

/// An optional flattened member whose type the Kotlin emitter recorded no keys for: a struct
/// declared below its holder with a flattened member of its own, and an enum under each tagging
/// but serde's own.
#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "kotlin_tests/unrecorded_options_models.rs"]
mod unrecorded_options_models;

/// An optional flattened member that names a function: a struct the Kotlin emitter recorded keys
/// for, under a function that answers the payload as it was handed and under one that answers
/// `null`, and an enum under a tag alone, read by the Kotlin, the Swift and the Dart run.
#[cfg(test)]
#[cfg(any(feature = "dart", feature = "kotlin", feature = "swift"))]
#[path = "kotlin_tests/corrected_options_models.rs"]
mod corrected_options_models;

/// A map flattened as the one member of its struct, optional or not and keyed by a plain enum, a
/// flattened struct beside a member of its own, an optional member beside a flattened one, and a
/// member flattened after a struct or beside a member of its holder's own, read and written by
/// the emitted Kotlin types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_flattened.rs"]
mod run_kotlin_flattened;

/// A fixed array held in each place a read reads it, and what serde reads of a payload of each.
#[cfg(test)]
#[cfg(feature = "kotlin")]
#[path = "kotlin_tests/fixed_arrays_models.rs"]
mod fixed_arrays_models;

/// A payload serde or Zod refuses, refused by a serializer the Kotlin emitter writes with
/// `SerializationException`.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_refusals.rs"]
mod run_kotlin_refusals;

/// A `char` and a chrono date, time and date and time held in each place, and what serde, Zod and
/// each client's read answer of each payload.
#[cfg(test)]
#[cfg(all(
    feature = "chrono",
    any(feature = "dart", feature = "kotlin", feature = "swift")
))]
#[path = "kotlin_tests/text_forms_models.rs"]
mod text_forms_models;

/// Those payloads, read by the emitted Kotlin types where the table states.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_text_forms.rs"]
mod run_kotlin_text_forms;

/// Those payloads, read by the emitted Dart types where the table states.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_text_forms.rs"]
mod run_dart_text_forms;

/// Those payloads, read by the emitted Swift types where the table states.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_text_forms.rs"]
mod run_swift_text_forms;

/// A model per type the wire holds as a text, a number or a `bool`, and what each client sends of
/// each value a caller hands it.
#[cfg(test)]
#[cfg(all(
    feature = "chrono",
    any(feature = "dart", feature = "kotlin", feature = "swift")
))]
#[path = "wire_forms_tests/models.rs"]
mod wire_forms_models;

/// What each client is handed of those values, and what it prints of them.
#[cfg(test)]
#[cfg(all(
    feature = "chrono",
    any(feature = "dart", feature = "kotlin", feature = "swift")
))]
#[path = "service_schema_emitted_client_tests/wire_form_rows.rs"]
mod wire_form_rows;

/// Those values, sent by the emitted Kotlin types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_wire_forms.rs"]
mod run_kotlin_wire_forms;

/// Those values, sent by the emitted Dart types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_wire_forms.rs"]
mod run_dart_wire_forms;

/// Those values, sent by the emitted Swift types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_wire_forms.rs"]
mod run_swift_wire_forms;

/// A service that takes each type of the table in a path, a query, a header and a part, and what
/// each client is held to of each row handed there.
#[cfg(test)]
#[cfg(all(
    feature = "chrono",
    any(feature = "dart", feature = "kotlin", feature = "swift")
))]
#[macro_use]
#[path = "service_schema_emitted_client_tests/text_position_rows.rs"]
mod text_position_rows;

/// The `http_rest` dispatcher for `TextPlacingClientService`, which reads each request the runs
/// below record.
#[cfg(test)]
#[cfg(all(
    feature = "chrono",
    any(feature = "dart", feature = "kotlin", feature = "swift")
))]
#[path = "service_schema_emitted_client_tests/text_placing_http_rest_transport.rs"]
mod text_placing_http_rest_transport;

/// A service of a date message, a `char` and a date header in, and a date reply and header out.
#[cfg(test)]
#[cfg(all(
    feature = "chrono",
    any(
        all(feature = "typescript", feature = "zod"),
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
#[macro_use]
#[path = "service_schema_emitted_client_tests/sundial_service.rs"]
mod sundial_service;

/// The `http_rest` dispatcher for `SundialClientService`.
#[cfg(test)]
#[cfg(all(
    feature = "chrono",
    any(
        all(feature = "typescript", feature = "zod"),
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
#[path = "service_schema_emitted_client_tests/sundial_http_rest_transport.rs"]
mod sundial_http_rest_transport;

/// The `ws_rpc` dispatcher for `SundialClientService`.
#[cfg(test)]
#[cfg(all(
    feature = "chrono",
    any(
        all(feature = "typescript", feature = "zod"),
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
#[path = "service_schema_emitted_client_tests/sundial_ws_rpc_transport.rs"]
mod sundial_ws_rpc_transport;

/// `SundialClientService` called through the clients of each language, against those dispatchers.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_sundial.rs"]
mod run_sundial;

/// Each value of the table, written in a path, a query, a header and a part by the emitted
/// TypeScript client, beside the real `zod`.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_node_text_positions.rs"]
mod run_node_text_positions;

/// Those values, written by the emitted Dart client.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_text_positions.rs"]
mod run_dart_text_positions;

/// Those values, written by the emitted Swift client.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_text_positions.rs"]
mod run_swift_text_positions;

/// Those values, written by the emitted Kotlin client.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_text_positions.rs"]
mod run_kotlin_text_positions;

/// A bare date reply and a `char` header, read in their one form by the emitted Kotlin clients and
/// dispatcher.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_service_forms.rs"]
mod run_kotlin_service_forms;

/// The models a name serde reads beside the one it writes is read under.
#[cfg(test)]
#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "kotlin", feature = "swift")
))]
#[path = "kotlin_tests/aliased_models.rs"]
mod aliased_models;

/// A variant and a member under the `alias` serde reads them under, read by the emitted Kotlin
/// types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_aliased.rs"]
mod run_kotlin_aliased;

/// A map that is the one member of its object, optional too, and a member that names a function
/// flattened at each place a member can be handed fewer keys than its object holds.
#[cfg(test)]
#[cfg(feature = "dart")]
#[path = "dart_tests/flattened_models.rs"]
mod dart_flattened_models;

/// Those members, read and written back by the emitted Dart types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_flattened.rs"]
mod run_dart_flattened;

/// The enums of the models a name serde reads beside the one it writes is read under, flattened,
/// each beside a member of its holder's own. Declared after the module that declares the enums.
#[cfg(test)]
#[cfg(all(feature = "serde", any(feature = "dart", feature = "swift")))]
#[path = "dart_tests/flattened_aliased_models.rs"]
mod dart_flattened_aliased_models;

/// A variant and a member under the `alias` serde reads them under, read by the emitted Dart
/// types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_aliased.rs"]
mod run_dart_aliased;

/// A member under an alias in each way the Swift read reads a member under its key.
#[cfg(test)]
#[cfg(all(feature = "swift", feature = "serde"))]
#[path = "swift_tests/aliased_members_models.rs"]
mod aliased_members_models;

/// A variant and a member under the `alias` serde reads them under, read by the emitted Swift
/// types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_aliased.rs"]
mod run_swift_aliased;

/// An externally tagged enum flattened beside a member, each variant read and written back by the
/// emitted Dart types, a unit variant as an entry of its name over `null`, and a unit variant
/// under a content key read from an object with no content key.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_unit_entries.rs"]
mod run_dart_unit_entries;

/// A struct under its own tag, alone, with a parameter, with a member under its alias, with a
/// flattened member and flattened into a holder, read by the emitted Dart types under its name
/// alone, as TypeScript and Zod read it.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_own_tags.rs"]
mod run_dart_own_tags;

/// A payload serde refuses, refused by the emitted Dart types with a `FormatException`: a tag no
/// variant has under every tagging, a unit variant's value, an externally tagged object of no key
/// or of several, a payload no variant of an untagged enum reads, a range of a key it does not
/// write, a `Result` of both keys, and a tuple and a fixed array of another length.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_refusals.rs"]
mod run_dart_refusals;

/// Models whose names on the wire hold a `$`, a quote, a double quote and a backslash.
#[cfg(all(test, feature = "serde", feature = "dart"))]
#[path = "dart_tests/quoted_models.rs"]
mod quoted_models;

/// Those models, read and written back by the emitted Dart types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_quoted.rs"]
mod run_dart_quoted;

/// Those models, read and written back by the emitted Swift types. The models are declared where
/// the Dart client is built.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_quoted.rs"]
mod run_swift_quoted;

/// Those models, read and written back by the emitted Kotlin types. The models are declared where
/// the Dart client is built.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_quoted.rs"]
mod run_kotlin_quoted;

/// `()` held in each place a value is, and a tuple struct of no slot beside it.
#[cfg(test)]
#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
#[path = "service_schema_emitted_client_tests/unit_models.rs"]
mod unit_models;

/// `()`, read and written back by the emitted Dart types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_units.rs"]
mod run_dart_units;

/// `()`, read and written back by the emitted Swift types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_units.rs"]
mod run_swift_units;

/// `()`, read and written back by the emitted Kotlin types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_units.rs"]
mod run_kotlin_units;

/// An `i64` held in each place a member holds one, written as the string of its digits.
#[cfg(test)]
#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
#[path = "service_schema_emitted_client_tests/digit_models.rs"]
mod digit_models;

/// An `i64`, read from its digits and written as them by the emitted Dart types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_digits.rs"]
mod run_dart_digits;

/// An `i64`, read from its digits and written as them by the emitted Swift types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_digits.rs"]
mod run_swift_digits;

/// An `i64`, read from its digits and written as them by the emitted Kotlin types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_digits.rs"]
mod run_kotlin_digits;

/// A fixed array and a tuple of another length, refused by the emitted Swift types in serde's
/// words, in each place a value is held, and read from an array of their length as serde reads
/// them. The fixed arrays and the tuples of `run_dart_refusals.rs` are declared where the Dart
/// client is built.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_refusals.rs"]
mod run_swift_refusals;

/// A struct under its own tag, alone, with a parameter, with a member under its alias, naming a
/// function on itself, with a flattened member and flattened into a holder, read by the emitted
/// Swift types under its name alone, as TypeScript and Zod read it.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_own_tags.rs"]
mod run_swift_own_tags;

/// A map that is the one member of its object, under a key Swift wraps, of models, of dates and
/// naming a function, and a member that names a function at each place a member can be handed
/// fewer keys than its object holds.
#[cfg(test)]
#[cfg(feature = "swift")]
#[path = "swift_tests/flattened_models.rs"]
mod swift_flattened_models;

/// A flattened struct that reads a key a member of its holder's own, a struct flattened ahead of
/// it or the tag of the variant that flattens it took, read by the Dart, Kotlin and Swift runs.
#[cfg(test)]
#[cfg(any(feature = "dart", feature = "kotlin", feature = "swift"))]
#[path = "swift_tests/shared_key_models.rs"]
mod shared_key_models;

/// Those members, and the structs with a flattened member of their own that read a key taken
/// around them, read and written back by the emitted Swift types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_flattened.rs"]
mod run_swift_flattened;

/// An externally tagged enum flattened beside a member of its holder's own, ahead of a function
/// and of a struct, optional, and with a model its holder judges, read by the emitted Swift types
/// from the one entry under a variant's name and written as serde writes it, a unit variant as an
/// entry of its name over `null`, and read alone from an object of one key; and a unit variant
/// under a content key read from an object with no content key.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_flattened_enums.rs"]
mod run_swift_flattened_enums;

/// A flattened `Option` beside a member of its holder's own, alone in its holder and in a variant
/// under a tag key, of a struct of two keys, of a struct declared below its holder, of a model
/// `validate()` walks, of a struct that reads the keys no member around it took, read from what a
/// function answers, and of an enum of each tagging and one declared below its holder, and a
/// struct flattened bare beside a member of its holder's own. The Kotlin run reads the struct
/// flattened bare, the one that reads the keys no member around it took and the enums but the
/// externally tagged one declared above its holder as well, and the Dart run all but the map.
#[cfg(test)]
#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
#[path = "swift_tests/flattened_options_models.rs"]
mod swift_flattened_options_models;

/// Those members, held, absent and held with a key their type does not read, read and written
/// back by the emitted Swift types, and the optional members that name a function, read as Zod
/// reads them.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_flattened_options.rs"]
mod run_swift_flattened_options;

/// Those members, and an enum under a tag key that names a function on itself, read and written
/// back by the emitted Dart types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_flattened_options.rs"]
mod run_dart_flattened_options;

/// A member serde reads and never writes, in a struct of its own, beside a flattened member and in
/// an enum's variant under each tagging that writes a variant's named members.
#[cfg(test)]
#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
#[path = "service_schema_emitted_client_tests/unwritten_models.rs"]
mod unwritten_models;

/// A member serde never writes, read and written back by the emitted Dart types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_unwritten.rs"]
mod run_dart_unwritten;

/// A member serde never writes, read and written back by the emitted Swift types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_unwritten.rs"]
mod run_swift_unwritten;

/// A member serde never writes, read and written back by the emitted Kotlin types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_unwritten.rs"]
mod run_kotlin_unwritten;

/// Enums with a variant serde neither writes nor reads, under every tagging, and with a variant
/// serde drops in one direction alone.
#[cfg(test)]
#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
#[path = "skipped_variant_tests/models.rs"]
mod skipped_variant_models;

/// Those enums read and written back by the emitted Dart types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_skipped_variants.rs"]
mod run_dart_skipped_variants;

/// Those enums read and written back by the emitted Swift types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_skipped_variants.rs"]
mod run_swift_skipped_variants;

/// Those enums read and written back by the emitted Kotlin types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_skipped_variants.rs"]
mod run_kotlin_skipped_variants;

/// A key no member reads, refused by the emitted Kotlin types under the default `Json` as Zod
/// refuses it. Declared after the modules that declare the members its models flatten.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_unknown_keys.rs"]
mod run_kotlin_unknown_keys;

/// Services whose message names, paths, part name and header names hold a quote, called through
/// the emitted TypeScript clients and dispatchers under node, beside the Rust dispatchers.
#[cfg(test)]
#[macro_use]
#[path = "service_schema_emitted_client_tests/run_node_quoted.rs"]
mod run_node_quoted;

/// The `http_rest` dispatcher for `ImpostClientService`, the Rust twin each request the node run
/// writes is read by.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/impost_http_rest_transport.rs"]
mod impost_http_rest_transport;

/// The `http_rest` dispatcher for `ImpostFilingClientService`.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/impost_filing_http_rest_transport.rs"]
mod impost_filing_http_rest_transport;

/// An `ObjectId` in each position serde writes one, and whether each side takes each payload.
#[cfg(test)]
#[cfg(all(
    feature = "mongodb",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
#[path = "service_schema_emitted_client_tests/object_id_models.rs"]
mod object_id_models;

/// The `ObjectId` the README has the consumer declare in each client's language, and what a run
/// prints of each payload.
#[cfg(test)]
#[cfg(all(
    feature = "mongodb",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
#[path = "service_schema_emitted_client_tests/object_id_runs.rs"]
mod object_id_runs;

/// Those payloads, read and written back by the emitted Dart types beside the README's `ObjectId`.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_object_ids.rs"]
mod run_dart_object_ids;

/// Those payloads, read and written back by the emitted Swift types beside the README's
/// `ObjectId`.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_object_ids.rs"]
mod run_swift_object_ids;

/// Those payloads, read and written back by the emitted Kotlin types beside the README's
/// `ObjectId`.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_object_ids.rs"]
mod run_kotlin_object_ids;

/// The lone slot of a newtype variant under a tag alone, in each shape serde writes beside the
/// tag.
#[cfg(test)]
#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
#[path = "service_schema_emitted_client_tests/tagged_slot_models.rs"]
mod tagged_slot_models;

/// Those slots, read and written back by the emitted Kotlin types beside serde and Zod.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_kotlin_tagged_slots.rs"]
mod run_kotlin_tagged_slots;

/// Those slots, read and written back by the emitted Dart types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_dart_tagged_slots.rs"]
mod run_dart_tagged_slots;

/// Those slots, read and written back by the emitted Swift types.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_swift_tagged_slots.rs"]
mod run_swift_tagged_slots;

/// Tuple variants with a slot serde neither writes nor reads.
#[cfg(test)]
#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
#[path = "service_schema_emitted_client_tests/skipped_slot_models.rs"]
mod skipped_slot_models;

/// Those variants, read and written back by each client beside serde and Zod.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_skipped_slots.rs"]
mod run_skipped_slots;

/// A date under `as_number` in a member of an untagged enum.
#[cfg(test)]
#[cfg(all(
    feature = "chrono",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
#[path = "chrono_tests/untagged_millis_models.rs"]
mod untagged_millis_models;

/// That date, read and written back by each client beside serde.
#[cfg(test)]
#[cfg(feature = "chrono")]
#[path = "service_schema_emitted_client_tests/run_untagged_millis.rs"]
mod run_untagged_millis;

/// Untagged enums with a newtype variant over an `Option`.
#[cfg(test)]
#[cfg(all(
    feature = "serde",
    any(
        all(feature = "zod", feature = "typescript"),
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
#[path = "untagged_tests/null_models.rs"]
mod null_models;

/// Those enums, read and written back by Zod and each client beside serde.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_untagged_nulls.rs"]
mod run_untagged_nulls;

/// An alias of an `Option` held outside a field.
#[cfg(test)]
#[cfg(all(
    feature = "serde",
    any(feature = "dart", feature = "swift", feature = "kotlin")
))]
#[path = "semantic_types_tests/option_alias_models.rs"]
mod option_alias_models;

/// Those aliases, read and written back by each client beside serde.
#[cfg(test)]
#[path = "service_schema_emitted_client_tests/run_option_aliases.rs"]
mod run_option_aliases;

// The transport's expansion reaches each service through `$crate`, which is this binary's root:
// every trait and its schema module are named here for it to resolve.
#[cfg(all(test, feature = "dart"))]
use run_dart_preprocess::{IntakeClientService, intake_client_service_schema};
#[cfg(all(test, feature = "dart"))]
use run_dart_read::{ParcelClientService, parcel_client_service_schema};
#[cfg(test)]
use run_node_quoted::{
    ImpostClientService, ImpostFilingClientService, impost_client_service_schema,
    impost_filing_client_service_schema,
};
#[cfg(all(test, any(feature = "dart", feature = "swift", feature = "kotlin")))]
use run_quoted_service::{
    TollingClientService, TollingUploadClientService, tolling_client_service_schema,
    tolling_upload_client_service_schema,
};
#[cfg(all(
    test,
    feature = "chrono",
    any(
        all(feature = "typescript", feature = "zod"),
        feature = "dart",
        feature = "swift",
        feature = "kotlin"
    )
))]
use sundial_service::{SundialClientService, sundial_client_service_schema};
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
#[cfg(all(
    test,
    feature = "chrono",
    any(feature = "dart", feature = "kotlin", feature = "swift")
))]
use text_position_rows::{TextPlacingClientService, text_placing_client_service_schema};
