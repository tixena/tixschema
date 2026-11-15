//! The Rust surfaces of a `Vec<u8>`: what serde writes and reads, and what the other
//! descriptions say of it.

use std::collections::{HashMap, HashSet};

#[cfg(feature = "zod")]
use super::models::BytesExample;
use super::models::{
    BytesBoxDraft, BytesCountedDraft, BytesEitherDraft, BytesExampleDraft, BytesHolder,
    BytesHolderDraft, BytesImage, BytesImageDraft, BytesLookalikes, BytesLookalikesDraft,
    BytesOptionalDraft,
};

#[test]
fn serde_writes_and_reads_the_json_array() {
    let image = BytesImageDraft {
        content_type: "image/png".to_owned(),
        data: vec![137, 80, 78, 71],
    };
    let written = serde_json::to_string(&image).unwrap();
    assert_eq!(
        written,
        r#"{"contentType":"image/png","data":[137,80,78,71]}"#
    );
    assert_eq!(
        serde_json::from_str::<BytesImage>(&written).unwrap(),
        image.validate().unwrap()
    );
    let refused =
        serde_json::from_str::<BytesImage>(r#"{"contentType":"a","data":[256]}"#).unwrap_err();
    assert!(
        refused.to_string().contains("invalid value: integer `256`"),
        "{refused}"
    );
}

#[test]
fn serde_writes_a_vec_of_u8_as_the_json_array_in_every_place_a_member_holds_one() {
    let holder = BytesHolderDraft {
        aliased: vec![9],
        data: vec![137, 80, 78, 71],
        filled: BytesBoxDraft { held: vec![0, 255] }.into(),
        keyed: HashMap::from([("k".to_owned(), vec![1, 2])]),
        outcome: Ok(vec![4, 5]),
        paired: (vec![6], "x".to_owned()),
        rows: vec![vec![7], vec![]],
    };
    let written = serde_json::to_string(&holder).unwrap();
    assert_eq!(
        written,
        r#"{"aliased":[9],"data":[137,80,78,71],"filled":{"held":[0,255]},"keyed":{"k":[1,2]},"outcome":{"Ok":[4,5]},"paired":[[6],"x"],"rows":[[7],[]]}"#
    );
    assert_eq!(
        serde_json::from_str::<BytesHolder>(&written).unwrap(),
        holder.validate().unwrap()
    );
    assert_eq!(
        serde_json::to_string(&BytesEitherDraft::Bytes(vec![1, 2])).unwrap(),
        "[1,2]"
    );
    assert_eq!(
        serde_json::to_string(&BytesExampleDraft { data: vec![1] }).unwrap(),
        r#"{"data":[1]}"#
    );
    assert_eq!(
        serde_json::to_string(&BytesCountedDraft { data: vec![1] }).unwrap(),
        r#"{"data":[1]}"#
    );
    let lookalikes = BytesLookalikesDraft {
        boxed: Box::new([1]),
        digest: [1, 2, 3, 4],
        distinct: HashSet::from([1]),
        gaps: vec![None, Some(2)],
        queued: [1].into(),
        sevens: vec![7],
        signed: vec![-1],
        wide: vec![300],
    };
    assert_eq!(
        serde_json::to_string(&lookalikes).unwrap(),
        r#"{"boxed":[1],"digest":[1,2,3,4],"distinct":[1],"gaps":[null,2],"queued":[1],"sevens":[7],"signed":[-1],"wide":[300]}"#
    );
}

#[test]
fn an_optional_vec_of_u8_leaves_its_key_out_when_it_is_none() {
    assert_eq!(
        serde_json::to_string(&BytesOptionalDraft { maybe: None }).unwrap(),
        "{}"
    );
    assert_eq!(
        serde_json::to_string(&BytesOptionalDraft {
            maybe: Some(vec![3])
        })
        .unwrap(),
        r#"{"maybe":[3]}"#
    );
}

#[cfg(feature = "zod")]
#[test]
fn the_example_of_a_type_holding_bytes_is_the_array() {
    assert_eq!(
        serde_json::to_string(&BytesExample::schema_examples()).unwrap(),
        r#"[{"data":[137,80,78,71]}]"#
    );
    assert!(BytesExample::zod_schema().contains(r#"examples: [{"data":[137,80,78,71]}]"#));
}

#[cfg(not(feature = "zod"))]
#[test]
fn the_example_type_is_declared() {
    serde_json::to_string(&BytesExampleDraft { data: vec![1] }).unwrap();
}

#[cfg(feature = "zod")]
#[test]
fn validate_counts_the_bytes_as_it_counts_a_list() {
    assert_eq!(
        BytesCountedDraft { data: vec![1, 2] }.validation_issues(),
        Vec::<String>::new()
    );
    assert_eq!(
        BytesCountedDraft { data: vec![1] }.validation_issues(),
        vec!["'data': too short: minimum length is 2, got 1".to_owned()]
    );
    assert_eq!(
        BytesCountedDraft { data: vec![1; 5] }.validation_issues(),
        vec!["'data': too long: maximum length is 4, got 5".to_owned()]
    );
}

#[test]
fn the_json_schema_describes_the_array_of_bytes() {
    let described = BytesImage::json_schema();
    assert_eq!(
        described["properties"]["data"],
        serde_json::json!({ "type": "array", "items": { "type": "integer" } })
    );
    assert_eq!(
        BytesLookalikes::json_schema()["properties"]["digest"]["minItems"],
        serde_json::json!(4_i32)
    );
}
