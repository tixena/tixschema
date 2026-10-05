//! Flagged types whose serde hooks are functions named like the values their walkers bind: a
//! parameter (`path`, `out`), what a key holds (`held`), what a writer is handed (`read`). Each
//! hook goes on naming its author's function.

use core::fmt::Display;

use serde::de::Error as _;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tixschema::model_schema;

use super::lines;

/// What `from_value_with` lists for `$stored`, read as `$model` by a decider that rejects it.
macro_rules! listed {
    ($model:ty, $module:ident, $stored:expr) => {
        lines(
            &<$model>::from_value_with($stored, |_raw, _found| $module::Verdict::Reject)
                .unwrap_err(),
        )
    };
}

/// What `from_value_with` reads `$stored` as, by a decider that counts each run of its own into
/// `$calls`.
macro_rules! read_counting {
    ($model:ty, $module:ident, $stored:expr, $calls:ident) => {
        <$model>::from_value_with($stored, |_raw, _found| {
            $calls += 1;
            $module::Verdict::Reject
        })
    };
}

/// One read hook per name: text, refused where it is empty under the hook's own name.
macro_rules! read_hooks {
    ($($name:ident),+) => {$(
        fn $name<'de, D>(deserializer: D) -> Result<String, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let text = String::deserialize(deserializer)?;
            if text.is_empty() {
                return Err(D::Error::custom(concat!(
                    "`",
                    stringify!($name),
                    "` reads no empty text"
                )));
            }
            Ok(text)
        }
    )+};
}

/// What [`item`] reads: a count, or text that holds one.
#[derive(Deserialize)]
#[serde(untagged)]
enum Count {
    Number(u32),
    Text(String),
}

/// A read hook named like what the walker binds the value of a key as.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Named {
    #[serde(deserialize_with = "held")]
    name: String,
}

/// One field per name the walker gives a value of its own, each read by the hook of that name.
/// `stored` is bound where a key has an alias, and `read` is what a writer is handed.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Bound {
    #[serde(deserialize_with = "found")]
    found: String,
    #[serde(deserialize_with = "held")]
    held: String,
    #[serde(deserialize_with = "issue")]
    issue: String,
    #[serde(deserialize_with = "item", serialize_with = "read")]
    item: u32,
    #[serde(alias = "id", deserialize_with = "key")]
    key: String,
    #[serde(deserialize_with = "object")]
    object: String,
    #[serde(deserialize_with = "out")]
    out: String,
    #[serde(deserialize_with = "path")]
    path: String,
    #[serde(alias = "saved", deserialize_with = "stored")]
    stored: String,
}

/// A tuple struct's walker binds the value as `found`, and its positions as `items`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Placed(
    #[serde(deserialize_with = "found")] String,
    #[serde(deserialize_with = "items")] String,
);

/// Internally tagged: the tag is bound as `tag` where the variant's fields are walked.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind")]
enum Internal {
    Note {
        #[serde(deserialize_with = "content")]
        content: String,
        #[serde(deserialize_with = "inner")]
        inner: String,
        #[serde(deserialize_with = "tag")]
        tag: String,
    },
}

/// Adjacently tagged: what the variant holds is bound as `content`, and its object as `inner`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "body")]
enum Adjacent {
    Note {
        #[serde(deserialize_with = "content")]
        content: String,
        #[serde(deserialize_with = "inner")]
        inner: String,
        #[serde(deserialize_with = "tag")]
        tag: String,
    },
}

/// Externally tagged, its variant under an alias: the key it is found under is bound as `tag`.
#[model_schema(decode_with)]
#[derive(Debug, Deserialize, PartialEq, Serialize)]
enum External {
    #[serde(alias = "Memo")]
    Note {
        #[serde(deserialize_with = "content")]
        content: String,
        #[serde(deserialize_with = "inner")]
        inner: String,
        #[serde(deserialize_with = "tag")]
        tag: String,
    },
}

read_hooks!(
    content, found, held, inner, issue, items, key, object, out, path, stored, tag
);

/// The read hook of [`Bound::item`]: a count, or text that holds one.
fn item<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match Count::deserialize(deserializer)? {
        Count::Number(count) => Ok(count),
        Count::Text(text) => text.parse().map_err(D::Error::custom),
    }
}

/// The write hook of [`Bound::item`], named like the value the walker hands it: text.
fn read<S, T>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    T: Display,
{
    serializer.collect_str(value)
}

fn bound() -> Bound {
    Bound {
        found: "a".to_owned(),
        held: "b".to_owned(),
        issue: "c".to_owned(),
        item: 5,
        key: "d".to_owned(),
        object: "e".to_owned(),
        out: "f".to_owned(),
        path: "g".to_owned(),
        stored: "h".to_owned(),
    }
}

#[test]
fn a_read_hook_named_like_the_value_a_key_holds_is_the_authors_function() {
    let mut calls = 0_u32;
    let read = read_counting!(Named, named_schema, json!({ "name": "x" }), calls);
    assert_eq!(
        read,
        Ok(Named {
            name: "x".to_owned()
        })
    );
    assert_eq!(calls, 0);
}

/// `key` and `stored` are read under their aliases, and `item` in the form its writer writes.
#[test]
fn a_hook_named_like_any_value_a_fields_walker_binds_is_the_authors_function() {
    let written = json!({
        "found": "a",
        "held": "b",
        "issue": "c",
        "item": "5",
        "id": "d",
        "object": "e",
        "out": "f",
        "path": "g",
        "saved": "h",
    });
    let mut calls = 0_u32;
    let read = read_counting!(Bound, bound_schema, written, calls);
    assert_eq!(read, Ok(bound()));
    assert_eq!(calls, 0);
}

#[test]
fn a_value_a_hook_named_like_a_binding_refuses_is_invalid_with_the_hooks_own_message() {
    let refused = json!({
        "found": "",
        "held": "",
        "issue": "",
        "item": "5",
        "id": "",
        "object": "",
        "out": "",
        "path": "",
        "saved": "",
    });
    assert_eq!(
        listed!(Bound, bound_schema, refused),
        [
            "found: invalid: expected String, found String(\"\"): `found` reads no empty text",
            "held: invalid: expected String, found String(\"\"): `held` reads no empty text",
            "issue: invalid: expected String, found String(\"\"): `issue` reads no empty text",
            "id: invalid: expected String, found String(\"\"): `key` reads no empty text",
            "object: invalid: expected String, found String(\"\"): `object` reads no empty text",
            "out: invalid: expected String, found String(\"\"): `out` reads no empty text",
            "path: invalid: expected String, found String(\"\"): `path` reads no empty text",
            "saved: invalid: expected String, found String(\"\"): `stored` reads no empty text",
        ]
    );
}

/// `item` reads a count held as a number, which `read` writes back as text.
#[test]
fn a_value_held_in_another_form_than_a_writer_named_like_a_binding_writes_is_mistyped() {
    let mut held_as_a_number = serde_json::to_value(bound()).unwrap();
    held_as_a_number["item"] = json!(5_i32);
    assert_eq!(
        listed!(Bound, bound_schema, held_as_a_number),
        ["item: mistyped: expected U32, found Number(5)"]
    );
}

#[test]
fn a_hook_named_like_a_value_a_positions_walker_binds_is_the_authors_function() {
    let mut calls = 0_u32;
    let read = read_counting!(Placed, placed_schema, json!(["a", "b"]), calls);
    assert_eq!(read, Ok(Placed("a".to_owned(), "b".to_owned())));
    assert_eq!(calls, 0);
    assert_eq!(
        listed!(Placed, placed_schema, json!(["", ""])),
        [
            "[0]: invalid: expected String, found String(\"\"): `found` reads no empty text",
            "[1]: invalid: expected String, found String(\"\"): `items` reads no empty text",
        ]
    );
}

#[test]
fn a_hook_named_like_a_value_an_enum_walker_binds_reads_a_variants_field() {
    let fields = json!({ "content": "a", "inner": "b", "tag": "c" });
    let mut calls = 0_u32;

    let internal = json!({ "kind": "Note", "content": "a", "inner": "b", "tag": "c" });
    assert_eq!(
        read_counting!(Internal, internal_schema, internal, calls),
        Ok(Internal::Note {
            content: "a".to_owned(),
            inner: "b".to_owned(),
            tag: "c".to_owned(),
        })
    );
    let adjacent = json!({ "kind": "Note", "body": fields });
    assert_eq!(
        read_counting!(Adjacent, adjacent_schema, adjacent, calls),
        Ok(Adjacent::Note {
            content: "a".to_owned(),
            inner: "b".to_owned(),
            tag: "c".to_owned(),
        })
    );
    for name in ["Note", "Memo"] {
        let mut external = serde_json::Map::new();
        external.insert(name.to_owned(), fields.clone());
        assert_eq!(
            read_counting!(External, external_schema, external.into(), calls),
            Ok(External::Note {
                content: "a".to_owned(),
                inner: "b".to_owned(),
                tag: "c".to_owned(),
            })
        );
    }
    assert_eq!(calls, 0);
}

#[test]
fn a_variants_field_a_hook_named_like_a_binding_refuses_is_invalid_at_the_field() {
    let internal = json!({ "kind": "Note", "content": "", "inner": "", "tag": "" });
    assert_eq!(
        listed!(Internal, internal_schema, internal),
        [
            "content: invalid: expected String, found String(\"\"): `content` reads no empty text",
            "inner: invalid: expected String, found String(\"\"): `inner` reads no empty text",
            "tag: invalid: expected String, found String(\"\"): `tag` reads no empty text",
        ]
    );
    let fields = json!({ "content": "", "inner": "", "tag": "" });
    let adjacent = json!({ "kind": "Note", "body": fields });
    assert_eq!(
        listed!(Adjacent, adjacent_schema, adjacent),
        [
            "body.content: invalid: expected String, found String(\"\"): `content` reads no empty text",
            "body.inner: invalid: expected String, found String(\"\"): `inner` reads no empty text",
            "body.tag: invalid: expected String, found String(\"\"): `tag` reads no empty text",
        ]
    );
    assert_eq!(
        listed!(External, external_schema, json!({ "Memo": fields })),
        [
            "Memo.content: invalid: expected String, found String(\"\"): `content` reads no empty text",
            "Memo.inner: invalid: expected String, found String(\"\"): `inner` reads no empty text",
            "Memo.tag: invalid: expected String, found String(\"\"): `tag` reads no empty text",
        ]
    );
}
