//! Tests of the generated `validate()`: every bound, through every wrapper and shape that carries
//! one.

/// What a message's own validator does about a bound declared not on one of its fields but on a
/// field's *type* — a constrained brand, or a nested `#[model_schema()]` type of its own.
///
/// The types live in a module rather than in the test bodies because a reference to a sibling type
/// is written against that type's schema module, which a type declared inside a `fn` does not
/// publish anywhere a sibling can name.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod a_bound_the_fields_own_type_declares {
    use super::UnpublishedValidate;
    use alloc::borrow::Cow;
    use alloc::sync::Arc;
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema(minLength = 3)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct Slug(pub String);

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct BrandHolder {
        pub slug: Slug,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Wrapped {
        pub boxed: Box<Slug>,
        pub fixed: [Slug; 2],
        pub listed: Vec<Slug>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub maybe: Option<Slug>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub maybe_listed: Option<Vec<Slug>>,
    }

    #[model_schema()]
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Held {
        #[model_schema_prop(minLength = 3)]
        pub name: String,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Holder {
        pub holds: Held,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(tag = "kind")]
    pub enum Tagged {
        Named { slug: Slug },
        Plain { count: u32 },
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(untagged)]
    pub enum CheckedThenLoose {
        Checked { slug: Slug },
        Loose { slug: String },
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct RenderedThroughout {
        pub count: u32,
        pub name: String,
        pub tags: Vec<String>,
        pub whether: bool,
    }

    impl UnpublishedValidate for RenderedThroughout {}

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct WrappedHeld {
        /// A plain unconstrained sibling, so no row of the matrix is measured on a type whose
        /// every field is a declared one. `aud` beside `claims` is the shape an account context
        /// is written in, and a walk that only survived where it was the type's sole field would
        /// pass a matrix built without one.
        pub aud: String,
        pub boxed: Box<Held>,
        pub cow: Cow<'static, Held>,
        pub listed: Vec<Held>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub maybe: Option<Held>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub maybe_listed: Option<Vec<Held>>,
        pub plain: Held,
        pub shared: Arc<Held>,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct Claims {
        #[model_schema_prop(minLength = 1)]
        pub jti: String,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct PlainAccount {
        pub aud: String,
        pub claims: Claims,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct SoleField {
        pub claims: Claims,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct DeepAccount {
        pub claims: Claims,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct DeepEnvelope {
        pub account: DeepAccount,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct FlatAccount {
        pub aud: String,
        #[serde(flatten)]
        pub claims: Claims,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct FlatEnvelope {
        pub account: FlatAccount,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(untagged)]
    pub enum UntaggedFlat {
        Bearer {
            aud: String,
            #[serde(flatten)]
            claims: Claims,
        },
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(tag = "kind")]
    pub enum TaggedFlat {
        Bearer {
            aud: String,
            #[serde(flatten)]
            claims: Claims,
        },
    }

    fn long() -> Slug {
        Slug("abc".to_owned())
    }

    fn short() -> Slug {
        Slug("a".to_owned())
    }

    #[test]
    fn test_a_message_holding_a_constrained_brand_is_refused_by_its_own_validator() {
        assert_eq!(
            short().validate().unwrap_err(),
            vec!["too short: minimum length is 3, got 1".to_owned()],
            "the brand names no field of its own, which is the whole reason the holder has to"
        );
        assert_eq!(
            BrandHolder { slug: short() }.validate().unwrap_err(),
            vec!["'slug': too short: minimum length is 3, got 1".to_owned()],
            "a message publishing no validator would have answered Ok(()) and handed an \
             implementation a Slug violating its own declared pattern"
        );

        // The other direction, over the wire the brand is actually written on: a value the bound
        // admits reads back as the message and then validates clean.
        let read: BrandHolder = serde_json::from_str(r#"{"slug":"abc"}"#).unwrap();
        assert_eq!(read.slug, long());
        read.validate().unwrap();

        // The README prints this report beside the declaration it comes from, so it is held to a
        // run of the generator rather than to memory.
        let readme = include_str!("../../README.md");
        let shown = "// Err([\"'slug': too short: minimum length is 3, got 1\"])";
        assert!(readme.contains(shown), "the README no longer shows {shown}");
    }

    #[test]
    fn test_a_brands_bound_is_reached_through_every_wrapper_the_field_is_written_under() {
        assert_eq!(
            Wrapped {
                boxed: Box::new(long()),
                fixed: [long(), long()],
                listed: vec![long()],
                maybe: None,
                maybe_listed: None,
            }
            .validate(),
            Ok(()),
            "a None writes nothing, so there is nothing for the bound to describe"
        );

        assert_eq!(
            Wrapped {
                boxed: Box::new(short()),
                fixed: [long(), short()],
                listed: vec![long(), short()],
                maybe: Some(short()),
                maybe_listed: Some(vec![short()]),
            }
            .validate()
            .unwrap_err(),
            vec![
                "'boxed': too short: minimum length is 3, got 1".to_owned(),
                "'fixed.1': too short: minimum length is 3, got 1".to_owned(),
                "'listed.1': too short: minimum length is 3, got 1".to_owned(),
                "'maybe': too short: minimum length is 3, got 1".to_owned(),
                "'maybe_listed.0': too short: minimum length is 3, got 1".to_owned(),
            ],
        );
    }

    #[test]
    fn test_a_nested_types_own_report_is_carried_up_under_the_field_that_held_it() {
        // The holder reads what it holds unjudged and judges the whole value once, so the read
        // refuses with the holder's own report.
        assert_eq!(
            serde_json::from_str::<Holder>(r#"{"holds":{"name":"a"}}"#)
                .unwrap_err()
                .to_string(),
            "'holds.name': too short: minimum length is 3, got 1"
        );
        let built = Holder {
            holds: Held {
                name: "a".to_owned(),
            },
        };
        assert_eq!(
            built.holds.validate().unwrap_err(),
            vec!["'name': too short: minimum length is 3, got 1".to_owned()]
        );
        assert_eq!(
            built.validate().unwrap_err(),
            vec!["'holds.name': too short: minimum length is 3, got 1".to_owned()]
        );

        let good: Holder = serde_json::from_str(r#"{"holds":{"name":"abc"}}"#).unwrap();
        good.validate().unwrap();

        let readme = include_str!("../../README.md");
        assert!(
            readme.contains("`'holds.name': too short: ...`"),
            "the README no longer shows what a nested report reads as"
        );
    }

    #[test]
    fn test_a_tagged_variants_branded_member_is_reached_by_the_enums_validator() {
        assert_eq!(
            Tagged::Named { slug: short() }.validate().unwrap_err(),
            vec!["'slug': too short: minimum length is 3, got 1".to_owned()]
        );
        Tagged::Named { slug: long() }.validate().unwrap();
        assert_eq!(
            Tagged::Plain { count: 0 }.validate(),
            Ok(()),
            "a variant carrying nothing to reach is left unread by the arm"
        );
    }

    #[test]
    fn test_an_untagged_members_brand_still_chooses_the_variant_on_the_read() {
        let checked: CheckedThenLoose = serde_json::from_str(r#"{"slug":"abc"}"#).unwrap();
        assert!(
            matches!(&checked, CheckedThenLoose::Checked { slug } if *slug == long()),
            "got: {checked:?}"
        );
        checked.validate().unwrap();

        let loose: CheckedThenLoose = serde_json::from_str(r#"{"slug":"a"}"#).unwrap();
        assert!(
            matches!(&loose, CheckedThenLoose::Loose { slug } if slug == "a"),
            "the bound took the first member out of the running rather than ending the read. \
             Got: {loose:?}"
        );
        assert_eq!(
            loose.validate(),
            Ok(()),
            "the member that was chosen declares no bound of its own"
        );
    }

    #[test]
    fn test_a_message_made_of_values_the_crate_renders_itself_publishes_no_validator() {
        assert_eq!(
            RenderedThroughout {
                count: 0,
                name: String::new(),
                tags: Vec::new(),
                whether: false,
            }
            .validate(),
            "no inherent validate()"
        );
    }

    #[test]
    fn test_a_nested_types_bound_is_reached_through_every_wrapper_the_field_is_written_under() {
        fn held(name: &str) -> Held {
            Held {
                name: name.to_owned(),
            }
        }
        fn all_good() -> WrappedHeld {
            WrappedHeld {
                aud: "acme".to_owned(),
                boxed: Box::new(held("abc")),
                cow: Cow::Owned(held("abc")),
                listed: vec![held("abc")],
                maybe: Some(held("abc")),
                maybe_listed: Some(vec![held("abc")]),
                plain: held("abc"),
                shared: Arc::new(held("abc")),
            }
        }

        assert_eq!(all_good().validate(), Ok(()));

        let mut boxed = all_good();
        boxed.boxed = Box::new(held("a"));
        let mut cow = all_good();
        cow.cow = Cow::Owned(held("a"));
        let mut listed = all_good();
        // Only the second element breaks its bound, so an element past the first has to be walked
        // for this to be reported at all.
        listed.listed = vec![held("abc"), held("a")];
        let mut maybe = all_good();
        maybe.maybe = Some(held("a"));
        let mut maybe_listed = all_good();
        maybe_listed.maybe_listed = Some(vec![held("a")]);
        let mut plain = all_good();
        plain.plain = held("a");
        let mut shared = all_good();
        shared.shared = Arc::new(held("a"));

        for (field, broken) in [
            ("boxed", boxed),
            ("cow", cow),
            ("listed.1", listed),
            ("maybe", maybe),
            ("maybe_listed.0", maybe_listed),
            ("plain", plain),
            ("shared", shared),
        ] {
            assert_eq!(
                broken.validate().unwrap_err(),
                vec![format!(
                    "'{field}.name': too short: minimum length is 3, got 1"
                )],
                "`{field}` holds a type whose own bound was broken and validate() said nothing"
            );
        }

        let none_written = WrappedHeld {
            maybe: None,
            maybe_listed: None,
            ..all_good()
        };
        assert_eq!(
            none_written.validate(),
            Ok(()),
            "a None writes nothing, so there is nothing for the bound to describe"
        );
    }

    #[test]
    fn test_a_nested_field_keeps_its_walk_beside_a_plain_sibling() {
        assert_eq!(
            serde_json::from_str::<PlainAccount>(r#"{"aud":"acme","claims":{"jti":""}}"#)
                .unwrap_err()
                .to_string(),
            "'claims.jti': too short: minimum length is 1, got 0"
        );
        assert_eq!(
            serde_json::from_str::<PlainAccount>(r#"{"aud":"acme","claims":{"jti":"a"}}"#)
                .unwrap()
                .validate(),
            Ok(())
        );
        assert_eq!(
            serde_json::from_str::<SoleField>(r#"{"claims":{"jti":""}}"#)
                .unwrap_err()
                .to_string(),
            "'claims.jti': too short: minimum length is 1, got 0",
            "the sole-field shape has to keep answering what it always did"
        );
    }

    /// One hop working is what made this look closed, so the walk is pinned at two.
    #[test]
    fn test_a_bound_two_hops_down_is_reached_and_names_the_whole_path() {
        assert_eq!(
            serde_json::from_str::<DeepEnvelope>(r#"{"account":{"claims":{"jti":""}}}"#)
                .unwrap_err()
                .to_string(),
            "'account.claims.jti': too short: minimum length is 1, got 0"
        );
        assert_eq!(
            serde_json::from_str::<DeepEnvelope>(r#"{"account":{"claims":{"jti":"a"}}}"#)
                .unwrap()
                .validate(),
            Ok(())
        );
    }

    #[test]
    fn test_a_flattened_hop_keeps_the_walk_and_contributes_no_segment() {
        assert_eq!(
            serde_json::from_str::<FlatEnvelope>(r#"{"account":{"aud":"acme","jti":""}}"#)
                .unwrap_err()
                .to_string(),
            "'account.jti': too short: minimum length is 1, got 0"
        );
        assert_eq!(
            serde_json::from_str::<FlatEnvelope>(r#"{"account":{"aud":"acme","jti":"a"}}"#)
                .unwrap()
                .validate(),
            Ok(())
        );
    }

    #[test]
    fn test_an_untagged_members_flattened_field_keeps_its_walk() {
        assert_eq!(
            serde_json::from_str::<UntaggedFlat>(r#"{"aud":"acme","jti":""}"#)
                .unwrap_err()
                .to_string(),
            "'jti': too short: minimum length is 1, got 0"
        );
        assert_eq!(
            serde_json::from_str::<UntaggedFlat>(r#"{"aud":"acme","jti":"a"}"#)
                .unwrap()
                .validate(),
            Ok(())
        );
    }

    #[test]
    fn test_a_tagged_variants_flattened_member_keeps_its_walk() {
        assert_eq!(
            serde_json::from_str::<TaggedFlat>(r#"{"kind":"Bearer","aud":"acme","jti":""}"#)
                .unwrap_err()
                .to_string(),
            "'jti': too short: minimum length is 1, got 0"
        );
        assert_eq!(
            serde_json::from_str::<TaggedFlat>(r#"{"kind":"Bearer","aud":"acme","jti":"a"}"#)
                .unwrap()
                .validate(),
            Ok(())
        );
    }
}

/// What a message's own validator does about a bound reached through an `#[serde(untagged)]` enum
/// — one it holds as a field, and one the message itself is.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod a_bound_inside_an_untagged_variant {
    use super::UnpublishedValidate;
    use serde::{Deserialize, Serialize};
    use tixschema::model_schema;

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct WireClaims {
        #[model_schema_prop(minLength = 1)]
        pub jti: String,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AppUserAccount {
        pub aud: String,
        #[serde(flatten)]
        pub claims: WireClaims,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AdminAccount {
        #[serde(flatten)]
        pub claims: WireClaims,
        pub sys_admin_username: String,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SharedLinkAccount {
        #[serde(flatten)]
        pub claims: WireClaims,
        pub token_id: String,
    }

    /// A union of newtype members held by another union's newtype member, which is how a shared-link
    /// caller is written: the account a message declares is a union, and one of its members is a
    /// union in its own right.
    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(untagged)]
    pub enum SharedAccount {
        Link(SharedLinkAccount),
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(untagged)]
    pub enum ScopedAccount {
        Admin(AdminAccount),
        AppUser(AppUserAccount),
        Shared(SharedAccount),
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ScopedEnvelope {
        pub account: ScopedAccount,
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AdminEnvelope {
        pub account: AdminAccount,
    }

    /// The envelope itself as a union, which is the shape a balance request is declared in.
    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(untagged)]
    pub enum EitherEnvelope {
        Admin(AdminEnvelope),
        Scoped(ScopedEnvelope),
    }

    #[model_schema(minLength = 3)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct Tag(pub String);

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(untagged)]
    /// `Bounded` is declared first because an untagged read tries its members in order and
    /// `Free` admits every string: the bound is what takes the first member out of the running,
    /// and a `Free` ahead of it would take that decision away from the bound entirely.
    pub enum Label {
        Bounded(Tag),
        Free(String),
    }

    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    pub struct LabelHolder {
        pub label: Label,
    }

    /// A union whose members hold nothing any bound describes, so there is nothing for a walk to
    /// run and nothing to publish.
    #[model_schema()]
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(untagged)]
    pub enum Unbounded {
        Count(u32),
        Name(String),
    }

    impl UnpublishedValidate for Unbounded {}

    #[test]
    fn test_an_untagged_union_dispatches_to_whichever_variant_it_holds() {
        assert_eq!(
            serde_json::from_str::<ScopedAccount>(r#"{"aud":"app-user","jti":""}"#)
                .unwrap_err()
                .to_string(),
            "'jti': too short: minimum length is 1, got 0"
        );
        assert_eq!(
            serde_json::from_str::<ScopedAccount>(r#"{"sysAdminUsername":"ops","jti":""}"#)
                .unwrap_err()
                .to_string(),
            "'jti': too short: minimum length is 1, got 0",
            "the second member has to be walked too, or the arm is one variant's rather than the \
             union's"
        );
        assert_eq!(
            serde_json::from_str::<ScopedAccount>(r#"{"aud":"app-user","jti":"a"}"#)
                .unwrap()
                .validate(),
            Ok(())
        );
    }

    #[test]
    fn test_a_union_inside_a_union_is_walked_to_the_bound_beneath_both() {
        assert_eq!(
            serde_json::from_str::<ScopedEnvelope>(r#"{"account":{"tokenId":"t-1","jti":""}}"#)
                .unwrap_err()
                .to_string(),
            "'account.jti': too short: minimum length is 1, got 0"
        );
        assert_eq!(
            serde_json::from_str::<ScopedEnvelope>(r#"{"account":{"tokenId":"t-1","jti":"a"}}"#)
                .unwrap()
                .validate(),
            Ok(())
        );
    }

    #[test]
    fn test_a_field_holding_an_untagged_union_names_the_path_the_payload_spells() {
        assert_eq!(
            serde_json::from_str::<ScopedEnvelope>(r#"{"account":{"aud":"app-user","jti":""}}"#)
                .unwrap_err()
                .to_string(),
            "'account.jti': too short: minimum length is 1, got 0"
        );
        assert_eq!(
            serde_json::from_str::<ScopedEnvelope>(r#"{"account":{"aud":"app-user","jti":"a"}}"#)
                .unwrap()
                .validate(),
            Ok(())
        );
    }

    #[test]
    fn test_a_message_that_is_itself_untagged_answers_what_its_variant_answers() {
        let refused = "'account.jti': too short: minimum length is 1, got 0";
        assert_eq!(
            serde_json::from_str::<EitherEnvelope>(r#"{"account":{"aud":"app-user","jti":""}}"#)
                .unwrap_err()
                .to_string(),
            refused
        );
        assert_eq!(
            serde_json::from_str::<EitherEnvelope>(
                r#"{"account":{"sysAdminUsername":"ops","jti":""}}"#
            )
            .unwrap_err()
            .to_string(),
            refused
        );
        assert_eq!(
            serde_json::from_str::<AdminEnvelope>(
                r#"{"account":{"sysAdminUsername":"ops","jti":""}}"#
            )
            .unwrap_err()
            .to_string(),
            refused,
            "the shape that already refused has to keep refusing, in the same words"
        );
    }

    #[test]
    fn test_a_brand_behind_a_newtype_member_is_reached_and_named_by_its_holder() {
        assert_eq!(
            Label::Bounded(Tag("a".to_owned())).validate().unwrap_err(),
            vec!["too short: minimum length is 3, got 1".to_owned()]
        );
        assert_eq!(
            LabelHolder {
                label: Label::Bounded(Tag("a".to_owned())),
            }
            .validate()
            .unwrap_err(),
            vec!["'label': too short: minimum length is 3, got 1".to_owned()]
        );

        let refused: Label = serde_json::from_str(r#""ab""#).unwrap();
        assert!(
            matches!(&refused, Label::Free(loose) if loose == "ab"),
            "the brand's bound took the newtype member out of the running rather than ending the \
             read. Got: {refused:?}"
        );
        assert_eq!(refused.validate(), Ok(()));
        let admitted: Label = serde_json::from_str(r#""abc""#).unwrap();
        assert!(
            matches!(&admitted, Label::Bounded(tag) if tag.0 == "abc"),
            "got: {admitted:?}"
        );
        assert_eq!(admitted.validate(), Ok(()));
    }

    #[test]
    fn test_a_union_with_nothing_to_check_publishes_no_validator() {
        assert_eq!(Unbounded::Count(0).validate(), "no inherent validate()");
        assert_eq!(
            Unbounded::Name(String::new()).validate(),
            "no inherent validate()"
        );
    }

    #[cfg(feature = "zod")]
    #[test]
    fn test_the_zod_surface_carries_the_same_bound_behind_the_same_union() {
        // Each hop names the next rather than inlining it, so the chain is read one link at a time
        // — and it is the same chain of hops the Rust walk takes.
        for (named, schema, names) in [
            (
                "ScopedAccount",
                ScopedAccount::zod_schema(),
                "z.union([AdminAccount$Schema, AppUserAccount$Schema, SharedAccount$Schema])",
            ),
            (
                "EitherEnvelope",
                EitherEnvelope::zod_schema(),
                "z.union([AdminEnvelope$Schema, ScopedEnvelope$Schema])",
            ),
            (
                "AppUserAccount",
                AppUserAccount::zod_schema(),
                "WireClaims$Schema",
            ),
            (
                "AdminAccount",
                AdminAccount::zod_schema(),
                "WireClaims$Schema",
            ),
        ] {
            assert!(
                schema.contains(names),
                "`{named}` reaches the bound by naming `{names}`: {schema}"
            );
        }
        let claims = WireClaims::zod_schema();
        assert!(
            claims.contains(".min(1, { error: (issue) => `too short: minimum length is 1, got ${String(issue.input).length}` })"),
            "the bound the whole chain composes down to: {claims}"
        );
    }
}

/// What a check written on an enum holds: each string the enum carries, where that string's own
/// check is held, and nothing a list, a named type, a number or a type parameter carries.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
mod a_check_the_enum_declares {
    use alloc::borrow::Cow;
    use alloc::sync::Arc;
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;
    use tixschema::model_schema;

    #[model_schema(minLength = 2)]
    #[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(transparent)]
    pub struct Label(pub String);

    #[model_schema(minLength = 1, maxLength = 5)]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub enum Held {
        Bare,
        Branded {
            label: Label,
        },
        Counted {
            count: u32,
        },
        Listed {
            tags: Vec<String>,
        },
        Maybe {
            #[serde(default, skip_serializing_if = "Option::is_none")]
            note: Option<String>,
        },
        Named {
            label: String,
        },
        Own {
            #[model_schema_prop(minLength = 3)]
            label: String,
        },
        Pair(String, u32),
        Slot(String),
    }

    #[model_schema(minLength = 1, maxLength = 5)]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "kind")]
    pub enum HeldInternal {
        Bare,
        Named {
            label: String,
        },
        Own {
            #[model_schema_prop(minLength = 3)]
            label: String,
        },
    }

    #[model_schema(minLength = 1, maxLength = 5)]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "kind", content = "data")]
    pub enum HeldAdjacent {
        Bare,
        Named { label: String },
        Pair(String, u32),
        Slot(String),
    }

    #[model_schema(pattern = "^[a-z]+$")]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(tag = "kind")]
    pub enum Matched {
        Own {
            #[model_schema_prop(pattern = "^ab")]
            word: String,
        },
        Plain {
            word: String,
        },
    }

    #[model_schema(maxLength = 5, default_types(Carried = String))]
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    pub enum Given<Carried> {
        Filled { value: Carried },
        Text { value: String },
    }

    #[model_schema(maxLength = 3)]
    #[derive(Debug, Deserialize, Serialize)]
    pub enum Wrapped {
        Boxed { text: Box<str> },
        Cowed { text: Cow<'static, str> },
        Maybe(Option<Box<str>>),
        Path { at: PathBuf },
        Shared { text: Arc<str> },
    }

    #[test]
    fn test_an_enums_check_is_reported_where_serde_writes_the_member() {
        let short = "too short: minimum length is 1, got 0";
        let long = "too long: maximum length is 5, got 6";
        let (empty, six) = (String::new(), "abcdef".to_owned());

        assert_eq!(
            Held::Named {
                label: empty.clone()
            }
            .validate()
            .unwrap_err(),
            vec![format!("'Named.label': {short}")]
        );
        assert_eq!(
            Held::Named { label: six.clone() }.validate().unwrap_err(),
            vec![format!("'Named.label': {long}")]
        );
        assert_eq!(
            Held::Slot(empty.clone()).validate().unwrap_err(),
            vec![format!("'Slot': {short}")]
        );
        assert_eq!(
            Held::Pair(six.clone(), 0).validate().unwrap_err(),
            vec![format!("'Pair.0': {long}")]
        );
        assert_eq!(
            Held::Maybe {
                note: Some(empty.clone())
            }
            .validate()
            .unwrap_err(),
            vec![format!("'Maybe.note': {short}")]
        );
        assert_eq!(
            HeldInternal::Named { label: six.clone() }
                .validate()
                .unwrap_err(),
            vec![format!("'label': {long}")]
        );
        assert_eq!(
            HeldAdjacent::Named {
                label: empty.clone()
            }
            .validate()
            .unwrap_err(),
            vec![format!("'data.label': {short}")]
        );
        assert_eq!(
            HeldAdjacent::Slot(six).validate().unwrap_err(),
            vec![format!("'data': {long}")]
        );
        assert_eq!(
            HeldAdjacent::Pair(empty, 0).validate().unwrap_err(),
            vec![format!("'data.0': {short}")]
        );

        Held::Maybe { note: None }.validate().unwrap();
        Held::Named {
            label: "abcde".to_owned(),
        }
        .validate()
        .unwrap();
        Held::Bare.validate().unwrap();
        HeldInternal::Bare.validate().unwrap();
        HeldAdjacent::Bare.validate().unwrap();
        assert_eq!(
            held_schema::validate_slot_0_value("").unwrap_err(),
            vec![format!("'Slot': {short}")]
        );
    }

    #[test]
    fn test_an_enums_check_reaches_a_string_or_a_path_under_its_wrappers() {
        let long = "too long: maximum length is 3, got 4";
        for (held, reported) in [
            (
                Wrapped::Boxed {
                    text: "abcd".into(),
                },
                format!("'Boxed.text': {long}"),
            ),
            (
                Wrapped::Cowed {
                    text: Cow::Borrowed("abcd"),
                },
                format!("'Cowed.text': {long}"),
            ),
            (
                Wrapped::Maybe(Some("abcd".into())),
                format!("'Maybe': {long}"),
            ),
            (
                Wrapped::Path {
                    at: PathBuf::from("/tmp"),
                },
                format!("'Path.at': {long}"),
            ),
            (
                Wrapped::Shared {
                    text: Arc::from("abcd"),
                },
                format!("'Shared.text': {long}"),
            ),
        ] {
            assert_eq!(held.validate().unwrap_err(), vec![reported]);
        }
        Wrapped::Maybe(None).validate().unwrap();
        Wrapped::Path {
            at: PathBuf::from("/a"),
        }
        .validate()
        .unwrap();
    }

    #[test]
    fn test_an_enums_check_reaches_no_list_named_type_number_or_type_parameter() {
        Held::Listed {
            tags: vec![String::new(), "abcdef".to_owned()],
        }
        .validate()
        .unwrap();
        Held::Branded {
            label: Label("abcdefgh".to_owned()),
        }
        .validate()
        .unwrap();
        assert_eq!(
            Held::Branded {
                label: Label("a".to_owned())
            }
            .validate()
            .unwrap_err(),
            vec!["'Branded.label': too short: minimum length is 2, got 1"]
        );
        Held::Counted { count: 0 }.validate().unwrap();

        Given::<String>::Filled {
            value: "abcdefgh".to_owned(),
        }
        .validate()
        .unwrap();
        assert_eq!(
            Given::<String>::Text {
                value: "abcdefgh".to_owned()
            }
            .validate()
            .unwrap_err(),
            vec!["'Text.value': too long: maximum length is 5, got 8"]
        );
    }

    #[test]
    fn test_a_field_with_a_check_of_its_own_under_its_enums_is_held_to_both() {
        for (label, refused) in [
            ("ab", Some("too short: minimum length is 3, got 2")),
            ("abcdef", Some("too long: maximum length is 5, got 6")),
            ("abc", None),
            ("abcde", None),
        ] {
            let under = |named: &str| -> Vec<String> {
                refused
                    .into_iter()
                    .map(|sentence| format!("'{named}': {sentence}"))
                    .collect()
            };
            let external = Held::Own {
                label: label.to_owned(),
            };
            let internal = HeldInternal::Own {
                label: label.to_owned(),
            };
            assert_eq!(
                external.validate().err().unwrap_or_default(),
                under("Own.label")
            );
            assert_eq!(
                internal.validate().err().unwrap_or_default(),
                under("label")
            );
        }

        let (own, of_the_enum) = (
            "'word': does not match pattern '^ab'",
            "'word': does not match pattern '^[a-z]+$'",
        );
        for (word, refused) in [
            ("XY", vec![own, of_the_enum]),
            ("xyz", vec![own]),
            ("abC", vec![of_the_enum]),
            ("abc", vec![]),
        ] {
            let matched = Matched::Own {
                word: word.to_owned(),
            };
            assert_eq!(matched.validate().err().unwrap_or_default(), refused);
        }
        assert_eq!(
            Matched::Plain {
                word: "XY".to_owned()
            }
            .validate()
            .unwrap_err(),
            vec![of_the_enum]
        );
    }

    #[test]
    fn test_a_tagged_enum_under_a_check_is_written_as_before_and_read_as_validate_judges_it() {
        for (wire, read) in [
            (
                r#"{"Named":{"label":""}}"#,
                Held::Named {
                    label: String::new(),
                },
            ),
            (r#"{"Slot":"abcdef"}"#, Held::Slot("abcdef".to_owned())),
            (r#"{"Pair":["",0]}"#, Held::Pair(String::new(), 0)),
            (
                r#"{"Own":{"label":"ab"}}"#,
                Held::Own {
                    label: "ab".to_owned(),
                },
            ),
            (
                r#"{"Named":{"label":"abc"}}"#,
                Held::Named {
                    label: "abc".to_owned(),
                },
            ),
            (r#"{"Slot":"abc"}"#, Held::Slot("abc".to_owned())),
            (r#"{"Pair":["abc",0]}"#, Held::Pair("abc".to_owned(), 0)),
            (r#""Bare""#, Held::Bare),
        ] {
            assert_eq!(serde_json::to_string(&read).unwrap(), wire);
            // The value where it keeps the check, and what `validate()` answers of it where it
            // breaks it.
            let judged = read.validate().map_err(|reported| reported.join("; "));
            assert_eq!(
                serde_json::from_str::<Held>(wire).map_err(|refused| refused.to_string()),
                judged.map(|()| read)
            );
        }
        let internal = r#"{"kind":"Named","label":"abcdef"}"#;
        let named = HeldInternal::Named {
            label: "abcdef".to_owned(),
        };
        assert_eq!(
            serde_json::from_str::<HeldInternal>(internal)
                .unwrap_err()
                .to_string(),
            named.validate().unwrap_err().join("; ")
        );
        let adjacent = r#"{"kind":"Slot","data":""}"#;
        let slot = HeldAdjacent::Slot(String::new());
        assert_eq!(serde_json::to_string(&slot).unwrap(), adjacent);
        assert_eq!(
            serde_json::from_str::<HeldAdjacent>(adjacent)
                .unwrap_err()
                .to_string(),
            slot.validate().unwrap_err().join("; ")
        );
        let kept = r#"{"kind":"Slot","data":"abc"}"#;
        assert_eq!(
            serde_json::from_str::<HeldAdjacent>(kept).unwrap(),
            HeldAdjacent::Slot("abc".to_owned())
        );
    }

    #[cfg(feature = "zod")]
    #[test]
    fn test_an_enums_check_reaches_zod_on_each_string_it_carries() {
        let min = "min(1, { error: (issue) => `too short: minimum length is 1, got \
                   ${String(issue.input).length}` })";
        let own_min = "min(3, { error: (issue) => `too short: minimum length is 3, got \
                       ${String(issue.input).length}` })";
        let max = "max(5, { error: (issue) => `too long: maximum length is 5, got \
                   ${String(issue.input).length}` })";
        let held = format!("z.string().{min}.{max}");

        let external = Held::zod_schema();
        for member in [
            format!("label: {held},"),
            format!("label: z.string().{own_min}.{max},"),
            format!("\"Pair\": z.tuple([{held}, z.number().int().min(0).max(4294967295)]),"),
            format!("\"Slot\": {held},"),
            "label: Label$Schema,".to_owned(),
            "tags: z.array(z.string()),".to_owned(),
            "count: z.number().int().min(0).max(4294967295),".to_owned(),
            "z.literal(\"Bare\")".to_owned(),
        ] {
            assert!(external.contains(&member), "{member} in:\n{external}");
        }
        assert!(
            external.contains(&format!(
                "z.union([z.null().transform(() => undefined), {held}, z.undefined()])"
            )),
            "Got:\n{external}"
        );

        let internal = HeldInternal::zod_schema();
        assert!(
            internal.contains(&format!("label: {held},")),
            "Got:\n{internal}"
        );
        let adjacent = HeldAdjacent::zod_schema();
        for member in [
            format!("label: {held},"),
            format!("data: z.tuple([{held}, z.number().int().min(0).max(4294967295)]),"),
            format!("data: {held},"),
        ] {
            assert!(adjacent.contains(&member), "{member} in:\n{adjacent}");
        }

        let matched = Matched::zod_schema();
        let own = "check(z.regex(/^ab/, { error: \"does not match pattern '^ab'\" }))";
        let of_the_enum =
            "check(z.regex(/^[a-z]+$/, { error: \"does not match pattern '^[a-z]+$'\" }))";
        for member in [
            format!("word: z.string().{own}.{of_the_enum},"),
            format!("word: z.string().{of_the_enum},"),
        ] {
            assert!(matched.contains(&member), "{member} in:\n{matched}");
        }

        let given = Given::<String>::zod_schema();
        assert!(given.contains("value: carried,"), "Got:\n{given}");
    }

    #[cfg(feature = "jsonschema")]
    #[test]
    fn test_an_enums_check_reaches_the_json_schema_on_each_string_it_carries() {
        let held = serde_json::json!({ "type": "string", "minLength": 1_u32, "maxLength": 5_u32 });

        let external = Held::json_schema();
        let member =
            |at: usize, variant: &str| external["oneOf"][at]["properties"][variant].clone();
        assert_eq!(
            external["oneOf"][0],
            serde_json::json!({ "type": "string", "const": "Bare" })
        );
        assert_eq!(
            member(3, "Listed")["properties"]["tags"],
            serde_json::json!({ "type": "array", "items": { "type": "string" } })
        );
        assert_eq!(
            member(4, "Maybe")["properties"]["note"],
            serde_json::json!({ "anyOf": [held, { "type": "null" }] })
        );
        assert_eq!(member(5, "Named")["properties"]["label"], held);
        assert_eq!(
            member(6, "Own")["properties"]["label"],
            serde_json::json!({ "type": "string", "minLength": 3_u32, "maxLength": 5_u32 })
        );
        assert_eq!(
            member(7, "Pair")["prefixItems"],
            serde_json::json!([held, { "type": "integer" }])
        );
        assert_eq!(member(8, "Slot"), held);
        assert_eq!(
            member(2, "Counted")["properties"]["count"],
            serde_json::json!({ "type": "integer" })
        );
        let branded = member(1, "Branded")["properties"]["label"].to_string();
        assert!(!branded.contains("maxLength"), "Got: {branded}");

        let internal = HeldInternal::json_schema();
        assert_eq!(internal["oneOf"][1]["properties"]["label"], held);
        let adjacent = HeldAdjacent::json_schema();
        assert_eq!(
            adjacent["oneOf"][1]["properties"]["data"]["properties"]["label"],
            held
        );
        assert_eq!(adjacent["oneOf"][3]["properties"]["data"], held);

        let matched = Matched::json_schema();
        assert_eq!(
            matched["oneOf"][0]["properties"]["word"],
            serde_json::json!({
                "type": "string",
                "allOf": [{ "pattern": "^ab" }, { "pattern": "^[a-z]+$" }]
            })
        );
        assert_eq!(
            matched["oneOf"][1]["properties"]["word"],
            serde_json::json!({ "type": "string", "pattern": "^[a-z]+$" })
        );
    }

    #[test]
    fn test_readme_enum_check_example_prints_what_the_generator_writes() {
        #[model_schema(minLength = 1, maxLength = 5, pattern = "^[a-z]+$")]
        #[derive(Debug, Deserialize, Serialize)]
        #[serde(tag = "kind")]
        pub enum Tag {
            Plain {
                word: String,
            },
            Prefixed {
                #[model_schema_prop(minLength = 3, pattern = "^ab")]
                word: String,
            },
        }

        let readme = include_str!("../../README.md");
        for reported in [
            Tag::Plain {
                word: "abcdef".to_owned(),
            }
            .validate(),
            Tag::Prefixed {
                word: "XY".to_owned(),
            }
            .validate(),
        ] {
            reported.as_ref().unwrap_err();
            let shown = format!("// {reported:?}");
            assert!(
                readme.contains(&shown),
                "the README no longer shows {shown}"
            );
        }
        Tag::Prefixed {
            word: "abc".to_owned(),
        }
        .validate()
        .unwrap();

        #[cfg(feature = "jsonschema")]
        assert_eq!(
            Tag::json_schema()["oneOf"][1]["properties"]["word"],
            serde_json::json!({
                "type": "string",
                "minLength": 3_u32,
                "maxLength": 5_u32,
                "allOf": [{ "pattern": "^ab" }, { "pattern": "^[a-z]+$" }]
            })
        );
    }

    #[cfg(feature = "typescript")]
    #[test]
    fn test_an_enums_check_leaves_the_typescript_type_alone() {
        #[model_schema()]
        #[derive(Debug, Deserialize, Serialize)]
        #[serde(tag = "kind")]
        pub enum Open {
            Bare,
            Named { label: String },
        }

        #[model_schema(minLength = 1, maxLength = 5)]
        #[derive(Debug, Deserialize, Serialize)]
        #[serde(tag = "kind")]
        pub enum Shut {
            Bare,
            Named { label: String },
        }

        let uncommented = |ts: String| -> String {
            ts.lines()
                .map(str::trim_start)
                .filter(|line| !line.starts_with('*') && !line.starts_with("/*"))
                .collect::<Vec<&str>>()
                .join("\n")
                .replace("Shut", "Open")
        };
        assert_eq!(
            uncommented(Shut::ts_definition()),
            uncommented(Open::ts_definition())
        );
        let held = Shut::ts_definition();
        assert!(
            held.contains("   * Minimum length: 1\n   * Maximum length: 5\n"),
            "Got:\n{held}"
        );
    }
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
use alloc::borrow::Cow;
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
use alloc::sync::Arc;
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
use serde::{Deserialize, Serialize};
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
use tixschema::model_schema;

/// What a type answers when it publishes no inherent `validate()` of its own. An inherent method
/// takes precedence over a trait's, so reaching this one is what says none was published — the same
/// question asked of a constraint-free struct, which has never published one either.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
trait UnpublishedValidate {
    fn validate(&self) -> &'static str {
        "no inherent validate()"
    }
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn test_max_length_zod() {
    #[model_schema()]
    #[derive(Serialize, Deserialize)]
    pub struct MaxLengthZod {
        #[model_schema_prop(maxLength = 50)]
        pub username: String,
    }

    let schema = MaxLengthZod::zod_schema();
    assert!(
        schema.contains(".max(50, { error: (issue) => `too long: maximum length is 50, got ${String(issue.input).length}` })"),
        "Expected the maxLength check in Zod schema: {schema}"
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn test_min_length_zod() {
    #[model_schema()]
    #[derive(Serialize, Deserialize)]
    pub struct MinLengthZod {
        #[model_schema_prop(minLength = 5)]
        pub username: String,
    }

    let schema = MinLengthZod::zod_schema();
    assert!(
        schema.contains(".min(5, { error: (issue) => `too short: minimum length is 5, got ${String(issue.input).length}` })"),
        "Expected the minLength check in Zod schema: {schema}"
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn test_min_and_max_length_zod() {
    #[model_schema()]
    #[derive(Serialize, Deserialize)]
    pub struct MinMaxLengthZod {
        #[model_schema_prop(minLength = 5, maxLength = 50)]
        pub username: String,
    }

    let schema = MinMaxLengthZod::zod_schema();
    assert!(
        schema.contains(".min(5, { error: (issue) => `too short: minimum length is 5, got ${String(issue.input).length}` })"),
        "Expected the minLength check in Zod schema: {schema}"
    );
    assert!(
        schema.contains(".max(50, { error: (issue) => `too long: maximum length is 50, got ${String(issue.input).length}` })"),
        "Expected the maxLength check in Zod schema: {schema}"
    );
}

#[cfg(all(feature = "serde", feature = "jsonschema"))]
#[test]
fn test_max_length_json_schema() {
    #[model_schema()]
    #[derive(Serialize, Deserialize)]
    pub struct MaxLengthJsonSchema {
        #[model_schema_prop(maxLength = 50)]
        pub username: String,
    }

    let schema = MaxLengthJsonSchema::json_schema();
    let schema_str = serde_json::to_string(&schema).unwrap();
    assert!(
        schema_str.contains("\"maxLength\":50"),
        "Expected maxLength:50 in JSON schema: {schema_str}"
    );
}

#[cfg(all(feature = "serde", feature = "jsonschema"))]
#[test]
fn test_min_and_max_length_json_schema() {
    #[model_schema()]
    #[derive(Serialize, Deserialize)]
    pub struct MinMaxLengthJsonSchema {
        #[model_schema_prop(minLength = 5, maxLength = 50)]
        pub username: String,
    }

    let schema = MinMaxLengthJsonSchema::json_schema();
    let schema_str = serde_json::to_string(&schema).unwrap();
    assert!(
        schema_str.contains("\"minLength\":5"),
        "Expected minLength:5 in JSON schema: {schema_str}"
    );
    assert!(
        schema_str.contains("\"maxLength\":50"),
        "Expected maxLength:50 in JSON schema: {schema_str}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_max_length_rust_valid() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct MaxLengthValid {
        #[model_schema_prop(maxLength = 10)]
        pub name: String,
    }

    let valid = r#"{"name": "hello"}"#;
    let result: Result<MaxLengthValid, _> = serde_json::from_str(valid);
    assert!(
        result.is_ok(),
        "String within maxLength should succeed: {:?}",
        result.err()
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_max_length_rust_invalid() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct MaxLengthInvalid {
        #[model_schema_prop(maxLength = 5)]
        pub name: String,
    }

    let invalid = r#"{"name": "too long value"}"#;
    // The read refuses a value out of range, with what the validator answers of it.
    let refused = serde_json::from_str::<MaxLengthInvalid>(invalid)
        .unwrap_err()
        .to_string();
    assert!(
        refused.contains("too long"),
        "Error should mention 'too long': {refused}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_min_length_rust_valid() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct MinLengthRustValid {
        #[model_schema_prop(minLength = 3)]
        pub name: String,
    }

    let valid = r#"{"name": "hello"}"#;
    let result: Result<MinLengthRustValid, _> = serde_json::from_str(valid);
    assert!(
        result.is_ok(),
        "String meeting minLength should succeed: {:?}",
        result.err()
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_min_length_rust_invalid() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct MinLengthRustInvalid {
        #[model_schema_prop(minLength = 10)]
        pub name: String,
    }

    let invalid = r#"{"name": "hi"}"#;
    // The read refuses a value out of range, with what the validator answers of it.
    let refused = serde_json::from_str::<MinLengthRustInvalid>(invalid)
        .unwrap_err()
        .to_string();
    assert!(
        refused.contains("too short"),
        "Error should mention 'too short': {refused}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_combined_string_constraints_valid() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct CombinedStringValid {
        #[model_schema_prop(minLength = 3, maxLength = 20, pattern = "^[a-z]+$")]
        pub id: String,
    }

    let valid = r#"{"id": "hello"}"#;
    let result: Result<CombinedStringValid, _> = serde_json::from_str(valid);
    assert!(
        result.is_ok(),
        "Value meeting all constraints should succeed: {:?}",
        result.err()
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_combined_string_constraints_too_short() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct CombinedStringShort {
        #[model_schema_prop(minLength = 5, maxLength = 20, pattern = "^[a-z]+$")]
        pub id: String,
    }

    let invalid = r#"{"id": "ab"}"#;
    // The read refuses it, and says which of the three constraints it breaks.
    let refused = serde_json::from_str::<CombinedStringShort>(invalid)
        .unwrap_err()
        .to_string();
    assert!(
        refused.contains("too short"),
        "Too short value should be refused for that: {refused}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_combined_string_constraints_pattern_fail() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct CombinedStringPattern {
        #[model_schema_prop(minLength = 3, maxLength = 20, pattern = "^[a-z]+$")]
        pub id: String,
    }

    let invalid = r#"{"id": "Hello123"}"#;
    // The read refuses it, and says which of the three constraints it breaks.
    let refused = serde_json::from_str::<CombinedStringPattern>(invalid)
        .unwrap_err()
        .to_string();
    assert!(
        refused.contains("does not match pattern"),
        "Value failing pattern should be refused for that: {refused}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_validate_method_ok() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct ValidateOk {
        #[model_schema_prop(minLength = 3, maxLength = 20)]
        pub name: String,
    }

    let instance = ValidateOk {
        name: "hello".to_owned(),
    };
    let result = instance.validate();
    assert!(
        result.is_ok(),
        "validate() should return Ok for valid data: {:?}",
        result.err()
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_validate_method_err() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct ValidateErr {
        #[model_schema_prop(minLength = 10)]
        pub name: String,
    }

    let instance = ValidateErr {
        name: "hi".to_owned(),
    };
    let result = instance.validate();
    assert!(
        result.is_err(),
        "validate() should return Err for invalid data"
    );
    let errors = result.unwrap_err();
    assert!(!errors.is_empty(), "Errors vector should not be empty");
    assert!(
        errors[0].contains("too short"),
        "Error message should mention 'too short': {errors:?}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_validate_method_multiple_errors() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct ValidateMultiErr {
        #[model_schema_prop(maxLength = 3)]
        pub code: String,
        #[model_schema_prop(minLength = 10)]
        pub name: String,
    }

    let instance = ValidateMultiErr {
        name: "hi".to_owned(),
        code: "toolongcode".to_owned(),
    };
    let result = instance.validate();
    assert!(
        result.is_err(),
        "validate() should return Err for invalid data"
    );
    let errors = result.unwrap_err();
    assert_eq!(errors.len(), 2, "Should have 2 errors, got: {errors:?}");
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn test_minimum_zod() {
    #[model_schema()]
    #[derive(Serialize, Deserialize)]
    pub struct MinimumZod {
        #[model_schema_prop(minimum = 1)]
        pub count: i32,
    }

    let schema = MinimumZod::zod_schema();
    assert!(
        schema.contains(".min("),
        "Expected .min() in Zod schema: {schema}"
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn test_maximum_zod() {
    #[model_schema()]
    #[derive(Serialize, Deserialize)]
    pub struct MaximumZod {
        #[model_schema_prop(maximum = 100)]
        pub count: i32,
    }

    let schema = MaximumZod::zod_schema();
    assert!(
        schema.contains(".max("),
        "Expected .max() in Zod schema: {schema}"
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn test_float_minimum_maximum_zod() {
    #[model_schema()]
    #[derive(Serialize, Deserialize)]
    pub struct FloatMinMaxZod {
        #[model_schema_prop(minimum = 0, maximum = 1)]
        pub ratio: f64,
    }

    let schema = FloatMinMaxZod::zod_schema();
    assert!(
        schema.contains(".min("),
        "Expected .min() in Zod schema for f64: {schema}"
    );
    assert!(
        schema.contains(".max("),
        "Expected .max() in Zod schema for f64: {schema}"
    );
}

#[cfg(all(feature = "serde", feature = "jsonschema"))]
#[test]
fn test_minimum_json_schema() {
    #[model_schema()]
    #[derive(Serialize, Deserialize)]
    pub struct MinimumJsonSchema {
        #[model_schema_prop(minimum = 1)]
        pub count: i32,
    }

    let schema = MinimumJsonSchema::json_schema();
    let schema_str = serde_json::to_string(&schema).unwrap();
    assert!(
        schema_str.contains("\"minimum\""),
        "Expected 'minimum' in JSON schema: {schema_str}"
    );
}

#[cfg(all(feature = "serde", feature = "jsonschema"))]
#[test]
fn test_maximum_json_schema() {
    #[model_schema()]
    #[derive(Serialize, Deserialize)]
    pub struct MaximumJsonSchema {
        #[model_schema_prop(maximum = 100)]
        pub count: i32,
    }

    let schema = MaximumJsonSchema::json_schema();
    let schema_str = serde_json::to_string(&schema).unwrap();
    assert!(
        schema_str.contains("\"maximum\""),
        "Expected 'maximum' in JSON schema: {schema_str}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_minimum_rust_valid() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct MinimumRustValid {
        #[model_schema_prop(minimum = 1)]
        pub count: i32,
    }

    let valid = r#"{"count": 5}"#;
    let result: Result<MinimumRustValid, _> = serde_json::from_str(valid);
    assert!(
        result.is_ok(),
        "Value above minimum should succeed: {:?}",
        result.err()
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_minimum_rust_invalid() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct MinimumRustInvalid {
        #[model_schema_prop(minimum = 10)]
        pub count: i32,
    }

    let invalid = r#"{"count": 3}"#;
    // The read refuses a value out of range, with what the validator answers of it.
    let refused = serde_json::from_str::<MinimumRustInvalid>(invalid)
        .unwrap_err()
        .to_string();
    assert!(
        refused.contains("too small"),
        "Error should mention 'too small': {refused}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_maximum_rust_valid() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct MaximumRustValid {
        #[model_schema_prop(maximum = 100)]
        pub count: i32,
    }

    let valid = r#"{"count": 50}"#;
    let result: Result<MaximumRustValid, _> = serde_json::from_str(valid);
    assert!(
        result.is_ok(),
        "Value below maximum should succeed: {:?}",
        result.err()
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_maximum_rust_invalid() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct MaximumRustInvalid {
        #[model_schema_prop(maximum = 10)]
        pub count: i32,
    }

    let invalid = r#"{"count": 99}"#;
    // The read refuses a value out of range, with what the validator answers of it.
    let refused = serde_json::from_str::<MaximumRustInvalid>(invalid)
        .unwrap_err()
        .to_string();
    assert!(
        refused.contains("too large"),
        "Error should mention 'too large': {refused}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_minimum_at_boundary() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct MinBoundary {
        #[model_schema_prop(minimum = 5)]
        pub count: i32,
    }

    let valid = r#"{"count": 5}"#;
    let result: Result<MinBoundary, _> = serde_json::from_str(valid);
    assert!(
        result.is_ok(),
        "Value exactly at minimum should succeed: {:?}",
        result.err()
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_maximum_at_boundary() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct MaxBoundary {
        #[model_schema_prop(maximum = 5)]
        pub count: i32,
    }

    let valid = r#"{"count": 5}"#;
    let result: Result<MaxBoundary, _> = serde_json::from_str(valid);
    assert!(
        result.is_ok(),
        "Value exactly at maximum should succeed: {:?}",
        result.err()
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_validate_method_numeric_ok() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct ValidateNumericOk {
        #[model_schema_prop(minimum = 1, maximum = 100)]
        pub count: i32,
    }

    let instance = ValidateNumericOk { count: 50 };
    let result = instance.validate();
    assert!(
        result.is_ok(),
        "validate() should return Ok for valid numeric data: {:?}",
        result.err()
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_validate_method_numeric_err() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct ValidateNumericErr {
        #[model_schema_prop(minimum = 10, maximum = 100)]
        pub count: i32,
    }

    let instance = ValidateNumericErr { count: 3 };
    let result = instance.validate();
    assert!(
        result.is_err(),
        "validate() should return Err for numeric out of range"
    );
    let errors = result.unwrap_err();
    assert!(!errors.is_empty(), "Errors vector should not be empty");
    assert!(
        errors[0].contains("too small"),
        "Error message should mention 'too small': {errors:?}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_validate_method_pattern_ok() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct ValidatePatternOk {
        #[model_schema_prop(pattern = "^[a-z]+$")]
        pub slug: String,
    }

    let instance = ValidatePatternOk {
        slug: "hello".to_owned(),
    };
    let result = instance.validate();
    assert!(
        result.is_ok(),
        "validate() should return Ok for value matching pattern: {:?}",
        result.err()
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_validate_method_pattern_err() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct ValidatePatternErr {
        #[model_schema_prop(pattern = "^[a-z]+$")]
        pub slug: String,
    }

    let instance = ValidatePatternErr {
        slug: "ABC123".to_owned(),
    };
    let result = instance.validate();
    assert!(
        result.is_err(),
        "validate() should return Err for value not matching pattern"
    );
    let errors = result.unwrap_err();
    assert!(!errors.is_empty(), "Errors vector should not be empty");
    assert!(
        errors[0].contains("does not match pattern"),
        "Error message should mention 'does not match pattern': {errors:?}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_validate_method_pattern_and_length() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct ValidatePatternLength {
        #[model_schema_prop(pattern = "^[a-z]+$", minLength = 3, maxLength = 10)]
        pub tag: String,
    }

    let valid = ValidatePatternLength {
        tag: "hello".to_owned(),
    };
    assert!(
        valid.validate().is_ok(),
        "Valid value should pass all constraints"
    );

    let too_short = ValidatePatternLength {
        tag: "ab".to_owned(),
    };
    let too_short_result = too_short.validate();
    assert!(too_short_result.is_err(), "Too short value should fail");
    let too_short_errors = too_short_result.unwrap_err();
    assert!(
        too_short_errors.iter().any(|e| e.contains("too short")),
        "Should report 'too short' error: {too_short_errors:?}"
    );

    let bad_pattern = ValidatePatternLength {
        tag: "Hello".to_owned(),
    };
    let bad_pattern_result = bad_pattern.validate();
    assert!(
        bad_pattern_result.is_err(),
        "Value not matching pattern should fail"
    );
    let bad_pattern_errors = bad_pattern_result.unwrap_err();
    assert!(
        bad_pattern_errors
            .iter()
            .any(|e| e.contains("does not match pattern")),
        "Should report 'does not match pattern' error: {bad_pattern_errors:?}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_validate_method_max_length_ok() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct ValidateMaxLenOk {
        #[model_schema_prop(maxLength = 20)]
        pub label: String,
    }

    let instance = ValidateMaxLenOk {
        label: "short".to_owned(),
    };
    let result = instance.validate();
    assert!(
        result.is_ok(),
        "validate() should return Ok for value within maxLength: {:?}",
        result.err()
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_validate_method_max_length_err() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct ValidateMaxLenErr {
        #[model_schema_prop(maxLength = 5)]
        pub label: String,
    }

    let instance = ValidateMaxLenErr {
        label: "way too long value".to_owned(),
    };
    let result = instance.validate();
    assert!(
        result.is_err(),
        "validate() should return Err for value exceeding maxLength"
    );
    let errors = result.unwrap_err();
    assert!(!errors.is_empty(), "Errors vector should not be empty");
    assert!(
        errors[0].contains("too long"),
        "Error message should mention 'too long': {errors:?}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_validate_method_mixed_string_numeric() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct ValidateMixed {
        #[model_schema_prop(minimum = 10)]
        pub score: i32,
        #[model_schema_prop(minLength = 5)]
        pub title: String,
    }

    let instance = ValidateMixed {
        title: "hi".to_owned(),
        score: 2,
    };
    let result = instance.validate();
    assert!(
        result.is_err(),
        "validate() should return Err when both fields are invalid"
    );
    let errors = result.unwrap_err();
    assert_eq!(
        errors.len(),
        2,
        "Should have exactly 2 errors, got: {errors:?}"
    );
    assert!(
        errors.iter().any(|e| e.contains("too short")),
        "Should contain string 'too short' error: {errors:?}"
    );
    assert!(
        errors.iter().any(|e| e.contains("too small")),
        "Should contain numeric 'too small' error: {errors:?}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_minimum_float_rust_valid() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct MinFloatValid {
        #[model_schema_prop(minimum = 0)]
        pub ratio: f64,
    }

    let valid = r#"{"ratio": 0.5}"#;
    let result: Result<MinFloatValid, _> = serde_json::from_str(valid);
    assert!(
        result.is_ok(),
        "Float above minimum should succeed: {:?}",
        result.err()
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_minimum_float_rust_invalid() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct MinFloatInvalid {
        #[model_schema_prop(minimum = 1)]
        pub ratio: f64,
    }

    let invalid = r#"{"ratio": -0.5}"#;
    // The read refuses a value out of range, with what the validator answers of it.
    let refused = serde_json::from_str::<MinFloatInvalid>(invalid)
        .unwrap_err()
        .to_string();
    assert!(
        refused.contains("too small"),
        "Error should mention 'too small': {refused}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_maximum_float_rust_valid() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct MaxFloatValid {
        #[model_schema_prop(maximum = 100)]
        pub value: f64,
    }

    let valid = r#"{"value": 50.0}"#;
    let result: Result<MaxFloatValid, _> = serde_json::from_str(valid);
    assert!(
        result.is_ok(),
        "Float below maximum should succeed: {:?}",
        result.err()
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_maximum_float_rust_invalid() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct MaxFloatInvalid {
        #[model_schema_prop(maximum = 10)]
        pub value: f64,
    }

    let invalid = r#"{"value": 15.0}"#;
    // The read refuses a value out of range, with what the validator answers of it.
    let refused = serde_json::from_str::<MaxFloatInvalid>(invalid)
        .unwrap_err()
        .to_string();
    assert!(
        refused.contains("too large"),
        "Error should mention 'too large': {refused}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_validate_method_float_err() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct ValidateFloatErr {
        #[model_schema_prop(minimum = 0, maximum = 100)]
        pub percentage: f64,
    }

    let instance = ValidateFloatErr { percentage: 150.0 };
    let result = instance.validate();
    assert!(
        result.is_err(),
        "validate() should return Err for float out of range"
    );
    let errors = result.unwrap_err();
    assert!(!errors.is_empty(), "Errors vector should not be empty");
    assert!(
        errors[0].contains("too large"),
        "Error message should mention 'too large': {errors:?}"
    );
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn test_min_max_numeric_zod() {
    #[model_schema()]
    #[derive(Serialize, Deserialize)]
    pub struct MinMaxNumericZod {
        #[model_schema_prop(minimum = 1, maximum = 100)]
        pub level: i32,
    }

    let schema = MinMaxNumericZod::zod_schema();
    assert!(
        schema.contains(".min("),
        "Expected .min() in Zod schema: {schema}"
    );
    assert!(
        schema.contains(".max("),
        "Expected .max() in Zod schema: {schema}"
    );
}

#[cfg(all(feature = "serde", feature = "jsonschema"))]
#[test]
fn test_min_max_numeric_json_schema() {
    #[model_schema()]
    #[derive(Serialize, Deserialize)]
    pub struct MinMaxNumericJsonSchema {
        #[model_schema_prop(minimum = 1, maximum = 100)]
        pub level: i32,
    }

    let schema = MinMaxNumericJsonSchema::json_schema();
    let schema_str = serde_json::to_string(&schema).unwrap();
    assert!(
        schema_str.contains("\"minimum\""),
        "Expected 'minimum' in JSON schema: {schema_str}"
    );
    assert!(
        schema_str.contains("\"maximum\""),
        "Expected 'maximum' in JSON schema: {schema_str}"
    );
}

#[cfg(all(feature = "serde", feature = "typescript"))]
#[test]
fn test_constraints_dont_affect_typescript() {
    #[model_schema()]
    #[derive(Serialize, Deserialize)]
    pub struct ConstraintsTs {
        #[model_schema_prop(minimum = 0, maximum = 120)]
        pub age: u32,
        #[model_schema_prop(minLength = 3, maxLength = 50, pattern = "^[a-z]+$")]
        pub username: String,
    }

    let ts = ConstraintsTs::ts_definition();
    // The TypeScript type itself uses plain `string` and `number`: no Zod or JSON Schema constraint
    // methods.
    assert!(
        ts.contains("export type"),
        "Should contain 'export type': {ts}"
    );
    let type_body_start = ts.find("export type").unwrap();
    let type_section = &ts[type_body_start..];
    assert!(
        !type_section.contains(".min("),
        "TypeScript type should not contain .min(): {type_section}"
    );
    assert!(
        !type_section.contains(".max("),
        "TypeScript type should not contain .max(): {type_section}"
    );
    assert!(
        !type_section.contains(".check("),
        "TypeScript type should not contain .check(): {type_section}"
    );
    assert!(
        !type_section.contains("z.regex"),
        "TypeScript type should not contain z.regex: {type_section}"
    );
    assert!(
        !type_section.contains("z.string"),
        "TypeScript type should not contain z.string: {type_section}"
    );
    assert!(
        !type_section.contains("z.number"),
        "TypeScript type should not contain z.number: {type_section}"
    );
    assert!(
        type_section.contains("username: string"),
        "TypeScript should have 'username: string': {type_section}"
    );
    assert!(
        type_section.contains("age: number"),
        "TypeScript should have 'age: number': {type_section}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_boundary_min_length_zero() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct MinLenZero {
        #[model_schema_prop(minLength = 0)]
        pub tag: String,
    }

    let instance = MinLenZero { tag: String::new() };
    let validate_result = instance.validate();
    assert!(
        validate_result.is_ok(),
        "Empty string should pass minLength = 0: {:?}",
        validate_result.err()
    );

    let valid = r#"{"tag": ""}"#;
    let serde_result: Result<MinLenZero, _> = serde_json::from_str(valid);
    assert!(
        serde_result.is_ok(),
        "Empty string via serde should pass minLength = 0: {:?}",
        serde_result.err()
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_pattern_empty_string_match() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct PatternEmpty {
        #[model_schema_prop(pattern = "^\\s*$")]
        pub empty_field: String,
    }

    let valid_instance = PatternEmpty {
        empty_field: String::new(),
    };
    let valid_result = valid_instance.validate();
    assert!(
        valid_result.is_ok(),
        "Empty string should match pattern '^\\s*$': {:?}",
        valid_result.err()
    );

    let invalid_instance = PatternEmpty {
        empty_field: "not empty".to_owned(),
    };
    let invalid_result = invalid_instance.validate();
    assert!(
        invalid_result.is_err(),
        "Non-empty string should not match pattern '^\\s*$'"
    );
    let errors = invalid_result.unwrap_err();
    assert!(
        errors[0].contains("does not match pattern"),
        "Error should mention 'does not match pattern': {errors:?}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_pattern_anchored_single_character_prefix() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct PatternRootedPath {
        #[model_schema_prop(pattern = "^/")]
        pub mount: String,
    }

    let accepted = PatternRootedPath {
        mount: "/var/log".to_owned(),
    };
    assert!(
        accepted.validate().is_ok(),
        "A value starting with '/' should match pattern '^/'"
    );

    let rejected = PatternRootedPath {
        mount: "var/log".to_owned(),
    };
    let result = rejected.validate();
    assert!(
        result.is_err(),
        "A value not starting with '/' should not match pattern '^/'"
    );
    let errors = result.unwrap_err();
    assert_eq!(
        errors[0], "'mount': does not match pattern '^/'",
        "Rejection should read exactly as the regex path words it: {errors:?}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_pattern_pinning_both_ends_to_one_position() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct PatternBlankSlot {
        #[model_schema_prop(pattern = "^$")]
        pub slot: String,
    }

    PatternBlankSlot {
        slot: String::new(),
    }
    .validate()
    .unwrap();

    let rejected = PatternBlankSlot {
        slot: "x".to_owned(),
    };
    let errors = rejected.validate().unwrap_err();
    assert_eq!(
        errors[0], "'slot': does not match pattern '^$'",
        "Rejection should read exactly as the regex path words it: {errors:?}"
    );
}

/// The rewrite a lone `\b` is refused in favour of.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_pattern_boundary_with_a_word_beside_it() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct PatternBoundedWord {
        #[model_schema_prop(pattern = r"\b\w+")]
        pub caption: String,
    }

    PatternBoundedWord {
        caption: "hello there".to_owned(),
    }
    .validate()
    .unwrap();

    let rejected = PatternBoundedWord {
        caption: "...".to_owned(),
    };
    let errors = rejected.validate().unwrap_err();
    assert_eq!(
        errors[0], r"'caption': does not match pattern '\b[0-9A-Za-z_]+'",
        "Rejection should quote the pattern in the spelling every surface receives: {errors:?}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_validate_optional_string_some_too_short() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct OptionalConstrained {
        #[model_schema_prop(minLength = 3)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub nickname: Option<String>,
    }

    let instance = OptionalConstrained {
        nickname: Some("a".to_owned()),
    };
    let result = instance.validate();
    assert!(
        result.is_err(),
        "validate() should reject a Some holding a too-short string"
    );
    let errors = result.unwrap_err();
    assert_eq!(
        errors,
        vec!["'nickname': too short: minimum length is 3, got 1"],
        "Unexpected errors: {errors:?}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_validate_optional_string_none_is_ok() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct OptionalAbsent {
        #[model_schema_prop(minLength = 3)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub nickname: Option<String>,
    }

    let instance = OptionalAbsent { nickname: None };
    let result = instance.validate();
    assert!(
        result.is_ok(),
        "A None writes no string, so nothing constrains it: {:?}",
        result.err()
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_validate_boxed_string_too_short() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct BoxedConstrained {
        #[model_schema_prop(minLength = 3)]
        pub label: Box<str>,
    }

    let instance = BoxedConstrained { label: "a".into() };
    let result = instance.validate();
    assert!(
        result.is_err(),
        "validate() should reject a too-short string under a Box"
    );
    let errors = result.unwrap_err();
    assert_eq!(
        errors,
        vec!["'label': too short: minimum length is 3, got 1"],
        "Unexpected errors: {errors:?}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_validate_vec_string_counts_its_items() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct VecConstrained {
        #[model_schema_prop(minLength = 3)]
        pub tags: Vec<String>,
    }

    let held = VecConstrained {
        tags: vec!["ok!".to_owned(), "a".to_owned(), "b".to_owned()],
    };
    assert_eq!(
        held.validate(),
        Ok(()),
        "three items meet the bound, and no item is measured by it"
    );

    let short = VecConstrained {
        tags: vec!["long enough".to_owned(), "and this".to_owned()],
    };
    assert_eq!(
        short.validate().unwrap_err(),
        vec!["'tags': too short: minimum length is 3, got 2"],
        "the list is reported once, by its count"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_validate_optional_vec_string() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct OptionalVecConstrained {
        #[model_schema_prop(minLength = 3)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub tags: Option<Vec<String>>,
    }

    let present = OptionalVecConstrained {
        tags: Some(vec!["long enough".to_owned()]),
    };
    let errors = present.validate().unwrap_err();
    assert_eq!(
        errors,
        vec!["'tags': too short: minimum length is 3, got 1"],
        "Unexpected errors: {errors:?}"
    );
    assert_eq!(
        OptionalVecConstrained { tags: Some(vec![]) }
            .validate()
            .unwrap_err(),
        vec!["'tags': too short: minimum length is 3, got 0"],
        "an empty list is a list, and is counted"
    );

    let absent = OptionalVecConstrained { tags: None };
    assert!(absent.validate().is_ok(), "A None holds no list to count");
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_validate_nested_vec_counts_the_outer_list() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct NestedVecConstrained {
        #[model_schema_prop(minLength = 3)]
        pub rows: Vec<Vec<String>>,
    }

    let instance = NestedVecConstrained {
        rows: vec![vec!["ok!".to_owned()], vec!["a".to_owned()]],
    };
    let errors = instance.validate().unwrap_err();
    assert_eq!(
        errors,
        vec!["'rows': too short: minimum length is 3, got 2"],
        "The constraint counts the outer list: {errors:?}"
    );
    assert_eq!(
        NestedVecConstrained {
            rows: vec![vec![], vec!["a".to_owned()], vec![]],
        }
        .validate(),
        Ok(()),
        "nothing is said about an inner list, or about what it holds"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_validate_optional_numeric_minimum() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct OptionalNumeric {
        #[model_schema_prop(minimum = 18)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub age: Option<u32>,
    }

    let too_small = OptionalNumeric { age: Some(5) };
    let errors = too_small.validate().unwrap_err();
    assert_eq!(
        errors,
        vec!["'age': too small: minimum is 18, got 5"],
        "Unexpected errors: {errors:?}"
    );

    assert!(
        OptionalNumeric { age: None }.validate().is_ok(),
        "A None writes no number to constrain"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_validate_arc_slice_mixed_wrappers() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct MixedWrappers {
        #[model_schema_prop(maxLength = 2)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub codes: Option<Arc<[String]>>,
    }

    let held = MixedWrappers {
        codes: Some(Arc::from(["ok".to_owned(), "toolong".to_owned()])),
    };
    assert_eq!(held.validate(), Ok(()), "two items, of any length");

    let instance = MixedWrappers {
        codes: Some(Arc::from(["a".to_owned(), "b".to_owned(), "c".to_owned()])),
    };
    let errors = instance.validate().unwrap_err();
    assert_eq!(
        errors,
        vec!["'codes': too long: maximum length is 2, got 3"],
        "Unexpected errors: {errors:?}"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_validate_boxed_option_string() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct BoxedOption {
        #[model_schema_prop(minLength = 3)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub nickname: Box<Option<String>>,
    }

    let instance = BoxedOption {
        nickname: Box::new(Some("a".to_owned())),
    };
    let errors = instance.validate().unwrap_err();
    assert_eq!(
        errors,
        vec!["'nickname': too short: minimum length is 3, got 1"],
        "Unexpected errors: {errors:?}"
    );

    assert!(
        BoxedOption {
            nickname: Box::new(None)
        }
        .validate()
        .is_ok(),
        "A None under a Box still writes nothing to constrain"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_an_optional_string_is_held_to_its_bound_as_it_is_read() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct WireOptional {
        #[model_schema_prop(minLength = 3)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub nickname: Option<String>,
    }

    assert_eq!(
        serde_json::from_str::<WireOptional>(r#"{"nickname":"a"}"#)
            .unwrap_err()
            .to_string(),
        "'nickname': too short: minimum length is 3, got 1",
        "the bound reaches through the Option, and names the field"
    );

    let accepted = serde_json::from_str::<WireOptional>(r#"{"nickname":"abc"}"#).unwrap();
    assert_eq!(accepted.nickname.as_deref(), Some("abc"));
    accepted.validate().unwrap();
}

/// A `None` puts no string on the wire, so neither spelling of its absence has anything to reject.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_deserialize_optional_string_still_admits_an_absent_key() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct WireAbsent {
        #[serde(skip_serializing_if = "Option::is_none")]
        #[model_schema_prop(minLength = 3)]
        pub nickname: Option<String>,
    }

    assert!(
        serde_json::from_str::<WireAbsent>("{}")
            .unwrap()
            .nickname
            .is_none(),
        "A missing key is what the generated schemas describe as the absent form"
    );
    assert!(
        serde_json::from_str::<WireAbsent>(r#"{"nickname":null}"#)
            .unwrap()
            .nickname
            .is_none(),
        "A null reads as the same None it always did"
    );
}

/// A field that writes its own default keeps it, nothing being injected beside it.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_deserialize_optional_string_keeps_a_written_default() {
    fn preset() -> Option<String> {
        Some("preset".to_owned()).filter(|preset| preset.len() >= 3)
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct WirePreset {
        #[serde(default = "preset", skip_serializing_if = "Option::is_none")]
        #[model_schema_prop(minLength = 3)]
        pub nickname: Option<String>,
    }

    assert_eq!(
        serde_json::from_str::<WirePreset>("{}")
            .unwrap()
            .nickname
            .as_deref(),
        Some("preset"),
        "The field's own default is what answers for its missing key"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_a_boxed_string_is_held_to_its_bound_as_it_is_read() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct WireBoxed {
        #[model_schema_prop(minLength = 3)]
        pub label: Box<str>,
    }

    assert_eq!(
        serde_json::from_str::<WireBoxed>(r#"{"label":"a"}"#)
            .unwrap_err()
            .to_string(),
        "'label': too short: minimum length is 3, got 1"
    );
    assert_eq!(
        &*serde_json::from_str::<WireBoxed>(r#"{"label":"abc"}"#)
            .unwrap()
            .label,
        "abc"
    );
}

/// A `Cow` is reached as the `Box` is, its lifetime being no part of what it holds.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_a_cow_string_is_held_to_its_bound_as_it_is_read() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct WireCow {
        #[model_schema_prop(minLength = 3)]
        pub label: Cow<'static, str>,
    }

    assert_eq!(
        serde_json::from_str::<WireCow>(r#"{"label":"a"}"#)
            .unwrap_err()
            .to_string(),
        "'label': too short: minimum length is 3, got 1"
    );
    assert_eq!(
        serde_json::from_str::<WireCow>(r#"{"label":"abc"}"#)
            .unwrap()
            .label,
        "abc"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_a_vec_of_strings_is_held_to_its_bound_as_it_is_read() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct WireVec {
        #[model_schema_prop(minLength = 3)]
        pub tags: Vec<String>,
    }

    assert_eq!(
        serde_json::from_str::<WireVec>(r#"{"tags":["ok!","a"]}"#)
            .unwrap_err()
            .to_string(),
        "'tags': too short: minimum length is 3, got 2",
        "the list is read, and then reported by its count"
    );
    let held = serde_json::from_str::<WireVec>(r#"{"tags":["ok!","a","b"]}"#).unwrap();
    assert_eq!(
        held.tags,
        vec!["ok!".to_owned(), "a".to_owned(), "b".to_owned()]
    );
    held.validate().unwrap();
    assert_eq!(
        serde_json::from_str::<WireVec>(r#"{"tags":[]}"#)
            .unwrap_err()
            .to_string(),
        "'tags': too short: minimum length is 3, got 0",
        "An empty array is read, and is a list of no items"
    );
}

/// The wrappers compose in the order they were written, and the walk goes through them in it.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_an_optional_vec_is_held_to_its_bound_as_it_is_read() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct WireOptionalVec {
        #[model_schema_prop(minLength = 3)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub tags: Option<Vec<String>>,
    }

    assert_eq!(
        serde_json::from_str::<WireOptionalVec>(r#"{"tags":["ok!","a"]}"#)
            .unwrap_err()
            .to_string(),
        "'tags': too short: minimum length is 3, got 2"
    );
    assert!(
        serde_json::from_str::<WireOptionalVec>("{}")
            .unwrap()
            .tags
            .is_none(),
        "A missing key is still the absent form the schemas describe"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_an_optional_number_is_held_to_its_range_as_it_is_read() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct WireNumeric {
        #[model_schema_prop(minimum = 18, maximum = 120)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub age: Option<u32>,
        #[model_schema_prop(maxLength = 2)]
        pub counts: Vec<u32>,
    }

    assert_eq!(
        serde_json::from_str::<WireNumeric>(r#"{"age":5,"counts":[1]}"#)
            .unwrap_err()
            .to_string(),
        "'age': too small: minimum is 18, got 5"
    );

    assert_eq!(
        serde_json::from_str::<WireNumeric>(r#"{"age":21,"counts":[1,0,7]}"#)
            .unwrap_err()
            .to_string(),
        "'counts': too long: maximum length is 2, got 3"
    );

    let accepted = serde_json::from_str::<WireNumeric>(r#"{"age":21,"counts":[1,0]}"#).unwrap();
    assert_eq!(accepted.age, Some(21));
    accepted.validate().unwrap();
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_every_wrapped_shape_is_refused_as_it_is_read() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct WireAgreement {
        #[model_schema_prop(minLength = 3)]
        pub boxed: Box<str>,
        #[model_schema_prop(minLength = 3)]
        pub cow: Cow<'static, str>,
        #[model_schema_prop(minLength = 3)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub nested: Option<Vec<String>>,
        #[model_schema_prop(minLength = 3)]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pub opt: Option<String>,
        #[model_schema_prop(minLength = 3)]
        pub plain: String,
        #[model_schema_prop(minLength = 3)]
        pub tags: Vec<String>,
    }

    const GOOD: &str = r#"{"opt":"aaa","boxed":"bbb","tags":["c","c","c"],"cow":"ddd","nested":["e","e","e"],"plain":"fff"}"#;

    let accepted = serde_json::from_str::<WireAgreement>(GOOD).unwrap();
    assert!(
        accepted.validate().is_ok(),
        "the payload every bound admits has to pass: {:?}",
        accepted.validate().err()
    );

    for (field, short) in [
        ("opt", r#""a""#),
        ("boxed", r#""b""#),
        ("tags", r#"["c"]"#),
        ("cow", r#""d""#),
        ("nested", r#"["e"]"#),
        ("plain", r#""f""#),
    ] {
        let mut payload: serde_json::Value = serde_json::from_str(GOOD).unwrap();
        payload[field] = serde_json::from_str(short).unwrap();
        assert_eq!(
            serde_json::from_str::<WireAgreement>(&payload.to_string())
                .unwrap_err()
                .to_string(),
            format!("'{field}': too short: minimum length is 3, got 1"),
            "`{field}` broke its bound and the read did not say so"
        );
    }
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_two_variants_naming_one_field_keep_their_own_constraints() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(tag = "kind")]
    pub enum Action {
        Delete {
            #[model_schema_prop(minLength = 5)]
            note: String,
        },
        Upload {
            #[model_schema_prop(minLength = 3)]
            note: String,
        },
    }

    // The tag names the variant before its members are read, so the value that arrives is never
    // in doubt and the bound is the validator's to apply — the variant's own bound, and only it.
    assert_eq!(
        serde_json::from_str::<Action>(r#"{"kind":"Delete","note":"abc"}"#)
            .unwrap_err()
            .to_string(),
        "'note': too short: minimum length is 5, got 3"
    );
    assert!(
        serde_json::from_str::<Action>(r#"{"kind":"Delete","note":"abcde"}"#)
            .unwrap()
            .validate()
            .is_ok(),
        "Delete admits its own minimum"
    );

    assert_eq!(
        serde_json::from_str::<Action>(r#"{"kind":"Upload","note":"ab"}"#)
            .unwrap_err()
            .to_string(),
        "'note': too short: minimum length is 3, got 2"
    );
    assert!(
        serde_json::from_str::<Action>(r#"{"kind":"Upload","note":"abc"}"#)
            .unwrap()
            .validate()
            .is_ok(),
        "A value Upload admits is not held to Delete's minimum"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_tagged_enum_validate_answers_in_the_struct_twins_words() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct SlugStruct {
        #[model_schema_prop(minLength = 2)]
        pub slug: String,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(tag = "kind")]
    pub enum SlugTagged {
        One {
            #[model_schema_prop(minLength = 2)]
            slug: String,
        },
    }

    let from_struct = SlugStruct {
        slug: "A".to_owned(),
    }
    .validate()
    .unwrap_err();
    let from_enum = SlugTagged::One {
        slug: "A".to_owned(),
    }
    .validate()
    .unwrap_err();

    assert_eq!(
        from_struct,
        vec!["'slug': too short: minimum length is 2, got 1".to_owned()]
    );
    assert_eq!(from_enum, from_struct);

    SlugTagged::One {
        slug: "ab".to_owned(),
    }
    .validate()
    .unwrap();
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_every_tagged_flavor_publishes_validate_for_its_constrained_members() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub enum External {
        One {
            #[model_schema_prop(minLength = 2)]
            slug: String,
        },
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(tag = "kind")]
    pub enum Internal {
        One {
            #[model_schema_prop(minLength = 2)]
            slug: String,
        },
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(tag = "kind", content = "data")]
    pub enum Adjacent {
        One {
            #[model_schema_prop(minLength = 2)]
            slug: String,
        },
    }

    let under = |named: &str| vec![format!("'{named}': too short: minimum length is 2, got 1")];
    assert_eq!(
        External::One {
            slug: "A".to_owned()
        }
        .validate()
        .unwrap_err(),
        under("One.slug")
    );
    assert_eq!(
        Internal::One {
            slug: "A".to_owned()
        }
        .validate()
        .unwrap_err(),
        under("slug")
    );
    assert_eq!(
        Adjacent::One {
            slug: "A".to_owned()
        }
        .validate()
        .unwrap_err(),
        under("data.slug")
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_enum_validate_runs_only_the_held_variants_checks() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(tag = "kind")]
    pub enum Action {
        Delete {
            #[model_schema_prop(minLength = 5)]
            note: String,
        },
        Upload {
            #[model_schema_prop(minLength = 3)]
            note: String,
        },
    }

    assert_eq!(
        Action::Delete {
            note: "abc".to_owned()
        }
        .validate()
        .unwrap_err(),
        vec!["'note': too short: minimum length is 5, got 3".to_owned()]
    );
    assert!(
        Action::Upload {
            note: "abc".to_owned()
        }
        .validate()
        .is_ok(),
        "a value Upload admits is not held to Delete's minimum"
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_enum_validate_collects_every_violation_of_the_held_variant() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(tag = "kind")]
    pub enum Mixed {
        Bounded {
            #[model_schema_prop(maxLength = 3)]
            code: String,
            #[model_schema_prop(minimum = 10)]
            size: u32,
        },
        Free {
            note: String,
        },
        Nothing,
    }

    assert_eq!(
        Mixed::Bounded {
            code: "toolong".to_owned(),
            size: 1,
        }
        .validate()
        .unwrap_err(),
        vec![
            "'code': too long: maximum length is 3, got 7".to_owned(),
            "'size': too small: minimum is 10, got 1".to_owned(),
        ]
    );
    Mixed::Free {
        note: "anything".to_owned(),
    }
    .validate()
    .unwrap();
    Mixed::Nothing.validate().unwrap();
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_enum_validate_reaches_through_a_members_wrappers() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(tag = "kind")]
    pub enum Wrapped {
        One {
            #[model_schema_prop(minLength = 2)]
            tags: Vec<String>,
            #[model_schema_prop(minLength = 2)]
            #[serde(skip_serializing_if = "Option::is_none")]
            note: Option<String>,
        },
    }

    assert!(
        Wrapped::One {
            tags: vec!["a".to_owned(), "b".to_owned()],
            note: None,
        }
        .validate()
        .is_ok(),
        "a None writes nothing for the bound to describe, and two items are two whatever they hold"
    );
    assert_eq!(
        Wrapped::One {
            tags: vec!["long enough".to_owned()],
            note: Some("d".to_owned()),
        }
        .validate()
        .unwrap_err(),
        vec![
            "'tags': too short: minimum length is 2, got 1".to_owned(),
            "'note': too short: minimum length is 2, got 1".to_owned(),
        ]
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_a_constraint_free_enum_publishes_no_validate_just_as_a_struct_does() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct FreeStruct {
        pub name: String,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(tag = "kind")]
    pub enum FreeTagged {
        One { name: String },
        Two,
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(tag = "kind")]
    pub enum BoundTagged {
        One {
            #[model_schema_prop(minLength = 2)]
            name: String,
        },
    }

    impl UnpublishedValidate for FreeStruct {}
    impl UnpublishedValidate for FreeTagged {}

    assert_eq!(
        FreeStruct {
            name: String::new()
        }
        .validate(),
        "no inherent validate()"
    );
    assert_eq!(
        FreeTagged::One {
            name: String::new()
        }
        .validate(),
        "no inherent validate()"
    );
    assert_eq!(FreeTagged::Two.validate(), "no inherent validate()");
    // An enum whose member carries a bound answers with the accessor's own type: a published
    // `validate()` would shadow the trait's, and none of the above would compile.
    assert_eq!(
        BoundTagged::One {
            name: "A".to_owned()
        }
        .validate()
        .unwrap_err(),
        vec!["'name': too short: minimum length is 2, got 1".to_owned()]
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_a_member_named_like_the_walks_own_bindings_is_still_checked() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(tag = "kind")]
    pub enum Shadowing {
        One {
            #[model_schema_prop(minLength = 2)]
            errors: String,
            #[model_schema_prop(minLength = 2)]
            value_0: Vec<String>,
        },
    }

    assert_eq!(
        Shadowing::One {
            errors: "A".to_owned(),
            value_0: vec!["long enough".to_owned()],
        }
        .validate()
        .unwrap_err(),
        vec![
            "'errors': too short: minimum length is 2, got 1".to_owned(),
            "'value_0': too short: minimum length is 2, got 1".to_owned(),
        ]
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_every_variant_shape_reaches_the_accessor() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub enum EveryShape {
        Bare {
            note: String,
        },
        Named {
            #[model_schema_prop(minLength = 2)]
            slug: String,
        },
        Nothing,
        Pair(String, String),
        Single(String),
    }

    assert_eq!(
        EveryShape::Named {
            slug: "A".to_owned()
        }
        .validate()
        .unwrap_err(),
        vec!["'Named.slug': too short: minimum length is 2, got 1".to_owned()]
    );
    EveryShape::Bare {
        note: "A".to_owned(),
    }
    .validate()
    .unwrap();
    EveryShape::Single("A".to_owned()).validate().unwrap();
    EveryShape::Pair("A".to_owned(), "B".to_owned())
        .validate()
        .unwrap();
    EveryShape::Nothing.validate().unwrap();
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_a_tagged_tuple_variants_slot_is_checked_where_serde_writes_it() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub enum SlotExternal {
        Code(#[model_schema_prop(pattern = "^[A-Z]{3}$")] String),
        Pair(
            #[model_schema_prop(minLength = 2)] String,
            #[model_schema_prop(minimum = 1)] u32,
        ),
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(tag = "kind", content = "data")]
    pub enum SlotAdjacent {
        Code(#[model_schema_prop(pattern = "^[A-Z]{3}$")] String),
        Pair(
            #[model_schema_prop(minLength = 2)] String,
            #[model_schema_prop(minimum = 1)] u32,
        ),
    }

    // serde writes a variant under its name, or under the content key the enum names.
    let code = |held: &str| vec![format!("'{held}': does not match pattern '^[A-Z]{{3}}$'")];
    assert_eq!(
        SlotExternal::Code("abc".to_owned()).validate().unwrap_err(),
        code("Code")
    );
    assert_eq!(
        SlotAdjacent::Code("abc".to_owned()).validate().unwrap_err(),
        code("data")
    );
    SlotExternal::Code("ABC".to_owned()).validate().unwrap();
    SlotAdjacent::Code("ABC".to_owned()).validate().unwrap();

    let pair = |held: &str| {
        vec![
            format!("'{held}.0': too short: minimum length is 2, got 1"),
            format!("'{held}.1': too small: minimum is 1, got 0"),
        ]
    };
    assert_eq!(
        SlotExternal::Pair("a".to_owned(), 0)
            .validate()
            .unwrap_err(),
        pair("Pair")
    );
    assert_eq!(
        SlotAdjacent::Pair("a".to_owned(), 0)
            .validate()
            .unwrap_err(),
        pair("data")
    );
    assert_eq!(
        SlotExternal::Pair("ab".to_owned(), 0)
            .validate()
            .unwrap_err(),
        pair("Pair")[1..]
    );
    assert_eq!(
        SlotAdjacent::Pair("a".to_owned(), 1)
            .validate()
            .unwrap_err(),
        pair("data")[..1]
    );
    SlotExternal::Pair("ab".to_owned(), 1).validate().unwrap();

    assert_eq!(
        slot_external_schema::validate_code_0_value("abc").unwrap_err(),
        code("Code")
    );
    assert_eq!(
        slot_adjacent_schema::validate_pair_1_value(&0).unwrap_err(),
        pair("data")[1..]
    );
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_a_tagged_tuple_variants_checked_slot_is_written_as_before_and_read_as_validate_judges_it() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
    pub enum WireExternal {
        Code(#[model_schema_prop(pattern = "^[A-Z]{3}$")] String),
        Pair(
            #[model_schema_prop(minLength = 2)] String,
            #[model_schema_prop(minimum = 1)] u32,
        ),
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
    #[serde(tag = "kind", content = "data")]
    pub enum WireAdjacent {
        Code(#[model_schema_prop(pattern = "^[A-Z]{3}$")] String),
        Pair(
            #[model_schema_prop(minLength = 2)] String,
            #[model_schema_prop(minimum = 1)] u32,
        ),
    }

    // A slot that breaks its check is refused with what `validate()` answers of the same value.
    for (wire, built) in [
        (r#"{"Code":"abc"}"#, WireExternal::Code("abc".to_owned())),
        (r#"{"Pair":["a",0]}"#, WireExternal::Pair("a".to_owned(), 0)),
    ] {
        assert_eq!(serde_json::to_string(&built).unwrap(), wire);
        assert_eq!(
            serde_json::from_str::<WireExternal>(wire)
                .unwrap_err()
                .to_string(),
            built.validate().unwrap_err().join("; ")
        );
    }
    for (wire, built) in [
        (
            r#"{"kind":"Code","data":"abc"}"#,
            WireAdjacent::Code("abc".to_owned()),
        ),
        (
            r#"{"kind":"Pair","data":["a",0]}"#,
            WireAdjacent::Pair("a".to_owned(), 0),
        ),
    ] {
        assert_eq!(serde_json::to_string(&built).unwrap(), wire);
        assert_eq!(
            serde_json::from_str::<WireAdjacent>(wire)
                .unwrap_err()
                .to_string(),
            built.validate().unwrap_err().join("; ")
        );
    }

    for (wire, kept) in [
        (r#"{"Code":"ABC"}"#, WireExternal::Code("ABC".to_owned())),
        (
            r#"{"Pair":["ab",1]}"#,
            WireExternal::Pair("ab".to_owned(), 1),
        ),
    ] {
        assert_eq!(serde_json::to_string(&kept).unwrap(), wire);
        assert_eq!(serde_json::from_str::<WireExternal>(wire).unwrap(), kept);
    }
    for (wire, kept) in [
        (
            r#"{"kind":"Code","data":"ABC"}"#,
            WireAdjacent::Code("ABC".to_owned()),
        ),
        (
            r#"{"kind":"Pair","data":["ab",1]}"#,
            WireAdjacent::Pair("ab".to_owned(), 1),
        ),
    ] {
        assert_eq!(serde_json::to_string(&kept).unwrap(), wire);
        assert_eq!(serde_json::from_str::<WireAdjacent>(wire).unwrap(), kept);
    }
}

#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn test_a_tagged_tuple_variants_slot_check_reaches_zod() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub enum ZodExternal {
        Code(#[model_schema_prop(pattern = "^[A-Z]{3}$")] String),
        Pair(
            #[model_schema_prop(minLength = 2)] String,
            #[model_schema_prop(minimum = 1)] u32,
        ),
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(tag = "kind", content = "data")]
    pub enum ZodAdjacent {
        Code(#[model_schema_prop(pattern = "^[A-Z]{3}$")] String),
        Pair(
            #[model_schema_prop(minLength = 2)] String,
            #[model_schema_prop(minimum = 1)] u32,
        ),
    }

    let code = "z.string().check(z.regex(/^[A-Z]{3}$/, { error: \"does not match pattern \
                '^[A-Z]{3}$'\" })),";
    let pair = "z.tuple([z.string().min(2, { error: (issue) => `too short: minimum length is 2, \
                got ${String(issue.input).length}` }), z.number().int().max(4294967295).min(1, { error: (issue) \
                => `too small: minimum is 1, got ${String(issue.input)}` })]),";
    let external = ZodExternal::zod_schema();
    let adjacent = ZodAdjacent::zod_schema();
    for (key, member) in [("\"Code\"", code), ("\"Pair\"", pair)] {
        let written = format!("{key}: {member}");
        assert!(external.contains(&written), "Got:\n{external}");
    }
    for member in [code, pair] {
        let written = format!("data: {member}");
        assert!(adjacent.contains(&written), "Got:\n{adjacent}");
    }
}

#[cfg(all(feature = "serde", feature = "jsonschema"))]
#[test]
fn test_a_tagged_tuple_variants_slot_check_reaches_the_json_schema() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub enum JsonExternal {
        Code(#[model_schema_prop(minLength = 3, maxLength = 3, pattern = "^[A-Z]{3}$")] String),
        Pair(
            #[model_schema_prop(minLength = 2)] String,
            #[model_schema_prop(minimum = 1, maximum = 10)] u32,
        ),
        Tags(#[model_schema_prop(minLength = 2)] Vec<String>),
    }

    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(tag = "kind", content = "data")]
    pub enum JsonAdjacent {
        Code(#[model_schema_prop(minLength = 3, maxLength = 3, pattern = "^[A-Z]{3}$")] String),
        Pair(
            #[model_schema_prop(minLength = 2)] String,
            #[model_schema_prop(minimum = 1, maximum = 10)] u32,
        ),
        Tags(#[model_schema_prop(minLength = 2)] Vec<String>),
    }

    let code = serde_json::json!({
        "type": "string", "minLength": 3_u32, "maxLength": 3_u32, "pattern": "^[A-Z]{3}$"
    });
    let pair = serde_json::json!([
        { "type": "string", "minLength": 2_u32 },
        { "type": "integer", "minimum": 1.0_f64, "maximum": 10.0_f64 }
    ]);
    let tags = serde_json::json!({
        "type": "array", "items": { "type": "string" }, "minItems": 2_u32
    });

    let external = JsonExternal::json_schema();
    assert_eq!(external["oneOf"][0]["properties"]["Code"], code);
    assert_eq!(
        external["oneOf"][1]["properties"]["Pair"]["prefixItems"],
        pair
    );
    assert_eq!(external["oneOf"][2]["properties"]["Tags"], tags);

    let adjacent = JsonAdjacent::json_schema();
    assert_eq!(adjacent["oneOf"][0]["properties"]["data"], code);
    assert_eq!(
        adjacent["oneOf"][1]["properties"]["data"]["prefixItems"],
        pair
    );
    assert_eq!(adjacent["oneOf"][2]["properties"]["data"], tags);
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_a_tagged_slot_is_reached_through_its_wrappers() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub enum SlotWrapped {
        Note(#[model_schema_prop(minLength = 2)] Option<String>),
        Tags(#[model_schema_prop(minLength = 2)] Vec<String>),
    }

    SlotWrapped::Tags(vec!["c".to_owned(), "d".to_owned()])
        .validate()
        .unwrap();
    assert_eq!(
        SlotWrapped::Tags(vec!["long enough".to_owned()])
            .validate()
            .unwrap_err(),
        vec!["'Tags': too short: minimum length is 2, got 1".to_owned()],
        "a slot typed as a list is counted, as a named member is"
    );
    assert_eq!(
        SlotWrapped::Note(Some("e".to_owned()))
            .validate()
            .unwrap_err(),
        vec!["'Note': too short: minimum length is 2, got 1".to_owned()]
    );
    SlotWrapped::Note(None).validate().unwrap();
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_readme_slot_validate_example_prints_what_the_generator_writes() {
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
    pub enum Entry {
        Code(#[model_schema_prop(pattern = "^[A-Z]{3}$")] String),
        Pair(
            #[model_schema_prop(minLength = 2)] String,
            #[model_schema_prop(minimum = 1)] u32,
        ),
    }

    let readme = include_str!("../../README.md");
    for reported in [
        Entry::Code("abc".to_owned()).validate(),
        Entry::Pair("a".to_owned(), 0).validate(),
    ] {
        reported.as_ref().unwrap_err();
        let shown = format!("// {reported:?}");
        assert!(
            readme.contains(&shown),
            "the README no longer shows {shown}"
        );
    }

    for (wire, built) in [
        (r#"{"Code":"abc"}"#, Entry::Code("abc".to_owned())),
        (r#"{"Pair":["a",0]}"#, Entry::Pair("a".to_owned(), 0)),
    ] {
        assert_eq!(
            serde_json::from_str::<Entry>(wire).unwrap_err().to_string(),
            built.validate().unwrap_err().join("; ")
        );
        assert_eq!(serde_json::to_string(&built).unwrap(), wire);
        assert!(readme.contains(wire), "the README no longer shows {wire}");
    }
    for (wire, kept) in [
        (r#"{"Code":"ABC"}"#, Entry::Code("ABC".to_owned())),
        (r#"{"Pair":["ab",1]}"#, Entry::Pair("ab".to_owned(), 1)),
    ] {
        assert_eq!(serde_json::from_str::<Entry>(wire).unwrap(), kept);
        assert_eq!(serde_json::to_string(&kept).unwrap(), wire);
        assert!(readme.contains(wire), "the README no longer shows {wire}");
    }
    entry_schema::validate_code_0_value("ABC").unwrap();
    entry_schema::validate_pair_1_value(&1).unwrap();
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_readme_struct_validate_example_prints_what_the_generator_writes() {
    // The README declares `username` first; source here is ordered alphabetically, as this crate's
    // lints require of Rust source.
    #[model_schema()]
    #[derive(Serialize, Deserialize, Debug)]
    pub struct Registration {
        #[model_schema_prop(minimum = 0, maximum = 120)]
        pub age: u32,

        #[model_schema_prop(minLength = 3, maxLength = 30)]
        pub username: String,
    }

    let reg = Registration {
        age: 150,
        username: "ab".to_owned(),
    };
    let errors = reg.validate().unwrap_err();
    assert_eq!(
        errors,
        vec![
            "'age': too large: maximum is 120, got 150".to_owned(),
            "'username': too short: minimum length is 3, got 2".to_owned(),
        ]
    );

    let readme = include_str!("../../README.md");
    for error in &errors {
        assert!(
            readme.contains(error.as_str()),
            "the README no longer shows this error verbatim: {error}"
        );
    }
}

/// The same holding for the branded newtype's `validate()` example.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_readme_branded_validate_example_prints_what_the_generator_writes() {
    #[model_schema(pattern = "^[a-z0-9_]+$", minLength = 3, maxLength = 50)]
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(transparent)]
    pub struct SlugId(pub String);

    SlugId("hello_world".to_owned()).validate().unwrap();

    let errors = SlugId("ab".to_owned()).validate().unwrap_err();
    assert_eq!(
        errors,
        vec!["too short: minimum length is 3, got 2".to_owned()]
    );

    let readme = include_str!("../../README.md");
    for error in &errors {
        assert!(
            readme.contains(error.as_str()),
            "the README no longer shows this error verbatim: {error}"
        );
    }
}

#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn test_crate_rustdoc_quotes_the_messages_the_generator_writes() {
    #[model_schema()]
    #[derive(Serialize, Deserialize)]
    pub struct RegistrationData {
        #[model_schema_prop(minimum = 0, maximum = 120)]
        pub age: u32,

        #[model_schema_prop(minLength = 3, maxLength = 30)]
        pub username: String,
    }

    let errors = RegistrationData {
        age: 150,
        username: "ab".to_owned(),
    }
    .validate()
    .unwrap_err();

    assert_eq!(
        errors,
        vec![
            "'age': too large: maximum is 120, got 150".to_owned(),
            "'username': too short: minimum length is 3, got 2".to_owned(),
        ]
    );

    let rustdoc = include_str!("../../src/lib.rs");
    for message in &errors {
        let shown = format!("// \"{message}\"");
        assert!(
            rustdoc.contains(&shown),
            "src/lib.rs no longer shows {shown}"
        );
    }
}
