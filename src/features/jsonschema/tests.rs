//! Unit tests of the JSON Schema emitter: the `json_schema()` method it generates, and the merge a
//! flattened field earns.

use super::*;

#[test]
fn test_should_generate_json_schema() {
    assert!(should_generate_json_schema());
}

#[test]
fn test_json_schema_method_generation() {
    let fields = vec![];
    let method = generate_struct_json_schema_method(&fields, &[], "Node", &[]);
    let method_str = method.to_string();

    assert!(method_str.contains("json_schema"));
    assert!(method_str.contains("serde_json"));
    assert!(method_str.contains("properties"));
    assert!(method_str.contains("required"));
}

#[test]
fn test_json_schema_method_flatten_emits_merge() {
    let fields = vec![];
    let no_flatten = generate_struct_json_schema_method(&fields, &[], "Node", &[]).to_string();
    let with_flatten = generate_struct_json_schema_method(
        &fields,
        &[MergedSource {
            label: "Base".to_owned(),
            optional: false,
            value: quote::quote! { serde_json::json!({ "type": "object" }) },
        }],
        "Node",
        &[],
    )
    .to_string();

    assert!(!no_flatten.contains("merge_object_schemas"));
    assert!(with_flatten.contains("merge_object_schemas"));
    assert!(with_flatten.contains("oneOf"));
}

#[test]
fn test_the_merge_wraps_branches_in_the_spelling_its_source_used() {
    let merge = generate_struct_json_schema_method(
        &[],
        &[MergedSource {
            label: "Base".to_owned(),
            optional: false,
            value: quote::quote! { serde_json::json!({ "type": "object" }) },
        }],
        "Node",
        &[],
    )
    .to_string();

    assert!(
        merge.contains(
            "let :: core :: option :: Option :: Some ((spelling , branches)) = union_branches \
             (body)"
        ),
        "{merge}"
    );
    assert!(
        merge.contains("Branches :: Union (spelling , expanded)"),
        "{merge}"
    );
    assert!(
        merge.contains("Merged :: Union (spelling , merged)"),
        "{merge}"
    );
    assert!(merge.contains("\"anyOf\""), "{merge}");
}

#[test]
fn test_an_optional_merged_source_carries_its_absence_into_the_merge() {
    let merge_of = |optional| {
        generate_struct_json_schema_method(
            &[],
            &[MergedSource {
                label: "Base".to_owned(),
                optional,
                value: quote::quote! { serde_json::json!({ "type": "object" }) },
            }],
            "Node",
            &[],
        )
        .to_string()
    };

    let optional = merge_of(true);
    assert!(optional.contains("(\"Base\" , true ,"), "{optional}");
    assert!(
        optional.contains("if * optional { source . or_absent () } else { source }"),
        "{optional}"
    );
    assert!(
        optional.contains("Self :: Union (\"anyOf\" , vec ! [self , Self :: Absent])"),
        "{optional}"
    );

    let required = merge_of(false);
    assert!(required.contains("(\"Base\" , false ,"), "{required}");
}

#[test]
fn test_the_merge_expands_branches_to_a_fixed_point_under_a_path_terminator() {
    let merge = generate_struct_json_schema_method(
        &[],
        &[MergedSource {
            label: "Base".to_owned(),
            optional: false,
            value: quote::quote! { serde_json::json!({ "type": "object" }) },
        }],
        "Node",
        &[],
    )
    .to_string();

    assert!(
        merge.contains(
            "None => expanded_branches (branch , hoisted_defs , expanding , position , label)"
        ),
        "{merge}"
    );
    assert!(
        merge.contains("if expanding . contains (& name)"),
        "{merge}"
    );
    assert!(
        merge.contains("closes a flatten cycle through nested unions"),
        "{merge}"
    );
}

#[test]
fn test_the_merge_reads_a_tagged_unit_variant_at_the_edges_own_depth() {
    let merge = generate_struct_json_schema_method(
        &[],
        &[MergedSource {
            label: "Base".to_owned(),
            optional: false,
            value: quote::quote! { serde_json::json!({ "type": "object" }) },
        }],
        "Node",
        &[],
    )
    .to_string();

    assert!(
        merge.contains("(position . len () == 1 && spelling == \"oneOf\")"),
        "{merge}"
    );
    assert!(
        merge.contains(
            ":: core :: option :: Option :: Some (name) => :: core :: option :: Option :: Some \
             (Branches :: Tagged (name))"
        ),
        "{merge}"
    );
    assert!(
        merge.contains("schema . get (\"const\") ? . as_str ()"),
        "{merge}"
    );
}

#[test]
fn test_json_schema_methods_pair_an_entry_point_with_a_guarded_body() {
    let methods = json_schema_methods("Node", &quote::quote! { body }, &[]).to_string();

    assert!(methods.contains("pub fn json_schema ()"), "{methods}");
    assert!(methods.contains("pub fn json_schema_within"), "{methods}");
    assert!(methods.contains("in_flight"), "{methods}");
    assert!(methods.contains("\"$defs\""), "no $defs: {methods}");
}

#[test]
fn test_the_deferred_reference_points_at_the_hoisted_defs_entry() {
    let methods = json_schema_methods("Node", &quote::quote! { body }, &[]).to_string();

    assert!(
        methods.contains("\"#/$defs/\""),
        "no pointer prefix: {methods}"
    );
    assert!(methods.contains("\"Node\""), "not keyed by name: {methods}");
    assert!(
        methods.contains("format ! (\"{}{key}\""),
        "the pointer is not built off the same key the entry is hoisted under: {methods}"
    );
}

#[test]
fn test_plain_enum_publishes_the_guarded_method_too() {
    let method = generate_plain_enum_json_schema_method(&[quote::quote! { "a" }], "Flag", &[]);

    assert!(
        method.to_string().contains("pub fn json_schema_within"),
        "{method}"
    );
}

#[test]
fn test_the_in_flight_recording_carries_the_filling_beside_the_name() {
    let parameters = vec![SchemaParameter {
        binding: proc_macro2::Ident::new("_arg_value_type", proc_macro2::Span::call_site()),
        default: quote::quote! { serde_json::json!({ "type": "string" }) },
    }];
    let methods = json_schema_methods("Node", &quote::quote! { body }, &parameters).to_string();

    assert!(
        methods.contains(
            "in_flight : & mut :: std :: vec :: Vec < (& 'static str , :: std :: vec :: Vec < \
             serde_json :: Value >) >"
        ),
        "{methods}"
    );
    assert!(
        methods.contains(
            "let filling : :: std :: vec :: Vec < serde_json :: Value > = vec ! [_arg_value_type . \
             clone ()]"
        ),
        "{methods}"
    );
    assert!(
        methods.contains("in_flight . push ((\"Node\" , filling))"),
        "{methods}"
    );
}

#[test]
fn test_a_re_entered_name_is_read_against_the_filling_in_flight() {
    let methods = json_schema_methods("Node", &quote::quote! { body }, &[]).to_string();

    assert!(
        methods.contains("in_flight . iter () . find (| (named , _) | * named == \"Node\")"),
        "{methods}"
    );
    assert!(
        methods.contains("if * in_flight_filling != filling"),
        "{methods}"
    );
    assert!(
        methods.contains("a document holds one definition per name"),
        "the refusal does not state the limitation: {methods}"
    );
    assert!(
        methods.contains("key the definitions by name and filling"),
        "the refusal does not state the way past it: {methods}"
    );
}
