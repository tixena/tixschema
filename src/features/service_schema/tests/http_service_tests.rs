//! `ts_http_service()`, read off the emitted text.

use super::{
    BYTES_HTTP_SERVICE, DIGIT_QUERY_HTTP_SERVICE, EMITTED_CLIENT_TEST_SERVICE, MIXED_HTTP_SERVICE,
    MIXED_SERVICE, MULTIPART_HTTP_SERVICE, PATH_BOUND_HTTP_SERVICE, QUERY_HTTP_SERVICE,
    REQUIRED_HEADER_HTTP_SERVICE, RESERVED_WORD_SERVICE, SHADOWED_LOCAL_SERVICE,
    STREAM_HTTP_SERVICE, TS_UNIT_SUCCESS_SERVICE, http_service_of,
};
use crate::utils::{record_untagged_enum, record_wire_scalar};

/// A service whose declared errors answer one status whatever they hold, the status written where
/// the error is answered: a tagged enum with no table beside an `error_header_out`, and an untagged
/// enum whose table maps every variant to one status, beside an `error_header_out` and beside a
/// `header_out` alone. A tagged enum whose table a switch reads answers beside them.
const ONE_STATUS_HTTP_SERVICE: &str = "
    pub trait LapsedClientService<Ctx> {
        #[service_schema_op(http(
            method = \"POST\",
            path = \"/held\",
            error_header_out(\"x-reason\"),
        ))]
        async fn held(
            &self,
            ctx: &Ctx,
            id: String,
        ) -> Result<LapsedDocument, (HeldError, Option<String>)>;

        #[service_schema_op(http(
            method = \"POST\",
            path = \"/lapsed\",
            error_status(Busy = 410, Gone = 410),
            error_header_out(\"x-reason\"),
        ))]
        async fn lapsed(
            &self,
            ctx: &Ctx,
            id: String,
        ) -> Result<LapsedDocument, (LapsedError, Option<String>)>;

        #[service_schema_op(http(
            method = \"POST\",
            path = \"/stale\",
            error_status(Busy = 410, Gone = 410),
            header_out(\"etag\"),
        ))]
        async fn stale(
            &self,
            ctx: &Ctx,
            id: String,
        ) -> Result<(LapsedDocument, String), LapsedError>;

        #[service_schema_op(http(
            method = \"POST\",
            path = \"/mapped\",
            error_status(Missing = 404),
            error_header_out(\"x-reason\"),
        ))]
        async fn mapped(
            &self,
            ctx: &Ctx,
            id: String,
        ) -> Result<LapsedDocument, (HeldError, Option<String>)>;
    }
";

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

/**
 * `value` as it is written to the wire: each `bigint` through `JSON.rawJSON`, the JSON number of
 * its digits. Without `JSON.rawJSON`, a `bigint` a `number` holds exactly is that `number`, and any
 * other is refused. Each `Uint8Array` is the plain array of its bytes.
 */
function conversationClientServiceHttpAnswered(value: unknown): unknown {
  if (typeof value === \"bigint\") {
    const raw: unknown = Reflect.get(JSON, \"rawJSON\");
    if (typeof raw === \"function\") return raw(value.toString());
    if (Number.isSafeInteger(Number(value))) return Number(value);
    throw new RangeError(`${value} is written exactly only through JSON.rawJSON, which this runtime has not`);
  }
  if (value instanceof Uint8Array) return Array.from(value);
  if (Array.isArray(value)) return value.map((held) => conversationClientServiceHttpAnswered(held));
  if (typeof value === \"object\" && value !== null && Object.getPrototypeOf(value) === Object.prototype) {
    return Object.fromEntries(Object.entries(value).map(([key, held]) => [key, conversationClientServiceHttpAnswered(held)]));
  }
  return value;
}

function conversationClientServiceHttpJson(status: number, headers: ReadonlyArray<readonly [string, string]>, value: unknown): ConversationClientServiceHttpResponse {
  return { status, headers: [...headers, [\"content-type\", \"application/json\"]], body: new TextEncoder().encode(JSON.stringify(conversationClientServiceHttpAnswered(value))) };
}

export function conversationClientServiceHttpDefaultFaultHandler(fault: ConversationClientServiceFault): ConversationClientServiceHttpResponse {
  const status = fault.kind === \"unknown-operation\" ? 404 : fault.kind === \"handler-panic\" ? 500 : 400;
  return conversationClientServiceHttpJson(status, [], fault);
}

function conversationClientServiceHttpFault(kind: ConversationClientServiceFaultFields[\"kind\"], operation: string, detail: string, field?: string): ConversationClientServiceFault {
  const built: ConversationClientServiceFaultFields = { detail, field, kind, operation };
  return built as ConversationClientServiceFault;
}

/** Mirrors the Rust `match_path`: a literal token is stripped as a prefix, a placeholder takes one non-empty segment, percent-decoded, and nothing may remain. */
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
      captured.push(conversationClientServiceHttpPercentDecoded(value));
      rest = rest.slice(end);
    }
  }
  return rest === \"\" ? captured : undefined;
}

/** Mirrors the Rust `percent_decoded`: each `%XX` escape decoded to the byte it names, anything else kept, and the bytes read as UTF-8, lossily. */
function conversationClientServiceHttpPercentDecoded(raw: string): string {
  const bytes: Array<number> = [];
  for (const piece of raw.split(/(%[0-9A-Fa-f]{2})/)) {
    if (/^%[0-9A-Fa-f]{2}$/.test(piece)) bytes.push(parseInt(piece.slice(1), 16));
    else bytes.push(...new TextEncoder().encode(piece));
  }
  return new TextDecoder().decode(new Uint8Array(bytes));
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
  const answer = async (ctx: Ctx, operation: string, payload: unknown, headers: ReadonlyArray<readonly [string, string]>, okStatus: number, errorStatus: (error: unknown) => number) => {
    let dispatched: ConversationClientServiceDispatched | undefined;
    try {
      dispatched = await dispatch(ctx, operation, payload, headers);
    } catch (thrown) {
      return onFault(conversationClientServiceHttpFault(\"handler-panic\", operation, thrown instanceof Error ? thrown.message : String(thrown)));
    }
    if (dispatched === undefined) return { status: okStatus, headers: [], body: new Uint8Array() };
    const envelope = dispatched.answered as { ok: true; value: unknown } | { ok: false; error: unknown };
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
        return answer(ctx, \"purge-conversation\", conversation_id, [], 204, () => 422);
      }
    }
    // window: GET /v1/conversations/{conversation_id}/window \u{2014} a macro-generated message: placeholder-bound fields come from the path, the rest from the query string, each with its own coercion.
    if (method === \"GET\") {
      const captured = conversationClientServiceHttpMatchPath([\"/v1/conversations/\", null, \"/window\"], path);
      if (captured !== undefined) {
        const [conversation_id] = captured;
        const queryMap = conversationClientServiceHttpParseQuery(request.query);
        const message: Record<string, unknown> = {};
        message[\"conversationId\"] = conversation_id;
        {
          const raw = queryMap.get(\"limit\");
          message[\"limit\"] = raw === undefined ? null : conversationClientServiceHttpCoerceNumber(raw);
        }
        return answer(ctx, \"window\", message, [], 200, (error) => {
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
fn a_declared_error_crosses_with_every_field_it_carries() {
    let written = http_service_of(MIXED_SERVICE);
    assert!(
        written.contains(
            "return usageServiceHttpJson(errorStatus(envelope.error), [], envelope.error);"
        ),
        "the declared error is the response body, whole. Got: {written}"
    );
    assert!(
        !written.contains("errorMessage"),
        "no field of a declared error is picked out by name. Got: {written}"
    );
}

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
        written
            .contains("return answer(ctx, \"purge-document\", document_id, [], 204, () => 422);"),
        "got: {written}"
    );
}

#[test]
fn a_generated_message_bound_entirely_by_placeholders_takes_no_query() {
    let written = http_service_of(MIXED_HTTP_SERVICE);
    assert!(
        written.contains("message[\"documentId\"] = document_id;")
            && written.contains("message[\"versionId\"] = version_id;"),
        "got: {written}"
    );
    assert!(
        !written.contains("HttpParseQuery(request.query)"),
        "`get_version`'s two fields are both placeholder-bound, so nothing is left to read off \
         the query. Got: {written}"
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

/// A generated message's `i64` is read from the text of the path and the query as the `bigint` its
/// digits hold, exactly, where a `u32` is coerced to a number.
#[test]
fn a_generated_messages_i64_is_read_from_the_text_of_the_path_and_the_query_as_a_bigint() {
    let written = http_service_of(DIGIT_QUERY_HTTP_SERVICE);
    for expected in [
        "message[\"tally\"] = tallyClientServiceHttpCoerceBigInt(tally);",
        "message[\"limit\"] = raw === undefined ? null : tallyClientServiceHttpCoerceBigInt(raw);",
        "message[\"ids\"] = raw === undefined ? null : (raw).split(\",\").map((piece: string) => \
         tallyClientServiceHttpCoerceBigInt(piece));",
        "message[\"page\"] = raw === undefined ? null : tallyClientServiceHttpCoerceNumber(raw);",
        "function tallyClientServiceHttpCoerceBigInt(raw: string): unknown {\n  \
         return /^-?(0|[1-9][0-9]*)$/.test(raw) ? BigInt(raw) : raw;\n}",
    ] {
        assert!(
            written.contains(expected),
            "missing {expected}. Got: {written}"
        );
    }
    assert!(
        !http_service_of(QUERY_HTTP_SERVICE).contains("HttpCoerceBigInt"),
        "a service that reads no `u64` or `i64` from a text writes no reader of one"
    );
}

/// What the dispatcher answers is written as each client writes a message: each `bigint` through
/// `JSON.rawJSON`, as the JSON number of its digits. A body is read through a reader that rebuilds
/// an integer a `number` does not hold exactly as a `bigint`.
#[test]
fn the_answer_is_written_with_each_bigint_as_a_json_number() {
    let written = http_service_of(QUERY_HTTP_SERVICE);
    for expected in [
        "function searchClientServiceHttpAnswered(value: unknown): unknown {\n  \
         if (typeof value === \"bigint\") {\n    \
         const raw: unknown = Reflect.get(JSON, \"rawJSON\");\n    \
         if (typeof raw === \"function\") return raw(value.toString());",
        "body: new TextEncoder().encode(JSON.stringify(searchClientServiceHttpAnswered(value)))",
    ] {
        assert!(
            written.contains(expected),
            "missing {expected}. Got: {written}"
        );
    }
    let bodied = http_service_of(MIXED_HTTP_SERVICE);
    assert!(
        bodied.contains("HttpRequestRead(text: string): unknown {"),
        "a JSON body is read through a reader that rebuilds a `bigint`. Got: {bodied}"
    );
}

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
        written.contains("return answer(ctx, \"search\", message, [], 200, () => 422);"),
        "got: {written}"
    );
    assert!(
        !written.contains("$Variant"),
        "no table means no variant to read. Got: {written}"
    );
}

#[test]
fn a_header_in_binding_is_coerced_here_and_refused_by_the_dispatcher() {
    for source in [MIXED_HTTP_SERVICE, REQUIRED_HEADER_HTTP_SERVICE] {
        let written = http_service_of(source);
        assert!(
            written.contains(
                "const byteRangeText = request.headers.find(([name]) => name.toLowerCase() === \
                 \"range\")?.[1];\n        \
                 if (byteRangeText !== undefined) headersIn.push([\"range\", \
                 JSON.stringify(byteRangeText)]);"
            ),
            "the header's text is JSON-encoded for the dispatcher, and an absent one left out. \
             Got: {written}"
        );
        assert!(
            !written.contains("a required header was not carried"),
            "this server keeps no presence check of its own: the dispatcher refuses a missing \
             required header. Got: {written}"
        );
        assert!(
            written.contains("dispatch(ctx, \"get-version\", message, headersIn);")
                || written.contains("return answer(ctx, \"get-version\", message, headersIn, "),
            "got: {written}"
        );
    }
}

/// Mirrors the Rust `bytes_answer_block`.
#[test]
fn a_bytes_reply_answers_the_raw_bytes_with_their_content_type_and_declared_headers() {
    let written = http_service_of(BYTES_HTTP_SERVICE);
    assert!(
        written.contains("const [bytes, contentType] = envelope.value;")
            && written.contains(
                "const headerOut0 = thumbnailClientServiceHttpRepliedHeader(replied, \
                 \"x-document-id\") as string;\n        \
                 const headers: Array<[string, string]> = [];\n        \
             {\n            \
             const rendered = contentType;\n            \
             if (!thumbnailClientServiceHttpLegalResponseHeaderValue(rendered)) {\n              \
             return onFault(thumbnailClientServiceHttpFault(\"handler-panic\", \
             \"get-thumbnail\", \"a response header value contained a character illegal in an \
             HTTP header\"));\n            \
             }\n            \
             headers.push([\"content-type\", rendered]);\n          \
             }\n        \
             {\n            \
             const rendered = String(headerOut0);\n            \
             if (!thumbnailClientServiceHttpLegalResponseHeaderValue(rendered)) {\n              \
             return onFault(thumbnailClientServiceHttpFault(\"handler-panic\", \
             \"get-thumbnail\", \"a response header value contained a character illegal in an \
             HTTP header\"));\n            \
             }\n            \
             headers.push([\"x-document-id\", rendered]);\n          \
             }"
            )
            && written.contains("return { status: 200, headers, body: new Uint8Array(bytes) };"),
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
    let typed = written.find(
        "const rendered = answer.contentType;\n            \
         if (!contentClientServiceHttpLegalResponseHeaderValue(rendered)) {\n              \
         return onFault(contentClientServiceHttpFault(\"handler-panic\", \"get-file\", \"a \
         response header value contained a character illegal in an HTTP header\"));\n            \
         }\n            \
         headers.push([\"content-type\", rendered]);",
    );
    let ranged = written.find("const rendered = answer.contentRange;");
    assert!(
        typed.is_some() && typed < ranged,
        "the content type is checked and written on every streamed answer, ahead of the range. \
         Got: {written}"
    );
}

/// The route's arm reads the fields off the parts and hands the parts on. The bound part is read
/// by the `dispatch` this module answers an operation only `http_rest` carries with, which
/// refuses a missing required one through the dispatcher's own fault.
#[test]
fn a_multipart_operation_reads_its_own_fields_off_parts_and_hands_a_bound_part_to_the_dispatcher() {
    let written = http_service_of(MULTIPART_HTTP_SERVICE);
    let (dispatched, routed) = written
        .split_once("  return async (ctx, request) => {")
        .unwrap_or_default();
    assert!(
        routed.contains("request.parts.find(([name]) => name === \"title\")"),
        "an ordinary Generated field is read off a named part. Got: {written}"
    );
    assert!(
        !routed.contains("attachment"),
        "the route's arm names no part's own parameter. Got: {written}"
    );
    assert!(
        dispatched.contains(
            "  const enveloped = createUploadClientServiceDispatcher(impl);\n  \
             const dispatch = async (\n"
        ) && dispatched.contains("      case \"upload-document\": {")
            && dispatched
                .contains("const attachment = parts.find(([name]) => name === \"file\")?.[1];")
            && dispatched.contains("\"a required multipart part was not carried\"")
            && dispatched.contains(
                "return { answered: uploadClientServiceAnswered(await impl.uploadDocument(ctx, \
                 received.data, attachment)), headers: [] };"
            )
            && dispatched.contains("return enveloped(ctx, operation, payload, headers, parts);"),
        "the operation is answered by this module's own `dispatch`, which reads and checks the \
         part. Got: {written}"
    );
    assert!(
        written.contains("dispatch(ctx, operation, payload, headers, parts)"),
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
        written.contains("return answer(ctx, \"pulse\", {}, [], 200, () => 422);"),
        "got: {written}"
    );
    assert!(!written.contains(", null,"), "got: {written}");
}

/// A unit success answers through the same shared `answer` closure as any other JSON reply.
#[test]
fn a_unit_success_reaches_the_same_answer_closure_as_any_other_reply() {
    let written = http_service_of(TS_UNIT_SUCCESS_SERVICE);
    assert!(
        written.contains("return answer(ctx, \"ping\", message, [], 204, () => 422);"),
        "got: {written}"
    );
}

#[test]
fn a_placeholder_named_after_a_word_typescript_refuses_is_held_under_a_trailing_underscore() {
    let written = http_service_of(RESERVED_WORD_SERVICE);
    for moved in [
        "const [in_] = captured;",
        "message[\"in\"] = in_;",
        "const [class_] = captured;",
        "message[\"class\"] = class_;",
    ] {
        assert!(written.contains(moved), "no `{moved}` in: {written}");
    }
}

#[test]
fn a_placeholder_named_after_a_local_of_the_dispatcher_is_held_under_another_name() {
    let written = http_service_of(SHADOWED_LOCAL_SERVICE);
    for moved in [
        "const [message_] = captured;",
        "message[\"message\"] = message_;",
    ] {
        assert!(written.contains(moved), "no `{moved}` in: {written}");
    }
}

/// A declared error that answers one status whatever it holds is answered at that status, with no
/// closure of no parameter handed the error, and a table a switch reads is still called with it.
#[test]
fn a_declared_error_of_one_status_is_answered_at_it_with_no_call() {
    record_untagged_enum("LapsedError");
    let written = http_service_of(ONE_STATUS_HTTP_SERVICE);
    for answered in [
        "return lapsedClientServiceHttpJson(422, headers, declaredError);",
        "return lapsedClientServiceHttpJson(410, headers, declaredError);",
        "return lapsedClientServiceHttpJson(410, [], envelope.error);",
        "return lapsedClientServiceHttpJson(((error) => {\n          \
         switch (HeldError$Variant(error)) {\n            \
         case \"Missing\": return 404;\n            \
         default: return 422;\n          \
         }\n        \
         })(declaredError), headers, declaredError);",
    ] {
        assert!(
            written.contains(answered),
            "the dispatcher does not answer `{answered}`. Got: {written}"
        );
    }
    assert!(
        !written.contains("(() =>"),
        "no closure of no parameter is called. Got: {written}"
    );
}
