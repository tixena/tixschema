//! The `http_rest` Dart client, read off the emitted text.
//!
//! No Dart toolchain is reachable here, so nothing here type-checks the emitted source — these
//! tests read structure, the same way `tests/dart_tests/tests.rs` reads the plain `dart` backend's
//! own output: a substring that must appear, and a name that must not.

use super::{DART_HTTP_SERVICE, dart_http_client_of};

/// The body of one method, from its own doc comment through the closing brace of the method
/// following it (or the end of the class) — mirrors `http_client_tests`'s own `method_body`.
fn method_body<'written>(written: &'written str, call: &str) -> &'written str {
    let start = written
        .find(&format!(" {call}("))
        .unwrap_or_else(|| panic!("no method named `{call}` in: {written}"));
    let rest = &written[start..];
    let end = rest.find("\n\n").unwrap_or(rest.len());
    &rest[..end]
}

#[test]
fn exactly_one_seam_type_is_emitted_and_it_names_no_http_package() {
    let written = dart_http_client_of(DART_HTTP_SERVICE);
    assert_eq!(
        written
            .matches("abstract class DocumentClientServiceHttpTransport {")
            .count(),
        1,
        "one seam type serves every operation on the service. Got: {written}"
    );
    for named in [
        "package:http",
        "package:dio",
        "package:chopper",
        "dart:io",
        "import '",
    ] {
        assert!(
            !written.contains(named),
            "the seam speaks only in plain terms; the library that finally carries the call is \
             an adapter's business, never this crate's. Got: {written}"
        );
    }
}

#[test]
fn the_seam_carries_a_structural_request_record_in_and_response_record_out() {
    let written = dart_http_client_of(DART_HTTP_SERVICE);
    assert!(
        written.contains(
            "Future<({int status, List<(String, String)> headers, List<int> body})> send(\n    \
             ({String method, String path, String query, List<(String, String)> headers, List<int> body}) request,\n  \
             );"
        ),
        "the request and response are records, not named classes, so every service's transport \
         reads the exact same anonymous shape. Got: {written}"
    );
}

#[test]
fn a_bodied_operation_fills_a_literal_path_and_serializes_the_message_as_the_body() {
    let written = dart_http_client_of(DART_HTTP_SERVICE);
    let method = method_body(&written, "createDocument");
    assert!(
        method.contains("path += '/documents';"),
        "a path with no placeholder is pushed exactly as written. Got: {method}"
    );
    assert!(
        method.contains("const query = '';"),
        "a bodied method carries no query string. Got: {method}"
    );
    assert!(
        method.contains("final body = utf8.encode(jsonEncode(req.toJson()));"),
        "the body is the message's own JSON codec, never re-derived. Got: {method}"
    );
    assert!(method.contains("method: 'POST'"), "got: {method}");
}

#[test]
fn a_path_placeholder_is_filled_by_exact_segment_substitution() {
    let written = dart_http_client_of(DART_HTTP_SERVICE);
    let method = method_body(&written, "getVersion");
    assert!(
        method.contains("path += '/documents/';")
            && method.contains("path += Uri.encodeComponent('${req.document_id}');")
            && method.contains("path += '/versions/';")
            && method.contains("path += Uri.encodeComponent('${req.version_id}');"),
        "each segment is pushed in template order, a placeholder reading its own field off the \
         message under its own written spelling. Got: {method}"
    );
}

#[test]
fn a_header_in_binding_becomes_an_extra_parameter_and_a_built_header() {
    let written = dart_http_client_of(DART_HTTP_SERVICE);
    assert!(
        written.contains("getVersion(GetVersionRequest req, String? byte_range)"),
        "the client method spells the extra parameter beside the message, under its own Rust \
         spelling. Got: {written}"
    );
    let method = method_body(&written, "getVersion");
    assert!(
        method.contains(
            "final headers = <(String, String)>[('range', (byte_range == null ? '' : \
             '${byte_range!}'))];"
        ),
        "the header is built from the extra argument, never from the message. Got: {method}"
    );
}

#[test]
fn a_bodyless_operation_with_no_header_in_builds_an_empty_header_list() {
    let written = dart_http_client_of(DART_HTTP_SERVICE);
    let method = method_body(&written, "createDocument");
    assert!(
        method.contains("const headers = <(String, String)>[];"),
        "got: {method}"
    );
}

#[test]
fn unbound_optional_fields_build_a_query_string_a_vec_joining_by_comma() {
    let written = dart_http_client_of(DART_HTTP_SERVICE);
    let method = method_body(&written, "searchDocuments");
    assert!(
        method.contains("final value = req.q;")
            && method.contains("queryParts.add('q=' + Uri.encodeComponent('${value}'));"),
        "a scalar optional field pushes its own wire key when present. Got: {method}"
    );
    assert!(
        method.contains("final value = req.tags;")
            && method.contains(
                "queryParts.add('tags=' + Uri.encodeComponent((value).map((e) => \
                 '${e}').join(\",\")));"
            ),
        "a Vec field joins its elements with a comma before it is percent-encoded. Got: {method}"
    );
    assert!(
        method.contains("final query = queryParts.join('&');"),
        "got: {method}"
    );
}

#[test]
fn the_declared_ok_status_decodes_into_the_success_type() {
    let written = dart_http_client_of(DART_HTTP_SERVICE);
    let method = method_body(&written, "createDocument");
    assert!(
        method.contains("if (status == 200) {")
            && method.contains(
                "value = CreateDocumentResponse.fromJson(jsonDecode(utf8.decode(response.body)));"
            )
            && method.contains("return value;"),
        "got: {method}"
    );
}

#[test]
fn a_mapped_error_status_decodes_into_the_declared_error_and_is_thrown() {
    let written = dart_http_client_of(DART_HTTP_SERVICE);
    let method = method_body(&written, "createDocument");
    assert!(
        method.contains("if (status == 409) {")
            && method.contains(
                "declared = CreateDocumentError.fromJson(jsonDecode(utf8.decode(response.body)));"
            )
            && method.contains(
                "throw DocumentClientServiceHttpError<CreateDocumentError>.declared(declared);"
            ),
        "got: {method}"
    );
}

#[test]
fn a_fixed_fault_status_decodes_into_a_fault_reusing_the_generated_fault_fields_codec() {
    let written = dart_http_client_of(DART_HTTP_SERVICE);
    let method = method_body(&written, "createDocument");
    assert!(
        method.contains("if (status == 400 || status == 404 || status == 500) {")
            && method.contains(
                "throw DocumentClientServiceHttpError<CreateDocumentError>.fault(_documentClientServiceHttpFaultFromBody('create-document', response.body));"
            ),
        "got: {method}"
    );
    assert!(
        written.contains(
            "return DocumentClientServiceFaultFields.fromJson(jsonDecode(utf8.decode(body)));"
        ),
        "the fault body decodes through the same generated `FaultFields` codec every other \
         surface answers faults through, rather than a hand-rolled parse. Got: {written}"
    );
}

#[test]
fn an_unexpected_status_decodes_into_an_undeserializable_payload_fault() {
    let written = dart_http_client_of(DART_HTTP_SERVICE);
    let method = method_body(&written, "createDocument");
    assert!(
        method.contains(
            "_documentClientServiceHttpUndeserializablePayload('create-document', 'an unexpected status ($status) answered')"
        ),
        "got: {method}"
    );
}

#[test]
fn an_operation_naming_no_http_group_defaults_to_post_its_own_wire_name_and_the_fixed_error_status()
{
    let written = dart_http_client_of(DART_HTTP_SERVICE);
    let method = method_body(&written, "sweepDocuments");
    assert!(
        method.contains("path += '/sweep-documents';") && method.contains("method: 'POST'"),
        "got: {method}"
    );
    assert!(
        method.contains("if (status == 422) {"),
        "an operation naming no `error_status` table maps every declared error to the fixed \
         binding-error status. Got: {method}"
    );
}

#[test]
fn a_header_out_tuple_success_reads_the_body_and_the_header_back() {
    let written = dart_http_client_of(DART_HTTP_SERVICE);
    let method = method_body(&written, "getVersion");
    assert!(
        method
            .contains("value = VersionResponse.fromJson(jsonDecode(utf8.decode(response.body)));"),
        "got: {method}"
    );
    assert!(
        method.contains("final rawHeaderOut0 = _findHeader(response.headers, 'etag');")
            && method.contains("if (rawHeaderOut0 == null) {")
            && method.contains("final headerOut0 = rawHeaderOut0;")
            && method.contains("return (value, headerOut0);"),
        "the response header is read back and joined onto the decoded body as a record. Got: \
         {method}"
    );
    assert!(
        written.contains("Future<(VersionResponse, String)> getVersion("),
        "the method's own return type is the Dart record the success tuple describes. Got: \
         {written}"
    );
}

#[test]
fn a_no_payload_one_way_operation_resolves_on_its_declared_status_without_reading_a_body() {
    let written = dart_http_client_of(DART_HTTP_SERVICE);
    assert!(
        written.contains("Future<void> purgeDocument(String req) async {"),
        "got: {written}"
    );
    let method = method_body(&written, "purgeDocument");
    assert!(
        method.contains("path += Uri.encodeComponent('${req}');"),
        "a `Named` message answering to its one placeholder is the value itself. Got: {method}"
    );
    assert!(
        method.contains("if (status == 204) {\n      return;\n    }"),
        "a one-way operation resolves on its declared (default 204) status with no body read. \
         Got: {method}"
    );
    assert!(
        method.contains("throw DocumentClientServiceHttpRefusal("),
        "a one-way operation throws a fault-only refusal, having no declared error to carry one \
         in. Got: {method}"
    );
}

#[test]
fn a_bytes_operation_reads_the_body_and_content_type_back() {
    let written = dart_http_client_of(DART_HTTP_SERVICE);
    assert!(
        written.contains("Future<(List<int>, String)> getThumbnail(String req) async {"),
        "a `body = \"bytes\"` operation's return type is the byte list and its content type, no \
         `header_out` involved. Got: {written}"
    );
    let method = method_body(&written, "getThumbnail");
    assert!(
        method.contains("final contentType = _findHeader(response.headers, 'content-type') ?? '';")
            && method.contains("return (response.body, contentType);"),
        "no `jsonDecode` runs on a bytes body — it is read bare, and the content type is read \
         back from the response header. Got: {method}"
    );
}

#[test]
fn the_client_class_holds_one_constructor_and_every_operation_as_a_method() {
    let written = dart_http_client_of(DART_HTTP_SERVICE);
    assert!(
        written.contains("class DocumentClientServiceHttpClient {")
            && written.contains("DocumentClientServiceHttpClient(this._transport);")
            && written.contains("final DocumentClientServiceHttpTransport _transport;"),
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
