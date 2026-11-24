//! A jiff `Timestamp` stored as a BSON date through version 3's `FromJiff02Timestamp`, the one
//! converter of a jiff date the `bson` library has: serde stores it as a BSON date, the recovering
//! decode reads it back clean, and a typed filter on it writes a BSON date. A `Timestamp` with no
//! hook is stored as the text jiff writes.

#![cfg(all(feature = "jiff", feature = "mongodb"))]

tixschema::decode_with_foundation!();
tixschema::mongodb_foundation!();

#[cfg(test)]
mod tests {
    use bson::serde_helpers::datetime::FromJiff02Timestamp as date_hook;
    use bson::{Bson, Document};
    use closing_schema::DecodeWith as _;
    use jiff::Timestamp;
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema(enable = ["decode_with", "mongodb"])]
    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Closing {
        /// No hook: jiff's own `Serialize` writes this one as text.
        announced: Timestamp,
        #[serde(with = "date_hook")]
        closes_at: Timestamp,
    }

    /// The instant every case writes: `2026-10-07T17:40:00Z`.
    fn now() -> Timestamp {
        "2026-10-07T17:40:00Z".parse().unwrap()
    }

    /// The BSON date of [`now`].
    fn stored_now() -> Bson {
        Bson::DateTime(bson::DateTime::from_millis(1_791_394_800_000))
    }

    fn stored() -> Document {
        bson::serialize_to_document(&ClosingDraft {
            announced: now(),
            closes_at: now(),
        })
        .unwrap()
    }

    #[test]
    fn a_hooked_timestamp_is_stored_as_a_bson_date_and_one_with_no_hook_as_text() {
        let row = stored();
        assert_eq!(row.get("closesAt"), Some(&stored_now()));
        assert_eq!(
            row.get("announced"),
            Some(&Bson::String("2026-10-07T17:40:00Z".to_owned()))
        );
    }

    #[test]
    fn the_recovering_decode_reads_a_stored_bson_date_clean() {
        let mut calls = 0_u32;
        let read = Closing::from_bson_with(stored(), |_raw, _found| {
            calls += 1;
            closing_schema::Verdict::Reject
        });
        assert_eq!(
            read,
            Ok(ClosingDraft {
                announced: now(),
                closes_at: now(),
            }
            .validate()
            .unwrap())
        );
        assert_eq!(calls, 0);
    }

    #[test]
    fn a_typed_filter_on_a_hooked_timestamp_writes_a_bson_date() {
        let filter = Closing::MONGO_FIELDS
            .closes_at()
            .lt(now())
            .unwrap()
            .into_document();
        assert_eq!(
            filter.get_document("closesAt").unwrap().get("$lt"),
            Some(&stored_now())
        );
        let unhooked = Closing::MONGO_FIELDS
            .announced()
            .eq(now())
            .unwrap()
            .into_document();
        assert_eq!(
            unhooked.get_document("announced").unwrap().get("$eq"),
            stored().get("announced")
        );
    }
}
