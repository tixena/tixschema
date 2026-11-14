//! `validate()` of a type that declares a parameter is there at every filling of it, and a type
//! that holds one at any filling runs it.

#[cfg(feature = "serde")]
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
    pub fn validation_issues(&self) -> Vec<String> {
        if self.codes.contains(&0) {
            return vec!["'codes': no code is zero".to_owned()];
        }
        Vec::new()
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
    pub struct Code<CodeType>(CodeType);

    /// A brand over a list, counted whatever fills it.
    #[model_schema(no_display, minLength = 1, default_types(T = String))]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(transparent)]
    pub struct Codes<T>(Vec<T>);

    /// A brand whose declared default names a lifetime, which a borrowed filling differs in.
    #[model_schema(minLength = 2, default_types(Text = &'static str))]
    #[derive(Clone, Debug, Serialize)]
    #[serde(transparent)]
    pub struct Stamp<Text>(Text);

    #[model_schema(default_types(CodeType = String))]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Bin<CodeType> {
        pub code: Code<CodeType>,
        pub codes: Vec<Code<CodeType>>,
    }

    #[model_schema(default_types(CodeType = String))]
    #[derive(Clone, Debug, Deserialize, Serialize)]
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
        Crate::model_schema_wrap_unjudged(CrateDraft {
            codes: Vec::new(),
            label: String::new(),
        })
    }

    #[test]
    fn a_struct_is_held_to_its_bounds_at_a_filling_that_is_not_its_default() {
        assert_eq!(
            CrateDraft::<u32> {
                codes: vec![],
                label: String::new()
            }
            .validate()
            .unwrap_err(),
            [UNLABELLED]
        );
        assert_eq!(unlabelled::<String>().validation_issues(), [UNLABELLED]);
        assert_eq!(
            CrateDraft::<u32> {
                codes: vec![],
                label: "spares".to_owned()
            }
            .validation_issues(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_holder_reports_a_generic_type_held_at_a_filling_that_is_not_its_default() {
        let depot = DepotDraft {
            counted: unlabelled(),
            named: unlabelled(),
        };
        assert_eq!(
            depot.validation_issues(),
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
            SackDraft(String::new(), "a".to_owned()).validation_issues(),
            unlabelled_slot
        );
        assert_eq!(
            SackDraft(String::new(), 7_u32).validation_issues(),
            unlabelled_slot
        );
        assert_eq!(
            SackDraft("spares".to_owned(), 7_u32).validation_issues(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn an_enum_under_each_tagging_is_held_to_its_bounds_at_two_fillings() {
        assert_eq!(
            LoadExternalDraft::<String>::Packed {
                codes: vec![],
                label: String::new()
            }
            .validate()
            .unwrap_err(),
            ["'Packed.label': too short: minimum length is 1, got 0"]
        );
        assert_eq!(
            LoadExternalDraft::<u32>::Packed {
                codes: vec![],
                label: String::new()
            }
            .validate()
            .unwrap_err(),
            ["'Packed.label': too short: minimum length is 1, got 0"]
        );
        assert_eq!(
            LoadExternalDraft::<u32>::Unpacked.validation_issues(),
            Vec::<String>::new()
        );

        assert_eq!(
            LoadInternalDraft::<String>::Packed {
                codes: vec![],
                label: String::new()
            }
            .validate()
            .unwrap_err(),
            [UNLABELLED]
        );
        assert_eq!(
            LoadInternalDraft::<u32>::Packed {
                codes: vec![],
                label: String::new()
            }
            .validate()
            .unwrap_err(),
            [UNLABELLED]
        );
        assert_eq!(
            LoadInternalDraft::<u32>::Unpacked.validation_issues(),
            Vec::<String>::new()
        );

        assert_eq!(
            LoadAdjacentDraft::<String>::Packed {
                codes: vec![],
                label: String::new()
            }
            .validate()
            .unwrap_err(),
            ["'load.label': too short: minimum length is 1, got 0"]
        );
        assert_eq!(
            LoadAdjacentDraft::<u32>::Packed {
                codes: vec![],
                label: String::new()
            }
            .validate()
            .unwrap_err(),
            ["'load.label': too short: minimum length is 1, got 0"]
        );
        assert_eq!(
            LoadAdjacentDraft::<u32>::Unpacked.validation_issues(),
            Vec::<String>::new()
        );

        assert_eq!(
            LoadUntaggedDraft::<String>::Packed {
                codes: vec![],
                label: String::new()
            }
            .validate()
            .unwrap_err(),
            [UNLABELLED]
        );
        assert_eq!(
            LoadUntaggedDraft::<u32>::Packed {
                codes: vec![],
                label: String::new()
            }
            .validate()
            .unwrap_err(),
            [UNLABELLED]
        );
        assert_eq!(
            LoadUntaggedDraft::<u32>::Counted { count: 7 }.validation_issues(),
            Vec::<String>::new()
        );
    }

    /// The seven lines are one text: the brand alone, held by a generated holder, and held by a
    /// holder written by hand over a parameter with no bound.
    #[test]
    fn a_generic_brands_checks_run_at_its_declared_default_and_at_no_other_filling() {
        let borrowed = String::from("a");
        let reported = [
            format!(
                r#"Code<String>("a").validate():                 {:?}"#,
                Code::new("a".to_owned()).map(|_| ())
            ),
            format!(
                r#"Code<String>("ab").validate():                {:?}"#,
                Code::new("ab".to_owned()).map(|_| ())
            ),
            format!(
                "Code<u32>(7).validate():                      {:?}",
                Code::new(7_u32).map(|_| ())
            ),
            format!(
                r#"Bin<String> {{ code: "a", codes: ["ab", "b"] }}: {:?}"#,
                BinDraft {
                    code: Code::model_schema_read_unjudged(
                        serde_json::to_value("a".to_owned()).unwrap()
                    )
                    .unwrap(),
                    codes: vec![
                        Code::new("ab".to_owned()).unwrap(),
                        Code::model_schema_read_unjudged(
                            serde_json::to_value("b".to_owned()).unwrap()
                        )
                        .unwrap()
                    ],
                }
                .validate()
            ),
            format!(
                "Bin<u32> {{ code: 7, codes: [1] }}:              {:?}",
                BinDraft {
                    code: Code::new(7_u32).unwrap(),
                    codes: vec![Code::new(1_u32).unwrap()],
                }
                .validate()
                .map(|_| ())
            ),
            format!(
                r#"Plain<&str> {{ held: "a" }}:                    {:?}"#,
                Plain {
                    held: Code::new(borrowed.as_str()).unwrap(),
                }
                .validate()
            ),
            format!(
                r#"Plain<String> {{ held: "a" }}:                  {:?}"#,
                Plain {
                    held: Code::<String>::model_schema_read_unjudged(
                        serde_json::to_value(borrowed).unwrap()
                    )
                    .unwrap(),
                }
                .validate()
            ),
        ];
        assert_eq!(reported.join("\n"), BRAND_REPORT);
    }

    /// The brand's `validate()` asks nothing of what fills it: a filling with no `Display` has it.
    #[test]
    fn a_generic_brand_has_validate_at_a_filling_the_read_does_not_take() {
        assert_eq!(Code::new(vec![7_u8]).map(|_| ()), Ok(()));
        assert_eq!(Code::new(Some("a")).map(|_| ()), Ok(()));
    }

    /// A filling that differs from the declared default in a lifetime alone is that filling, so
    /// the value is read as it under the borrow `validate()` holds.
    #[test]
    fn a_default_that_names_a_lifetime_is_checked_at_a_shorter_one() {
        let too_short = ["too short: minimum length is 2, got 1"];
        let owned = String::from("a");
        assert_eq!(
            Stamp::new(owned.as_str()).map(|_| ()).unwrap_err(),
            too_short
        );
        assert_eq!(Stamp::new("a").map(|_| ()).unwrap_err(), too_short);
        assert_eq!(Stamp::new("ab").map(|_| ()), Ok(()));
        assert_eq!(Stamp::new(owned).map(|_| ()), Ok(()));
    }

    #[test]
    fn a_generic_holder_keeps_a_generic_brands_bound_in_a_field_a_list_and_an_option() {
        assert_eq!(
            BinDraft {
                code: Code::<String>::model_schema_read_unjudged(
                    serde_json::to_value("a".to_owned()).unwrap()
                )
                .unwrap(),
                codes: vec![
                    Code::model_schema_read_unjudged(serde_json::to_value("b".to_owned()).unwrap())
                        .unwrap()
                ],
            }
            .validate()
            .unwrap_err(),
            [
                "'code': too short: minimum length is 2, got 1",
                "'codes.0': too short: minimum length is 2, got 1",
            ]
        );
        assert_eq!(
            RackDraft {
                spare: Some(
                    Code::<String>::model_schema_read_unjudged(
                        serde_json::to_value("c".to_owned()).unwrap()
                    )
                    .unwrap()
                ),
            }
            .validate()
            .unwrap_err(),
            ["'spare': too short: minimum length is 2, got 1"]
        );
        assert_eq!(
            RackDraft {
                spare: Some(Code::new(7_u32).unwrap()),
            }
            .validation_issues(),
            Vec::<String>::new()
        );
        assert_eq!(
            RackDraft::<String> { spare: None }.validation_issues(),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_brand_over_a_list_counts_its_items_at_every_filling() {
        let empty = ["too short: minimum length is 1, got 0"];
        assert_eq!(Codes::<u32>::new(Vec::new()).unwrap_err(), empty);
        assert_eq!(Codes::<String>::new(Vec::new()).unwrap_err(), empty);
        assert_eq!(Codes::new(vec![7_u32]).map(|_| ()), Ok(()));
        assert_eq!(Codes::new(vec![String::new()]).map(|_| ()), Ok(()));
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
            Code::new("ab".to_owned()).unwrap()
        );
        assert_eq!(
            serde_json::from_str::<Code<u32>>("7").unwrap(),
            Code::new(7_u32).unwrap()
        );
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
            Code::new(7_u32).unwrap()
        );
    }

    /// `validate()` reads nothing behind a type parameter, and what fills one judges itself as it
    /// is read. The README shows both as a run answers them.
    #[test]
    fn a_value_behind_a_type_parameter_is_judged_by_its_own_read_alone() {
        let outer = CrateDraft {
            codes: vec![unlabelled::<u32>()],
            label: "outer".to_owned(),
        };
        let payload = serde_json::to_string(&outer).unwrap();
        let refused = serde_json::from_str::<Crate<Crate<u32>>>(&payload)
            .unwrap_err()
            .to_string();
        let readme = include_str!("../../docs/guide/validation.md");
        for shown in [
            format!("outer.validate();\n// {:?}\n", outer.validate()),
            format!(
                "serde_json::from_str::<Crate<Crate<u32>>>(r#\"{payload}\"#);\n// Err: \
                 {refused}\n"
            ),
        ] {
            assert!(
                readme.contains(&shown),
                "the README no longer shows:\n{shown}"
            );
        }
    }

    /// What the README and the crate's rustdoc show of a generic type's `validate()` is what a
    /// run answers, and the refusal they quote is the one the emitter's doc comment records.
    #[test]
    fn the_documents_show_validate_at_every_filling() {
        let readme = concat!(
            include_str!("../../docs/guide/generics-and-brands.md"),
            include_str!("../../docs/guide/validation.md")
        );
        assert!(
            readme.contains(&format!("```text\n{BRAND_REPORT}\n```")),
            "the README no longer shows the brand's report"
        );
        let depot = DepotDraft {
            counted: unlabelled(),
            named: unlabelled(),
        };
        for shown in [
            format!(
                "{:?}",
                CrateDraft::<u32> {
                    codes: Vec::new(),
                    label: String::new(),
                }
                .validate()
            ),
            format!("{:?}", depot.validate()),
        ] {
            assert!(
                readme.contains(&format!("// {shown}\n")),
                "the README no longer shows `{shown}`"
            );
        }

        let refusal = "error[E0592]: duplicate definitions with name `validation_issues`";
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
