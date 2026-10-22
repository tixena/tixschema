use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::models::{
    DigitAdjacent, DigitBounded, DigitEvent, DigitHolder, DigitId, DigitKept, DigitPair,
    DigitTagged, DigitTally, DigitUntagged,
};

/// The digits serde writes for [`tally`].
const TALLY: &str = r#"{"boxed":"-1152921504606846976","grid":["1","1152921504606846976"],"keyed":{"a":"-7"},"listed":["0","9223372036854775807"],"maybe":"-9223372036854775808","nested":[["3"],null],"paired":["9","x"],"set":["2","5"],"signed":"-1152921504606846976"}"#;

/// What serde writes of `value`.
fn written<T>(value: &T) -> String
where
    T: Serialize,
{
    serde_json::to_string(value).unwrap()
}

/// What serde reads of `json` as a `T`, or its refusal.
fn read<T>(json: &str) -> Result<T, String>
where
    T: DeserializeOwned,
{
    serde_json::from_str(json).map_err(|refused| refused.to_string())
}

/// A `DigitTally` with an `i64` beyond what a JavaScript `number` holds exactly in each place.
fn tally() -> DigitTally {
    DigitTally {
        boxed: Box::new(-(1_i64 << 60_i32)),
        grid: [1_i64, 1_i64 << 60_i32],
        keyed: BTreeMap::from([("a".to_owned(), -7)]),
        listed: vec![0, i64::MAX],
        maybe: Some(i64::MIN),
        nested: vec![Some(vec![3]), None],
        paired: (9, "x".to_owned()),
        set: BTreeSet::from([2, 5]),
        signed: -(1_i64 << 60_i32),
    }
}

#[test]
fn serde_writes_each_i64_a_member_holds_as_its_digits_and_reads_them_back() {
    assert_eq!(written(&tally()), TALLY, "the digits serde writes");
    assert_eq!(
        read::<DigitTally>(TALLY),
        Ok(tally()),
        "the digits serde reads"
    );
}

#[test]
fn serde_reads_a_number_where_it_writes_digits() {
    let numbers = r#"{"boxed":-1152921504606846976,"grid":[1,1152921504606846976],"keyed":{"a":-7},"listed":[0,9223372036854775807],"maybe":-9223372036854775808,"nested":[[3],null],"paired":[9,"x"],"set":[2,5],"signed":-1152921504606846976}"#;
    assert_eq!(
        read::<DigitTally>(numbers),
        Ok(tally()),
        "the numbers serde reads"
    );
}

#[test]
fn a_missing_optional_key_reads_as_none() {
    let left_out = r#"{"boxed":"1","grid":["1","2"],"keyed":{},"listed":[],"nested":[],"paired":["9","x"],"set":[],"signed":"1"}"#;
    assert_eq!(
        read::<DigitTally>(left_out).unwrap().maybe,
        None,
        "an absent key"
    );
    assert_eq!(
        read::<DigitTally>(&left_out.replace(r#""set""#, r#""maybe":null,"set""#))
            .unwrap()
            .maybe,
        None,
        "a null"
    );
}

#[test]
fn serde_refuses_a_digit_string_outside_the_range_of_the_type() {
    assert_eq!(
        read::<DigitId>(r#""9223372036854775808""#),
        Err(
            "invalid value: string \"9223372036854775808\", expected an i64 as a string of its \
             decimal digits at line 1 column 21"
                .to_owned()
        ),
        "a digit string beyond an i64"
    );
    assert_eq!(
        read::<DigitId>(r#""seven""#),
        Err(
            "invalid value: string \"seven\", expected an i64 as a string of its decimal digits \
             at line 1 column 7"
                .to_owned()
        ),
        "a text of no digits"
    );
    assert_eq!(
        read::<DigitId>("1.5"),
        Err(
            "invalid type: floating point `1.5`, expected an i64 as a string of its decimal \
             digits at line 1 column 3"
                .to_owned()
        ),
        "a float"
    );
}

#[test]
fn a_brand_and_a_struct_of_slots_write_their_digits() {
    assert_eq!(written(&DigitId(-3)), r#""-3""#, "a brand");
    assert_eq!(read::<DigitId>(r#""-3""#), Ok(DigitId(-3)), "a brand read");
    assert_eq!(
        written(&DigitPair(4, "y".to_owned())),
        r#"["4","y"]"#,
        "a struct of two slots"
    );
    assert_eq!(
        read::<DigitPair>(r#"["4","y"]"#),
        Ok(DigitPair(4, "y".to_owned())),
        "a struct of two slots read"
    );
}

#[test]
fn every_variant_writes_its_digits_under_each_tagging() {
    for (value, json) in [
        (DigitEvent::Counted(5), r#"{"Counted":"5"}"#),
        (
            DigitEvent::Moved { from: -1, to: 2 },
            r#"{"Moved":{"from":"-1","to":"2"}}"#,
        ),
        (DigitEvent::Pair(6, "z".to_owned()), r#"{"Pair":["6","z"]}"#),
        (DigitEvent::Reset, r#""Reset""#),
    ] {
        assert_eq!(written(&value), json, "for {value:?}");
        assert_eq!(read::<DigitEvent>(json), Ok(value), "for {json}");
    }
    let tagged = DigitTagged::Moved { from: 1, to: 2 };
    assert_eq!(
        written(&tagged),
        r#"{"kind":"Moved","from":"1","to":"2"}"#,
        "under a tag"
    );
    assert_eq!(
        read::<DigitTagged>(r#"{"kind":"Moved","from":"1","to":"2"}"#),
        Ok(tagged),
        "under a tag, read"
    );
    for (value, json) in [
        (
            DigitAdjacent::Counted(5),
            r#"{"kind":"Counted","body":"5"}"#,
        ),
        (
            DigitAdjacent::Moved { from: 1, to: 2 },
            r#"{"kind":"Moved","body":{"from":"1","to":"2"}}"#,
        ),
    ] {
        assert_eq!(written(&value), json, "for {value:?}");
        assert_eq!(read::<DigitAdjacent>(json), Ok(value), "for {json}");
    }
}

/// The bound of an untagged member chooses the variant as it reads the digit string, and a
/// member's digits are read in a variant chosen by its keys.
#[test]
fn an_untagged_member_is_read_from_its_digits_and_chosen_by_its_bound() {
    assert_eq!(written(&DigitUntagged::Small(4)), r#""4""#, "a slot");
    assert_eq!(
        read::<DigitUntagged>(r#""4""#),
        Ok(DigitUntagged::Small(4)),
        "within the bound"
    );
    assert_eq!(
        read::<DigitUntagged>(r#""40""#),
        Ok(DigitUntagged::Text("40".to_owned())),
        "beyond the bound"
    );
    assert_eq!(
        written(&DigitUntagged::Named { count: 12 }),
        r#"{"count":"12"}"#,
        "a named member"
    );
    assert_eq!(
        read::<DigitUntagged>(r#"{"count":"12"}"#),
        Ok(DigitUntagged::Named { count: 12 }),
        "a named member read"
    );
}

/// `validate()` compares the integer, and the read refuses what it refuses.
#[test]
fn a_bound_holds_the_integer_and_a_default_is_read_where_the_key_is_left_out() {
    assert_eq!(
        read::<DigitBounded>(r#"{"level":"-5"}"#),
        Ok(DigitBounded {
            level: -5,
            start: 7
        }),
        "within the bound, the default read"
    );
    assert_eq!(
        read::<DigitBounded>(r#"{"level":"6","start":"1"}"#),
        Err("'level': too large: maximum is 5, got 6".to_owned()),
        "beyond the bound"
    );
    assert_eq!(
        written(&DigitBounded { level: 2, start: 7 }),
        r#"{"level":"2","start":"7"}"#,
        "the digits written"
    );
}

/// A map's key, a range's bound, a `Result`'s arm, what fills another type's parameter and a
/// member under a `literal` keep the number serde's own impl writes.
#[test]
fn an_i64_another_impl_writes_keeps_the_number() {
    let kept = DigitKept {
        filled: DigitHolder { held: 3 },
        fixed: 214,
        keyed: BTreeMap::from([(-4, "k".to_owned())]),
        ranged: 1..2,
        resulted: Ok(5),
    };
    let json = r#"{"filled":{"held":3},"fixed":214,"keyed":{"-4":"k"},"ranged":{"start":1,"end":2},"resulted":{"Ok":5}}"#;
    assert_eq!(written(&kept), json, "the numbers serde writes");
    assert_eq!(read::<DigitKept>(json), Ok(kept), "the numbers serde reads");
}

/// A build without `swift` and `kotlin` holds a `u64` in a member, and writes it as its digits.
#[cfg(not(any(feature = "swift", feature = "kotlin")))]
#[test]
fn serde_writes_each_u64_a_member_holds_as_its_digits_and_refuses_one_beyond_its_range() {
    use super::models::DigitWide;
    use std::collections::HashMap;

    let wide = DigitWide {
        big: u64::MAX,
        keyed: HashMap::from([("a".to_owned(), 1_u64 << 60_i32)]),
        listed: vec![7],
        maybe: None,
        signed: -(1_i64 << 60_i32),
    };
    let json = r#"{"big":"18446744073709551615","keyed":{"a":"1152921504606846976"},"listed":["7"],"signed":"-1152921504606846976"}"#;
    assert_eq!(written(&wide), json, "the digits serde writes");
    assert_eq!(read::<DigitWide>(json), Ok(wide), "the digits serde reads");
    assert_eq!(
        read::<DigitWide>(&json.replace("18446744073709551615", "18446744073709551616")),
        Err(
            "invalid value: string \"18446744073709551616\", expected a u64 as a string of its \
             decimal digits at line 1 column 29"
                .to_owned()
        ),
        "a digit string beyond a u64"
    );
}
