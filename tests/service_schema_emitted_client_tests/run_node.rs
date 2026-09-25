//! The emitted TypeScript client run by node, with a real message object.
//!
//! `String(sending)` renders every object as the constant `[object Object]`, so a client that
//! stringifies the whole message sends the same segment for every request.

use super::runtime::ran;
use super::tests::{ConversationClientServiceSchema, StampClientServiceSchema};

/// Names the runtime to run, for a machine that has one somewhere other than `PATH`.
const RUNTIME_VAR: &str = "TIXSCHEMA_NODE";

/// The schema surface the emitted client names, passing the message through untouched: the URL is
/// what is under test, and the real `$Schema` consts are zod expressions no bare runtime can
/// evaluate.
const SCHEMA_STUBS: &str = "const WindowRequest$Schema = {
  safeParse: (value) => ({ success: true, data: value }),
};
const ConversationId$Schema = {
  safeParse: (value) => ({ success: true, data: value }),
};
";

/// Records the request it is handed and answers each operation's own declared status — `204` for
/// the one-way `DELETE`, where `200` would be a fault the client throws instead.
const DRIVER: &str = r#"
const sent = [];
const transport = {
  async send(request) {
    sent.push(request);
    return request.method === "DELETE"
      ? { status: 204, headers: [], body: "" }
      : { status: 200, headers: [], body: JSON.stringify({ items: [] }) };
  },
};
const client = createConversationClientServiceHttpClient(transport);
await client.window({ conversationId: "652f1a3b4c5d6e7f8a9b0c1d", limit: 10 });
await client.window({ conversationId: "652f1a3b4c5d6e7f8a9b0c1d" });
await client.purgeConversation("652f1a3b4c5d6e7f8a9b0c1d");
console.log(JSON.stringify(sent));
"#;

/// The schema stub `StampClientServiceSchema::ts_http_client()` validates `stamp`'s message
/// against: a bare `z.string()`, since `label` is the operation's own whole message.
const STAMP_SCHEMA_STUB: &str =
    "const z = { string: () => ({ safeParse: (value) => ({ success: true, data: value }) }) };\n";

/// Four calls: `x-age` present, absent, and not a number, then the declared error with no
/// `x-reason` — what an absent optional `header_out`/`error_header_out` element reads `null` for,
/// and a present one failing to decode as its declared type faults for.
const STAMP_DRIVER: &str = r#"
const answers = [];

async function probeHeaders(headers) {
  const transport = {
    async send(_request) {
      return {
        status: 200,
        headers,
        body: JSON.stringify({ label: "r1", tenant: "acme" }),
      };
    },
  };
  const client = createStampClientServiceHttpClient(transport);
  const answered = await client.stamp("r1", "acme", undefined);
  answers.push(
    answered.ok
      ? { ok: true, age: answered.value[2] }
      : { ok: false, kind: answered.error.fault.kind, detail: answered.error.fault.detail },
  );
}

await probeHeaders([
  ["etag", "e1"],
  ["x-age", "8"],
]);
await probeHeaders([["etag", "e1"]]);
await probeHeaders([
  ["etag", "e1"],
  ["x-age", "soon"],
]);

const errorTransport = {
  async send(_request) {
    return {
      status: 409,
      headers: [["etag", "e1"]],
      body: JSON.stringify({ errorCode: "refused" }),
    };
  },
};
const errorAnswered = await createStampClientServiceHttpClient(errorTransport).stamp(
  "r1",
  "acme",
  undefined,
);
answers.push(errorAnswered.ok ? { ok: true } : { ok: false, reason: errorAnswered.error[1] });

console.log(JSON.stringify(answers));
"#;

/// The emitted client between the stubs it names and the driver that calls it.
fn module() -> String {
    format!(
        "{SCHEMA_STUBS}\n{client}\n{DRIVER}",
        client = ConversationClientServiceSchema::ts_http_client()
    )
}

/// The requests the driver recorded, or `None` where no runtime was reachable.
fn sent() -> Option<Vec<serde_json::Value>> {
    let wrote = ran("node", RUNTIME_VAR, "node", "client.mts", &module())?;
    Some(serde_json::from_str(wrote.trim()).unwrap())
}

#[test]
fn a_lone_placeholder_sends_the_field_it_names_and_never_the_stringified_message() {
    let Some(sent) = sent() else {
        return;
    };
    assert_eq!(
        sent[0]["path"], "/v1/conversations/652f1a3b4c5d6e7f8a9b0c1d/window",
        "the placeholder is filled by the field it names. Got: {sent:#?}"
    );
    assert!(
        !sent[0]["path"].as_str().unwrap().contains("object"),
        "`String(sending)` on a message object is the constant `[object Object]`, which reaches \
         the wire as `%5Bobject%20Object%5D` — the same segment for every request the operation \
         ever makes. Got: {sent:#?}"
    );
}

#[test]
fn a_field_the_path_does_not_bind_reaches_the_query_string() {
    let Some(sent) = sent() else {
        return;
    };
    assert_eq!(
        sent[0]["query"], "limit=10",
        "`limit` is bound to no placeholder, so the query string is the only place left for it. \
         Got: {sent:#?}"
    );
    assert_eq!(
        sent[1]["query"], "",
        "the same operation with `limit` absent sends no key for it rather than `limit=undefined`. \
         Got: {sent:#?}"
    );
}

#[test]
fn a_scalar_message_is_still_the_whole_segment() {
    let Some(sent) = sent() else {
        return;
    };
    assert_eq!(
        sent[2]["path"], "/v1/conversations/652f1a3b4c5d6e7f8a9b0c1d",
        "a message that already is a wire scalar has no field to read: it is the segment. \
         Got: {sent:#?}"
    );
    assert_eq!(
        sent[2]["query"], "",
        "and no key is left over for a query. Got: {sent:#?}"
    );
}

/// The emitted client between the stub it names and the driver that calls it.
fn stamp_module() -> String {
    format!(
        "{STAMP_SCHEMA_STUB}\n{client}\n{STAMP_DRIVER}",
        client = StampClientServiceSchema::ts_http_client()
    )
}

/// The four answers the driver recorded, or `None` where no runtime was reachable.
fn stamp_answers() -> Option<Vec<serde_json::Value>> {
    let wrote = ran("node", RUNTIME_VAR, "node", "stamp.mts", &stamp_module())?;
    Some(serde_json::from_str(wrote.trim()).unwrap())
}

#[test]
fn an_optional_header_out_element_reads_present_absent_and_a_mismatch() {
    let Some(answers) = stamp_answers() else {
        return;
    };
    assert_eq!(
        answers[0]["age"], 8_i32,
        "a present header reads its declared value. Got: {answers:#?}"
    );
    assert!(
        answers[1]["age"].is_null(),
        "an absent optional header reads null, the value its tuple slot declares. \
         Got: {answers:#?}"
    );
    assert_eq!(
        answers[2]["kind"], "undeserializable-payload",
        "a header that does not decode as its declared type faults. Got: {answers:#?}"
    );
    assert_eq!(
        answers[2]["detail"], "a response header did not match its declared type",
        "got: {answers:#?}"
    );
}

#[test]
fn an_optional_error_header_out_element_reads_an_absent_header_as_null() {
    let Some(answers) = stamp_answers() else {
        return;
    };
    assert_eq!(answers[3]["ok"], false, "got: {answers:#?}");
    assert!(
        answers[3]["reason"].is_null(),
        "an absent optional error header reads null too. Got: {answers:#?}"
    );
}
