//! A set, a deque and a heap are walked only where they hold a declared type: a type whose only
//! member is a set of strings holds no rule and publishes no `validate()`.

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod a_set_of_values_no_declared_type_describes {
    use alloc::collections::BTreeSet;
    use serde::{Deserialize, Serialize};
    use std::collections::HashSet;
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TallyLine {
        #[model_schema_prop(minLength = 1)]
        pub item_label: String,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TallyNames {
        pub dock_names: HashSet<String>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TallyOwn {
        pub dock_names: HashSet<String>,
    }

    impl TallyOwn {
        pub fn validate(&self) -> usize {
            self.dock_names.len()
        }
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TallyFleet {
        pub lines: BTreeSet<TallyLine>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct TallyHolder {
        pub fleet: TallyFleet,
        pub names: TallyNames,
    }

    #[test]
    fn a_type_whose_only_member_is_a_set_of_strings_publishes_no_validate() {
        let own = TallyOwn {
            dock_names: HashSet::from(["north".to_owned()]),
        };
        assert_eq!(own.validate(), 1);
    }

    #[test]
    fn a_set_of_declared_types_is_walked_and_named_by_the_index_of_each_item() {
        let fleet = TallyFleet {
            lines: BTreeSet::from([TallyLine {
                item_label: String::new(),
            }]),
        };
        assert_eq!(
            fleet.validate(),
            Err(vec![
                "'lines.0.itemLabel': too short: minimum length is 1, got 0".to_owned()
            ])
        );
    }

    #[test]
    fn a_holder_of_such_a_type_validates_through_the_fallback() {
        let holder = TallyHolder {
            names: TallyNames {
                dock_names: HashSet::from(["north".to_owned()]),
            },
            fleet: TallyFleet {
                lines: BTreeSet::new(),
            },
        };
        assert_eq!(holder.validate(), Ok(()));
    }
}
