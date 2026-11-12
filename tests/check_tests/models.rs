//! The models the `check` tests run, and the function each check names: the parcel of the shared
//! table, and one type per shape a check is written on. Each case is a value built here, with the
//! payload serde writes for it and what `validate()` answers of it.

/// The places serde writes a member at, where a bound is held and a holder runs what it holds.
#[cfg(feature = "serde")]
#[path = "places.rs"]
pub mod places;

use core::fmt::Write as _;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// What every side prints for the shapes: the Rust `validate()` and the Zod schema alike.
pub const SHAPES: &str = "\
Route, no stop: refused
    'stops': a route makes at least one stop
Route: passes
Stack, a code twice: refused
    '0': duplicate item code a
Stack, over its limit: refused
    '0': a stack holds at most 1 codes
Stack: passes
Pallet, three codes: refused
    'codes.2': c is one code too many
Pallet: passes
ClosedDock: refused
    a closed dock takes no parcel
Shipment, three stops by air: refused
    'stops': an air shipment makes at most 2 stops
Shipment, a stop twice: refused
    'stops': duplicate item code a
Shipment, three stops by ground: passes
Transfer, a road with no stop: refused
    'Road.stops': a road transfer makes a stop
Transfer, a rail with no line: refused
    'Rail': a rail transfer names its line
Transfer, held: passes
Handover, signed by no one: refused
    'detail.by': a handover is signed by someone
Handover, left: passes
Reference, numbered zero: refused
    a reference is not zero
Reference, with no name: refused
    'name': a reference has a name
Reference: passes
Grade, unrated: refused
    a parcel is rated
Grade, express: passes
Grade, standard: passes
Code, in lower case: refused
    a code is upper case
Code: passes
Crate<String>, empty: refused
    'codes': a crate holds at least one code
Crate<String>: passes
Crate<u32>, empty: refused
    'codes': a crate holds at least one code
Crate<u32>: passes
Batch<String>, empty: refused
    a batch holds at least one code
Batch<u32>, empty: refused
    a batch holds at least one code
Batch<u32>: passes
Ledger<String>, no entry: refused
    'entries': a ledger holds at least one entry
Ledger<String>: passes
Ledger<u32>: passes
Archive<String>, no entry: refused
    'ledger.entries': a ledger holds at least one entry
    'ledger.entries': a ledger holds at least one entry
Archive<String>: passes
Archive<u32>, no entry: refused
    'ledger.entries': a ledger holds at least one entry
    'ledger.entries': a ledger holds at least one entry
Archive<u32>: passes
";

/// The 13 lines of the shared table, in a build that holds `label` to its bound and runs a held
/// type's `validate()`. A member's lines are in the order the parcel declares its members, which
/// is the order of its names.
#[cfg(feature = "serde")]
pub const TABLE: &str = "\
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

/// One case: the payload serde writes for the value built for it, and what `validate()` answers
/// of that value.
pub struct Case {
    pub answered: Result<(), Vec<String>>,
    pub name: &'static str,
    pub payload: String,
}

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

/// A struct with a check and no bound.
#[model_schema(check = ["withARoute"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Route {
    pub stops: Vec<String>,
}

/// A tuple struct: a check on its first slot, and one on the struct.
#[model_schema(check = ["withinItsLimit"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Stack(
    #[model_schema_prop(check = ["withoutDuplicates"])] pub Vec<String>,
    pub u32,
);

/// A list whose check names a place below it, which the member's name is written in front of.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Pallet {
    #[model_schema_prop(check = ["shortList"])]
    pub codes: Vec<String>,
}

/// A unit struct, which has one value: its check answers the same of every one.
#[model_schema(check = ["takingParcels"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ClosedDock;

/// An enum under a tag: a check on a member of one variant, and one on the enum.
#[model_schema(check = ["withFewStops"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Shipment {
    Air {
        flight: String,
        #[model_schema_prop(check = ["withoutDuplicates"])]
        stops: Vec<String>,
    },
    Ground {
        stops: Vec<String>,
    },
}

/// An enum serde writes under its variant's name.
#[model_schema(check = ["withAStop"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Transfer {
    Held,
    Rail(String),
    Road { stops: Vec<String> },
}

/// An enum under a tag and a content key.
#[model_schema(check = ["signedBySomeone"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "detail")]
pub enum Handover {
    Left(String),
    Signed { by: String },
}

#[model_schema(check = ["aReference"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Reference {
    Named { name: String },
    Numbered(u32),
}

/// An enum no variant of which holds a value.
#[model_schema(check = ["rated"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Grade {
    Express,
    Standard,
    Unrated,
}

#[model_schema(check = ["upperCase"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Code(String);

/// A type with a parameter, whose check is written once over every filling.
#[model_schema(default_types(CodeType = String), check = ["withACode"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Crate<CodeType> {
    pub codes: Vec<CodeType>,
    pub label: String,
}

/// A brand with a parameter.
#[model_schema(no_display, default_types(CodeType = String), check = ["holdingACode"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Batch<CodeType>(Vec<CodeType>);

/// A type with a parameter that holds a value of the parameter itself, in a key of its own.
#[model_schema(default_types(IdType = String), check = ["withEntries"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ledger<IdType> {
    pub entries: Vec<String>,
    pub owner_id: IdType,
}

/// A type with a parameter whose member holds a generic type at the holder's own parameter, under
/// a check of its own.
#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Archive<IdType> {
    pub label: String,
    #[model_schema_prop(check = ["withEntries"])]
    pub ledger: Ledger<IdType>,
}

fn without_duplicates(value: &[String]) -> Result<(), Vec<String>> {
    for (index, code) in value.iter().enumerate() {
        if value[..index].contains(code) {
            return Err(vec![format!("duplicate item code {code}")]);
        }
    }
    Ok(())
}

fn within_parcel(value: &ParcelDraft) -> Result<(), Vec<String>> {
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

fn with_a_route(value: &RouteDraft) -> Result<(), Vec<String>> {
    if value.stops.is_empty() {
        return Err(vec!["'stops': a route makes at least one stop".to_owned()]);
    }
    Ok(())
}

fn within_its_limit(value: &StackDraft) -> Result<(), Vec<String>> {
    if u32::try_from(value.0.len()).is_ok_and(|held| held <= value.1) {
        return Ok(());
    }
    Err(vec![format!(
        "'0': a stack holds at most {} codes",
        value.1
    )])
}

fn short_list(value: &[String]) -> Result<(), Vec<String>> {
    value.get(2).map_or(Ok(()), |third| {
        Err(vec![format!("'2': {third} is one code too many")])
    })
}

fn taking_parcels(_value: &ClosedDockDraft) -> Result<(), Vec<String>> {
    Err(vec!["a closed dock takes no parcel".to_owned()])
}

fn with_few_stops(value: &ShipmentDraft) -> Result<(), Vec<String>> {
    if let ShipmentDraft::Air {
        flight: _flight,
        stops,
    } = value
        && stops.len() > 2
    {
        return Err(vec![
            "'stops': an air shipment makes at most 2 stops".to_owned(),
        ]);
    }
    Ok(())
}

fn with_a_stop(value: &TransferDraft) -> Result<(), Vec<String>> {
    if let TransferDraft::Road { stops } = value
        && stops.is_empty()
    {
        return Err(vec![
            "'Road.stops': a road transfer makes a stop".to_owned(),
        ]);
    }
    if let TransferDraft::Rail(line) = value
        && line.is_empty()
    {
        return Err(vec!["'Rail': a rail transfer names its line".to_owned()]);
    }
    Ok(())
}

fn signed_by_someone(value: &HandoverDraft) -> Result<(), Vec<String>> {
    if let HandoverDraft::Signed { by } = value
        && by.is_empty()
    {
        return Err(vec![
            "'detail.by': a handover is signed by someone".to_owned(),
        ]);
    }
    Ok(())
}

fn a_reference(value: &ReferenceDraft) -> Result<(), Vec<String>> {
    if let ReferenceDraft::Named { name } = value
        && name.is_empty()
    {
        return Err(vec!["'name': a reference has a name".to_owned()]);
    }
    if matches!(value, ReferenceDraft::Numbered(0)) {
        return Err(vec!["a reference is not zero".to_owned()]);
    }
    Ok(())
}

fn rated(value: &GradeDraft) -> Result<(), Vec<String>> {
    if matches!(value, GradeDraft::Unrated) {
        return Err(vec!["a parcel is rated".to_owned()]);
    }
    Ok(())
}

fn upper_case(value: &Code) -> Result<(), Vec<String>> {
    if value.as_inner().chars().any(char::is_lowercase) {
        return Err(vec!["a code is upper case".to_owned()]);
    }
    Ok(())
}

fn with_a_code<CodeType>(value: &CrateDraft<CodeType>) -> Result<(), Vec<String>> {
    if value.codes.is_empty() {
        return Err(vec!["'codes': a crate holds at least one code".to_owned()]);
    }
    Ok(())
}

fn holding_a_code<CodeType>(value: &Batch<CodeType>) -> Result<(), Vec<String>> {
    if value.as_inner().is_empty() {
        return Err(vec!["a batch holds at least one code".to_owned()]);
    }
    Ok(())
}

fn with_entries<IdType>(value: &LedgerDraft<IdType>) -> Result<(), Vec<String>> {
    if value.entries.is_empty() {
        return Err(vec![
            "'entries': a ledger holds at least one entry".to_owned(),
        ]);
    }
    Ok(())
}

fn archived<IdType>(entries: Vec<String>, owner_id: IdType) -> Archive<IdType> {
    Archive::model_schema_wrap_unjudged(ArchiveDraft {
        ledger: Ledger::model_schema_wrap_unjudged(LedgerDraft { entries, owner_id }),
        label: "a".to_owned(),
    })
}

fn texts(written: &[&str]) -> Vec<String> {
    written.iter().map(|&text| text.to_owned()).collect()
}

fn case<T>(name: &'static str, value: &T, answered: Result<(), Vec<String>>) -> Case
where
    T: Serialize,
{
    Case {
        answered,
        name,
        payload: serde_json::to_string(value).unwrap(),
    }
}

fn parcel(label: &str, item_code_list: &[&str], fragile_item_codes: &[&str]) -> Parcel {
    Parcel::model_schema_wrap_unjudged(ParcelDraft {
        fragile_item_codes: texts(fragile_item_codes),
        item_code_list: texts(item_code_list),
        label: label.to_owned(),
    })
}

/// The five parcels of the shared table, in the order of its cases.
pub fn parcels() -> [Parcel; 5] {
    [
        parcel("A", &["x"], &["x"]),
        parcel("", &["x"], &["x"]),
        parcel("A", &["x", "x"], &["x"]),
        parcel("A", &["x"], &["x", "y"]),
        parcel("", &["x", "x"], &["y"]),
    ]
}

/// The manifest of the table's last case: its fourth parcel, held under `parcel`.
pub fn manifest() -> Manifest {
    let [_, _, _, outside, _] = parcels();
    Manifest::model_schema_wrap_unjudged(ManifestDraft { parcel: outside })
}

/// Cases 1 to 5 of the shared table.
pub fn parcel_cases() -> Vec<Case> {
    ["case 1", "case 2", "case 3", "case 4", "case 5"]
        .into_iter()
        .zip(parcels())
        .map(|(name, held)| case(name, &held, crate::judged_issues(held.validation_issues())))
        .collect()
}

/// Case 6 of the shared table, where a holder runs what it holds.
#[cfg(feature = "serde")]
pub fn manifest_case() -> Case {
    let held = manifest();
    case(
        "case 6",
        &held,
        crate::judged_issues(held.validation_issues()),
    )
}

/// The cases of the structs: one with named fields, one with slots, and one with neither.
fn struct_cases() -> Vec<Case> {
    let route = |stops: &[&str]| RouteDraft {
        stops: texts(stops),
    };
    let stack = |codes: &[&str], limit: u32| StackDraft(texts(codes), limit);
    let pallet = |codes: &[&str]| PalletDraft {
        codes: texts(codes),
    };
    vec![
        case(
            "Route, no stop",
            &route(&[]),
            crate::judged_issues(route(&[]).validation_issues()),
        ),
        case(
            "Route",
            &route(&["a"]),
            crate::judged_issues(route(&["a"]).validation_issues()),
        ),
        case(
            "Stack, a code twice",
            &stack(&["a", "a"], 5),
            crate::judged_issues(stack(&["a", "a"], 5).validation_issues()),
        ),
        case(
            "Stack, over its limit",
            &stack(&["a", "b"], 1),
            crate::judged_issues(stack(&["a", "b"], 1).validation_issues()),
        ),
        case(
            "Stack",
            &stack(&["a"], 1),
            crate::judged_issues(stack(&["a"], 1).validation_issues()),
        ),
        case(
            "Pallet, three codes",
            &pallet(&["a", "b", "c"]),
            crate::judged_issues(pallet(&["a", "b", "c"]).validation_issues()),
        ),
        case(
            "Pallet",
            &pallet(&["a"]),
            crate::judged_issues(pallet(&["a"]).validation_issues()),
        ),
        case(
            "ClosedDock",
            &ClosedDockDraft,
            crate::judged_issues(ClosedDockDraft.validation_issues()),
        ),
    ]
}

/// The cases of the enums: one under each form serde writes an enum in.
fn enum_cases() -> Vec<Case> {
    let air = |stops: &[&str]| ShipmentDraft::Air {
        flight: "F1".to_owned(),
        stops: texts(stops),
    };
    let ground = ShipmentDraft::Ground {
        stops: texts(&["a", "b", "c"]),
    };
    let road = TransferDraft::Road { stops: Vec::new() };
    let rail = TransferDraft::Rail(String::new());
    let signed = HandoverDraft::Signed { by: String::new() };
    let left = HandoverDraft::Left("porch".to_owned());
    let unnamed = ReferenceDraft::Named {
        name: String::new(),
    };
    let named = ReferenceDraft::Named {
        name: "north".to_owned(),
    };
    vec![
        case(
            "Shipment, three stops by air",
            &air(&["a", "b", "c"]),
            crate::judged_issues(air(&["a", "b", "c"]).validation_issues()),
        ),
        case(
            "Shipment, a stop twice",
            &air(&["a", "a"]),
            crate::judged_issues(air(&["a", "a"]).validation_issues()),
        ),
        case(
            "Shipment, three stops by ground",
            &ground,
            crate::judged_issues(ground.validation_issues()),
        ),
        case(
            "Transfer, a road with no stop",
            &road,
            crate::judged_issues(road.validation_issues()),
        ),
        case(
            "Transfer, a rail with no line",
            &rail,
            crate::judged_issues(rail.validation_issues()),
        ),
        case(
            "Transfer, held",
            &TransferDraft::Held,
            crate::judged_issues(TransferDraft::Held.validation_issues()),
        ),
        case(
            "Handover, signed by no one",
            &signed,
            crate::judged_issues(signed.validation_issues()),
        ),
        case(
            "Handover, left",
            &left,
            crate::judged_issues(left.validation_issues()),
        ),
        case(
            "Reference, numbered zero",
            &ReferenceDraft::Numbered(0),
            crate::judged_issues(ReferenceDraft::Numbered(0).validation_issues()),
        ),
        case(
            "Reference, with no name",
            &unnamed,
            crate::judged_issues(unnamed.validation_issues()),
        ),
        case(
            "Reference",
            &named,
            crate::judged_issues(named.validation_issues()),
        ),
        case(
            "Grade, unrated",
            &GradeDraft::Unrated,
            crate::judged_issues(GradeDraft::Unrated.validation_issues()),
        ),
        case(
            "Grade, express",
            &GradeDraft::Express,
            crate::judged_issues(GradeDraft::Express.validation_issues()),
        ),
        case(
            "Grade, standard",
            &GradeDraft::Standard,
            crate::judged_issues(GradeDraft::Standard.validation_issues()),
        ),
    ]
}

/// A brand over a list, over a string and over a number.
fn batch_cases() -> Vec<Case> {
    vec![
        case(
            "Batch<String>, empty",
            &Batch::<String>::model_schema_read_unjudged(
                serde_json::to_value(Vec::<String>::new()).unwrap(),
            )
            .unwrap(),
            Batch::<String>::new(Vec::new()).map(|_| ()),
        ),
        case(
            "Batch<u32>, empty",
            &Batch::<u32>::model_schema_read_unjudged(
                serde_json::to_value(Vec::<u32>::new()).unwrap(),
            )
            .unwrap(),
            Batch::<u32>::new(Vec::new()).map(|_| ()),
        ),
        case(
            "Batch<u32>",
            &Batch::new(vec![7_u32]).unwrap(),
            Batch::new(vec![7_u32]).map(|_| ()),
        ),
    ]
}

/// The cases of the brands and of the type with a parameter, each at two fillings.
fn brand_and_crate_cases() -> Vec<Case> {
    let filled = |codes: Vec<String>| CrateDraft {
        codes,
        label: "c".to_owned(),
    };
    let counted = |codes: Vec<u32>| CrateDraft {
        codes,
        label: "c".to_owned(),
    };
    vec![
        case(
            "Code, in lower case",
            &Code::model_schema_read_unjudged(serde_json::to_value("ab".to_owned()).unwrap())
                .unwrap(),
            Code::new("ab".to_owned()).map(|_| ()),
        ),
        case(
            "Code",
            &Code::new("AB".to_owned()).unwrap(),
            Code::new("AB".to_owned()).map(|_| ()),
        ),
        case(
            "Crate<String>, empty",
            &filled(Vec::new()),
            crate::judged_issues(filled(Vec::new()).validation_issues()),
        ),
        case(
            "Crate<String>",
            &filled(texts(&["a"])),
            crate::judged_issues(filled(texts(&["a"])).validation_issues()),
        ),
        case(
            "Crate<u32>, empty",
            &counted(Vec::new()),
            crate::judged_issues(counted(Vec::new()).validation_issues()),
        ),
        case(
            "Crate<u32>",
            &counted(vec![7]),
            crate::judged_issues(counted(vec![7]).validation_issues()),
        ),
    ]
}

/// The cases of the types that hold a value of their own parameter, each at two fillings.
fn ledger_and_archive_cases() -> Vec<Case> {
    let owned = |entries: Vec<String>| LedgerDraft {
        entries,
        owner_id: "o".to_owned(),
    };
    vec![
        case(
            "Ledger<String>, no entry",
            &owned(Vec::new()),
            crate::judged_issues(owned(Vec::new()).validation_issues()),
        ),
        case(
            "Ledger<String>",
            &owned(texts(&["a"])),
            crate::judged_issues(owned(texts(&["a"])).validation_issues()),
        ),
        case(
            "Ledger<u32>",
            &LedgerDraft {
                entries: texts(&["a"]),
                owner_id: 7_u32,
            },
            crate::judged_issues(
                LedgerDraft {
                    entries: texts(&["a"]),
                    owner_id: 7_u32,
                }
                .validation_issues(),
            ),
        ),
        case(
            "Archive<String>, no entry",
            &archived(Vec::new(), "o".to_owned()),
            crate::judged_issues(archived(Vec::new(), "o".to_owned()).validation_issues()),
        ),
        case(
            "Archive<String>",
            &archived(texts(&["a"]), "o".to_owned()),
            crate::judged_issues(archived(texts(&["a"]), "o".to_owned()).validation_issues()),
        ),
        case(
            "Archive<u32>, no entry",
            &archived(Vec::new(), 7_u32),
            crate::judged_issues(archived(Vec::new(), 7_u32).validation_issues()),
        ),
        case(
            "Archive<u32>",
            &archived(texts(&["a"]), 7_u32),
            crate::judged_issues(archived(texts(&["a"]), 7_u32).validation_issues()),
        ),
    ]
}

/// One breaking value and one passing value of every shape. A unit struct has one value, which
/// its check answers the same of every time.
pub fn shape_cases() -> Vec<Case> {
    let mut cases = struct_cases();
    cases.extend(enum_cases());
    cases.extend(brand_and_crate_cases());
    cases.extend(batch_cases());
    cases.extend(ledger_and_archive_cases());
    cases
}

/// `cases` as every side prints them: a case's name and whether it passes, then each message it
/// is refused with on a line of its own.
pub fn report(cases: &[Case]) -> String {
    let mut printed = String::new();
    for held in cases {
        match &held.answered {
            Ok(()) => writeln!(printed, "{}: passes", held.name).unwrap(),
            Err(messages) => {
                writeln!(printed, "{}: refused", held.name).unwrap();
                for message in messages {
                    writeln!(printed, "    {message}").unwrap();
                }
            }
        }
    }
    printed
}
