//! A plain enum that names a check, read on its own and held by another type, and one that names
//! none: the models, and the payloads each emitted client is held to. A payload is read as serde
//! reads it, and prints what that read prints.

#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
use alloc::collections::BTreeMap;
#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
use core::fmt::Write as _;
use core::future::ready;

#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
use serde_json::Value;
use tixschema::{model_schema, service_schema};

/// A plain enum that names a check.
#[model_schema(check = ["affirmed"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Verdict {
    #[serde(alias = "Reversed")]
    Overturned,
    Upheld,
}

#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
/// A plain enum that names a check and a function, one variant of which is read under an alias.
#[model_schema(check = ["allowed"], preprocess = ["tidied"])]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Ruling {
    #[serde(alias = "Overruled")]
    Denied,
    Granted,
}

#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
/// A plain enum that names no check.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Bench {
    Lower,
    Upper,
}

#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
/// A holder of the enum in a member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Appeal {
    pub verdict: Verdict,
}

#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
/// A holder of the enum in a list and in a map.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Hearing {
    pub by_court: BTreeMap<String, Verdict>,
    pub verdicts: Vec<Verdict>,
}

/// The error the service below declares.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum AppraisalError {
    Missing,
}

/// A service whose one reply is the plain enum that names a check.
#[service_schema(transports = [])]
pub trait AppraisalClientService<Ctx> {
    async fn appraise(&self, ctx: &Ctx, case_number: String) -> Result<Verdict, AppraisalError>;
}

/// A back end answering the contract: it writes the reply the check refuses, as writing judges
/// nothing.
pub struct Court;

impl AppraisalClientService<()> for Court {
    async fn appraise(&self, _ctx: &(), case_number: String) -> Result<Verdict, AppraisalError> {
        ready(()).await;
        if case_number.is_empty() {
            return Err(AppraisalError::Missing);
        }
        Ok(Verdict::Overturned)
    }
}

#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
/// One payload, the type it is read as, how serde reads it, and whether the sentence of a refusal
/// is held to Rust's.
pub struct Row {
    pub answer: fn(&Self) -> String,
    pub detail: bool,
    pub name: &'static str,
    pub payload: &'static str,
    pub read_as: &'static str,
}

#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
fn allowed(value: &Ruling) -> Result<(), Vec<String>> {
    if matches!(value, Ruling::Denied) {
        return Err(vec!["a ruling is allowed".to_owned()]);
    }
    Ok(())
}

fn affirmed(value: &Verdict) -> Result<(), Vec<String>> {
    if matches!(value, Verdict::Overturned) {
        return Err(vec!["a verdict is upheld".to_owned()]);
    }
    Ok(())
}

#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
fn tidied(payload: Value) -> Value {
    if let Value::String(text) = payload {
        Value::String(text.trim().to_owned())
    } else {
        payload
    }
}

#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
fn answered<T>(row: &Row) -> String
where
    T: DeserializeOwned + Serialize,
{
    let mut text = String::new();
    match serde_json::from_str::<T>(row.payload) {
        Ok(read) => {
            let _ = writeln!(
                text,
                "{}: read {}",
                row.name,
                serde_json::to_string(&read).unwrap()
            );
        }
        Err(refused) => {
            let _ = writeln!(text, "{}: refused", row.name);
            if row.detail {
                let message = refused.to_string();
                let held = message
                    .rsplit_once(" at line ")
                    .map_or(message.as_str(), |(head, _)| head);
                for line in held.split("; ") {
                    let _ = writeln!(text, "    {line}");
                }
            }
        }
    }
    text
}

#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
fn row<T>(name: &'static str, read_as: &'static str, payload: &'static str) -> Row
where
    T: DeserializeOwned + Serialize,
{
    Row {
        answer: answered::<T>,
        detail: true,
        name,
        payload,
        read_as,
    }
}

#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
/// The payloads of the table. A name that is no case is refused in words each language has.
pub fn rows() -> Vec<Row> {
    vec![
        row::<Verdict>("Verdict, upheld", "Verdict", r#""Upheld""#),
        row::<Verdict>("Verdict, overturned", "Verdict", r#""Overturned""#),
        Row {
            detail: false,
            ..row::<Verdict>("Verdict, no such case", "Verdict", r#""Quashed""#)
        },
        row::<Verdict>("Verdict, read under an alias", "Verdict", r#""Reversed""#),
        row::<Ruling>("Ruling, granted", "Ruling", r#"" Granted ""#),
        row::<Ruling>("Ruling, denied", "Ruling", r#"" Denied ""#),
        row::<Ruling>("Ruling, read under an alias", "Ruling", r#""Overruled""#),
        row::<Appeal>("Appeal, upheld", "Appeal", r#"{"verdict":"Upheld"}"#),
        row::<Appeal>(
            "Appeal, overturned",
            "Appeal",
            r#"{"verdict":"Overturned"}"#,
        ),
        row::<Hearing>(
            "Hearing, items overturned",
            "Hearing",
            r#"{"by_court":{"north":"Overturned","south":"Upheld"},"verdicts":["Upheld","Overturned","Overturned"]}"#,
        ),
        row::<Hearing>(
            "Hearing, upheld",
            "Hearing",
            r#"{"by_court":{"north":"Upheld"},"verdicts":["Upheld"]}"#,
        ),
        row::<Bench>("Bench, upper", "Bench", r#""Upper""#),
    ]
}

#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
/// What the serde read prints of every row.
pub fn expected(rows: &[Row]) -> String {
    rows.iter().map(|row| (row.answer)(row)).collect()
}

#[cfg(test)]
mod tests {
    use super::super::tests::poll_once;
    use super::{AppraisalClientService as _, Court, Verdict};
    #[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
    use super::{expected, rows};

    /// The reply the clients below are handed is one the Rust back end writes, and the serde read
    /// refuses it with the check's sentence.
    #[test]
    fn the_back_end_writes_the_reply_the_serde_read_refuses() {
        let written = poll_once(Court.appraise(&(), "c".to_owned()))
            .unwrap()
            .unwrap();
        let payload = serde_json::to_string(&written).unwrap();
        assert_eq!(payload, r#""Overturned""#);
        assert_eq!(
            serde_json::from_str::<Verdict>(&payload)
                .unwrap_err()
                .to_string(),
            "a verdict is upheld"
        );
    }

    /// The refusals the serde read gives the table, which every client is held to.
    #[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
    #[test]
    fn the_serde_read_refuses_what_the_check_refuses() {
        assert_eq!(
            expected(&rows()),
            "\
Verdict, upheld: read \"Upheld\"
Verdict, overturned: refused
    a verdict is upheld
Verdict, no such case: refused
Verdict, read under an alias: refused
    a verdict is upheld
Ruling, granted: read \"Granted\"
Ruling, denied: refused
    a ruling is allowed
Ruling, read under an alias: refused
    a ruling is allowed
Appeal, upheld: read {\"verdict\":\"Upheld\"}
Appeal, overturned: refused
    'verdict': a verdict is upheld
Hearing, items overturned: refused
    'by_court.north': a verdict is upheld
    'verdicts.1': a verdict is upheld
    'verdicts.2': a verdict is upheld
Hearing, upheld: read {\"by_court\":{\"north\":\"Upheld\"},\"verdicts\":[\"Upheld\"]}
Bench, upper: read \"Upper\"
"
        );
    }
}
