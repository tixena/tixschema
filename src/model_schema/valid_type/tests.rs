//! Unit tests of the value `new` writes for a field a `literal` fixes.

use syn::parse::Parser as _;

use super::literal_fill;

fn fill(field: &str) -> Option<String> {
    literal_fill(&syn::Field::parse_named.parse_str(field).unwrap())
}

#[test]
fn a_pointer_over_text_converts_the_text_it_holds() {
    for held in ["Box<str>", "Rc<str>", "Arc<str>"] {
        assert_eq!(
            fill(&format!(
                "#[model_schema_prop(literal = \"a\")] pub f: {held}"
            ))
            .as_deref(),
            Some("::core::convert::Into::into(\"a\")"),
            "{held}"
        );
    }
}

#[test]
fn a_pointer_over_a_path_converts_a_buffer() {
    assert_eq!(
        fill("#[model_schema_prop(literal = \"a\")] pub f: Box<Path>").as_deref(),
        Some("::core::convert::Into::into(::std::path::PathBuf::from(\"a\"))")
    );
}

#[test]
fn a_pointer_over_an_owned_value_converts_a_value_of_its_own_type() {
    assert_eq!(
        fill("#[model_schema_prop(literal = \"a\")] pub f: Arc<String>").as_deref(),
        Some("::core::convert::Into::into(::std::string::String::from(\"a\"))")
    );
    assert_eq!(
        fill("#[model_schema_prop(literal = \"a\")] pub f: Arc<PathBuf>").as_deref(),
        Some("::core::convert::Into::into(::std::path::PathBuf::from(\"a\"))")
    );
    assert_eq!(
        fill("#[model_schema_prop(literal = 3)] pub f: Rc<u8>").as_deref(),
        Some("::core::convert::Into::into(3u8)")
    );
}

#[test]
fn a_cow_is_borrowed_over_text_and_owned_over_a_value() {
    assert_eq!(
        fill("#[model_schema_prop(literal = \"a\")] pub f: Cow<'static, str>").as_deref(),
        Some("::std::borrow::Cow::Borrowed(\"a\")")
    );
    assert_eq!(
        fill("#[model_schema_prop(literal = \"a\")] pub f: Cow<'static, String>").as_deref(),
        Some("::std::borrow::Cow::Owned(::std::string::String::from(\"a\"))")
    );
    assert_eq!(
        fill("#[model_schema_prop(literal = true)] pub f: Cow<'static, bool>").as_deref(),
        Some("::std::borrow::Cow::Owned(true)")
    );
}

#[test]
fn a_static_reference_is_the_literal_or_a_reference_to_it() {
    assert_eq!(
        fill("#[model_schema_prop(literal = \"a\")] pub f: &'static str").as_deref(),
        Some("\"a\"")
    );
    assert_eq!(
        fill("#[model_schema_prop(literal = 3)] pub f: &'static u16").as_deref(),
        Some("&3u16")
    );
    assert_eq!(
        fill("#[model_schema_prop(literal = true)] pub f: &'static bool").as_deref(),
        Some("&true")
    );
}

#[test]
fn no_one_value_fills_an_optional_a_list_or_what_a_reference_cannot_hold() {
    for held in [
        "Option<String>",
        "Option<Box<str>>",
        "Vec<String>",
        "Box<Box<str>>",
        "Cow<'static, Path>",
        "&'static String",
        "&'static mut str",
        "&str",
    ] {
        assert_eq!(
            fill(&format!(
                "#[model_schema_prop(literal = \"a\")] pub f: {held}"
            )),
            None,
            "{held}"
        );
    }
}
