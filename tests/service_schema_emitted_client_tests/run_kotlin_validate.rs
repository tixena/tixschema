//! `validate()` of the emitted Kotlin types, compiled with `-Werror` and run. A row whose name is
//! a named field's, a list's index or a map's key is held to what Rust's `validate()` answers of
//! the same value. A row of a slot or of a variant's member is held to the text written here.
//!
//! A row's value is read by `{Type}Unjudged`, the read alone. The type's own serializer reads the
//! same payload in a run of its own, and is held to throwing what `validate()` answered.

#![cfg(feature = "kotlin")]

use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::runtime::{ran_kotlin, refused_by_kotlinc};

const KOTLIN_IMPORTS: &str = "import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// The functions the checks of the shared table name, written by hand as a consumer writes them.
const TABLE_CHECKS: &str = r#"
fun withoutDuplicates(value: List<String>): List<String> {
    val seen = mutableSetOf<String>()
    for (code in value) {
        if (!seen.add(code)) return listOf("duplicate item code $code")
    }
    return emptyList()
}

fun withinParcel(value: Parcel): List<String> =
    value.fragileItemCodes.withIndex()
        .filter { it.value !in value.itemCodeList }
        .map { "'fragileItemCodes.${it.index}': ${it.value} is outside of itemCodeList" }
"#;

/// The functions the checks of the shapes name.
const SHAPE_CHECKS: &str = r#"
fun sealedParcel(value: Parcel): List<String> =
    if (value.label == "open") listOf("an open parcel is not sealed") else emptyList()

fun unreserved(value: Code): List<String> =
    if (value.value == "NONE") listOf("a code is not NONE") else emptyList()

fun takingParcels(value: ClosedDock): List<String> =
    if (value == ClosedDock) listOf("a closed dock takes no parcel") else emptyList()

fun <CodeType> withACode(value: Crate<CodeType>): List<String> =
    if (value.codes.isEmpty()) listOf("'codes': a crate holds at least one code") else emptyList()

fun <IdType> withEntries(value: Casket<IdType>): List<String> =
    if (value.entries.isEmpty()) listOf("'entries': a casket holds at least one entry") else emptyList()

fun rated(value: Grade): List<String> =
    if (value == Grade.Unrated) listOf("a parcel is rated") else emptyList()

fun labelled(value: Freight): List<String> =
    if (value is FreightLoose && value.freightLabel.isEmpty()) {
        listOf("'loose.freightLabel': a loose freight is labelled")
    } else {
        emptyList()
    }

fun aReference(value: Reference): List<String> =
    if (value is ReferenceNumbered && value.value == 7u) listOf("a reference is not 7") else emptyList()
"#;

/// What every driver prints a row with: the words Rust's side of the comparison is written in.
const REPORT: &str = r#"
fun report(name: String, violations: List<String>) {
    if (violations.isEmpty()) {
        println("$name: passes")
    } else {
        println("$name: refused")
        violations.forEach { println("    $it") }
    }
}
"#;

/// What the driver of the reads prints a row with: whether the type's own serializer handed the
/// value back, and each message of what it threw.
const JUDGED: &str = r#"
fun judged(name: String, read: () -> Any?) {
    try {
        read()
        println("$name: read")
    } catch (refused: SerializationException) {
        println("$name: refused")
        (refused.message ?: "").split("; ").forEach { println("    $it") }
    }
}
"#;

/// The 13 lines of the shared table, for a parcel whose members are declared in the order of
/// their names.
const TABLE: &str = "\
case 1: passes
case 2: refused
    'label': too short: minimum length is 1, got 0
case 3: refused
    'itemCodeList': duplicate item code x
case 4: refused
    'fragileItemCodes.1': y is outside of itemCodeList
case 5: refused
    'itemCodeList': duplicate item code x
    'label': too short: minimum length is 1, got 0
    'fragileItemCodes.0': y is outside of itemCodeList
case 6: refused
    'parcel.fragileItemCodes.1': y is outside of itemCodeList
";

/// The read of a row's payload by the type's own serializer: the Kotlin expression, and the
/// messages it throws, none where it hands the value back.
struct Read {
    kotlin: String,
    refused: Vec<String>,
}

/// One row of a run: the Kotlin expression of the value, and the messages `validate()` is held
/// to for it.
struct Row {
    expected: Vec<String>,
    kotlin: String,
    name: &'static str,
    /// `None` for a value built in Kotlin.
    read: Option<Read>,
}

/// The parcel of the shared table.
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

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub parcel: Parcel,
}

/// One field per bound and per type a bound is measured on, a nullable of each kind among them.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gauge {
    #[model_schema_prop(minimum = 1, maximum = 9)]
    pub axle_count: u8,
    #[model_schema_prop(minLength = 1, maxLength = 2)]
    pub deck_codes: Vec<String>,
    #[model_schema_prop(minLength = 2, maxLength = 4, pattern = "^[a-z]+$")]
    pub dock_code: String,
    #[model_schema_prop(minimum = -5, maximum = 5)]
    pub hold_depth: i64,
    #[model_schema_prop(minimum = 0.5, maximum = 1.5)]
    pub load_ratio: f64,
    #[model_schema_prop(minLength = 1)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_codes: Option<Vec<String>>,
    #[model_schema_prop(maxLength = 2, pattern = "^[a-z]+$")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_label: Option<String>,
    #[model_schema_prop(maximum = 120)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_weight: Option<u32>,
    #[model_schema_prop(minimum = 0.25)]
    pub tilt: f32,
}

/// A string held to one character.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Stamp {
    #[model_schema_prop(maxLength = 1)]
    pub mark: String,
}

/// A plain enum a map is keyed by, each member under the name serde writes it.
#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Wing {
    North,
    South,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Base {
    #[model_schema_prop(minLength = 1)]
    pub base_name: String,
}

/// A model held in each place a holder reports one under.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Depot {
    pub by_number: BTreeMap<u32, Parcel>,
    pub by_wing: BTreeMap<Wing, Parcel>,
    pub deck_pair: (String, Parcel),
    #[model_schema_prop(check = ["sealedParcel"])]
    pub first_parcel: Parcel,
    pub lists_by_dock: BTreeMap<String, Vec<Parcel>>,
    pub parcel_grid: Vec<Vec<Parcel>>,
    #[model_schema_prop(minLength = 3)]
    pub parcel_list: Vec<Parcel>,
    pub parcels_by_dock: BTreeMap<String, Parcel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_parcel: Option<Parcel>,
}

/// A model written into the holder's own object.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Yard {
    #[serde(flatten)]
    pub base: Base,
    pub gate: String,
}

/// A map of models written into the holder's own object.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Docks {
    #[serde(flatten)]
    pub by_dock: BTreeMap<String, Parcel>,
}

/// A brand with each of its three bounds and a check.
#[model_schema(minLength = 2, maxLength = 4, pattern = "^[A-Z]+$", check = ["unreserved"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Code(pub String);

/// A brand over a list, whose two lengths count its items.
#[model_schema(no_display, minLength = 1, maxLength = 2)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Roster(pub Vec<String>);

/// A brand with a parameter, bounded at its declared default.
#[model_schema(minLength = 3, default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct UserId<IdType>(pub IdType);

/// A brand that declares no bound: a field typed with it writes its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Slug(pub String);

#[model_schema()]
pub type Title = String;

/// A brand over a brand, with a bound of its own.
#[model_schema(minLength = 2)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Badge(pub Slug);

/// A brand with a parameter whose declared default is a brand.
#[model_schema(minLength = 3, default_types(WrapType = Slug))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Wrapped<WrapType>(pub WrapType);

/// A brand's own bounds under the field that holds it, and a field's bound on a brand declared
/// above, on one declared below, on a brand over a brand, and on an alias of a string declared
/// above and below.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    #[model_schema_prop(minLength = 3)]
    pub badge: Badge,
    pub code: Code,
    #[model_schema_prop(minLength = 3)]
    pub late_slug: LateSlug,
    #[model_schema_prop(minLength = 2)]
    pub late_titled: LateTitle,
    #[model_schema_prop(minLength = 3, pattern = "^[a-z-]+$")]
    pub slug: Slug,
    #[model_schema_prop(minLength = 3)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_slug: Option<Slug>,
    #[model_schema_prop(minLength = 2)]
    pub titled: Title,
    pub user: UserId<String>,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct LateSlug(pub String);

#[model_schema()]
pub type LateTitle = String;

#[model_schema(check = ["takingParcels"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ClosedDock;

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tally(
    #[model_schema_prop(minLength = 1)] pub String,
    #[model_schema_prop(minimum = 1)] pub u32,
);

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Consignment(
    #[model_schema_prop(check = ["withoutDuplicates"])] pub Vec<String>,
    pub Parcel,
);

/// A struct of one slot, which serde writes as the slot's value.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Sealed(#[model_schema_prop(minLength = 1)] pub String);

/// An enum serde writes under its variant's name.
#[model_schema(check = ["labelled"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase", rename_all_fields = "camelCase")]
pub enum Freight {
    Boxed(Parcel),
    Held,
    Loose {
        #[model_schema_prop(minLength = 1)]
        freight_label: String,
    },
    #[serde(rename = "under-seal")]
    Sealed(#[model_schema_prop(minLength = 1)] String),
}

/// An enum under a tag and a content key.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "held", rename_all_fields = "camelCase")]
pub enum Haul {
    Boxed(Parcel),
    Loose {
        #[model_schema_prop(minLength = 1)]
        haul_label: String,
    },
    Sealed(#[model_schema_prop(minLength = 1)] String),
    Stacked {
        #[model_schema_prop(check = ["withoutDuplicates"])]
        stacked_codes: Vec<String>,
    },
}

/// An enum under a tag alone, with a string check of its own that reaches each string it holds.
#[model_schema(minLength = 2, pattern = "^[a-z]+$")]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all_fields = "camelCase")]
pub enum Lift {
    Boxed(Parcel),
    Loose {
        #[model_schema_prop(maxLength = 3, pattern = "^[a-m]+$")]
        lift_label: String,
        lift_weight: u32,
    },
    Tall {
        lift_label: String,
    },
}

#[model_schema(check = ["aReference"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Reference {
    Named {
        #[model_schema_prop(minLength = 1)]
        name: String,
    },
    Numbered(#[model_schema_prop(minimum = 1)] u32),
}

#[model_schema(check = ["rated"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Grade {
    Express,
    Unrated,
}

/// A type with a parameter, whose check is written once over every filling.
#[model_schema(default_types(CodeType = String), check = ["withACode"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Crate<CodeType> {
    pub codes: Vec<CodeType>,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

/// A brand over a list, held by the members below.
#[model_schema(no_display)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Slogans(pub Vec<String>);

/// An alias of a list.
#[model_schema()]
pub type Mottos = Vec<String>;

/// A brand over a brand over a list, with a length of its own.
#[model_schema(no_display, minLength = 1)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct SomeSlogans(pub Slogans);

/// A generic brand whose declared default is a list.
#[model_schema(no_display, minLength = 1, default_types(T = Vec<String>))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Carried<T>(pub T);

/// Members typed with the names above, each under a length of its own.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Poster {
    pub carried: Carried<Vec<String>>,
    #[model_schema_prop(minLength = 1)]
    pub mottos: Mottos,
    #[model_schema_prop(minLength = 1, maxLength = 2)]
    pub slogans: Slogans,
    #[model_schema_prop(maxLength = 2)]
    pub some: SomeSlogans,
}

/// A type with a parameter, held by a member of another that reaches the same parameter.
#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Casket<IdType> {
    pub entries: Vec<String>,
    pub owner_id: IdType,
}

/// A type with a parameter whose member holds a generic type at that parameter, under a check.
#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Archive<IdType> {
    #[model_schema_prop(check = ["withEntries"])]
    pub casket: Casket<IdType>,
}

/// A member named after the method, one named after the list the method fills, and a member
/// typed with a type declared below.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Probe {
    pub later: Later,
    pub numbered: Crate<u32>,
    #[model_schema_prop(minLength = 1)]
    pub validate: String,
    #[model_schema_prop(minimum = 1)]
    pub violations: u32,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Later {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

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

fn sealed_parcel(value: &Parcel) -> Result<(), Vec<String>> {
    if value.label == "open" {
        return Err(vec!["an open parcel is not sealed".to_owned()]);
    }
    Ok(())
}

fn unreserved(value: &Code) -> Result<(), Vec<String>> {
    if value.0 == "NONE" {
        return Err(vec!["a code is not NONE".to_owned()]);
    }
    Ok(())
}

fn taking_parcels(_value: &ClosedDock) -> Result<(), Vec<String>> {
    Err(vec!["a closed dock takes no parcel".to_owned()])
}

fn with_a_code<CodeType>(value: &Crate<CodeType>) -> Result<(), Vec<String>> {
    if value.codes.is_empty() {
        return Err(vec!["'codes': a crate holds at least one code".to_owned()]);
    }
    Ok(())
}

fn with_entries<IdType>(value: &Casket<IdType>) -> Result<(), Vec<String>> {
    if value.entries.is_empty() {
        return Err(vec![
            "'entries': a casket holds at least one entry".to_owned(),
        ]);
    }
    Ok(())
}

fn rated(value: &Grade) -> Result<(), Vec<String>> {
    if matches!(value, Grade::Unrated) {
        return Err(vec!["a parcel is rated".to_owned()]);
    }
    Ok(())
}

fn labelled(value: &Freight) -> Result<(), Vec<String>> {
    if let Freight::Loose { freight_label } = value
        && freight_label.is_empty()
    {
        return Err(vec![
            "'loose.freightLabel': a loose freight is labelled".to_owned(),
        ]);
    }
    Ok(())
}

fn a_reference(value: &Reference) -> Result<(), Vec<String>> {
    if matches!(value, Reference::Numbered(7)) {
        return Err(vec!["a reference is not 7".to_owned()]);
    }
    Ok(())
}

fn texts(written: &[&str]) -> Vec<String> {
    written.iter().map(|&text| text.to_owned()).collect()
}

fn parcel(label: &str, item_code_list: &[&str], fragile_item_codes: &[&str]) -> Parcel {
    Parcel {
        fragile_item_codes: texts(fragile_item_codes),
        item_code_list: texts(item_code_list),
        label: label.to_owned(),
    }
}

/// A parcel that breaks its one bound.
fn unlabelled() -> Parcel {
    parcel("", &["x"], &["x"])
}

/// `text` as a Kotlin string writes it: a backslash, a quote and a `$` each behind a backslash.
fn kotlin_string(text: &str) -> String {
    format!(
        "\"{}\"",
        text.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('$', "\\$")
    )
}

/// The serializer that reads `read_as` with no judgement: `{Type}Unjudged`, over the serializer
/// of its argument where the type is written with one.
fn unjudged(read_as: &str) -> String {
    read_as.split_once('<').map_or_else(
        || format!("{read_as}Unjudged"),
        |(named, argument)| {
            format!(
                "{named}Unjudged(serializer<{}>())",
                argument.strip_suffix('>').unwrap_or(argument)
            )
        },
    )
}

/// A row over the payload serde writes for `value`: the value `{Type}Unjudged` reads from it, and
/// the read of it by the type's own serializer, held to throwing what `validate()` answers.
fn read_row<T>(name: &'static str, read_as: &str, value: &T, expected: Vec<String>) -> Row
where
    T: Serialize,
{
    let payload = kotlin_string(&serde_json::to_string(value).unwrap());
    let read = Read {
        kotlin: format!("Json.decodeFromString(serializer<{read_as}>(), {payload})"),
        refused: expected.clone(),
    };
    Row {
        expected,
        kotlin: format!("Json.decodeFromString({}, {payload})", unjudged(read_as)),
        name,
        read: Some(read),
    }
}

/// A row held to what Rust's `validate()` answered of `value`.
fn row<T>(name: &'static str, read_as: &str, value: &T, answered: Result<(), Vec<String>>) -> Row
where
    T: Serialize,
{
    read_row(name, read_as, value, answered.err().unwrap_or_default())
}

/// A row of a slot or of a variant's member, held to the names written here.
fn pinned<T>(name: &'static str, read_as: &str, value: &T, expected: &[&str]) -> Row
where
    T: Serialize,
{
    read_row(name, read_as, value, texts(expected))
}

/// A row whose value is built in Kotlin, where the emitted serializer does not read what serde
/// writes for the shape, or reads no variant of an untagged enum from it.
fn built(name: &'static str, kotlin: &str, expected: &[&str]) -> Row {
    Row {
        expected: texts(expected),
        kotlin: kotlin.to_owned(),
        name,
        read: None,
    }
}

/// What the driver prints for `rows` where each `validate()` answers what its row expects.
fn report(rows: &[Row]) -> String {
    let mut text = String::new();
    for held in rows {
        if held.expected.is_empty() {
            let _ = writeln!(text, "{}: passes", held.name);
        } else {
            let _ = writeln!(text, "{}: refused", held.name);
            for line in &held.expected {
                let _ = writeln!(text, "    {line}");
            }
        }
    }
    text
}

fn driver(rows: &[Row]) -> String {
    let mut main = String::from("fun main() {\n");
    for held in rows {
        let _ = writeln!(
            main,
            "    report({}, {}.validate())",
            kotlin_string(held.name),
            held.kotlin
        );
    }
    main.push_str("}\n");
    main
}

fn module(definitions: Vec<String>, written_by_hand: &[&str], rows: &[Row]) -> String {
    let mut parts = vec![KOTLIN_IMPORTS.to_owned()];
    parts.extend(definitions);
    parts.extend(written_by_hand.iter().map(|&part| part.to_owned()));
    parts.push(REPORT.to_owned());
    parts.push(driver(rows));
    parts.join("\n\n")
}

/// What the driver of the reads prints for `rows` where each read answers what its row expects.
fn read_report(rows: &[Row]) -> String {
    let mut text = String::new();
    for (held, read) in rows
        .iter()
        .filter_map(|held| held.read.as_ref().map(|read| (held, read)))
    {
        if read.refused.is_empty() {
            let _ = writeln!(text, "{}: read", held.name);
        } else {
            let _ = writeln!(text, "{}: refused", held.name);
            for line in &read.refused {
                let _ = writeln!(text, "    {line}");
            }
        }
    }
    text
}

/// The module that reads each row's payload with the type's own serializer.
fn read_module(definitions: Vec<String>, written_by_hand: &[&str], rows: &[Row]) -> String {
    let mut main = String::from("fun main() {\n");
    for (held, read) in rows
        .iter()
        .filter_map(|held| held.read.as_ref().map(|read| (held, read)))
    {
        let _ = writeln!(
            main,
            "    judged({}) {{ {} }}",
            kotlin_string(held.name),
            read.kotlin
        );
    }
    main.push_str("}\n");
    let mut parts = vec![KOTLIN_IMPORTS.to_owned()];
    parts.extend(definitions);
    parts.extend(written_by_hand.iter().map(|&part| part.to_owned()));
    parts.push(JUDGED.to_owned());
    parts.push(main);
    parts.join("\n\n")
}

fn table_definitions() -> Vec<String> {
    vec![
        parcel_kotlin::kotlin_definition(),
        manifest_kotlin::kotlin_definition(),
    ]
}

/// The six cases of the shared table, each held to what Rust's `validate()` answers.
fn table_rows() -> Vec<Row> {
    let parcels = [
        ("case 1", parcel("A", &["x"], &["x"])),
        ("case 2", parcel("", &["x"], &["x"])),
        ("case 3", parcel("A", &["x", "x"], &["x"])),
        ("case 4", parcel("A", &["x"], &["x", "y"])),
        ("case 5", parcel("", &["x", "x"], &["y"])),
    ];
    let mut rows: Vec<Row> = parcels
        .iter()
        .map(|(name, held)| row(name, "Parcel", held, held.validate()))
        .collect();
    let manifest = Manifest {
        parcel: parcel("A", &["x"], &["x", "y"]),
    };
    rows.push(row("case 6", "Manifest", &manifest, manifest.validate()));
    rows
}

fn gauge() -> Gauge {
    Gauge {
        axle_count: 2,
        deck_codes: texts(&["a"]),
        dock_code: "ab".to_owned(),
        hold_depth: 0,
        load_ratio: 1.0,
        spare_codes: None,
        spare_label: None,
        spare_weight: None,
        tilt: 0.5,
    }
}

/// Each bound broken from below and from above, bare and under a nullable.
fn bound_rows() -> Vec<Row> {
    let low = Gauge {
        axle_count: 0,
        deck_codes: Vec::new(),
        dock_code: "A".to_owned(),
        hold_depth: -6,
        load_ratio: 0.25,
        spare_codes: Some(Vec::new()),
        spare_label: Some("A1".to_owned()),
        spare_weight: Some(120),
        tilt: 0.125,
    };
    let high = Gauge {
        axle_count: 10,
        deck_codes: texts(&["a", "b", "c"]),
        dock_code: "abcde".to_owned(),
        hold_depth: 6,
        load_ratio: 150.0,
        spare_codes: Some(texts(&["a"])),
        spare_label: Some("abc".to_owned()),
        spare_weight: Some(121),
        tilt: 0.5,
    };
    let far = Gauge {
        load_ratio: 25_000_000.0,
        ..gauge()
    };
    let held = gauge();
    vec![
        row("Gauge, held", "Gauge", &held, held.validate()),
        row(
            "Gauge, each bound from below",
            "Gauge",
            &low,
            low.validate(),
        ),
        row(
            "Gauge, each bound from above",
            "Gauge",
            &high,
            high.validate(),
        ),
        row(
            "Gauge, a ratio of eight digits",
            "Gauge",
            &far,
            far.validate(),
        ),
    ]
}

/// A length is counted in Unicode code points: each of these marks is one.
fn code_point_rows() -> Vec<Row> {
    let stamp = |mark: &str| Stamp {
        mark: mark.to_owned(),
    };
    vec![
        pinned("Stamp, an accented letter", "Stamp", &stamp("\u{e9}"), &[]),
        pinned(
            "Stamp, a character outside the Basic Multilingual Plane",
            "Stamp",
            &stamp("\u{1f600}"),
            &[],
        ),
        pinned(
            "Stamp, two accented letters",
            "Stamp",
            &stamp("\u{e9}\u{e9}"),
            &["'mark': too long: maximum length is 1, got 2"],
        ),
    ]
}

fn depot() -> Depot {
    Depot {
        by_number: BTreeMap::from([(7, unlabelled())]),
        by_wing: BTreeMap::from([(Wing::North, unlabelled())]),
        deck_pair: ("deck".to_owned(), unlabelled()),
        first_parcel: parcel("open", &["x"], &["x", "y"]),
        lists_by_dock: BTreeMap::from([("north".to_owned(), vec![unlabelled()])]),
        parcel_grid: vec![vec![parcel("A", &["x"], &["x"]), unlabelled()]],
        parcel_list: vec![parcel("A", &["x"], &["x"]), unlabelled()],
        parcels_by_dock: BTreeMap::from([("north".to_owned(), unlabelled())]),
        spare_parcel: Some(unlabelled()),
    }
}

/// A model held in a field, a nullable, a list, a map and a tuple, and written into its holder.
fn holder_rows() -> Vec<Row> {
    let full = depot();
    let empty = Depot {
        by_number: BTreeMap::new(),
        by_wing: BTreeMap::new(),
        deck_pair: ("deck".to_owned(), parcel("A", &["x"], &["x"])),
        first_parcel: parcel("A", &["x"], &["x"]),
        lists_by_dock: BTreeMap::new(),
        parcel_grid: Vec::new(),
        parcel_list: vec![parcel("A", &["x"], &["x"]); 3],
        parcels_by_dock: BTreeMap::new(),
        spare_parcel: None,
    };
    let yard = Yard {
        base: Base {
            base_name: String::new(),
        },
        gate: "east".to_owned(),
    };
    let docks = Docks {
        by_dock: BTreeMap::from([("north".to_owned(), unlabelled())]),
    };
    vec![
        row(
            "Depot, a broken parcel in each place",
            "Depot",
            &full,
            full.validate(),
        ),
        row("Depot, held", "Depot", &empty, empty.validate()),
        row("Yard", "Yard", &yard, yard.validate()),
        row("Docks", "Docks", &docks, docks.validate()),
    ]
}

/// A brand, a brand over a list, a brand with a parameter, and a field that holds or bounds one.
fn brand_rows() -> Vec<Row> {
    let code = |text: &str| Code(text.to_owned());
    let badge = |text: &str| Badge(Slug(text.to_owned()));
    let rows = [
        ("Code, short and in lower case", code("a")),
        ("Code, too long", code("ABCDE")),
        ("Code, reserved", code("NONE")),
        ("Code, held", code("AB")),
    ];
    let mut listed: Vec<Row> = rows
        .iter()
        .map(|(name, held)| row(name, "Code", held, held.validate()))
        .collect();
    let (none, three) = (Roster(Vec::new()), Roster(texts(&["a", "b", "c"])));
    let (named, numbered) = (UserId("ab".to_owned()), UserId(7_u32));
    let (short, wrapped, counted) = (badge("a"), Wrapped(Slug("ab".to_owned())), Wrapped(7_u32));
    listed.extend([
        row("Roster, empty", "Roster", &none, none.validate()),
        row("Roster, three", "Roster", &three, three.validate()),
        row("UserId<String>", "UserId<String>", &named, named.validate()),
        row(
            "UserId<UInt>",
            "UserId<UInt>",
            &numbered,
            numbered.validate(),
        ),
        row("Badge, short", "Badge", &short, short.validate()),
        row(
            "Wrapped<Slug>",
            "Wrapped<Slug>",
            &wrapped,
            wrapped.validate(),
        ),
        row(
            "Wrapped<UInt>",
            "Wrapped<UInt>",
            &counted,
            counted.validate(),
        ),
    ]);
    listed.extend(entry_rows());
    listed
}

/// A field that holds a brand, and one that bounds a brand declared above, a brand declared
/// below, a brand over a brand, and an alias of a string declared above and below.
fn entry_rows() -> Vec<Row> {
    let broken = Entry {
        badge: Badge(Slug("ab".to_owned())),
        code: Code("a".to_owned()),
        late_slug: LateSlug("ab".to_owned()),
        late_titled: "t".to_owned(),
        slug: Slug("A".to_owned()),
        spare_slug: Some(Slug("ab".to_owned())),
        titled: "t".to_owned(),
        user: UserId("ab".to_owned()),
    };
    let kept = Entry {
        badge: Badge(Slug("abc".to_owned())),
        code: Code("AB".to_owned()),
        late_slug: LateSlug("abc".to_owned()),
        late_titled: "ti".to_owned(),
        slug: Slug("a-b".to_owned()),
        spare_slug: None,
        titled: "ti".to_owned(),
        user: UserId("abc".to_owned()),
    };
    vec![
        row(
            "Entry, each bound broken",
            "Entry",
            &broken,
            broken.validate(),
        ),
        row("Entry, held", "Entry", &kept, kept.validate()),
    ]
}

/// The rows of a type that holds another at its own parameter under a check, and of members typed
/// with the name of a list or of a text under a length.
fn counted_name_rows() -> Vec<Row> {
    let archived = |entries: &[&str]| Archive::<String> {
        casket: Casket {
            entries: texts(entries),
            owner_id: "o".to_owned(),
        },
    };
    let archived_by_number = Archive::<u32> {
        casket: Casket {
            entries: Vec::new(),
            owner_id: 7,
        },
    };
    let (unfilled, filled) = (archived(&[]), archived(&["a"]));
    let poster = |slogans: usize, mottos: usize, some: usize, carried: usize| Poster {
        slogans: Slogans(texts(&vec!["w"; slogans])),
        mottos: texts(&vec!["w"; mottos]),
        some: SomeSlogans(Slogans(texts(&vec!["w"; some]))),
        carried: Carried(texts(&vec!["w"; carried])),
    };
    let (poster_kept, poster_empty, poster_wide) =
        (poster(1, 1, 1, 1), poster(0, 0, 0, 0), poster(3, 1, 3, 1));
    vec![
        row("Poster", "Poster", &poster_kept, poster_kept.validate()),
        row(
            "Poster, no item",
            "Poster",
            &poster_empty,
            poster_empty.validate(),
        ),
        row(
            "Poster, too many items",
            "Poster",
            &poster_wide,
            poster_wide.validate(),
        ),
        row(
            "Archive<String>, no entry",
            "Archive<String>",
            &unfilled,
            unfilled.validate(),
        ),
        row(
            "Archive<String>",
            "Archive<String>",
            &filled,
            filled.validate(),
        ),
        row(
            "Archive<UInt>, no entry",
            "Archive<UInt>",
            &archived_by_number,
            archived_by_number.validate(),
        ),
    ]
}

/// A unit struct, a plain enum, a type with a parameter, and the members whose names the method
/// writes itself.
fn struct_rows() -> Vec<Row> {
    let crate_of = |label: &str| Crate::<String> {
        codes: Vec::new(),
        label: label.to_owned(),
    };
    let numbered = Crate::<u32> {
        codes: Vec::new(),
        label: String::new(),
    };
    let named = crate_of("");
    let probe = Probe {
        later: Later {
            label: String::new(),
        },
        numbered: numbered.clone(),
        validate: String::new(),
        violations: 0,
    };
    let labelled = Probe {
        later: Later {
            label: "a".to_owned(),
        },
        ..probe.clone()
    };
    vec![
        row(
            "ClosedDock",
            "ClosedDock",
            &ClosedDock,
            ClosedDock.validate(),
        ),
        row(
            "Grade, unrated",
            "Grade",
            &Grade::Unrated,
            Grade::Unrated.validate(),
        ),
        row(
            "Grade, express",
            "Grade",
            &Grade::Express,
            Grade::Express.validate(),
        ),
        row("Crate<String>", "Crate<String>", &named, named.validate()),
        row("Crate<UInt>", "Crate<UInt>", &numbered, numbered.validate()),
        // `Later` is declared below `Probe`, which reads it by its own serializer: a probe that
        // holds one that breaks its bound is built, and its read is another run's.
        Row {
            expected: probe.validate().err().unwrap_or_default(),
            kotlin: "Probe(Later(\"\"), Crate<UInt>(emptyList(), \"\"), \"\", 0u)".to_owned(),
            name: "Probe",
            read: None,
        },
        row(
            "Probe, the member typed with a type declared below held",
            "Probe",
            &labelled,
            labelled.validate(),
        ),
    ]
}

/// The slots of a tuple struct, each under the step serde writes on the way to it.
fn slot_rows() -> Vec<Row> {
    vec![
        pinned(
            "Tally",
            "Tally",
            &Tally(String::new(), 0),
            &[
                "'0': too short: minimum length is 1, got 0",
                "'1': too small: minimum is 1, got 0",
            ],
        ),
        pinned(
            "Consignment",
            "Consignment",
            &Consignment(texts(&["a", "a"]), unlabelled()),
            &[
                "'0': duplicate item code a",
                "'1.label': too short: minimum length is 1, got 0",
            ],
        ),
        built(
            "Sealed",
            "Sealed(\"\")",
            &["too short: minimum length is 1, got 0"],
        ),
    ]
}

/// The members of a variant under serde's own tagging and under a tag and a content key.
fn wrapped_variant_rows() -> Vec<Row> {
    let short = "too short: minimum length is 1, got 0";
    vec![
        pinned(
            "Freight, a model in the one slot",
            "Freight",
            &Freight::Boxed(unlabelled()),
            &[&format!("'boxed.label': {short}")],
        ),
        pinned(
            "Freight, a named member",
            "Freight",
            &Freight::Loose {
                freight_label: String::new(),
            },
            &[
                &format!("'loose.freightLabel': {short}"),
                "'loose.freightLabel': a loose freight is labelled",
            ],
        ),
        pinned(
            "Freight, the one slot",
            "Freight",
            &Freight::Sealed(String::new()),
            &[&format!("'under-seal': {short}")],
        ),
        pinned("Freight, no member", "Freight", &Freight::Held, &[]),
        pinned(
            "Haul, a model in the one slot",
            "Haul",
            &Haul::Boxed(unlabelled()),
            &[&format!("'held.label': {short}")],
        ),
        pinned(
            "Haul, a named member",
            "Haul",
            &Haul::Loose {
                haul_label: String::new(),
            },
            &[&format!("'held.haulLabel': {short}")],
        ),
        pinned(
            "Haul, the one slot",
            "Haul",
            &Haul::Sealed(String::new()),
            &[&format!("'held': {short}")],
        ),
        pinned(
            "Haul, a member's check",
            "Haul",
            &Haul::Stacked {
                stacked_codes: texts(&["a", "a"]),
            },
            &["'held.stackedCodes': duplicate item code a"],
        ),
    ]
}

/// The members of a variant under a tag alone and under no tag.
fn bare_variant_rows() -> Vec<Row> {
    vec![
        pinned(
            "Lift, a named member held to its own bound and to the enum's",
            "Lift",
            &Lift::Loose {
                lift_label: "n1".to_owned(),
                lift_weight: 1,
            },
            &[
                "'liftLabel': does not match pattern '^[a-m]+$'",
                "'liftLabel': does not match pattern '^[a-z]+$'",
            ],
        ),
        pinned(
            "Lift, a named member the enum alone bounds",
            "Lift",
            &Lift::Tall {
                lift_label: "a".to_owned(),
            },
            &["'liftLabel': too short: minimum length is 2, got 1"],
        ),
        built(
            "Lift, a model in the one slot",
            "LiftBoxed(Parcel(listOf(\"x\"), listOf(\"x\"), \"\"))",
            &["'label': too short: minimum length is 1, got 0"],
        ),
        built(
            "Reference, a named member",
            "ReferenceNamed(\"\")",
            &["'name': too short: minimum length is 1, got 0"],
        ),
        built(
            "Reference, the one slot",
            "ReferenceNumbered(0u)",
            &["too small: minimum is 1, got 0"],
        ),
        pinned(
            "Reference, the enum's check",
            "Reference",
            &Reference::Numbered(7),
            &["a reference is not 7"],
        ),
    ]
}

/// The rows of the shared table, then those of every shape.
fn every_row() -> Vec<Row> {
    let mut rows = table_rows();
    rows.extend(bound_rows());
    rows.extend(code_point_rows());
    rows.extend(holder_rows());
    rows.extend(brand_rows());
    rows.extend(struct_rows());
    rows.extend(counted_name_rows());
    rows.extend(slot_rows());
    rows.extend(wrapped_variant_rows());
    rows.extend(bare_variant_rows());
    rows
}

fn shape_definitions() -> Vec<String> {
    let mut definitions = table_definitions();
    definitions.extend([
        gauge_kotlin::kotlin_definition(),
        stamp_kotlin::kotlin_definition(),
        wing_kotlin::kotlin_definition(),
        base_kotlin::kotlin_definition(),
        depot_kotlin::kotlin_definition(),
        yard_kotlin::kotlin_definition(),
        docks_kotlin::kotlin_definition(),
        code_kotlin::kotlin_definition(),
        roster_kotlin::kotlin_definition(),
        user_id_kotlin::kotlin_definition(),
        slug_kotlin::kotlin_definition(),
        title_kotlin::kotlin_definition(),
        badge_kotlin::kotlin_definition(),
        wrapped_kotlin::kotlin_definition(),
        entry_kotlin::kotlin_definition(),
        late_slug_kotlin::kotlin_definition(),
        late_title_kotlin::kotlin_definition(),
        closed_dock_kotlin::kotlin_definition(),
        tally_kotlin::kotlin_definition(),
        consignment_kotlin::kotlin_definition(),
        sealed_kotlin::kotlin_definition(),
        freight_kotlin::kotlin_definition(),
        haul_kotlin::kotlin_definition(),
        lift_kotlin::kotlin_definition(),
        reference_kotlin::kotlin_definition(),
        grade_kotlin::kotlin_definition(),
        crate_kotlin::kotlin_definition(),
        slogans_kotlin::kotlin_definition(),
        mottos_kotlin::kotlin_definition(),
        some_slogans_kotlin::kotlin_definition(),
        carried_kotlin::kotlin_definition(),
        poster_kotlin::kotlin_definition(),
        casket_kotlin::kotlin_definition(),
        archive_kotlin::kotlin_definition(),
        probe_kotlin::kotlin_definition(),
        later_kotlin::kotlin_definition(),
    ]);
    definitions
}

#[test]
fn the_shared_table_prints_the_lines_rust_prints() {
    let rows = table_rows();
    assert_eq!(report(&rows), TABLE);
    let Some(printed) = ran_kotlin(&module(table_definitions(), &[TABLE_CHECKS], &rows)) else {
        return;
    };
    assert_eq!(printed, TABLE);
}

#[test]
fn every_shape_runs_its_bounds_and_its_checks() {
    let mut rows = bound_rows();
    rows.extend(code_point_rows());
    rows.extend(holder_rows());
    rows.extend(brand_rows());
    rows.extend(struct_rows());
    rows.extend(counted_name_rows());
    rows.extend(slot_rows());
    rows.extend(wrapped_variant_rows());
    rows.extend(bare_variant_rows());
    let Some(printed) = ran_kotlin(&module(
        shape_definitions(),
        &[TABLE_CHECKS, SHAPE_CHECKS],
        &rows,
    )) else {
        return;
    };
    assert_eq!(printed, report(&rows));
}

/// The type's own serializer reads the payload of each row and throws what `validate()` answers
/// of the value, each message at its full place, joined by `; `.
#[test]
fn the_read_of_every_shape_throws_what_validate_answers() {
    let rows = every_row();
    let Some(printed) = ran_kotlin(&read_module(
        shape_definitions(),
        &[TABLE_CHECKS, SHAPE_CHECKS],
        &rows,
    )) else {
        return;
    };
    assert_eq!(printed, read_report(&rows));
}

/// serde reads no variant from a payload that breaks the bound of each, so the two rows of such a
/// value are built in Kotlin.
#[test]
fn serde_reads_no_variant_from_a_payload_that_breaks_the_bound_of_each() {
    serde_json::from_str::<Reference>("{\"name\":\"\"}").unwrap_err();
    serde_json::from_str::<Reference>("0").unwrap_err();
}

/// What Rust answers for the rows held to it, written out: a broken bound in each place of a
/// holder, and each bound in the sentence `src/bound_message.rs` states.
#[test]
fn rust_answers_each_place_and_each_bound_in_these_words() {
    assert_eq!(
        depot().validate().unwrap_err(),
        [
            "'byNumber.7.label': too short: minimum length is 1, got 0",
            "'byWing.north.label': too short: minimum length is 1, got 0",
            "'deckPair.1.label': too short: minimum length is 1, got 0",
            "'firstParcel.fragileItemCodes.1': y is outside of itemCodeList",
            "'firstParcel': an open parcel is not sealed",
            "'listsByDock.north.0.label': too short: minimum length is 1, got 0",
            "'parcelGrid.0.1.label': too short: minimum length is 1, got 0",
            "'parcelList.1.label': too short: minimum length is 1, got 0",
            "'parcelList': too short: minimum length is 3, got 2",
            "'parcelsByDock.north.label': too short: minimum length is 1, got 0",
            "'spareParcel.label': too short: minimum length is 1, got 0",
        ]
    );
    let high = bound_rows().remove(2);
    assert_eq!(
        high.expected,
        [
            "'axleCount': too large: maximum is 9, got 10",
            "'deckCodes': too long: maximum length is 2, got 3",
            "'dockCode': too long: maximum length is 4, got 5",
            "'holdDepth': too large: maximum is 5, got 6",
            "'loadRatio': too large: maximum is 1.5, got 150",
            "'spareLabel': too long: maximum length is 2, got 3",
            "'spareWeight': too large: maximum is 120, got 121",
        ]
    );
}

/// A string bound on a field typed with a declared type is run on the string that type holds:
/// a brand declared above the field's type, one declared below it, a brand over a brand, and an
/// alias of a string declared above and below.
#[test]
fn rust_answers_a_bound_on_each_declared_type_in_these_words() {
    assert_eq!(
        entry_rows().remove(0).expected,
        [
            "'badge': too short: minimum length is 3, got 2",
            "'code': too short: minimum length is 2, got 1",
            "'code': does not match pattern '^[A-Z]+$'",
            "'lateSlug': too short: minimum length is 3, got 2",
            "'lateTitled': too short: minimum length is 2, got 1",
            "'slug': too short: minimum length is 3, got 1",
            "'slug': does not match pattern '^[a-z-]+$'",
            "'spareSlug': too short: minimum length is 3, got 2",
            "'titled': too short: minimum length is 2, got 1",
            "'user': too short: minimum length is 3, got 2",
        ]
    );
}

/// The two shapes a row builds in Kotlin: the emitted serializer reads an array for the first
/// and a `value` key for the second, where serde writes these.
#[test]
fn serde_writes_the_two_shapes_a_row_builds_in_kotlin() {
    assert_eq!(
        serde_json::to_value(Sealed("a".to_owned())).unwrap(),
        serde_json::json!("a")
    );
    assert_eq!(
        serde_json::to_value(Lift::Boxed(parcel("A", &["x"], &["x"]))).unwrap(),
        serde_json::json!({
            "kind": "Boxed", "fragileItemCodes": ["x"], "itemCodeList": ["x"], "label": "A"
        })
    );
}

/// The README's Kotlin section, held to the functions this file compiles and to the statements
/// the emitter writes for the holders it runs.
#[test]
fn the_readme_shows_the_functions_and_the_statements_that_compile() {
    let readme = include_str!("../../README.md");
    assert!(
        readme.contains(&format!("```kotlin{TABLE_CHECKS}```")),
        "the README no longer shows the functions the run compiles"
    );
    let (depot, entry) = (
        depot_kotlin::kotlin_definition(),
        entry_kotlin::kotlin_definition(),
    );
    for (emitted, statement) in [
        (
            &depot,
            "violations.addAll(firstParcel.validate().map { kotlinSchemaUnder(\"firstParcel\", \
             it) })",
        ),
        (
            &depot,
            "parcelList.forEachIndexed { index, held -> violations.addAll(held.validate().map { \
             kotlinSchemaUnder(\"parcelList.$index\", it) }) }",
        ),
        (
            &depot,
            "spareParcel?.let { held -> violations.addAll(held.validate().map { \
             kotlinSchemaUnder(\"spareParcel\", it) }) }",
        ),
        (
            &depot,
            "for ((key, held) in parcelsByDock) { violations.addAll(held.validate().map { \
             kotlinSchemaUnder(\"parcelsByDock.$key\", it) }) }",
        ),
        (
            &entry,
            "if (slug.kotlinSchemaText().codePointCount(0, slug.kotlinSchemaText().length) < 3) \
             violations.add(\"'slug': too short: minimum length is 3, got \
             ${slug.kotlinSchemaText().codePointCount(0, slug.kotlinSchemaText().length)}\")",
        ),
        (
            &entry,
            "private fun String.kotlinSchemaText(): String = this",
        ),
    ] {
        assert!(emitted.contains(statement), "not emitted: {statement}");
        assert!(readme.contains(statement), "not in the README: {statement}");
    }
    let example = include_str!("../../src/check_example.md");
    for line in [
        "'label': too short: minimum length is 1, got 0",
        "'itemCodeList': duplicate item code x",
        "'fragileItemCodes.0': y is outside of itemCodeList",
        "'parcel.fragileItemCodes.1': y is outside of itemCodeList",
    ] {
        assert!(example.contains(&format!("\"{line}\"")), "{line}");
        assert!(readme.contains(&format!("\n{line}\n")), "{line}");
    }
}

/// A check with no Kotlin function is the compiler's to refuse, at the call.
#[test]
fn a_check_with_no_kotlin_function_does_not_compile() {
    let Some(refused) = refused_by_kotlinc(&module(table_definitions(), &[], &table_rows())) else {
        return;
    };
    for named in ["withoutDuplicates", "withinParcel"] {
        assert!(
            refused.contains(&format!("error: unresolved reference '{named}'.")),
            "no refusal of `{named}` in:\n{refused}"
        );
    }
    assert!(
        include_str!("../../README.md")
            .contains("```text\nerror: unresolved reference 'withinParcel'.\n```"),
        "the README no longer shows what the compiler says"
    );
}
