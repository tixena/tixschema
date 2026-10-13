//! What the one sentence looks like in each of the spellings it is handed out in.
//!
//! The proof that Rust and Zod *agree* is not here — it takes a declaration, a value, and both
//! emitters, and lives in `tests/language_parity_tests`. What is here is the shape of each
//! rendering, so a change to any is a change somebody wrote down.

use super::Bound;
#[cfg(feature = "dart")]
use super::dart_violation;
#[cfg(all(feature = "serde", feature = "kotlin"))]
use super::kotlin_violation;
#[cfg(feature = "serde")]
use super::rust_violation;
#[cfg(feature = "zod")]
use super::zod_error_arg;

#[cfg(feature = "serde")]
use quote::quote;
#[cfg(all(feature = "serde", feature = "kotlin"))]
use syn::punctuated::Punctuated;
#[cfg(all(feature = "serde", feature = "kotlin"))]
use syn::{Expr, ExprLit, ExprMacro, ExprMethodCall, Lit, Token};

/// The tokens `rust_violation` builds, as one line with the spacing `quote!` prints.
#[cfg(feature = "serde")]
fn rendered(bound: Bound<'_>, field: Option<&str>, observed: &proc_macro2::TokenStream) -> String {
    rust_violation(bound, field, observed).to_string()
}

#[cfg(feature = "serde")]
#[test]
fn a_bound_that_quotes_the_value_back_names_the_field_and_the_measure() {
    assert_eq!(
        rendered(
            Bound::MinLength(3),
            Some("organization_id"),
            &quote! { value.len() }
        ),
        r#"format ! ("'{}': {}, got {}" , "organization_id" , "too short: minimum length is 3" , value . len ())"#
    );
    assert_eq!(
        rendered(Bound::Maximum(120.0), Some("age"), &quote! { value }),
        r#"format ! ("'{}': {}, got {}" , "age" , "too large: maximum is 120" , value)"#
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_bound_that_quotes_nothing_back_states_only_itself() {
    assert_eq!(
        rendered(
            Bound::Pattern("^[a-z]+$"),
            Some("slug"),
            &quote! { value.len() }
        ),
        r#"format ! ("'{}': {}" , "slug" , "does not match pattern '^[a-z]+$'")"#
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_report_with_no_field_to_name_names_none() {
    assert_eq!(
        rendered(Bound::MinLength(3), None, &quote! { value.len() }),
        r#"format ! ("{}, got {}" , "too short: minimum length is 3" , value . len ())"#
    );
    assert_eq!(
        rendered(Bound::Pattern("^/"), None, &quote! { value.len() }),
        r#""does not match pattern '^/'" . to_owned ()"#
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_length_bound_reads_its_measure_off_the_issue() {
    assert_eq!(
        zod_error_arg(Bound::MinLength(3)),
        "{ error: (issue) => `too short: minimum length is 3, got ${String(issue.input).length}` }"
    );
    assert_eq!(
        zod_error_arg(Bound::MaxLength(50)),
        "{ error: (issue) => `too long: maximum length is 50, got ${String(issue.input).length}` }"
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_bound_on_a_list_counts_the_items_the_issue_holds() {
    assert_eq!(
        zod_error_arg(Bound::MinItems(1)),
        "{ error: (issue) => `too short: minimum length is 1, got ${Array.isArray(issue.input) ? issue.input.length : 0}` }"
    );
    assert_eq!(
        zod_error_arg(Bound::MaxItems(3)),
        "{ error: (issue) => `too long: maximum length is 3, got ${Array.isArray(issue.input) ? issue.input.length : 0}` }"
    );
}

#[cfg(feature = "serde")]
#[test]
fn a_bound_on_a_list_is_reported_in_a_strings_words() {
    assert_eq!(
        rendered(Bound::MinItems(1), Some("name_config"), &quote! { held }),
        r#"format ! ("'{}': {}, got {}" , "name_config" , "too short: minimum length is 1" , held)"#
    );
    assert_eq!(
        rendered(Bound::MaxItems(3), None, &quote! { held }),
        r#"format ! ("{}, got {}" , "too long: maximum length is 3" , held)"#
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_numeric_bound_reads_the_value_itself_off_the_issue() {
    assert_eq!(
        zod_error_arg(Bound::Minimum(0.0)),
        "{ error: (issue) => `too small: minimum is 0, got ${String(issue.input)}` }"
    );
    assert_eq!(
        zod_error_arg(Bound::Maximum(120.0)),
        "{ error: (issue) => `too large: maximum is 120, got ${String(issue.input)}` }"
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_pattern_states_itself_and_takes_no_function() {
    assert_eq!(
        zod_error_arg(Bound::Pattern("^[a-z]+$")),
        r#"{ error: "does not match pattern '^[a-z]+$'" }"#
    );
}

#[cfg(feature = "zod")]
#[test]
fn a_pattern_reaches_the_quoted_string_escaped() {
    assert_eq!(
        zod_error_arg(Bound::Pattern(r#"^\d+"x"$"#)),
        r#"{ error: "does not match pattern '^\\d+\"x\"$'" }"#
    );
}

/// The text of a string literal, or `None` for any other expression.
#[cfg(all(feature = "serde", feature = "kotlin"))]
fn literal_text(written: &Expr) -> Option<String> {
    if let Expr::Lit(ExprLit {
        lit: Lit::Str(text),
        attrs: _attrs,
    }) = written
    {
        Some(text.value())
    } else {
        None
    }
}

/// The sentence the `format!` `rust_violation` builds writes at run time, with `<observed>` where
/// it quotes the measured value back.
#[cfg(all(feature = "serde", feature = "kotlin"))]
fn rust_sentence(bound: Bound<'_>, field: Option<&str>) -> String {
    let tokens = rust_violation(bound, field, &quote! { observed });
    let Ok(formatted) = syn::parse2::<ExprMacro>(tokens.clone()) else {
        let owned: ExprMethodCall = syn::parse2(tokens).unwrap();
        return literal_text(&owned.receiver).unwrap();
    };
    let mut written = formatted
        .mac
        .parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated)
        .unwrap()
        .into_iter();
    let template = literal_text(&written.next().unwrap()).unwrap();
    let mut parts = template.split("{}");
    let mut sentence = parts.next().unwrap().to_owned();
    for part in parts {
        let filled = written.next().unwrap();
        sentence.push_str(&literal_text(&filled).unwrap_or_else(|| "<observed>".to_owned()));
        sentence.push_str(part);
    }
    sentence
}

/// The sentence the Kotlin string `kotlin_violation` builds writes at run time, read the same way.
#[cfg(all(feature = "serde", feature = "kotlin"))]
fn kotlin_sentence(bound: Bound<'_>, place: Option<&str>) -> String {
    let literal = kotlin_violation(bound, place, "observed");
    literal
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap()
        .replace("${observed}", "<observed>")
        .replace("\\$", "$")
        .replace("\\\"", "\"")
        .replace("\\\\", "\\")
}

/// Each bound is reported in Kotlin in the sentence Rust reports it in, under a name and under
/// none.
#[cfg(all(feature = "serde", feature = "kotlin"))]
#[test]
fn kotlin_reports_each_bound_in_the_sentence_rust_reports_it_in() {
    let written = [
        Bound::MaxItems(3),
        Bound::MaxLength(50),
        Bound::Maximum(120.0),
        Bound::Maximum(1.5),
        Bound::MinItems(1),
        Bound::MinLength(3),
        Bound::Minimum(0.0),
        Bound::Pattern("^[a-z]+$"),
        Bound::Pattern(r#"^\x41"x"$"#),
    ];
    for bound in written {
        for named in [Some("itemLabel"), None] {
            assert_eq!(kotlin_sentence(bound, named), rust_sentence(bound, named));
        }
    }
    assert_eq!(
        kotlin_violation(Bound::MinLength(1), Some("label"), "label.length"),
        r#""'label': too short: minimum length is 1, got ${label.length}""#
    );
    assert_eq!(
        kotlin_violation(Bound::Pattern("^[a-z]+$"), None, "label.length"),
        r#""does not match pattern '^[a-z]+\$'""#
    );
}

/// The list a reader tells this crate's sentences by, held against the sentences themselves.
#[cfg(feature = "serde")]
#[test]
fn every_sentence_opens_with_a_stem_the_list_spells_and_no_stem_is_unwritten() {
    let written = [
        Bound::MaxItems(3),
        Bound::MaxLength(50),
        Bound::Maximum(120.0),
        Bound::MinItems(1),
        Bound::MinLength(3),
        Bound::Minimum(0.0),
        Bound::Pattern("^[a-z]+$"),
    ];
    for bound in written {
        let stated = bound.stated();
        assert!(
            super::VIOLATION_STEMS
                .iter()
                .any(|stem| stated.starts_with(stem)),
            "no stem in the list opens: {stated}"
        );
    }
    for stem in super::VIOLATION_STEMS {
        assert!(
            written.iter().any(|bound| bound.stated().starts_with(stem)),
            "no bound writes the stem: {stem}"
        );
    }
}

#[cfg(feature = "dart")]
#[test]
fn a_dart_string_names_the_place_and_quotes_the_measure_back() {
    assert_eq!(
        dart_violation(Bound::MinLength(1), Some("label"), "label.runes.length"),
        r#""'label': too short: minimum length is 1, got ${label.runes.length}""#
    );
    assert_eq!(
        dart_violation(Bound::MaxItems(3), Some("codes"), "codes.length"),
        r#""'codes': too long: maximum length is 3, got ${codes.length}""#
    );
    assert_eq!(
        dart_violation(Bound::Maximum(3.0), None, "_number(value)"),
        r#""too large: maximum is 3, got ${_number(value)}""#,
        "a whole bound is stated without `.0`, and a value at no place opens with no name"
    );
}

#[cfg(feature = "dart")]
#[test]
fn a_pattern_reaches_the_dart_string_escaped() {
    assert_eq!(
        dart_violation(Bound::Pattern(r#"^\d+"x"$"#), Some("code"), ""),
        r#""'code': does not match pattern '^\\d+\"x\"\$'""#
    );
}

/// Each bound is stated once, and every rendering holds that one sentence.
#[cfg(all(feature = "serde", feature = "zod", feature = "dart"))]
#[test]
fn the_three_renderings_of_a_bound_hold_one_sentence() {
    let written = [
        Bound::MaxItems(3),
        Bound::MaxLength(50),
        Bound::Maximum(120.0),
        Bound::Maximum(0.5),
        Bound::MinItems(1),
        Bound::MinLength(3),
        Bound::Minimum(0.0),
        Bound::Pattern("^[a-z]+"),
    ];
    for bound in written {
        let stated = bound.stated();
        let rust = rendered(bound, Some("field"), &quote! { held });
        assert!(
            rust.contains(&format!("\"{stated}\"")),
            "Rust does not write `{stated}`: {rust}"
        );
        let zod = zod_error_arg(bound);
        assert!(
            zod.contains(&stated),
            "Zod does not write `{stated}`: {zod}"
        );
        let dart = dart_violation(bound, Some("field"), "held");
        assert!(
            dart.contains(&stated),
            "Dart does not write `{stated}`: {dart}"
        );
    }
}
