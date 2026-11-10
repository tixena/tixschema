//! The `mcp` clients of `LibraryService`, whose resources, listing and completion are no tools: each
//! language's client holds a method for the one tool and for nothing else, and its text compiles.

use super::LibraryServiceSchema;
use super::mcp_library_service::{BookError, BookInfo, BookRequest};
use super::runtime::{compiled_with_modules, node_modules, stand_down_modules};

const ABSENT: [&str; 8] = [
    "libraryGuide",
    "bookCover",
    "listBooks",
    "suggestBooks",
    "LibraryGuide",
    "BookCover",
    "ListBooks",
    "SuggestBooks",
];

fn holds_the_tool_alone(text: &str) {
    assert!(text.contains("getBook"), "{text}");
    for name in ABSENT {
        assert!(!text.contains(name), "the client holds `{name}`:\n{text}");
    }
}

#[test]
fn the_typescript_client_holds_a_method_for_the_tool_alone() {
    holds_the_tool_alone(&LibraryServiceSchema::ts_mcp_client());
}

#[cfg(feature = "dart")]
#[test]
fn the_dart_client_holds_a_method_for_the_tool_alone() {
    holds_the_tool_alone(&LibraryServiceSchema::dart_mcp_client());
}

#[cfg(feature = "kotlin")]
#[test]
fn the_kotlin_client_holds_a_method_for_the_tool_alone() {
    holds_the_tool_alone(&LibraryServiceSchema::kotlin_mcp_client());
}

#[cfg(feature = "swift")]
#[test]
fn the_swift_client_holds_a_method_for_the_tool_alone() {
    holds_the_tool_alone(&LibraryServiceSchema::swift_mcp_client());
}

fn typescript() -> String {
    [
        "import { z, type ZodType, type $brand } from \"zod\";".to_owned(),
        BookError::ts_definition(),
        BookError::zod_schema(),
        BookInfo::ts_definition(),
        BookInfo::zod_schema(),
        BookRequest::ts_definition(),
        BookRequest::zod_schema(),
        LibraryServiceSchema::ts_definition(),
        LibraryServiceSchema::ts_service(),
        LibraryServiceSchema::ts_mcp_content(),
        LibraryServiceSchema::ts_mcp_service(),
        LibraryServiceSchema::ts_mcp_client(),
    ]
    .join("\n\n")
}

#[test]
fn the_typescript_client_of_the_library_service_type_checks() {
    let Some(modules) = node_modules(&["zod"]) else {
        stand_down_modules(&["zod"], "the type-check of the library service's client");
        return;
    };
    let text = typescript();
    let usage = "import { createLibraryServiceMcpClient, type LibraryServiceMcpTransport } from \"./bundle.js\";\n\
                 declare const transport: LibraryServiceMcpTransport;\n\
                 export const client = createLibraryServiceMcpClient(transport);\n\
                 export const call = client.getBook({ book_id: \"book-1\" });\n";
    let Some((succeeded, said)) = compiled_with_modules(
        "mcp-library-client-types",
        "TIXSCHEMA_TSC",
        &[("bundle.ts", &text), ("usage.ts", usage)],
        &modules,
    ) else {
        return;
    };
    assert!(succeeded, "the emitted text does not type-check:\n{said}");
}

#[cfg(feature = "dart")]
#[test]
fn the_dart_client_of_the_library_service_passes_the_analyzer() {
    use super::mcp_library_service::{book_error_dart, book_info_dart, book_request_dart};
    let source = [
        "import 'dart:async';".to_owned(),
        "import 'dart:convert';".to_owned(),
        "import 'dart:typed_data';".to_owned(),
        book_error_dart::dart_definition(),
        book_info_dart::dart_definition(),
        book_request_dart::dart_definition(),
        LibraryServiceSchema::dart_definition(),
        LibraryServiceSchema::dart_ws_client(),
        LibraryServiceSchema::dart_mcp_content(),
        LibraryServiceSchema::dart_mcp_service(),
        LibraryServiceSchema::dart_mcp_client(),
    ]
    .join("\n\n");
    super::run_dart_validate::analyzed("mcp-library-client-analyze", &source);
}

#[cfg(feature = "kotlin")]
#[test]
fn the_kotlin_client_of_the_library_service_compiles() {
    use super::mcp_library_service::{
        book_error_kotlin, book_info_kotlin, book_request_kotlin, library_guide_request_kotlin,
        suggest_books_request_kotlin,
    };
    use super::run_kotlin_mcp_resources::KOTLIN_IMPORTS;
    let source = [
        KOTLIN_IMPORTS.to_owned(),
        book_info_kotlin::kotlin_definition(),
        book_error_kotlin::kotlin_definition(),
        book_request_kotlin::kotlin_definition(),
        library_guide_request_kotlin::kotlin_definition(),
        suggest_books_request_kotlin::kotlin_definition(),
        super::library_service_schema::library_service_fault_fields_kotlin::kotlin_definition(),
        super::library_service_schema::library_service_fault_kind_kotlin::kotlin_definition(),
        LibraryServiceSchema::kotlin_http_client(),
        LibraryServiceSchema::kotlin_ws_client(),
        LibraryServiceSchema::kotlin_mcp_content(),
        LibraryServiceSchema::kotlin_mcp_service(),
        LibraryServiceSchema::kotlin_mcp_client(),
        "fun main() {}".to_owned(),
    ]
    .join("\n\n");
    let ran: Option<String> = super::runtime::ran_kotlin(&source);
    drop(ran);
}

#[cfg(feature = "swift")]
#[test]
fn the_swift_client_of_the_library_service_compiles() {
    use super::mcp_library_service::{
        book_error_swift, book_info_swift, book_request_swift, library_guide_request_swift,
        suggest_books_request_swift,
    };
    let source = [
        "import Foundation".to_owned(),
        book_error_swift::swift_definition(),
        book_info_swift::swift_definition(),
        book_request_swift::swift_definition(),
        library_guide_request_swift::swift_definition(),
        suggest_books_request_swift::swift_definition(),
        super::library_service_schema::library_service_fault_fields_swift::swift_definition(),
        super::library_service_schema::library_service_fault_kind_swift::swift_definition(),
        LibraryServiceSchema::swift_http_client(),
        LibraryServiceSchema::swift_mcp_client(),
        LibraryServiceSchema::swift_mcp_content(),
    ]
    .join("\n\n");
    let ran: Option<String> = super::runtime::ran(
        "swift-mcp-library",
        "TIXSCHEMA_SWIFT",
        "swift",
        "main.swift",
        &source,
    );
    drop(ran);
}
