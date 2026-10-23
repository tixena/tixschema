use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};
use core::cell::RefCell;
use std::sync::{Mutex, RwLock};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::models::{
    DigitAdjacent, DigitBeside, DigitBorrowed, DigitBounded, DigitChosen, DigitEvent, DigitHolder,
    DigitId, DigitKept, DigitLater, DigitLocked, DigitPair, DigitReached, DigitTagged, DigitTally,
    DigitUntagged,
};

/// The digits serde writes for [`reached`].
const REACHED: &str = r#"{"aliased":"-1152921504606846976","aliases":["1","9223372036854775807"],"fixed":"214","from":{"start":"-3"},"inclusive":{"start":"4","end":"5"},"ranged":{"start":"1","end":"1152921504606846976"},"resulted":{"Ok":"7"},"to":{"end":"6"},"wrong":{"Err":["8","-9"]}}"#;

/// The digits serde writes for [`tally`].
const TALLY: &str = r#"{"boxed":"-1152921504606846976","grid":["1","1152921504606846976"],"keyed":{"a":"-7"},"listed":["0","9223372036854775807"],"maybe":"-9223372036854775808","nested":[["3"],null],"paired":["9","x"],"set":["2","5"],"signed":"-1152921504606846976"}"#;

/// An `i64` held to a literal no `f64` holds exactly.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DigitExact {
    #[model_schema_prop(literal = 9007199254740993)]
    pub exact: i64,
}

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
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
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
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
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

/// A map's key and what fills another type's parameter keep the number serde's own impl writes.
#[test]
fn an_i64_another_impl_writes_keeps_the_number() {
    let kept = DigitKept {
        filled: DigitHolder { held: 3 },
        keyed: BTreeMap::from([(-4, "k".to_owned())]),
    };
    let json = r#"{"filled":{"held":3},"keyed":{"-4":"k"}}"#;
    assert_eq!(written(&kept), json, "the numbers serde writes");
    assert_eq!(read::<DigitKept>(json), Ok(kept), "the numbers serde reads");
}

/// A `DigitReached` with an `i64` in each place it holds one.
fn reached() -> DigitReached {
    DigitReached {
        aliased: -(1_i64 << 60_i32),
        aliases: Some(vec![1, i64::MAX]),
        fixed: 214,
        from: -3..,
        inclusive: 4..=5,
        ranged: 1..(1_i64 << 60_i32),
        resulted: Ok(7),
        to: ..6,
        wrong: Err(vec![8, -9]),
    }
}

/// An `i64` through an alias declared above, as a range's bound, as a `Result`'s arm and under a
/// `literal` is written as its digits, and read back from them and from a number alike.
#[test]
fn an_i64_through_an_alias_a_range_a_result_and_a_literal_travels_as_its_digits() {
    assert_eq!(written(&reached()), REACHED, "the digits serde writes");
    assert_eq!(
        read::<DigitReached>(REACHED),
        Ok(reached()),
        "the digits serde reads"
    );
    let numbers = r#"{"aliased":-1152921504606846976,"aliases":[1,9223372036854775807],"fixed":214,"from":{"start":-3},"inclusive":{"start":4,"end":5},"ranged":{"start":1,"end":1152921504606846976},"resulted":{"Ok":7},"to":{"end":6},"wrong":{"Err":[8,-9]}}"#;
    assert_eq!(
        read::<DigitReached>(numbers),
        Ok(reached()),
        "the numbers serde reads"
    );
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    assert_eq!(
        read::<DigitReached>(&REACHED.replace(r#""fixed":"214""#, r#""fixed":"215""#)),
        Err("'fixed': not the literal 214, got 215".to_owned()),
        "a value beside the literal"
    );
}

/// An `i64` a lock guards below a list, a map, an `Option` and a tuple is written as its digits,
/// and read back from them.
#[test]
fn an_i64_a_lock_guards_below_a_level_travels_as_its_digits() {
    let locked = DigitLocked {
        keyed: BTreeMap::from([("a".to_owned(), Mutex::new(-7))]),
        listed: vec![RefCell::new(i64::MAX)],
        maybe: Some(RwLock::new(i64::MIN)),
        paired: (RefCell::new(9), "x".to_owned()),
    };
    let json = r#"{"keyed":{"a":"-7"},"listed":["9223372036854775807"],"maybe":"-9223372036854775808","paired":["9","x"]}"#;
    assert_eq!(written(&locked), json, "the digits serde writes");
    let read_back = read::<DigitLocked>(json).unwrap();
    assert_eq!(written(&read_back), json, "the digits serde reads");
    let taken = locked.listed[0].borrow_mut();
    assert_eq!(
        serde_json::to_string(&locked).map_err(|refused| refused.to_string()),
        Err("already mutably borrowed".to_owned()),
        "a lock not taken is refused in the words of serde's own impl"
    );
    drop(taken);
}

/// An `i64` beside the type's own parameter is written as its digits, and read back from them.
#[test]
fn an_i64_beside_the_types_own_parameter_travels_as_its_digits() {
    let beside = DigitBeside {
        keyed: BTreeMap::from([("a".to_owned(), -7_i64)]),
        paired: ("b".to_owned(), 1_i64 << 60_i32),
    };
    let json = r#"{"keyed":{"a":"-7"},"paired":["b","1152921504606846976"]}"#;
    assert_eq!(written(&beside), json, "the digits serde writes");
    assert_eq!(
        read::<DigitBeside<String>>(json),
        Ok(beside),
        "the digits serde reads"
    );
}

/// An `i64` beside the type's own parameter in an untagged enum is read from its digits by the
/// read that holds its member to the count that chooses the variant.
#[test]
fn an_i64_beside_the_types_own_parameter_in_an_untagged_enum_is_counted_as_it_is_read() {
    let chosen = DigitChosen::Listed {
        listed: vec![(i64::MAX, "a".to_owned())],
    };
    let json = r#"{"listed":[["9223372036854775807","a"]]}"#;
    assert_eq!(written(&chosen), json, "the digits serde writes");
    assert_eq!(
        read::<DigitChosen<String>>(json),
        Ok(chosen),
        "the digits serde reads"
    );
    assert_eq!(
        read::<DigitChosen<String>>(r#"{"listed":[[7,"a"]]}"#),
        Ok(DigitChosen::Listed {
            listed: vec![(7, "a".to_owned())],
        }),
        "the number serde reads"
    );
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    assert_eq!(
        read::<DigitChosen<String>>(r#"{"listed":[]}"#),
        Err("data did not match any variant of untagged enum DigitChosen".to_owned()),
        "a list the count refuses chooses no variant"
    );
}

/// An `i64` through an alias declared below its holder keeps the number serde writes, and the
/// holder is described by the alias's own schema.
#[test]
fn an_i64_through_an_alias_declared_below_keeps_the_number() {
    let later = DigitLater { revision: -7 };
    let json = r#"{"revision":-7}"#;
    assert_eq!(written(&later), json, "the number serde writes");
    assert_eq!(
        read::<DigitLater>(json),
        Ok(later),
        "the number serde reads"
    );
    #[cfg(feature = "typescript")]
    assert!(
        DigitLater::ts_definition().contains("revision: DigitLaterRevision;"),
        "the alias's name: {}",
        DigitLater::ts_definition()
    );
    #[cfg(feature = "jsonschema")]
    assert_eq!(
        DigitLater::json_schema()["properties"]["revision"],
        serde_json::json!({"type": "integer"}),
        "the alias's own schema"
    );
}

/// An integer literal is held exactly, beyond the integers an `f64` holds.
#[test]
fn an_integer_literal_no_f64_holds_is_held_exactly() {
    let exact = DigitExact {
        exact: (1_i64 << 53_i32) + 1,
    };
    let json = r#"{"exact":"9007199254740993"}"#;
    assert_eq!(written(&exact), json, "the digits serde writes");
    assert_eq!(read::<DigitExact>(json), Ok(exact), "the literal read");
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    assert_eq!(
        read::<DigitExact>(r#"{"exact":"9007199254740992"}"#),
        Err("'exact': not the literal 9007199254740993, got 9007199254740992".to_owned()),
        "the f64 nearest the literal"
    );
    #[cfg(feature = "typescript")]
    assert!(
        DigitExact::ts_definition().contains("exact: 9007199254740993n;"),
        "the TypeScript literal: {}",
        DigitExact::ts_definition()
    );
    #[cfg(feature = "jsonschema")]
    assert_eq!(
        DigitExact::json_schema()["properties"]["exact"],
        serde_json::json!({"type": "string", "const": "9007199254740993"}),
        "the JSON Schema literal"
    );
}

/// An `i64` a reference borrows is written as its digits.
#[test]
fn an_i64_a_reference_borrows_is_written_as_its_digits() {
    let (counted, listed) = (-(1_i64 << 60_i32), [1_i64, 2_i64]);
    assert_eq!(
        written(&DigitBorrowed {
            counted: &counted,
            listed: &listed,
        }),
        r#"{"counted":"-1152921504606846976","listed":["1","2"]}"#,
        "the digits serde writes"
    );
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
