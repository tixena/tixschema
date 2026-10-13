//! What the Dart backend writes for `validate()`: the text of each shape. No test here calls a
//! Rust `validate()`, so every one holds in a build with a schema surface and in one with none.
//! A brand's own string check is in `bounded_brands`: a build with no schema surface refuses it.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// The codec of each type below but `Audit`, as it was written before any type had `validate()`.
const CODEC: &str = include_str!("codec_before_validate.txt");

/// The helper a type that names a place carries, as it is written into the type.
const UNDER: &str = "static String _under(String place, String violation) { if \
     (violation.startsWith(\"'\")) { final end = violation.indexOf(\"'\", 1); if (end > 0) return \
     \"'$place.${violation.substring(1, end)}'${violation.substring(end + 1)}\"; } return \
     \"'$place': $violation\"; }";

/// The model of the shared table: a bound, a field's check and the type's check.
#[model_schema(check = ["withinParcel"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Parcel {
    pub fragile_item_codes: Vec<String>,
    #[model_schema_prop(check = ["withoutDuplicates"])]
    pub item_code_list: Vec<String>,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// A type with no rule.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Depot {
    pub name: String,
}

/// One field per key a bound is written with, on each kind of value it measures.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gauge {
    #[model_schema_prop(minLength = 1, maxLength = 3)]
    pub code_list: Vec<String>,
    #[model_schema_prop(minimum = 0, maximum = 120)]
    pub item_count: u32,
    #[model_schema_prop(minLength = 2, maxLength = 8, pattern = "^[a-z]+$")]
    pub item_label: String,
    #[model_schema_prop(minLength = 2)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_note: Option<String>,
    #[model_schema_prop(minLength = 1)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort_code: Option<Code>,
    #[model_schema_prop(minimum = 0.5, maximum = 3.0)]
    pub unit_weight: f64,
}

/// A model held in a field, a list, a map and an optional.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub first_parcel: Parcel,
    pub parcel_list: Vec<Parcel>,
    pub parcels_by_dock: HashMap<String, Parcel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_parcel: Option<Parcel>,
}

/// A brand with a check of its own.
#[model_schema(check = ["upperCase"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Code(pub String);

/// A tuple struct of several slots.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Pair(
    #[model_schema_prop(minLength = 1)] pub String,
    #[model_schema_prop(minimum = 1)] pub u32,
);

/// A struct of one slot, which serde writes as the slot's value.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Sleeve(#[model_schema_prop(minLength = 1)] pub String);

/// An enum serde writes under its variant's name.
#[model_schema(check = ["routed"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all_fields = "camelCase")]
pub enum Freight {
    Boxed(Parcel),
    Held,
    Loose {
        #[model_schema_prop(minLength = 1)]
        freight_label: String,
    },
    Split(
        #[model_schema_prop(minLength = 1)] String,
        #[model_schema_prop(minimum = 1)] u32,
    ),
}

/// An enum under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held", rename_all_fields = "camelCase")]
pub enum Haul {
    Boxed(#[model_schema_prop(minLength = 1)] String),
    Loose {
        #[model_schema_prop(minLength = 1)]
        haul_label: String,
    },
    Split(
        #[model_schema_prop(minLength = 1)] String,
        #[model_schema_prop(minimum = 1)] u32,
    ),
}

/// An enum under a tag alone.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all_fields = "camelCase")]
pub enum Lift {
    Boxed(Parcel),
    Loose {
        #[model_schema_prop(minLength = 1)]
        lift_label: String,
    },
}

/// An enum serde writes as the value of its variant.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Reference {
    Named {
        #[model_schema_prop(minLength = 1)]
        name: String,
    },
    Numbered(#[model_schema_prop(minimum = 1)] u32),
}

/// An enum no variant of which holds a value, and which names no check.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Grade {
    Express,
    Standard,
}

/// An enum no variant of which holds a value, with a check.
#[model_schema(check = ["rated"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Rating {
    Express,
    Unrated,
}

/// An enum whose own string check reaches the fields written as text.
#[model_schema(minLength = 2, pattern = "^[a-z]+$")]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum Stamp {
    Inked {
        #[model_schema_prop(maxLength = 9, pattern = "[aeiou]")]
        ink: String,
        pages: u32,
    },
    Pressed {
        seal: String,
    },
}

/// A field named after the method every emitted type has.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Audit {
    pub validate: bool,
}

/// A type with a parameter, whose check is written once over every filling.
#[model_schema(default_types(CodeType = String), check = ["withACode"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Crate<CodeType> {
    pub codes: Vec<CodeType>,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// A unit struct with a check.
#[model_schema(check = ["takingParcels"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ClosedDock;

/// An alias of a model.
#[model_schema()]
pub type Consignment = Parcel;

fn without_duplicates(value: &[String]) -> Result<(), Vec<String>> {
    for (index, code) in value.iter().enumerate() {
        if value[..index].contains(code) {
            return Err(vec![format!("duplicate item code {code}")]);
        }
    }
    Ok(())
}

fn within_parcel(value: &Parcel) -> Result<(), Vec<String>> {
    let outside: Vec<String> = value
        .fragile_item_codes
        .iter()
        .enumerate()
        .filter(|(_, code)| !value.item_code_list.contains(code))
        .map(|(index, code)| {
            format!("'fragileItemCodes.{index}': {code} is outside of itemCodeList")
        })
        .collect();
    if outside.is_empty() {
        Ok(())
    } else {
        Err(outside)
    }
}

fn upper_case(value: &Code) -> Result<(), Vec<String>> {
    if value.0.chars().any(char::is_lowercase) {
        return Err(vec!["a code is upper case".to_owned()]);
    }
    Ok(())
}

fn routed(value: &Freight) -> Result<(), Vec<String>> {
    if matches!(value, Freight::Held) {
        return Err(vec!["a held freight has no route".to_owned()]);
    }
    Ok(())
}

fn rated(value: &Rating) -> Result<(), Vec<String>> {
    if matches!(value, Rating::Unrated) {
        return Err(vec!["a parcel is rated".to_owned()]);
    }
    Ok(())
}

fn with_a_code<CodeType>(value: &Crate<CodeType>) -> Result<(), Vec<String>> {
    if value.codes.is_empty() {
        return Err(vec!["'codes': a crate holds at least one code".to_owned()]);
    }
    Ok(())
}

fn taking_parcels(_value: &ClosedDock) -> Result<(), Vec<String>> {
    Err(vec!["a closed dock takes no parcel".to_owned()])
}

/// Every definition `CODEC` holds the codec of, in its order.
fn definitions() -> String {
    [
        parcel_dart::dart_definition(),
        depot_dart::dart_definition(),
        gauge_dart::dart_definition(),
        manifest_dart::dart_definition(),
        code_dart::dart_definition(),
        pair_dart::dart_definition(),
        sleeve_dart::dart_definition(),
        freight_dart::dart_definition(),
        haul_dart::dart_definition(),
        lift_dart::dart_definition(),
        reference_dart::dart_definition(),
        grade_dart::dart_definition(),
        rating_dart::dart_definition(),
        stamp_dart::dart_definition(),
        crate_dart::dart_definition(),
        closed_dock_dart::dart_definition(),
        consignment_dart::dart_definition(),
    ]
    .join("\n@@\n")
}

/// Asserts that `definition` holds each of `written`.
fn assert_written(definition: &str, written: &[&str]) {
    for fragment in written {
        assert!(
            definition.contains(fragment),
            "missing `{fragment}` in: {definition}"
        );
    }
}

#[test]
fn every_declared_type_is_constructible() {
    let parcel = Parcel {
        fragile_item_codes: Vec::new(),
        item_code_list: Vec::new(),
        label: "A".to_owned(),
    };
    let gauge = Gauge {
        code_list: vec!["a".to_owned()],
        item_count: 1,
        item_label: "ab".to_owned(),
        short_note: None,
        sort_code: Some(Code("AB".to_owned())),
        unit_weight: 1.0,
    };
    assert_eq!(gauge.item_count, 1);
    let manifest = Manifest {
        first_parcel: parcel.clone(),
        parcel_list: vec![parcel.clone()],
        parcels_by_dock: HashMap::from([("north".to_owned(), parcel.clone())]),
        spare_parcel: None,
    };
    assert_eq!(manifest.parcel_list.len(), 1);
    assert_eq!(
        Depot {
            name: "d".to_owned()
        }
        .name,
        "d"
    );
    assert_eq!(Pair("a".to_owned(), 1).1, 1);
    assert_eq!(Sleeve("a".to_owned()).0, "a");
    let freights = [
        Freight::Boxed(parcel.clone()),
        Freight::Held,
        Freight::Loose {
            freight_label: "f".to_owned(),
        },
        Freight::Split("a".to_owned(), 1),
    ];
    assert_eq!(freights.len(), 4);
    let hauls = [
        Haul::Boxed("a".to_owned()),
        Haul::Loose {
            haul_label: "h".to_owned(),
        },
        Haul::Split("a".to_owned(), 1),
    ];
    assert_eq!(hauls.len(), 3);
    let lifts = [
        Lift::Boxed(parcel.clone()),
        Lift::Loose {
            lift_label: "l".to_owned(),
        },
    ];
    assert_eq!(lifts.len(), 2);
    let references = [
        Reference::Named {
            name: "n".to_owned(),
        },
        Reference::Numbered(1),
    ];
    assert_eq!(references.len(), 2);
    assert_eq!([Grade::Express, Grade::Standard].len(), 2);
    assert_eq!([Rating::Express, Rating::Unrated].len(), 2);
    let stamps = [
        Stamp::Inked {
            ink: "blue".to_owned(),
            pages: 1,
        },
        Stamp::Pressed {
            seal: "wax".to_owned(),
        },
    ];
    assert_eq!(stamps.len(), 2);
    assert!(Audit { validate: true }.validate);
    let filled = Crate {
        codes: vec![7_u32],
        label: "c".to_owned(),
    };
    assert_eq!(filled.codes.len(), 1);
    assert_eq!(
        serde_json::to_string(&ClosedDock).unwrap(),
        "{}",
        "a unit struct is written as an empty object"
    );
    let consignment: Consignment = parcel;
    assert_eq!(consignment.label, "A");
}

/// Each function a check names, called as the Rust side calls it.
#[test]
fn each_named_function_answers_for_its_value() {
    let parcel = Parcel {
        fragile_item_codes: vec!["y".to_owned()],
        item_code_list: vec!["x".to_owned(), "x".to_owned()],
        label: "A".to_owned(),
    };
    assert_eq!(
        without_duplicates(&parcel.item_code_list).unwrap_err(),
        ["duplicate item code x"]
    );
    assert_eq!(
        within_parcel(&parcel).unwrap_err(),
        ["'fragileItemCodes.0': y is outside of itemCodeList"]
    );
    assert_eq!(
        upper_case(&Code("ab".to_owned())).unwrap_err(),
        ["a code is upper case"]
    );
    assert_eq!(upper_case(&Code("AB".to_owned())), Ok(()));
    assert_eq!(
        routed(&Freight::Held).unwrap_err(),
        ["a held freight has no route"]
    );
    assert_eq!(routed(&Freight::Boxed(parcel)), Ok(()));
    assert_eq!(rated(&Rating::Unrated).unwrap_err(), ["a parcel is rated"]);
    assert_eq!(rated(&Rating::Express), Ok(()));
    let empty = Crate {
        codes: Vec::<String>::new(),
        label: "c".to_owned(),
    };
    assert_eq!(
        with_a_code(&empty).unwrap_err(),
        ["'codes': a crate holds at least one code"]
    );
    assert_eq!(
        taking_parcels(&ClosedDock).unwrap_err(),
        ["a closed dock takes no parcel"]
    );
}

/// The emitted text of the shared table's model, whole: the bound, each check under the name the
/// attribute writes, the type's check last, and the one helper the type calls.
#[test]
fn the_parcel_of_the_shared_table_is_written_with_its_bound_and_its_checks() {
    assert_eq!(
        parcel_dart::dart_definition(),
        format!(
            "class Parcel {{ const Parcel({{required this.fragile_item_codes,required \
             this.item_code_list,required this.label,}}); final List<String> \
             fragile_item_codes;final List<String> item_code_list;final String label; factory \
             Parcel.fromJson(Map<String, dynamic> json) => Parcel(fragile_item_codes: \
             (json['fragileItemCodes'] as List<dynamic>).map((e) => e as \
             String).toList(),item_code_list: (json['itemCodeList'] as List<dynamic>).map((e) => e \
             as String).toList(),label: json['label'] as String,); Map<String, dynamic> toJson() \
             => {{ 'fragileItemCodes': fragile_item_codes.map((e) => e).toList(),'itemCodeList': \
             item_code_list.map((e) => e).toList(),'label': label, }}; List<String> validate() => \
             [ for (final violation in withoutDuplicates(item_code_list)) _under('itemCodeList', \
             violation), if (label.runes.length < 1) \"'label': too short: minimum length is 1, \
             got ${{label.runes.length}}\", ...withinParcel(this), ]; {UNDER} }}"
        )
    );
}

#[test]
fn a_type_with_no_rule_answers_an_empty_list() {
    let depot = depot_dart::dart_definition();
    assert!(
        depot.ends_with("toJson() => { 'name': name, }; List<String> validate() => const []; }"),
        "got: {depot}"
    );
    assert!(!depot.contains("_under"), "got: {depot}");
}

/// A string's length is its code points, a list's its items, and a number is compared as it is.
/// A whole float is written without `.0`.
#[test]
fn each_bound_is_written_on_the_value_it_measures() {
    let gauge = gauge_dart::dart_definition();
    assert!(
        gauge.contains(
            "List<String> validate() => [ \
             if (code_list.length < 1) \"'codeList': too short: minimum length is 1, got \
             ${code_list.length}\", \
             if (code_list.length > 3) \"'codeList': too long: maximum length is 3, got \
             ${code_list.length}\", \
             if (item_count < 0) \"'itemCount': too small: minimum is 0, got ${item_count}\", \
             if (item_count > 120) \"'itemCount': too large: maximum is 120, got ${item_count}\", \
             if (item_label.runes.length < 2) \"'itemLabel': too short: minimum length is 2, got \
             ${item_label.runes.length}\", \
             if (item_label.runes.length > 8) \"'itemLabel': too long: maximum length is 8, got \
             ${item_label.runes.length}\", \
             if (!RegExp(\"^[a-z]+\\$\").hasMatch(item_label)) \"'itemLabel': does not match \
             pattern '^[a-z]+\\$'\", \
             if (short_note case final held?) if (held.runes.length < 2) \"'shortNote': too \
             short: minimum length is 2, got ${held.runes.length}\", \
             if (sort_code case final held?) if (_boundText(held).runes.length < 1) \
             \"'sortCode': too short: minimum length is 1, got ${_boundText(held).runes.length}\", \
             if (unit_weight < 0.5) \"'unitWeight': too small: minimum is 0.5, got \
             ${_number(unit_weight)}\", \
             if (unit_weight > 3) \"'unitWeight': too large: maximum is 3, got \
             ${_number(unit_weight)}\", ];"
        ),
        "got: {gauge}"
    );
    assert_written(
        &gauge,
        &[
            "static String _boundText(Object held) => held is String ? held : \
             _boundText((held as dynamic).value as Object);",
            "static String _number(double value) => value == value.truncateToDouble() ? \
             value.toStringAsFixed(0) : value.toString();",
        ],
    );
    assert!(!gauge.contains("_under"), "got: {gauge}");
}

#[test]
fn a_holder_runs_what_it_holds_under_its_place() {
    let manifest = manifest_dart::dart_definition();
    assert!(
        manifest.contains(&format!(
            "List<String> validate() => [ \
             for (final violation in first_parcel.validate()) _under('firstParcel', violation), \
             for (final (index, held) in parcel_list.indexed) for (final violation in \
             held.validate()) _under('parcelList.$index', violation), \
             for (final entry in parcels_by_dock.entries) for (final violation in \
             entry.value.validate()) _under('parcelsByDock.${{entry.key}}', violation), \
             if (spare_parcel case final held?) for (final violation in held.validate()) \
             _under('spareParcel', violation), ]; {UNDER} }}"
        )),
        "got: {manifest}"
    );
}

/// A brand is the value and no member of anything: its check takes the brand, and its messages
/// are listed as the function writes them.
#[test]
fn a_brand_calls_its_check_on_itself() {
    let code = code_dart::dart_definition();
    assert!(
        code.ends_with(
            "dynamic toJson() => value; List<String> validate() => [ ...upperCase(this), ]; }"
        ),
        "got: {code}"
    );
}

/// A slot of several is named by its position in the array serde writes, and the one slot of a
/// struct serde writes as the value itself by nothing.
#[test]
fn a_slot_is_named_by_its_position_and_a_lone_slot_by_nothing() {
    let pair = pair_dart::dart_definition();
    assert!(
        pair.ends_with(
            "List<String> validate() => [ if (value.$1.runes.length < 1) \"'0': too short: \
             minimum length is 1, got ${value.$1.runes.length}\", if (value.$2 < 1) \"'1': too \
             small: minimum is 1, got ${value.$2}\", ]; }"
        ),
        "got: {pair}"
    );
    let sleeve = sleeve_dart::dart_definition();
    assert!(
        sleeve.ends_with(
            "List<String> validate() => [ if (value.$1.runes.length < 1) \"too short: minimum \
             length is 1, got ${value.$1.runes.length}\", ]; }"
        ),
        "got: {sleeve}"
    );
}

/// The base of a sealed enum declares `validate()`, and each variant writes its members under the
/// steps serde writes on the way to them: the variant's name, the content key, or nothing.
#[test]
fn a_variant_is_named_by_the_steps_serde_writes() {
    let freight = freight_dart::dart_definition();
    assert_written(
        &freight,
        &[
            "dynamic toJson(); List<String> validate(); } class FreightBoxed extends Freight {",
            "@override List<String> validate() => [ for (final violation in value.validate()) \
             _under('Boxed', violation), ...routed(this), ];",
            "@override dynamic toJson() => 'Held'; @override List<String> validate() => [ \
             ...routed(this), ]; }",
            "if (freight_label.runes.length < 1) \"'Loose.freightLabel': too short: minimum \
             length is 1, got ${freight_label.runes.length}\", ...routed(this), ];",
            "if (value.$1.runes.length < 1) \"'Split.0': too short: minimum length is 1, got \
             ${value.$1.runes.length}\", if (value.$2 < 1) \"'Split.1': too small: minimum is 1, \
             got ${value.$2}\", ...routed(this), ];",
        ],
    );
    let haul = haul_dart::dart_definition();
    assert_written(
        &haul,
        &[
            "dynamic toJson(); List<String> validate(); } class HaulBoxed extends Haul {",
            "@override List<String> validate() => [ if (value.runes.length < 1) \"'held': too \
             short: minimum length is 1, got ${value.runes.length}\", ];",
            "if (haul_label.runes.length < 1) \"'held.haulLabel': too short: minimum length is \
             1, got ${haul_label.runes.length}\", ];",
            "if (value.$1.runes.length < 1) \"'held.0': too short: minimum length is 1, got \
             ${value.$1.runes.length}\", if (value.$2 < 1) \"'held.1': too small: minimum is 1, \
             got ${value.$2}\", ];",
        ],
    );
    let lift = lift_dart::dart_definition();
    assert_written(
        &lift,
        &[
            "dynamic toJson(); List<String> validate(); } class LiftBoxed extends Lift {",
            "@override List<String> validate() => [ ...value.validate(), ]; }",
            "@override List<String> validate() => [ if (lift_label.runes.length < 1) \
             \"'liftLabel': too short: minimum length is 1, got ${lift_label.runes.length}\", ]; }",
        ],
    );
    let reference = reference_dart::dart_definition();
    assert_written(
        &reference,
        &[
            "dynamic toJson(); List<String> validate(); } class ReferenceNamed extends Reference {",
            "@override List<String> validate() => [ if (name.runes.length < 1) \"'name': too \
             short: minimum length is 1, got ${name.runes.length}\", ]; }",
            "@override List<String> validate() => [ if (value < 1) \"too small: minimum is 1, \
             got ${value}\", ]; }",
        ],
    );
}

#[test]
fn an_enum_with_no_data_answers_its_checks() {
    let grade = grade_dart::dart_definition();
    assert!(
        grade.ends_with("String toJson() => wireValue; List<String> validate() => const []; }"),
        "got: {grade}"
    );
    let rating = rating_dart::dart_definition();
    assert!(
        rating.ends_with(
            "String toJson() => wireValue; List<String> validate() => [ ...rated(this), ]; }"
        ),
        "got: {rating}"
    );
}

/// The longer `minLength`, the shorter `maxLength`, and the field's pattern ahead of the enum's.
/// A field that is no text is not reached.
#[test]
fn an_enums_string_check_reaches_the_fields_written_as_text() {
    let stamp = stamp_dart::dart_definition();
    assert_written(
        &stamp,
        &[
            "@override List<String> validate() => [ \
             if (ink.runes.length < 2) \"'ink': too short: minimum length is 2, got \
             ${ink.runes.length}\", \
             if (ink.runes.length > 9) \"'ink': too long: maximum length is 9, got \
             ${ink.runes.length}\", \
             if (!RegExp(\"[aeiou]\").hasMatch(ink)) \"'ink': does not match pattern '[aeiou]'\", \
             if (!RegExp(\"^[a-z]+\\$\").hasMatch(ink)) \"'ink': does not match pattern \
             '^[a-z]+\\$'\", ]; }",
            "@override List<String> validate() => [ \
             if (seal.runes.length < 2) \"'seal': too short: minimum length is 2, got \
             ${seal.runes.length}\", \
             if (!RegExp(\"^[a-z]+\\$\").hasMatch(seal)) \"'seal': does not match pattern \
             '^[a-z]+\\$'\", ]; }",
        ],
    );
    assert!(!stamp.contains("pages."), "got: {stamp}");
}

#[test]
fn a_field_named_validate_moves_and_keeps_its_key() {
    assert_eq!(
        audit_dart::dart_definition(),
        "class Audit { const Audit({required this.validate_,}); final bool validate_; factory \
         Audit.fromJson(Map<String, dynamic> json) => Audit(validate_: json['validate'] as \
         bool,); Map<String, dynamic> toJson() => { 'validate': validate_, }; List<String> \
         validate() => const []; }"
    );
    assert_eq!(
        serde_json::to_string(&Audit { validate: true }).unwrap(),
        r#"{"validate":true}"#,
        "the key serde writes, which the definition above reads and writes"
    );
}

/// A generic type's check is called once, with the value at whatever filling it holds. A unit
/// struct runs its check, and an alias of a model runs the model.
#[test]
fn a_types_own_check_is_called_on_the_whole_value() {
    let filled = crate_dart::dart_definition();
    assert!(
        filled.ends_with(
            "List<String> validate() => [ if (label.runes.length < 1) \"'label': too short: \
             minimum length is 1, got ${label.runes.length}\", ...withACode(this), ]; }"
        ),
        "got: {filled}"
    );
    let closed = closed_dock_dart::dart_definition();
    assert!(
        closed.ends_with("List<String> validate() => [ ...takingParcels(this), ]; }"),
        "got: {closed}"
    );
    let consignment = consignment_dart::dart_definition();
    assert!(
        consignment.contains(
            "dynamic toJson() => (value).toJson(); List<String> validate() => [ \
             ...value.validate(), ]; }"
        ),
        "got: {consignment}"
    );
}

/// `validate()` is written at the end of each class, after its codec, so every piece of the text
/// each type was written as before is still there, in its order.
#[test]
fn the_codec_of_each_type_is_the_text_it_was() {
    let written = definitions();
    let mut rest = written.as_str();
    for piece in CODEC.trim_end().split("; }") {
        let found = rest.find(piece);
        assert!(
            found.is_some(),
            "the codec no longer holds `{piece}` in: {written}"
        );
        rest = &rest[found.unwrap_or_default() + piece.len()..];
    }
}
