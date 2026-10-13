//! `validate()` of a type that declares a parameter is there at every filling of it, and a type
//! that holds one at any filling runs it.

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod validate_at_every_filling {
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    /// What a generic brand bounded at its declared default answers, alone and held, which the
    /// README prints as it is here.
    const BRAND_REPORT: &str = r#"Code<String>("a").validate():                 Err(["too short: minimum length is 2, got 1"])
Code<String>("ab").validate():                Ok(())
Code<u32>(7).validate():                      Ok(())
Bin<String> { code: "a", codes: ["ab", "b"] }: Err(["'code': too short: minimum length is 2, got 1", "'codes.1': too short: minimum length is 2, got 1"])
Bin<u32> { code: 7, codes: [1] }:              Ok(())
Plain<&str> { held: "a" }:                    Ok(())
Plain<String> { held: "a" }:                  Err(["too short: minimum length is 2, got 1"])"#;

    /// The `validate` of a consumer's own on one filling, as the README shows it refused.
    const OWN_VALIDATE: &str = r#"impl Crate<u32> {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        if self.codes.contains(&0) {
            return Err(vec!["'codes': no code is zero".to_owned()]);
        }
        Ok(())
    }
}"#;

    const UNLABELLED: &str = "'label': too short: minimum length is 1, got 0";

    #[model_schema(default_types(CodeType = String))]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Crate<CodeType> {
        pub codes: Vec<CodeType>,
        #[model_schema_prop(minLength = 1)]
        pub label: String,
    }

    /// Two crates, the first at a filling that is not the declared default.
    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Depot {
        #[serde(rename = "numbered")]
        pub counted: Crate<u32>,
        pub named: Crate<String>,
    }

    #[model_schema(default_types(CodeType = String))]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Sack<CodeType>(#[model_schema_prop(minLength = 1)] pub String, pub CodeType);

    #[model_schema(default_types(CodeType = String))]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub enum LoadExternal<CodeType> {
        Packed {
            codes: Vec<CodeType>,
            #[model_schema_prop(minLength = 1)]
            label: String,
        },
        Unpacked,
    }

    #[model_schema(default_types(CodeType = String))]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(tag = "kind")]
    pub enum LoadInternal<CodeType> {
        Packed {
            codes: Vec<CodeType>,
            #[model_schema_prop(minLength = 1)]
            label: String,
        },
        Unpacked,
    }

    #[model_schema(default_types(CodeType = String))]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(tag = "kind", content = "load")]
    pub enum LoadAdjacent<CodeType> {
        Packed {
            codes: Vec<CodeType>,
            #[model_schema_prop(minLength = 1)]
            label: String,
        },
        Unpacked,
    }

    #[model_schema(default_types(CodeType = String))]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(untagged)]
    pub enum LoadUntagged<CodeType> {
        Counted {
            count: u32,
        },
        Packed {
            codes: Vec<CodeType>,
            #[model_schema_prop(minLength = 1)]
            label: String,
        },
    }

    /// A brand whose bound is its declared default's.
    #[model_schema(minLength = 2, default_types(CodeType = String))]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct Code<CodeType>(pub CodeType);

    /// A brand over a list, counted whatever fills it.
    #[model_schema(no_display, minLength = 1, default_types(T = String))]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(transparent)]
    pub struct Codes<T>(pub Vec<T>);

    /// A brand whose declared default names a lifetime, which a borrowed filling differs in.
    #[model_schema(minLength = 2, default_types(Text = &'static str))]
    #[derive(Clone, Debug, Serialize)]
    #[serde(transparent)]
    pub struct Stamp<Text>(pub Text);

    #[model_schema(default_types(CodeType = String))]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(bound(deserialize = "CodeType: serde::de::DeserializeOwned + core::fmt::Display"))]
    pub struct Bin<CodeType> {
        pub code: Code<CodeType>,
        pub codes: Vec<Code<CodeType>>,
    }

    #[model_schema(default_types(CodeType = String))]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(bound(deserialize = "CodeType: serde::de::DeserializeOwned + core::fmt::Display"))]
    pub struct Rack<CodeType> {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub spare: Option<Code<CodeType>>,
    }

    /// A holder written by hand, whose parameter carries no bound.
    pub struct Plain<Held> {
        pub held: Code<Held>,
    }

    impl<Held> Plain<Held> {
        pub fn validate(&self) -> Result<(), Vec<String>> {
            self.held.validate()
        }
    }

    fn unlabelled<CodeType>() -> Crate<CodeType> {
        Crate {
            codes: Vec::new(),
            label: String::new(),
        }
    }

    #[test]
    fn a_struct_is_held_to_its_bounds_at_a_filling_that_is_not_its_default() {
        assert_eq!(
            Crate::<u32> {
                codes: vec![],
                label: String::new()
            }
            .validate()
            .unwrap_err(),
            [UNLABELLED]
        );
        assert_eq!(unlabelled::<String>().validate().unwrap_err(), [UNLABELLED]);
        assert_eq!(
            Crate::<u32> {
                codes: vec![],
                label: "spares".to_owned()
            }
            .validate(),
            Ok(())
        );
    }

    #[test]
    fn a_holder_reports_a_generic_type_held_at_a_filling_that_is_not_its_default() {
        let depot = Depot {
            counted: unlabelled(),
            named: unlabelled(),
        };
        assert_eq!(
            depot.validate().unwrap_err(),
            [
                "'numbered.label': too short: minimum length is 1, got 0",
                "'named.label': too short: minimum length is 1, got 0",
            ]
        );
    }

    #[test]
    fn a_tuple_struct_is_held_to_its_bounds_at_two_fillings() {
        let unlabelled_slot = ["'0': too short: minimum length is 1, got 0"];
        assert_eq!(
            Sack(String::new(), "a".to_owned()).validate().unwrap_err(),
            unlabelled_slot
        );
        assert_eq!(
            Sack(String::new(), 7_u32).validate().unwrap_err(),
            unlabelled_slot
        );
        assert_eq!(Sack("spares".to_owned(), 7_u32).validate(), Ok(()));
    }

    #[test]
    fn an_enum_under_each_tagging_is_held_to_its_bounds_at_two_fillings() {
        assert_eq!(
            LoadExternal::<String>::Packed {
                codes: vec![],
                label: String::new()
            }
            .validate()
            .unwrap_err(),
            [UNLABELLED]
        );
        assert_eq!(
            LoadExternal::<u32>::Packed {
                codes: vec![],
                label: String::new()
            }
            .validate()
            .unwrap_err(),
            [UNLABELLED]
        );
        assert_eq!(LoadExternal::<u32>::Unpacked.validate(), Ok(()));

        assert_eq!(
            LoadInternal::<String>::Packed {
                codes: vec![],
                label: String::new()
            }
            .validate()
            .unwrap_err(),
            [UNLABELLED]
        );
        assert_eq!(
            LoadInternal::<u32>::Packed {
                codes: vec![],
                label: String::new()
            }
            .validate()
            .unwrap_err(),
            [UNLABELLED]
        );
        assert_eq!(LoadInternal::<u32>::Unpacked.validate(), Ok(()));

        assert_eq!(
            LoadAdjacent::<String>::Packed {
                codes: vec![],
                label: String::new()
            }
            .validate()
            .unwrap_err(),
            [UNLABELLED]
        );
        assert_eq!(
            LoadAdjacent::<u32>::Packed {
                codes: vec![],
                label: String::new()
            }
            .validate()
            .unwrap_err(),
            [UNLABELLED]
        );
        assert_eq!(LoadAdjacent::<u32>::Unpacked.validate(), Ok(()));

        assert_eq!(
            LoadUntagged::<String>::Packed {
                codes: vec![],
                label: String::new()
            }
            .validate()
            .unwrap_err(),
            [UNLABELLED]
        );
        assert_eq!(
            LoadUntagged::<u32>::Packed {
                codes: vec![],
                label: String::new()
            }
            .validate()
            .unwrap_err(),
            [UNLABELLED]
        );
        assert_eq!(LoadUntagged::<u32>::Counted { count: 7 }.validate(), Ok(()));
    }

    /// The seven lines are one text: the brand alone, held by a generated holder, and held by a
    /// holder written by hand over a parameter with no bound.
    #[test]
    fn a_generic_brands_checks_run_at_its_declared_default_and_at_no_other_filling() {
        let borrowed = String::from("a");
        let reported = [
            format!(
                r#"Code<String>("a").validate():                 {:?}"#,
                Code("a".to_owned()).validate()
            ),
            format!(
                r#"Code<String>("ab").validate():                {:?}"#,
                Code("ab".to_owned()).validate()
            ),
            format!(
                "Code<u32>(7).validate():                      {:?}",
                Code(7_u32).validate()
            ),
            format!(
                r#"Bin<String> {{ code: "a", codes: ["ab", "b"] }}: {:?}"#,
                Bin {
                    code: Code("a".to_owned()),
                    codes: vec![Code("ab".to_owned()), Code("b".to_owned())],
                }
                .validate()
            ),
            format!(
                "Bin<u32> {{ code: 7, codes: [1] }}:              {:?}",
                Bin {
                    code: Code(7_u32),
                    codes: vec![Code(1_u32)],
                }
                .validate()
            ),
            format!(
                r#"Plain<&str> {{ held: "a" }}:                    {:?}"#,
                Plain {
                    held: Code(borrowed.as_str()),
                }
                .validate()
            ),
            format!(
                r#"Plain<String> {{ held: "a" }}:                  {:?}"#,
                Plain {
                    held: Code(borrowed),
                }
                .validate()
            ),
        ];
        assert_eq!(reported.join("\n"), BRAND_REPORT);
    }

    /// The brand's `validate()` asks nothing of what fills it: a filling with no `Display` has it.
    #[test]
    fn a_generic_brand_has_validate_at_a_filling_the_read_does_not_take() {
        assert_eq!(Code(vec![7_u8]).validate(), Ok(()));
        assert_eq!(Code(Some("a")).validate(), Ok(()));
    }

    /// A filling that differs from the declared default in a lifetime alone is that filling, so
    /// the value is read as it under the borrow `validate()` holds.
    #[test]
    fn a_default_that_names_a_lifetime_is_checked_at_a_shorter_one() {
        let too_short = ["too short: minimum length is 2, got 1"];
        let owned = String::from("a");
        assert_eq!(Stamp(owned.as_str()).validate().unwrap_err(), too_short);
        assert_eq!(Stamp("a").validate().unwrap_err(), too_short);
        assert_eq!(Stamp("ab").validate(), Ok(()));
        assert_eq!(Stamp(owned).validate(), Ok(()));
    }

    #[test]
    fn a_generic_holder_keeps_a_generic_brands_bound_in_a_field_a_list_and_an_option() {
        assert_eq!(
            Bin {
                code: Code("a".to_owned()),
                codes: vec![Code("b".to_owned())],
            }
            .validate()
            .unwrap_err(),
            [
                "'code': too short: minimum length is 2, got 1",
                "'codes.0': too short: minimum length is 2, got 1",
            ]
        );
        assert_eq!(
            Rack {
                spare: Some(Code("c".to_owned())),
            }
            .validate()
            .unwrap_err(),
            ["'spare': too short: minimum length is 2, got 1"]
        );
        assert_eq!(
            Rack {
                spare: Some(Code(7_u32)),
            }
            .validate(),
            Ok(())
        );
        assert_eq!(Rack::<String> { spare: None }.validate(), Ok(()));
    }

    #[test]
    fn a_brand_over_a_list_counts_its_items_at_every_filling() {
        let empty = ["too short: minimum length is 1, got 0"];
        assert_eq!(Codes::<u32>(Vec::new()).validate().unwrap_err(), empty);
        assert_eq!(Codes::<String>(Vec::new()).validate().unwrap_err(), empty);
        assert_eq!(Codes(vec![7_u32]).validate(), Ok(()));
        assert_eq!(Codes(vec![String::new()]).validate(), Ok(()));
    }

    /// The read is the one the brand had: it refuses its declared default alone.
    #[test]
    fn the_read_of_a_generic_brand_refuses_and_reads_what_it_did() {
        assert_eq!(
            serde_json::from_str::<Code<String>>(r#""a""#)
                .unwrap_err()
                .to_string(),
            "too short: minimum length is 2, got 1"
        );
        assert_eq!(
            serde_json::from_str::<Code<String>>(r#""ab""#).unwrap(),
            Code("ab".to_owned())
        );
        assert_eq!(serde_json::from_str::<Code<u32>>("7").unwrap(), Code(7));
        assert!(
            serde_json::from_str::<Bin<String>>(r#"{"code":"a","codes":[]}"#)
                .unwrap_err()
                .to_string()
                .contains("too short: minimum length is 2, got 1")
        );
        assert_eq!(
            serde_json::from_str::<Bin<u32>>(r#"{"code":7,"codes":[1]}"#)
                .unwrap()
                .code,
            Code(7)
        );
    }

    /// What the README and the crate's rustdoc show of a generic type's `validate()` is what a
    /// run answers, and the refusal they quote is the one the emitter's doc comment records.
    #[test]
    fn the_documents_show_validate_at_every_filling() {
        let readme = include_str!("../../README.md");
        assert!(
            readme.contains(&format!("```text\n{BRAND_REPORT}\n```")),
            "the README no longer shows the brand's report"
        );
        let depot = Depot {
            counted: unlabelled(),
            named: unlabelled(),
        };
        for shown in [
            format!("{:?}", unlabelled::<u32>().validate()),
            format!("{:?}", depot.validate()),
        ] {
            assert!(
                readme.contains(&format!("// {shown}\n")),
                "the README no longer shows `{shown}`"
            );
        }

        let refusal = "error[E0592]: duplicate definitions with name `validate`";
        let emitter = include_str!("../../src/model_schema.rs");
        assert!(
            readme.contains(&format!("```rust,compile_fail\n{OWN_VALIDATE}\n```")),
            "the README no longer shows the consumer's own `validate`"
        );
        assert!(
            readme.contains(&format!("```text\n{refusal}\n```")),
            "the README no longer shows `{refusal}`"
        );
        let recorded = OWN_VALIDATE
            .lines()
            .map(|line| format!("/// {line}"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            emitter.contains(&recorded),
            "the emitter's doc comment no longer holds the consumer's own `validate`"
        );
        assert!(
            emitter.contains(&format!("/// {refusal}\n")),
            "the emitter's doc comment no longer records `{refusal}`"
        );
        assert!(
            include_str!("../../src/lib.rs").contains("duplicate definition (`E0592`)"),
            "the rustdoc no longer names the refusal"
        );
    }
}
