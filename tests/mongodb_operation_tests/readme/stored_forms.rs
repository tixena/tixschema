//! What the README shows of a member whose serde form depends on whether the format calls itself
//! human-readable: the generated write stores one form, and the other is refused by the read.

use core::net::{IpAddr, Ipv4Addr};

use bson::doc;
use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use super::{assert_declared_and_documented, assert_documented};
use crate::BSON_MAJOR;

/// The type the README declares, character for character.
const DECLARED_HOST: &str = "#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Host {
    pub address: IpAddr,
}";

/// What the README shows the generated write storing for a host.
const SHOWN_STORED: &str = r#"{ "address": "127.0.0.1" }"#;

/// What the README shows the form a typed collection stores told as, under version 2 of the
/// `bson` library and under version 3.
const SHOWN_UNREADABLE: [&str; 2] = [
    r#"the row without an _id does not read as expected: address: invalid: expected Unknown, found Document({"V4": Array([Int32(127), Int32(0), Int32(0), Int32(1)])}): invalid type: map, expected IP address"#,
    r#"the row without an _id does not read as expected: address: invalid: expected Unknown, found Document({"V4": Array([Int32(127), Int32(0), Int32(0), Int32(1)])}): BSON error. Kind: A deserialization-related error occurred. Message: invalid type: map, expected IP address."#,
];

#[model_schema(decode_with)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Host {
    pub address: IpAddr,
}

#[test]
fn the_readme_shows_the_two_stored_forms_of_an_address() {
    assert_declared_and_documented(include_str!("stored_forms.rs"), DECLARED_HOST);
    let host = Host {
        address: IpAddr::V4(Ipv4Addr::LOCALHOST),
    };
    let stored = host.mongo_written_row().unwrap();
    assert_eq!(stored.to_string(), SHOWN_STORED);
    assert_eq!(
        Host::mongo_read_row(stored, &[]).unwrap().address,
        host.address
    );

    let by_a_typed_collection = doc! { "address": { "V4": [127_i32, 0_i32, 0_i32, 1_i32] } };
    let refused = Host::mongo_read_row(by_a_typed_collection, &[]).unwrap_err();
    let [under_2, under_3] = SHOWN_UNREADABLE;
    assert_eq!(
        refused.to_string(),
        if BSON_MAJOR == 2 { under_2 } else { under_3 }
    );
    for shown in [SHOWN_STORED, under_2, under_3] {
        assert_documented(shown);
    }
}
