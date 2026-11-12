//! A `usize` as a member, under a bound, as a map's key and as a range's bound, and in each other
//! place [`Placed`] holds one, written as the JSON number serde writes on a 64-bit host. Each client
//! reads each payload serde writes and writes it back as serde does, the values above `i64::MAX`
//! included, refuses each payload serde refuses, and its `validate()` answers what the Rust
//! `validate()` answers.

use alloc::collections::BTreeMap;
use core::ops::Range;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::usize_positions_models::Placed;

/// Each payload serde writes, and the type it is read as: the bottom of the range, `i64::MAX`, the
/// value above it, and `usize::MAX`, in each place the type holds a `usize`.
pub const WRITTEN: [(&str, &str); 6] = [
    (
        "Offsets",
        r#"{"at":0,"bounded":1,"by_index":{"0":"a"},"span":{"start":0,"end":0}}"#,
    ),
    (
        "Offsets",
        r#"{"at":9223372036854775807,"bounded":4096,"by_index":{"9223372036854775807":"b"},"span":{"start":0,"end":9223372036854775807}}"#,
    ),
    (
        "Offsets",
        r#"{"at":9223372036854775808,"bounded":12,"by_index":{"9223372036854775808":"c"},"span":{"start":9223372036854775808,"end":9223372036854775808}}"#,
    ),
    (
        "Offsets",
        r#"{"at":18446744073709551615,"bounded":12,"by_index":{"18446744073709551615":"d"},"span":{"start":8,"end":18446744073709551615}}"#,
    ),
    (
        "Placed",
        r#"{"at":9223372036854775808,"defaulted":9223372036854775808,"held":{"held":18446744073709551615},"indexed":{"18446744073709551615":"x"},"keyed":{"a":9223372036854775808},"listed":[0,9223372036854775807,9223372036854775808,18446744073709551615],"maybe":18446744073709551615,"paired":[18446744073709551615,"y"],"pinned":18446744073709551615,"slot":18446744073709551615,"span":{"start":9223372036854775808,"end":18446744073709551615}}"#,
    ),
    (
        "Placed",
        r#"{"at":0,"defaulted":0,"held":{"held":0},"indexed":{},"keyed":{},"listed":[],"paired":[0,"z"],"pinned":18446744073709551615,"slot":0,"span":{"start":0,"end":0}}"#,
    ),
];

/// Each payload serde refuses, the type it is read as, and what serde answers, its position left
/// out: a negative number, one past `usize::MAX`, a fraction, a value on each side of the bound,
/// and a value the `literal` refuses.
pub const REFUSED: [(&str, &str, &str); 6] = [
    (
        "Offsets",
        r#"{"at":-1,"bounded":12,"by_index":{},"span":{"start":0,"end":0}}"#,
        "invalid value: integer `-1`, expected usize",
    ),
    (
        "Offsets",
        r#"{"at":18446744073709551616,"bounded":12,"by_index":{},"span":{"start":0,"end":0}}"#,
        "invalid type: floating point `1.8446744073709552e+19`, expected usize",
    ),
    (
        "Offsets",
        r#"{"at":1.5,"bounded":12,"by_index":{},"span":{"start":0,"end":0}}"#,
        "invalid type: floating point `1.5`, expected usize",
    ),
    (
        "Offsets",
        r#"{"at":0,"bounded":0,"by_index":{},"span":{"start":0,"end":0}}"#,
        "'bounded': too small: minimum is 1, got 0",
    ),
    (
        "Offsets",
        r#"{"at":0,"bounded":4097,"by_index":{},"span":{"start":0,"end":0}}"#,
        "'bounded': too large: maximum is 4096, got 4097",
    ),
    (
        "Placed",
        r#"{"at":0,"defaulted":0,"held":{"held":0},"indexed":{},"keyed":{},"listed":[],"paired":[0,"z"],"pinned":9223372036854775808,"slot":0,"span":{"start":0,"end":0}}"#,
        "'pinned': not the literal 18446744073709551615, got 9223372036854775808",
    ),
];

/// A [`Placed`] that leaves `defaulted` out, which every read takes as `usize::MAX`.
pub const DEFAULTED: &str = r#"{"at":0,"held":{"held":0},"indexed":{},"keyed":{},"listed":[],"paired":[0,"z"],"pinned":18446744073709551615,"slot":0,"span":{"start":0,"end":0}}"#;

/// The values of `bounded` the Rust `validate()` refuses.
pub const OUT_OF_BOUNDS: [usize; 2] = [0, 4097];

/// A `usize` as a member, under a bound, as a map's key and as a range's bound.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Offsets {
    pub at: usize,
    #[model_schema_prop(minimum = 1, maximum = 4096)]
    pub bounded: usize,
    pub by_index: BTreeMap<usize, String>,
    pub span: Range<usize>,
}

/// What serde writes back of `json` read as the type `read_as` names.
pub fn serde_written(read_as: &str, json: &str) -> String {
    if read_as == "Placed" {
        written_back::<Placed>(json)
    } else {
        written_back::<Offsets>(json)
    }
}

/// What serde answers of `json` read as the type `read_as` names, its position left out, or
/// `None` where it reads it.
pub fn serde_refusal(read_as: &str, json: &str) -> Option<String> {
    if read_as == "Placed" {
        refusal_of::<Placed>(json)
    } else {
        refusal_of::<Offsets>(json)
    }
}

/// What serde reads `defaulted` of [`DEFAULTED`] as.
pub fn serde_defaulted() -> usize {
    serde_json::from_str::<Placed>(DEFAULTED).unwrap().defaulted
}

/// What the Rust `validate()` answers of an [`Offsets`] whose `bounded` is `bounded`, its messages
/// joined as each client's read joins them.
pub fn rust_violations(bounded: usize) -> String {
    let offsets = OffsetsDraft {
        at: 0,
        bounded,
        by_index: BTreeMap::new(),
        span: 0..0,
    };
    offsets.validation_issues().join("; ")
}

/// What serde writes back of `json` read as `T`.
fn written_back<T>(json: &str) -> String
where
    T: DeserializeOwned + Serialize,
{
    serde_json::to_string(&serde_json::from_str::<T>(json).unwrap()).unwrap()
}

/// What serde answers of `json` read as `T`, or `None` where it reads it.
fn refusal_of<T>(json: &str) -> Option<String>
where
    T: DeserializeOwned,
{
    serde_json::from_str::<T>(json).err().map(|refused| {
        let said = refused.to_string();
        said.split_once(" at line ")
            .map_or_else(|| said.clone(), |(stated, _)| stated.to_owned())
    })
}

/// serde reads each payload of [`WRITTEN`] and writes it back as it was, and refuses each of
/// [`REFUSED`] in the words beside it.
#[test]
fn serde_reads_and_refuses_each_usize_payload_as_the_rows_state() {
    for (read_as, json) in WRITTEN {
        assert_eq!(serde_refusal(read_as, json), None, "serde refuses {json}");
        assert_eq!(
            serde_written(read_as, json),
            json,
            "serde writes {json} back"
        );
    }
    for (read_as, json, said) in REFUSED {
        assert_eq!(
            serde_refusal(read_as, json).as_deref(),
            Some(said),
            "serde's refusal of {json}"
        );
    }
    assert_eq!(serde_defaulted(), usize::MAX);
}

/// A `usize::MAX` is written as the twenty digits of the number, and read back.
#[test]
fn serde_writes_usize_max_as_its_digits() {
    let offsets = OffsetsDraft {
        at: usize::MAX,
        bounded: 12,
        by_index: BTreeMap::from([(3, "c".to_owned())]),
        span: 8..13,
    };
    let written = serde_json::to_string(&offsets).unwrap();
    assert_eq!(
        written,
        r#"{"at":18446744073709551615,"bounded":12,"by_index":{"3":"c"},"span":{"start":8,"end":13}}"#
    );
    assert_eq!(
        serde_json::from_str::<Offsets>(&written).unwrap(),
        offsets.validate().unwrap()
    );
}
