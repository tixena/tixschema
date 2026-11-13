//! What the Dart backend writes for the read of each shape: `fromJson`, which refuses a value its
//! `validate()` refuses, and `fromJsonUnjudged`, the read alone. The definitions of `validate` are
//! read here, beside the shapes that module does not declare.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::held_numbers::{u8_held, u32_held};
use super::unknown_keys::{READ_AT, plain_enum_read, refusing};
use super::validate::{
    closed_dock_dart, code_dart, consignment_dart, crate_dart, depot_dart, freight_dart,
    gauge_dart, grade_dart, haul_dart, lift_dart, manifest_dart, pair_dart, parcel_dart,
    rating_dart, reference_dart, sleeve_dart,
};

/// The helper a variant asks a value it holds for the bound of its own with.
const HELD_BOUND: &str = "static List<String> _heldBound(Object held) { try { return (held as \
     dynamic).validateBound() as List<String>; } on NoSuchMethodError { return const []; } }";

/// What a judged read does with the value its unjudged read answered.
const JUDGED: &str = "final violations = read.validationIssues(); if (violations.isNotEmpty) throw \
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
pub struct Slip(String);

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
pub struct Wrapped(Item);

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
    Shelved(bool),
}

/// A plain enum one member of which is named after the unjudged read.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Reading {
    FromJsonUnjudged,
    Judged,
}

/// A member of a fixed length, one of fixed lengths at two levels, and a list of any length.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Vise {
    pub grid: [[u8; 2]; 3],
    pub listed: Vec<u8>,
    pub pair: [u8; 2],
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

/// The one read of a draft, whose name is `named`, taking `parameters`.
fn draft(named: &str, parameters: &str) -> String {
    format!("factory {named}.fromJson({parameters}) ")
}

/// The pair of a brand, which has the factory `of` besides.
fn sealed_pair(named: &str, parameters: &str, arguments: &str) -> String {
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
    let item = ItemDraft {
        label: "A".to_owned(),
    }
    .validate()
    .unwrap();
    let hold = HoldDraft {
        by_dock: HashMap::from([
            (DockDraft::North.validate().unwrap(), item.clone()),
            (DockDraft::South.validate().unwrap(), item.clone()),
        ]),
        filled: BinDraft {
            held: vec![item.clone()],
        }
        .validate()
        .unwrap(),
    };
    assert_eq!(hold.by_dock.len(), 2);
    assert_eq!(hold.filled.held.len(), 1);
    assert_eq!(Wrapped::new(item.clone()).unwrap().as_inner().label, "A");
    assert_eq!(LotDraft("a".to_owned(), item.clone()).1.label, "A");
    let load: Load = vec![item];
    assert_eq!(load.len(), 1);
    let shelfmarks = [
        ShelfmarkDraft::Coded(Slip::new("AB".to_owned()).unwrap()),
        ShelfmarkDraft::Listed(vec![Slip::new("AB".to_owned()).unwrap()]),
        ShelfmarkDraft::Shelved(true),
    ];
    assert_eq!(shelfmarks.len(), 3);
    assert_eq!(
        serde_json::to_string(&[ReadingDraft::FromJsonUnjudged, ReadingDraft::Judged]).unwrap(),
        r#"["FromJsonUnjudged","Judged"]"#
    );
    let vise = ViseDraft {
        grid: [[1, 2]; 3],
        listed: vec![3],
        pair: [4, 5],
    };
    assert_eq!(
        serde_json::to_string(&vise).unwrap(),
        r#"{"grid":[[1,2],[1,2],[1,2]],"listed":[3],"pair":[4,5]}"#,
        "serde no longer writes a fixed array as an array of its items"
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
            "{}{{ {}return ParcelDraft(fragile_item_codes: _readAt(json, 'fragileItemCodes', () => \
             (json['fragileItemCodes'] as List<dynamic>).indexed.map((e) => _readAt(null, e.$1, \
             () => e.$2 as String)).toList()),item_code_list: _readAt(json, 'itemCodeList', () => \
             (json['itemCodeList'] as List<dynamic>).indexed.map((e) => _readAt(null, e.$1, () => \
             e.$2 as String)).toList()),label: _readAt(json, 'label', () => json['label'] as \
             String),); }} {READ_AT} Map<String, dynamic> toJson() =>",
            draft("ParcelDraft", "Map<String, dynamic> json"),
            refusing("Parcel", &["fragileItemCodes", "itemCodeList", "label"])
        ),
    );
    assert_written(
        &depot_dart::dart_definition(),
        &format!(
            "{}{{ {}return DepotDraft(name: _readAt(json, 'name', () => json['name'] as String),); }}",
            draft("DepotDraft", "Map<String, dynamic> json"),
            refusing("Depot", &["name"])
        ),
    );
    assert_written(
        &closed_dock_dart::dart_definition(),
        &format!(
            "{}{{ {}return ClosedDockDraft(); }}",
            draft("ClosedDockDraft", "Map<String, dynamic> json"),
            refusing("ClosedDock", &[])
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
            "{}{{ {}return ManifestDraft(first_parcel: _readAt(json, 'firstParcel', () => \
             Parcel.fromJsonUnjudged(json['firstParcel'])),parcel_list: _readAt(json, \
             'parcelList', () => (json['parcelList'] as List<dynamic>).indexed.map((e) => \
             _readAt(null, e.$1, () => Parcel.fromJsonUnjudged(e.$2))).toList()),parcels_by_dock: \
             _readAt(json, 'parcelsByDock', () => Map<String, \
             Parcel>.fromEntries((json['parcelsByDock'] as Map<String, dynamic>).entries.map((e) \
             => MapEntry(e.key, _readAt(null, e.key, () => \
             Parcel.fromJsonUnjudged(e.value)))))),spare_parcel: _readAt(json, 'spareParcel', () \
             => (json['spareParcel'] == null ? null : \
             Parcel.fromJsonUnjudged(json['spareParcel']))),); }}",
            draft("ManifestDraft", "Map<String, dynamic> json"),
            refusing(
                "Manifest",
                &["firstParcel", "parcelList", "parcelsByDock", "spareParcel"]
            )
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
            "{}{{ {}return CrateDraft(codes: _readAt(json, 'codes', () => (json['codes'] as \
             List<dynamic>).indexed.map((e) => _readAt(null, e.$1, () => \
             codeTypeFromJson(e.$2))).toList()),label: _readAt(json, 'label', () => json['label'] \
             as String),); }}",
            draft("CrateDraft", converters),
            refusing("Crate", &["codes", "label"])
        ),
    );
    let hold = hold_dart::dart_definition();
    assert_written(
        &hold,
        "filled: _readAt(json, 'filled', () => Bin.fromJsonUnjudged(json['filled'], (e) => \
         Item.fromJson(e))),",
    );
    assert_written(
        &hold,
        "MapEntry(Dock.fromJson(e.key), _readAt(null, e.key, () => Item.fromJsonUnjudged(e.value))",
    );
}

/// A member's own bound runs beside what its type declares, so `validate()` runs the brand it
/// holds, and the brand is read by its unjudged read.
#[test]
fn a_bounded_member_reads_the_type_it_holds_unjudged() {
    assert_written(
        &gauge_dart::dart_definition(),
        "sort_code: _readAt(json, 'sortCode', () => (json['sortCode'] == null ? null : \
         Code.fromJsonUnjudged(json['sortCode']))),",
    );
}

/// A brand's `validate()` runs nothing of what it wraps, and an alias runs its target only where
/// that is a declared type itself.
#[test]
fn a_wrapper_reads_unjudged_what_its_validate_runs() {
    assert_written(
        &code_dart::dart_definition(),
        &format!(
            "{}=> Code._(json as String); dynamic toJson() => value;",
            sealed_pair("Code", "dynamic json", "json")
        ),
    );
    assert_written(
        &wrapped_dart::dart_definition(),
        &format!(
            "{}=> Wrapped._(Item.fromJson(json));",
            sealed_pair("Wrapped", "dynamic json", "json")
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
            "{}=> LoadType((json as List<dynamic>).indexed.map((e) => _readAt(null, e.$1, () => \
             Item.fromJson(e.$2))).toList());",
            pair("LoadType", "dynamic json", "json")
        ),
    );
}

/// A tuple struct reads each slot as a member. The struct of one slot is read from the slot's
/// value and written as it, as serde writes it.
#[test]
fn a_tuple_struct_reads_each_slot_as_a_member() {
    let second = format!("_readAt(null, 1, () => {})", u32_held("tuple[1] as int"));
    assert_written(
        &pair_dart::dart_definition(),
        &format!(
            "{}=> PairDraft(switch (json as List<dynamic>) {{ final tuple when tuple.length == 2 => \
             (_readAt(null, 0, () => tuple[0] as String), {second}), final tuple => throw \
             FormatException('invalid length ${{tuple.length}}, expected 2') }});",
            draft("PairDraft", "dynamic json"),
        ),
    );
    assert_written(
        &lot_dart::dart_definition(),
        &format!(
            "{}=> LotDraft(switch (json as List<dynamic>) {{ final tuple when tuple.length == 2 => \
             (_readAt(null, 0, () => tuple[0] as String), _readAt(null, 1, () => \
             Item.fromJsonUnjudged(tuple[1]))), final tuple => throw FormatException('invalid \
             length ${{tuple.length}}, expected 2') }});",
            draft("LotDraft", "dynamic json")
        ),
    );
    assert_written(
        &sleeve_dart::dart_definition(),
        &format!(
            "{}=> SleeveDraft(json as String); dynamic toJson() => value;",
            draft("SleeveDraft", "dynamic json")
        ),
    );
}

/// The base of a tagged enum judges, and its unjudged read dispatches on the tag: to the
/// unjudged read of a variant with named members, and to the constructor of any other.
#[test]
fn a_tagged_enum_dispatches_unjudged_and_judges_once() {
    let freight = freight_dart::dart_definition();
    let second = format!("_readAt(null, 1, () => {})", u32_held("tuple[1] as int"));
    assert_written(
        &freight,
        &format!(
            "{}{{ if (json is String) {{ switch (json) {{ case 'Held': return FreightHeld(); \
             case 'Boxed': throw FormatException('invalid type: unit variant, expected newtype variant', <Object>[json]); case 'Loose': throw FormatException('invalid type: unit variant, expected struct variant', <Object>[json]); case 'Split': throw FormatException('invalid type: unit variant, expected tuple variant', <Object>[json]); default: throw FormatException('Unknown Freight: ' + json); }} }} final map = json is \
             Map<String, dynamic> && json.length == 1 ? json : throw const \
             FormatException('expected value'); final tag = map.keys.first; switch (tag) {{ case 'Boxed': \
             return _readAt(null, tag, () => \
             FreightBoxed(Parcel.fromJsonUnjudged(map['Boxed']))); case 'Held': return \
             _readAt(null, tag, () => map[tag] == null ? FreightHeld() : throw \
             FormatException('Freight: ' + tag + ' takes no value')); case 'Loose': return \
             _readAt(null, tag, () => FreightLoose.fromJson(map['Loose'])); case 'Split': \
             return _readAt(null, tag, () => FreightSplit(switch (map['Split'] as List<dynamic>) \
             {{ final tuple when tuple.length == 2 => (_readAt(null, 0, () => tuple[0] as \
             String), {second}), final tuple => throw FormatException('invalid length \
             ${{tuple.length}}, expected 2') }})); default: throw FormatException('Unknown \
             Freight: ' + tag); }} }} {READ_AT} dynamic toJson();",
            draft("FreightDraft", "dynamic json"),
        ),
    );
    assert_written(
        &freight,
        &format!(
            "{}{{ {}return FreightLoose(freight_label: _readAt(json, 'freightLabel', () => \
             json['freightLabel'] as String),); }} {READ_AT} @override \
             dynamic toJson() =>",
            draft("FreightLoose", "Map<String, dynamic> json"),
            refusing("FreightLoose", &["freightLabel"])
        ),
    );
    let haul = haul_dart::dart_definition();
    assert_written(
        &haul,
        &format!(
            "{}{{ {}final tag = _readAt(json, 'kind', () => json['kind'] as String); switch (tag) \
             {{ case 'Boxed': return _readAt(json, 'held', () => HaulBoxed(json['held'] as \
             String)); case 'Loose': return _readAt(json, 'held', () => \
             HaulLoose.fromJson(json['held']));",
            draft("HaulDraft", "Map<String, dynamic> json"),
            refusing("Haul", &["kind", "held"])
        ),
    );
    let lift = lift_dart::dart_definition();
    assert_written(
        &lift,
        &format!(
            "{}{{ final tag = _readAt(json, 'kind', () => json['kind'] as String); switch (tag) \
             {{ case 'Boxed': return \
             LiftBoxed(Parcel.fromJsonUnjudged(({{...json}}..remove('kind')))); case 'Loose': \
             return _readFlattened(() => LiftLoose.fromJson(json)); default:",
            draft("LiftDraft", "Map<String, dynamic> json")
        ),
    );
    assert_written(
        &lift,
        &format!(
            "{}{{ {}return LiftLoose(lift_label: _readAt(json, 'liftLabel', () => \
             json['liftLabel'] as String),); }} {READ_AT} Map<String, dynamic> toJson() =>",
            draft("LiftLoose", "Map<String, dynamic> json"),
            refusing("LiftLoose", &["kind", "liftLabel"])
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
            "{}{{ try {{ final read = ReferenceNamed.fromJson(json as Map<String, \
             dynamic>); if (read._bounds().isEmpty) return read; }} catch (_) {{}} try {{ final \
             read = ReferenceNumbered({numbered}); if (read._bounds().isEmpty) return read; }} \
             catch (_) {{}} throw FormatException('No variant of Reference matched'); }} dynamic \
             toJson();",
            draft("ReferenceDraft", "dynamic json"),
            numbered = u32_held("json as int")
        ),
    );
    assert_written(
        &reference,
        "Map<String, dynamic> toJson() => { 'name': name, }; List<String> _bounds() => [ if \
         (name.runes.length < 1) \"too short: minimum length is 1, got ${name.runes.length}\", \
         ]; @override List<String> validationIssues() =>",
    );
    assert_written(
        &reference,
        &format!(
            "@override dynamic toJson() => {}; List<String> _bounds() => [ if (value < 1) \"too \
             small: minimum is 1, got ${{value}}\", ]; @override List<String> validationIssues() =>",
            u32_held("value")
        ),
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
         ShelfmarkListed((json as List<dynamic>).indexed.map((e) => _readAt(null, e.$1, () => \
         Slip.fromJsonUnjudged(e.$2))).toList()); if (read._bounds().isEmpty) return read; } \
         catch (_) {} try { return ShelfmarkShelved(json as bool); } catch (_) {} throw",
    );
    assert_written(
        &shelfmark,
        &format!(
            "List<String> _bounds() => [ ..._heldBound(value), ]; {HELD_BOUND} @override \
             List<String> validationIssues() =>"
        ),
    );
    assert_written(
        &shelfmark,
        &format!(
            "List<String> _bounds() => [ for (final held in value) ..._heldBound(held), ]; \
             {HELD_BOUND} @override List<String> validationIssues() =>"
        ),
    );
    assert!(
        shelfmark.ends_with(
            "@override dynamic toJson() => value; @override List<String> validationIssues() => const []; }"
        ),
        "got: {shelfmark}"
    );
}

/// The draft of a plain enum reads with no check, and the valid type judges what its draft reads
/// and carries the unjudged read a holder calls.
#[test]
fn a_plain_enum_judges_where_it_names_a_check() {
    assert_written(
        &grade_dart::dart_definition(),
        &format!(
            "static GradeDraft fromJson(dynamic json) {{ {} }} String toJson() => wireValue;",
            plain_enum_read(
                "Grade",
                "json",
                "'Express' => GradeDraft.express,'Standard' => GradeDraft.standard,"
            )
        ),
    );
    assert_written(
        &rating_dart::dart_definition(),
        &format!(
            "static RatingDraft fromJson(dynamic json) {{ {} }} String toJson() => wireValue;",
            plain_enum_read(
                "Rating",
                "json",
                "'Express' => RatingDraft.express,'Unrated' => RatingDraft.unrated,"
            )
        ),
    );
    assert_written(
        &rating_dart::dart_definition(),
        &format!(
            "factory Rating.fromJson(dynamic json) {{ final read = RatingDraft.fromJson(json); \
             {} factory Rating.fromJsonUnjudged(dynamic json) => \
             Rating._(RatingDraft.fromJson(json));",
            "final violations = read.validationIssues(); if (violations.isNotEmpty) throw \
             FormatException(violations.join('; ')); return Rating._(read); }"
        ),
    );
}

/// The member moves off the name of the read, and the value serde writes stays.
#[test]
fn an_enum_member_named_after_the_unjudged_read_moves() {
    let reading = reading_dart::dart_definition();
    assert_written(
        &reading,
        "enum ReadingDraft { fromJsonUnjudged_('FromJsonUnjudged'), judged('Judged');",
    );
    assert_written(
        &reading,
        "'FromJsonUnjudged' => ReadingDraft.fromJsonUnjudged_,'Judged' => ReadingDraft.judged,",
    );
}

/// A level written as `[T; N]` binds its array once and refuses one of another length before it
/// maps the items, at each level; a list of any length is read as it was.
#[test]
fn a_fixed_array_is_read_from_an_array_of_its_length() {
    let vise = vise_dart::dart_definition();
    let item = format!("_readAt(null, e.$1, () => {})", u8_held("e.$2 as int"));
    assert_written(
        &vise,
        &format!(
            "grid: _readAt(json, 'grid', () => switch (json['grid'] as List<dynamic>) {{ final \
             array when array.length == 3 => array.indexed.map((e) => _readAt(null, e.$1, () => \
             switch (e.$2 as List<dynamic>) {{ final array when array.length == 2 => \
             array.indexed.map((e) => {item}).toList(), final array => throw \
             FormatException('invalid length ${{array.length}}, expected 2') }})).toList(), final \
             array => throw FormatException('invalid length ${{array.length}}, expected 3') }}),"
        ),
    );
    assert_written(
        &vise,
        &format!(
            "listed: _readAt(json, 'listed', () => Uint8List.fromList((json['listed'] as \
             List<dynamic>).indexed.map((e) => {item}).toList())),"
        ),
    );
    assert_written(
        &vise,
        &format!(
            "pair: _readAt(json, 'pair', () => switch (json['pair'] as List<dynamic>) {{ final \
             array when array.length == 2 => array.indexed.map((e) => {item}).toList(), final \
             array => throw FormatException('invalid length ${{array.length}}, expected 2') }}),"
        ),
    );
}
