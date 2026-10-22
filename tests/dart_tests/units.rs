//! `()` in the Dart read and write: read from the `null` serde writes for it and from nothing
//! else, written as `null`, and refused where its key is absent, as serde refuses it. A tuple
//! struct of no slot, which is no `()`, is read from and written as `[]`.

use super::held_numbers::u32_held;
use super::unit_models::{
    probe_nothing_dart, probe_slotless_dart, probe_unit_adjacent_dart,
    probe_unit_either_holder_dart, probe_units_dart,
};

/// The read of `()` from `expr`, as each position writes it.
fn unit_read(expr: &str) -> String {
    format!(
        "switch ({expr}) {{ null => (), final unit => throw FormatException('invalid type: \
         ${{jsonEncode(unit)}}, expected unit') }}"
    )
}

/// A field, a list's item and a tuple's position read `()` from `null` alone, a key that holds
/// `()` is refused where it is absent, and each writes `null`.
#[test]
fn unit_is_read_from_null_and_written_as_null_in_each_position() {
    let units = probe_units_dart::dart_definition();
    for read in [
        format!(
            "listed: (json['listed'] as List<dynamic>).map((e) => {}).toList()",
            unit_read("e")
        ),
        format!(
            "({}, {})",
            u32_held("tuple[0] as int"),
            unit_read("tuple[1]")
        ),
        format!(
            "unit: (json.containsKey('unit') ? {} : throw FormatException('missing field \
             `unit`'))",
            unit_read("json['unit']")
        ),
        "nothing: (json.containsKey('nothing') ? ProbeNothing.fromJsonUnjudged(json['nothing']) \
         : throw FormatException('missing field `nothing`'))"
            .to_owned(),
        format!(
            "'listed': listed.map((e) => null).toList(),'nothing': (nothing).toJson(),'paired': \
             [{}, null],'unit': null,",
            u32_held("(paired).$1")
        ),
        "final List<()> listed;final ProbeNothing nothing;final (int, ()) paired;final () unit;"
            .to_owned(),
    ] {
        assert!(units.contains(&read), "no {read} in: {units}");
    }
}

/// A brand over `()` reads `null` alone and writes `null`.
#[test]
fn a_brand_over_unit_reads_and_writes_null() {
    let nothing = probe_nothing_dart::dart_definition();
    for read in [
        format!(
            "factory ProbeNothing.fromJsonUnjudged(dynamic json) => ProbeNothing({});",
            unit_read("json")
        ),
        "dynamic toJson() => null;".to_owned(),
    ] {
        assert!(nothing.contains(&read), "no {read} in: {nothing}");
    }
}

/// A variant's one value of `()` under a content key is refused where the content key is absent.
#[test]
fn a_unit_under_a_content_key_is_refused_where_the_key_is_absent() {
    let adjacent = probe_unit_adjacent_dart::dart_definition();
    let read = format!(
        "case 'Nothing': return ProbeUnitAdjacentNothing((json.containsKey('body') ? {} : throw \
         FormatException('missing field `body`')));",
        unit_read("json['body']")
    );
    assert!(adjacent.contains(&read), "no {read} in: {adjacent}");
}

/// A key that holds an untagged enum with a member over `()` is refused where it is absent, the
/// enum reading `null` as that member.
#[test]
fn an_untagged_enum_over_unit_is_refused_where_its_key_is_absent() {
    let holder = probe_unit_either_holder_dart::dart_definition();
    let read = "either: (json.containsKey('either') ? \
                ProbeUnitEither.fromJsonUnjudged(json['either']) : throw \
                FormatException('missing field `either`'))";
    assert!(holder.contains(read), "no {read} in: {holder}");
}

/// A tuple struct of no slot keeps the read and the write of `[]`.
#[test]
fn a_tuple_struct_of_no_slot_reads_and_writes_an_empty_array() {
    let slotless = probe_slotless_dart::dart_definition();
    for read in [
        "ProbeSlotless(switch (json as List<dynamic>) { final tuple when tuple.length == 0 => (), \
         final tuple => throw FormatException('invalid length ${tuple.length}, expected 0') });",
        "dynamic toJson() => [];",
    ] {
        assert!(slotless.contains(read), "no {read} in: {slotless}");
    }
}
