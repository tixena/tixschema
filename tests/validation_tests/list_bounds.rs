//! A key written on a list measures the list: `minLength` and `maxLength` count its items.

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod a_key_on_a_list {
    use alloc::collections::{BTreeSet, BinaryHeap, VecDeque};
    use serde::{Deserialize, Serialize};
    use std::collections::HashSet;
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Part {
        pub value: String,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Document {
        /// One to three name parts.
        #[model_schema_prop(minLength = 1, maxLength = 3)]
        pub name_config: Vec<Part>,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Counted {
        #[model_schema_prop(minLength = 1, maxLength = 3)]
        pub numbers: Vec<u32>,
        #[model_schema_prop(minLength = 1, maxLength = 3)]
        pub strings: Vec<String>,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct EachAlone {
        #[model_schema_prop(minLength = 1)]
        pub least: Vec<String>,
        #[model_schema_prop(maxLength = 3)]
        pub most: Vec<String>,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct EveryList {
        #[model_schema_prop(minLength = 2)]
        pub deque: VecDeque<String>,
        #[model_schema_prop(minLength = 2)]
        pub hashed: HashSet<String>,
        #[model_schema_prop(minLength = 2)]
        pub heap: BinaryHeap<u32>,
        #[model_schema_prop(minLength = 2)]
        pub listed: Vec<String>,
        #[model_schema_prop(minLength = 2)]
        pub ordered: BTreeSet<String>,
        #[model_schema_prop(minLength = 2)]
        pub pair: [String; 2],
        #[model_schema_prop(minLength = 2)]
        pub sliced: Box<[String]>,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct TooWide {
        #[model_schema_prop(maxLength = 1)]
        pub pair: [String; 2],
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Wrapped {
        #[model_schema_prop(minLength = 1)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub maybe: Option<Vec<Part>>,
        #[model_schema_prop(minLength = 1)]
        pub rows: Vec<Vec<String>>,
    }

    /// A tag held to a rule of its own, which is where a rule for each item of a list is written.
    #[model_schema(minLength = 3)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct Tag(pub String);

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Tagged {
        #[model_schema_prop(maxLength = 2)]
        pub counted: Vec<Tag>,
        pub free: Vec<Tag>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Line {
        #[model_schema_prop(minLength = 2)]
        pub code: String,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Order {
        #[model_schema_prop(minLength = 1)]
        pub lines: Vec<Line>,
    }

    #[model_schema(default_types(T = String))]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Page<T> {
        #[model_schema_prop(maxLength = 2)]
        pub items: Vec<T>,
        #[model_schema_prop(minLength = 1)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub more: Option<Box<[T]>>,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Tree {
        #[model_schema_prop(maxLength = 2)]
        pub children: Vec<Self>,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub enum Shipment {
        Boxes {
            #[model_schema_prop(minLength = 1)]
            labels: Vec<String>,
        },
        Pallets(#[model_schema_prop(maxLength = 2)] Vec<u32>),
    }

    fn parts(count: usize) -> Vec<Part> {
        vec![
            Part {
                value: "part".to_owned()
            };
            count
        ]
    }

    fn strings(count: usize) -> Vec<String> {
        (0..count).map(|at| format!("long enough {at}")).collect()
    }

    #[test]
    fn a_list_of_models_is_held_to_its_item_count() {
        assert_eq!(
            Document {
                name_config: parts(0)
            }
            .validate()
            .unwrap_err(),
            vec!["'nameConfig': too short: minimum length is 1, got 0"]
        );
        assert_eq!(
            Document {
                name_config: parts(4)
            }
            .validate()
            .unwrap_err(),
            vec!["'nameConfig': too long: maximum length is 3, got 4"]
        );
        for held in 1..=3 {
            let admitted = Document {
                name_config: parts(held),
            };
            assert_eq!(admitted.validate(), Ok(()), "{held} parts");
        }
    }

    #[test]
    fn a_list_of_strings_and_a_list_of_numbers_are_counted_and_no_item_is_measured() {
        assert_eq!(
            Counted {
                numbers: Vec::new(),
                strings: Vec::new(),
            }
            .validate()
            .unwrap_err(),
            vec![
                "'numbers': too short: minimum length is 1, got 0",
                "'strings': too short: minimum length is 1, got 0",
            ]
        );
        assert_eq!(
            Counted {
                numbers: vec![1, 2, 3, 4],
                strings: strings(4),
            }
            .validate()
            .unwrap_err(),
            vec![
                "'numbers': too long: maximum length is 3, got 4",
                "'strings': too long: maximum length is 3, got 4",
            ]
        );
        let held = Counted {
            numbers: vec![0, 4000],
            strings: vec!["abcd".to_owned(), String::new()],
        };
        assert_eq!(held.validate(), Ok(()));
    }

    #[test]
    fn each_key_is_enforced_alone() {
        assert_eq!(
            EachAlone {
                least: Vec::new(),
                most: strings(4),
            }
            .validate()
            .unwrap_err(),
            vec![
                "'least': too short: minimum length is 1, got 0",
                "'most': too long: maximum length is 3, got 4",
            ]
        );
        let held = EachAlone {
            least: strings(9),
            most: Vec::new(),
        };
        assert_eq!(held.validate(), Ok(()));
    }

    #[test]
    fn every_list_type_is_counted() {
        let one = || "only".to_owned();
        let short = EveryList {
            deque: VecDeque::from([one()]),
            hashed: HashSet::from([one()]),
            heap: BinaryHeap::from([1]),
            listed: vec![one()],
            ordered: BTreeSet::from([one()]),
            pair: [one(), one()],
            sliced: Box::from([one()]),
        };
        assert_eq!(
            short.validate().unwrap_err(),
            ["deque", "hashed", "heap", "listed", "ordered", "sliced"]
                .map(|field| format!("'{field}': too short: minimum length is 2, got 1"))
        );

        let two = || ["a".to_owned(), "b".to_owned()];
        let held = EveryList {
            deque: VecDeque::from(two()),
            hashed: HashSet::from(two()),
            heap: BinaryHeap::from([1, 2]),
            listed: Vec::from(two()),
            ordered: BTreeSet::from(two()),
            pair: two(),
            sliced: Box::from(two()),
        };
        assert_eq!(held.validate(), Ok(()));

        assert_eq!(
            TooWide { pair: two() }.validate().unwrap_err(),
            vec!["'pair': too long: maximum length is 1, got 2"]
        );
    }

    #[test]
    fn an_optional_list_is_counted_inside_its_some() {
        let rows = || vec![Vec::new()];
        assert_eq!(
            Wrapped {
                maybe: None,
                rows: rows()
            }
            .validate(),
            Ok(())
        );
        assert_eq!(
            Wrapped {
                maybe: Some(Vec::new()),
                rows: rows()
            }
            .validate()
            .unwrap_err(),
            vec!["'maybe': too short: minimum length is 1, got 0"]
        );
        assert_eq!(
            Wrapped {
                maybe: Some(parts(1)),
                rows: rows()
            }
            .validate(),
            Ok(())
        );
    }

    #[test]
    fn a_list_of_lists_is_counted_at_the_outer_list() {
        assert_eq!(
            Wrapped {
                maybe: None,
                rows: Vec::new()
            }
            .validate()
            .unwrap_err(),
            vec!["'rows': too short: minimum length is 1, got 0"]
        );
        assert_eq!(
            Wrapped {
                maybe: None,
                rows: vec![Vec::new()]
            }
            .validate(),
            Ok(())
        );
    }

    #[test]
    fn each_item_of_a_list_of_brands_is_held_by_the_brand() {
        let tag = |text: &str| Tag(text.to_owned());
        assert_eq!(
            Tagged {
                counted: vec![tag("abc")],
                free: vec![tag("abc"), tag("ab"), tag("a")],
            }
            .validate()
            .unwrap_err(),
            vec![
                "'free.1': too short: minimum length is 3, got 2",
                "'free.2': too short: minimum length is 3, got 1",
            ]
        );
        assert_eq!(
            Tagged {
                counted: vec![tag("abc"), tag("ab"), tag("abcd")],
                free: Vec::new(),
            }
            .validate()
            .unwrap_err(),
            vec![
                "'counted': too long: maximum length is 2, got 3",
                "'counted.1': too short: minimum length is 3, got 2",
            ],
            "the list is counted, and each item is still held by its brand"
        );
        assert_eq!(
            serde_json::from_str::<Tagged>(r#"{"counted":["ab"],"free":[]}"#)
                .unwrap_err()
                .to_string(),
            "'counted.0': too short: minimum length is 3, got 2"
        );
    }

    #[test]
    fn each_item_of_a_counted_list_of_models_is_still_validated() {
        let line = |code: &str| Line {
            code: code.to_owned(),
        };
        assert_eq!(
            Order { lines: Vec::new() }.validate().unwrap_err(),
            vec!["'lines': too short: minimum length is 1, got 0"]
        );
        assert_eq!(
            Order {
                lines: vec![line("ok"), line("x")]
            }
            .validate()
            .unwrap_err(),
            vec!["'lines.1.code': too short: minimum length is 2, got 1"]
        );
    }

    #[test]
    fn a_list_of_a_type_parameter_and_a_list_of_the_type_itself_are_counted() {
        assert_eq!(
            Page {
                items: strings(3),
                more: Some(Box::from([])),
            }
            .validate()
            .unwrap_err(),
            vec![
                "'items': too long: maximum length is 2, got 3",
                "'more': too short: minimum length is 1, got 0",
            ]
        );
        assert_eq!(
            Page {
                items: strings(2),
                more: None,
            }
            .validate(),
            Ok(())
        );

        let leaf = || Tree {
            children: Vec::new(),
        };
        assert_eq!(
            Tree {
                children: vec![leaf(), leaf(), leaf()]
            }
            .validate()
            .unwrap_err(),
            vec!["'children': too long: maximum length is 2, got 3"]
        );
        assert_eq!(
            Tree {
                children: vec![leaf()]
            }
            .validate(),
            Ok(())
        );
    }

    #[test]
    fn a_tagged_variants_list_is_counted_as_a_named_member_and_as_a_slot() {
        assert_eq!(
            Shipment::Boxes { labels: Vec::new() }
                .validate()
                .unwrap_err(),
            vec!["'Boxes.labels': too short: minimum length is 1, got 0"]
        );
        assert_eq!(
            Shipment::Pallets(vec![1, 2, 3]).validate().unwrap_err(),
            vec!["'Pallets': too long: maximum length is 2, got 3"]
        );
        assert_eq!(Shipment::Pallets(Vec::new()).validate(), Ok(()));
    }

    /// The README prints these two reports beside the declaration they come from.
    #[test]
    fn the_readme_shows_what_validate_reports_for_a_list() {
        let readme = include_str!("../../README.md");
        let empty = Document {
            name_config: parts(0),
        };
        let full = Document {
            name_config: parts(4),
        };
        for (reported, shown) in [
            (
                empty.validate().unwrap_err(),
                "// Err([\"'nameConfig': too short: minimum length is 1, got 0\"])",
            ),
            (
                full.validate().unwrap_err(),
                "answers `'nameConfig': too long: maximum length is 3, got 4`",
            ),
        ] {
            assert_eq!(reported.len(), 1, "one report for one bound: {reported:?}");
            let said = reported.concat();
            assert!(shown.contains(said.as_str()), "{said} is not in {shown}");
            assert!(readme.contains(shown), "the README no longer shows {shown}");
        }
    }

    /// serde refuses a list that breaks its bound, with what `validate()` answers of it.
    #[test]
    fn serde_refuses_a_list_its_bound_refuses() {
        assert_eq!(
            serde_json::from_str::<Document>(r#"{"nameConfig":[]}"#)
                .unwrap_err()
                .to_string(),
            "'nameConfig': too short: minimum length is 1, got 0"
        );
    }

    /// The hook a list field publishes reads the list and then counts it.
    #[test]
    fn the_hook_a_list_publishes_refuses_a_list_that_breaks_its_bound() {
        let hooked = |stored: &str| {
            let mut read = serde_json::Deserializer::from_str(stored);
            document_schema::deserialize_name_config::<_, Vec<Part>>(&mut read)
                .map(|held| held.len())
                .map_err(|refused| refused.to_string())
        };
        assert_eq!(hooked(r#"[{"value":"a"}]"#), Ok(1));
        assert_eq!(
            hooked("[]"),
            Err("'nameConfig': too short: minimum length is 1, got 0".to_owned())
        );
        assert_eq!(
            document_schema::validate_name_config_value(&parts(4)),
            Err(vec![
                "'nameConfig': too long: maximum length is 3, got 4".to_owned()
            ])
        );
    }

    #[cfg(feature = "zod")]
    #[test]
    fn the_zod_schema_counts_the_array_and_writes_no_check_on_its_items() {
        let short = |least: usize| {
            format!(
                ".min({least}, {{ error: (issue) => `too short: minimum length is {least}, got \
                 ${{Array.isArray(issue.input) ? issue.input.length : 0}}` }})"
            )
        };
        let long = |most: usize| {
            format!(
                ".max({most}, {{ error: (issue) => `too long: maximum length is {most}, got \
                 ${{Array.isArray(issue.input) ? issue.input.length : 0}}` }})"
            )
        };
        let every = |member: &str, item: &str, pinned: &str| {
            format!("  {member}: z.array({item}){pinned}{},\n", short(2))
        };
        // A set's array refuses a repeat after its count.
        let every_set = |member: &str| {
            format!(
                "  {member}: z.array(z.string()){}.check((ctx) => {{",
                short(2)
            )
        };
        for (schema, members) in [
            (
                Document::zod_schema(),
                vec![format!(
                    "  nameConfig: z.array(Part$Schema){}{},\n",
                    short(1),
                    long(3)
                )],
            ),
            (
                Counted::zod_schema(),
                vec![
                    format!(
                        "  numbers: z.array(z.number().int()){}{},\n",
                        short(1),
                        long(3)
                    ),
                    format!("  strings: z.array(z.string()){}{},\n", short(1), long(3)),
                ],
            ),
            (
                EachAlone::zod_schema(),
                vec![
                    format!("  least: z.array(z.string()){},\n", short(1)),
                    format!("  most: z.array(z.string()){},\n", long(3)),
                ],
            ),
            (
                EveryList::zod_schema(),
                vec![
                    every("deque", "z.string()", ""),
                    every_set("hashed"),
                    every("heap", "z.number().int()", ""),
                    every("listed", "z.string()", ""),
                    every_set("ordered"),
                    every("pair", "z.string()", ".length(2)"),
                    every("sliced", "z.string()", ""),
                ],
            ),
            (
                Wrapped::zod_schema(),
                vec![
                    format!(
                        "z.union([z.null().transform(() => undefined), z.array(Part$Schema){}, \
                         z.undefined()]).prefault(undefined)",
                        short(1)
                    ),
                    format!("  rows: z.array(z.array(z.string())){},\n", short(1)),
                ],
            ),
            (
                Tagged::zod_schema(),
                vec![
                    format!("  counted: z.array(Tag$Schema){},\n", long(2)),
                    "  free: z.array(Tag$Schema),\n".to_owned(),
                ],
            ),
            (
                Shipment::zod_schema(),
                vec![
                    format!("  labels: z.array(z.string()){},\n", short(1)),
                    format!("  \"Pallets\": z.array(z.number().int()){},\n", long(2)),
                ],
            ),
        ] {
            for written in members {
                assert!(schema.contains(&written), "{written} is not in:\n{schema}");
            }
        }
    }

    #[cfg(feature = "jsonschema")]
    #[test]
    fn the_json_schema_writes_the_count_as_min_items_and_max_items() {
        use serde_json::json;

        let string = || json!({ "type": "string" });
        assert_eq!(
            Document::json_schema()["properties"]["nameConfig"],
            json!({
                "type": "array", "items": Part::json_schema(), "minItems": 1_u32, "maxItems": 3_u32
            })
        );
        assert_eq!(
            Counted::json_schema()["properties"],
            json!({
                "numbers": {
                    "type": "array", "items": { "type": "integer" },
                    "minItems": 1_u32, "maxItems": 3_u32
                },
                "strings": {
                    "type": "array", "items": string(), "minItems": 1_u32, "maxItems": 3_u32
                },
            })
        );
        assert_eq!(
            EachAlone::json_schema()["properties"],
            json!({
                "least": { "type": "array", "items": string(), "minItems": 1_u32 },
                "most": { "type": "array", "items": string(), "maxItems": 3_u32 },
            })
        );

        let every = EveryList::json_schema();
        for member in ["deque", "listed", "sliced"] {
            assert_eq!(
                every["properties"][member],
                json!({ "type": "array", "items": string(), "minItems": 2_u32 }),
                "for {member}"
            );
        }
        for member in ["hashed", "ordered"] {
            assert_eq!(
                every["properties"][member],
                json!({
                    "type": "array", "items": string(), "uniqueItems": true, "minItems": 2_u32
                }),
                "for {member}"
            );
        }
        assert_eq!(
            every["properties"]["heap"],
            json!({ "type": "array", "items": { "type": "integer" }, "minItems": 2_u32 })
        );
        assert_eq!(
            every["properties"]["pair"],
            json!({ "type": "array", "items": string(), "minItems": 2_u32, "maxItems": 2_u32 })
        );
        assert_eq!(
            TooWide::json_schema()["properties"]["pair"],
            json!({ "type": "array", "items": string(), "minItems": 2_u32, "maxItems": 1_u32 }),
            "the declared length and the bound are both stated, and no array meets the two"
        );

        let wrapped = Wrapped::json_schema();
        assert_eq!(
            wrapped["properties"]["maybe"],
            json!({
                "anyOf": [
                    { "type": "array", "items": Part::json_schema(), "minItems": 1_u32 },
                    { "type": "null" },
                ]
            })
        );
        assert_eq!(
            wrapped["properties"]["rows"],
            json!({
                "type": "array",
                "items": { "type": "array", "items": string() },
                "minItems": 1_u32
            })
        );
        assert_eq!(
            Tagged::json_schema()["properties"]["counted"],
            json!({ "type": "array", "items": Tag::json_schema(), "maxItems": 2_u32 })
        );
    }

    #[cfg(feature = "typescript")]
    #[test]
    fn the_typescript_type_stays_an_array_and_its_comment_states_the_count() {
        let document = Document::ts_definition();
        let written = "  /**\n   * One to three name parts.\n   * \n   * Minimum length: 1\n   * \
                       Maximum length: 3\n   */\n  nameConfig: Array<Part>;";
        assert!(
            document.contains(written),
            "{written} is not in:\n{document}"
        );
    }
}
