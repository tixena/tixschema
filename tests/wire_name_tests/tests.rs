mod without_one {
    use super::*;

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(tag = "kind")]
    pub enum Tagged {
        Custom,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(tag = "kind", content = "data")]
    pub enum Adjacent {
        Custom,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub enum Carrying {
        Custom,
        Valued(String),
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub enum Skipped {
        Custom,
        #[serde(skip)]
        Hidden,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub enum SkippedWhenWritten {
        Custom,
        #[serde(skip_serializing)]
        Quiet,
    }

    /// Calling `wire_name` on a type with no such method of its own reaches this one instead.
    trait Missing {
        fn wire_name(&self) -> Option<&'static str> {
            None
        }
    }

    impl<T> Missing for T {}

    #[test]
    fn an_enum_serde_writes_any_other_way_gets_no_wire_name() {
        assert_eq!(TaggedDraft::Custom.wire_name(), None);
        assert_eq!(AdjacentDraft::Custom.wire_name(), None);
        assert_eq!(CarryingDraft::Custom.wire_name(), None);
        assert_eq!(Skipped::Custom.wire_name(), None);
        assert_eq!(Skipped::Hidden.wire_name(), None);
        assert_eq!(SkippedWhenWritten::Custom.wire_name(), None);
    }
}

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

const MAX: &str = Dated::MaxIsCurrentDate.wire_name();

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub enum Plain {
    Custom,
    MaxIsCurrentDate,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kebab {
    DocumentGroupNotFound,
    Other,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Camel {
    DocumentGroupNotFound,
    Other,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Renamed {
    #[serde(rename = "bpm-tasks-widget")]
    BpmTasks,
    DocumentGroupNotFound,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub enum Split {
    #[serde(alias = "other-alias")]
    Aliased,
    #[serde(rename(serialize = "written", deserialize = "written"))]
    Sided,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub enum Fallback {
    Known,
    #[serde(other)]
    Unknown,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub enum Dated {
    Custom,
    MaxIsCurrentDate,
}

fn written<T>(value: &T) -> String
where
    T: Serialize,
{
    let text = serde_json::to_value(value).unwrap();
    text.as_str().map(str::to_owned).unwrap()
}

#[test]
fn a_variant_with_no_rename_is_written_as_its_ident() {
    for draft in [Plain::Custom, Plain::MaxIsCurrentDate] {
        assert_eq!(draft.wire_name(), written(&draft));
    }
    assert_eq!(Plain::MaxIsCurrentDate.wire_name(), "MaxIsCurrentDate");
}

#[test]
fn a_rename_all_rule_is_applied() {
    for draft in [Kebab::DocumentGroupNotFound, Kebab::Other] {
        assert_eq!(draft.wire_name(), written(&draft));
    }
    assert_eq!(
        Kebab::DocumentGroupNotFound.wire_name(),
        "document-group-not-found"
    );
    for draft in [Camel::DocumentGroupNotFound, Camel::Other] {
        assert_eq!(draft.wire_name(), written(&draft));
    }
    assert_eq!(
        Camel::DocumentGroupNotFound.wire_name(),
        "documentGroupNotFound"
    );
}

#[test]
fn a_variant_rename_wins_over_rename_all() {
    for draft in [Renamed::DocumentGroupNotFound, Renamed::BpmTasks] {
        assert_eq!(draft.wire_name(), written(&draft));
    }
    assert_eq!(Renamed::BpmTasks.wire_name(), "bpm-tasks-widget");
}

#[test]
fn a_list_form_rename_is_written_by_its_serialize_side_and_an_alias_is_ignored() {
    for draft in [Split::Sided, Split::Aliased] {
        assert_eq!(draft.wire_name(), written(&draft));
    }
    assert_eq!(Split::Sided.wire_name(), "written");
    assert_eq!(Split::Aliased.wire_name(), "Aliased");
}

#[test]
fn an_other_variant_is_written_as_its_own_name() {
    for draft in [Fallback::Known, Fallback::Unknown] {
        assert_eq!(draft.wire_name(), written(&draft));
    }
}

#[test]
fn an_enum_that_is_not_copy_is_read_by_reference() {
    let draft = Plain::Custom;
    let held = &draft;
    assert_eq!(held.wire_name(), "Custom");
    assert_eq!(draft.wire_name(), written(&draft));
}

#[test]
fn the_name_is_a_constant_and_a_valid_value_reaches_it_through_deref() {
    assert_eq!(MAX, "MaxIsCurrentDate");
    let bound: Dated = Dated::Custom;
    assert_eq!(bound.wire_name(), "Custom");
}
