//! What the Dart backend writes for the read of each shape: `fromJson`, which refuses a value its
//! `validate()` refuses, and `fromJsonUnjudged`, the read alone. The definitions of `validate` are
//! read here, beside the shapes that module does not declare.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::validate::{
    closed_dock_dart, code_dart, consignment_dart, crate_dart, depot_dart, freight_dart,
    gauge_dart, grade_dart, haul_dart, lift_dart, manifest_dart, pair_dart, parcel_dart,
    rating_dart, reference_dart, sleeve_dart,
};

/// The helper a variant asks a value it holds for the bound of its own with.
const HELD_BOUND: &str = "static List<String> _heldBound(Object held) { try { return (held as \
     dynamic).validateBound() as List<String>; } on NoSuchMethodError { return const []; } }";

/// What a judged read does with the value its unjudged read answered.
const JUDGED: &str = "final violations = read.validate(); if (violations.isNotEmpty) throw \
     FormatException(violations.join('; ')); return read; }";

/// A model with a bound.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Item {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// A brand with no rule.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Slip(pub String);

/// A type with a parameter.
#[model_schema(default_types(Held = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Bin<Held> {
    pub held: Vec<Held>,
}

/// The name a dock is keyed by.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Dock {
    North,
    South,
}

/// A model as a map's value under a key that is a declared type, and behind a type parameter.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Hold {
    pub by_dock: HashMap<Dock, Item>,
    pub filled: Bin<Item>,
}

/// A brand over a model, whose `validate()` runs nothing of what it wraps.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Wrapped(pub Item);

/// A tuple struct one slot of which holds a model.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Lot(pub String, pub Item);

/// An alias of a list of models, whose `validate()` runs nothing.
#[model_schema()]
pub type Load = Vec<Item>;

/// An enum serde writes as the value of its variant, two of which hold a declared type.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Shelfmark {
    Coded(Slip),
    Listed(Vec<Slip>),
    Titled(String),
}

/// A plain enum one member of which is named after the unjudged read.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Reading {
    FromJsonUnjudged,
    Judged,
}

/// The pair every class type writes: `named`'s judged read over its unjudged one, which takes
/// `parameters` and is handed `arguments`.
fn pair(named: &str, parameters: &str, arguments: &str) -> String {
    format!(
        "factory {named}.fromJson({parameters}) {{ final read = \
         {named}.fromJsonUnjudged({arguments}); {JUDGED} factory \
         {named}.fromJsonUnjudged({parameters}) "
    )
}

/// Asserts that `definition` holds `fragment`.
fn assert_written(definition: &str, fragment: &str) {
    assert!(
        definition.contains(fragment),
        "missing `{fragment}` in: {definition}"
    );
}

#[test]
fn every_declared_type_is_constructible() {
    let item = Item {
        label: "A".to_owned(),
    };
    let hold = Hold {
        by_dock: HashMap::from([(Dock::North, item.clone()), (Dock::South, item.clone())]),
        filled: Bin {
            held: vec![item.clone()],
        },
    };
    assert_eq!(hold.by_dock.len(), 2);
    assert_eq!(hold.filled.held.len(), 1);
    assert_eq!(Wrapped(item.clone()).0.label, "A");
    assert_eq!(Lot("a".to_owned(), item.clone()).1.label, "A");
    let load: Load = vec![item];
    assert_eq!(load.len(), 1);
    let shelfmarks = [
        Shelfmark::Coded(Slip("AB".to_owned())),
        Shelfmark::Listed(vec![Slip("AB".to_owned())]),
        Shelfmark::Titled("t".to_owned()),
    ];
    assert_eq!(shelfmarks.len(), 3);
    assert_eq!(
        serde_json::to_string(&[Reading::FromJsonUnjudged, Reading::Judged]).unwrap(),
        r#"["FromJsonUnjudged","Judged"]"#
    );
}

/// `fromJson` reads the value unjudged and throws what `validate()` answers of it, joined by
/// `; `. `fromJsonUnjudged` holds the read alone.
#[test]
fn a_class_reads_unjudged_then_refuses_what_validate_answers() {
    let parcel = parcel_dart::dart_definition();
    assert_written(
        &parcel,
        &format!(
            "{}=> Parcel(fragile_item_codes: (json['fragileItemCodes'] as List<dynamic>).map((e) \
             => e as String).toList(),item_code_list: (json['itemCodeList'] as \
             List<dynamic>).map((e) => e as String).toList(),label: json['label'] as String,); \
             Map<String, dynamic> toJson() =>",
            pair("Parcel", "Map<String, dynamic> json", "json")
        ),
    );
    assert_written(
        &depot_dart::dart_definition(),
        &format!(
            "{}=> Depot(name: json['name'] as String,);",
            pair("Depot", "Map<String, dynamic> json", "json")
        ),
    );
    assert_written(
        &closed_dock_dart::dart_definition(),
        &format!(
            "{}=> ClosedDock();",
            pair("ClosedDock", "Map<String, dynamic> json", "json")
        ),
    );
}

/// A holder reads each model it names unjudged, in a field, a list, a map's value and an
/// optional, so the whole value is judged once.
#[test]
fn a_holder_reads_what_it_names_unjudged() {
    assert_written(
        &manifest_dart::dart_definition(),
        &format!(
            "{}=> Manifest(first_parcel: Parcel.fromJsonUnjudged(json['firstParcel']),parcel_list: \
             (json['parcelList'] as List<dynamic>).map((e) => \
             Parcel.fromJsonUnjudged(e)).toList(),parcels_by_dock: Map<String, \
             Parcel>.fromEntries((json['parcelsByDock'] as Map<String, dynamic>).entries.map((e) \
             => MapEntry(e.key as String, Parcel.fromJsonUnjudged(e.value)))),spare_parcel: \
             (json['spareParcel'] == null ? null : \
             Parcel.fromJsonUnjudged(json['spareParcel'])),);",
            pair("Manifest", "Map<String, dynamic> json", "json")
        ),
    );
}

/// A generic type takes its converters in both reads. A value behind a parameter and the key of
/// a map are not reached by `validate()`, so each is read by its own judged read.
#[test]
fn a_value_validate_does_not_reach_judges_itself() {
    let converters = "Map<String, dynamic> json, CodeType Function(dynamic) codeTypeFromJson";
    assert_written(
        &crate_dart::dart_definition(),
        &format!(
            "{}=> Crate(codes: (json['codes'] as List<dynamic>).map((e) => \
             codeTypeFromJson(e)).toList(),label: json['label'] as String,);",
            pair("Crate", converters, "json, codeTypeFromJson")
        ),
    );
    let hold = hold_dart::dart_definition();
    assert_written(
        &hold,
        "filled: Bin.fromJsonUnjudged(json['filled'], (e) => Item.fromJson(e)),",
    );
    assert_written(
        &hold,
        "MapEntry(Dock.fromJson(e.key), Item.fromJsonUnjudged(e.value))",
    );
}

/// A member's own bound stands in for what its type declares, so `validate()` does not run the
/// brand it holds, and the brand is read by its own judged read.
#[test]
fn a_bounded_member_reads_the_type_it_holds_judged() {
    assert_written(
        &gauge_dart::dart_definition(),
        "sort_code: (json['sortCode'] == null ? null : Code.fromJson(json['sortCode'])),",
    );
}

/// A brand's `validate()` runs nothing of what it wraps, and an alias runs its target only where
/// that is a declared type itself.
#[test]
fn a_wrapper_reads_unjudged_what_its_validate_runs() {
    assert_written(
        &code_dart::dart_definition(),
        &format!(
            "{}=> Code(json as String); dynamic toJson() => value;",
            pair("Code", "dynamic json", "json")
        ),
    );
    assert_written(
        &wrapped_dart::dart_definition(),
        &format!(
            "{}=> Wrapped(Item.fromJson(json));",
            pair("Wrapped", "dynamic json", "json")
        ),
    );
    assert_written(
        &consignment_dart::dart_definition(),
        &format!(
            "{}=> ConsignmentType(Parcel.fromJsonUnjudged(json));",
            pair("ConsignmentType", "dynamic json", "json")
        ),
    );
    assert_written(
        &load_dart::dart_definition(),
        &format!(
            "{}=> LoadType((json as List<dynamic>).map((e) => Item.fromJson(e)).toList());",
            pair("LoadType", "dynamic json", "json")
        ),
    );
}

/// A tuple struct reads each slot as a member. The struct of one slot is read as the array the
/// backend writes it as.
#[test]
fn a_tuple_struct_reads_each_slot_as_a_member() {
    assert_written(
        &pair_dart::dart_definition(),
        &format!(
            "{}=> Pair(((json as List<dynamic>)[0] as String, (json as List<dynamic>)[1] as \
             int));",
            pair("Pair", "dynamic json", "json")
        ),
    );
    assert_written(
        &lot_dart::dart_definition(),
        &format!(
            "{}=> Lot(((json as List<dynamic>)[0] as String, Item.fromJsonUnjudged((json as \
             List<dynamic>)[1])));",
            pair("Lot", "dynamic json", "json")
        ),
    );
    assert_written(
        &sleeve_dart::dart_definition(),
        &format!(
            "{}=> Sleeve(((json as List<dynamic>)[0] as String,)); dynamic toJson() => \
             [(value).$1];",
            pair("Sleeve", "dynamic json", "json")
        ),
    );
}

/// The base of a tagged enum judges, and its unjudged read dispatches on the tag: to the
/// unjudged read of a variant with named members, and to the constructor of any other.
#[test]
fn a_tagged_enum_dispatches_unjudged_and_judges_once() {
    let freight = freight_dart::dart_definition();
    assert_written(
        &freight,
        &format!(
            "{}{{ if (json is String) {{ switch (json) {{ case 'Held': return FreightHeld(); \
             default: throw ArgumentError('Unknown Freight: ' + json); }} }} final map = json as \
             Map<String, dynamic>; final tag = map.keys.first; switch (tag) {{ case 'Boxed': \
             return FreightBoxed(Parcel.fromJsonUnjudged(map['Boxed'])); case 'Loose': return \
             FreightLoose.fromJsonUnjudged(map['Loose']); case 'Split': return \
             FreightSplit(((map['Split'] as List<dynamic>)[0] as String, (map['Split'] as \
             List<dynamic>)[1] as int)); default: throw ArgumentError('Unknown Freight: ' + tag); \
             }} }} dynamic toJson();",
            pair("Freight", "dynamic json", "json")
        ),
    );
    assert_written(
        &freight,
        &format!(
            "{}=> FreightLoose(freight_label: json['freightLabel'] as String,); @override \
             dynamic toJson() =>",
            pair("FreightLoose", "Map<String, dynamic> json", "json")
        ),
    );
    let haul = haul_dart::dart_definition();
    assert_written(
        &haul,
        &format!(
            "{}{{ final tag = json['kind'] as String; switch (tag) {{ case 'Boxed': return \
             HaulBoxed(json['held'] as String); case 'Loose': return \
             HaulLoose.fromJsonUnjudged(json['held']);",
            pair("Haul", "Map<String, dynamic> json", "json")
        ),
    );
    let lift = lift_dart::dart_definition();
    assert_written(
        &lift,
        &format!(
            "{}{{ final tag = json['kind'] as String; switch (tag) {{ case 'Boxed': return \
             LiftBoxed(Parcel.fromJsonUnjudged(json)); case 'Loose': return \
             LiftLoose.fromJsonUnjudged(json); default:",
            pair("Lift", "Map<String, dynamic> json", "json")
        ),
    );
    assert_written(
        &lift,
        &format!(
            "{}=> LiftLoose(lift_label: json['liftLabel'] as String,); Map<String, dynamic> \
             toJson() =>",
            pair("LiftLoose", "Map<String, dynamic> json", "json")
        ),
    );
}

/// An untagged enum takes the first variant whose payload decodes and whose members' own bounds
/// hold. `_bounds()` lists them, with no check and nothing a held model reports.
#[test]
fn an_untagged_enum_takes_the_first_variant_whose_bounds_hold() {
    let reference = reference_dart::dart_definition();
    assert_written(
        &reference,
        &format!(
            "{}{{ try {{ final read = ReferenceNamed.fromJsonUnjudged(json as Map<String, \
             dynamic>); if (read._bounds().isEmpty) return read; }} catch (_) {{}} try {{ final \
             read = ReferenceNumbered(json as int); if (read._bounds().isEmpty) return read; }} \
             catch (_) {{}} throw ArgumentError('No variant of Reference matched'); }} dynamic \
             toJson();",
            pair("Reference", "dynamic json", "json")
        ),
    );
    assert_written(
        &reference,
        "Map<String, dynamic> toJson() => { 'name': name, }; List<String> _bounds() => [ if \
         (name.runes.length < 1) \"too short: minimum length is 1, got ${name.runes.length}\", \
         ]; @override List<String> validate() =>",
    );
    assert_written(
        &reference,
        "@override dynamic toJson() => value; List<String> _bounds() => [ if (value < 1) \"too \
         small: minimum is 1, got ${value}\", ]; @override List<String> validate() =>",
    );
}

/// A variant asks each declared type it holds for the bound of its own, through a list as
/// through the value itself. A variant that holds none is taken by its shape alone.
#[test]
fn a_variant_asks_what_it_holds_for_its_own_bound() {
    let shelfmark = shelfmark_dart::dart_definition();
    assert_written(
        &shelfmark,
        "try { final read = ShelfmarkCoded(Slip.fromJsonUnjudged(json)); if \
         (read._bounds().isEmpty) return read; } catch (_) {} try { final read = \
         ShelfmarkListed((json as List<dynamic>).map((e) => \
         Slip.fromJsonUnjudged(e)).toList()); if (read._bounds().isEmpty) return read; } catch \
         (_) {} try { return ShelfmarkTitled(json as String); } catch (_) {} throw",
    );
    assert_written(
        &shelfmark,
        &format!(
            "List<String> _bounds() => [ ..._heldBound(value), ]; {HELD_BOUND} @override \
             List<String> validate() =>"
        ),
    );
    assert_written(
        &shelfmark,
        &format!(
            "List<String> _bounds() => [ for (final held in value) ..._heldBound(held), ]; \
             {HELD_BOUND} @override List<String> validate() =>"
        ),
    );
    assert!(
        shelfmark.ends_with(
            "@override dynamic toJson() => value; @override List<String> validate() => const []; }"
        ),
        "got: {shelfmark}"
    );
}

/// A plain enum that names no check reads as it did, and one that names a check judges. Both
/// carry the unjudged read a holder calls.
#[test]
fn a_plain_enum_judges_where_it_names_a_check() {
    assert_written(
        &grade_dart::dart_definition(),
        "static Grade fromJson(String json) => switch (json) { 'Express' => \
         Grade.express,'Standard' => Grade.standard, _ => throw ArgumentError('Unknown Grade: ' \
         + json) }; static Grade fromJsonUnjudged(String json) => fromJson(json); String \
         toJson() => wireValue;",
    );
    assert_written(
        &rating_dart::dart_definition(),
        &format!(
            "static Rating fromJson(String json) {{ final read = Rating.fromJsonUnjudged(json); \
             {JUDGED} static Rating fromJsonUnjudged(String json) => switch (json) {{ 'Express' \
             => Rating.express,'Unrated' => Rating.unrated, _ => throw ArgumentError('Unknown \
             Rating: ' + json) }}; String toJson() => wireValue;"
        ),
    );
}

/// The member moves off the name of the read, and the value serde writes stays.
#[test]
fn an_enum_member_named_after_the_unjudged_read_moves() {
    let reading = reading_dart::dart_definition();
    assert_written(
        &reading,
        "enum Reading { fromJsonUnjudged_('FromJsonUnjudged'), judged('Judged');",
    );
    assert_written(
        &reading,
        "'FromJsonUnjudged' => Reading.fromJsonUnjudged_,'Judged' => Reading.judged,",
    );
}
