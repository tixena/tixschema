//! The payloads each emitted client reads a std `Result` from, and what the serde read prints of
//! each: the value written back, or `refused`, with each message where `validate()` refused it.

use core::fmt::Write as _;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::result_models::{
    Attempted, Bureau, Errand, Errata, Missive, Notebook, Postbox, Reply, Summary, Turn,
    judged_payloads, read_payloads, refused_payloads,
};

/// One payload a client reads: its name, the type it is read as, and the JSON.
pub struct Row {
    pub json: String,
    pub name: &'static str,
    pub read_as: &'static str,
}

/// A model a row is read as: its name, whether `validate()` judges a model it holds, and what the
/// serde read prints of a row read as it.
type Printer = (&'static str, bool, fn(&Row) -> String);

/// Every payload of a `Result`: what reads, what serde refuses, and what `validate()` judges.
pub fn rows() -> Vec<Row> {
    read_payloads()
        .into_iter()
        .chain(refused_payloads())
        .chain(judged_payloads())
        .map(|payload| Row {
            json: payload.json,
            name: payload.name,
            read_as: payload.read_as,
        })
        .collect()
}

/// What the serde read prints of `row`, read as `T`. A refusal of `validate()`, of a type that
/// holds a model or a brand, lists each message; any other refusal is the runtime's to word.
fn printed<T>(row: &Row, judged: bool) -> String
where
    T: DeserializeOwned + Serialize,
{
    match serde_json::from_str::<T>(&row.json) {
        Ok(read) => format!(
            "{}: read {}\n",
            row.name,
            serde_json::to_string(&read).unwrap()
        ),
        Err(refused) => {
            let mut said = format!("{}: refused\n", row.name);
            if judged {
                for message in refused.to_string().split("; ") {
                    let _ = writeln!(said, "    {message}");
                }
            }
            said
        }
    }
}

/// Each model a row is read as.
fn models() -> Vec<Printer> {
    vec![
        ("Attempted", false, |row| {
            printed::<Attempted<u32>>(row, false)
        }),
        ("Bureau", false, |row| printed::<Bureau>(row, false)),
        ("Errand", false, |row| printed::<Errand>(row, false)),
        ("Errata", true, |row| printed::<Errata>(row, true)),
        ("Missive", true, |row| printed::<Missive>(row, true)),
        ("Notebook", true, |row| printed::<Notebook>(row, true)),
        ("Postbox", false, |row| printed::<Postbox>(row, false)),
        ("Reply", false, |row| printed::<Reply>(row, false)),
        ("Summary", false, |row| printed::<Summary>(row, false)),
        ("Turn", false, |row| printed::<Turn<u32>>(row, false)),
    ]
}

/// Whether `read_as` names a type that declares a parameter, which a client reads at `u32`.
pub fn declares_a_parameter(read_as: &str) -> bool {
    matches!(read_as, "Attempted" | "Turn")
}

/// Whether `validate()` judges a model the type `read_as` names holds, so that a client lists
/// what it refuses.
#[cfg(any(feature = "dart", feature = "kotlin"))]
pub fn judged(read_as: &str) -> bool {
    models()
        .into_iter()
        .any(|(named, judges, _)| named == read_as && judges)
}

/// What the serde read prints of each of `rows`.
pub fn expected(rows: &[Row]) -> String {
    let models = models();
    rows.iter()
        .map(|row| {
            let (_, _, print) = models
                .iter()
                .find(|(named, _, _)| *named == row.read_as)
                .unwrap();
            print(row)
        })
        .collect()
}
