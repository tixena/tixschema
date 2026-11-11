//! The Kotlin and Swift `mcp` text of each service of `mcp_note_services`, read for a helper that
//! nothing in the same text calls. Neither compiler reports an unused private declaration, so the
//! text itself is read: a helper's name written once is written only where it is declared.

use super::mcp_note_services::{
    NoteBlocksServiceSchema, NoteCountServiceSchema, NoteGreetingServiceSchema,
    NotePreviewServiceSchema, NoteReviewServiceSchema,
};
use regex::Regex;

/// The names `declared` captures in `text` that `text` holds as a whole word nowhere else.
fn named_once(text: &str, declared: &Regex) -> Vec<String> {
    declared
        .captures_iter(text)
        .map(|found| found[1].to_owned())
        .filter(|name| {
            Regex::new(&format!(r"\b{}\b", regex::escape(name)))
                .unwrap()
                .find_iter(text)
                .count()
                == 1
        })
        .collect()
}

/// Every name in `surfaces` that `declared` captures and its own text never reads, labelled with
/// the surface it is declared in.
fn unread(surfaces: &[(&str, String)], declared: &Regex) -> Vec<String> {
    surfaces
        .iter()
        .flat_map(|(named, text)| {
            named_once(text, declared)
                .into_iter()
                .map(move |name| format!("{named}: {name}"))
        })
        .collect()
}

#[cfg(feature = "kotlin")]
#[test]
fn every_private_declaration_of_the_kotlin_mcp_text_of_each_note_service_is_read() {
    let private = Regex::new(
        r"\bprivate\s+(?:(?:const|inline|suspend|data|sealed|enum|abstract|open|override|lateinit|value)\s+)*(?:fun|val|var|class|object|interface)\s+(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>\s*)?(?:[A-Za-z_]\w*\.)?([A-Za-z_]\w*)",
    )
    .unwrap();
    let servers = [
        ("messages", NoteReviewServiceSchema::kotlin_mcp_service()),
        ("text", NoteGreetingServiceSchema::kotlin_mcp_service()),
        ("pair", NotePreviewServiceSchema::kotlin_mcp_service()),
        ("list", NoteBlocksServiceSchema::kotlin_mcp_service()),
        ("plain", NoteCountServiceSchema::kotlin_mcp_service()),
    ];
    let clients = [
        ("messages", NoteReviewServiceSchema::kotlin_mcp_client()),
        ("text", NoteGreetingServiceSchema::kotlin_mcp_client()),
        ("pair", NotePreviewServiceSchema::kotlin_mcp_client()),
        ("list", NoteBlocksServiceSchema::kotlin_mcp_client()),
        ("plain", NoteCountServiceSchema::kotlin_mcp_client()),
    ];
    assert_eq!(unread(&servers, &private), Vec::<String>::new(), "servers");
    assert_eq!(unread(&clients, &private), Vec::<String>::new(), "clients");
}

/// Every top-level Swift declaration not marked `public` is a helper, and each one is read.
#[cfg(feature = "swift")]
#[test]
fn every_helper_of_the_swift_mcp_client_of_each_note_service_is_read() {
    let helper = Regex::new(
        r"(?m)^(?:(?:private|fileprivate|internal|final|indirect)\s+)*(?:func|struct|enum|class|actor|protocol|typealias|let|var)\s+([A-Za-z_]\w*)",
    )
    .unwrap();
    let clients = [
        ("messages", NoteReviewServiceSchema::swift_mcp_client()),
        ("text", NoteGreetingServiceSchema::swift_mcp_client()),
        ("pair", NotePreviewServiceSchema::swift_mcp_client()),
        ("list", NoteBlocksServiceSchema::swift_mcp_client()),
        ("plain", NoteCountServiceSchema::swift_mcp_client()),
    ];
    assert_eq!(unread(&clients, &helper), Vec::<String>::new());
}
