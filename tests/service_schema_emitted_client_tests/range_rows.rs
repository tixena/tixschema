//! The payloads each emitted client reads a range from, and what the serde read prints of each:
//! the value written back, or `refused`, with each message where `validate()` refused it.

use core::fmt::Write as _;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::range_models::{
    Compass, Course, Detour, Excursion, Frontier, Positioned, Signposted, Spanned, Stride, Waypost,
    judged_payloads, read_payloads, refused_payloads,
};

/// One payload a client reads: its name, the type it is read as, and the JSON.
pub struct Row {
    pub json: String,
    pub name: &'static str,
    pub read_as: &'static str,
}

/// A model a row is read as: its name, whether `validate()` judges a model or a brand it holds,
/// and what the serde read prints of a row read as it.
type Printer = (&'static str, bool, fn(&Row) -> String);

/// Every payload of the ranges, among them one that holds a key no range writes, which each
/// client refuses as serde does.
pub fn rows() -> Vec<Row> {
    let payloads: Vec<_> = read_payloads()
        .into_iter()
        .chain(refused_payloads())
        .chain(judged_payloads())
        .collect();
    assert!(
        payloads.iter().any(|payload| payload.undeclared_key),
        "no payload holds a key no range writes"
    );
    payloads
        .into_iter()
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
        ("Compass", false, |row| printed::<Compass>(row, false)),
        ("Course", false, |row| printed::<Course<u32>>(row, false)),
        ("Detour", false, |row| printed::<Detour>(row, false)),
        ("Excursion", true, |row| printed::<Excursion>(row, true)),
        ("Frontier", true, |row| printed::<Frontier>(row, true)),
        ("Positioned", false, |row| printed::<Positioned>(row, false)),
        #[cfg(feature = "chrono")]
        ("Rostered", false, |row| {
            printed::<super::range_models::Rostered>(row, false)
        }),
        ("Signposted", true, |row| printed::<Signposted>(row, true)),
        ("Spanned", false, |row| printed::<Spanned>(row, false)),
        ("Stride", false, |row| printed::<Stride<u32>>(row, false)),
        ("Waypost", true, |row| printed::<Waypost>(row, true)),
    ]
}

/// Whether `read_as` names a type that declares a parameter, which a client reads at `u32`.
pub fn declares_a_parameter(read_as: &str) -> bool {
    matches!(read_as, "Course" | "Stride")
}

/// Whether `validate()` judges a model or a brand the type `read_as` names holds, so that a client
/// lists what it refuses.
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
