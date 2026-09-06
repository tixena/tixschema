//! The `http_rest` TypeScript client, read off the emitted text.
//!
//! What these prove and what they cannot: the structure of the emitted TypeScript — the seam's own
//! shape, how a path, a query string and a request header get built, how a status decodes into a
//! success, a declared error or a fault. No TypeScript toolchain is reachable here, so none of them
//! type-checks the bundle.

use super::{MIXED_HTTP_SERVICE, http_client_of};

#[test]
fn exactly_one_seam_type_is_emitted_and_it_names_no_http_library() {
    let written = http_client_of(MIXED_HTTP_SERVICE);
    assert_eq!(
        written
            .matches("export type DocumentClientServiceHttpTransport = {")
            .count(),
        1,
        "one seam type serves every operation on the service. Got: {written}"
    );
    for named in ["fetch", "axios", "Fetch", "Axios"] {
        assert!(
            !written.contains(named),
            "the seam speaks only in plain terms; the library that finally carries the call is \
             an adapter's business, never this crate's. Got: {written}"
        );
    }
}

#[test]
fn the_seam_carries_a_request_record_in_and_a_response_record_out() {
    let written = http_client_of(MIXED_HTTP_SERVICE);
    let found = written
        .split("export type DocumentClientServiceHttpTransport = {")
        .nth(1)
        .and_then(|rest| rest.split_once("\n};"))
        .map(|(body, _)| body.to_owned());
    assert!(found.is_some(), "got: {written}");
    let seam = found.unwrap();
    assert!(
        seam.contains("send(request: {")
            && seam.contains("method: string;")
            && seam.contains("path: string;")
            && seam.contains("query: string;")
            && seam.contains("headers: ReadonlyArray<readonly [string, string]>;")
            && seam.contains("body: string;"),
        "got: {seam}"
    );
    assert!(
        seam.contains("}): Promise<{") && seam.contains("status: number;"),
        "got: {seam}"
    );
}

#[test]
fn a_bodied_operation_fills_a_literal_path_and_serializes_the_validated_message_as_the_body() {
    let written = http_client_of(MIXED_HTTP_SERVICE);
    let method = method_body(&written, "createDocument");
    assert!(
        method.contains("path += \"/documents\";"),
        "a path with no placeholder is pushed exactly as written. Got: {method}"
    );
    assert!(
        method.contains("const query = \"\";"),
        "a bodied method carries no query string. Got: {method}"
    );
    assert!(
        method.contains("const body = JSON.stringify(sending);"),
        "the body is the validated message, not the raw argument. Got: {method}"
    );
    assert!(method.contains("method: \"POST\","), "got: {method}");
}

#[test]
fn a_path_placeholder_is_filled_by_exact_segment_substitution() {
    let written = http_client_of(MIXED_HTTP_SERVICE);
    let method = method_body(&written, "getVersion");
    assert!(
        method.contains("path += \"/documents/\";")
            && method.contains("path += encodeURIComponent(String(sending.document_id));")
            && method.contains("path += \"/versions/\";")
            && method.contains("path += encodeURIComponent(String(sending.version_id));"),
        "each segment is pushed in template order, a placeholder reading its own field off the \
         validated message rather than splitting a shared prefix. Got: {method}"
    );
}

#[test]
fn a_header_in_binding_becomes_an_extra_argument_and_a_built_header() {
    let written = http_client_of(MIXED_HTTP_SERVICE);
    assert!(
        written.contains("getVersion(req: GetVersionRequest, byteRange: string | undefined)"),
        "the client type spells the extra argument beside the message. Got: {written}"
    );
    let method = method_body(&written, "getVersion");
    assert!(
        method
            .contains("const headers: Array<[string, string]> = [[\"range\", String(byteRange)]];"),
        "the header is built from the extra argument, never from the message. Got: {method}"
    );
}

#[test]
fn a_bodyless_operation_with_no_header_in_builds_an_empty_header_array() {
    let written = http_client_of(MIXED_HTTP_SERVICE);
    let method = method_body(&written, "createDocument");
    assert!(
        method.contains("const headers: Array<[string, string]> = [];"),
        "got: {method}"
    );
}

#[test]
fn the_declared_ok_status_decodes_into_the_success_type() {
    let written = http_client_of(MIXED_HTTP_SERVICE);
    let method = method_body(&written, "createDocument");
    assert!(
        method.contains("if (status === 200) {")
            && method.contains("value = JSON.parse(response.body) as CreateDocumentResponse;")
            && method.contains("return { ok: true, value };"),
        "got: {method}"
    );
}

#[test]
fn a_mapped_status_decodes_into_the_declared_error_type() {
    let written = http_client_of(MIXED_HTTP_SERVICE);
    let method = method_body(&written, "createDocument");
    assert!(
        method.contains("if (status === 409) {")
            && method.contains("declared = JSON.parse(response.body) as CreateDocumentError;")
            && method.contains("return { ok: false, error: declared };"),
        "got: {method}"
    );
}

#[test]
fn a_fixed_fault_status_decodes_through_the_shared_fault_reader() {
    let written = http_client_of(MIXED_HTTP_SERVICE);
    let method = method_body(&written, "createDocument");
    assert!(
        method.contains("if (status === 400 || status === 404 || status === 500) {")
            && method.contains(
                "fault: documentClientServiceHttpFaultFromBody(\"create-document\", response.body),"
            ),
        "got: {method}"
    );
}

#[test]
fn a_status_naming_no_declared_or_fixed_outcome_becomes_a_typed_undeserializable_payload_fault() {
    let written = http_client_of(MIXED_HTTP_SERVICE);
    let method = method_body(&written, "createDocument");
    assert!(
        method.contains(
            "fault: documentClientServiceHttpUndeserializablePayload(\n            \
             \"create-document\",\n            \
             `an unexpected status (${status}) answered`,\n          \
             ),"
        ),
        "got: {method}"
    );
}

#[test]
fn an_operation_naming_no_http_group_defaults_to_post_and_the_fixed_binding_error_status() {
    let written = http_client_of(MIXED_HTTP_SERVICE);
    let method = method_body(&written, "sweepDocuments");
    assert!(
        method.contains("method: \"POST\","),
        "an operation naming no `http(...)` group defaults to `POST /{{wire-name}}`. \
         Got: {method}"
    );
    assert!(
        method.contains("path += \"/sweep-documents\";"),
        "got: {method}"
    );
    assert!(
        method.contains("if (status === 422) {")
            && method.contains("declared = JSON.parse(response.body) as SweepError;"),
        "an operation naming no `http(...)` group declares no `error_status` table either, so \
         its declared error answers at the fixed binding-error status. Got: {method}"
    );
}

#[test]
fn a_tuple_success_reads_its_header_out_element_back_off_the_response() {
    let written = http_client_of(MIXED_HTTP_SERVICE);
    let method = method_body(&written, "getVersion");
    assert!(
        method.contains(
            "const rawHeaderOut0 = response.headers.find(\n          \
             ([name]) => name.toLowerCase() === \"etag\",\n        \
             );"
        ),
        "got: {method}"
    );
    assert!(
        method.contains("if (rawHeaderOut0 === undefined) {")
            && method.contains("\"a declared response header was missing\","),
        "a missing declared header is a defect, not a silently absent value. Got: {method}"
    );
    assert!(
        method.contains("const headerOut0 = rawHeaderOut0[1] as string;")
            && method.contains("return { ok: true, value: [value, headerOut0] };"),
        "the body and the header ride the same tuple the result type declares. Got: {method}"
    );
}

#[test]
fn a_no_payload_operation_resolves_on_its_declared_status_and_reads_no_body() {
    let written = http_client_of(MIXED_HTTP_SERVICE);
    let method = method_body(&written, "purgeDocument");
    assert!(
        method.contains("if (status === 204) {") && method.contains("return;"),
        "got: {method}"
    );
    assert!(
        !method.contains("JSON.parse(response.body)"),
        "a one-way method's success case has no body to decode. Got: {method}"
    );
}

#[test]
fn a_one_way_operation_answers_promise_void_and_throws_instead_of_returning_a_fault() {
    let written = http_client_of(MIXED_HTTP_SERVICE);
    assert!(
        written.contains("purgeDocument(req: string): Promise<void>;"),
        "got: {written}"
    );
    let method = method_body(&written, "purgeDocument");
    assert!(
        method.contains(
            "throw documentClientServiceHttpRefused(\n          \
             documentClientServiceHttpOutboundFault(\"purge-document\", validated.error.issues),\n        \
             );"
        ),
        "a refused outbound message throws before the transport is ever reached. Got: {method}"
    );
    assert!(
        method.contains(
            "throw documentClientServiceHttpRefused(\n          \
             documentClientServiceHttpTransportFailure(\"purge-document\", String(uncarried)),\n        \
             );"
        ),
        "got: {method}"
    );
}

#[test]
fn a_named_message_answering_to_one_placeholder_is_read_back_as_the_whole_value() {
    let written = http_client_of(MIXED_HTTP_SERVICE);
    let method = method_body(&written, "purgeDocument");
    assert!(
        method.contains("path += encodeURIComponent(String(sending));"),
        "a `Named` message with exactly one placeholder is that placeholder's own value, not a \
         field read off it. Got: {method}"
    );
}

#[test]
fn every_method_validates_before_it_ever_names_the_transport() {
    let written = http_client_of(MIXED_HTTP_SERVICE);
    for (call, schema) in [
        ("createDocument", "CreateDocumentRequest$Schema"),
        ("getVersion", "GetVersionRequest$Schema"),
    ] {
        let method = method_body(&written, call);
        let checked = method.find(&format!("{schema}.safeParse(req)"));
        let reached = method.find("transport.send(");
        assert!(checked.is_some() && reached.is_some(), "got: {method}");
        assert!(
            checked.unwrap() < reached.unwrap(),
            "the transport is reached only once the message has passed its own validator. \
             Got: {method}"
        );
    }
}

#[test]
fn the_factory_binds_a_transport_and_answers_with_the_client() {
    let written = http_client_of(MIXED_HTTP_SERVICE);
    assert!(
        written.contains(
            "export function createDocumentClientServiceHttpClient(\n  \
             transport: DocumentClientServiceHttpTransport,\n\
             ): DocumentClientServiceHttpClient {"
        ),
        "got: {written}"
    );
}

/// One method's body, read out of the factory's own object literal by the call it is declared
/// under.
fn method_body(written: &str, call: &str) -> String {
    let found = written
        .split(&format!("async {call}("))
        .nth(1)
        .and_then(|rest| rest.split_once("\n},"))
        .map(|(body, _)| body.to_owned());
    assert!(found.is_some(), "no `{call}` method found. Got: {written}");
    found.unwrap()
}
