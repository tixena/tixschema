//! The valid struct the Swift emitter writes beside the draft of every model type: built from its
//! draft by the draft's `validate()` alone, read through the draft, and writing what it writes.

use serde::{Deserialize, Serialize};
use tixschema::model_schema;

use crate::swift_valid::draft_of;
use crate::tests::{
    envelope_swift, external_tagged_swift, status_swift, tuple_point_swift, window_request_swift,
};

/// A struct with a member named after the method a draft declares.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Audit {
    pub validation_issues: Vec<String>,
}

/// A struct with a member named after the valid type's own property.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Holder {
    pub draft: u32,
}

/// The valid type of `swift`, the text a model type writes after its draft.
fn valid_of(swift: &str) -> &str {
    swift
        .find("\n\n@dynamicMemberLookup")
        .map_or("", |at| swift[at..].trim_start())
}

/// A struct is wrapped under the declared name.
#[test]
fn a_struct_has_a_valid_struct_that_reads_through_its_draft() {
    let swift = window_request_swift::swift_definition();
    assert_eq!(
        valid_of(&swift),
        concat!(
            "@dynamicMemberLookup public struct WindowRequest: Codable, Sendable { ",
            "public let draft: WindowRequestDraft; ",
            "fileprivate init(draft: WindowRequestDraft) { self.draft = draft }; ",
            "public static func of(conversationId: String, limit: UInt32?) throws -> ",
            "WindowRequest { try WindowRequestDraft(conversationId: conversationId, limit: ",
            "limit).validate() }; ",
            "public subscript<SwiftSchemaHeld>(dynamicMember path: KeyPath<WindowRequestDraft, ",
            "SwiftSchemaHeld>) -> SwiftSchemaHeld { draft[keyPath: path] }; ",
            "public init(from decoder: Decoder) throws { draft = try WindowRequestDraft(from: ",
            "decoder) }; ",
            "fileprivate init(swiftSchemaUnjudged decoder: Decoder) throws { ",
            "draft = try WindowRequestDraft(swiftSchemaUnjudged: decoder) }; ",
            "fileprivate init(swiftSchemaUnjudged decoder: Decoder, swiftSchemaWithout taken: ",
            "[String]) throws { draft = try WindowRequestDraft(swiftSchemaUnjudged: decoder, ",
            "swiftSchemaWithout: taken) }; ",
            "public func encode(to encoder: Encoder) throws { try draft.encode(to: encoder) }; ",
            "fileprivate func validate() -> [String] { draft.validationIssues() }; ",
            "fileprivate func swiftSchemaValidateBound() -> [String] { ",
            "draft.swiftSchemaValidateBound() } };",
        )
    );
    assert!(
        draft_of(&swift).contains("public func validate() throws -> WindowRequest {"),
        "got: {swift}"
    );
}

/// A generic type is wrapped at its parameters, and an enum under every tagging the same way.
#[test]
fn a_generic_type_and_an_enum_are_wrapped_at_their_parameters() {
    let envelope = envelope_swift::swift_definition();
    assert!(
        valid_of(&envelope).starts_with(
            "@dynamicMemberLookup public struct Envelope<T: Codable & Sendable>: Codable, \
             Sendable { public let draft: EnvelopeDraft<T>; "
        ),
        "got: {envelope}"
    );
    let external = external_tagged_swift::swift_definition();
    assert!(
        valid_of(&external).starts_with(
            "@dynamicMemberLookup public struct ExternalTagged: Codable, Sendable { \
             public let draft: ExternalTaggedDraft; "
        ),
        "got: {external}"
    );
}

/// A plain enum is built from its raw value only where the generated file reads it, and is
/// `Hashable`, which a map keyed by it asks.
#[test]
fn a_plain_enum_is_hashable_and_built_from_its_raw_value_in_the_generated_file_alone() {
    let swift = status_swift::swift_definition();
    assert!(
        valid_of(&swift).starts_with(
            "@dynamicMemberLookup public struct Status: Codable, Sendable, Hashable { \
             public let draft: StatusDraft; fileprivate init(draft: StatusDraft) { \
             self.draft = draft }; fileprivate init?(rawValue: String) { guard let draft = \
             StatusDraft(rawValue: rawValue) else { return nil }; self.draft = draft }; "
        ),
        "got: {swift}"
    );
}

/// A tuple struct wraps its draft, which holds the `value` it reads.
#[test]
fn a_tuple_struct_has_a_draft_that_holds_its_value() {
    let swift = tuple_point_swift::swift_definition();
    assert!(
        valid_of(&swift).contains("public let draft: TuplePointDraft; "),
        "got: {swift}"
    );
    assert!(
        draft_of(&swift).contains("public struct TuplePointDraft"),
        "got: {swift}"
    );
}

/// A member named `validationIssues` is declared `validationIssues_`, the draft having the method,
/// and keeps its key.
#[test]
fn a_member_named_validation_issues_keeps_its_key() {
    let swift = audit_swift::swift_definition();
    let draft = draft_of(&swift);
    assert!(
        draft.contains("public var validationIssues_: [String]; "),
        "got: {swift}"
    );
    assert!(
        draft.contains("case validationIssues_ = \"validationIssues\"; "),
        "got: {swift}"
    );
    assert_eq!(
        serde_json::to_string(&AuditDraft {
            validation_issues: Vec::new()
        })
        .unwrap(),
        r#"{"validationIssues":[]}"#
    );
}

/// A member named `draft` is declared `draft_` and keeps its key, the valid type having the `draft`
/// property.
#[test]
fn a_member_named_draft_keeps_its_key() {
    let swift = holder_swift::swift_definition();
    let draft = draft_of(&swift);
    assert!(
        draft.contains("public var draft_: UInt32; "),
        "got: {swift}"
    );
    assert!(draft.contains("case draft_ = \"draft\"; "), "got: {swift}");
    assert!(
        valid_of(&swift).contains("public let draft: HolderDraft; "),
        "got: {swift}"
    );
    assert_eq!(
        serde_json::to_string(&HolderDraft { draft: 7 }).unwrap(),
        r#"{"draft":7}"#
    );
}
