//! Unit tests of the read the macro writes for a type that publishes `validate()`.

use quote::ToTokens as _;

use super::{of_enum, of_struct};
use crate::model_schema::exec_model_schema;

/// The whole expansion of `source` under `#[model_schema(...)]` written with `args`.
fn expansion_under(args: &str, source: &str) -> String {
    exec_model_schema(
        syn::parse_str(args).unwrap(),
        syn::parse_str(source).unwrap(),
    )
    .to_string()
}

/// The item as it is left, and what its read gains, where it publishes `validate()`.
fn judged_struct(source: &str) -> (String, String) {
    let mut item: syn::ItemStruct = syn::parse_str(source).unwrap();
    let read = of_struct(&mut item, true).to_string();
    (item.to_token_stream().to_string(), read)
}

#[test]
fn a_conversion_of_the_authors_own_is_refused_on_a_type_that_publishes_validate() {
    for key in ["remote", "from", "try_from", "into"] {
        let source = format!(
            "#[derive(Deserialize)] #[serde({key} = \"Wire\")] pub struct Sealed {{ pub label: \
             String }}"
        );
        let expanded = expansion_under("check = [\"withinSealed\"]", &source);
        assert!(expanded.contains("compile_error"), "for {key}: {expanded}");
        let refusal = format!(
            "model_schema: type `Sealed`: `#[serde({key} = ...)]` puts a conversion of its \
             author's between the type and what serde reads or writes for it, and `Sealed` \
             publishes `validate()`, so `#[model_schema()]` writes its `Deserialize`: the read \
             of its members, then `validate()`. It cannot put that judgement behind a conversion \
             it does not write. Take `{key}` off the type and convert where the value is read, \
             or take off the bounds and the checks that make the type publish `validate()`."
        );
        assert!(
            expanded.contains(&refusal),
            "for {key}, missing `{refusal}` in: {expanded}"
        );
        assert!(
            !expanded.contains("model_schema_read_unjudged"),
            "for {key}: {expanded}"
        );
        if key == "try_from" {
            let readme = include_str!("../../../README.md");
            assert!(
                readme.contains(&format!("error: {refusal}\n")),
                "the README no longer shows the refusal"
            );
        }

        let on_an_enum = format!(
            "#[derive(Deserialize)] #[serde({key} = \"Wire\")] pub enum Sealed {{ Open, Shut }}"
        );
        let mut item: syn::ItemEnum = syn::parse_str(&on_an_enum).unwrap();
        let refused = of_enum(&mut item, true).to_string();
        assert!(refused.contains("compile_error"), "for {key}: {refused}");
        assert!(
            refused.contains(&format!("`#[serde({key} = ...)]`")),
            "for {key}: {refused}"
        );
    }
}

#[test]
fn a_conversion_is_left_alone_where_the_macro_writes_no_read() {
    for source in [
        // No `validate()` is published.
        "#[derive(Deserialize)] #[serde(try_from = \"Wire\")] pub struct Sealed { pub label: String }",
        // Nothing is read.
        "#[derive(Serialize)] #[serde(into = \"Wire\")] pub struct Sealed { pub label: String }",
    ] {
        let expanded = expansion_under("", source);
        assert!(
            !expanded.contains("compile_error"),
            "for {source}: {expanded}"
        );
    }
    let written =
        "#[derive(Serialize)] #[serde(into = \"Wire\")] pub struct Sealed { pub label: String }";
    let mut item: syn::ItemStruct = syn::parse_str(written).unwrap();
    assert!(of_struct(&mut item, true).is_empty());
}

#[test]
fn an_item_the_macro_writes_no_read_for_is_left_as_it_was_written() {
    for (source, publishes_validate) in [
        (
            "#[derive(Debug, Deserialize)] #[serde(rename_all = \"camelCase\")] pub struct \
             Parcel { #[serde(default)] pub item_label: String }",
            false,
        ),
        (
            "#[derive(Debug, Serialize)] #[serde(rename_all = \"camelCase\")] pub struct Parcel \
             { #[serde(skip_serializing_if = \"String::is_empty\")] pub item_label: String }",
            true,
        ),
        ("pub struct Parcel { pub item_label: String }", true),
    ] {
        let mut item: syn::ItemStruct = syn::parse_str(source).unwrap();
        let written = item.to_token_stream().to_string();
        assert!(
            of_struct(&mut item, publishes_validate).is_empty(),
            "for {source}"
        );
        assert_eq!(item.to_token_stream().to_string(), written, "for {source}");
    }
    let source = "#[derive(Deserialize)] pub enum Dock { North, South }";
    let mut item: syn::ItemEnum = syn::parse_str(source).unwrap();
    let written = item.to_token_stream().to_string();
    assert!(of_enum(&mut item, false).is_empty());
    assert_eq!(item.to_token_stream().to_string(), written);
}

#[test]
fn the_read_is_serdes_derive_on_a_copy_of_the_item_followed_by_validate() {
    let (item, read) = judged_struct(
        "#[derive(Debug, Serialize, Deserialize)] #[serde(rename_all = \"camelCase\")] pub \
         struct Parcel { #[serde(default)] pub item_label: String }",
    );
    // `Serialize` stays the derive's, and reads the attributes as they were written.
    assert_eq!(
        item,
        "# [derive (Debug , Serialize)] # [serde (rename_all = \"camelCase\")] pub struct Parcel \
         { # [serde (default)] pub item_label : String }"
    );
    for written in [
        "impl Parcel { # [doc = \" The read alone, for a holder that judges the whole value \
         once.\"] pub fn model_schema_read_unjudged < 'de , D > (deserializer : D) -> :: core :: \
         result :: Result < Self , D :: Error > where D : serde :: Deserializer < 'de > , {",
        "# [derive (Deserialize)] # [serde (remote = \"Parcel\")] # [serde (bound (deserialize = \
         \"\"))] # [serde (rename_all = \"camelCase\")] struct ModelSchemaRemote { # [serde \
         (default)] item_label : String }",
        "< ModelSchemaRemote > :: deserialize (deserializer) } }",
        "impl < 'de > serde :: Deserialize < 'de > for Parcel { fn deserialize < D > \
         (deserializer : D) -> :: core :: result :: Result < Self , D :: Error > where D : \
         serde :: Deserializer < 'de > , { let read = Self :: model_schema_read_unjudged \
         (deserializer) ? ; read . validate () . map_err (",
        "serde :: de :: Error :: custom (violations . join (\"; \"))",
        "? ; :: core :: result :: Result :: Ok (read) } }",
    ] {
        assert!(read.contains(written), "missing `{written}` in: {read}");
    }
    for unwritten in ["allow", "expect", "doc (hidden)", "impl Trait"] {
        assert!(!read.contains(unwritten), "found `{unwritten}` in: {read}");
    }
}

#[test]
fn an_item_that_only_reads_keeps_no_attribute_only_the_read_understood() {
    let (item, read) = judged_struct(
        "#[derive(serde::Deserialize)] #[serde(deny_unknown_fields)] pub struct Parcel { \
         #[serde(alias = \"label\")] pub item_label: String }",
    );
    assert_eq!(item, "pub struct Parcel { pub item_label : String }");
    for written in [
        "# [derive (serde :: Deserialize)] # [serde (remote = \"Parcel\")]",
        "# [serde (deny_unknown_fields)] struct ModelSchemaRemote { # [serde (alias = \
         \"label\")] item_label : String }",
    ] {
        assert!(read.contains(written), "missing `{written}` in: {read}");
    }
}

#[test]
fn the_read_of_a_generic_item_is_written_under_the_bounds_of_serdes_derive() {
    let (_, read) = judged_struct(
        "#[derive(Deserialize)] pub struct Lent<'a, T: Clone> where T: Send { pub held: T, \
         pub marked: PhantomData<T>, pub text: &'a str }",
    );
    for written in [
        "impl < 'a , T : Clone > Lent < 'a , T > where T : Send {",
        "pub fn model_schema_read_unjudged < 'de , D > (deserializer : D) -> :: core :: result \
         :: Result < Self , D :: Error > where D : serde :: Deserializer < 'de > , 'de : 'a , T \
         : serde :: Deserialize < 'de > , {",
        "# [serde (remote = \"Lent\")] # [serde (bound (deserialize = \"T : serde :: \
         Deserialize < 'de >\"))] struct ModelSchemaRemote < 'a , T : Clone > where T : Send {",
        "< ModelSchemaRemote < 'a , T > > :: deserialize (deserializer)",
        "impl < 'de , 'a , T : Clone > serde :: Deserialize < 'de > for Lent < 'a , T > where T \
         : Send , 'de : 'a , T : serde :: Deserialize < 'de > {",
    ] {
        assert!(read.contains(written), "missing `{written}` in: {read}");
    }
}

#[test]
fn a_type_that_borrows_for_static_is_read_at_static() {
    let (_, read) =
        judged_struct("#[derive(Deserialize)] pub struct Fixed { pub name: &'static str }");
    for written in [
        "pub fn model_schema_read_unjudged < D > (deserializer : D) -> :: core :: result :: \
         Result < Self , D :: Error > where D : serde :: Deserializer < 'static > , {",
        "impl serde :: Deserialize < 'static > for Fixed {",
    ] {
        assert!(read.contains(written), "missing `{written}` in: {read}");
    }
}

#[test]
fn the_copy_names_the_type_wherever_the_item_writes_self() {
    let (_, read) = judged_struct(
        "#[derive(Deserialize)] #[serde(default)] pub struct Crate { pub nested: \
         Vec<Self>, pub label: String }",
    );
    for written in [
        "where D : serde :: Deserializer < 'de > , Self : :: core :: default :: Default , {",
        "# [serde (bound (deserialize = \"Crate : :: core :: default :: Default\"))] # [serde \
         (default)] struct ModelSchemaRemote { nested : Vec < Crate > , label : String }",
    ] {
        assert!(read.contains(written), "missing `{written}` in: {read}");
    }
}

#[test]
fn the_reader_is_never_a_name_the_item_writes() {
    let (_, read) =
        judged_struct("#[derive(Deserialize)] pub struct Docked<D> { pub dock: D, pub d2: D2 }");
    for written in [
        "pub fn model_schema_read_unjudged < 'de , D3 > (deserializer : D3) -> :: core :: \
         result :: Result < Self , D3 :: Error > where D3 : serde :: Deserializer < 'de > , D : \
         serde :: Deserialize < 'de > , {",
        "fn deserialize < D3 > (deserializer : D3)",
    ] {
        assert!(read.contains(written), "missing `{written}` in: {read}");
    }
}

#[test]
fn an_enum_is_read_on_a_copy_that_keeps_what_serde_reads_of_each_variant() {
    let mut item: syn::ItemEnum = syn::parse_str(
        "#[derive(Deserialize)] #[serde(tag = \"kind\")] pub enum Step { #[serde(rename = \
         \"rename\")] Rename { #[serde(default)] to: String } = 1, Stop }",
    )
    .unwrap();
    let read = of_enum(&mut item, true).to_string();
    assert_eq!(
        item.to_token_stream().to_string(),
        "pub enum Step { Rename { to : String } = 1 , Stop }"
    );
    let copy = "# [derive (Deserialize)] # [serde (remote = \"Step\")] # [serde (bound \
                (deserialize = \"\"))] # [serde (tag = \"kind\")] enum ModelSchemaRemote { # \
                [serde (rename = \"rename\")] Rename { # [serde (default)] to : String } , Stop }";
    assert!(read.contains(copy), "missing `{copy}` in: {read}");
}

/// A member of an untagged enum is held to the bound a brand carries as it is read, behind a
/// trait that answers for a type with no such bound, and a struct's member is held to none.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_member_that_chooses_a_variant_is_held_to_the_bound_of_a_brand() {
    let chosen = expansion_under(
        "",
        "#[derive(Deserialize)] #[serde(untagged)] pub enum ChosenMark { Labelled(ChosenLabel), \
         Named(ChosenByShape) }",
    );
    for written in [
        "trait ChosenByShape2 { fn model_schema_validate_bound (& self) -> :: core :: result :: \
         Result < () , :: std :: vec :: Vec < :: std :: string :: String >> { :: core :: result \
         :: Result :: Ok (()) } } impl < T : ? :: core :: marker :: Sized > ChosenByShape2 for & \
         T { }",
        "let held = < ChosenLabel > :: model_schema_read_unjudged (deserializer) ? ; let chosen \
         = & held ; chosen . model_schema_validate_bound () . map_err (",
        ":: core :: result :: Result :: Ok (Unjudged (held))",
    ] {
        assert!(chosen.contains(written), "missing `{written}` in: {chosen}");
    }

    let held = expansion_under(
        "",
        "#[derive(Deserialize)] pub struct ChosenHolder { pub label: ChosenLabel }",
    );
    assert!(
        held.contains(
            "< ChosenLabel > :: model_schema_read_unjudged (deserializer) . map (Unjudged)"
        ),
        "got: {held}"
    );
    for unwritten in ["model_schema_validate_bound", "ChosenByShape"] {
        assert!(!held.contains(unwritten), "found `{unwritten}` in: {held}");
    }
}

/// A member that names one of the item's type parameters is read through a hook generic over
/// exactly the parameters it names, every bound in its `where` clause, each once, and the type the
/// model is read as declares only the parameters the model names.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_member_that_names_a_type_parameter_is_read_through_a_hook_generic_over_it() {
    let read = expansion_under(
        "default_types(IdType = String, Held = String)",
        "#[derive(Deserialize)] #[serde(bound(deserialize = \"IdType: \
         serde::de::DeserializeOwned, Held: serde::de::DeserializeOwned\"))] pub struct \
         HookedLedger<IdType: Display + Eq + Hash, Held> where IdType: Display { pub by_id: \
         HashMap<IdType, HookedTag>, pub entries: Vec<HookedEntry<IdType>>, pub held: Vec<Held> }",
    );
    for written in [
        "fn read_unjudged_entries < 'de , IdType , D > (deserializer : D) -> :: core :: result :: \
         Result < Vec < HookedEntry < IdType > > , D :: Error > where D : serde :: Deserializer < \
         'de > , IdType : Display + Eq + Hash + serde :: de :: DeserializeOwned , { struct \
         Unjudged < IdType > (HookedEntry < IdType >) where IdType : Display + Eq + Hash ; impl < \
         'de , IdType > serde :: Deserialize < 'de > for Unjudged < IdType > where IdType : \
         Display + Eq + Hash + serde :: de :: DeserializeOwned {",
        "let held_0 = < Vec < Unjudged < IdType > > as serde :: Deserialize > :: deserialize \
         (deserializer) ? ;",
        "fn read_unjudged_by_id < 'de , IdType , D > (deserializer : D) -> :: core :: result :: \
         Result < HashMap < IdType , HookedTag > , D :: Error > where D : serde :: Deserializer < \
         'de > , IdType : Display + Eq + Hash + serde :: de :: DeserializeOwned , { struct \
         Unjudged (HookedTag) ; impl < 'de > serde :: Deserialize < 'de > for Unjudged {",
        "# [serde (deserialize_with = \"read_unjudged_entries\")] entries : Vec < HookedEntry < \
         IdType > >",
    ] {
        assert!(read.contains(written), "missing `{written}` in: {read}");
    }
    assert!(!read.contains("read_unjudged_held"), "got: {read}");
}

/// `Self` is written as the item where a hook writes the member's type and the model it holds.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_member_written_with_self_is_read_through_a_hook_that_names_the_item() {
    let node = expansion_under(
        "check = [\"countedNode\"]",
        "#[derive(Deserialize)] pub struct HookedNode { pub children: Vec<Self> }",
    );
    for written in [
        "fn read_unjudged_children < 'de , D > (deserializer : D) -> :: core :: result :: Result \
         < Vec < HookedNode > , D :: Error > where D : serde :: Deserializer < 'de > , { struct \
         Unjudged (HookedNode) ; impl < 'de > serde :: Deserialize < 'de > for Unjudged {",
        "< HookedNode > :: model_schema_read_unjudged (deserializer) . map (Unjudged)",
        ". collect :: < Vec < HookedNode > > ()",
    ] {
        assert!(node.contains(written), "missing `{written}` in: {node}");
    }

    let tree = expansion_under(
        "default_types(IdType = String), check = [\"countedTree\"]",
        "#[derive(Deserialize)] pub struct HookedTree<IdType> { pub children: Vec<Self>, pub id: \
         IdType }",
    );
    let generic = "fn read_unjudged_children < 'de , IdType , D > (deserializer : D) -> :: core \
                   :: result :: Result < Vec < HookedTree < IdType > > , D :: Error > where D : \
                   serde :: Deserializer < 'de > , IdType : serde :: Deserialize < 'de > , { \
                   struct Unjudged < IdType > (HookedTree < IdType >) ;";
    assert!(tree.contains(generic), "missing `{generic}` in: {tree}");

    // The type the model is read as is named after none of the names the item writes.
    let named_alike = expansion_under(
        "check = [\"countedUnjudged\"]",
        "#[derive(Deserialize)] pub struct Unjudged { pub children: Vec<Self> }",
    );
    let renamed = "struct Unjudged2 (Unjudged) ;";
    assert!(
        named_alike.contains(renamed),
        "missing `{renamed}` in: {named_alike}"
    );
}

/// A bound that ties the parameter a member names to another, or to a lifetime of the item, and
/// a read at `'static`, leave the member to its own `Deserialize`.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_member_the_items_bounds_keep_from_a_hook_is_read_with_none() {
    for (args, source) in [
        (
            "default_types(IdType = String, Held = String)",
            "#[derive(Deserialize)] pub struct TiedLedger<IdType, Held> where IdType: From<Held> \
             { pub entries: Vec<HookedEntry<IdType>>, pub held: Vec<Held> }",
        ),
        (
            "default_types(IdType = String)",
            "#[derive(Deserialize)] pub struct LentLedger<'text, IdType: 'text> { pub entries: \
             Vec<HookedEntry<IdType>>, pub label: &'text str }",
        ),
        (
            "default_types(IdType = String)",
            "#[derive(Deserialize)] pub struct FixedLedger<IdType> { pub entries: \
             Vec<HookedEntry<IdType>>, pub label: &'static str }",
        ),
    ] {
        let read = expansion_under(args, source);
        assert!(
            read.contains("model_schema_read_unjudged"),
            "for {source}: {read}"
        );
        // No hook is written, and no trait a hook would call is left unused.
        for unwritten in ["read_unjudged_entries", "ReadUnjudged"] {
            assert!(
                !read.contains(unwritten),
                "found `{unwritten}` for {source}: {read}"
            );
        }
    }
}

/// A member of a variant serde never reads, under `skip` or `skip_deserializing`, is read by no
/// hook under any tagging, and a bound of a member of an untagged one hangs no hook on it, where a
/// member of a variant serde reads keeps each of its own.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_member_of_a_variant_serde_never_reads_is_read_by_no_hook() {
    for tagging in ["", "#[serde(tag = \"kind\")]", "#[serde(untagged)]"] {
        let source = format!(
            "#[derive(Serialize, Deserialize)] {tagging} pub enum UnreadHooks {{ Open {{ label: \
             HookedLabel, #[model_schema_prop(minLength = 1)] text: String }}, #[serde(skip)] \
             Closed {{ cache: HookedLabel }}, #[serde(skip_deserializing)] Shut {{ cache: \
             HookedLabel, #[model_schema_prop(minLength = 1)] note: String }} }}"
        );
        let read = expansion_under("", &source);
        assert!(
            read.contains("fn read_unjudged_open_label"),
            "for {source}: {read}"
        );
        for unwritten in ["read_unjudged_closed_cache", "read_unjudged_shut_cache"] {
            assert!(
                !read.contains(unwritten),
                "found `{unwritten}` for {source}: {read}"
            );
        }
        if tagging.contains("untagged") {
            assert!(
                read.contains("deserialize_with = \"unread_hooks_schema::deserialize_open_text\""),
                "for {source}: {read}"
            );
        }
        assert!(
            !read.contains("deserialize_with = \"unread_hooks_schema::deserialize_shut_note\""),
            "for {source}: {read}"
        );
    }
}

/// The parameters a member's type names, in the order the item declares them, and `None` where
/// serde's call of a hook could not infer one, or where it names a const parameter.
#[cfg(feature = "serde")]
#[test]
fn a_hook_declares_the_parameters_its_members_type_names_where_they_are_inferred() {
    let input: syn::DeriveInput = syn::parse_str(
        "pub struct Scoped<IdType, Held, const N: usize> { pub held: Held, pub id: IdType }",
    )
    .unwrap();
    let scope = super::HookScope::of(&input, &super::read_bounds(&input));
    for (ty, named) in [
        ("Vec<Entry<IdType>>", Some(vec!["IdType"])),
        ("HashMap<IdType, Tag>", Some(vec!["IdType"])),
        ("Pair<Held, IdType>", Some(vec!["IdType", "Held"])),
        ("Vec<other::IdType>", Some(vec![])),
        ("Vec<IdType::Out>", None),
        ("Vec<<IdType as Trait>::Out>", None),
        ("Vec<Grid<N>>", None),
        ("[Entry<IdType>; N]", None),
    ] {
        let written: syn::Type = syn::parse_str(ty).unwrap();
        assert_eq!(
            scope.named_by(&written.to_token_stream()),
            named.map(|listed| listed.into_iter().map(str::to_owned).collect::<Vec<_>>()),
            "for {ty}"
        );
    }
}
