//! Each integer width Kotlin reads, held as a member of [`Spans`]. Each client refuses a value
//! just outside a width's range as serde refuses it, and reads the values at its ends.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

/// Each member and a value outside its range: just beyond it, or beyond what serde reads as an
/// integer.
pub const OUTSIDE: [(&str, &str); 12] = [
    ("byte", "256"),
    ("byte", "-1"),
    ("short", "65536"),
    ("word", "4294967296"),
    ("word", "18446744073709551616"),
    ("signed_byte", "128"),
    ("signed_byte", "-129"),
    ("signed_short", "32768"),
    ("signed_short", "-32769"),
    ("signed_word", "2147483648"),
    ("signed_word", "-2147483649"),
    ("signed_word", "-9223372036854775809"),
];

/// The members of [`Spans`] in the order it declares them, and the value each is read as when no
/// row sets it.
const MEMBERS: [(&str, &str); 6] = [
    ("byte", "0"),
    ("short", "0"),
    ("signed_byte", "0"),
    ("signed_short", "0"),
    ("signed_word", "0"),
    ("word", "0"),
];

/// The payloads read as written: each width at its least and its greatest.
pub const ENDS: [&str; 2] = [
    r#"{"byte":255,"short":65535,"signed_byte":127,"signed_short":32767,"signed_word":2147483647,"word":4294967295}"#,
    r#"{"byte":0,"short":0,"signed_byte":-128,"signed_short":-32768,"signed_word":-2147483648,"word":0}"#,
];

/// Every width of Kotlin's integers below 64 bits as a member.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Spans {
    pub byte: u8,
    pub short: u16,
    pub signed_byte: i8,
    pub signed_short: i16,
    pub signed_word: i32,
    pub word: u32,
}

/// The payload that sets `member` to `literal` and every other member to zero.
pub fn payload(member: &str, literal: &str) -> String {
    let members: Vec<String> = MEMBERS
        .iter()
        .map(|(name, zero)| {
            format!(
                "\"{name}\":{}",
                if *name == member { literal } else { zero }
            )
        })
        .collect();
    format!("{{{}}}", members.join(","))
}

/// What serde answers of `json` read as [`Spans`], its position left out, or `None` where it
/// reads it.
pub fn serde_refusal(json: &str) -> Option<String> {
    serde_json::from_str::<Spans>(json).err().map(|refused| {
        let said = refused.to_string();
        said.split_once(" at line ")
            .map_or_else(|| said.clone(), |(stated, _)| stated.to_owned())
    })
}

/// What serde writes back of `json` read as [`Spans`].
pub fn serde_written(json: &str) -> String {
    serde_json::to_string(&serde_json::from_str::<Spans>(json).unwrap()).unwrap()
}
