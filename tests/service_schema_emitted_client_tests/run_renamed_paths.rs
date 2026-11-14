//! A service whose paths bind a field of a declared message that serde names apart from Rust, or
//! that holds a number, a `bool`, an `i64` or a list, called through each emitted `http_rest`
//! client: each path holds the field's value and each body carries it under the key serde writes,
//! the Rust dispatcher reads each request, and the TypeScript dispatcher hands an implementation
//! the path's value, in the form its member publishes, whatever the body carries.

use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::{model_schema, service_schema};

use super::keyed_path_http_rest_transport as transport;
use super::runtime::{node_modules, ran_with_modules, stand_down_modules};

#[cfg(feature = "dart")]
use super::run_dart_validate::dart;
#[cfg(feature = "swift")]
use super::runtime::judged;
#[cfg(feature = "dart")]
use super::runtime::ran;
#[cfg(feature = "kotlin")]
use super::runtime::ran_kotlin;

/// The key `KeyedQuoted` reads its field from: a `$`, a quote, a double quote and a backslash, each
/// a character a Dart, a Swift, a Kotlin or a TypeScript string literal reads apart from text.
const QUOTED_KEY: &str = "doc\"it's$\\";

/// One call of each operation, in the order each driver makes them, and what the Rust dispatcher
/// answers its request.
const CARRIED: [Carried; 14] = [
    ("/beside/d1", Some(("docId", r#""d1""#)), "d1"),
    ("/generated/d1", None, "d1"),
    (
        "/genlabelled/x%2Cy",
        Some(("labels", r#"["x","y"]"#)),
        "x|y",
    ),
    (
        "/genlist/1%2C2%2C3",
        Some(("pages", "[1,2,3]")),
        "[1, 2, 3]",
    ),
    (
        "/labelled/a%20b%2Cc%2Fd",
        Some(("labels", r#"["a b","c/d"]"#)),
        "a b|c/d",
    ),
    ("/listed/1%2C2%2C3", Some(("pages", "[1,2,3]")), "[1, 2, 3]"),
    ("/own/d1", Some(("doc-id", r#""d1""#)), "d1"),
    ("/pages/7", Some(("page_size", "7")), "7"),
    ("/pinned/true", Some(("pinned", "true")), "true"),
    ("/plain/d1", Some(("document_id", r#""d1""#)), "d1"),
    ("/quoted/d1", Some((QUOTED_KEY, r#""d1""#)), "d1"),
    ("/renamed/d1", Some(("documentId", r#""d1""#)), "d1"),
    (
        "/tallied/-9007199254740993",
        Some(("tally", "-9007199254740993")),
        "-9007199254740993",
    ),
    ("/two/d1/t", Some(("documentId", r#""d1""#)), "d1|t"),
];

/// Bodies that carry another value under the key of a member a path reads that is no text: `path`,
/// `body`, then the id the implementation is handed.
const TYPED: [(&str, &str, &str); 4] = [
    ("/listed/1,2,3", r#"{"pages":[9],"title":"t"}"#, "[1, 2, 3]"),
    ("/pages/7", r#"{"page_size":9,"title":"t"}"#, "7"),
    ("/pinned/true", r#"{"pinned":false,"title":"t"}"#, "true"),
    (
        "/tallied/-9007199254740993",
        r#"{"tally":1,"title":"t"}"#,
        "-9007199254740993",
    ),
];

/// A path whose text the form of its member does not read, and its body.
const UNREAD: (&str, &str) = ("/pages/abc", r#"{"title":"t"}"#);

/// Bodies that carry another value under the key each operation reads its field from, and bodies
/// that carry none: `path`, then `body`.
const ELSEWHERE: [(&str, &str); 10] = [
    ("/beside/d1", r#"{"docId":"elsewhere","title":"t"}"#),
    ("/beside/d1", r#"{"title":"t"}"#),
    ("/plain/d1", r#"{"document_id":"elsewhere","title":"t"}"#),
    ("/plain/d1", r#"{"title":"t"}"#),
    ("/renamed/d1", r#"{"documentId":"elsewhere","title":"t"}"#),
    ("/renamed/d1", r#"{"title":"t"}"#),
    ("/own/d1", r#"{"doc-id":"elsewhere","title":"t"}"#),
    ("/own/d1", r#"{"title":"t"}"#),
    ("/quoted/d1", r#"{"doc\"it's$\\":"elsewhere","title":"t"}"#),
    ("/quoted/d1", r#"{"title":"t"}"#),
];

/// Records each request and answers `200`, then makes one call of each operation.
#[cfg(feature = "dart")]
const DART_DRIVER: &str = "
class _KeyedRecorder implements KeyedPathClientServiceHttpTransport {
  final List<Map<String, dynamic>> sent = <Map<String, dynamic>>[];

  @override
  Future<({int status, List<(String, String)> headers, List<int> body, Stream<List<int>> bodyStream})> send(
    ({String method, String path, String query, List<(String, String)> headers, List<int> body, List<(String, dynamic)> parts}) request,
  ) async {
    sent.add(<String, dynamic>{
      'method': request.method,
      'path': request.path,
      'query': request.query,
      'body': utf8.decode(request.body),
    });
    return (
      status: 200,
      headers: <(String, String)>[],
      body: utf8.encode(jsonEncode(<String, dynamic>{'id': ''})),
      bodyStream: const Stream<List<int>>.empty(),
    );
  }
}

void main() async {
  final recorder = _KeyedRecorder();
  final client = KeyedPathClientServiceHttpClient(recorder);
  await client.besideFlatten(KeyedBesideDraft(document_id: 'd1', extra: KeyedExtraDraft(title: 't').validate()).validate());
  await client.keyedGenerated(KeyedGeneratedRequest(document_id: 'd1', title: 't'));
  await client.keyedGenLabelled(KeyedGenLabelledRequest(labels: ['x', 'y'], title: 't'));
  await client.keyedGenList(KeyedGenListRequest(pages: [1, 2, 3], title: 't'));
  await client.labelled(KeyedLabelledDraft(labels: ['a b', 'c/d'], title: 't').validate());
  await client.listed(KeyedListedDraft(pages: [1, 2, 3], title: 't').validate());
  await client.ownKey(KeyedOwnDraft(document_id: 'd1', title: 't').validate());
  await client.paged(KeyedPagedDraft(page_size: 7, title: 't').validate());
  await client.pinned(KeyedPinnedDraft(pinned: true, title: 't').validate());
  await client.plain(KeyedPlainDraft(document_id: 'd1', title: 't').validate());
  await client.quotedKey(KeyedQuotedDraft(document_id: 'd1', title: 't').validate());
  await client.renamed(KeyedRenamedDraft(document_id: 'd1', title: 't').validate());
  await client.tallied(KeyedTallyDraft(tally: -9007199254740993, title: 't').validate());
  await client.twoPlaceholders(KeyedRenamedDraft(document_id: 'd1', title: 't').validate());
  await client.keyedGenLabelled(KeyedGenLabelledRequest(labels: ['a,b', 'c'], title: 't'));
  await client.labelled(KeyedLabelledDraft(labels: ['a,b', 'c'], title: 't').validate());
  print(jsonEncode(recorder.sent));
}
";

/// The Kotlin twin of [`DART_DRIVER`].
#[cfg(feature = "kotlin")]
const KOTLIN_DRIVER: &str = r#"
class KeyedRecorder : KeyedPathClientServiceHttpTransport {
    val sent = mutableListOf<KeyedPathClientServiceHttpRequest>()
    override suspend fun send(request: KeyedPathClientServiceHttpRequest): KeyedPathClientServiceHttpResponse {
        sent.add(request)
        return KeyedPathClientServiceHttpResponse(200, emptyList(), """{"id":""}""".encodeToByteArray())
    }
}

fun main() = runBlocking {
    val transport = KeyedRecorder()
    val client = KeyedPathClientServiceHttpClient(transport)
    client.besideFlatten(KeyedBesideDraft(documentId = "d1", extra = KeyedExtraDraft(title = "t").validate()).validate())
    client.keyedGenerated(KeyedGeneratedRequest(documentId = "d1", title = "t"))
    client.keyedGenLabelled(KeyedGenLabelledRequest(labels = listOf("x", "y"), title = "t"))
    client.keyedGenList(KeyedGenListRequest(pages = listOf(1u, 2u, 3u), title = "t"))
    client.labelled(KeyedLabelledDraft(labels = listOf("a b", "c/d"), title = "t").validate())
    client.listed(KeyedListedDraft(pages = listOf(1u, 2u, 3u), title = "t").validate())
    client.ownKey(KeyedOwnDraft(documentId = "d1", title = "t").validate())
    client.paged(KeyedPagedDraft(pageSize = 7u, title = "t").validate())
    client.pinned(KeyedPinnedDraft(pinned = true, title = "t").validate())
    client.plain(KeyedPlainDraft(documentId = "d1", title = "t").validate())
    client.quotedKey(KeyedQuotedDraft(documentId = "d1", title = "t").validate())
    client.renamed(KeyedRenamedDraft(documentId = "d1", title = "t").validate())
    client.tallied(KeyedTallyDraft(tally = -9007199254740993L, title = "t").validate())
    client.twoPlaceholders(KeyedRenamedDraft(documentId = "d1", title = "t").validate())
    client.keyedGenLabelled(KeyedGenLabelledRequest(labels = listOf("a,b", "c"), title = "t"))
    client.labelled(KeyedLabelledDraft(labels = listOf("a,b", "c"), title = "t").validate())
    val report = buildJsonArray {
        for (request in transport.sent) {
            add(buildJsonObject {
                put("method", request.method)
                put("path", request.path)
                put("query", request.query)
                put("body", request.body.decodeToString())
            })
        }
    }
    println(report.toString())
}
"#;

/// The imports the emitted Kotlin and its driver reach for.
#[cfg(feature = "kotlin")]
const KOTLIN_IMPORTS: &str = "import kotlinx.coroutines.*\n\
     import kotlinx.coroutines.flow.*\n\
     import kotlinx.serialization.*\n\
     import kotlinx.serialization.json.*\n\
     import kotlinx.serialization.descriptors.*\n\
     import kotlinx.serialization.encoding.*\n\
     import kotlinx.serialization.builtins.*";

/// The Swift twin of [`DART_DRIVER`].
#[cfg(feature = "swift")]
const SWIFT_DRIVER: &str = r##"
struct Sent: Codable {
  let method: String
  let path: String
  let query: String
  let body: String
}

actor KeyedRecorder: KeyedPathClientServiceHttpTransport {
  private(set) var sent: [Sent] = []

  func send(_ request: KeyedPathClientServiceHttpRequest) async throws -> KeyedPathClientServiceHttpResponse {
    sent.append(Sent(
      method: request.method,
      path: request.path,
      query: request.query,
      body: String(decoding: request.body, as: UTF8.self)
    ))
    return KeyedPathClientServiceHttpResponse(status: 200, headers: [], body: Data(#"{"id":""}"#.utf8))
  }
}

let recorder = KeyedRecorder()
let client = KeyedPathClientServiceHttpClient(transport: recorder)
_ = await client.besideFlatten(try KeyedBesideDraft(documentId: "d1", extra: try KeyedExtraDraft(title: "t").validate()).validate())
_ = await client.keyedGenerated(KeyedGeneratedRequest(documentId: "d1", title: "t"))
_ = await client.keyedGenLabelled(KeyedGenLabelledRequest(labels: ["x", "y"], title: "t"))
_ = await client.keyedGenList(KeyedGenListRequest(pages: [1, 2, 3], title: "t"))
_ = await client.labelled(try KeyedLabelledDraft(labels: ["a b", "c/d"], title: "t").validate())
_ = await client.listed(try KeyedListedDraft(pages: [1, 2, 3], title: "t").validate())
_ = await client.ownKey(try KeyedOwnDraft(documentId: "d1", title: "t").validate())
_ = await client.paged(try KeyedPagedDraft(pageSize: 7, title: "t").validate())
_ = await client.pinned(try KeyedPinnedDraft(pinned: true, title: "t").validate())
_ = await client.plain(try KeyedPlainDraft(documentId: "d1", title: "t").validate())
_ = await client.quotedKey(try KeyedQuotedDraft(documentId: "d1", title: "t").validate())
_ = await client.renamed(try KeyedRenamedDraft(documentId: "d1", title: "t").validate())
_ = await client.tallied(try KeyedTallyDraft(tally: -9007199254740993, title: "t").validate())
_ = await client.twoPlaceholders(try KeyedRenamedDraft(documentId: "d1", title: "t").validate())
_ = await client.keyedGenLabelled(KeyedGenLabelledRequest(labels: ["a,b", "c"], title: "t"))
_ = await client.labelled(try KeyedLabelledDraft(labels: ["a,b", "c"], title: "t").validate())
print(String(data: try! JSONEncoder().encode(await recorder.sent), encoding: .utf8)!)
"##;

/// One call of each operation through the TypeScript client bound to the TypeScript dispatcher,
/// then each body of `ELSEWHERE`, `TYPED` and `UNREAD` handed to the dispatcher. Those and
/// `QUOTED_KEY` are written above this by the Rust side. A member that is no text answers its id
/// only where it reached the implementation as the type its member holds.
const NODE_DRIVER: &str = r#"
const impl = {
  async besideFlatten(ctx, req) { return { ok: true, value: { id: req.docId } }; },
  async keyedGenerated(ctx, req) { return { ok: true, value: { id: req.documentId } }; },
  async keyedGenLabelled(ctx, req) { return { ok: true, value: { id: req.labels.join("|") } }; },
  async keyedGenList(ctx, req) {
    return { ok: true, value: { id: `[${req.pages.join(", ")}]` } };
  },
  async labelled(ctx, req) { return { ok: true, value: { id: req.labels.join("|") } }; },
  async listed(ctx, req) {
    const numbers = req.pages.every((page) => typeof page === "number");
    return { ok: true, value: { id: numbers ? `[${req.pages.join(", ")}]` : "text" } };
  },
  async ownKey(ctx, req) { return { ok: true, value: { id: req["doc-id"] } }; },
  async paged(ctx, req) {
    return { ok: true, value: { id: typeof req.page_size === "number" ? String(req.page_size) : "text" } };
  },
  async pinned(ctx, req) {
    return { ok: true, value: { id: typeof req.pinned === "boolean" ? String(req.pinned) : "text" } };
  },
  async plain(ctx, req) { return { ok: true, value: { id: req.document_id } }; },
  async quotedKey(ctx, req) { return { ok: true, value: { id: req[QUOTED_KEY] } }; },
  async renamed(ctx, req) {
    return req.documentId === "refused"
      ? { ok: false, error: { errorCode: "Nope" } }
      : { ok: true, value: { id: req.documentId } };
  },
  async tallied(ctx, req) {
    return { ok: true, value: { id: typeof req.tally === "bigint" ? String(req.tally) : "text" } };
  },
  async twoPlaceholders(ctx, req) { return { ok: true, value: { id: `${req.documentId}|${req.title}` } }; },
};

const dispatch = createKeyedPathClientServiceHttpDispatcher(impl);
const sent = [];
const client = createKeyedPathClientServiceHttpClient({
  async send(request) {
    const response = await dispatch({}, {
      method: request.method,
      path: request.path,
      query: request.query,
      headers: request.headers,
      body: request.body,
    });
    sent.push({
      method: request.method,
      path: request.path,
      query: request.query,
      body: new TextDecoder().decode(request.body),
    });
    return { status: response.status, headers: response.headers, body: response.body };
  },
});

const answers = [
  await client.besideFlatten({ docId: "d1", title: "t" }),
  await client.keyedGenerated({ documentId: "d1", title: "t" }),
  await client.keyedGenLabelled({ labels: ["x", "y"], title: "t" }),
  await client.keyedGenList({ pages: [1, 2, 3], title: "t" }),
  await client.labelled({ labels: ["a b", "c/d"], title: "t" }),
  await client.listed({ pages: [1, 2, 3], title: "t" }),
  await client.ownKey({ "doc-id": "d1", title: "t" }),
  await client.paged({ page_size: 7, title: "t" }),
  await client.pinned({ pinned: true, title: "t" }),
  await client.plain({ document_id: "d1", title: "t" }),
  await client.quotedKey({ [QUOTED_KEY]: "d1", title: "t" }),
  await client.renamed({ documentId: "d1", title: "t" }),
  await client.tallied({ tally: -9007199254740993n, title: "t" }),
  await client.twoPlaceholders({ documentId: "d1", title: "t" }),
];
const declaredError = await client.renamed({ documentId: "refused", title: "t" });
const sentBefore = sent.length;
const commaRefusals = [
  await client.keyedGenLabelled({ labels: ["a,b", "c"], title: "t" }),
  await client.labelled({ labels: ["a,b", "c"], title: "t" }),
];

async function put([path, body]) {
  const response = await dispatch({}, {
    method: "PUT",
    path,
    query: "",
    headers: [],
    body: new TextEncoder().encode(body),
  });
  return { status: response.status, body: new TextDecoder().decode(response.body) };
}

const elsewhere = [];
for (const row of ELSEWHERE) elsewhere.push(await put(row));
const typed = [];
for (const row of TYPED) typed.push(await put(row));
const unread = await put(UNREAD);
console.log(JSON.stringify({
  sent: sent.slice(0, CARRIED_COUNT),
  answers,
  declaredError,
  commaRefusals,
  refusedSent: sent.length - sentBefore,
  elsewhere,
  typed,
  unread,
}));
"#;

/// The package the emitted schemas import.
const REQUIRED_PACKAGES: &[&str] = &["zod"];

/// The runtime the TypeScript driver runs under, named for a machine with one elsewhere.
const RUNTIME_VAR: &str = "TIXSCHEMA_NODE";

/// The path a client writes, the key its body holds the field under beside the field's JSON, and
/// the id the Rust dispatcher answers.
type Carried = (
    &'static str,
    Option<(&'static str, &'static str)>,
    &'static str,
);

/// The id an implementation was handed.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct KeyedAnswer {
    pub id: String,
}

/// The error each operation declares.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "errorCode")]
pub enum KeyedRefused {
    Nope,
}

/// A message serde names as Rust does.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KeyedPlain {
    pub document_id: String,
    pub title: String,
}

/// A message under a `rename_all`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyedRenamed {
    pub document_id: String,
    pub title: String,
}

/// A message whose member has a key of its own, which is no identifier.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KeyedOwn {
    #[serde(rename = "doc-id")]
    pub document_id: String,
    pub title: String,
}

/// What `KeyedBeside` flattens.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KeyedExtra {
    pub title: String,
}

/// A renamed member beside a flattened one.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KeyedBeside {
    #[serde(rename = "docId")]
    pub document_id: String,
    #[serde(flatten)]
    pub extra: KeyedExtra,
}

/// A message whose member has the key [`QUOTED_KEY`].
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KeyedQuoted {
    #[serde(rename = "doc\"it's$\\")]
    pub document_id: String,
    pub title: String,
}

/// A message whose member a path reads as a number.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KeyedPaged {
    pub page_size: u32,
    pub title: String,
}

/// A message whose member a path reads as a `bool`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KeyedPinned {
    pub pinned: bool,
    pub title: String,
}

/// A message whose member a path reads as an `i64`, which TypeScript holds as a `bigint`.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KeyedTally {
    pub tally: i64,
    pub title: String,
}

/// A message whose member a path reads as a list of numbers, which the dispatchers are handed by
/// hand.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KeyedListed {
    pub pages: Vec<u32>,
    pub title: String,
}

/// A message whose member a path reads as a list of texts.
#[model_schema()]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct KeyedLabelled {
    pub labels: Vec<String>,
    pub title: String,
}

/// One operation for each way serde names a field, one beside a flattened field, one whose path
/// binds two fields, one whose message the macro generates, and one for each form a path reads a
/// member that is no text in.
#[service_schema(transports = ["http_rest"])]
pub trait KeyedPathClientService<Ctx> {
    #[service_schema_op(http(method = "PUT", path = "/beside/{document_id}"))]
    async fn beside_flatten(
        &self,
        ctx: &Ctx,
        req: KeyedBeside,
    ) -> Result<KeyedAnswer, KeyedRefused>;

    #[service_schema_op(http(method = "PUT", path = "/genlabelled/{labels}"))]
    async fn keyed_gen_labelled(
        &self,
        ctx: &Ctx,
        labels: Vec<String>,
        title: String,
    ) -> Result<KeyedAnswer, KeyedRefused>;

    #[service_schema_op(http(method = "PUT", path = "/genlist/{pages}"))]
    async fn keyed_gen_list(
        &self,
        ctx: &Ctx,
        pages: Vec<u32>,
        title: String,
    ) -> Result<KeyedAnswer, KeyedRefused>;

    #[service_schema_op(http(method = "PUT", path = "/generated/{document_id}"))]
    async fn keyed_generated(
        &self,
        ctx: &Ctx,
        document_id: String,
        title: String,
    ) -> Result<KeyedAnswer, KeyedRefused>;

    #[service_schema_op(http(method = "PUT", path = "/labelled/{labels}"))]
    async fn labelled(&self, ctx: &Ctx, req: KeyedLabelled) -> Result<KeyedAnswer, KeyedRefused>;

    #[service_schema_op(http(method = "PUT", path = "/listed/{pages}"))]
    async fn listed(&self, ctx: &Ctx, req: KeyedListed) -> Result<KeyedAnswer, KeyedRefused>;

    #[service_schema_op(http(method = "PUT", path = "/own/{document_id}"))]
    async fn own_key(&self, ctx: &Ctx, req: KeyedOwn) -> Result<KeyedAnswer, KeyedRefused>;

    #[service_schema_op(http(method = "PUT", path = "/pages/{page_size}"))]
    async fn paged(&self, ctx: &Ctx, req: KeyedPaged) -> Result<KeyedAnswer, KeyedRefused>;

    #[service_schema_op(http(method = "PUT", path = "/pinned/{pinned}"))]
    async fn pinned(&self, ctx: &Ctx, req: KeyedPinned) -> Result<KeyedAnswer, KeyedRefused>;

    #[service_schema_op(http(method = "PUT", path = "/plain/{document_id}"))]
    async fn plain(&self, ctx: &Ctx, req: KeyedPlain) -> Result<KeyedAnswer, KeyedRefused>;

    #[service_schema_op(http(method = "PUT", path = "/quoted/{document_id}"))]
    async fn quoted_key(&self, ctx: &Ctx, req: KeyedQuoted) -> Result<KeyedAnswer, KeyedRefused>;

    #[service_schema_op(http(
        method = "PUT",
        path = "/renamed/{document_id}",
        error_status(Nope = 409),
    ))]
    async fn renamed(&self, ctx: &Ctx, req: KeyedRenamed) -> Result<KeyedAnswer, KeyedRefused>;

    #[service_schema_op(http(method = "PUT", path = "/tallied/{tally}"))]
    async fn tallied(&self, ctx: &Ctx, req: KeyedTally) -> Result<KeyedAnswer, KeyedRefused>;

    #[service_schema_op(http(method = "PUT", path = "/two/{document_id}/{title}"))]
    async fn two_placeholders(
        &self,
        ctx: &Ctx,
        req: KeyedRenamed,
    ) -> Result<KeyedAnswer, KeyedRefused>;
}

/// Answers each operation with the id its implementation was handed.
pub struct KeyedBackEnd;

impl KeyedPathClientService<()> for KeyedBackEnd {
    async fn beside_flatten(
        &self,
        _ctx: &(),
        req: KeyedBeside,
    ) -> Result<KeyedAnswer, KeyedRefused> {
        ready(()).await;
        Ok(KeyedAnswerDraft {
            id: req.document_id.clone(),
        }
        .into())
    }

    async fn keyed_gen_labelled(
        &self,
        _ctx: &(),
        labels: Vec<String>,
        _title: String,
    ) -> Result<KeyedAnswer, KeyedRefused> {
        ready(()).await;
        Ok(KeyedAnswerDraft {
            id: labels.join("|"),
        }
        .into())
    }

    async fn keyed_gen_list(
        &self,
        _ctx: &(),
        pages: Vec<u32>,
        _title: String,
    ) -> Result<KeyedAnswer, KeyedRefused> {
        ready(()).await;
        Ok(KeyedAnswerDraft {
            id: format!("{pages:?}"),
        }
        .into())
    }

    async fn keyed_generated(
        &self,
        _ctx: &(),
        document_id: String,
        _title: String,
    ) -> Result<KeyedAnswer, KeyedRefused> {
        ready(()).await;
        Ok(KeyedAnswerDraft { id: document_id }.into())
    }

    async fn labelled(&self, _ctx: &(), req: KeyedLabelled) -> Result<KeyedAnswer, KeyedRefused> {
        ready(()).await;
        Ok(KeyedAnswerDraft {
            id: req.labels.join("|"),
        }
        .into())
    }

    async fn listed(&self, _ctx: &(), req: KeyedListed) -> Result<KeyedAnswer, KeyedRefused> {
        ready(()).await;
        Ok(KeyedAnswerDraft {
            id: format!("{:?}", req.pages),
        }
        .into())
    }

    async fn own_key(&self, _ctx: &(), req: KeyedOwn) -> Result<KeyedAnswer, KeyedRefused> {
        ready(()).await;
        Ok(KeyedAnswerDraft {
            id: req.document_id.clone(),
        }
        .into())
    }

    async fn paged(&self, _ctx: &(), req: KeyedPaged) -> Result<KeyedAnswer, KeyedRefused> {
        ready(()).await;
        Ok(KeyedAnswerDraft {
            id: req.page_size.to_string(),
        }
        .into())
    }

    async fn pinned(&self, _ctx: &(), req: KeyedPinned) -> Result<KeyedAnswer, KeyedRefused> {
        ready(()).await;
        Ok(KeyedAnswerDraft {
            id: req.pinned.to_string(),
        }
        .into())
    }

    async fn plain(&self, _ctx: &(), req: KeyedPlain) -> Result<KeyedAnswer, KeyedRefused> {
        ready(()).await;
        Ok(KeyedAnswerDraft {
            id: req.document_id.clone(),
        }
        .into())
    }

    async fn quoted_key(&self, _ctx: &(), req: KeyedQuoted) -> Result<KeyedAnswer, KeyedRefused> {
        ready(()).await;
        Ok(KeyedAnswerDraft {
            id: req.document_id.clone(),
        }
        .into())
    }

    async fn renamed(&self, _ctx: &(), req: KeyedRenamed) -> Result<KeyedAnswer, KeyedRefused> {
        ready(()).await;
        if req.document_id == "refused" {
            return Err(KeyedRefused::Nope);
        }
        Ok(KeyedAnswerDraft {
            id: req.document_id.clone(),
        }
        .into())
    }

    async fn tallied(&self, _ctx: &(), req: KeyedTally) -> Result<KeyedAnswer, KeyedRefused> {
        ready(()).await;
        Ok(KeyedAnswerDraft {
            id: req.tally.to_string(),
        }
        .into())
    }

    async fn two_placeholders(
        &self,
        _ctx: &(),
        req: KeyedRenamed,
    ) -> Result<KeyedAnswer, KeyedRefused> {
        ready(()).await;
        Ok(KeyedAnswerDraft {
            id: format!("{}|{}", req.document_id, req.title),
        }
        .into())
    }
}

/// Answers each fault `499`, through the response an adapter would build.
struct Refused;

impl transport::FaultHandler for Refused {
    fn on_fault(
        &self,
        fault: &keyed_path_client_service_schema::ServiceFault,
    ) -> transport::OutgoingResponse {
        transport::OutgoingResponse::new(
            499,
            vec![("x-fault".to_owned(), format!("{}", fault.kind()))],
            b"refused".to_vec(),
        )
    }
}

/// The back end never suspends, so one poll answers it.
fn poll_once<Answering>(answering: Answering) -> Option<Answering::Output>
where
    Answering: Future,
{
    let mut pinned = pin!(answering);
    match pinned
        .as_mut()
        .poll(&mut PollContext::from_waker(Waker::noop()))
    {
        Poll::Ready(answer) => Some(answer),
        Poll::Pending => None,
    }
}

/// The status and the body the Rust dispatcher answers a request with.
fn rust_answered(method: &str, path: &str, query: &str, body: &str) -> (u16, Value) {
    let request = transport::IncomingRequest::new(
        method.to_owned(),
        path.to_owned(),
        query.to_owned(),
        Vec::new(),
        body.as_bytes().to_vec(),
    );
    let response = poll_once(transport::dispatch(
        &KeyedBackEnd,
        &(),
        &request,
        &transport::DefaultFaultHandler,
    ))
    .unwrap();
    (
        response.status(),
        serde_json::from_slice(response.body()).unwrap(),
    )
}

/// Holds the requests a client wrote to [`CARRIED`], each read by the Rust dispatcher.
fn assert_carried(sent: &[Value]) {
    assert_eq!(sent.len(), CARRIED.len(), "{sent:#?}");
    for (request, (path, held, id)) in sent.iter().zip(CARRIED) {
        let text = |named: &str| request[named].as_str().unwrap();
        assert_eq!(text("path"), path, "{request}");
        if let Some((held_under, value)) = held {
            let body: Value = serde_json::from_str(text("body")).unwrap();
            assert_eq!(
                body[held_under],
                serde_json::from_str::<Value>(value).unwrap(),
                "the body holds it under another key: {request}"
            );
        }
        assert_eq!(
            rust_answered(text("method"), text("path"), text("query"), text("body")),
            (200, json!({ "id": id })),
            "{request}"
        );
    }
}

#[cfg(any(feature = "dart", feature = "swift", feature = "kotlin"))]
fn assert_printed(printed: &str) {
    let sent: Vec<Value> = serde_json::from_str(printed.trim()).unwrap();
    assert_carried(&sent);
}

#[test]
fn the_rust_dispatcher_hands_the_implementation_the_paths_value() {
    for (path, body) in ELSEWHERE {
        assert_eq!(
            rust_answered("PUT", path, "", body),
            (200, json!({ "id": "d1" })),
            "PUT {path} {body}"
        );
    }
}

#[test]
fn the_rust_dispatcher_reads_each_member_in_the_form_it_publishes() {
    for (path, body, id) in TYPED {
        assert_eq!(
            rust_answered("PUT", path, "", body),
            (200, json!({ "id": id })),
            "PUT {path} {body}"
        );
    }
    let (status, refused) = rust_answered("PUT", UNREAD.0, "", UNREAD.1);
    assert_eq!(
        (status, &refused["issues"]),
        (
            400,
            &json!([{
                "detail": r#"invalid type: string "abc", expected u32"#,
                "field": "page_size",
            }])
        )
    );
}

#[test]
fn the_rust_dispatcher_answers_a_renamed_field_through_the_handler_it_is_given() {
    let request = transport::IncomingRequest::new(
        "PUT".to_owned(),
        "/renamed/d1".to_owned(),
        "unused=1".to_owned(),
        vec![("x-trace".to_owned(), "abc".to_owned())],
        b"[1]".to_vec(),
    );
    assert_eq!(request.query(), "unused=1");
    assert_eq!(request.body(), b"[1]");
    assert_eq!(request.header("X-Trace"), Some("abc"));
    assert_eq!(request.headers().len(), 1);
    let response = poll_once(transport::dispatch(&KeyedBackEnd, &(), &request, &Refused)).unwrap();
    assert_eq!(response.status(), 499, "{:?}", response.headers());
}

#[test]
fn the_route_table_names_each_placeholder_as_the_path_writes_it() {
    let renamed = transport::ROUTES
        .iter()
        .find(|route: &&transport::Route| route.operation() == "renamed")
        .unwrap();
    assert_eq!(renamed.method(), "PUT");
    assert_eq!(renamed.path(), "/renamed/{document_id}");
    assert_eq!(renamed.ok_status(), 200);
    assert_eq!(renamed.error_statuses(), &[409]);
}

#[test]
fn the_typescript_client_and_dispatcher_carry_the_field_under_its_key() {
    let Some(modules) = node_modules(REQUIRED_PACKAGES) else {
        stand_down_modules(
            REQUIRED_PACKAGES,
            "the emitted TypeScript service whose paths bind a renamed field",
        );
        return;
    };
    let module = [
        "import { z } from \"zod\";".to_owned(),
        KeyedAnswer::ts_definition(),
        KeyedAnswer::zod_schema(),
        KeyedRefused::ts_definition(),
        KeyedRefused::zod_schema(),
        KeyedPlain::ts_definition(),
        KeyedPlain::zod_schema(),
        KeyedRenamed::ts_definition(),
        KeyedRenamed::zod_schema(),
        KeyedOwn::ts_definition(),
        KeyedOwn::zod_schema(),
        KeyedQuoted::ts_definition(),
        KeyedQuoted::zod_schema(),
        KeyedExtra::ts_definition(),
        KeyedExtra::zod_schema(),
        KeyedBeside::ts_definition(),
        KeyedBeside::zod_schema(),
        KeyedPaged::ts_definition(),
        KeyedPaged::zod_schema(),
        KeyedPinned::ts_definition(),
        KeyedPinned::zod_schema(),
        KeyedTally::ts_definition(),
        KeyedTally::zod_schema(),
        KeyedListed::ts_definition(),
        KeyedListed::zod_schema(),
        KeyedLabelled::ts_definition(),
        KeyedLabelled::zod_schema(),
        KeyedPathClientServiceSchema::ts_definition(),
        KeyedPathClientServiceSchema::ts_service(),
        KeyedPathClientServiceSchema::ts_http_client(),
        KeyedPathClientServiceSchema::ts_http_service(),
        format!(
            "const ELSEWHERE = {};\nconst TYPED = {};\nconst UNREAD = {};\n\
             const QUOTED_KEY = {};\nconst CARRIED_COUNT = {};",
            json!(ELSEWHERE),
            json!(TYPED.map(|(path, body, _)| (path, body))),
            json!(UNREAD),
            json!(QUOTED_KEY),
            CARRIED.len()
        ),
        NODE_DRIVER.to_owned(),
    ]
    .join("\n\n");
    let Some(printed) = ran_with_modules(
        "renamed-paths",
        RUNTIME_VAR,
        "node",
        "renamed.mts",
        &module,
        &modules,
    ) else {
        return;
    };
    let ran: Value = serde_json::from_str(printed.trim()).unwrap();
    assert_carried(ran["sent"].as_array().unwrap());
    assert_eq!(
        ran["refusedSent"], 0_u8,
        "a refused list was sent: {printed}"
    );
    let comma = json!([{
        "detail": "a list item holds a comma, which a path would read as two items",
        "field": "labels",
    }]);
    for refusal in ran["commaRefusals"].as_array().unwrap() {
        assert_eq!(refusal["ok"], false, "{printed}");
        assert_eq!(refusal["error"]["fault"]["issues"], comma, "{printed}");
    }
    let answered: Vec<Value> = CARRIED
        .iter()
        .map(|(_, _, id)| json!({ "ok": true, "value": { "id": id } }))
        .collect();
    assert_eq!(ran["answers"], Value::Array(answered), "{printed}");
    assert_eq!(
        ran["declaredError"],
        json!({ "ok": false, "error": { "errorCode": "Nope" } }),
        "{printed}"
    );
    let elsewhere: Vec<Value> = ELSEWHERE
        .iter()
        .map(|_| json!({ "status": 200_u16, "body": r#"{"id":"d1"}"# }))
        .collect();
    assert_eq!(ran["elsewhere"], Value::Array(elsewhere), "{printed}");
    let typed: Vec<Value> = TYPED
        .iter()
        .map(|(_, _, id)| json!({ "status": 200_u16, "body": json!({ "id": id }).to_string() }))
        .collect();
    assert_eq!(ran["typed"], Value::Array(typed), "{printed}");
    assert_eq!(ran["unread"]["status"], 400_u16, "{printed}");
    let refused: Value = serde_json::from_str(ran["unread"]["body"].as_str().unwrap()).unwrap();
    assert_eq!(refused["issues"][0]["field"], "page_size", "{printed}");
}

#[cfg(feature = "dart")]
#[test]
fn the_dart_client_carries_the_field_under_its_key() {
    let module = [
        "import 'dart:async';".to_owned(),
        "import 'dart:convert';".to_owned(),
        keyed_answer_dart::dart_definition(),
        keyed_refused_dart::dart_definition(),
        keyed_plain_dart::dart_definition(),
        keyed_renamed_dart::dart_definition(),
        keyed_own_dart::dart_definition(),
        keyed_quoted_dart::dart_definition(),
        keyed_extra_dart::dart_definition(),
        keyed_beside_dart::dart_definition(),
        keyed_paged_dart::dart_definition(),
        keyed_pinned_dart::dart_definition(),
        keyed_tally_dart::dart_definition(),
        keyed_listed_dart::dart_definition(),
        keyed_labelled_dart::dart_definition(),
        KeyedPathClientServiceSchema::dart_definition(),
        KeyedPathClientServiceSchema::dart_http_client(),
        DART_DRIVER.to_owned(),
    ]
    .join("\n\n");
    let Some(printed) = ran("dart", "TIXSCHEMA_DART", "dart", "client.dart", &module) else {
        return;
    };
    assert_printed(&printed);
    let Some((clean, said)) = dart("renamed-paths-analyze", &["analyze"], &module) else {
        return;
    };
    assert!(clean && said.contains("No issues found!"), "{said}");
}

#[cfg(feature = "swift")]
#[test]
fn the_swift_client_carries_the_field_under_its_key() {
    use keyed_path_client_service_schema::{
        keyed_path_client_service_fault_fields_swift, keyed_path_client_service_fault_kind_swift,
    };

    let module = [
        "import Foundation".to_owned(),
        keyed_answer_swift::swift_definition(),
        keyed_refused_swift::swift_definition(),
        keyed_plain_swift::swift_definition(),
        keyed_renamed_swift::swift_definition(),
        keyed_own_swift::swift_definition(),
        keyed_quoted_swift::swift_definition(),
        keyed_extra_swift::swift_definition(),
        keyed_beside_swift::swift_definition(),
        keyed_paged_swift::swift_definition(),
        keyed_pinned_swift::swift_definition(),
        keyed_tally_swift::swift_definition(),
        keyed_listed_swift::swift_definition(),
        keyed_labelled_swift::swift_definition(),
        keyed_generated_request_swift::swift_definition(),
        keyed_gen_labelled_request_swift::swift_definition(),
        keyed_gen_list_request_swift::swift_definition(),
        keyed_path_client_service_fault_fields_swift::swift_definition(),
        keyed_path_client_service_fault_kind_swift::swift_definition(),
        KeyedPathClientServiceSchema::swift_http_client(),
        SWIFT_DRIVER.to_owned(),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-renamed-paths",
        "TIXSCHEMA_SWIFT",
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &module,
    ) else {
        return;
    };
    assert!(
        run.succeeded,
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}",
        run.stdout, run.stderr
    );
    assert_printed(&run.stdout);
}

#[cfg(feature = "kotlin")]
#[test]
fn the_kotlin_client_carries_the_field_under_its_key() {
    use keyed_path_client_service_schema::{
        keyed_path_client_service_fault_fields_kotlin, keyed_path_client_service_fault_kind_kotlin,
    };

    let module = [
        KOTLIN_IMPORTS.to_owned(),
        keyed_answer_kotlin::kotlin_definition(),
        keyed_refused_kotlin::kotlin_definition(),
        keyed_plain_kotlin::kotlin_definition(),
        keyed_renamed_kotlin::kotlin_definition(),
        keyed_own_kotlin::kotlin_definition(),
        keyed_quoted_kotlin::kotlin_definition(),
        keyed_extra_kotlin::kotlin_definition(),
        keyed_beside_kotlin::kotlin_definition(),
        keyed_paged_kotlin::kotlin_definition(),
        keyed_pinned_kotlin::kotlin_definition(),
        keyed_tally_kotlin::kotlin_definition(),
        keyed_listed_kotlin::kotlin_definition(),
        keyed_labelled_kotlin::kotlin_definition(),
        keyed_generated_request_kotlin::kotlin_definition(),
        keyed_gen_labelled_request_kotlin::kotlin_definition(),
        keyed_gen_list_request_kotlin::kotlin_definition(),
        keyed_path_client_service_fault_fields_kotlin::kotlin_definition(),
        keyed_path_client_service_fault_kind_kotlin::kotlin_definition(),
        KeyedPathClientServiceSchema::kotlin_http_client(),
        KOTLIN_DRIVER.to_owned(),
    ]
    .join("\n\n");
    let Some(printed) = ran_kotlin(&module) else {
        return;
    };
    assert_printed(&printed);
}
