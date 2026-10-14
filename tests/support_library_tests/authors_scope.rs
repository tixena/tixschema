//! A flagged type in a module whose author declares or imports a name its crate's library declares
//! too: the type's schema module holds the library's, and the author's stays the author's.

mod filters;
mod paths;

use serde_json::json;

#[test]
fn the_schema_module_holds_the_librarys_path_and_the_author_keeps_theirs() {
    let library =
        paths::invoice_lookup_schema::Path(vec![paths::invoice_lookup_schema::Segment::Key(
            "number".to_owned(),
        )]);
    assert_eq!(library.to_string(), "number");
    assert_eq!(paths::authors("a/b").to_str(), Some("a/b"));
    let read =
        paths::InvoiceLookup::from_value_with(json!({ "number": "INV-1" }), |_raw, _found| {
            paths::invoice_lookup_schema::Verdict::Reject
        });
    assert_eq!(
        read,
        Ok(paths::InvoiceLookup {
            number: "INV-1".to_owned()
        })
    );
}

#[test]
fn the_author_keeps_their_filter() {
    assert_eq!(
        filters::Filter {
            number: "INV-1".to_owned()
        }
        .number,
        "INV-1"
    );
}

#[cfg(feature = "mongodb")]
#[test]
fn the_schema_module_holds_the_librarys_filter() {
    let library: filters::invoice_schema::Filter<filters::Invoice> = filters::Invoice::MONGO_FIELDS
        .number()
        .eq("INV-1".to_owned())
        .unwrap();
    assert_eq!(
        library.into_document().to_string(),
        r#"{ "number": { "$eq": "INV-1" } }"#
    );
}
