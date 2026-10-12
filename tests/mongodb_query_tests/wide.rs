//! Types with many nested keys: four levels of six fields each, the inner three held in lists,
//! and two tagged enums whose every variant holds the widest of those lists.
//!
//! Each binary declares this module outside `cfg(test)`: inside it clippy's `large_stack_frames`
//! reports nothing, and that lint over what these types generate is what the module is for. The
//! sixth field of each level is named apart from the other five: clippy refuses a struct whose
//! fields all share a prefix.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
struct Leaf {
    last: String,
    text_0: String,
    text_1: String,
    text_2: String,
    text_3: String,
    text_4: String,
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
struct Branch {
    last: Vec<Leaf>,
    leaf_0: Vec<Leaf>,
    leaf_1: Vec<Leaf>,
    leaf_2: Vec<Leaf>,
    leaf_3: Vec<Leaf>,
    leaf_4: Vec<Leaf>,
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
struct Trunk {
    branch_0: Vec<Branch>,
    branch_1: Vec<Branch>,
    branch_2: Vec<Branch>,
    branch_3: Vec<Branch>,
    branch_4: Vec<Branch>,
    last: Vec<Branch>,
}

#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
struct Root {
    last: Vec<Trunk>,
    trunk_0: Vec<Trunk>,
    trunk_1: Vec<Trunk>,
    trunk_2: Vec<Trunk>,
    trunk_3: Vec<Trunk>,
    trunk_4: Vec<Trunk>,
}

/// serde writes the list of a variant under `list`, beside the tag.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", content = "list")]
enum Listed {
    Fifth(Vec<Trunk>),
    First(Vec<Trunk>),
    Fourth(Vec<Trunk>),
    Second(Vec<Trunk>),
    Sixth(Vec<Trunk>),
    Third(Vec<Trunk>),
}

/// serde writes the list of a variant under its field's own key, beside the tag.
#[model_schema(decode_with)]
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind")]
enum Keyed {
    Fifth { trunks: Vec<Trunk> },
    First { trunks: Vec<Trunk> },
    Fourth { trunks: Vec<Trunk> },
    Second { trunks: Vec<Trunk> },
    Sixth { trunks: Vec<Trunk> },
    Third { trunks: Vec<Trunk> },
}

#[cfg(test)]
mod tests {
    use core::mem::size_of;

    use bson::{Bson, Document, doc};

    use super::{
        Branch, Keyed, Leaf, Listed, Root, Trunk, keyed_schema, leaf_schema, listed_schema,
        root_schema,
    };
    use crate::tests::shown;

    fn leaf() -> Leaf {
        Leaf {
            last: "f".to_owned(),
            text_0: "a".to_owned(),
            text_1: "b".to_owned(),
            text_2: "c".to_owned(),
            text_3: "Clerk".to_owned(),
            text_4: "e".to_owned(),
        }
    }

    /// One trunk that holds one branch under `branch_4`, which holds one leaf under `leaf_1`.
    fn trunk() -> Trunk {
        Trunk {
            branch_0: Vec::new(),
            branch_1: Vec::new(),
            branch_2: Vec::new(),
            branch_3: Vec::new(),
            branch_4: vec![Branch {
                last: Vec::new(),
                leaf_0: Vec::new(),
                leaf_1: vec![leaf()],
                leaf_2: Vec::new(),
                leaf_3: Vec::new(),
                leaf_4: Vec::new(),
            }],
            last: Vec::new(),
        }
    }

    /// [`trunk`] as it is stored, with `text_3` of its one leaf holding `text_3`.
    fn stored_trunk(text_3: Bson) -> Document {
        doc! {
            "branch_0": [],
            "branch_1": [],
            "branch_2": [],
            "branch_3": [],
            "branch_4": [{
                "last": [],
                "leaf_0": [],
                "leaf_1": [{
                    "last": "f",
                    "text_0": "a",
                    "text_1": "b",
                    "text_2": "c",
                    "text_3": text_3,
                    "text_4": "e",
                }],
                "leaf_2": [],
                "leaf_3": [],
                "leaf_4": [],
            }],
            "last": [],
        }
    }

    /// A row of `Root` that holds [`stored_trunk`] under `trunk_2`.
    fn stored_root(text_3: Bson) -> Document {
        doc! {
            "last": [],
            "trunk_0": [],
            "trunk_1": [],
            "trunk_2": [stored_trunk(text_3)],
            "trunk_3": [],
            "trunk_4": [],
        }
    }

    fn clerk() -> Bson {
        Bson::String("Clerk".to_owned())
    }

    #[test]
    fn a_row_of_the_widest_type_is_read_by_both_entry_points() {
        let root = Root {
            last: Vec::new(),
            trunk_0: Vec::new(),
            trunk_1: Vec::new(),
            trunk_2: vec![trunk()],
            trunk_3: Vec::new(),
            trunk_4: Vec::new(),
        };
        assert_eq!(
            Root::from_bson_piped(stored_root(clerk()), &[]),
            Ok(root.clone())
        );
        assert_eq!(
            Root::from_bson_with(stored_root(clerk()), |_row, _issues| {
                root_schema::Verdict::Reject
            }),
            Ok(root)
        );
    }

    #[test]
    fn an_issue_four_types_down_is_listed_at_its_full_path() {
        let refused = Root::from_bson_piped(stored_root(Bson::Int32(7)), &[]).unwrap_err();
        assert_eq!(refused.issues.len(), 1);
        let shown = refused.to_string();
        assert!(
            shown.starts_with("trunk_2[0].branch_4[0].leaf_1[0].text_3: "),
            "got: {shown}"
        );
    }

    #[test]
    fn a_row_of_each_wide_enum_is_read() {
        assert_eq!(
            Listed::from_bson_piped(
                doc! { "kind": "Second", "list": [stored_trunk(clerk())] },
                &[]
            ),
            Ok(Listed::Second(vec![trunk()]))
        );
        assert_eq!(
            Keyed::from_bson_piped(
                doc! { "kind": "Fourth", "trunks": [stored_trunk(clerk())] },
                &[]
            ),
            Ok(Keyed::Fourth {
                trunks: vec![trunk()]
            })
        );
    }

    #[test]
    fn a_path_four_types_down_writes_its_full_key() {
        let clerk = Root::MONGO_FIELDS.trunk_2().nest(
            Trunk::MONGO_FIELDS.branch_4().nest(
                Branch::MONGO_FIELDS
                    .leaf_1()
                    .nest(Leaf::MONGO_FIELDS.text_3().eq("Clerk".to_owned()).unwrap()),
            ),
        );
        assert_eq!(
            shown(clerk),
            r#"{ "trunk_2.branch_4.leaf_1.text_3": { "$eq": "Clerk" } }"#
        );
    }

    #[test]
    fn the_paths_of_a_type_are_as_large_whatever_it_holds() {
        assert_eq!(
            size_of::<root_schema::MongoFields<Root>>(),
            size_of::<leaf_schema::MongoFields<Leaf>>()
        );
        assert_eq!(
            size_of::<listed_schema::MongoFields<Listed>>(),
            size_of::<leaf_schema::MongoFields<Leaf>>()
        );
        assert_eq!(
            size_of::<keyed_schema::MongoFields<Keyed>>(),
            size_of::<leaf_schema::MongoFields<Leaf>>()
        );
    }

    #[test]
    fn what_a_variant_of_a_wide_enum_holds_is_under_the_key_serde_writes_it_at() {
        let two_branches = || Trunk::MONGO_FIELDS.branch_4().size(2);
        assert_eq!(
            shown(Listed::MONGO_FIELDS.second().nest(two_branches())),
            r#"{ "list.branch_4": { "$size": Int64(2) } }"#
        );
        assert_eq!(
            shown(Listed::MONGO_FIELDS.is_second()),
            r#"{ "kind": { "$eq": "Second" } }"#
        );
        assert_eq!(
            shown(Keyed::MONGO_FIELDS.fourth().trunks().nest(two_branches())),
            r#"{ "trunks.branch_4": { "$size": Int64(2) } }"#
        );
        assert_eq!(
            shown(Keyed::MONGO_FIELDS.is_fourth()),
            r#"{ "kind": { "$eq": "Fourth" } }"#
        );
    }
}
