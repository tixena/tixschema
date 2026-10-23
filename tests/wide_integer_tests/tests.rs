use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};
use core::cell::RefCell;
use std::sync::{Mutex, RwLock};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::models::{
    WideAdjacent, WideBeside, WideBorrowed, WideBounded, WideChosen, WideEvent, WideHolder, WideId,
    WideKept, WideLater, WideLocked, WidePair, WideReached, WideTagged, WideTally, WideUntagged,
};

/// The numbers serde writes for [`reached`].
const REACHED: &str = r#"{"aliased":-1152921504606846976,"aliases":[1,9223372036854775807],"fixed":214,"from":{"start":-3},"inclusive":{"start":4,"end":5},"ranged":{"start":1,"end":1152921504606846976},"resulted":{"Ok":7},"to":{"end":6},"wrong":{"Err":[8,-9]}}"#;

/// The numbers serde writes for [`tally`].
const TALLY: &str = r#"{"boxed":-1152921504606846976,"grid":[1,1152921504606846976],"keyed":{"a":-7},"listed":[0,9223372036854775807],"maybe":-9223372036854775808,"nested":[[3],null],"paired":[9,"x"],"set":[2,5],"signed":-1152921504606846976}"#;

/// An `i64` held to a literal no `f64` holds exactly.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct WideExact {
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

/// A `WideTally` with an `i64` beyond what a JavaScript `number` holds exactly in each place.
fn tally() -> WideTally {
    WideTally {
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

/// A `u64` and an `i64` member is written as the JSON number serde writes, and read back from it.
#[cfg(not(any(feature = "swift", feature = "kotlin")))]
#[test]
fn a_64_bit_member_is_written_as_a_json_number() {
    use super::models::Stamped;

    let written = serde_json::to_string(&Stamped {
        hold_bytes: 4_194_304,
        sounded_at: 1_719_624_000_000,
    })
    .unwrap();
    assert_eq!(
        written, r#"{"holdBytes":4194304,"soundedAt":1719624000000}"#,
        "serde writes each 64-bit member as a JSON number"
    );
    let read: Stamped = serde_json::from_str(&written).unwrap();
    assert_eq!(
        (read.sounded_at, read.hold_bytes),
        (1_719_624_000_000, 4_194_304),
        "serde reads each number back"
    );
}

#[test]
fn serde_writes_each_i64_a_member_holds_as_a_number_and_reads_it_back() {
    assert_eq!(written(&tally()), TALLY, "the numbers serde writes");
    assert_eq!(
        read::<WideTally>(TALLY),
        Ok(tally()),
        "the numbers serde reads"
    );
}

#[test]
fn a_missing_optional_key_reads_as_none() {
    let left_out = r#"{"boxed":1,"grid":[1,2],"keyed":{},"listed":[],"nested":[],"paired":[9,"x"],"set":[],"signed":1}"#;
    assert_eq!(
        read::<WideTally>(left_out).unwrap().maybe,
        None,
        "an absent key"
    );
    assert_eq!(
        read::<WideTally>(&left_out.replace(r#""set""#, r#""maybe":null,"set""#))
            .unwrap()
            .maybe,
        None,
        "a null"
    );
}

/// serde's own impl reads the number, and refuses one beyond the type and a string of digits in
/// its own words.
#[test]
fn serde_refuses_a_number_beyond_the_type_and_a_string_of_digits() {
    assert_eq!(
        read::<WideId>("9223372036854775808"),
        Err(
            "invalid value: integer `9223372036854775808`, expected i64 at line 1 column 19"
                .to_owned()
        ),
        "a number beyond an i64"
    );
    assert_eq!(
        read::<WideId>(r#""7""#),
        Err("invalid type: string \"7\", expected i64 at line 1 column 3".to_owned()),
        "a string of digits"
    );
    assert_eq!(
        read::<WideId>("1.5"),
        Err("invalid type: floating point `1.5`, expected i64 at line 1 column 3".to_owned()),
        "a float"
    );
}

#[test]
fn a_brand_and_a_struct_of_slots_write_their_numbers() {
    assert_eq!(written(&WideId(-3)), "-3", "a brand");
    assert_eq!(read::<WideId>("-3"), Ok(WideId(-3)), "a brand read");
    assert_eq!(
        written(&WidePair(4, "y".to_owned())),
        r#"[4,"y"]"#,
        "a struct of two slots"
    );
    assert_eq!(
        read::<WidePair>(r#"[4,"y"]"#),
        Ok(WidePair(4, "y".to_owned())),
        "a struct of two slots read"
    );
}

#[test]
fn every_variant_writes_its_numbers_under_each_tagging() {
    for (value, json) in [
        (WideEvent::Counted(5), r#"{"Counted":5}"#),
        (
            WideEvent::Moved { from: -1, to: 2 },
            r#"{"Moved":{"from":-1,"to":2}}"#,
        ),
        (WideEvent::Pair(6, "z".to_owned()), r#"{"Pair":[6,"z"]}"#),
        (WideEvent::Reset, r#""Reset""#),
    ] {
        assert_eq!(written(&value), json, "for {value:?}");
        assert_eq!(read::<WideEvent>(json), Ok(value), "for {json}");
    }
    let tagged = WideTagged::Moved { from: 1, to: 2 };
    assert_eq!(
        written(&tagged),
        r#"{"kind":"Moved","from":1,"to":2}"#,
        "under a tag"
    );
    assert_eq!(
        read::<WideTagged>(r#"{"kind":"Moved","from":1,"to":2}"#),
        Ok(tagged),
        "under a tag, read"
    );
    for (value, json) in [
        (WideAdjacent::Counted(5), r#"{"kind":"Counted","body":5}"#),
        (
            WideAdjacent::Moved { from: 1, to: 2 },
            r#"{"kind":"Moved","body":{"from":1,"to":2}}"#,
        ),
    ] {
        assert_eq!(written(&value), json, "for {value:?}");
        assert_eq!(read::<WideAdjacent>(json), Ok(value), "for {json}");
    }
}

/// The bound of an untagged member chooses the variant as it reads the number, and a string of
/// digits is the text of the variant that holds a `String`.
#[test]
fn an_untagged_member_is_read_from_its_number_and_chosen_by_its_bound() {
    assert_eq!(written(&WideUntagged::Small(4)), "4", "a slot");
    assert_eq!(
        read::<WideUntagged>("4"),
        Ok(WideUntagged::Small(4)),
        "within the bound"
    );
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    assert_eq!(
        read::<WideUntagged>("40"),
        Err("data did not match any variant of untagged enum WideUntagged".to_owned()),
        "beyond the bound"
    );
    assert_eq!(
        read::<WideUntagged>(r#""40""#),
        Ok(WideUntagged::Text("40".to_owned())),
        "a string of digits"
    );
    assert_eq!(
        written(&WideUntagged::Named { count: 12 }),
        r#"{"count":12}"#,
        "a named member"
    );
    assert_eq!(
        read::<WideUntagged>(r#"{"count":12}"#),
        Ok(WideUntagged::Named { count: 12 }),
        "a named member read"
    );
}

/// `validate()` compares the integer, and the read refuses what it refuses.
#[test]
fn a_bound_holds_the_integer_and_a_default_is_read_where_the_key_is_left_out() {
    assert_eq!(
        read::<WideBounded>(r#"{"level":-5}"#),
        Ok(WideBounded {
            level: -5,
            start: 7
        }),
        "within the bound, the default read"
    );
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    assert_eq!(
        read::<WideBounded>(r#"{"level":6,"start":1}"#),
        Err("'level': too large: maximum is 5, got 6".to_owned()),
        "beyond the bound"
    );
    assert_eq!(
        written(&WideBounded { level: 2, start: 7 }),
        r#"{"level":2,"start":7}"#,
        "the numbers written"
    );
}

/// A map's key is written as its text, and what fills another type's parameter as a number.
#[test]
fn an_i64_as_a_map_key_and_as_a_filling_is_written_as_serde_writes_it() {
    let kept = WideKept {
        filled: WideHolder { held: 3 },
        keyed: BTreeMap::from([(-4, "k".to_owned())]),
    };
    let json = r#"{"filled":{"held":3},"keyed":{"-4":"k"}}"#;
    assert_eq!(written(&kept), json, "what serde writes");
    assert_eq!(read::<WideKept>(json), Ok(kept), "what serde reads");
}

/// A `WideReached` with an `i64` in each place it holds one.
fn reached() -> WideReached {
    WideReached {
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
/// `literal` is written as a number, and read back from it.
#[test]
fn an_i64_through_an_alias_a_range_a_result_and_a_literal_travels_as_a_number() {
    assert_eq!(written(&reached()), REACHED, "the numbers serde writes");
    assert_eq!(
        read::<WideReached>(REACHED),
        Ok(reached()),
        "the numbers serde reads"
    );
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    assert_eq!(
        read::<WideReached>(&REACHED.replace(r#""fixed":214"#, r#""fixed":215"#)),
        Err("'fixed': not the literal 214, got 215".to_owned()),
        "a value beside the literal"
    );
}

/// An `i64` a lock guards below a list, a map, an `Option` and a tuple is written as a number,
/// and read back from it.
#[test]
fn an_i64_a_lock_guards_below_a_level_travels_as_a_number() {
    let locked = WideLocked {
        keyed: BTreeMap::from([("a".to_owned(), Mutex::new(-7))]),
        listed: vec![RefCell::new(i64::MAX)],
        maybe: Some(RwLock::new(i64::MIN)),
        paired: (RefCell::new(9), "x".to_owned()),
    };
    let json = r#"{"keyed":{"a":-7},"listed":[9223372036854775807],"maybe":-9223372036854775808,"paired":[9,"x"]}"#;
    assert_eq!(written(&locked), json, "the numbers serde writes");
    let read_back = read::<WideLocked>(json).unwrap();
    assert_eq!(written(&read_back), json, "the numbers serde reads");
    let taken = locked.listed[0].borrow_mut();
    assert_eq!(
        serde_json::to_string(&locked).map_err(|refused| refused.to_string()),
        Err("already mutably borrowed".to_owned()),
        "a lock not taken is refused in the words of serde's own impl"
    );
    drop(taken);
}

/// An `i64` beside the type's own parameter is written as a number, and read back from it.
#[test]
fn an_i64_beside_the_types_own_parameter_travels_as_a_number() {
    let beside = WideBeside {
        keyed: BTreeMap::from([("a".to_owned(), -7_i64)]),
        paired: ("b".to_owned(), 1_i64 << 60_i32),
    };
    let json = r#"{"keyed":{"a":-7},"paired":["b",1152921504606846976]}"#;
    assert_eq!(written(&beside), json, "the numbers serde writes");
    assert_eq!(
        read::<WideBeside<String>>(json),
        Ok(beside),
        "the numbers serde reads"
    );
}

/// An `i64` beside the type's own parameter in an untagged enum is read from its number.
#[test]
fn an_i64_beside_the_types_own_parameter_in_an_untagged_enum_is_read_from_its_number() {
    let chosen = WideChosen::Listed {
        listed: vec![(i64::MAX, "a".to_owned())],
    };
    let json = r#"{"listed":[[9223372036854775807,"a"]]}"#;
    assert_eq!(written(&chosen), json, "the numbers serde writes");
    assert_eq!(
        read::<WideChosen<String>>(json),
        Ok(chosen),
        "the numbers serde reads"
    );
    assert_eq!(
        read::<WideChosen<String>>(r#""7""#),
        Ok(WideChosen::Text("7".to_owned())),
        "a string of digits is the text"
    );
}

/// An `i64` through an alias declared below its holder is the number serde writes, and the
/// holder is described by the alias's own schema.
#[test]
fn an_i64_through_an_alias_declared_below_is_a_number() {
    let later = WideLater { revision: -7 };
    let json = r#"{"revision":-7}"#;
    assert_eq!(written(&later), json, "the number serde writes");
    assert_eq!(read::<WideLater>(json), Ok(later), "the number serde reads");
    #[cfg(feature = "typescript")]
    assert!(
        WideLater::ts_definition().contains("revision: WideLaterRevision;"),
        "the alias's name: {}",
        WideLater::ts_definition()
    );
    #[cfg(feature = "jsonschema")]
    assert_eq!(
        WideLater::json_schema()["properties"]["revision"],
        serde_json::json!({"type": "integer"}),
        "the alias's own schema"
    );
}

/// The JSON Schema describes a `u64` and an `i64` as the integer serde writes.
#[cfg(feature = "jsonschema")]
#[test]
fn the_json_schema_describes_the_integer() {
    let schema = WideTally::json_schema();
    assert_eq!(
        schema["properties"]["signed"],
        serde_json::json!({"type": "integer"}),
        "an i64"
    );
    assert_eq!(
        schema["properties"]["listed"],
        serde_json::json!({"type": "array", "items": {"type": "integer"}}),
        "a list of i64"
    );
}

/// TypeScript holds a `u64` and an `i64` as a `bigint`, and a map's key, the text serde writes,
/// as a `number`.
#[cfg(feature = "typescript")]
#[test]
fn typescript_holds_a_bigint() {
    let declared = WideTally::ts_definition();
    for expected in [
        "signed: bigint;",
        "listed: Array<bigint>;",
        "paired: [bigint, string];",
    ] {
        assert!(
            declared.contains(expected),
            "missing `{expected}` in: {declared}"
        );
    }
    assert!(
        WideKept::ts_definition().contains("keyed: Partial<Record<number, string>>;"),
        "a map's key, which no `bigint` keys: {}",
        WideKept::ts_definition()
    );
}

/// Zod reads a `u64` and an `i64` from a `bigint` or a `number` it holds exactly, as a `bigint`
/// held to the type's range, and a map's key as a `number`.
#[cfg(feature = "zod")]
#[test]
fn zod_reads_a_bigint_or_an_exact_number() {
    let schema = WideTally::zod_schema();
    let codec = "z.codec(z.union([z.bigint(), z.number().int()]), \
                 z.bigint().min(-9223372036854775808n).max(9223372036854775807n), { decode: \
                 (value) => BigInt(value), encode: (value) => value })";
    assert!(
        schema.contains(&format!("signed: {codec},")),
        "missing the codec in: {schema}"
    );
    assert!(
        WideKept::zod_schema().contains("keyed: z.record(z.number().int(), z.string()),"),
        "a map's key: {}",
        WideKept::zod_schema()
    );
}

/// An integer literal is held exactly, beyond the integers an `f64` holds.
#[test]
fn an_integer_literal_no_f64_holds_is_held_exactly() {
    let exact = WideExact {
        exact: (1_i64 << 53_i32) + 1,
    };
    let json = r#"{"exact":9007199254740993}"#;
    assert_eq!(written(&exact), json, "the number serde writes");
    assert_eq!(read::<WideExact>(json), Ok(exact), "the literal read");
    #[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
    assert_eq!(
        read::<WideExact>(r#"{"exact":9007199254740992}"#),
        Err("'exact': not the literal 9007199254740993, got 9007199254740992".to_owned()),
        "the f64 nearest the literal"
    );
    #[cfg(feature = "typescript")]
    assert!(
        WideExact::ts_definition().contains("exact: 9007199254740993n;"),
        "the TypeScript literal: {}",
        WideExact::ts_definition()
    );
    #[cfg(feature = "jsonschema")]
    assert_eq!(
        WideExact::json_schema()["properties"]["exact"],
        serde_json::json!({"type": "integer", "const": (1_i64 << 53_i32) + 1}),
        "the JSON Schema literal"
    );
}

/// An `i64` a reference borrows is written as a number.
#[test]
fn an_i64_a_reference_borrows_is_written_as_a_number() {
    let (counted, listed) = (-(1_i64 << 60_i32), [1_i64, 2_i64]);
    assert_eq!(
        written(&WideBorrowed {
            counted: &counted,
            listed: &listed,
        }),
        r#"{"counted":-1152921504606846976,"listed":[1,2]}"#,
        "the numbers serde writes"
    );
}

/// A build without `swift` and `kotlin` holds a `u64` in a member, and writes it as a number.
#[cfg(not(any(feature = "swift", feature = "kotlin")))]
#[test]
fn serde_writes_each_u64_a_member_holds_as_a_number_and_refuses_one_beyond_its_range() {
    use super::models::WideUnsigned;
    use std::collections::HashMap;

    let wide = WideUnsigned {
        big: u64::MAX,
        keyed: HashMap::from([("a".to_owned(), 1_u64 << 60_i32)]),
        listed: vec![7],
        maybe: None,
        signed: -(1_i64 << 60_i32),
    };
    let json = r#"{"big":18446744073709551615,"keyed":{"a":1152921504606846976},"listed":[7],"signed":-1152921504606846976}"#;
    assert_eq!(written(&wide), json, "the numbers serde writes");
    assert_eq!(
        read::<WideUnsigned>(json),
        Ok(wide),
        "the numbers serde reads"
    );
    assert!(
        read::<WideUnsigned>(&json.replace("18446744073709551615", "18446744073709551616"))
            .is_err(),
        "a number beyond a u64"
    );
}
