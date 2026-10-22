//! What the Dart emitter reads of a flattened member: the object at its place, without the keys
//! the holder's own members take and the entries each member flattened ahead of it took, as serde
//! hands it. A map is the one member of its object, and is read from every key. `toJson` writes
//! each flattened member at its place. The reads are run
//! under the Dart SDK in `tests/service_schema_emitted_client_tests/run_dart_flattened.rs`.

use super::flattened_models::{
    belaying_dart, clamping_dart, doubling_dart, fastening_dart, hoisting_dart, kedging_dart,
    lacing_dart, loading_dart, mousing_dart, rigging_dart, shoring_dart, spare_dart, trimming_dart,
    twin_dart,
};

/// The read of a map of `u8` values from the object `source` names.
fn counts_from(source: &str) -> String {
    format!(
        "Map<String, int>.fromEntries(({source} as Map<String, dynamic>).entries.map((e) => \
         MapEntry(e.key as String, e.value as int)))"
    )
}

/// The read of a member that names `withItsKeys` from the object `source` names.
fn keyed_from(source: &str) -> String {
    format!("Keying.fromJsonUnjudged(withItsKeys({source}))")
}

/// The object `source` names without its first entry under one of `names`, in the object's order.
fn without_first(source: &str, names: &str) -> String {
    format!(
        "(switch ({{...{source}}}) {{ final rest => \
         rest..remove(rest.keys.where(const [{names}].contains).firstOrNull) }})"
    )
}

/// Asserts that `definition` holds `fragment`.
fn assert_written(definition: &str, fragment: &str) {
    assert!(
        definition.contains(fragment),
        "missing `{fragment}` in: {definition}"
    );
}

/// A map that is the one member of its struct is read from every key, and written entry by entry.
#[test]
fn a_map_alone_is_read_from_every_key() {
    let spare = spare_dart::dart_definition();
    assert_written(&spare, &format!("Spare(rest: {},);", counts_from("json")));
    assert_written(
        &spare,
        "toJson() => { ...(Map<String, dynamic>.fromEntries((rest).entries.map((e) => \
         MapEntry((e.key) as String, e.value))) as Map<String, dynamic>), };",
    );
}

/// A member that names a function flattened after a struct is handed the object without the keys
/// the struct took, and beside a member of its holder's own without that member's key too. The
/// struct is read as it was.
#[test]
fn a_function_after_a_flattened_struct_is_handed_the_object_without_the_keys_the_struct_took() {
    assert_written(
        &clamping_dart::dart_definition(),
        &format!(
            "Clamping(marking: Marking.fromJsonUnjudged(({{...json}}..remove('note'))),note: \
             json['note'] as String,other: {},);",
            keyed_from("({...json}..remove('note')..remove('label'))")
        ),
    );
    assert_written(
        &fastening_dart::dart_definition(),
        &format!(
            "Fastening(note: json['note'] as String,other: {},);",
            keyed_from("({...json}..remove('note'))")
        ),
    );
    assert_written(
        &twin_dart::dart_definition(),
        &format!(
            "Twin(marking: Marking.fromJsonUnjudged(json),other: {},);",
            keyed_from("({...json}..remove('label'))")
        ),
    );
    assert_written(
        &doubling_dart::dart_definition(),
        "Doubling(marking: Marking.fromJsonUnjudged(json),twice: \
         Marking.fromJsonUnjudged(({...json}..remove('label'))),);",
    );
}

/// A member that names a function flattened ahead of a struct is handed every key, and so is one
/// after a struct that names a function on itself, a member that names one, and a struct with a
/// flattened member of its own: none of them takes a key.
#[test]
fn a_function_ahead_of_every_struct_or_after_a_member_that_takes_no_key_is_handed_every_key() {
    assert_written(
        &hoisting_dart::dart_definition(),
        &format!(
            "Hoisting(extra: {},marking: Marking.fromJsonUnjudged(json),);",
            keyed_from("json")
        ),
    );
    assert_written(
        &lacing_dart::dart_definition(),
        &format!(
            "Lacing(issued: Ticketing.fromJsonUnjudged(json),rest: {},);",
            keyed_from("json")
        ),
    );
    assert_written(
        &rigging_dart::dart_definition(),
        &format!(
            "Rigging(marking: Marking.fromJsonUnjudged(asGiven(json)),rest: {},);",
            keyed_from("json")
        ),
    );
    assert_written(
        &belaying_dart::dart_definition(),
        &format!(
            "Belaying(bracing: Bracing.fromJsonUnjudged(json),rest: {},);",
            keyed_from("json")
        ),
    );
}

/// A member that names a function flattened after an externally tagged enum is handed the object
/// without the first entry under the name of a variant serde reads, or an alias of one, each as
/// serde reads it, and a member after both without the keys the second took as well. The enum is
/// read as it was.
#[test]
fn a_function_after_a_flattened_externally_tagged_enum_is_handed_no_entry_of_the_variant() {
    assert_written(
        &loading_dart::dart_definition(),
        &format!(
            "Loading(hatching: Hatching.fromJsonUnjudged(json),rest: {},);",
            keyed_from(&without_first("json", "'Bay', 'Dock', 'Quay'"))
        ),
    );
    assert_written(
        &kedging_dart::dart_definition(),
        &format!(
            "Kedging(kedge: Warping.fromJsonUnjudged(json),rest: {},);",
            keyed_from(&without_first("json", "'pier', 'open_bay'"))
        ),
    );
    let after_the_enum = without_first("json", "'Bay', 'Dock', 'Quay'");
    assert_written(
        &trimming_dart::dart_definition(),
        &format!(
            "Trimming(hatching: Hatching.fromJsonUnjudged(json),marking: \
             Marking.fromJsonUnjudged({after_the_enum}),rest: {},);",
            keyed_from(&format!("({{...{after_the_enum}}}..remove('label'))"))
        ),
    );
}

/// A member that names a function flattened after an enum under a tag key alone, or after an
/// untagged one, is handed every key.
#[test]
fn a_function_after_an_enum_read_as_a_map_is_handed_every_key() {
    assert_written(
        &mousing_dart::dart_definition(),
        &format!(
            "Mousing(frapping: Frapping.fromJsonUnjudged(json),rest: {},);",
            keyed_from("json")
        ),
    );
    assert_written(
        &shoring_dart::dart_definition(),
        &format!(
            "Shoring(chocking: Chocking.fromJsonUnjudged(json),rest: {},);",
            keyed_from("json")
        ),
    );
}
