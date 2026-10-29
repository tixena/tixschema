//! The models the emitted Swift `validate()` is run on, and the Rust function each check names.

use alloc::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// The parcel of the shared table: a bound, a check on a field and a check on the type.
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

/// One field per bound key, a float among them, and two under an `Option`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gauge {
    #[model_schema_prop(minimum = 0.5, maximum = 9.5)]
    pub draft_metres: f64,
    #[model_schema_prop(minLength = 1, maxLength = 2)]
    pub hold_codes: Vec<String>,
    #[model_schema_prop(minimum = 0, maximum = 40)]
    pub knots: u32,
    #[model_schema_prop(minimum = 2)]
    pub load_tonnes: f64,
    #[model_schema_prop(pattern = "^[A-Z]{3}$")]
    pub port_code: String,
    #[model_schema_prop(minLength = 2, maxLength = 4)]
    pub short_name: String,
    #[model_schema_prop(minimum = 1)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_knots: Option<u32>,
    #[model_schema_prop(minLength = 2)]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_name: Option<String>,
}

/// A member named after the method every emitted type has, and one named after a local it writes.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Inspection {
    #[model_schema_prop(minLength = 1)]
    pub errors: Vec<String>,
    pub validate: String,
}

/// A string held to no length at all, so every character it holds is counted back.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Nameplate {
    #[model_schema_prop(maxLength = 0)]
    pub engraved: String,
}

/// A brand with a bound of its own.
#[model_schema(minLength = 2)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Code(pub String);

/// A brand over a brand, with a bound of its own.
#[model_schema(maxLength = 4)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct SealedCode(pub Code);

#[model_schema()]
pub type Title = String;

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    #[model_schema_prop(minLength = 1)]
    pub item_label: String,
    pub parcel_code: Code,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Base {
    #[model_schema_prop(minLength = 1)]
    pub base_name: String,
}

/// A model held in each place a holder walks: a field, a list of lists, a list, a map under a
/// string and under a number, an optional, a tuple and a flattened member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Depot {
    pub by_number: BTreeMap<u32, Package>,
    pub deck_pair: (String, Package),
    pub first_parcel: Package,
    pub grid: Vec<Vec<Package>>,
    pub parcel_list: Vec<Package>,
    pub parcels_by_dock: BTreeMap<String, Package>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_parcel: Option<Package>,
    #[serde(flatten)]
    pub starting_base: Base,
}

/// A model held under two wrappers at once: a map under a plain enum, a list of maps, a map of
/// lists, a list of optionals and an optional list.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Yard {
    pub by_grade: BTreeMap<Grade, Package>,
    pub docks_in_order: Vec<BTreeMap<String, Package>>,
    pub lists_by_dock: BTreeMap<String, Vec<Package>>,
    pub maybe_each: Vec<Option<Package>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare_list: Option<Vec<Package>>,
}

/// A list held to a count, with a model in each item and a check: a member's rules in the order
/// they run.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ledger {
    #[model_schema_prop(minLength = 3, check = ["withoutASparePair"])]
    pub parcel_list: Vec<Package>,
}

/// A bound written on a field typed with an alias of a string declared below it.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Docking {
    #[model_schema_prop(minLength = 2)]
    pub berth_title: LaterTitle,
}

/// A field typed with an alias of a string declared below it, with no bound.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Quay {
    pub quay_title: LaterTitle,
}

#[model_schema()]
pub type LaterTitle = String;

/// A bound written on a field typed with a brand declared above, with an alias of a string, and
/// with a brand declared below.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Waybill {
    #[model_schema_prop(minLength = 2)]
    pub later_code: LaterCode,
    #[model_schema_prop(maxLength = 3)]
    pub short_code: Code,
    #[model_schema_prop(minLength = 2)]
    pub titled: Title,
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct LaterCode(pub String);

/// A type with a parameter: a bound, and a check written once over every filling.
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

/// An alias of an `IndexSet`.
#[cfg(feature = "indexmap")]
#[model_schema()]
pub type Queue = indexmap::IndexSet<String>;

/// A brand over the alias of an `IndexSet`, with a length of its own.
#[cfg(feature = "indexmap")]
#[model_schema(no_display, minLength = 1)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct SomeQueue(pub Queue);

/// Members typed with the alias of an `IndexSet` and with a brand over it, each under a length.
#[cfg(feature = "indexmap")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Lineup {
    #[model_schema_prop(minLength = 1, maxLength = 2)]
    pub queue: Queue,
    #[model_schema_prop(maxLength = 2)]
    pub some: SomeQueue,
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

/// A brand with a parameter, whose bound holds the declared default alone.
#[model_schema(minLength = 2, default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Docket<IdType>(pub IdType);

/// A tuple struct of several slots: a bound on the first, a model in the second.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Consignment(#[model_schema_prop(minLength = 1)] pub String, pub Package);

/// A struct serde writes as the value of its one slot.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Sealed(#[model_schema_prop(minLength = 1)] pub String);

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Strongbox {
    pub seal: Sealed,
}

/// A unit struct, which has one value: its check answers the same of every one.
#[model_schema(check = ["takingParcels"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ClosedDock;

#[model_schema()]
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Grade {
    Express,
    Standard,
}

/// An enum with no tag attribute: a slot, a named member, and a slot of several.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all_fields = "camelCase")]
pub enum Freight {
    Boxed(#[model_schema_prop(minLength = 1)] String),
    Loose {
        #[model_schema_prop(minLength = 1)]
        freight_label: String,
    },
    Split(
        #[model_schema_prop(minLength = 1)] String,
        #[model_schema_prop(minimum = 1)] u32,
    ),
    Unladen,
}

/// An enum with no tag attribute whose one slot holds a model.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Cargo {
    Boxed(Package),
    Unladen,
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
    Unladen,
}

/// An enum under a tag alone, with a check of its own.
#[model_schema(check = ["withALift"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all_fields = "camelCase")]
pub enum Lift {
    Boxed(Package),
    Loose {
        #[model_schema_prop(minLength = 1)]
        lift_label: String,
    },
    Unladen,
}

/// An untagged enum: a named member and a slot.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged, rename_all_fields = "camelCase")]
pub enum Tally {
    Counted {
        #[model_schema_prop(minimum = 1)]
        tally_count: u32,
    },
    Named(#[model_schema_prop(minLength = 1)] String),
}

/// An enum whose own string check reaches the text its variants hold, and no number.
#[model_schema(minLength = 2)]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all_fields = "camelCase")]
pub enum Berth {
    Named { quay_name: String, quay_number: u32 },
    Open,
    Tagged(String),
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

fn with_a_lift(value: &Lift) -> Result<(), Vec<String>> {
    if matches!(value, Lift::Unladen) {
        return Err(vec!["a lift carries something".to_owned()]);
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

fn without_a_spare_pair(value: &[Package]) -> Result<(), Vec<String>> {
    if value.len() == 2 {
        return Err(vec!["a ledger holds no spare pair".to_owned()]);
    }
    Ok(())
}

fn without_duplicates(value: &[String]) -> Result<(), Vec<String>> {
    for (index, code) in value.iter().enumerate() {
        if value[..index].contains(code) {
            return Err(vec![format!("duplicate item code {code}")]);
        }
    }
    Ok(())
}
