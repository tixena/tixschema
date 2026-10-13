//! Each rule serde's derive bounds a read by, held to what the derive writes for the same item.

use quote::ToTokens as _;
use syn::DeriveInput;

use super::{ReadBounds, read_bounds};

const BUILDS: &str = "T : :: core :: default :: Default";
const READS: &str = "T : serde :: Deserialize < 'de >";

fn bounds_of(source: &str) -> ReadBounds {
    read_bounds(&syn::parse_str::<DeriveInput>(source).unwrap())
}

/// The predicates the read of `source` is written under, as text.
fn asked_of(source: &str) -> Vec<String> {
    bounds_of(source)
        .predicates
        .iter()
        .map(|predicate| predicate.to_token_stream().to_string())
        .collect()
}

/// The lifetimes `'de` outlives in the read of `source`, or `None` where it is read at `'static`.
fn borrowed_for(source: &str) -> Option<Vec<String>> {
    bounds_of(source)
        .borrowed
        .map(|lifetimes| lifetimes.iter().map(ToString::to_string).collect())
}

#[test]
fn a_parameter_a_read_field_names_is_asked_to_read_once_in_the_order_declared() {
    for (source, asked) in [
        ("struct Page<T> { items: Vec<T> }", vec![READS]),
        (
            "struct Pair<U, T> { first: (T, U), second: Option<[T; 2]>, third: Box<U> }",
            vec!["U : serde :: Deserialize < 'de >", READS],
        ),
        ("struct Slots<T>(u8, T);", vec![READS]),
        ("enum Answer<T> { Empty, Value(T) }", vec![READS]),
        ("enum Named<T> { Held { value: T } }", vec![READS]),
        ("struct Plain { name: String }", Vec::new()),
        (
            "struct Counted<const N: usize> { slots: [u8; N] }",
            Vec::new(),
        ),
        // A path that only starts at a parameter asks the type it names, and not the parameter.
        (
            "struct Each<T: Iterator> { item: T::Item }",
            vec!["T :: Item : serde :: Deserialize < 'de >"],
        ),
    ] {
        assert_eq!(asked_of(source), asked, "for {source}");
        assert!(!bounds_of(source).written_by_container, "for {source}");
    }
}

#[test]
fn a_field_serde_does_not_read_with_its_own_deserialize_asks_nothing_to_read() {
    for (source, asked) in [
        ("struct Marked<T> { held: PhantomData<T> }", Vec::new()),
        (
            "struct Marked<T> { held: core::marker::PhantomData<T> }",
            Vec::new(),
        ),
        (
            "struct Stamped<T> { #[serde(deserialize_with = \"stamp\")] at: T }",
            Vec::new(),
        ),
        (
            "struct Stamped<T> { #[serde(with = \"stamps\")] at: T }",
            Vec::new(),
        ),
        // A skipped field is built, with `Default` where nothing names a function for it.
        (
            "struct Cached<T> { #[serde(skip)] cached: T }",
            vec![BUILDS],
        ),
        (
            "struct Cached<T> { #[serde(skip_deserializing)] cached: T }",
            vec![BUILDS],
        ),
        (
            "struct Cached<T> { #[serde(skip, default = \"blank\")] cached: T }",
            Vec::new(),
        ),
        (
            "enum Answer<T, U> { Value(T), #[serde(skip_deserializing)] Hidden(U) }",
            vec![READS],
        ),
        (
            "enum Answer<T> { #[serde(deserialize_with = \"held\")] Value(T) }",
            Vec::new(),
        ),
    ] {
        assert_eq!(asked_of(source), asked, "for {source}");
    }
}

#[test]
fn a_default_asks_default_of_what_it_builds() {
    for (source, asked) in [
        (
            "struct Kept<T> { #[serde(default)] kept: T }",
            vec![READS, BUILDS],
        ),
        (
            "struct Kept<T> { #[serde(default = \"blank\")] kept: T }",
            vec![READS],
        ),
        (
            "#[serde(default)] struct Filled<T> { held: T }",
            vec!["Self : :: core :: default :: Default", READS],
        ),
        (
            "#[serde(default = \"filled\")] struct Filled<T> { held: T }",
            vec![READS],
        ),
        // The container builds a field it skips.
        (
            "#[serde(default)] struct Filled<T> { #[serde(skip)] held: T }",
            vec!["Self : :: core :: default :: Default"],
        ),
    ] {
        assert_eq!(asked_of(source), asked, "for {source}");
    }
}

#[test]
fn a_bound_written_on_the_container_is_the_whole_of_what_the_read_asks() {
    for (source, asked) in [
        (
            "#[serde(bound = \"T: Readable\")] struct Keyed<T> { held: T }",
            vec!["T : Readable"],
        ),
        (
            "#[serde(bound(deserialize = \"T: Readable<'de>\", serialize = \"T: Shown\"))] \
             struct Keyed<T> { #[serde(default)] held: T }",
            vec!["T : Readable < 'de >"],
        ),
        (
            "#[serde(bound = \"\")] struct Keyed<T> { held: T }",
            Vec::new(),
        ),
        (
            "#[serde(bound = \"T: Readable\", default)] struct Keyed<T> { held: T }",
            vec!["T : Readable"],
        ),
    ] {
        assert_eq!(asked_of(source), asked, "for {source}");
        assert!(bounds_of(source).written_by_container, "for {source}");
    }
    let written_for_the_write =
        "#[serde(bound(serialize = \"T: Shown\"))] struct Keyed<T> { held: T }";
    assert_eq!(asked_of(written_for_the_write), [READS]);
    assert!(!bounds_of(written_for_the_write).written_by_container);
}

#[test]
fn a_bound_written_on_a_field_or_a_variant_stands_in_for_what_it_would_ask() {
    for (source, asked) in [
        (
            "struct Keyed<T, U> { #[serde(bound = \"T: Readable\")] held: T, other: U }",
            vec!["T : Readable", "U : serde :: Deserialize < 'de >"],
        ),
        (
            "struct Keyed<T> { #[serde(bound(deserialize = \"T: Readable\"))] held: T }",
            vec!["T : Readable"],
        ),
        (
            "enum Keyed<T, U> { #[serde(bound = \"T: Readable\")] Held(T), Other(U) }",
            vec!["T : Readable", "U : serde :: Deserialize < 'de >"],
        ),
    ] {
        assert_eq!(asked_of(source), asked, "for {source}");
    }
}

#[test]
fn the_read_outlives_each_lifetime_serde_borrows_for() {
    for (source, borrowed) in [
        ("struct Owned { name: String }", Some(Vec::new())),
        ("struct Lent<'a> { text: &'a str }", Some(vec!["'a"])),
        ("struct Lent<'a> { bytes: &'a [u8] }", Some(vec!["'a"])),
        (
            "struct Lent<'a> { text: Option<&'a str> }",
            Some(vec!["'a"]),
        ),
        // A `Cow` is read owned where no `borrow` is written.
        ("struct Cowed<'a> { label: Cow<'a, str> }", Some(Vec::new())),
        (
            "struct Cowed<'a> { #[serde(borrow)] label: Cow<'a, str> }",
            Some(vec!["'a"]),
        ),
        (
            "struct Held<'a, 'b> { #[serde(borrow = \"'b\")] inner: Inner<'a, 'b> }",
            Some(vec!["'b"]),
        ),
        (
            "struct Held<'a, 'b> { #[serde(borrow)] inner: Inner<'a, 'b>, text: &'a str }",
            Some(vec!["'a", "'b"]),
        ),
        (
            "enum Step<'a> { #[serde(borrow)] Named(Cow<'a, str>), Rename { to: &'a str } }",
            Some(vec!["'a"]),
        ),
        (
            "struct Kept<'a> { #[serde(skip)] text: Option<&'a str> }",
            Some(Vec::new()),
        ),
        ("struct Fixed { name: &'static str }", None),
    ] {
        let named = borrowed.map(|lifetimes| {
            lifetimes
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<String>>()
        });
        assert_eq!(borrowed_for(source), named, "for {source}");
    }
}

#[test]
fn a_type_that_borrows_for_static_is_read_at_static() {
    assert_eq!(
        asked_of("struct Fixed<T> { held: T, name: &'static str }"),
        ["T : serde :: Deserialize < 'static >"]
    );
}

#[test]
fn a_borrowing_cow_is_read_by_a_function_of_serdes_own_and_asks_nothing() {
    assert_eq!(
        asked_of("struct Cowed<'a, T> { #[serde(borrow)] label: Cow<'a, str>, other: T }"),
        [READS]
    );
    assert_eq!(
        asked_of("struct Cowed<'a, T: ToOwned> { #[serde(borrow)] label: Cow<'a, T> }"),
        [READS]
    );
}
