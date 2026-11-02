//! What `serde` adds to validation: the read that refuses a broken rule, the recovering decode that
//! hands it to its callback, and a service's request check.

use core::future::ready;

use futures::executor::block_on;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::models::{
    AccountService, AdjacentContact, Contact, Done, Handle, Pair, Profile, Roster, Signup,
    SignupError, TaggedContact, UntaggedContact, Username, account_service_schema, handle_schema,
};
use handle_schema::DecodeWith as _;

/// Welcomes every account it is handed.
struct Accounts;

impl AccountService<()> for Accounts {
    fn sign_up(
        &self,
        _ctx: &(),
        _req: Signup,
    ) -> impl Future<Output = Result<Done, SignupError>> + Send {
        ready(Ok(Done { welcomed: true }))
    }
}

/// Answers a `sign_up` payload as a dispatcher does: the read alone, the request's check, and the
/// handler only for a request that passes it.
fn dispatched(payload: &str) -> Result<Result<Done, SignupError>, Vec<String>> {
    let request =
        account_service_schema::unjudged_sign_up(&mut serde_json::Deserializer::from_str(payload))
            .map_err(|error| vec![error.to_string()])?;
    account_service_schema::validated_sign_up(&request)?;
    Ok(block_on(Accounts.sign_up(&(), request)))
}

/// What `serde_json` answers when it refuses `payload` as a `Read`.
fn refused<Read>(payload: &str) -> String
where
    Read: DeserializeOwned,
{
    serde_json::from_str::<Read>(payload)
        .err()
        .map_or_else(|| format!("{payload} was read"), |error| error.to_string())
}

#[test]
fn a_named_struct_read_refuses_its_broken_rule() {
    assert_eq!(
        refused::<Signup>(r#"{"username":"ab"}"#),
        "'username': too short: minimum length is 3, got 2"
    );
}

#[test]
fn a_tuple_struct_read_refuses_each_broken_slot() {
    assert_eq!(
        refused::<Pair>(r#"["",0]"#),
        "'0': too short: minimum length is 1, got 0; '1': too small: minimum is 1, got 0"
    );
}

#[test]
fn a_brand_read_refuses_its_broken_rule() {
    assert_eq!(
        refused::<Username>(r#""ab""#),
        "too short: minimum length is 3, got 2"
    );
}

#[test]
fn an_alias_of_a_brand_read_refuses_the_brands_rule() {
    assert_eq!(
        refused::<Profile>(r#"{"displayLogin":"ab"}"#),
        "'displayLogin': too short: minimum length is 3, got 2"
    );
}

#[test]
fn an_enum_read_of_each_tagging_refuses_its_members_broken_rule() {
    assert_eq!(
        [
            refused::<Contact>(r#"{"Email":{"address":"ab"}}"#),
            refused::<TaggedContact>(r#"{"kind":"Email","address":"ab"}"#),
            refused::<AdjacentContact>(r#"{"kind":"Email","contact":{"address":"ab"}}"#),
            refused::<UntaggedContact>(r#"{"address":"ab"}"#),
        ],
        [
            "'Email.address': too short: minimum length is 3, got 2",
            "'address': too short: minimum length is 3, got 2",
            "'contact.address': too short: minimum length is 3, got 2",
            "data did not match any variant of untagged enum UntaggedContact",
        ]
    );
}

#[test]
fn a_generic_struct_read_refuses_its_broken_rule_at_a_filling_that_is_not_its_default() {
    assert_eq!(
        refused::<Roster<u32>>(r#"{"entries":[7],"title":""}"#),
        "'title': too short: minimum length is 1, got 0"
    );
}

#[test]
fn the_recovering_decode_hands_its_callback_the_broken_rule() {
    let mut seen = Vec::new();
    let read = Handle::from_value_with(json!({ "name": "ab" }), |_raw, found| {
        seen.extend_from_slice(found);
        handle_schema::Verdict::Reject
    });
    let invalid = vec![handle_schema::Issue::Invalid {
        path: handle_schema::Path(vec![handle_schema::Segment::Key("name".to_owned())]),
        expected: handle_schema::Expected::String,
        found: Value::String("ab".to_owned()),
        reason: "too short: minimum length is 3, got 2".to_owned(),
    }];
    assert_eq!(seen, invalid);
    assert_eq!(read, Err(handle_schema::Unrecovered { issues: invalid }));
}

#[test]
fn a_service_request_check_refuses_a_broken_request() {
    let broken = Signup {
        username: "ab".to_owned(),
    };
    let short = ["'username': too short: minimum length is 3, got 2"];
    assert_eq!(
        account_service_schema::validated_sign_up(&broken).unwrap_err(),
        short
    );
    assert_eq!(dispatched(r#"{"username":"ab"}"#).unwrap_err(), short);
    assert_eq!(
        dispatched(r#"{"username":"abc"}"#),
        Ok(Ok(Done { welcomed: true }))
    );
}
