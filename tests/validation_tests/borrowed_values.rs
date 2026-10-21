//! A bound on a field reached through a reference is held by `validate()` as one on the owned
//! field is, whatever the reference borrows, and a model held behind a reference is walked as the
//! owned model is.

/// One declaration with its values borrowed and one with them owned, under the same bounds.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod a_bound_behind_any_reference {
    use std::sync::LazyLock;

    use serde::Serialize;
    use tixschema::model_schema;

    static EMPTY: String = String::new();
    static NAME: LazyLock<String> = LazyLock::new(|| "alice".to_owned());
    static ONE_PART: [String; 1] = [String::new()];
    static TWO_PARTS: [String; 2] = [String::new(), String::new()];

    #[model_schema()]
    #[derive(Debug, Serialize)]
    pub struct LentLabels {
        #[model_schema_prop(minLength = 2)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub alias: Option<&'static String>,
        #[model_schema_prop(minimum = 5)]
        pub count: &'static u32,
        #[model_schema_prop(minLength = 3)]
        pub label: &'static String,
        #[model_schema_prop(maxLength = 1)]
        pub parts: &'static [String],
    }

    #[model_schema()]
    #[derive(Debug, Serialize)]
    pub struct OwnedLabels {
        #[model_schema_prop(minLength = 2)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub alias: Option<String>,
        #[model_schema_prop(minimum = 5)]
        pub count: u32,
        #[model_schema_prop(minLength = 3)]
        pub label: String,
        #[model_schema_prop(maxLength = 1)]
        pub parts: Vec<String>,
    }

    /// A reference whose walk ends on an `Option`, and one that holds another reference.
    #[model_schema()]
    #[derive(Debug, Serialize)]
    pub struct LentNote<'text> {
        #[model_schema_prop(maxLength = 4)]
        pub nested: &'text &'text String,
        #[model_schema_prop(minLength = 2)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub note: &'text Option<String>,
    }

    #[model_schema()]
    #[derive(Debug, Serialize)]
    #[serde(untagged)]
    pub enum LentHandle<'text> {
        Long {
            #[model_schema_prop(minLength = 3)]
            text: &'text String,
        },
        Noted {
            #[model_schema_prop(maxLength = 2)]
            #[serde(skip_serializing_if = "Option::is_none")]
            note: &'text Option<String>,
        },
    }

    fn broken() -> LentLabels {
        LentLabels {
            alias: Some(&EMPTY),
            count: &1,
            label: &EMPTY,
            parts: &TWO_PARTS,
        }
    }

    fn kept() -> LentLabels {
        LentLabels {
            alias: None,
            count: &7,
            label: &NAME,
            parts: &ONE_PART,
        }
    }

    #[test]
    fn validate_reports_each_value_behind_a_reference_that_breaks_its_bound() {
        assert_eq!(
            broken().validate().unwrap_err(),
            vec![
                "'alias': too short: minimum length is 2, got 0",
                "'count': too small: minimum is 5, got 1",
                "'label': too short: minimum length is 3, got 0",
                "'parts': too long: maximum length is 1, got 2",
            ]
        );
    }

    #[test]
    fn validate_answers_a_value_behind_a_reference_in_the_words_it_answers_the_owned_one() {
        let owned = OwnedLabels {
            alias: Some(String::new()),
            count: 1,
            label: String::new(),
            parts: TWO_PARTS.to_vec(),
        };
        assert_eq!(
            broken().validate().unwrap_err(),
            owned.validate().unwrap_err()
        );
    }

    #[test]
    fn a_value_behind_a_reference_that_keeps_its_bound_passes() {
        kept().validate().unwrap();

        let short_alias = LentLabels {
            alias: Some(&EMPTY),
            ..kept()
        };
        assert_eq!(
            short_alias.validate().unwrap_err(),
            vec!["'alias': too short: minimum length is 2, got 0"]
        );
    }

    #[test]
    fn a_reference_to_an_option_and_to_a_reference_are_read_through() {
        let long = "longer".to_owned();
        let short = Some("a".to_owned());
        assert_eq!(
            LentNote {
                nested: &&long,
                note: &short,
            }
            .validate()
            .unwrap_err(),
            vec![
                "'nested': too long: maximum length is 4, got 6",
                "'note': too short: minimum length is 2, got 1",
            ]
        );

        let held = "al".to_owned();
        LentNote {
            nested: &&held,
            note: &None,
        }
        .validate()
        .unwrap();
    }

    #[test]
    fn an_untagged_member_behind_a_reference_is_held_to_its_bound() {
        let short = "al".to_owned();
        assert_eq!(
            LentHandle::Long { text: &short }.validate().unwrap_err(),
            vec!["'text': too short: minimum length is 3, got 2"]
        );
        let long = Some("long".to_owned());
        assert_eq!(
            LentHandle::Noted { note: &long }.validate().unwrap_err(),
            vec!["'note': too long: maximum length is 2, got 4"]
        );

        LentHandle::Long { text: &NAME }.validate().unwrap();
    }
}

/// A model held behind each reference, and the same model held owned.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod a_model_behind_any_reference {
    use std::sync::LazyLock;

    use serde::Serialize;
    use tixschema::model_schema;

    static BLANK: Stamp = Stamp {
        code: String::new(),
    };
    static KEPT: LazyLock<Stamp> = LazyLock::new(|| Stamp {
        code: "abc".to_owned(),
    });
    static NOTED_BLANK: Option<Stamp> = Some(Stamp {
        code: String::new(),
    });
    static NOTED_NONE: Option<Stamp> = None;
    static SECOND_BLANK: LazyLock<Vec<Stamp>> = LazyLock::new(|| vec![KEPT.clone(), BLANK.clone()]);

    #[model_schema()]
    #[derive(Clone, Debug, Serialize)]
    pub struct Stamp {
        #[model_schema_prop(minLength = 3)]
        pub code: String,
    }

    #[model_schema()]
    #[derive(Debug, Serialize)]
    pub struct LentStamps {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub maybe: Option<&'static Stamp>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub noted: &'static Option<Stamp>,
        pub stamp: &'static Stamp,
        pub stamps: &'static [Stamp],
    }

    #[model_schema()]
    #[derive(Debug, Serialize)]
    pub struct OwnedStamps {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub maybe: Option<Stamp>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub noted: Option<Stamp>,
        pub stamp: Stamp,
        pub stamps: Vec<Stamp>,
    }

    fn kept() -> LentStamps {
        LentStamps {
            maybe: None,
            noted: &NOTED_NONE,
            stamp: &KEPT,
            stamps: &[],
        }
    }

    #[test]
    fn validate_walks_a_model_behind_each_reference_as_it_walks_the_owned_one() {
        let lent = LentStamps {
            maybe: Some(&BLANK),
            noted: &NOTED_BLANK,
            stamp: &BLANK,
            stamps: &SECOND_BLANK,
        };
        let owned = OwnedStamps {
            maybe: Some(BLANK.clone()),
            noted: NOTED_BLANK.clone(),
            stamp: BLANK.clone(),
            stamps: SECOND_BLANK.clone(),
        };
        assert_eq!(
            lent.validate().unwrap_err(),
            vec![
                "'maybe.code': too short: minimum length is 3, got 0",
                "'noted.code': too short: minimum length is 3, got 0",
                "'stamp.code': too short: minimum length is 3, got 0",
                "'stamps.1.code': too short: minimum length is 3, got 0",
            ]
        );
        assert_eq!(lent.validate(), owned.validate());
    }

    #[test]
    fn a_model_behind_an_optional_reference_is_walked_where_one_is_held() {
        kept().validate().unwrap();
        LentStamps {
            maybe: Some(&KEPT),
            ..kept()
        }
        .validate()
        .unwrap();
        assert_eq!(
            LentStamps {
                maybe: Some(&BLANK),
                ..kept()
            }
            .validate(),
            OwnedStamps {
                maybe: Some(BLANK.clone()),
                noted: None,
                stamp: KEPT.clone(),
                stamps: Vec::new(),
            }
            .validate()
        );
    }

    #[test]
    fn a_model_in_a_borrowed_list_is_named_by_its_index() {
        assert_eq!(
            LentStamps {
                stamps: &SECOND_BLANK,
                ..kept()
            }
            .validate()
            .unwrap_err(),
            vec!["'stamps.1.code': too short: minimum length is 3, got 0"]
        );
    }
}
