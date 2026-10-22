//! What each surface writes for a set: the JSON Schema an array of distinct items, Zod a check
//! that refuses a repeat, TypeScript the array a list writes. Rust's read keeps one item.

use alloc::collections::{BinaryHeap, VecDeque};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::model_schema;

use super::models::{
    EveryList, Folder, Lanyard, Nesting, PAYLOADS, PRINTED, Pouch, Shelving, Sorting, Ticket,
    Wallet, assert_rust_keeps_one_item, declares_a_parameter,
};

/// The message Zod refuses a repeat with, as its schema writes it.
const REPEATED: &str = "\"repeats an item: a set holds each item once\"";

/// The check a set's array is written with, as Zod's schema of [`Ticket`] writes it.
const TICKET_TAGS: &str = "  tags: z.array(z.string()).check((ctx) => {
  const seen = new Set<string>();
  for (const item of ctx.value) {
    const text = JSON.stringify(item);
    if (seen.has(text)) {
      ctx.issues.push({ code: \"custom\", message: \"repeats an item: a set holds each item \
                           once\", input: ctx.value, continue: true });
      return;
    }
    seen.add(text);
  }
}),
";

/// Each sequence that is no set.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Queues {
    pub binary_heap: BinaryHeap<u32>,
    pub listed: Vec<Vec<u32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maybe: Option<Vec<u32>>,
    pub vec_deque: VecDeque<u32>,
}

/// The `indexmap` set.
#[cfg(feature = "indexmap")]
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IndexedTicket {
    pub tags: indexmap::IndexSet<String>,
}

/// The JSON pointer of each schema in `document` that holds `uniqueItems`, each held to `true`.
fn distinct_at(document: &Value) -> Vec<String> {
    let mut found = Vec::new();
    collect_distinct(document, "", &mut found);
    found
}

/// [`distinct_at`] below `at`.
fn collect_distinct(schema: &Value, at: &str, found: &mut Vec<String>) {
    match schema {
        Value::Object(members) => {
            if let Some(distinct) = members.get("uniqueItems") {
                assert_eq!(distinct, &json!(true), "at {at}");
                found.push(at.to_owned());
            }
            for (key, member) in members {
                collect_distinct(member, &format!("{at}/{key}"), found);
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                collect_distinct(item, &format!("{at}/{index}"), found);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

#[test]
fn each_set_writes_unique_items_at_its_own_level() {
    for (document, expected) in [
        (Ticket::json_schema(), vec!["/properties/tags"]),
        (
            Folder::json_schema(),
            vec!["/properties/ticket/properties/tags"],
        ),
        (
            EveryList::json_schema(),
            vec!["/properties/btree_set", "/properties/hash_set"],
        ),
        (
            Nesting::json_schema(),
            vec![
                "/properties/grid/items",
                "/properties/keyed/additionalProperties",
                "/properties/listed",
                "/properties/maybe/anyOf/0",
                "/properties/points",
            ],
        ),
        (Lanyard::json_schema(), vec![""]),
        (Pouch::json_schema(), vec!["/prefixItems/0"]),
        (
            Shelving::json_schema(),
            vec![
                "/oneOf/0/properties/held",
                "/oneOf/1/properties/held/properties/members",
            ],
        ),
        (
            Sorting::json_schema(),
            vec!["/anyOf/0/properties/labels", "/anyOf/1/properties/numbers"],
        ),
        #[cfg(feature = "indexmap")]
        (IndexedTicket::json_schema(), vec!["/properties/tags"]),
    ] {
        assert_eq!(distinct_at(&document), expected, "{document}");
    }
}

#[test]
fn each_payload_is_read_as_a_model_that_holds_a_set_and_printed_in_its_order() {
    let documents = [
        ("EveryList", EveryList::json_schema()),
        ("Folder", Folder::json_schema()),
        ("Lanyard", Lanyard::json_schema()),
        ("Nesting", Nesting::json_schema()),
        ("Pouch", Pouch::json_schema()),
        ("Shelving", Shelving::json_schema()),
        ("Sorting", Sorting::json_schema()),
        ("Ticket", Ticket::json_schema()),
        ("Wallet", Wallet::<String>::json_schema()),
    ];
    for payload in &PAYLOADS {
        let (_, document) = documents
            .iter()
            .find(|(named, _)| *named == payload.read_as)
            .unwrap();
        assert!(!distinct_at(document).is_empty(), "{}", payload.read_as);
    }
    assert!(
        PAYLOADS
            .iter()
            .any(|payload| declares_a_parameter(payload.read_as))
    );
    let printed: Vec<&str> = PRINTED
        .lines()
        .filter(|line| !line.starts_with(' '))
        .map(|line| line.split_once(": ").unwrap().0)
        .collect();
    let named: Vec<&str> = PAYLOADS.iter().map(|payload| payload.name).collect();
    assert_eq!(printed, named);
}

#[test]
fn a_set_keeps_the_count_written_on_it_beside_its_distinct_items() {
    let document = EveryList::json_schema();
    for set in ["btree_set", "hash_set"] {
        assert_eq!(
            document["properties"][set],
            json!({
                "type": "array",
                "items": { "type": "string" },
                "uniqueItems": true,
                "minItems": 2_u32,
            })
        );
    }
    assert_eq!(
        document["properties"]["vec"],
        json!({ "type": "array", "items": { "type": "string" }, "minItems": 2_u32 })
    );
}

#[test]
fn a_list_that_is_no_set_writes_no_unique_items() {
    assert_eq!(distinct_at(&Queues::json_schema()), Vec::<String>::new());
    assert!(!Queues::zod_schema().contains(REPEATED));
}

#[test]
fn zod_checks_each_set_for_a_repeat_after_its_count() {
    assert!(Ticket::zod_schema().contains(TICKET_TAGS));
    for (zod, sets) in [
        (Ticket::zod_schema(), 1),
        (Folder::zod_schema(), 0),
        (EveryList::zod_schema(), 2),
        (Nesting::zod_schema(), 5),
        (Lanyard::zod_schema(), 1),
        (Pouch::zod_schema(), 1),
        (Shelving::zod_schema(), 2),
        (Sorting::zod_schema(), 2),
        #[cfg(feature = "indexmap")]
        (IndexedTicket::zod_schema(), 1),
    ] {
        assert_eq!(zod.matches(REPEATED).count(), sets, "{zod}");
    }
    let every_list = EveryList::zod_schema();
    let counted = "btree_set: z.array(z.string()).min(2, { error: (issue) => `too short: minimum \
                   length is 2, got ${Array.isArray(issue.input) ? issue.input.length : 0}` \
                   }).check((ctx) => {";
    assert!(every_list.contains(counted), "{every_list}");
    let nesting = Nesting::zod_schema();
    assert!(
        nesting.contains(
            "  grid: z.array(z.array(z.number().int().min(0).max(4294967295)).check((ctx) => {"
        ),
        "{nesting}"
    );
    assert!(
        nesting.contains(
            "  listed: z.array(z.array(z.number().int().min(0).max(4294967295))).check((ctx) => {"
        ),
        "{nesting}"
    );
}

#[test]
fn typescript_writes_a_set_as_the_array_a_list_writes() {
    assert!(
        Ticket::ts_definition().contains("tags: Array<string>;"),
        "{}",
        Ticket::ts_definition()
    );
    let nesting = Nesting::ts_definition();
    for written in [
        "grid: Array<Array<number>>;",
        "keyed: Partial<Record<string, Array<number>>>;",
        "listed: Array<Array<number>>;",
        "maybe: Array<number> | undefined;",
        "points: Array<Point>;",
    ] {
        assert!(nesting.contains(written), "{written} in: {nesting}");
    }
}

#[test]
fn rust_reads_a_repeat_keeping_one_item() {
    assert_rust_keeps_one_item();
}

#[test]
fn rust_validate_answers_nothing_of_a_set_it_read() {
    let read: EveryList =
        serde_json::from_str(r#"{"btree_set":["a","b"],"hash_set":["c","d"],"vec":["e","e"]}"#)
            .unwrap();
    assert_eq!(read.validate(), Ok(()));
}
