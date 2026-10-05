#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
use super::{ADDED_TYPE_NAMES, reading_the_authors_scope};
use super::{module_items, struct_recovering_decode};

/// One of every way a field is walked: read whole, through each kind of hook, item by item, and by
/// another type's walker, under its name or an alias.
const EVERY_WALK: &str = "pub struct Walked { \
     #[serde(alias = \"old\")] pub aliased: Option<Inner>, \
     #[serde(deserialize_with = \"lenient\")] pub read_hooked: Inner, \
     #[serde(deserialize_with = \"parsed\", serialize_with = \"shown\")] pub both_hooked: u16, \
     #[serde(with = \"as_text\")] pub in_module: u32, \
     pub latest: Option<Inner>, pub lists: Vec<Vec<Inner>>, pub owners: HashMap<String, ObjectId>, \
     pub plain: String }";

/// The `impl` the flag adds to `source`, as text.
fn type_impl_of(source: &str) -> String {
    let item: syn::ItemStruct = syn::parse_str(source).unwrap();
    struct_recovering_decode(&item).type_impl.to_string()
}

/// The statements `decode_with_value_fields` runs for `source`, as text.
fn fields_walk_of(source: &str) -> String {
    let emitted = type_impl_of(source);
    let (_, walk) = emitted
        .split_once("pub fn decode_with_value_fields")
        .unwrap();
    walk.to_owned()
}

/// The statements `decode_with_value_issues` runs for `source`, as text, up to the next method.
fn issues_walk_of(source: &str) -> String {
    let emitted = type_impl_of(source);
    let (_, walk) = emitted
        .split_once("pub fn decode_with_value_issues")
        .unwrap();
    walk.split_once("pub fn ")
        .map_or(walk, |(body, _)| body)
        .to_owned()
}

/// The statements `decode_with_bson_fields` runs for `source`, as text.
#[cfg(feature = "bson")]
fn bson_fields_walk_of(source: &str) -> String {
    let emitted = type_impl_of(source);
    let (_, walk) = emitted
        .split_once("pub fn decode_with_bson_fields")
        .unwrap();
    walk.to_owned()
}

/// Everything the flag emits for `source`, as text: what goes into the module, then the `impl`.
fn emission_of(source: &str) -> String {
    let item: syn::ItemStruct = syn::parse_str(source).unwrap();
    let added = struct_recovering_decode(&item);
    format!("{} {}", added.schema_module, added.type_impl)
}

/// The `impl`s the flag adds to `source`: each one's header as text, and its methods in the order
/// written.
fn added_impls(source: &str) -> Vec<(String, Vec<String>)> {
    let item: syn::ItemStruct = syn::parse_str(source).unwrap();
    let added: syn::File = syn::parse2(struct_recovering_decode(&item).type_impl).unwrap();
    let impls: Vec<(String, Vec<String>)> = added
        .items
        .iter()
        .filter_map(|added_item| {
            let syn::Item::Impl(block) = added_item else {
                return None;
            };
            let (generics, self_ty, where_clause) = (
                &block.generics,
                &block.self_ty,
                &block.generics.where_clause,
            );
            let methods: Vec<String> = block
                .items
                .iter()
                .filter_map(|member| {
                    if let syn::ImplItem::Fn(method) = member {
                        Some(method.sig.ident.to_string())
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(methods.len(), block.items.len());
            Some((
                quote::quote!(impl #generics #self_ty #where_clause).to_string(),
                methods,
            ))
        })
        .collect();
    assert_eq!(impls.len(), added.items.len());
    impls
}

/// The methods one source adds to a type that gets a fields walker, and to one that gets none.
fn methods_of(stem: &str, keyed: bool) -> Vec<String> {
    let mut named = vec![
        format!("from_{stem}_with"),
        format!("decode_with_{stem}_report"),
        format!("decode_with_{stem}_issues"),
    ];
    if keyed {
        named.push(format!("decode_with_{stem}_fields"));
    }
    named
}

/// Every token `tokens` is made of, in order, with each group opened and its delimiters left out.
#[cfg(feature = "bson")]
fn leaf_tokens(tokens: proc_macro2::TokenStream, leaves: &mut Vec<String>) {
    for token in tokens {
        if let proc_macro2::TokenTree::Group(group) = token {
            leaf_tokens(group.stream(), leaves);
        } else {
            leaves.push(token.to_string());
        }
    }
}

/// The methods of the `impl Path` the flag puts into a schema module, in the order written.
fn path_methods() -> Vec<String> {
    let added: syn::File = syn::parse2(module_items()).unwrap();
    added
        .items
        .iter()
        .filter_map(|item| {
            if let syn::Item::Impl(block) = item
                && block.trait_.is_none()
                && block.self_ty == syn::parse_quote!(Path)
            {
                Some(&block.items)
            } else {
                None
            }
        })
        .flatten()
        .filter_map(|member| {
            if let syn::ImplItem::Fn(method) = member {
                Some(method.sig.ident.to_string())
            } else {
                None
            }
        })
        .collect()
}

/// Every method the flag adds carries the flag's name, so none can meet one the type's author
/// wrote. Each entry point is the one it is named for.
#[test]
fn every_added_method_but_the_entry_point_carries_the_flags_name() {
    let item: syn::ItemStruct =
        syn::parse_str("pub struct Named { pub title: String, pub versions: Vec<Version> }")
            .unwrap();
    let added: syn::ItemImpl = syn::parse2(struct_recovering_decode(&item).type_impl).unwrap();
    let methods: Vec<String> = added
        .items
        .iter()
        .filter_map(|added_item| {
            if let syn::ImplItem::Fn(method) = added_item {
                Some(method.sig.ident.to_string())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(methods.len(), added.items.len());
    let mut named = vec![
        "from_value_with",
        "decode_with_value_report",
        "decode_with_value_issues",
        "decode_with_value_fields",
    ];
    if cfg!(feature = "bson") {
        named.extend([
            "from_bson_with",
            "decode_with_bson_report",
            "decode_with_bson_issues",
            "decode_with_bson_fields",
        ]);
    }
    assert_eq!(methods, named);
}

/// The BSON entry point and walker are written as their JSON twins are: a named type parameter
/// under a `where` clause, `core::result::Result` in full, and a report only the type calls.
#[cfg(feature = "bson")]
#[test]
fn the_bson_methods_carry_the_signatures_of_their_json_twins() {
    let item: syn::ItemStruct = syn::parse_str("pub struct Named { pub title: String }").unwrap();
    let added: syn::ItemImpl = syn::parse2(struct_recovering_decode(&item).type_impl).unwrap();
    let signatures: Vec<String> = added
        .items
        .iter()
        .filter_map(|added_item| {
            if let syn::ImplItem::Fn(method) = added_item
                && method.sig.ident.to_string().contains("bson")
            {
                let (visibility, signature) = (&method.vis, &method.sig);
                Some(quote::quote!(#visibility #signature).to_string())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        signatures,
        [
            "pub fn from_bson_with < F > (mut document : bson :: Document , decide : F ,) \
             -> core :: result :: Result < Self , named_schema :: Unrecovered < bson :: Bson > > \
             where F : FnOnce (& mut bson :: Document , & [named_schema :: Issue < bson :: Bson >]) \
             -> named_schema :: Verdict ,",
            "fn decode_with_bson_report (document : & bson :: Document) \
             -> Vec < named_schema :: Issue < bson :: Bson > >",
            "pub fn decode_with_bson_issues < I > (found : & bson :: Bson , \
             path : & [core :: result :: Result < String , usize >] , \
             issue : named_schema :: IssueFromParts < bson :: Bson , I > , out : & mut Vec < I > ,)",
            "pub fn decode_with_bson_fields < 'a , I > (object : & 'a bson :: Document , \
             path : & [core :: result :: Result < String , usize >] , \
             issue : named_schema :: IssueFromParts < bson :: Bson , I > , out : & mut Vec < I > ,) \
             -> Vec < & 'a str >",
        ]
    );
}

/// A build without `bson` emits nothing of the BSON read: nothing names the library, and `Path`
/// fixes a JSON value only.
#[cfg(not(feature = "bson"))]
#[test]
fn without_bson_nothing_of_the_bson_read_is_emitted() {
    let emitted = emission_of(EVERY_WALK);
    for absent in ["bson", "same_bracket", "_document"] {
        assert!(!emitted.contains(absent), "found `{absent}` in: {emitted}");
    }
    assert_eq!(path_methods(), ["set_in_value", "remove_from_value"]);
}

/// Both major versions of the `bson` library have `Deserializer::new` and `Serializer::new`, and
/// each names the functions that read a whole document and write a whole value differently. The
/// emission names four items of the library and no other, so it names none of those.
#[cfg(feature = "bson")]
#[test]
fn the_bson_emission_names_only_what_both_major_versions_have() {
    let item: syn::ItemStruct = syn::parse_str(EVERY_WALK).unwrap();
    let added = struct_recovering_decode(&item);
    let mut leaves = Vec::new();
    leaf_tokens(added.schema_module, &mut leaves);
    leaf_tokens(added.type_impl, &mut leaves);
    let mut named: Vec<&str> = leaves
        .windows(4)
        .filter_map(|window| {
            if let [library, first, second, item_named] = window
                && library == "bson"
                && first == ":"
                && second == ":"
            {
                Some(item_named.as_str())
            } else {
                None
            }
        })
        .collect();
    named.sort_unstable();
    named.dedup();
    assert_eq!(named, ["Bson", "Deserializer", "Document", "Serializer"]);
    for one_version_only in [
        "deserialize_from_document",
        "from_document",
        "serialize_to_bson",
        "to_bson",
        "deserialize_from_bson",
        "from_bson",
    ] {
        assert!(
            leaves.iter().all(|leaf| leaf != one_version_only),
            "found `{one_version_only}`"
        );
    }
    let emitted = emission_of(EVERY_WALK);
    for through in [
        "bson :: Deserializer :: new (held . clone ())",
        "bson :: Serializer :: new ()",
        "bson :: Deserializer :: new (bson :: Bson :: Document (document . clone ()))",
    ] {
        assert!(
            emitted.contains(through),
            "missing `{through}` in: {emitted}"
        );
    }
}

/// The BSON items sit where the JSON ones do: the helpers inside the one `impl Path`, and the
/// function a value is read through in the module, once.
#[cfg(feature = "bson")]
#[test]
fn the_bson_items_are_emitted_beside_the_json_ones() {
    assert_eq!(
        path_methods(),
        [
            "set_in_value",
            "remove_from_value",
            "set_in_document",
            "remove_from_document",
        ]
    );
    let added: syn::File = syn::parse2(module_items()).unwrap();
    let functions: Vec<String> = added
        .items
        .iter()
        .filter_map(|item| {
            if let syn::Item::Fn(function) = item {
                Some(function.sig.ident.to_string())
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        functions,
        [
            "expected_from_tokens",
            "issue_from_parts",
            "value_leaf",
            "same_bracket",
            "bson_leaf",
        ]
    );
}

/// The BSON walk is the JSON one over the library's own types: a list, a map and `null` are
/// matched as the members of `bson::Bson` that hold them, and another type is reached by its BSON
/// walker.
#[cfg(feature = "bson")]
#[test]
fn the_bson_walker_matches_the_librarys_own_types() {
    let walk = bson_fields_walk_of(EVERY_WALK);
    for written in [
        "match [\"aliased\" , \"old\"] . into_iter () . find_map (| stored | object . get (stored) . map (| held | (stored , held)))",
        "Some ((stored , bson :: Bson :: Null)) => { }",
        "None | Some (bson :: Bson :: Null) => { }",
        "Some (bson :: Bson :: Array (items)) =>",
        "bson :: Bson :: Array (items_1) =>",
        "Some (bson :: Bson :: Document (entries)) =>",
        "< Inner > :: decode_with_bson_issues (item_1 ,",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
    // Only `mongodb` makes `ObjectId` a type tixschema knows, and so one it reads whole.
    #[cfg(feature = "mongodb")]
    assert!(
        walk.contains(
            "walked_schema :: bson_leaf (item , < ObjectId as serde :: Deserialize > :: deserialize , | read , to | serde :: Serialize :: serialize (read , to) . ok () ,"
        ),
        "got: {walk}"
    );
    assert!(!walk.contains("serde_json"), "got: {walk}");
    assert!(!walk.contains("decode_with_value"), "got: {walk}");
}

/// Without `mongodb`, `ObjectId` is a type tixschema does not know, so the BSON walker reaches it
/// as it reaches any model type: by that type's own BSON walker, never read whole.
#[cfg(all(feature = "bson", not(feature = "mongodb")))]
#[test]
fn without_mongodb_the_bson_walker_reaches_an_id_as_a_model_type() {
    let walk = bson_fields_walk_of(EVERY_WALK);
    assert!(
        walk.contains(
            "for (key , item) in entries { < ObjectId > :: decode_with_bson_issues (item , & [path , & [Ok (\"owners\" . to_owned ()) , Ok (key . clone ())]] . concat () , issue , out) ; }"
        ),
        "got: {walk}"
    );
    assert!(
        !walk.contains("< ObjectId as serde :: Deserialize >"),
        "got: {walk}"
    );
}

/// A hook takes a deserializer and a serializer, not a value, so the BSON walker hands it the
/// library's own: the reader is passed as it is, and the writer is called with the serializer.
#[cfg(feature = "bson")]
#[test]
fn a_hook_is_handed_the_bson_deserializer_and_serializer() {
    let walk = bson_fields_walk_of(EVERY_WALK);
    for written in [
        "walked_schema :: bson_leaf (held , lenient , | read : & Inner , to | serde :: Serialize :: serialize (read , to) . ok () ,",
        "walked_schema :: bson_leaf (held , parsed , | read : & u16 , to | shown (read , to) . ok () ,",
        "walked_schema :: bson_leaf (held , as_text :: deserialize , | read : & u32 , to | as_text :: serialize (read , to) . ok () ,",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
}

/// The key a field is looked up under is the one serde reads: `rename` over `rename_all`, and a raw
/// identifier without its `r#`.
#[test]
fn a_field_is_looked_up_under_the_key_serde_reads() {
    let walk = fields_walk_of(
        "#[serde(rename_all = \"camelCase\")] pub struct Keyed { \
         pub created_at: String, #[serde(rename = \"kind\")] pub flavor: String, pub r#type: String }",
    );
    for key in ["createdAt", "kind", "type"] {
        assert!(
            walk.contains(&format!("match object . get (\"{key}\")")),
            "for {key}, got: {walk}"
        );
    }
    assert!(
        walk.ends_with("vec ! [\"createdAt\" , \"kind\" , \"type\"] } }"),
        "got: {walk}"
    );
}

/// A struct's own `tag` is a key serde writes into the object and reads past, so it is declared.
#[test]
fn a_structs_own_tag_is_a_declared_key() {
    let walk = fields_walk_of("#[serde(tag = \"kind\")] pub struct Tagged { pub name: String }");
    assert!(
        walk.ends_with("vec ! [\"name\" , \"kind\"] } }"),
        "got: {walk}"
    );
}

/// A type with no field to walk takes the `Vec` every walker is handed and binds nothing of it:
/// no statement is written to give the parameter a use.
#[test]
fn a_type_with_no_field_to_walk_leaves_the_issue_list_unbound() {
    for source in [
        "pub struct Blank {}",
        "pub struct Unwritten { #[serde(skip)] pub cached: u8 }",
        "pub struct Derived { #[serde(skip_deserializing)] pub total: u32 }",
    ] {
        let walk = fields_walk_of(source);
        assert!(
            walk.contains(", _ : & mut Vec < I > ,) -> Vec < & 'a str > { vec ! ["),
            "for {source}, got: {walk}"
        );
    }
    let walked = fields_walk_of("pub struct Named { pub title: String }");
    assert!(
        walked.contains(
            ", out : & mut Vec < I > ,) -> Vec < & 'a str > { match object . get (\"title\")"
        ),
        "got: {walked}"
    );
}

/// The BSON walker binds what the JSON one binds: a parameter it does not read is `_`.
#[cfg(feature = "bson")]
#[test]
fn a_bson_walker_with_no_field_to_walk_leaves_the_issue_list_unbound() {
    let unwalked = bson_fields_walk_of("pub struct Blank {}");
    assert!(
        unwalked.starts_with(
            " < 'a , I > (object : & 'a bson :: Document , path : & [core :: result :: Result < String , usize >] , \
             issue : blank_schema :: IssueFromParts < bson :: Bson , I > , _ : & mut Vec < I > ,) \
             -> Vec < & 'a str > { vec ! []"
        ),
        "got: {unwalked}"
    );
    let walked = bson_fields_walk_of("pub struct Named { pub title: String }");
    assert!(
        walked.contains(
            ", out : & mut Vec < I > ,) -> Vec < & 'a str > { match object . get (\"title\")"
        ),
        "got: {walked}"
    );
}

/// The read hook tixschema hangs to name a field in its type's refusal changes a message and no
/// verdict: the field is still walked by its type's own walker. The author's hook is read through.
#[test]
fn a_generated_named_hook_is_read_past_and_an_authors_hook_is_read_through() {
    let generated = fields_walk_of(
        "pub struct Holder { \
         #[serde(deserialize_with = \"holder_schema::deserialize_named_inner\")] pub inner: Inner }",
    );
    assert!(
        generated.contains("< Inner > :: decode_with_value_issues (held ,"),
        "got: {generated}"
    );
    let authored = fields_walk_of(
        "pub struct Holder { #[serde(deserialize_with = \"lenient\")] pub inner: Inner }",
    );
    assert!(
        authored.contains(
            "holder_schema :: value_leaf (held , lenient , | read : & Inner | serde_json :: to_value (read) . ok () ,"
        ),
        "got: {authored}"
    );
}

/// Only a bare use of an added name is written through `super`: one already under a path names
/// what its path says, and every other identifier is left alone.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn only_a_bare_added_name_is_read_from_the_authors_scope() {
    let scoped = reading_the_authors_scope(quote::quote! {
        fn hook(held: Vec<Issue>, at: &Path, kept: crate::Issue, other: Version) -> Segment {
            Issue::default()
        }
    })
    .to_string();
    assert_eq!(
        scoped,
        quote::quote! {
            fn hook(
                held: Vec<super::Issue>,
                at: &super::Path,
                kept: crate::Issue,
                other: Version
            ) -> super::Segment {
                super::Issue::default()
            }
        }
        .to_string()
    );
}

/// The names read past are the types the flag adds, all of them: one left out would be taken for
/// the author's type of that name by whatever the module already held.
#[cfg(any(feature = "typescript", feature = "zod", feature = "jsonschema"))]
#[test]
fn every_type_the_flag_adds_is_a_name_read_past() {
    let added: syn::File = syn::parse2(module_items()).unwrap();
    let mut declared: Vec<String> = added
        .items
        .iter()
        .filter_map(|item| {
            if let syn::Item::Enum(declared_enum) = item {
                Some(declared_enum.ident.to_string())
            } else if let syn::Item::Struct(declared_struct) = item {
                Some(declared_struct.ident.to_string())
            } else if let syn::Item::Type(declared_alias) = item {
                Some(declared_alias.ident.to_string())
            } else {
                None
            }
        })
        .collect();
    declared.sort();
    assert_eq!(declared, ADDED_TYPE_NAMES);
}

/// Each struct shape gets the methods serde's form of it has a use for, each named for the flag,
/// in one `impl`. A type serde cannot flatten gets no fields walker.
#[test]
fn each_struct_shape_adds_the_methods_its_form_has_a_use_for() {
    for (source, keyed) in [
        ("pub struct Pair(pub String, pub u32);", false),
        ("pub struct Brand(pub String);", false),
        ("pub struct Tags(pub Vec<Inner>);", false),
        ("pub struct Pinned(pub Inner);", true),
        ("pub struct Ping;", true),
    ] {
        let mut named = methods_of("value", keyed);
        if cfg!(feature = "bson") {
            named.extend(methods_of("bson", keyed));
        }
        let header = format!("impl {}", source.split([' ', '(', ';']).nth(2).unwrap());
        assert_eq!(added_impls(source), [(header, named)], "for {source}");
    }
}

/// A tuple struct is the array serde writes: each slot at its position, an absent one `Missing`,
/// and every position past the last slot `Unknown`.
#[test]
fn a_tuple_struct_is_walked_by_position() {
    let walk = issues_walk_of("pub struct Pair(pub String, pub Inner);");
    for written in [
        "let Some (items) = found . as_array () else { out . extend (pair_schema :: value_leaf (found , < Self as serde :: Deserialize > :: deserialize , | read | serde_json :: to_value (read) . ok () , path . to_vec () , & [(\"Model\" , & [\"Pair\"] , 0)] , issue)) ; return ; } ;",
        "match items . first () { Some (held) => out . extend (pair_schema :: value_leaf (held , < String as serde :: Deserialize > :: deserialize ,",
        "None => out . push (issue (\"Missing\" , [path , & [Err (0)]] . concat () , & [(\"String\" , & [] , 0)] , None , None , Vec :: new ())) ,",
        "match items . get (1) { Some (held) => < Inner > :: decode_with_value_issues (held , & [path , & [Err (1)]] . concat () , issue , out) ,",
        "for (index , held) in items . iter () . enumerate () . skip (2) { out . push (issue (\"Unknown\" , [path , & [Err (index)]] . concat () , & [] , Some (held . clone ()) , None , Vec :: new ())) ; }",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
}

/// The array holds no position for a slot serde does not read, and serde reads a `default` slot
/// when its position is absent. With no slot at all, no position is skipped past.
#[test]
fn a_slot_is_a_position_only_where_serde_reads_one() {
    let walk = issues_walk_of(
        "pub struct Loose(pub String, #[serde(skip)] pub u8, #[serde(default)] pub u32);",
    );
    assert!(
        walk.contains("if let Some (held) = items . get (1) { out . extend (loose_schema :: value_leaf (held , < u32 as serde :: Deserialize > :: deserialize ,"),
        "got: {walk}"
    );
    assert!(
        walk.contains("in items . iter () . enumerate () . skip (2)"),
        "got: {walk}"
    );
    assert!(!walk.contains("u8"), "got: {walk}");

    let empty = issues_walk_of("pub struct Empty();");
    assert!(
        empty.contains("for (index , held) in items . iter () . enumerate () {"),
        "got: {empty}"
    );
    assert!(!empty.contains("skip"), "got: {empty}");
}

/// A single-slot tuple struct is read with its own reader, which runs whatever hook its slot
/// carries, and the issue names what the slot holds. The hook itself is never named.
#[test]
fn a_single_slot_struct_is_read_with_the_types_own_reader() {
    for source in [
        "pub struct Code(pub String);",
        "#[serde(transparent)] pub struct Code(pub String);",
        "pub struct Code(#[serde(deserialize_with = \"code_schema::deserialize_value\")] pub String);",
        "pub struct Code(#[serde(with = \"as_text\")] pub Vec<Inner>);",
    ] {
        let walk = issues_walk_of(source);
        assert!(
            walk.contains(
                "{ out . extend (code_schema :: value_leaf (found , < Self as serde :: Deserialize > :: deserialize , | read | serde_json :: to_value (read) . ok () , path . to_vec () , & ["
            ),
            "for {source}, got: {walk}"
        );
        assert!(
            !walk.contains("deserialize_value") && !walk.contains("as_text"),
            "for {source}, got: {walk}"
        );
    }
    let plain = issues_walk_of("pub struct Code(pub String);");
    assert!(
        plain.contains("path . to_vec () , & [(\"String\" , & [] , 0)] , issue)) ; }"),
        "got: {plain}"
    );
}

/// Over a flagged model type, a single-slot tuple struct hands both walks to that type at the
/// path it sits at, through any wrapper serde writes as the value it holds.
#[test]
fn a_single_slot_struct_over_a_model_type_hands_both_walks_to_it() {
    for source in [
        "pub struct Pinned(pub Inner);",
        "pub struct Pinned(pub Box<Inner>);",
    ] {
        let emitted = type_impl_of(source);
        for written in [
            "{ < Inner > :: decode_with_value_issues (found , path , issue , out) ; }",
            "-> Vec < & 'a str > { < Inner > :: decode_with_value_fields (object , path , issue , out) }",
        ] {
            assert!(
                emitted.contains(written),
                "for {source}, missing `{written}` in: {emitted}"
            );
        }
    }
}

/// Over a list or an `Option`, what a single-slot tuple struct holds is walked as a field of that
/// type is, at the path the struct itself sits at.
#[test]
fn a_single_slot_struct_over_a_list_or_an_option_walks_what_it_holds() {
    let listed = issues_walk_of("pub struct Tags(pub Vec<Inner>);");
    for written in [
        "match found { serde_json :: Value :: Array (items) => { for (index , item) in items . iter () . enumerate () { < Inner > :: decode_with_value_issues (item , & [path , & [Err (index)]] . concat () , issue , out) ; } } ,",
        "found => out . push (issue (\"Invalid\" , path . to_vec () , & [(\"Array\" , & [] , 1) , (\"Model\" , & [\"Inner\"] , 0)] , Some (found . clone ()) , Some (\"not an array\" . to_owned ()) , Vec :: new ())) ,",
    ] {
        assert!(listed.contains(written), "missing `{written}` in: {listed}");
    }
    let optional = issues_walk_of("pub struct Latest(pub Option<Inner>);");
    assert!(
        optional.contains(
            "match found { serde_json :: Value :: Null => { } , found => < Inner > :: decode_with_value_issues (found , path , issue , out) , }"
        ),
        "got: {optional}"
    );
}

/// A unit struct is the `{}` tixschema makes it write: every key is `Unknown`, and the fields
/// walker reads none of what it is handed, so it binds none of it.
#[test]
fn a_unit_struct_lists_every_key_and_its_fields_walker_binds_nothing() {
    let emitted = type_impl_of("pub struct Ping;");
    for written in [
        "let Some (object) = found . as_object () else { out . extend (ping_schema :: value_leaf (found , < Self as serde :: Deserialize > :: deserialize ,",
        "for (key , held) in object { out . push (issue (\"Unknown\" , [path , & [Ok (key . clone ())]] . concat () , & [] , Some (held . clone ()) , None , Vec :: new ())) ; }",
        "pub fn decode_with_value_fields < 'a , I > (_ : & 'a serde_json :: Map < String , serde_json :: Value > , \
         _ : & [core :: result :: Result < String , usize >] , \
         _ : ping_schema :: IssueFromParts < serde_json :: Value , I > , _ : & mut Vec < I > ,) \
         -> Vec < & 'a str > { Vec :: new () }",
    ] {
        assert!(
            emitted.contains(written),
            "missing `{written}` in: {emitted}"
        );
    }
}

/// A tuple is one more step of the walk, so it is walked by position under a list, a map and an
/// `Option`, each level binding names of its own. `()` is no tuple to serde, which writes `null`.
#[test]
fn a_tuple_in_a_fields_type_is_walked_by_position_wherever_it_is_held() {
    let walk = fields_walk_of(
        "pub struct Placed { pub by_name: HashMap<String, (String, u32)>, \
         pub maybe: Option<(String, u32)>, pub nothing: (), pub spot: (String, Inner), \
         pub spots: Vec<(String, u32)> }",
    );
    for written in [
        "for (key , item) in entries { match item { serde_json :: Value :: Array (items_1) => { match items_1 . first () { Some (held_1) =>",
        "[path , & [Ok (\"by_name\" . to_owned ()) , Ok (key . clone ()) , Err (1)]] . concat ()",
        "match object . get (\"maybe\") { None | Some (serde_json :: Value :: Null) => { } Some (serde_json :: Value :: Array (items)) => { match items . first () {",
        "Some (held) => out . push (issue (\"Invalid\" , [path , & [Ok (\"maybe\" . to_owned ())]] . concat () , & [(\"Optional\" , & [] , 1) , (\"Tuple\" , & [] , 2) , (\"String\" , & [] , 0) , (\"U32\" , & [] , 0)] , Some (held . clone ()) , Some (\"not an array\" . to_owned ()) , Vec :: new ())) ,",
        "placed_schema :: value_leaf (held , < () as serde :: Deserialize > :: deserialize ,",
        "match items . get (1) { Some (held) => < Inner > :: decode_with_value_issues (held , & [path , & [Ok (\"spot\" . to_owned ()) , Err (1)]] . concat () , issue , out) ,",
        "None => out . push (issue (\"Missing\" , [path , & [Ok (\"spot\" . to_owned ())]] . concat () , & [(\"Tuple\" , & [] , 2) , (\"String\" , & [] , 0) , (\"Model\" , & [\"Inner\"] , 0)] , None , None , Vec :: new ())) ,",
        "for (index , item) in items . iter () . enumerate () { match item { serde_json :: Value :: Array (items_1) =>",
        "for (index_1 , held_1) in items_1 . iter () . enumerate () . skip (2) { out . push (issue (\"Unknown\" , [path , & [Ok (\"spots\" . to_owned ()) , Err (index) , Err (index_1)]] . concat () ,",
        "item => out . push (issue (\"Invalid\" , [path , & [Ok (\"spots\" . to_owned ()) , Err (index)]] . concat () , & [(\"Tuple\" , & [] , 2) , (\"String\" , & [] , 0) , (\"U32\" , & [] , 0)] , Some (item . clone ()) , Some (\"not an array\" . to_owned ()) , Vec :: new ())) ,",
    ] {
        assert!(walk.contains(written), "missing `{written}` in: {walk}");
    }
}

/// A type with a type parameter gets one `impl` per source. Each joins what that source reads and
/// writes a value with to the bounds the type declares, on every parameter and on the type itself.
/// A parameter is bounded in one place: its `where` predicate where the type wrote one, taking
/// along what the type wrote beside its name, and beside its name otherwise.
#[test]
fn a_generic_type_gets_one_impl_per_source_under_that_sources_bounds() {
    let added = added_impls(
        "pub struct Page<T: Clone, const N: usize, U = String, V: Send, W> \
         where U: Copy, V: Sync, Vec<W>: Clone { pub items: Vec<T>, pub other: (U, V, W) }",
    );
    let mut headers = vec![(
        "impl < T : Clone + serde :: de :: DeserializeOwned , const N : usize , U , V , \
         W : serde :: de :: DeserializeOwned > Page < T , N , U , V , W > \
         where U : Copy + serde :: de :: DeserializeOwned , \
         V : Sync + Send + serde :: de :: DeserializeOwned , Vec < W > : Clone , \
         Self : serde :: de :: DeserializeOwned"
            .to_owned(),
        methods_of("value", true),
    )];
    if cfg!(feature = "bson") {
        headers.push((
            "impl < T : Clone + serde :: de :: DeserializeOwned + serde :: Serialize , \
             const N : usize , U , V , \
             W : serde :: de :: DeserializeOwned + serde :: Serialize > \
             Page < T , N , U , V , W > \
             where U : Copy + serde :: de :: DeserializeOwned + serde :: Serialize , \
             V : Sync + Send + serde :: de :: DeserializeOwned + serde :: Serialize , \
             Vec < W > : Clone , Self : serde :: de :: DeserializeOwned + serde :: Serialize"
                .to_owned(),
            methods_of("bson", true),
        ));
    }
    assert_eq!(added, headers);
}

/// A method's own type parameter cannot be named as one the type declares, so each takes the next
/// free name where the type declares `F` or `I`, and keeps its own everywhere else.
#[test]
fn a_methods_own_type_parameter_is_never_one_the_type_declares() {
    let clashing =
        type_impl_of("pub struct Keyed<I, const F: usize, F2> { pub id: I, pub kept: F2 }");
    for written in [
        "pub fn from_value_with < F3 > (mut value : serde_json :: Value , decide : F3 ,)",
        "where F3 : FnOnce (& mut serde_json :: Value , & [keyed_schema :: Issue < serde_json :: Value >]) -> keyed_schema :: Verdict ,",
        "pub fn decode_with_value_issues < I2 > (found : & serde_json :: Value , \
         path : & [core :: result :: Result < String , usize >] , \
         issue : keyed_schema :: IssueFromParts < serde_json :: Value , I2 > , out : & mut Vec < I2 > ,)",
        "pub fn decode_with_value_fields < 'a , I2 > (",
        "< I as serde :: Deserialize > :: deserialize , | _ | None ,",
    ] {
        assert!(
            clashing.contains(written),
            "missing `{written}` in: {clashing}"
        );
    }
    let plain = type_impl_of("pub struct Page<T> { pub items: Vec<T> }");
    for written in [
        "pub fn from_value_with < F > (",
        "pub fn decode_with_value_issues < I > (",
        "pub fn decode_with_value_fields < 'a , I > (",
    ] {
        assert!(plain.contains(written), "missing `{written}` in: {plain}");
    }
}

/// Only a type parameter asks for a bound: with none, one `impl` under the type's own generics
/// holds every method, and its values are written back as any other type's are.
#[test]
fn a_type_with_no_type_parameter_keeps_one_impl_under_its_own_generics() {
    let source = "pub struct Grid<const N: usize> { pub cells: Vec<u8> }";
    let headers: Vec<String> = added_impls(source)
        .into_iter()
        .map(|(header, _methods)| header)
        .collect();
    assert_eq!(headers, ["impl < const N : usize > Grid < N >"]);
    assert!(
        type_impl_of(source).contains(
            "< Self as serde :: Deserialize > :: deserialize , | read | serde_json :: to_value (read) . ok () ,"
        )
    );
}

/// The JSON walker of a generic type is bound to read a parameter's value and the type itself,
/// never to write them: each is read whole and nothing is written back to compare. What holds a
/// parameter's value is read whole with it, and every other value keeps its write-back.
#[test]
fn a_parameters_value_and_the_type_itself_are_never_written_back_from_json() {
    let source = "pub struct Parcel<T> { pub both: (T, u32), pub extra: HashMap<String, T>, \
                  #[serde(deserialize_with = \"lenient\")] pub hooked: T, pub inner: Vec<Self>, \
                  pub items: Vec<T>, pub one: T, pub page: Page<T>, pub projected: T::Item, \
                  pub sender: Option<T>, pub total: u32 }";
    let emitted = type_impl_of(source);
    let (json, _bson) = emitted
        .split_once("pub fn from_bson_with")
        .unwrap_or((emitted.as_str(), ""));
    for written in [
        "parcel_schema :: value_leaf (found , < Self as serde :: Deserialize > :: deserialize , | _ | None , path . to_vec () , & [(\"Model\" , & [\"Parcel\"] , 0)] , issue)",
        "< (T , u32) as serde :: Deserialize > :: deserialize , | _ | None , [path , & [Ok (\"both\" . to_owned ())]] . concat () , & [(\"Tuple\" , & [] , 2) , (\"TypeParam\" , & [\"T\"] , 0) , (\"U32\" , & [] , 0)] ,",
        "< HashMap < String , T > as serde :: Deserialize > :: deserialize , | _ | None , [path , & [Ok (\"extra\" . to_owned ())]] . concat () , & [(\"Map\" , & [] , 1) , (\"TypeParam\" , & [\"T\"] , 0)] ,",
        "parcel_schema :: value_leaf (held , lenient , | _ : & T | None ,",
        "< Self > :: decode_with_value_issues (item , & [path , & [Ok (\"inner\" . to_owned ()) , Err (index)]] . concat () , issue , out)",
        "< Vec < T > as serde :: Deserialize > :: deserialize , | _ | None , [path , & [Ok (\"items\" . to_owned ())]] . concat () , & [(\"Array\" , & [] , 1) , (\"TypeParam\" , & [\"T\"] , 0)] ,",
        "< T as serde :: Deserialize > :: deserialize , | _ | None , [path , & [Ok (\"one\" . to_owned ())]] . concat () , & [(\"TypeParam\" , & [\"T\"] , 0)] ,",
        "< Page < T > > :: decode_with_value_issues (held , & [path , & [Ok (\"page\" . to_owned ())]] . concat () , issue , out)",
        "< T :: Item as serde :: Deserialize > :: deserialize , | _ | None ,",
        "< Option < T > as serde :: Deserialize > :: deserialize , | _ | None , [path , & [Ok (\"sender\" . to_owned ())]] . concat () , & [(\"Optional\" , & [] , 1) , (\"TypeParam\" , & [\"T\"] , 0)] ,",
        "< u32 as serde :: Deserialize > :: deserialize , | read | serde_json :: to_value (read) . ok () ,",
    ] {
        assert!(json.contains(written), "missing `{written}` in: {json}");
    }
    assert_eq!(
        json.matches("serde_json :: to_value").count(),
        1,
        "got: {json}"
    );
}

/// The BSON walker of a generic type is bound `Serialize` throughout, so it writes a parameter's
/// value and the type itself back as it writes any other.
#[cfg(feature = "bson")]
#[test]
fn the_bson_walker_of_a_generic_type_writes_every_value_back() {
    let emitted = type_impl_of("pub struct Page<T> { pub items: Vec<T>, pub total: u32 }");
    let (_json, bson) = emitted.split_once("pub fn from_bson_with").unwrap();
    for written in [
        "page_schema :: bson_leaf (found , < Self as serde :: Deserialize > :: deserialize , | read , to | serde :: Serialize :: serialize (read , to) . ok () ,",
        "page_schema :: bson_leaf (held , < Vec < T > as serde :: Deserialize > :: deserialize , | read , to | serde :: Serialize :: serialize (read , to) . ok () ,",
    ] {
        assert!(bson.contains(written), "missing `{written}` in: {bson}");
    }
    assert!(!bson.contains("| _ | None"), "got: {bson}");
}

/// The BSON walk of each struct shape is its JSON one over the library's own types.
#[cfg(feature = "bson")]
#[test]
fn the_bson_walker_of_each_struct_shape_matches_the_librarys_own_types() {
    for (source, written) in [
        (
            "pub struct Pair(pub String, pub u32);",
            "let bson :: Bson :: Array (items) = found else { out . extend (pair_schema :: bson_leaf (found , < Self as serde :: Deserialize > :: deserialize , | read , to | serde :: Serialize :: serialize (read , to) . ok () , path . to_vec () , & [(\"Model\" , & [\"Pair\"] , 0)] , issue)) ; return ; } ;",
        ),
        (
            "pub struct Code(pub String);",
            "{ out . extend (code_schema :: bson_leaf (found , < Self as serde :: Deserialize > :: deserialize , | read , to | serde :: Serialize :: serialize (read , to) . ok () , path . to_vec () , & [(\"String\" , & [] , 0)] , issue)) ; }",
        ),
        (
            "pub struct Pinned(pub Inner);",
            "-> Vec < & 'a str > { < Inner > :: decode_with_bson_fields (object , path , issue , out) }",
        ),
        (
            "pub struct Ping;",
            "pub fn decode_with_bson_fields < 'a , I > (_ : & 'a bson :: Document , _ : & [core :: result :: Result < String , usize >] , _ : ping_schema :: IssueFromParts < bson :: Bson , I > , _ : & mut Vec < I > ,) -> Vec < & 'a str > { Vec :: new () }",
        ),
        (
            "pub struct Placed { pub spot: (String, u32) }",
            "Some (bson :: Bson :: Array (items)) => { match items . first () { Some (held) => out . extend (placed_schema :: bson_leaf (held ,",
        ),
    ] {
        let emitted = type_impl_of(source);
        let (_json, bson) = emitted.split_once("pub fn from_bson_with").unwrap();
        assert!(
            bson.contains(written),
            "for {source}, missing `{written}` in: {bson}"
        );
        assert!(!bson.contains("serde_json"), "for {source}, got: {bson}");
    }
}
