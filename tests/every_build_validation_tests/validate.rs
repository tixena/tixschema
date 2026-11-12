//! `validate()` of each shape, which every build writes whatever its features. Without `serde` no
//! `#[serde(...)]` attribute is read, so a field is named as Rust spells it, every enum is
//! externally tagged and no struct is a brand.

use crate::models::{
    AdjacentContact, Contact, Pair, Roster, Signup, TaggedContact, UntaggedContact,
};
#[cfg(feature = "serde")]
use crate::models::{Profile, Username};

/// The path `validate()` names `address` by in `Contact`, `TaggedContact`, `AdjacentContact` and
/// `UntaggedContact`.
#[cfg(feature = "serde")]
const ADDRESS_PATHS: [&str; 4] = ["Email.address", "address", "contact.address", "address"];
#[cfg(not(feature = "serde"))]
const ADDRESS_PATHS: [&str; 4] = ["Email.address"; 4];

#[test]
fn a_named_struct_reports_its_broken_rule() {
    let broken = Signup {
        username: "ab".to_owned(),
    };
    assert_eq!(
        broken.validate().unwrap_err(),
        ["'username': too short: minimum length is 3, got 2"]
    );
    let kept = Signup {
        username: "abc".to_owned(),
    };
    assert_eq!(kept.validate(), Ok(()));
}

#[test]
fn a_tuple_struct_reports_each_broken_slot() {
    assert_eq!(
        Pair(String::new(), 0).validate().unwrap_err(),
        [
            "'0': too short: minimum length is 1, got 0",
            "'1': too small: minimum is 1, got 0",
        ]
    );
    assert_eq!(Pair("a".to_owned(), 1).validate(), Ok(()));
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
    let broken = Profile {
        display_login: Username::model_schema_read_unjudged(
            serde_json::to_value("ab".to_owned()).unwrap(),
        )
        .unwrap(),
    };
    assert_eq!(
        broken.validate().unwrap_err(),
        ["'displayLogin': too short: minimum length is 3, got 2"]
    );
    let kept = Profile {
        display_login: Username::new("abc".to_owned()).unwrap(),
    };
    assert_eq!(kept.validate(), Ok(()));
}

#[test]
fn an_enum_of_each_tagging_reports_its_members_broken_rule() {
    let address = || "ab".to_owned();
    assert_eq!(
        [
            Contact::Email { address: address() }.validate(),
            TaggedContact::Email { address: address() }.validate(),
            AdjacentContact::Email { address: address() }.validate(),
            UntaggedContact::Email { address: address() }.validate(),
        ],
        ADDRESS_PATHS.map(|path| Err(vec![format!(
            "'{path}': too short: minimum length is 3, got 2"
        )]))
    );
    let kept = || "abc".to_owned();
    assert_eq!(
        [
            Contact::Email { address: kept() }.validate(),
            TaggedContact::Email { address: kept() }.validate(),
            AdjacentContact::Email { address: kept() }.validate(),
            UntaggedContact::Email { address: kept() }.validate(),
        ],
        [Ok(()), Ok(()), Ok(()), Ok(())]
    );
}

#[test]
fn a_generic_struct_reports_its_broken_rule_at_a_filling_that_is_not_its_default() {
    let broken = Roster::<u32> {
        entries: vec![7],
        title: String::new(),
    };
    assert_eq!(
        broken.validate().unwrap_err(),
        ["'title': too short: minimum length is 1, got 0"]
    );
    let kept = Roster::<u32> {
        entries: vec![7],
        title: "crew".to_owned(),
    };
    assert_eq!(kept.validate(), Ok(()));
}
