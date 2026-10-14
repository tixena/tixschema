//! The README's section on a key read as its default, held to the models it shows, to what the
//! serde read prints and to the text each surface writes.

use super::models::REPORT;

/// The README's section, up to the one after it.
fn section() -> &'static str {
    let readme = include_str!("../../README.md");
    let opens = readme
        .find("### A Key Read as Its Default (`#[serde(default)]`)")
        .unwrap();
    let rest = &readme[opens..];
    &rest[..rest.find("### Collections and Maps").unwrap()]
}

/// The item of `models.rs` that `opens` starts, up to the brace that closes it.
fn item(opens: &str) -> &'static str {
    let models = include_str!("models.rs");
    let rest = &models[models.find(opens).unwrap()..];
    &rest[..rest.find("\n}").unwrap() + 2]
}

/// The declaration the section shows is the one the tests compile, and the value it says serde
/// reads is the one the table prints.
#[test]
fn the_readme_shows_the_declaration_and_the_read_of_the_models() {
    for shown in [
        item("pub fn north_dock()"),
        item(
            "#[model_schema()]\n#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]\n\
             pub struct Wharf {",
        ),
    ] {
        assert!(section().contains(shown), "{shown}");
    }
    let read = "{\"dock\":\"\",\"gangway\":{\"label\":\"main\",\"width\":2},\"gate\":\"north\",\
                \"label\":\"w\"}";
    assert!(section().contains(&format!("`{read}`")));
    assert!(REPORT.contains(&format!("every default at once: read {read}\n")));
    assert!(section().contains("`'dock': too short: minimum length is 1, got 0`"));
    assert!(REPORT.contains("    'dock': too short: minimum length is 1, got 0\n"));
}

/// Each line of the section's table is text the surface writes for `Wharf`.
#[cfg(feature = "zod")]
#[test]
fn the_zod_line_of_the_table_is_what_zod_writes() {
    let shown = "dock: z.string().prefault(\"\")";
    assert!(section().contains(&format!("`{shown}`")));
    assert!(super::models::Wharf::zod_schema().contains(shown));
}

#[cfg(feature = "jsonschema")]
#[test]
fn the_json_schema_line_of_the_table_is_what_the_json_schema_writes() {
    assert!(section().contains("`\"default\": \"\"`"));
    assert_eq!(
        super::models::Wharf::json_schema()["properties"]["dock"]["default"],
        serde_json::json!("")
    );
}

#[cfg(feature = "dart")]
#[test]
fn the_dart_line_of_the_table_is_what_dart_writes() {
    let shown =
        "json.containsKey('dock') ? (json['dock'] as String) : (jsonDecode('\"\"') as String)";
    assert!(section().contains(&format!("`{shown}`")));
    assert!(super::models::wharf_dart::dart_definition().contains(shown));
}

#[cfg(feature = "swift")]
#[test]
fn the_swift_line_of_the_table_is_what_swift_writes() {
    assert!(section().contains("`if container.contains(.dock) { ... } else { ... }`"));
    assert!(
        super::models::wharf_swift::swift_definition().contains("if container.contains(.dock) { ")
    );
}

#[cfg(feature = "kotlin")]
#[test]
fn the_kotlin_line_of_the_table_is_what_kotlin_writes() {
    let shown = "val dock: String = Json.decodeFromString(serializer<String>(), \"\\\"\\\"\")";
    assert!(section().contains(&format!("`{shown}`")));
    assert!(super::models::wharf_kotlin::kotlin_definition().contains(shown));
}
