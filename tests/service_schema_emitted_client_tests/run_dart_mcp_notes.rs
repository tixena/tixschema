//! `dart analyze` over the Dart `mcp` server and client of each service of `mcp_note_services`,
//! which reports a private helper nothing calls as `unused_element`.

use super::mcp_note_services::{
    NoteBlocksServiceSchema, NoteCountServiceSchema, NoteGreetingServiceSchema,
    NotePreviewServiceSchema, NoteReviewServiceSchema, note_count_dart, note_missing_dart,
};
use super::run_dart_validate::analyzed;

/// The imports, the models and `surfaces`; `bytes` says whether the text names `Uint8List`.
fn module(bytes: bool, surfaces: Vec<String>) -> String {
    let mut written = vec![
        "import 'dart:async';".to_owned(),
        "import 'dart:convert';".to_owned(),
    ];
    if bytes {
        written.push("import 'dart:typed_data';".to_owned());
    }
    written.extend([
        note_missing_dart::dart_definition(),
        note_count_dart::dart_definition(),
    ]);
    written.extend(surfaces);
    written.join("\n\n")
}

#[test]
fn the_dart_mcp_text_of_each_note_service_passes_the_analyzer() {
    macro_rules! surfaces {
        ($service:ident $(, $content:ident)?) => {
            vec![
                $service::dart_definition(),
                $($service::$content(),)?
                $service::dart_ws_client(),
                $service::dart_mcp_service(),
                $service::dart_mcp_client(),
            ]
        };
    }
    for (named, written) in [
        (
            "mcp-notes-messages-analyze",
            module(true, surfaces!(NoteReviewServiceSchema, dart_mcp_content)),
        ),
        (
            "mcp-notes-text-analyze",
            module(false, surfaces!(NoteGreetingServiceSchema)),
        ),
        (
            "mcp-notes-pair-analyze",
            module(true, surfaces!(NotePreviewServiceSchema)),
        ),
        (
            "mcp-notes-list-analyze",
            module(true, surfaces!(NoteBlocksServiceSchema, dart_mcp_content)),
        ),
        (
            "mcp-notes-plain-analyze",
            module(false, surfaces!(NoteCountServiceSchema)),
        ),
    ] {
        analyzed(named, &written);
    }
}
