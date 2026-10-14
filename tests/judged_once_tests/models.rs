//! Holders that hold a checked model in a member whose type names one of their own type
//! parameters or `Self`, and the checks, which count their calls on the thread that runs them.

use core::cell::Cell;
use core::fmt::Display;
use core::hash::Hash;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tixschema::model_schema;

thread_local! {
    /// How many times each check has run on this thread: `Entry`'s, `Tag`'s, and `Node`'s and
    /// `Tree`'s.
    static CHECKS: [Cell<usize>; 3] = const { [Cell::new(0), Cell::new(0), Cell::new(0)] };
    /// How many times [`listed_entries`] has run on this thread.
    static CORRECTIONS: Cell<usize> = const { Cell::new(0) };
}

/// A checked model a holder fills with its own type parameter.
#[model_schema(default_types(IdType = String), check = ["countedEntry"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Entry<IdType> {
    pub id: IdType,
    pub label: String,
}

/// A checked model with no type parameter.
#[model_schema(check = ["countedTag"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tag {
    pub name: String,
}

/// `entries` names the holder's type parameter; `tags` names none.
#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Ledger<IdType> {
    pub entries: Vec<Entry<IdType>>,
    pub tags: Vec<Tag>,
}

/// A holder of holders, one level up.
#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Book<IdType> {
    pub ledgers: Vec<Ledger<IdType>>,
}

/// The same entries with a concrete filling.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StringLedger {
    pub entries: Vec<Entry<String>>,
}

/// A checked type that holds itself, written as `Self`.
#[model_schema(check = ["countedNode"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Node {
    pub children: Vec<Self>,
}

/// An entry under each wrapper a holder reads a model unjudged through.
#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Wrapped<IdType> {
    pub boxed: Box<Entry<IdType>>,
    pub by_key: HashMap<String, Entry<IdType>>,
    pub listed: Vec<Option<Entry<IdType>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spare: Option<Entry<IdType>>,
}

/// The parameter is named by the map's key alone, and the model it holds names none.
#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Index<IdType: Eq + Hash> {
    pub tags: HashMap<IdType, Tag>,
}

/// Two parameters, the member that holds the model naming one.
#[model_schema(default_types(IdType = String, Held = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Pair<IdType, Held> {
    pub entries: Vec<Entry<IdType>>,
    pub held: Vec<Held>,
}

/// A parameter bounded where it is declared, read under a bound the container writes.
#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(bound(deserialize = "IdType: serde::de::DeserializeOwned"))]
pub struct DisplayedLedger<IdType: Display> {
    pub entries: Vec<Entry<IdType>>,
}

/// A bound that ties the parameter the member names to one it does not: the member is read with
/// the entries judged as they are read, and again by the holder.
#[model_schema(default_types(IdType = String, Held = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TiedPair<IdType, Held>
where
    IdType: From<Held>,
{
    pub entries: Vec<Entry<IdType>>,
    pub held: Vec<Held>,
}

/// A checked type with a parameter that holds itself, written as `Self`.
#[model_schema(default_types(IdType = String), check = ["countedTree"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tree<IdType> {
    pub children: Vec<Self>,
    pub id: IdType,
}

/// A member that names a `preprocess`, read from the payload the function corrected.
#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Corrected<IdType> {
    #[model_schema_prop(preprocess = ["listedEntries"])]
    pub entries: Vec<Entry<IdType>>,
}

/// What fills the parameter is a model itself, which `validate()` does not walk.
#[model_schema(default_types(Held = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Shelf<Held> {
    pub held: Vec<Held>,
}

/// An enum whose variants hold an entry under the enum's own parameter.
#[model_schema(default_types(IdType = String))]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Posting<IdType> {
    Listed { entries: Vec<Entry<IdType>> },
    Single(Entry<IdType>),
}

fn counted(check: usize) {
    CHECKS.with(|counts| {
        if let Some(count) = counts.get(check) {
            count.set(count.get() + 1);
        }
    });
}

fn counted_entry<IdType>(value: &Entry<IdType>) -> Result<(), Vec<String>> {
    counted(0);
    if value.label.is_empty() {
        return Err(vec!["'label': an entry has a label".to_owned()]);
    }
    Ok(())
}

fn counted_tag(value: &Tag) -> Result<(), Vec<String>> {
    counted(1);
    if value.name.is_empty() {
        return Err(vec!["'name': a tag has a name".to_owned()]);
    }
    Ok(())
}

fn counted_node(value: &Node) -> Result<(), Vec<String>> {
    counted(2);
    at_most_two_children(value.children.len())
}

fn counted_tree<IdType>(value: &Tree<IdType>) -> Result<(), Vec<String>> {
    counted(2);
    at_most_two_children(value.children.len())
}

fn at_most_two_children(children: usize) -> Result<(), Vec<String>> {
    if children > 2 {
        return Err(vec![
            "'children': a node holds at most two children".to_owned(),
        ]);
    }
    Ok(())
}

/// Reads one entry written alone as a list of it.
fn listed_entries(payload: Value) -> Value {
    CORRECTIONS.with(|count| count.set(count.get() + 1));
    match payload {
        Value::Object(_) => Value::Array(vec![payload]),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Array(_) => {
            payload
        }
    }
}

/// The checks of `Entry`, `Tag`, and `Node` with `Tree`, run on this thread since the last call.
pub fn taken() -> [usize; 3] {
    CHECKS.with(|counts| counts.each_ref().map(|count| count.replace(0)))
}

/// The calls of [`listed_entries`] on this thread since the last call.
pub fn corrections_taken() -> usize {
    CORRECTIONS.with(|count| count.replace(0))
}
