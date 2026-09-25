//! The `http_rest` Kotlin client, read off the emitted text — twin of `dart_http_client_tests.rs`.
//!
//! No Kotlin toolchain is reachable in `cargo test`, so nothing here compiles the emitted source;
//! these tests read structure, the same way `dart_http_client_tests.rs` reads Dart's own output. A
//! live kotlinc compile-and-run pass covers the `KOTLIN_SINGLE_PLACEHOLDER_HTTP_SERVICE` shape
//! separately, outside this crate's own test suite.

use super::{
    KOTLIN_BYTES_HEADER_OUT_SERVICE, KOTLIN_HTTP_SERVICE, KOTLIN_MULTIPART_HTTP_SERVICE,
    KOTLIN_NUMERIC_HEADER_OUT_SERVICE, KOTLIN_SINGLE_PLACEHOLDER_HTTP_SERVICE,
    KOTLIN_STREAM_HTTP_SERVICE, KOTLIN_UNIT_SUCCESS_HTTP_SERVICE, kotlin_http_client_of,
};

/// The body of one method, from its own doc comment through the closing brace of the method
/// following it (or the end of the class) — mirrors `dart_http_client_tests`'s own `method_body`.
fn method_body<'written>(written: &'written str, call: &str) -> &'written str {
    let start = written.find(&format!(" fun {call}("));
    assert!(start.is_some(), "no method named `{call}` in: {written}");
    let rest = &written[start.unwrap()..];
    let end = rest.find("\n\n").unwrap_or(rest.len());
    &rest[..end]
}

#[test]
fn a_reply_method_answers_the_sealed_result_and_a_one_way_method_answers_nothing() {
    let written = kotlin_http_client_of(KOTLIN_HTTP_SERVICE);
    assert!(
        written.contains(
            "suspend fun createDocument(req: CreateDocumentRequest): DocumentClientServiceCreateDocumentResult"
        ),
        "got: {written}"
    );
    assert!(
        written.contains("suspend fun purgeDocument(req: String) {"),
        "a one-way method still answers plainly (no declared return type). Got: {written}"
    );
    let reply_method = method_body(&written, "createDocument");
    assert!(
        !reply_method.contains("throw DocumentClientServiceRefusal"),
        "a reply method answers the sealed result rather than throwing a refusal — only a \
         one-way method does that. Got: {reply_method}"
    );
}

#[test]
fn exactly_one_seam_type_is_emitted_and_it_names_no_networking_library() {
    let written = kotlin_http_client_of(KOTLIN_HTTP_SERVICE);
    assert_eq!(
        written
            .matches("interface DocumentClientServiceHttpTransport {")
            .count(),
        1,
        "one seam type serves every operation on the service. Got: {written}"
    );
    for named in ["OkHttp", "Ktor", "okhttp3", "import "] {
        assert!(
            !written.contains(named),
            "the seam speaks only in plain terms; the library that finally carries the call is \
             an adapter's business, never this crate's. Got: {written}"
        );
    }
}

#[test]
fn the_seam_carries_named_request_and_response_data_classes() {
    let written = kotlin_http_client_of(KOTLIN_HTTP_SERVICE);
    assert!(
        written.contains(
            "data class DocumentClientServiceHttpRequest(\n  \
             val method: String,\n  \
             val path: String,\n  \
             val query: String,\n  \
             val headers: List<Pair<String, String>>,\n  \
             val body: ByteArray,\n)"
        ),
        "got: {written}"
    );
    assert!(
        written.contains(
            "interface DocumentClientServiceHttpTransport {\n  \
             suspend fun send(request: DocumentClientServiceHttpRequest): DocumentClientServiceHttpResponse\n}"
        ),
        "got: {written}"
    );
}

#[test]
fn a_bodied_operation_fills_a_literal_path_and_serializes_the_message_as_the_body() {
    let written = kotlin_http_client_of(KOTLIN_HTTP_SERVICE);
    let method = method_body(&written, "createDocument");
    assert!(
        method.contains("path += \"/documents\""),
        "a path with no placeholder is pushed exactly as written. Got: {method}"
    );
    assert!(
        method.contains("val query = \"\""),
        "a bodied method carries no query string. Got: {method}"
    );
    assert!(
        method.contains(
            "val body = Json.encodeToString(serializer<CreateDocumentRequest>(), req).encodeToByteArray()"
        ),
        "the body is the message's own JSON codec, never re-derived. Got: {method}"
    );
    assert!(method.contains("method = \"POST\""), "got: {method}");
}

#[test]
fn a_path_placeholder_is_filled_by_exact_segment_substitution() {
    let written = kotlin_http_client_of(KOTLIN_HTTP_SERVICE);
    let method = method_body(&written, "getVersion");
    assert!(
        method.contains("path += \"/documents/\"")
            && method
                .contains("path += documentClientServiceHttpPercentEncode(\"${req.documentId}\")")
            && method.contains("path += \"/versions/\"")
            && method
                .contains("path += documentClientServiceHttpPercentEncode(\"${req.versionId}\")"),
        "each segment is pushed in template order, a placeholder reading its own property off the \
         message. Got: {method}"
    );
}

#[test]
fn a_lone_placeholder_on_an_author_s_own_message_reads_the_field_it_names() {
    let written = kotlin_http_client_of(KOTLIN_SINGLE_PLACEHOLDER_HTTP_SERVICE);
    let method = method_body(&written, "window");
    assert!(
        method.contains(
            "path += conversationClientServiceHttpPercentEncode(\"${req.conversationId}\")"
        ),
        "a message the author declared is a class whatever the path is shaped like, so the one \
         placeholder reads the field it names off it. Got: {method}"
    );
}

#[test]
fn a_mapped_status_answers_declared_and_a_transport_throw_answers_fault() {
    let written = kotlin_http_client_of(KOTLIN_SINGLE_PLACEHOLDER_HTTP_SERVICE);
    let method = method_body(&written, "window");
    assert!(
        method.contains("if (status == 404) {") && method.contains(".Declared("),
        "a mapped status decodes the declared error. Got: {method}"
    );
    assert!(
        method.contains("catch (thrown: Throwable)")
            && method.contains("conversationClientServiceHttpTransportFailure"),
        "a transport throw answers a fault, never propagates. Got: {method}"
    );
}

#[test]
fn a_one_way_method_throws_only_the_refusal() {
    let written = kotlin_http_client_of(KOTLIN_SINGLE_PLACEHOLDER_HTTP_SERVICE);
    assert!(
        written.contains(
            "class ConversationClientServiceRefusal(val fault: ConversationClientServiceFaultFields) : Exception(fault.detail)"
        ),
        "got: {written}"
    );
    let method = method_body(&written, "purgeConversation");
    assert!(
        method.contains("throw ConversationClientServiceRefusal("),
        "got: {method}"
    );
    assert!(
        !method.contains(".Fault(") && !method.contains(".Declared("),
        "a one-way operation has no sealed result to answer with. Got: {method}"
    );
}

#[test]
fn a_bytes_operation_composes_content_type_and_header_out_into_a_named_class() {
    let written = kotlin_http_client_of(KOTLIN_BYTES_HEADER_OUT_SERVICE);
    assert!(
        written.contains(
            "data class ThumbnailClientServiceGetThumbnailResultBytes(val body: ByteArray, val contentType: String, val headerOut0: String)"
        ),
        "got: {written}"
    );
    let method = method_body(&written, "getThumbnail");
    assert!(
        method.contains("thumbnailClientServiceHttpFindHeader(response.headers, \"content-type\")"),
        "got: {method}"
    );
    assert!(
        method
            .contains("thumbnailClientServiceHttpFindHeader(response.headers, \"x-document-id\")"),
        "got: {method}"
    );
    assert!(
        method.contains("val headerOut0 = rawHeaderOut0\n")
            && !method.contains("headerOut0 = rawHeaderOut0 ?:"),
        "a string-shaped `header_out` element is already non-null, so it reads with no elvis for \
         kotlinc to warn is unconditionally true. Got: {method}"
    );
}

#[test]
fn a_numeric_header_out_element_parses_with_its_own_declared_type() {
    let written = kotlin_http_client_of(KOTLIN_NUMERIC_HEADER_OUT_SERVICE);
    let method = method_body(&written, "getMetric");
    for (raw, conversion) in [
        ("rawHeaderOut0", "toUIntOrNull"),
        ("rawHeaderOut1", "toIntOrNull"),
        ("rawHeaderOut2", "toFloatOrNull"),
    ] {
        assert!(
            method.contains(&format!("({raw}).{conversion}()")),
            "got: {method}"
        );
    }
    assert!(
        !method.contains("toLong()") && !method.contains("toDouble()"),
        "no numeric header element still parses through the old blanket conversion. Got: {method}"
    );
    assert!(
        method.matches(
            "metricClientServiceHttpUndeserializablePayload(\"get-metric\", \"a response header did not match its declared type\")"
        ).count() >= 3,
        "a present header that will not decode as its declared type answers the fault the Rust \
         client answers, rather than throwing. Got: {method}"
    );
}

#[test]
fn a_stream_operation_answers_a_flow_and_a_content_range() {
    let written = kotlin_http_client_of(KOTLIN_STREAM_HTTP_SERVICE);
    assert!(
        written.contains("val bodyStream: Flow<ByteArray> = emptyFlow()"),
        "got: {written}"
    );
    assert!(
        written.contains(
            "data class ContentClientServiceGetFileResultStreamed(val contentRange: String?, val body: Flow<ByteArray>)"
        ),
        "got: {written}"
    );
    let method = method_body(&written, "getFile");
    assert!(
        method.contains("if (status == 206) {") && method.contains("if (status == 200) {"),
        "both the partial and the full answer decode the streamed record. Got: {method}"
    );
}

#[test]
fn a_multipart_operation_builds_named_parts() {
    let written = kotlin_http_client_of(KOTLIN_MULTIPART_HTTP_SERVICE);
    assert!(
        written.contains("val parts: List<Pair<String, Any?>> = emptyList()"),
        "got: {written}"
    );
    let method = method_body(&written, "uploadDocument");
    assert!(
        method.contains("parts.add(\"title\" to \"${req.title}\")"),
        "got: {method}"
    );
    assert!(
        method.contains("req.description?.let { parts.add(\"description\" to \"${it}\") }"),
        "an absent optional part is added nowhere rather than as empty text. Got: {method}"
    );
    assert!(
        method.contains("parts.add(\"file\" to attachment)"),
        "got: {method}"
    );
}

#[test]
fn a_unit_success_answers_a_bare_ok_object() {
    let written = kotlin_http_client_of(KOTLIN_UNIT_SUCCESS_HTTP_SERVICE);
    assert!(
        written.contains("data object Ok : PingClientServicePingResult"),
        "a unit success carries no value, so `Ok` is an object rather than a data class holding \
         one. Got: {written}"
    );
}
