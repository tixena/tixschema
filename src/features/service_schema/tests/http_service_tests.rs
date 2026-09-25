//! `ts_http_service()`, read off the emitted text.
//!
//! What these prove and what they cannot: the structure of the emitted TypeScript -- the route
//! table, the request and response shapes, the fault handler and its default, the three helpers,
//! and the dispatcher's own message assembly and status mapping. No TypeScript toolchain is
//! reachable here, so none of them type-checks the bundle;
//! `tests/service_schema_typescript_tests/type_check.rs` is what proves a complete implementation
//! compiles against the emitted dispatcher.

use super::{
    BYTES_HTTP_SERVICE, EMITTED_CLIENT_TEST_SERVICE, MIXED_HTTP_SERVICE, MULTIPART_HTTP_SERVICE,
    PATH_BOUND_HTTP_SERVICE, QUERY_HTTP_SERVICE, REQUIRED_HEADER_HTTP_SERVICE, STREAM_HTTP_SERVICE,
    TS_UNIT_SUCCESS_SERVICE, http_service_of,
};
use crate::utils::record_wire_scalar;

/// The expected emitted dispatcher text: the error status is read through the declared enum's
/// own generated `WindowError$Variant` reader rather than an `errorCode` cast.
const EXPECTED: &str = "/** One operation's method, path template and status table, for an adapter that registers a handler per route. */
export type ConversationClientServiceHttpRoute = {
  errorStatuses: ReadonlyArray<number>;
  method: string;
  okStatus: number;
  operation: string;
  path: string;
};

/** The declared route table, in declaration order. `{field}` placeholders are written as declared. */
export const conversationClientServiceHttpRoutes: ReadonlyArray<ConversationClientServiceHttpRoute> = [
  { errorStatuses: [], method: \"DELETE\", okStatus: 204, operation: \"purge-conversation\", path: \"/v1/conversations/{conversation_id}\" },
  { errorStatuses: [404], method: \"GET\", okStatus: 200, operation: \"window\", path: \"/v1/conversations/{conversation_id}/window\" },
];

/** One HTTP request in plain terms. The body is undecoded bytes, as the Rust `IncomingRequest` carries it. */
export type ConversationClientServiceHttpRequest = {
  body: Uint8Array;
  headers: ReadonlyArray<readonly [string, string]>;
  method: string;
  path: string;
  query: string;
};

/** One HTTP response in plain terms, for an adapter to write back however its framework answers. */
export type ConversationClientServiceHttpResponse = {
  body: Uint8Array;
  headers: ReadonlyArray<readonly [string, string]>;
  status: number;
};

/** Decides what one fault answers with. The default: 404 unknown-operation, 500 handler-panic, 400 otherwise, the fault as JSON. */
export type ConversationClientServiceHttpFaultHandler = (fault: ConversationClientServiceFault) => ConversationClientServiceHttpResponse;

function conversationClientServiceHttpJson(status: number, headers: ReadonlyArray<readonly [string, string]>, value: unknown): ConversationClientServiceHttpResponse {
  return { status, headers: [...headers, [\"content-type\", \"application/json\"]], body: new TextEncoder().encode(JSON.stringify(value)) };
}

export function conversationClientServiceHttpDefaultFaultHandler(fault: ConversationClientServiceFault): ConversationClientServiceHttpResponse {
  const status = fault.kind === \"unknown-operation\" ? 404 : fault.kind === \"handler-panic\" ? 500 : 400;
  return conversationClientServiceHttpJson(status, [], fault);
}

function conversationClientServiceHttpFault(kind: ConversationClientServiceFaultFields[\"kind\"], operation: string, detail: string, field?: string): ConversationClientServiceFault {
  const built: ConversationClientServiceFaultFields = { detail, field, kind, operation };
  return built as ConversationClientServiceFault;
}

/** Mirrors the Rust `match_path`: a literal token is stripped as a prefix, a placeholder takes one non-empty segment, and nothing may remain. */
function conversationClientServiceHttpMatchPath(template: ReadonlyArray<string | null>, path: string): Array<string> | undefined {
  let rest = path;
  const captured: Array<string> = [];
  for (const token of template) {
    if (token !== null) {
      if (!rest.startsWith(token)) return undefined;
      rest = rest.slice(token.length);
    } else {
      const end = rest.indexOf(\"/\") === -1 ? rest.length : rest.indexOf(\"/\");
      const value = rest.slice(0, end);
      if (value === \"\") return undefined;
      captured.push(value);
      rest = rest.slice(end);
    }
  }
  return rest === \"\" ? captured : undefined;
}

/** Mirrors the Rust `parse_query`: split on `&`, split each pair once on `=`, percent-decode both halves. */
function conversationClientServiceHttpParseQuery(raw: string): Map<string, string> {
  const parsed = new Map<string, string>();
  for (const pair of raw.split(\"&\")) {
    if (pair === \"\") continue;
    const at = pair.indexOf(\"=\");
    const [key, value] = at === -1 ? [pair, \"\"] : [pair.slice(0, at), pair.slice(at + 1)];
    parsed.set(decodeURIComponent(key), decodeURIComponent(value));
  }
  return parsed;
}

/** The Rust `decode_expr` coercion for a numeric argument: an integer, else a float, else the text itself. */
function conversationClientServiceHttpCoerceNumber(raw: string): unknown {
  if (/^-?\\d+$/.test(raw)) return Number(raw);
  const asFloat = Number(raw);
  return raw.trim() !== \"\" && Number.isFinite(asFloat) ? asFloat : raw;
}

/**
 * Turns an implementation into the function an adapter drives it with: one request in, one
 * response out. Matches the route table in declaration order, assembles the operation's message
 * exactly as the Rust dispatcher does, parses it through the generated dispatcher (which checks
 * it against its schema), and maps the outcome to a status and a body.
 */
export function createConversationClientServiceHttpDispatcher<Ctx>(
  impl: ConversationClientServiceImpl<Ctx>,
  onFault: ConversationClientServiceHttpFaultHandler = conversationClientServiceHttpDefaultFaultHandler,
): (ctx: Ctx, request: ConversationClientServiceHttpRequest) => Promise<ConversationClientServiceHttpResponse> {
  const dispatch = createConversationClientServiceDispatcher(impl);
  const answer = async (ctx: Ctx, request: ConversationClientServiceHttpRequest, operation: string, payload: unknown, okStatus: number, errorStatus: (error: unknown) => number) => {
    let answered: unknown;
    try {
      answered = await dispatch(ctx, operation, payload, request.headers);
    } catch (thrown) {
      return onFault(conversationClientServiceHttpFault(\"handler-panic\", operation, thrown instanceof Error ? thrown.message : String(thrown)));
    }
    if (answered === undefined) return { status: okStatus, headers: [], body: new Uint8Array() };
    const envelope = answered as { ok: true; value: unknown } | { ok: false; error: unknown };
    if (envelope.ok) return conversationClientServiceHttpJson(okStatus, [], envelope.value);
    const error = envelope.error as { isServiceFault?: true; fault?: ConversationClientServiceFault };
    if (typeof error === \"object\" && error !== null && error.isServiceFault === true && error.fault !== undefined) return onFault(error.fault);
    return conversationClientServiceHttpJson(errorStatus(envelope.error), [], envelope.error);
  };
  return async (ctx, request) => {
    const { method, path } = request;
    // purge-conversation: DELETE /v1/conversations/{conversation_id} \u{2014} the message IS the one placeholder (a wire scalar).
    if (method === \"DELETE\") {
      const captured = conversationClientServiceHttpMatchPath([\"/v1/conversations/\", null], path);
      if (captured !== undefined) {
        const [conversation_id] = captured;
        return answer(ctx, request, \"purge-conversation\", conversation_id, 204, () => 422);
      }
    }
    // window: GET /v1/conversations/{conversation_id}/window \u{2014} an author-declared message: the placeholder
    // is inserted under its written spelling as a string; a bodyless method reads no body (Rust reads no query here either).
    if (method === \"GET\") {
      const captured = conversationClientServiceHttpMatchPath([\"/v1/conversations/\", null, \"/window\"], path);
      if (captured !== undefined) {
        const [conversation_id] = captured;
        const message: Record<string, unknown> = {};
        message[\"conversation_id\"] = conversation_id;
        return answer(ctx, request, \"window\", message, 200, (error) => {
          switch (WindowError$Variant(error)) {
            case \"NotFound\": return 404;
            default: return 422;
          }
        });
      }
    }
    return onFault(conversationClientServiceHttpFault(\"unknown-operation\", `${method} ${path}`, \"the service answers to no route by that method and path\"));
  };
}";

#[test]
fn the_emitted_client_test_service_reproduces_the_design_document_verbatim() {
    record_wire_scalar("ConversationId");
    let written = http_service_of(EMITTED_CLIENT_TEST_SERVICE);
    assert_eq!(written, EXPECTED, "got:\n{written}");
}

#[test]
fn exported_names_follow_the_service_and_lower_camel_case_convention() {
    let written = http_service_of(MIXED_HTTP_SERVICE);
    for named in [
        "export type DocumentClientServiceHttpRoute",
        "export type DocumentClientServiceHttpRequest",
        "export type DocumentClientServiceHttpResponse",
        "export type DocumentClientServiceHttpFaultHandler",
        "export function createDocumentClientServiceHttpDispatcher",
        "export const documentClientServiceHttpRoutes",
        "export function documentClientServiceHttpDefaultFaultHandler",
    ] {
        assert!(written.contains(named), "got: {written}");
    }
}

#[test]
fn declaration_order_is_preserved_in_the_route_table_and_the_dispatcher_arms() {
    let written = http_service_of(MIXED_HTTP_SERVICE);
    let wires = [
        "create-document",
        "get-version",
        "purge-document",
        "sweep-documents",
    ];
    let table_positions: Vec<usize> = wires
        .iter()
        .map(|wire| written.find(&format!("operation: \"{wire}\"")).unwrap())
        .collect();
    assert!(
        table_positions.is_sorted(),
        "the table is in declaration order. Got: {written}"
    );
    let arm_positions: Vec<usize> = wires
        .iter()
        .map(|wire| {
            written
                .find(&format!("\"{wire}\", message"))
                .or_else(|| written.find(&format!(", \"{wire}\",")))
                .unwrap()
        })
        .collect();
    assert!(
        arm_positions.is_sorted(),
        "the arms answer in the same order the table lists them. Got: {written}"
    );
}

#[test]
fn both_shapes_carry_uint8array_bodies_and_the_requests_own_content_type_is_never_read() {
    let written = http_service_of(MIXED_HTTP_SERVICE);
    assert!(
        written.contains("HttpRequest = {\n  body: Uint8Array;")
            && written.contains("HttpResponse = {\n  body: Uint8Array;"),
        "got: {written}"
    );
    assert!(
        !written
            .contains("request.headers.find(([name]) => name.toLowerCase() === \"content-type\")"),
        "the dispatcher hands the request's own headers to `dispatch` unread, and never reads a \
         content-type off them itself. Got: {written}"
    );
}

#[test]
fn a_single_scalar_placeholder_message_is_the_placeholder_itself() {
    let written = http_service_of(MIXED_HTTP_SERVICE);
    assert!(
        written.contains(
            "return answer(ctx, request, \"purge-document\", document_id, 204, () => 422);"
        ),
        "got: {written}"
    );
}

#[test]
fn an_author_declared_message_takes_only_its_placeholders_as_strings() {
    let written = http_service_of(MIXED_HTTP_SERVICE);
    assert!(
        written.contains("message[\"document_id\"] = document_id;")
            && written.contains("message[\"version_id\"] = version_id;"),
        "got: {written}"
    );
    assert!(
        !written.contains("HttpParseQuery(request.query)"),
        "`get_version` is bodyless and reads no query, mirroring the Rust dispatcher's own drop. \
         Got: {written}"
    );
}

#[test]
fn a_generated_bodyless_message_reads_its_unbound_fields_off_the_query_with_their_own_coercion() {
    let written = http_service_of(QUERY_HTTP_SERVICE);
    assert!(
        written.contains("const queryMap = searchClientServiceHttpParseQuery(request.query);"),
        "got: {written}"
    );
    assert!(
        written.contains("message[\"category\"] = category;"),
        "a placeholder-bound field never touches the query. Got: {written}"
    );
    assert!(
        written.contains("message[\"limit\"] = raw === undefined ? null : searchClientServiceHttpCoerceNumber(raw);"),
        "a numeric field is coerced through the shared helper. Got: {written}"
    );
    assert!(
        written.contains("message[\"verbose\"] = raw === undefined ? null : (raw === \"true\" ? true : raw === \"false\" ? false : raw);"),
        "a boolean field is coerced inline, exactly as the Rust `decode_expr` reads one. Got: {written}"
    );
}

/// A macro-generated message with two or more fields, all bound by the path, carries no
/// `queryMap` at all - beside an operation that does read the query, which still carries one.
#[test]
fn a_fully_path_bound_generated_message_reads_no_query_beside_one_that_does() {
    let written = http_service_of(PATH_BOUND_HTTP_SERVICE);
    assert!(
        written.contains("message[\"org\"] = org;")
            && written.contains("message[\"documentId\"] = document_id;"),
        "both fields decode straight off the path. Got: {written}"
    );
    assert_eq!(
        written.matches("HttpParseQuery(request.query)").count(),
        1,
        "only `list_documents` reads the query; `get_document` binds both its fields from the \
         path. Got: {written}"
    );
}

#[test]
fn an_operation_with_no_error_status_table_answers_422_and_calls_no_reader() {
    let written = http_service_of(QUERY_HTTP_SERVICE);
    assert!(
        written.contains("return answer(ctx, request, \"search\", message, 200, () => 422);"),
        "got: {written}"
    );
    assert!(
        !written.contains("$Variant"),
        "no table means no variant to read. Got: {written}"
    );
}

/// Read and refused by the dispatcher itself, not this module — see `service_tests`.
#[test]
fn a_header_in_binding_is_read_by_the_dispatcher_not_by_this_server() {
    for source in [MIXED_HTTP_SERVICE, REQUIRED_HEADER_HTTP_SERVICE] {
        let written = http_service_of(source);
        assert!(
            written.contains("operation: \"get-version\""),
            "the route exists. Got: {written}"
        );
        assert!(
            !written.contains("byte_range") && !written.contains("byteRange"),
            "the header's own parameter is named nowhere in this module. Got: {written}"
        );
        assert!(
            !written.contains("a required header was not carried"),
            "this server keeps no presence check of its own: the dispatcher refuses a missing \
             required header. Got: {written}"
        );
        assert!(written.contains("dispatch(ctx, "), "got: {written}");
    }
}

/// Mirrors the Rust `bytes_answer_block`.
#[test]
fn a_bytes_reply_answers_the_raw_bytes_with_their_content_type_and_declared_headers() {
    let written = http_service_of(BYTES_HTTP_SERVICE);
    assert!(
        written.contains(
            "const [bytes, contentType, headerOut0] = envelope.value;"
        ) && written.contains(
            "const headers: Array<[string, string]> = [[\"content-type\", contentType], [\"x-document-id\", String(headerOut0)]];"
        ) && written.contains("return { status: 200, headers, body: new Uint8Array(bytes) };"),
        "got: {written}"
    );
}

/// Mirrors the Rust `stream_answer_block`.
#[test]
fn a_stream_reply_answers_206_or_the_declared_status_off_the_streamed_records_own_range() {
    let written = http_service_of(STREAM_HTTP_SERVICE);
    assert!(
        written.contains("const status = answer.contentRange === undefined ? 200 : 206;")
            && written.contains("return { status, headers, body: answer.body };"),
        "got: {written}"
    );
}

/// A bound part is neither checked nor read here — see `service_tests`.
#[test]
fn a_multipart_operation_reads_its_own_fields_off_parts_and_hands_a_bound_part_to_the_dispatcher() {
    let written = http_service_of(MULTIPART_HTTP_SERVICE);
    assert!(
        written.contains("request.parts.find(([name]) => name === \"title\")"),
        "an ordinary Generated field is read off a named part. Got: {written}"
    );
    assert!(
        !written.contains("\"a required multipart part was not carried\""),
        "this server keeps no presence check of its own for a `part(...)` binding: the \
         dispatcher refuses a missing required one. Got: {written}"
    );
    assert!(
        !written.contains("attachment"),
        "the part's own parameter is named nowhere in this module. Got: {written}"
    );
    assert!(
        written.contains("dispatch(ctx, operation, payload, request.headers, request.parts)"),
        "a multipart service hands the request's own parts to the dispatcher beside its \
         headers. Got: {written}"
    );
}

/// Never `null`, which `z.strictObject({})` refuses.
#[test]
fn a_bodyless_operation_with_no_field_assembles_the_empty_object_not_null() {
    let written = http_service_of(
        "
        pub trait PulseClientService<Ctx> {
            #[service_schema_op(http(method = \"GET\", path = \"/pulse\"))]
            async fn pulse(&self, ctx: &Ctx) -> Result<PulseResponse, PulseError>;
        }
        ",
    );
    assert!(
        written.contains("return answer(ctx, request, \"pulse\", {}, 200, () => 422);"),
        "got: {written}"
    );
    assert!(!written.contains(", null,"), "got: {written}");
}

/// A unit success answers through the same shared `answer` closure as any other JSON reply.
#[test]
fn a_unit_success_reaches_the_same_answer_closure_as_any_other_reply() {
    let written = http_service_of(TS_UNIT_SUCCESS_SERVICE);
    assert!(
        written.contains("return answer(ctx, request, \"ping\", message, 204, () => 422);"),
        "got: {written}"
    );
}
