//! What every part of `flatten_matrix_part*_tests.rs` builds its rows with and holds each to:
//! the values of each member type, every value of a holder of one, two or three of them, and the
//! check of each row against what serde was measured to do with it.

use alloc::collections::BTreeMap;
use core::fmt::Debug;

use serde::Serialize;
use serde::de::DeserializeOwned;

/// The values of a member type, one per variant, each under the label the measure names it by.
pub trait Sample: Sized {
    fn samples() -> Vec<(String, Self)>;
}

impl<T: Sample> Sample for Option<T> {
    fn samples() -> Vec<(String, Self)> {
        let mut values = vec![("None".to_owned(), None)];
        values.extend(
            T::samples()
                .into_iter()
                .map(|(label, value)| (label, Some(value))),
        );
        values
    }
}

impl Sample for BTreeMap<String, u32> {
    fn samples() -> Vec<(String, Self)> {
        vec![("-".to_owned(), Self::from([("m0_k".to_owned(), 80)]))]
    }
}

/// Every value of a holder of one member, built by `build`.
pub fn rows1<A, H, F>(build: F) -> Vec<(String, H)>
where
    A: Sample,
    F: Fn(A) -> H,
{
    A::samples()
        .into_iter()
        .map(|(label, held)| (label, build(held)))
        .collect()
}

/// Every value of a holder of two members, built by `build`.
pub fn rows2<A, B, H, F>(build: F) -> Vec<(String, H)>
where
    A: Clone + Sample,
    B: Sample,
    F: Fn(A, B) -> H,
{
    let mut values = Vec::new();
    for (first, a) in A::samples() {
        for (second, b) in B::samples() {
            values.push((format!("{first},{second}"), build(a.clone(), b)));
        }
    }
    values
}

/// Every value of a holder of three members, built by `build`.
pub fn rows3<A, B, C, H, F>(build: F) -> Vec<(String, H)>
where
    A: Clone + Sample,
    B: Clone + Sample,
    C: Sample,
    F: Fn(A, B, C) -> H,
{
    let mut values = Vec::new();
    for (first, a) in A::samples() {
        for (second, b) in B::samples() {
            for (third, c) in C::samples() {
                values.push((
                    format!("{first},{second},{third}"),
                    build(a.clone(), b.clone(), c),
                ));
            }
        }
    }
    values
}

/// What does not hold of `shape`: each row written, read back equal, and read with `"stray":1`
/// first as `measured` states, `R` refused, `E` read and dropped, `U` read into a map.
pub fn check<H>(shape: &str, rows: &[(String, H)], measured: &[(&str, char)]) -> Vec<String>
where
    H: DeserializeOwned + Debug + PartialEq + Serialize,
{
    let mut failed = Vec::new();
    if rows.len() != measured.len() {
        failed.push(format!(
            "{shape}: {} rows, {} measured",
            rows.len(),
            measured.len()
        ));
    }
    for (label, value) in rows {
        let text = serde_json::to_string(value).unwrap();
        match serde_json::from_str::<H>(&text) {
            Ok(back) if back == *value => {}
            read => failed.push(format!("{shape} {label}: {text} reads back as {read:?}")),
        }
        let stray = text
            .strip_prefix('{')
            .filter(|rest| *rest != "}")
            .map_or_else(
                || "{\"stray\":1}".to_owned(),
                |rest| format!("{{\"stray\":1,{rest}"),
            );
        let found = match serde_json::from_str::<H>(&stray) {
            Ok(back) if back == *value => 'E',
            Ok(_) => 'U',
            Err(_) => 'R',
        };
        let stated = measured
            .iter()
            .find(|(named, _)| named == label)
            .map(|(_, outcome)| *outcome);
        if stated != Some(found) {
            failed.push(format!(
                "{shape} {label}: {stray} is {found}, measured {stated:?}"
            ));
        }
    }
    failed
}
