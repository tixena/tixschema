//! `validate()` of each shape, which every build writes whatever its features. Without `serde` no
//! `#[serde(...)]` attribute is read, so a field is named as Rust spells it, every enum is
//! externally tagged and no struct is a brand.

use crate::models::{
    AdjacentContactDraft, ContactDraft, PairDraft, RosterDraft, SignupDraft, TaggedContactDraft,
    UntaggedContactDraft,
};
#[cfg(feature = "serde")]
use crate::models::{ProfileDraft, Username};

/// The path `validate()` names `address` by in `Contact`, `TaggedContact`, `AdjacentContact` and
/// `UntaggedContact`.
#[cfg(feature = "serde")]
const ADDRESS_PATHS: [&str; 4] = ["Email.address", "address", "contact.address", "address"];
#[cfg(not(feature = "serde"))]
const ADDRESS_PATHS: [&str; 4] = ["Email.address"; 4];

#[test]
fn a_named_struct_reports_its_broken_rule() {
    let broken = SignupDraft {
        username: "ab".to_owned(),
    };
    assert_eq!(
        broken.validation_issues(),
        ["'username': too short: minimum length is 3, got 2"]
    );
    let kept = SignupDraft {
        username: "abc".to_owned(),
    };
    assert_eq!(kept.validation_issues(), Vec::<String>::new());
}

#[test]
fn a_tuple_struct_reports_each_broken_slot() {
    assert_eq!(
        PairDraft(String::new(), 0).validation_issues(),
        [
            "'0': too short: minimum length is 1, got 0",
            "'1': too small: minimum is 1, got 0",
        ]
    );
    assert_eq!(
        PairDraft("a".to_owned(), 1).validation_issues(),
        Vec::<String>::new()
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_brand_reports_its_broken_rule() {
    assert_eq!(
        Username::new("ab".to_owned()).map(|_| ()).unwrap_err(),
        ["too short: minimum length is 3, got 2"]
    );
    assert_eq!(Username::new("abc".to_owned()).map(|_| ()), Ok(()));
}

#[cfg(feature = "serde")]
#[test]
fn a_field_typed_with_an_alias_of_a_brand_reports_the_brands_rule() {
    let broken = ProfileDraft {
        display_login: Username::model_schema_read_unjudged(
            serde_json::to_value("ab".to_owned()).unwrap(),
        )
        .unwrap(),
    };
    assert_eq!(
        broken.validation_issues(),
        ["'displayLogin': too short: minimum length is 3, got 2"]
    );
    let kept = ProfileDraft {
        display_login: Username::new("abc".to_owned()).unwrap(),
    };
    assert_eq!(kept.validation_issues(), Vec::<String>::new());
}

#[test]
fn an_enum_of_each_tagging_reports_its_members_broken_rule() {
    let address = || "ab".to_owned();
    assert_eq!(
        [
            ContactDraft::Email { address: address() }
                .validate()
                .map(|_| ()),
            TaggedContactDraft::Email { address: address() }
                .validate()
                .map(|_| ()),
            AdjacentContactDraft::Email { address: address() }
                .validate()
                .map(|_| ()),
            UntaggedContactDraft::Email { address: address() }
                .validate()
                .map(|_| ()),
        ],
        ADDRESS_PATHS.map(|path| Err(vec![format!(
            "'{path}': too short: minimum length is 3, got 2"
        )]))
    );
    let kept = || "abc".to_owned();
    assert_eq!(
        [
            ContactDraft::Email { address: kept() }
                .validate()
                .map(|_| ()),
            TaggedContactDraft::Email { address: kept() }
                .validate()
                .map(|_| ()),
            AdjacentContactDraft::Email { address: kept() }
                .validate()
                .map(|_| ()),
            UntaggedContactDraft::Email { address: kept() }
                .validate()
                .map(|_| ()),
        ],
        [Ok(()), Ok(()), Ok(()), Ok(())]
    );
}

#[test]
fn a_generic_struct_reports_its_broken_rule_at_a_filling_that_is_not_its_default() {
    let broken = RosterDraft::<u32> {
        entries: vec![7],
        title: String::new(),
    };
    assert_eq!(
        broken.validation_issues(),
        ["'title': too short: minimum length is 1, got 0"]
    );
    let kept = RosterDraft::<u32> {
        entries: vec![7],
        title: "crew".to_owned(),
    };
    assert_eq!(kept.validation_issues(), Vec::<String>::new());
}
