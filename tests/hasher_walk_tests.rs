//! A set that names its hasher is walked for nested validation exactly as the same set with the
//! hasher implied.

#[cfg(test)]
mod hasher_walk {
    use core::hash::BuildHasherDefault;
    use serde::de::DeserializeOwned;
    use serde::{Deserialize, Serialize};
    use serde_json::{Value, json};
    use std::collections::{HashMap, HashSet};
    use std::hash::DefaultHasher;
    use tixschema::model_schema;

    type NamedHasher = BuildHasherDefault<DefaultHasher>;

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
    struct Tag {
        #[model_schema_prop(minLength = 1)]
        label: String,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    struct NamedSet {
        tags: HashSet<Tag, NamedHasher>,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    struct ImpliedSet {
        tags: HashSet<Tag>,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    struct NamedInMap {
        groups: HashMap<String, HashSet<Tag, NamedHasher>>,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    struct ImpliedInMap {
        groups: HashMap<String, HashSet<Tag>>,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    struct NamedInOption {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tags: Option<HashSet<Tag, NamedHasher>>,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
    struct ImpliedInOption {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tags: Option<HashSet<Tag>>,
    }

    fn refusal<Read>(payload: &Value) -> String
    where
        Read: DeserializeOwned,
    {
        serde_json::from_value::<Read>(payload.clone())
            .err()
            .map_or_else(|| format!("{payload} was read"), |error| error.to_string())
    }

    fn refusal_from_str<Read>(payload: &str) -> String
    where
        Read: DeserializeOwned,
    {
        serde_json::from_str::<Read>(payload)
            .err()
            .map_or_else(|| format!("{payload} was read"), |error| error.to_string())
    }

    #[test]
    fn a_set_naming_its_hasher_refuses_as_the_implied_one() {
        let payload = r#"{"tags":[{"label":""}]}"#;
        assert_eq!(
            refusal_from_str::<NamedSet>(payload),
            refusal_from_str::<ImpliedSet>(payload)
        );
        assert_ne!(
            refusal_from_str::<NamedSet>(payload),
            format!("{payload} was read")
        );
    }

    #[test]
    fn a_set_naming_its_hasher_reports_as_the_implied_one() {
        let payload = json!({"tags": [{"label": ""}]});
        let named = NamedSet::model_schema_read_unjudged(payload.clone()).unwrap();
        let implied = ImpliedSet::model_schema_read_unjudged(payload).unwrap();
        assert_eq!(named.validation_issues(), implied.validation_issues());
        assert_eq!(
            named.validation_issues(),
            ["'tags.0.label': too short: minimum length is 1, got 0"]
        );
    }

    #[test]
    fn a_set_naming_its_hasher_in_a_map_value_behaves_as_the_implied_one() {
        let payload = json!({"groups": {"a": [{"label": ""}]}});
        assert_eq!(
            refusal::<NamedInMap>(&payload),
            refusal::<ImpliedInMap>(&payload)
        );
        let named = NamedInMap::model_schema_read_unjudged(payload.clone()).unwrap();
        let implied = ImpliedInMap::model_schema_read_unjudged(payload).unwrap();
        assert_eq!(named.validation_issues(), implied.validation_issues());
        assert_eq!(
            named.validation_issues(),
            ["'groups.a.0.label': too short: minimum length is 1, got 0"]
        );
    }

    #[test]
    fn a_set_naming_its_hasher_in_an_option_behaves_as_the_implied_one() {
        let payload = json!({"tags": [{"label": ""}]});
        assert_eq!(
            refusal::<NamedInOption>(&payload),
            refusal::<ImpliedInOption>(&payload)
        );
        let named = NamedInOption::model_schema_read_unjudged(payload.clone()).unwrap();
        let implied = ImpliedInOption::model_schema_read_unjudged(payload).unwrap();
        assert_eq!(named.validation_issues(), implied.validation_issues());
        assert_eq!(
            named.validation_issues(),
            ["'tags.0.label': too short: minimum length is 1, got 0"]
        );
    }
}
