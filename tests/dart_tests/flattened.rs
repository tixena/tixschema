//! What the Dart emitter reads of a flattened member: the object at its place, without the keys
//! the holder's own members take and the entries each member flattened ahead of it took, as serde
//! hands it. `toJson` writes each flattened member at its place. The reads are run under the Dart
//! SDK in `tests/service_schema_emitted_client_tests/run_dart_flattened.rs`.

use super::flattened_models::{
    belaying_dart, clamping_dart, coupling_dart, doubling_dart, fastening_dart, hoisting_dart,
    kedging_dart, lacing_dart, loading_dart, loosening_dart, mousing_dart, pairing_dart,
    rigging_dart, shoring_dart, stowing_dart, tethering_dart, trimming_dart, twin_dart,
};

/// The read of a map of `u8` values from the object `source` names.
fn counts_from(source: &str) -> String {
    format!(
        "Map<String, int>.fromEntries(({source} as Map<String, dynamic>).entries.map((e) => \
         MapEntry(e.key as String, e.value as int)))"
    )
}

/// The read of a map of JSON values from the object `source` names.
fn values_from(source: &str) -> String {
    format!(
        "Map<String, dynamic>.fromEntries(({source} as Map<String, dynamic>).entries.map((e) => \
         MapEntry(e.key as String, e.value)))"
    )
}

/// The object `source` names without its first entry under one of `names`, in the object's order.
fn without_first(source: &str, names: &str) -> String {
    format!(
        "(switch ({{...{source}}}) {{ final rest => \
         rest..remove(rest.keys.where(const [{names}].contains).firstOrNull) }})"
    )
}

/// The `toJson` entry of the flattened map `member`.
fn map_written(member: &str) -> String {
    format!(
        "...(Map<String, dynamic>.fromEntries(({member}).entries.map((e) => MapEntry((e.key) as \
         String, e.value))) as Map<String, dynamic>),"
    )
}

/// Asserts that `definition` holds `fragment`.
fn assert_written(definition: &str, fragment: &str) {
    assert!(
        definition.contains(fragment),
        "missing `{fragment}` in: {definition}"
    );
}

/// A map flattened after a struct is read without the keys the struct took, and beside a member
/// of its holder's own without that member's key too. The struct is read as it was.
#[test]
fn a_map_after_a_flattened_struct_is_read_without_the_keys_the_struct_took() {
    assert_written(
        &coupling_dart::dart_definition(),
        &format!(
            "Coupling(marking: Marking.fromJsonUnjudged(json),rest: {},);",
            counts_from("({...json}..remove('label'))")
        ),
    );
    assert_written(
        &clamping_dart::dart_definition(),
        &format!(
            "Clamping(marking: Marking.fromJsonUnjudged(({{...json}}..remove('note'))),note: \
             json['note'] as String,rest: {},);",
            counts_from("({...json}..remove('note')..remove('label'))")
        ),
    );
}

/// In a variant under a tag key, a map flattened after a struct is read without the tag and the
/// keys the struct took.
#[test]
fn a_map_after_a_flattened_struct_in_a_variant_is_read_without_the_tag_and_the_structs_keys() {
    assert_written(
        &tethering_dart::dart_definition(),
        &format!(
            "TetheringHeld(marking: Marking.fromJsonUnjudged(({{...json}}..remove('kind'))),rest: \
             {},);",
            counts_from("({...json}..remove('kind')..remove('label'))")
        ),
    );
}

/// A map that is the one flattened member is read as it was: without its holder's own keys.
#[test]
fn a_map_that_is_the_one_flattened_member_is_read_without_its_holders_keys() {
    assert_written(
        &fastening_dart::dart_definition(),
        &format!(
            "Fastening(note: json['note'] as String,rest: {},);",
            counts_from("({...json}..remove('note'))")
        ),
    );
}

/// A map flattened ahead of a struct is handed every key, and so is each of two maps. Each is
/// written at its place.
#[test]
fn a_map_ahead_of_every_flattened_struct_is_read_from_every_key() {
    let hoisting = hoisting_dart::dart_definition();
    assert_written(
        &hoisting,
        &format!(
            "Hoisting(extra: {},marking: Marking.fromJsonUnjudged(json),);",
            values_from("json")
        ),
    );
    assert_written(
        &hoisting,
        &format!(
            "toJson() => {{ {}...((marking).toJson() as Map<String, dynamic>), }};",
            map_written("extra")
        ),
    );
    let stowing = stowing_dart::dart_definition();
    assert_written(
        &stowing,
        &format!(
            "Stowing(first: {},second: {},);",
            counts_from("json"),
            counts_from("json")
        ),
    );
    assert_written(
        &stowing,
        &format!(
            "toJson() => {{ {}{} }};",
            map_written("first"),
            map_written("second")
        ),
    );
}

/// A struct that names a function on itself, a member that names one, and a struct with a
/// flattened member of its own take no key, and a map flattened after them is handed every key.
#[test]
fn a_map_after_a_member_that_takes_no_key_is_read_from_every_key() {
    assert_written(
        &lacing_dart::dart_definition(),
        &format!(
            "Lacing(issued: Ticketing.fromJsonUnjudged(json),rest: {},);",
            values_from("json")
        ),
    );
    assert_written(
        &rigging_dart::dart_definition(),
        &format!(
            "Rigging(marking: Marking.fromJsonUnjudged(asGiven(json)),rest: {},);",
            values_from("json")
        ),
    );
    assert_written(
        &belaying_dart::dart_definition(),
        &format!(
            "Belaying(bracing: Bracing.fromJsonUnjudged(json),rest: {},);",
            values_from("json")
        ),
    );
}

/// A struct with a flattened map, a member that names a function, a JSON value and a struct that
/// reads the same key, each flattened after a struct, are read without the keys the struct took.
/// The struct is read as it was.
#[test]
fn a_member_after_a_flattened_struct_is_read_without_the_keys_the_struct_took() {
    assert_written(
        &pairing_dart::dart_definition(),
        "Pairing(marking: Marking.fromJsonUnjudged(json),spare: \
         Spare.fromJsonUnjudged(({...json}..remove('label'))),);",
    );
    assert_written(
        &twin_dart::dart_definition(),
        "Twin(marking: Marking.fromJsonUnjudged(json),other: \
         Keying.fromJsonUnjudged(withItsKeys(({...json}..remove('label')))),);",
    );
    assert_written(
        &loosening_dart::dart_definition(),
        "Loosening(marking: Marking.fromJsonUnjudged(json),rest: ({...json}..remove('label')),);",
    );
    assert_written(
        &doubling_dart::dart_definition(),
        "Doubling(marking: Marking.fromJsonUnjudged(json),twice: \
         Marking.fromJsonUnjudged(({...json}..remove('label'))),);",
    );
}

/// A map flattened after an externally tagged enum is read without the first entry under the name
/// of a variant serde reads, or an alias of one, each as serde reads it. The enum is read as it
/// was.
#[test]
fn a_map_after_a_flattened_externally_tagged_enum_is_read_without_the_variants_entry() {
    assert_written(
        &loading_dart::dart_definition(),
        &format!(
            "Loading(hatching: Hatching.fromJsonUnjudged(json),rest: {},);",
            values_from(&without_first("json", "'Bay', 'Dock', 'Quay'"))
        ),
    );
    assert_written(
        &kedging_dart::dart_definition(),
        &format!(
            "Kedging(kedge: Warping.fromJsonUnjudged(json),rest: {},);",
            values_from(&without_first("json", "'pier', 'open_bay'"))
        ),
    );
}

/// A member flattened after an externally tagged enum is handed the object without the enum's
/// entry, and a member after both is handed it without the keys the second took as well.
#[test]
fn a_member_after_a_flattened_externally_tagged_enum_is_read_without_the_variants_entry() {
    let after_the_enum = without_first("json", "'Bay', 'Dock', 'Quay'");
    assert_written(
        &trimming_dart::dart_definition(),
        &format!(
            "Trimming(hatching: Hatching.fromJsonUnjudged(json),marking: \
             Marking.fromJsonUnjudged({after_the_enum}),rest: {},);",
            values_from(&format!("({{...{after_the_enum}}}..remove('label'))"))
        ),
    );
}

/// A map flattened after an enum under a tag key alone, or after an untagged one, is read as it
/// was: from every key.
#[test]
fn a_map_after_an_enum_read_as_a_map_is_read_from_every_key() {
    assert_written(
        &mousing_dart::dart_definition(),
        &format!(
            "Mousing(frapping: Frapping.fromJsonUnjudged(json),rest: {},);",
            values_from("json")
        ),
    );
    assert_written(
        &shoring_dart::dart_definition(),
        &format!(
            "Shoring(chocking: Chocking.fromJsonUnjudged(json),rest: {},);",
            values_from("json")
        ),
    );
}
