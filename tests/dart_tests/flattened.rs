//! What the Dart emitter reads of a flattened member: the object at its place, without the keys
//! the holder's own members take and the entries each member flattened ahead of it took, as serde
//! hands it, and an externally tagged enum the one entry of that object it reads, an optional one
//! no value where that object holds no such entry. A map is the one member of its object, and is
//! read from every key, an optional one as no value where an entry does not read. `toJson` writes
//! each flattened member at its place. The reads are run under the Dart SDK in
//! `tests/service_schema_emitted_client_tests/run_dart_flattened.rs`.

#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use super::flattened_models::limber_dart;
use super::flattened_models::{
    battening_dart, belaying_dart, bilging_dart, caulking_dart, clamping_dart, doubling_dart,
    dunnage_dart, fastening_dart, freeboard_dart, hawsehole_dart, hoisting_dart, kedging_dart,
    lacing_dart, loading_dart, mousing_dart, rigging_dart, scuppering_dart, shoring_dart,
    spare_dart, steeving_dart, trimming_dart, twin_dart,
};
use super::held_numbers::u8_held;

/// The names of each variant of `Hatching`, as the read of a flattened one lists them.
const HATCHING: &str = "['Bay'], ['Dock', 'Quay']";

/// The read of a map of `u8` values from the entries of `object`, which Dart types
/// `Map<String, dynamic>`.
fn counts_from(object: &str) -> String {
    format!(
        "Map<String, int>.fromEntries({object}.entries.map((e) => MapEntry(e.key, {})))",
        u8_held("e.value as int")
    )
}

/// The read of a member that names `withItsKeys` from the object `source` names.
fn keyed_from(source: &str) -> String {
    format!("Keying.fromJsonUnjudged(withItsKeys({source}))")
}

/// An object of the one entry of the object `source` names under one of `names`, refused where it
/// holds entries under two of them, naming once each variant of the enum `enum_name` they belong
/// to, each a list of `variants`.
fn first_of(source: &str, enum_name: &str, names: &str, variants: &str) -> String {
    format!(
        "(switch ({source}) {{ final whole => whole.keys.where(const [{names}].contains).length > \
         1 ? throw FormatException('more than one variant of enum {enum_name} found in flattened \
         data: ' + [for (final variant in const [{variants}]) if \
         (variant.any(whole.containsKey)) variant.first].join(', ')) : {{ for (final key in \
         whole.keys.where(const [{names}].contains).take(1)) key: whole[key] }} }})"
    )
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
        &format!(
            "toJson() => {{ ...Map<String, dynamic>.fromEntries((rest).entries.map((e) => \
             MapEntry(e.key, {}))), }};",
            u8_held("e.value")
        ),
    );
}

/// A member that names a function flattened after a struct is handed the object without the keys
/// the struct took, and beside a member of its holder's own without that member's key too. The
/// struct is read from the whole object, as TypeScript and Zod read it, and so is a struct
/// flattened after it.
#[test]
fn a_function_after_a_flattened_struct_is_handed_the_object_without_the_keys_the_struct_took() {
    assert_written(
        &clamping_dart::dart_definition(),
        &format!(
            "Clamping(marking: Marking.fromJsonUnjudged(json),note: json['note'] as String,other: \
             {},);",
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
         Marking.fromJsonUnjudged(json),);",
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

/// A flattened externally tagged enum is read from the one entry under the name of a variant serde
/// reads, or an alias of one, each as serde reads it, and a member that names a function flattened
/// after it is handed the object without that entry, and a member after both without the keys the
/// second took as well. A struct flattened after it is read from the whole object.
#[test]
fn a_function_after_a_flattened_externally_tagged_enum_is_handed_no_entry_of_the_variant() {
    let hatching = first_of("json", "Hatching", "'Bay', 'Dock', 'Quay'", HATCHING);
    assert_written(
        &loading_dart::dart_definition(),
        &format!(
            "Loading(hatching: Hatching.fromJsonUnjudged({hatching}),rest: {},);",
            keyed_from(&without_first("json", "'Bay', 'Dock', 'Quay'"))
        ),
    );
    assert_written(
        &kedging_dart::dart_definition(),
        &format!(
            "Kedging(kedge: Warping.fromJsonUnjudged({}),rest: {},);",
            first_of(
                "json",
                "Warping",
                "'pier', 'open_bay'",
                "['pier'], ['open_bay']"
            ),
            keyed_from(&without_first("json", "'pier', 'open_bay'"))
        ),
    );
    let after_the_enum = without_first("json", "'Bay', 'Dock', 'Quay'");
    assert_written(
        &trimming_dart::dart_definition(),
        &format!(
            "Trimming(hatching: Hatching.fromJsonUnjudged({hatching}),marking: \
             Marking.fromJsonUnjudged(json),rest: {},);",
            keyed_from(&format!("({{...{after_the_enum}}}..remove('label'))"))
        ),
    );
}

/// A flattened externally tagged enum beside a member of its holder's own is read from the first
/// entry under the name of a variant in the object without the member's key, wherever it is. An
/// enum with no unit variant writes an object, which the holder spreads into its own.
#[test]
fn a_flattened_externally_tagged_enum_is_read_from_its_variants_entry() {
    assert_written(
        &battening_dart::dart_definition(),
        &format!(
            "Battening(hatching: Hatching.fromJsonUnjudged({}),note: json['note'] as String,);",
            first_of(
                "({...json}..remove('note'))",
                "Hatching",
                "'Bay', 'Dock', 'Quay'",
                HATCHING
            )
        ),
    );
    assert_written(
        &battening_dart::dart_definition(),
        "Map<String, dynamic> toJson() => { ...((hatching).toJson() as Map<String, dynamic>),'note': \
         note, };",
    );
}

/// An optional flattened externally tagged enum is read as no value where the object it is handed
/// holds no entry under a variant's name, and from that entry where it holds one.
#[test]
fn an_optional_flattened_externally_tagged_enum_is_null_where_no_entry_is() {
    assert_written(
        &caulking_dart::dart_definition(),
        &format!(
            "Caulking(hatching: (switch ({}) {{ final entry => entry.isEmpty ? null : \
             Hatching.fromJsonUnjudged(entry) }}),note: json['note'] as String,);",
            first_of(
                "({...json}..remove('note'))",
                "Hatching",
                "'Bay', 'Dock', 'Quay'",
                HATCHING
            )
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

/// A read that answers no value wherever `read` throws.
fn or_null(read: &str) -> String {
    format!("(() {{ try {{ return {read}; }} catch (_) {{ return null; }} }})()")
}

/// An optional map is read from every key of its object in a closure that answers `null` where an
/// entry does not read, alone in its struct and as the one member of a variant under its name,
/// under a content key and untagged, and keyed by a plain enum. It is written entry by entry where
/// it holds a value, and as no entry where it holds none.
#[test]
fn an_optional_map_is_null_where_an_entry_does_not_read() {
    let dunnage = dunnage_dart::dart_definition();
    assert_written(
        &dunnage,
        &format!("Dunnage(counts: {},);", or_null(&counts_from("json"))),
    );
    assert_written(
        &dunnage,
        &format!(
            "toJson() => {{ if (counts != null) ...((counts == null ? null : \
             Map<String, dynamic>.fromEntries((counts!).entries.map((e) => MapEntry(e.key, \
             {})))) as Map<String, dynamic>), }};",
            u8_held("e.value")
        ),
    );
    for (definition, class) in [
        (steeving_dart::dart_definition(), "SteevingOpen"),
        (freeboard_dart::dart_definition(), "FreeboardOpen"),
        (scuppering_dart::dart_definition(), "ScupperingOpen"),
    ] {
        assert_written(
            &definition,
            &format!("{class}(counts: {},);", or_null(&counts_from("json"))),
        );
    }
    assert_written(
        &bilging_dart::dart_definition(),
        &format!(
            "Bilging(counts: {},);",
            or_null(&format!(
                "Map<Garboard, int>.fromEntries(json.entries.map((e) => \
                 MapEntry(Garboard.fromJson(e.key), {})))",
                u8_held("e.value as int")
            ))
        ),
    );
}

/// An optional map of models is read in the closure, each model with no judgement.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn an_optional_map_of_models_reads_each_unjudged_in_the_closure() {
    assert_written(
        &limber_dart::dart_definition(),
        &format!(
            "Limber(notches: {},);",
            or_null(
                "Map<String, Scantling>.fromEntries(json.entries.map((e) => MapEntry(e.key, \
                 Scantling.fromJsonUnjudged(e.value))))"
            )
        ),
    );
}

/// An optional map that names a function is read from what the function answers, with no closure:
/// an entry that does not read refuses the payload, as serde refuses it.
#[test]
fn an_optional_map_that_names_a_function_is_read_as_it_was() {
    assert_written(
        &hawsehole_dart::dart_definition(),
        &format!(
            "Hawsehole(counts: switch (asGiven(json)) {{ final corrected => (corrected == null ? \
             null : {}) }},);",
            counts_from("(corrected as Map<String, dynamic>)")
        ),
    );
}
