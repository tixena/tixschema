//! The `http_rest` Swift client, read off the emitted text.
//!
//! No Swift toolchain is reachable here, so nothing here type-checks the emitted source — these
//! tests read structure, the same way `dart_http_client_tests` reads the Dart backend's own
//! output: a substring that must appear, and a name that must not.

use super::{
    SWIFT_BYTES_HEADER_OUT_SERVICE, SWIFT_HEADER_VEC_OF_OPTIONS_SERVICE, SWIFT_HTTP_SERVICE,
    SWIFT_MULTIPART_HTTP_SERVICE, SWIFT_NUMERIC_HEADER_OUT_SERVICE,
    SWIFT_SINGLE_PLACEHOLDER_HTTP_SERVICE, SWIFT_STREAM_HTTP_SERVICE,
    SWIFT_UNIT_SUCCESS_HTTP_SERVICE, swift_http_client_of,
};

/// The body of one method, from its own doc comment through the closing brace of the method
/// following it (or the end of the client) — mirrors `dart_http_client_tests`'s own `method_body`.
fn method_body<'written>(written: &'written str, call: &str) -> &'written str {
    let start = written.find(&format!(" {call}("));
    assert!(start.is_some(), "no method named `{call}` in: {written}");
    let rest = &written[start.unwrap()..];
    let end = rest.find("\n\n").unwrap_or(rest.len());
    &rest[..end]
}

#[test]
fn a_reply_answers_result_and_a_one_way_throws() {
    let written = swift_http_client_of(SWIFT_HTTP_SERVICE);
    assert!(
        written.contains(
            "public func createDocument(_ req: CreateDocumentRequest) async -> Result<CreateDocumentResponse, DocumentClientServiceCreateDocumentFailure> {"
        ),
        "got: {written}"
    );
    assert!(
        written.contains("public func purgeDocument(_ req: String) async throws {"),
        "a one-way method still answers `async throws`, no `Result`. Got: {written}"
    );
}

#[test]
fn exactly_one_seam_type_is_emitted_and_it_names_no_networking_library() {
    let written = swift_http_client_of(SWIFT_HTTP_SERVICE);
    assert_eq!(
        written
            .matches("public protocol DocumentClientServiceHttpTransport: Sendable {")
            .count(),
        1,
        "one seam type serves every operation on the service. Got: {written}"
    );
    for named in ["URLSession", "Alamofire", "import "] {
        assert!(
            !written.contains(named),
            "the seam speaks only in plain terms; the library that finally carries the call is \
             an adapter's business, never this crate's. Got: {written}"
        );
    }
}

#[test]
fn the_seam_carries_a_structural_request_struct_in_and_response_struct_out() {
    let written = swift_http_client_of(SWIFT_HTTP_SERVICE);
    assert!(
        written.contains(
            "public struct DocumentClientServiceHttpRequest: Sendable {\n  \
             public let method: String\n  \
             public let path: String\n  \
             public let query: String\n  \
             public let headers: [(String, String)]\n  \
             public let body: Data"
        ),
        "got: {written}"
    );
    assert!(
        written.contains(
            "public struct DocumentClientServiceHttpResponse: Sendable {\n  \
             public let status: Int\n  \
             public let headers: [(String, String)]\n  \
             public let body: Data"
        ),
        "got: {written}"
    );
}

#[test]
fn a_bodied_operation_fills_a_literal_path_and_encodes_the_message_as_the_body() {
    let written = swift_http_client_of(SWIFT_HTTP_SERVICE);
    let method = method_body(&written, "createDocument");
    assert!(
        method.contains("path += \"/documents\""),
        "a path with no placeholder is pushed exactly as written. Got: {method}"
    );
    assert!(
        method.contains("let query: [(String, String)] = []"),
        "a bodied method carries no query string. Got: {method}"
    );
    assert!(
        method.contains("body = try JSONEncoder().encode(req)"),
        "the body is the message's own Codable encode, never re-derived. Got: {method}"
    );
    assert!(method.contains("method: \"POST\""), "got: {method}");
}

#[test]
fn a_path_placeholder_is_filled_by_percent_encoding_the_field_it_names() {
    let written = swift_http_client_of(SWIFT_HTTP_SERVICE);
    let method = method_body(&written, "getVersion");
    assert!(
        method.contains("path += \"/documents/\"")
            && method
                .contains("path += documentClientServicePercentEncode(\"\\(req.documentId)\")")
            && method.contains("path += \"/versions/\"")
            && method.contains("path += documentClientServicePercentEncode(\"\\(req.versionId)\")"),
        "each segment is pushed in template order, a placeholder reading its own property off \
         the message under Swift's own camelCase spelling. Got: {method}"
    );
}

#[test]
fn a_placeholder_bound_generated_field_fills_the_path() {
    let written = swift_http_client_of(SWIFT_SINGLE_PLACEHOLDER_HTTP_SERVICE);
    let method = method_body(&written, "window");
    assert!(
        method
            .contains("path += conversationClientServicePercentEncode(\"\\(req.conversationId)\")"),
        "the field the placeholder names is read off the message under Swift's own camelCase \
         spelling. Got: {method}"
    );
}

#[test]
fn a_lone_placeholder_on_a_scalar_message_still_is_the_whole_message() {
    let written = swift_http_client_of(SWIFT_SINGLE_PLACEHOLDER_HTTP_SERVICE);
    let method = method_body(&written, "purgeConversation");
    assert!(
        method.contains("path += conversationClientServicePercentEncode(\"\\(req)\")"),
        "a message that already is a wire scalar has no field to read: it is the segment. \
         Got: {method}"
    );
}

#[test]
fn an_unbound_generated_field_builds_the_query_string_of_a_bodyless_method() {
    let written = swift_http_client_of(SWIFT_SINGLE_PLACEHOLDER_HTTP_SERVICE);
    let method = method_body(&written, "window");
    assert!(
        method.contains("var query: [(String, String)] = []")
            && method.contains("if let value = req.limit {")
            && method.contains("query.append((\"limit\", \"\\(value)\"))"),
        "a field the path does not bind is a query parameter, read off the message by its own \
         type. Got: {method}"
    );
    assert!(
        !method.contains("let query: [(String, String)] = []"),
        "a message carrying more than the path spends does not send an empty query. \
         Got: {method}"
    );
}

#[test]
fn a_scalar_named_message_builds_no_query_string() {
    let written = swift_http_client_of(SWIFT_SINGLE_PLACEHOLDER_HTTP_SERVICE);
    let method = method_body(&written, "purgeConversation");
    assert!(
        method.contains("let query: [(String, String)] = []"),
        "a message that already is a wire scalar has no keys left over: the path spent it. \
         Got: {method}"
    );
}

#[test]
fn a_header_in_binding_becomes_an_extra_parameter_and_a_built_header() {
    let written = swift_http_client_of(SWIFT_HTTP_SERVICE);
    assert!(
        written.contains("getVersion(_ req: GetVersionRequest, byte_range: String?)"),
        "the client method spells the extra parameter beside the message, under its own Rust \
         spelling. Got: {written}"
    );
    let method = method_body(&written, "getVersion");
    assert!(
        method.contains("var headers: [(String, String)] = []")
            && method.contains(
                "if let byte_range = byte_range {\n      \
                 if !documentClientServiceLegalHeaderValue(\"\\(byte_range)\") {\n        \
                 return .failure(.fault(documentClientServiceHttpOutboundFault(\"get-version\", \
                 \"range\", \"a header value contains a character illegal in an HTTP \
                 header\")))\n      \
                 }\n      \
                 headers.append((\"range\", \"\\(byte_range)\"))\n    }"
            ),
        "the header is built from the extra argument, never from the message, reads the \
         parameter the `if let` has already unwrapped, and checks the rendered value before it \
         is appended. Got: {method}"
    );
}

#[test]
fn an_absent_option_header_in_is_omitted_rather_than_sent_as_an_empty_string() {
    let written = swift_http_client_of(SWIFT_HTTP_SERVICE);
    let method = method_body(&written, "getVersion");
    assert!(
        !method.contains("?? \"\""),
        "a header the caller never bound must not travel as an empty-string value. Got: {method}"
    );
}

#[test]
fn a_bodyless_operation_with_no_header_in_builds_an_empty_header_list() {
    let written = swift_http_client_of(SWIFT_HTTP_SERVICE);
    let method = method_body(&written, "createDocument");
    assert!(
        method.contains("let headers: [(String, String)] = []"),
        "got: {method}"
    );
}

#[test]
fn unbound_optional_fields_build_a_query_string_a_vec_joining_by_comma() {
    let written = swift_http_client_of(SWIFT_HTTP_SERVICE);
    let method = method_body(&written, "searchDocuments");
    assert!(
        method.contains("if let value = req.q {")
            && method.contains("query.append((\"q\", \"\\(value)\"))"),
        "a scalar optional field pushes its own wire key when present. Got: {method}"
    );
    assert!(
        method.contains("if let value = req.tags {")
            && method.contains(
                "query.append((\"tags\", ((value).map { element in \"\\(element)\" }.joined(separator: \",\"))))"
            ),
        "a Vec field joins its elements with a comma before it is percent-encoded. Got: {method}"
    );
}

#[test]
fn the_declared_ok_status_decodes_into_the_success_type() {
    let written = swift_http_client_of(SWIFT_HTTP_SERVICE);
    let method = method_body(&written, "createDocument");
    assert!(
        method.contains("if status == 200 {")
            && method.contains(
                "value = try JSONDecoder().decode(CreateDocumentResponse.self, from: response.body)"
            )
            && method.contains("return .success(value)"),
        "got: {method}"
    );
}

#[test]
fn a_mapped_error_status_decodes_into_the_declared_error_and_is_returned() {
    let written = swift_http_client_of(SWIFT_HTTP_SERVICE);
    let method = method_body(&written, "createDocument");
    assert!(
        method.contains("if status == 409 {")
            && method.contains(
                "declared = try JSONDecoder().decode(CreateDocumentError.self, from: response.body)"
            )
            && method.contains("return .failure(.declared(declared))"),
        "got: {method}"
    );
}

#[test]
fn a_fixed_fault_status_decodes_into_a_fault_reusing_the_generated_fault_fields_codec() {
    let written = swift_http_client_of(SWIFT_HTTP_SERVICE);
    let method = method_body(&written, "createDocument");
    assert!(
        method.contains("if status == 400 || status == 404 || status == 500 {")
            && method.contains(
                "return .failure(.fault(documentClientServiceFaultFromBody(\"create-document\", response.body)))"
            ),
        "got: {method}"
    );
    assert!(
        written.contains(
            "if let decoded = try? JSONDecoder().decode(DocumentClientServiceFaultFields.self, from: body) {"
        ),
        "the fault body decodes through the same generated `FaultFields` codec every other \
         surface answers faults through, rather than a hand-rolled parse. Got: {written}"
    );
}

#[test]
fn an_unexpected_status_decodes_into_an_undeserializable_payload_fault() {
    let written = swift_http_client_of(SWIFT_HTTP_SERVICE);
    let method = method_body(&written, "createDocument");
    assert!(
        method.contains(
            "documentClientServiceUndeserializablePayload(\n        \"create-document\", \"an unexpected status (\\(status)) answered\"\n      )"
        ),
        "got: {method}"
    );
}

#[test]
fn an_operation_naming_no_http_group_defaults_to_post_its_own_wire_name_and_the_fixed_error_status()
{
    let written = swift_http_client_of(SWIFT_HTTP_SERVICE);
    let method = method_body(&written, "sweepDocuments");
    assert!(
        method.contains("path += \"/sweep-documents\"") && method.contains("method: \"POST\""),
        "got: {method}"
    );
    assert!(
        method.contains("if status == 422 {"),
        "an operation naming no `error_status` table maps every declared error to the fixed \
         binding-error status. Got: {method}"
    );
}

#[test]
fn a_header_out_tuple_success_reads_the_body_and_the_header_back() {
    let written = swift_http_client_of(SWIFT_HTTP_SERVICE);
    let method = method_body(&written, "getVersion");
    assert!(
        method.contains(
            "value = try JSONDecoder().decode(VersionResponse.self, from: response.body)"
        ),
        "got: {method}"
    );
    assert!(
        method.contains(
            "guard let rawHeaderOut0 = documentClientServiceFindHeader(response.headers, \"etag\") else {"
        ) && method.contains("let headerOut0 = rawHeaderOut0")
            && method.contains("return .success((value, headerOut0))"),
        "the response header is read back and joined onto the decoded body as the method's own \
         success value. Got: {method}"
    );
    assert!(
        written.contains(
            "public func getVersion(_ req: GetVersionRequest, byte_range: String?) async -> Result<(VersionResponse, String), DocumentClientServiceGetVersionFailure> {"
        ),
        "the method's own return type carries the success tuple the operation declared. \
         Got: {written}"
    );
}

#[test]
fn a_numeric_header_out_element_parses_into_its_own_declared_width() {
    let written = swift_http_client_of(SWIFT_NUMERIC_HEADER_OUT_SERVICE);
    let method = method_body(&written, "getMetric");
    assert!(
        method.contains("guard let headerOut0 = UInt32(rawHeaderOut0) else {"),
        "got: {method}"
    );
    assert!(
        method.contains("guard let headerOut1 = Float(rawHeaderOut1) else {"),
        "got: {method}"
    );
}

#[test]
fn a_present_header_that_will_not_decode_faults_rather_than_defaulting() {
    let written = swift_http_client_of(SWIFT_NUMERIC_HEADER_OUT_SERVICE);
    let method = method_body(&written, "getMetric");
    assert!(
        method.matches(
            "metricClientServiceUndeserializablePayload(\"get-metric\", \"a response header did \
             not match its declared type\")"
        )
        .count()
            == 2,
        "both numeric elements fault the same way on a present value that will not parse. \
         Got: {method}"
    );
    assert!(
        !method.contains("?? 0"),
        "a bad numeric header must not silently answer 0. Got: {method}"
    );
}

#[test]
fn a_no_payload_one_way_operation_resolves_on_its_declared_status_without_reading_a_body() {
    let written = swift_http_client_of(SWIFT_HTTP_SERVICE);
    assert!(
        written.contains("public func purgeDocument(_ req: String) async throws {"),
        "got: {written}"
    );
    let method = method_body(&written, "purgeDocument");
    assert!(
        method.contains("path += documentClientServicePercentEncode(\"\\(req)\")"),
        "a `Named` message answering to its one placeholder is the value itself. Got: {method}"
    );
    assert!(
        method.contains("if status == 204 { return }"),
        "a one-way operation resolves on its declared (default 204) status with no body read. \
         Got: {method}"
    );
    assert!(
        method.contains("throw DocumentClientServiceRefusal("),
        "a one-way operation throws a fault-only refusal, having no declared error to carry one \
         in. Got: {method}"
    );
}

#[test]
fn a_bytes_operation_reads_the_body_and_content_type_back() {
    let written = swift_http_client_of(SWIFT_HTTP_SERVICE);
    assert!(
        written.contains(
            "public func getThumbnail(_ req: String) async -> Result<(Data, String), DocumentClientServiceGetThumbnailFailure> {"
        ),
        "a `body = \"bytes\"` operation's success carries the raw bytes and its content type, no \
         `header_out` involved. Got: {written}"
    );
    let method = method_body(&written, "getThumbnail");
    assert!(
        method.contains(
            "let contentType = documentClientServiceFindHeader(response.headers, \"content-type\") ?? \"\""
        ) && method.contains("return .success((response.body, contentType))"),
        "no `JSONDecoder` runs on a bytes body — it is read bare, and the content type is read \
         back from the response header. Got: {method}"
    );
}

#[test]
fn the_client_struct_holds_one_initializer_and_every_operation_as_a_method() {
    let written = swift_http_client_of(SWIFT_HTTP_SERVICE);
    assert!(
        written.contains("public struct DocumentClientServiceHttpClient: Sendable {")
            && written.contains(
                "public init(transport: any DocumentClientServiceHttpTransport) { self.transport = transport }"
            )
            && written.contains("private let transport: any DocumentClientServiceHttpTransport"),
        "got: {written}"
    );
    for call in [
        "createDocument",
        "getVersion",
        "searchDocuments",
        "getThumbnail",
        "purgeDocument",
        "sweepDocuments",
    ] {
        assert!(
            written.contains(&format!(" {call}(")),
            "operation `{call}` should have a method on the client. Got: {written}"
        );
    }
}

#[test]
fn a_bytes_operation_with_header_out_composes_body_content_type_and_the_header() {
    let written = swift_http_client_of(SWIFT_BYTES_HEADER_OUT_SERVICE);
    assert!(
        written.contains(
            "public func getThumbnail(_ req: String) async -> Result<(Data, String, String), ThumbnailClientServiceGetThumbnailFailure> {"
        ),
        "the success carries the bytes-and-content-type tuple, with one more slot per declared \
         `header_out` entry. Got: {written}"
    );
    let method = method_body(&written, "getThumbnail");
    assert!(
        method.contains(
            "let contentType = thumbnailClientServiceFindHeader(response.headers, \"content-type\") ?? \"\""
        ) && method.contains(
            "guard let rawHeaderOut0 = thumbnailClientServiceFindHeader(response.headers, \"x-document-id\") else {"
        ) && method.contains("let headerOut0 = rawHeaderOut0")
            && method.contains("return .success((response.body, contentType, headerOut0))"),
        "the declared header is read back the same way the JSON and stream paths read one, \
         after the bytes and their content type. Got: {method}"
    );
}

#[test]
fn a_stream_operation_answers_a_content_range_and_body_pair_at_200_and_206() {
    let written = swift_http_client_of(SWIFT_STREAM_HTTP_SERVICE);
    assert!(
        written.contains("public func getFile(_ req: String) async -> Result<(contentRange: String?, body: AsyncThrowingStream<Data, Error>), ContentClientServiceGetFileFailure> {"),
        "a bare `StreamedAnswer` renders as a pair of a nullable `contentRange` and a lazy \
         `AsyncThrowingStream<Data, Error>` body, carried as the method's own success value. \
         Got: {written}"
    );
    let method = method_body(&written, "getFile");
    assert!(
        method.contains("if status == 206 {")
            && method.contains(
                "let contentRange = contentClientServiceFindHeader(response.headers, \"content-range\") ?? \"\""
            )
            && method.contains(
                "let answer = (contentRange: contentRange, body: response.bodyStream)"
            )
            && method.contains("return .success(answer)"),
        "a `206` answers the pair with `contentRange` read back off the response. Got: {method}"
    );
    assert!(
        method.contains("if status == 200 {") && method.contains("let contentRange: String? = nil"),
        "the declared `ok_status` answers the same pair with `contentRange` left `nil`. \
         Got: {method}"
    );
}

#[test]
fn a_stream_operation_with_header_out_wraps_the_pair_in_a_tuple() {
    let written = swift_http_client_of(SWIFT_STREAM_HTTP_SERVICE);
    let method = method_body(&written, "getTaggedFile");
    assert!(
        method.contains(
            "guard let rawHeaderOut0 = contentClientServiceFindHeader(response.headers, \"x-checksum\") else {"
        ) && method.contains("let headerOut0 = rawHeaderOut0")
            && method.contains("return .success((answer, headerOut0))"),
        "the header is read back once the pair is built, in both the `206` and `200` arms. \
         Got: {method}"
    );
}

#[test]
fn the_seam_carries_a_lazy_body_stream_only_where_a_service_declares_one() {
    let plain = swift_http_client_of(SWIFT_HTTP_SERVICE);
    assert!(
        !plain.contains("bodyStream"),
        "a service with no streamed operation carries no `bodyStream` field. Got: {plain}"
    );
    let streamed = swift_http_client_of(SWIFT_STREAM_HTTP_SERVICE);
    assert!(
        streamed.contains("public let bodyStream: AsyncThrowingStream<Data, Error>"),
        "a service with a streamed operation carries `bodyStream` on the seam's own response \
         struct, and no networking library is named to spell it. Got: {streamed}"
    );
    for named in ["URLSession", "Alamofire", "import "] {
        assert!(
            !streamed.contains(named),
            "the seam still names no networking library once it carries a stream. \
             Got: {streamed}"
        );
    }
}

#[test]
fn a_multipart_operation_carries_parts_on_the_seam_and_takes_an_extra_file_argument() {
    let written = swift_http_client_of(SWIFT_MULTIPART_HTTP_SERVICE);
    assert!(
        written.contains("public let parts: [(String, any Sendable)]"),
        "the seam's own request struct carries `parts` for a multipart-declaring service. \
         Got: {written}"
    );
    assert!(
        written.contains(
            "public func uploadDocument(_ req: UploadDocumentRequest, attachment: any Sendable) async -> Result<UploadResponse, UploadClientServiceUploadDocumentFailure> {"
        ),
        "the client method spells the extra file argument beside the message, under its own \
         Rust spelling and an opaque `Sendable` type. Got: {written}"
    );
}

#[test]
fn a_multipart_method_builds_one_text_part_per_field_and_one_file_part_per_binding() {
    let written = swift_http_client_of(SWIFT_MULTIPART_HTTP_SERVICE);
    let method = method_body(&written, "uploadDocument");
    assert!(
        method.contains("let body = Data()"),
        "a multipart method's content rides in `parts`, never `body`. Got: {method}"
    );
    assert!(
        method.contains("var parts: [(String, any Sendable)] = []")
            && method.contains("parts.append((\"title\", \"\\(req.title)\"))")
            && method.contains(
                "if let value = req.description {\n      parts.append((\"description\", \"\\(value)\"))\n    }"
            )
            && method.contains("parts.append((\"file\", attachment))"),
        "one text part per carried field not otherwise placeholder-bound, then one file part per \
         `part` binding. Got: {method}"
    );
    assert!(
        !method.contains("parts.append((\"folderId\""),
        "the path-bound field is read off the path, not sent as a text part too. Got: {method}"
    );
    assert!(
        method.contains(
            "UploadClientServiceHttpRequest(method: \"POST\", path: path, query: uploadClientServiceQueryText(query), headers: headers, body: body, parts: parts)"
        ),
        "`parts` rides beside `body` in the request the seam is handed. Got: {method}"
    );
}

#[test]
fn the_header_reader_is_published_under_the_service_prefix_only_where_it_is_called() {
    for (source, reader, service) in [
        (
            SWIFT_HTTP_SERVICE,
            "documentClientServiceFindHeader",
            "a declared header_out",
        ),
        (
            SWIFT_BYTES_HEADER_OUT_SERVICE,
            "thumbnailClientServiceFindHeader",
            "a bytes answer's content type",
        ),
        (
            SWIFT_STREAM_HTTP_SERVICE,
            "contentClientServiceFindHeader",
            "a streamed answer's content range",
        ),
    ] {
        let written = swift_http_client_of(source);
        assert!(
            written.contains(&format!("func {reader}(")),
            "{service} reads a response header back, so the reader is published beside the \
             client — under the service's own prefix, since two clients vendored into one file \
             would otherwise both declare it. Got: {written}"
        );
    }
    let written = swift_http_client_of(SWIFT_MULTIPART_HTTP_SERVICE);
    assert!(
        !written.contains("FindHeader"),
        "every operation on this service answers plain JSON and declares no header_out, so \
         nothing calls the reader — and an unused top-level function is a dead-code diagnostic \
         in the file this client is vendored into. Got: {written}"
    );
}

#[test]
fn a_header_vec_of_options_narrows_its_element_without_a_forced_unwrap() {
    let written = swift_http_client_of(SWIFT_HEADER_VEC_OF_OPTIONS_SERVICE);
    let method = method_body(&written, "listTags");
    assert!(
        method.contains(
            "headers.append((\"x-tags\", ((tags).map { element in ((element).map { unwrapped in \"\\(unwrapped)\" } ?? \"\") }.joined(separator: \",\"))))"
        ),
        "an absent element inside a present `Vec` renders as the empty string, through `.map`, \
         never a forced unwrap. Got: {method}"
    );
}

#[test]
fn a_unit_success_answers_void() {
    let written = swift_http_client_of(SWIFT_UNIT_SUCCESS_HTTP_SERVICE);
    assert!(
        written.contains(
            "public func ping(_ req: PingRequest) async -> Result<Void, PingClientServicePingFailure> {"
        ),
        "got: {written}"
    );
    let method = method_body(&written, "ping");
    assert!(method.contains("return .success(())"), "got: {method}");
}
