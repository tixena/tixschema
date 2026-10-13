//! Unit tests of `check`: the key, its refusals, and the calls it writes.

use proc_macro2::{TokenStream, TokenTree};

use super::{CheckStep, reach};
use crate::model_schema::exec_model_schema;

/// The struct each refusal is raised on, with `prop` written on its one field.
fn parcel(prop: &str) -> String {
    format!("pub struct Parcel {{ {prop} pub item_code_list: Vec<String> }}")
}

/// The whole expansion of `source` under the arguments `args`.
fn expansion(args: &str, source: &str) -> TokenStream {
    exec_model_schema(
        syn::parse_str(args).unwrap(),
        syn::parse_str(source).unwrap(),
    )
}

/// Every refusal `tokens` carries: what it says, and the text of the source it is spanned on.
fn refusals(tokens: TokenStream) -> Vec<(String, Option<String>)> {
    let mut found = Vec::new();
    for tree in tokens {
        match tree {
            TokenTree::Group(group) => found.extend(refusals(group.stream())),
            TokenTree::Literal(literal) => {
                if let syn::Lit::Str(text) = syn::Lit::new(literal.clone())
                    && text.value().starts_with("model_schema: ")
                {
                    found.push((text.value(), literal.span().source_text()));
                }
            }
            TokenTree::Ident(_) | TokenTree::Punct(_) => {}
        }
    }
    found
}

/// The one refusal of `check = {written}` on a type, and of the same on a field.
fn refused(written: &str) -> [(String, Option<String>); 2] {
    let on_the_type = refusals(expansion(&format!("check = {written}"), &parcel("")));
    let on_a_field = refusals(expansion(
        "",
        &parcel(&format!("#[model_schema_prop(check = {written})]")),
    ));
    assert_eq!(on_the_type.len(), 1, "for a type: {on_the_type:?}");
    assert_eq!(on_a_field.len(), 1, "for a field: {on_a_field:?}");
    [on_the_type[0].clone(), on_a_field[0].clone()]
}

/// Every identifier of `tokens` named `name`.
fn idents_named(tokens: TokenStream, name: &str) -> Vec<proc_macro2::Ident> {
    let mut found = Vec::new();
    for tree in tokens {
        match tree {
            TokenTree::Group(group) => found.extend(idents_named(group.stream(), name)),
            TokenTree::Ident(ident) => {
                if ident == name {
                    found.push(ident);
                }
            }
            TokenTree::Literal(_) | TokenTree::Punct(_) => {}
        }
    }
    found
}

#[test]
fn a_check_is_read_on_a_type_and_on_a_field_in_this_build() {
    let tokens = expansion(
        "check = [\"withinParcel\"]",
        &parcel("#[model_schema_prop(check = [\"withoutDuplicates\"])]"),
    );
    assert_eq!(refusals(tokens.clone()), Vec::new(), "got: {tokens}");
    let rendered = tokens.to_string();
    for written in [
        "pub fn validate (& self)",
        "without_duplicates (& self . item_code_list)",
        "within_parcel (self)",
    ] {
        assert!(rendered.contains(written), "no `{written}` in: {rendered}");
    }
}

/// A name is called as the snake case of what is written, at the place it is written: a missing
/// function is then reported at the string literal.
#[test]
fn a_call_is_spanned_on_the_name_as_written() {
    let tokens = expansion(
        "check = [\"withinParcel\"]",
        &parcel("#[model_schema_prop(check = [\"withoutDuplicates\"])]"),
    );
    for (function, written) in [
        ("within_parcel", "\"withinParcel\""),
        ("without_duplicates", "\"withoutDuplicates\""),
    ] {
        let called = idents_named(tokens.clone(), function);
        assert!(!called.is_empty(), "no call of `{function}` in: {tokens}");
        for ident in called {
            assert_eq!(ident.span().source_text().as_deref(), Some(written));
        }
    }
}

#[test]
fn a_check_that_is_no_array_is_refused() {
    for (said, located) in refused("\"withinParcel\"") {
        assert!(
            said.contains("`check` takes an array of string literals"),
            "got: {said}"
        );
        assert_eq!(located.as_deref(), Some("\"withinParcel\""));
    }
    let bare = refusals(expansion("check", &parcel("")));
    assert!(
        matches!(bare.as_slice(), [(said, located)]
            if said.contains("`check` takes an array of string literals")
                && located.as_deref() == Some("check")),
        "got: {bare:?}"
    );
    let on_a_field = refusals(expansion("", &parcel("#[model_schema_prop(check)]")));
    assert!(
        matches!(on_a_field.as_slice(), [(said, located)]
            if said.contains("`check` takes an array of string literals")
                && located.as_deref() == Some("check")),
        "got: {on_a_field:?}"
    );
}

#[test]
fn an_element_that_is_no_string_literal_is_refused() {
    for (said, located) in refused("[withinParcel]") {
        assert!(
            said.contains("each element is a string literal that names a function"),
            "got: {said}"
        );
        assert_eq!(located.as_deref(), Some("withinParcel"));
    }
}

#[test]
fn a_name_that_is_no_identifier_is_refused() {
    for written in ["within-parcel", "7parcels", "", "sin parcel"] {
        for (said, located) in refused(&format!("[\"{written}\"]")) {
            assert!(
                said.contains(
                    "a name is an identifier: ASCII letters, digits and `_`, not led by a digit. \
                     The name is written as a function's name in every language"
                ),
                "got: {said}"
            );
            assert_eq!(located, Some(format!("\"{written}\"")));
        }
    }
}

#[test]
fn a_check_that_names_no_function_is_refused() {
    for (said, located) in refused("[]") {
        assert!(said.contains("`check` names no function"), "got: {said}");
        assert_eq!(located.as_deref(), Some("[]"));
    }
}

#[test]
fn a_name_written_twice_is_refused() {
    for (said, located) in refused("[\"sealed\", \"addressed\", \"sealed\"]") {
        assert!(
            said.contains("names `sealed` twice: the name is written twice"),
            "got: {said}"
        );
        assert_eq!(located.as_deref(), Some("\"sealed\""));
    }
}

/// Each row of the README's table of refusals: what is written, and the words it is refused in.
#[test]
fn the_readme_states_each_refusal_in_the_words_it_is_raised_in() {
    let readme = include_str!("../../../README.md");
    for (written, sentence) in [
        (
            "\"withinParcel\"",
            "takes an array of string literals, each naming a function: `check = \
             [\"withinParcel\"]`",
        ),
        (
            "[withinParcel]",
            "takes string literals: each element is a string literal that names a function",
        ),
        (
            "[\"within-parcel\"]",
            "names `within-parcel`, and a name is an identifier: ASCII letters, digits and `_`, \
             not led by a digit. The name is written as a function's name in every language",
        ),
        (
            "[]",
            "names no function: write one name at least, or drop the key",
        ),
        (
            "[\"a\", \"a\"]",
            "names `a` twice: the name is written twice, and a function runs once",
        ),
    ] {
        for (said, _located) in refused(written) {
            assert!(said.ends_with(sentence), "got: {said}");
        }
        assert!(
            readme.contains(&format!("| `check = {written}` | {sentence} |")),
            "the README no longer shows the refusal of `check = {written}`"
        );
    }
}

/// An alias is another name for its target: it has no `impl` to call a function from.
#[test]
fn a_check_on_a_type_alias_is_refused() {
    let tokens = expansion(
        "check = [\"withinParcel\"]",
        "pub type Parcels = Vec<Parcel>;",
    );
    let found = refusals(tokens.clone());
    assert_eq!(found.len(), 1, "got: {found:?}");
    let (said, located) = &found[0];
    assert_eq!(
        said,
        "model_schema: type `Parcels`: `check` cannot be written on a type alias: `Parcels` is \
         another name for `Vec<Parcel>`, so it has no `impl` of its own to call `withinParcel` \
         from. Write the check on `Vec<Parcel>`, or on a field that holds a `Parcels`, in \
         `#[model_schema_prop(...)]`."
    );
    assert_eq!(located.as_deref(), Some("\"withinParcel\""));
    assert!(
        !tokens.to_string().contains("fn validate"),
        "a refused alias publishes nothing: {tokens}"
    );
}

/// A brand publishes its inner's own schema, so every key written on its slot is refused.
#[test]
fn a_check_on_the_slot_of_a_brand_is_refused() {
    let found = refusals(expansion(
        "",
        "#[serde(transparent)] pub struct Code(#[model_schema_prop(check = [\"upperCase\"])] pub \
         String);",
    ));
    assert_eq!(found.len(), 1, "got: {found:?}");
    let (said, located) = &found[0];
    assert!(
        said.contains(
            "type `Code`: `#[model_schema_prop]` is unread on the slot of a `#[serde(transparent)]` \
             newtype"
        ),
        "got: {said}"
    );
    assert_eq!(
        located.as_deref(),
        Some("[model_schema_prop(check = [\"upperCase\"])]")
    );
}

/// A check is read through an `Option`, an ownership wrapper and a reference, and stops at
/// whatever is under them: a list is the value.
#[test]
fn a_check_takes_the_value_under_an_option_and_a_wrapper() {
    let steps = |written: &str| reach(&syn::parse_str::<syn::Type>(written).unwrap());
    assert_eq!(steps("String"), []);
    assert_eq!(steps("Vec<String>"), []);
    assert_eq!(steps("HashMap<String, Vec<u32>>"), []);
    assert_eq!(steps("(u32, String)"), []);
    assert_eq!(steps("Option<String>"), [CheckStep::Present]);
    assert_eq!(steps("Box<Parcel>"), [CheckStep::Held]);
    assert_eq!(steps("&'a str"), [CheckStep::Held]);
    assert_eq!(steps("Cow<'a, str>"), [CheckStep::Held]);
    assert_eq!(
        steps("Option<Box<Vec<Option<String>>>>"),
        [CheckStep::Present, CheckStep::Held]
    );
    assert_eq!(
        steps("Option<Option<Rc<Parcel>>>"),
        [CheckStep::Present, CheckStep::Present, CheckStep::Held]
    );
}

/// A check is read as a bound is: wherever a bound has a shape, the two walks take the same
/// steps.
#[cfg(feature = "serde")]
#[test]
fn the_reach_of_a_check_is_the_reach_of_a_bound() {
    use crate::model_schema::{ConstraintWrap, constrained_shape};

    for written in [
        "String",
        "Vec<String>",
        "u32",
        "Label",
        "Option<String>",
        "Option<Vec<String>>",
        "Box<String>",
        "Option<Box<Vec<Label>>>",
        "Cow<'a, str>",
        "&'a str",
        "&'a [String]",
        "Arc<Option<Rc<PathBuf>>>",
    ] {
        let ty: syn::Type = syn::parse_str(written).unwrap();
        // A bound stops at a list and reads no map, so its walk holds no step into either.
        let bound: Option<Vec<CheckStep>> = constrained_shape(&ty).and_then(|shape| {
            shape
                .wraps
                .iter()
                .map(|wrap| match wrap {
                    ConstraintWrap::Optional => Some(CheckStep::Present),
                    ConstraintWrap::Borrowed | ConstraintWrap::Transparent => Some(CheckStep::Held),
                    ConstraintWrap::Map | ConstraintWrap::Sequence => None,
                })
                .collect()
        });
        assert_eq!(Some(reach(&ty)), bound, "for `{written}`");
    }
}

/// A check is called by its name alone, so no import of the schema module is in scope of the
/// call: each bound of a type that names a check brings the module in for itself.
#[cfg(all(
    feature = "serde",
    any(feature = "typescript", feature = "zod", feature = "jsonschema")
))]
#[test]
fn a_bound_imports_the_schema_module_for_itself_where_the_type_names_a_check() {
    let bounded = "pub struct Tagged { #[model_schema_prop(minLength = 1, check = \
                   [\"validateLabelValue\"])] pub label: String }";
    let checked = expansion("", bounded).to_string();
    assert!(
        checked.contains(
            "{ use tagged_schema :: * ; if let :: core :: result :: Result :: Err (reported) = \
             validate_label_value (& self . label) { errors . extend (reported) ; } } if let :: \
             core :: result :: Result :: Err (reported) = validate_label_value (& self . label)"
        ),
        "got: {checked}"
    );
    assert!(
        !checked.contains(":: string :: String >> { use tagged_schema :: * ;"),
        "the import reaches the check: {checked}"
    );

    let unchecked = expansion(
        "",
        "pub struct Tagged { #[model_schema_prop(minLength = 1)] pub label: String }",
    )
    .to_string();
    assert!(
        unchecked.contains(
            ":: string :: String >> { use tagged_schema :: * ; let mut errors : :: std :: vec :: \
             Vec < :: std :: string :: String > = :: std :: vec :: Vec :: new () ; if let :: core \
             :: result :: Result :: Err (reported) = validate_label_value (& self . label)"
        ),
        "a type that names no check is written as before: {unchecked}"
    );
}

/// A type that names neither a bound nor a check publishes no `validate()`.
#[test]
fn a_type_with_no_check_and_no_bound_publishes_no_validate() {
    let plain = expansion("", &parcel("")).to_string();
    assert!(!plain.contains("fn validate"), "got: {plain}");
    let checked = expansion("check = [\"withinParcel\"]", &parcel("")).to_string();
    assert_eq!(checked.matches("pub fn validate (& self)").count(), 1);
}

/// The text of one wrap, which copies each issue with `continue: true`.
#[cfg(feature = "zod")]
#[test]
fn the_zod_wrap_hands_each_function_a_payload_of_its_own() {
    use crate::features::model_schema_prop::CheckName;
    use crate::field_type::zod_check_wrap;

    let named = |name: &str| CheckName {
        name: name.to_owned(),
        written_at: proc_macro2::Span::call_site(),
    };
    assert_eq!(zod_check_wrap("z.string()".to_owned(), &[]), "z.string()");
    assert_eq!(
        zod_check_wrap(
            "z.string()".to_owned(),
            &[named("withinParcel"), named("sealed")]
        ),
        "z.string().check((ctx) => {\n  const own = { value: ctx.value, issues: \
         ctx.issues.slice(0, 0) };\n  withinParcel(own.value, own);\n  for (const issue of \
         own.issues) ctx.issues.push({ ...issue, continue: true });\n}).check((ctx) => {\n  const \
         own = { value: ctx.value, issues: ctx.issues.slice(0, 0) };\n  sealed(own.value, own);\n  \
         for (const issue of own.issues) ctx.issues.push({ ...issue, continue: true });\n})"
    );
}

/// A field's check is on the schema of the value the Rust check takes: after the field's bounds,
/// inside the `Option`'s union and inside the `preprocess`, and on the array of a list after its
/// count.
#[cfg(all(feature = "serde", feature = "zod"))]
#[test]
fn a_fields_check_is_written_on_the_schema_of_its_own_value() {
    let wrap = |name: &str| {
        format!(
            ".check((ctx) => {{\n  const own = {{ value: ctx.value, issues: \
             ctx.issues.slice(0, 0) }};\n  {name}(own.value, own);\n  for (const issue of \
             own.issues) ctx.issues.push({{ ...issue, continue: true }});\n}})"
        )
    };
    let zod = expansion(
        "",
        "pub struct Wrapped {
            #[model_schema_prop(minLength = 1, check = [\"shortList\"])]
            pub counted: Vec<String>,
            #[model_schema_prop(maxLength = 9, preprocess = [\"trimmed\"], check = [\"known\"])]
            #[serde(default, skip_serializing_if = \"Option::is_none\")]
            pub spare: Option<String>,
            #[model_schema_prop(nullable, check = [\"known\"])]
            pub written: Option<String>,
        }",
    );
    let emitted = strings(zod).concat();
    for written in [
        format!(
            "  counted: z.array(z.string()).min(1, {{ error: (issue) => `too short: minimum \
             length is 1, got ${{Array.isArray(issue.input) ? issue.input.length : 0}}` }}){},\n",
            wrap("shortList")
        ),
        format!(
            "z.preprocess(trimmed, z.string().max(9, {{ error: (issue) => `too long: maximum \
             length is 9, got ${{String(issue.input).length}}` }}){}), z.undefined()])",
            wrap("known")
        ),
        format!(
            "  written: z.union([z.string(){}, z.null()]),\n",
            wrap("known")
        ),
    ] {
        assert!(emitted.contains(&written), "no `{written}` in: {emitted}");
    }
}

/// The text of every string literal `tokens` holds, in the order written.
#[cfg(all(feature = "serde", feature = "zod"))]
fn strings(tokens: TokenStream) -> Vec<String> {
    let mut found = Vec::new();
    for tree in tokens {
        match tree {
            TokenTree::Group(group) => found.extend(strings(group.stream())),
            TokenTree::Literal(literal) => {
                if let syn::Lit::Str(text) = syn::Lit::new(literal) {
                    found.push(text.value());
                }
            }
            TokenTree::Ident(_) | TokenTree::Punct(_) => {}
        }
    }
    found
}
