//! Where an HTTP host of an emitted `mcp` dispatcher streams to: every call kept in order, with
//! the status and headers the answer began with.

use alloc::collections::VecDeque;
use alloc::sync::Arc;
use core::future::poll_fn;
use core::task::{Poll, Waker};
use serde_json::{Value, json};
use std::sync::Mutex;

/// Implements, in the module a dispatcher was expanded into, what `Recorded` needs to be the
/// stream of its HTTP entry points.
macro_rules! streams_here {
    () => {
        impl McpEventWriter for $crate::mcp_streams::Recorded {
            fn write(
                &self,
                message: String,
            ) -> impl ::core::future::Future<Output = Result<(), McpClientGone>> + Send {
                ::core::future::ready(
                    self.record($crate::mcp_streams::Seen::Wrote(message))
                        .then_some(())
                        .ok_or(McpClientGone),
                )
            }
        }

        impl McpHttpStream for $crate::mcp_streams::Recorded {
            fn begin(
                &self,
                status: u16,
                headers: &[(String, String)],
            ) -> impl ::core::future::Future<Output = Result<(), McpClientGone>> + Send {
                ::core::future::ready(
                    self.record($crate::mcp_streams::Seen::Began(status, headers.to_vec()))
                        .then_some(())
                        .ok_or(McpClientGone),
                )
            }
        }

        impl From<$crate::mcp_streams::Streamed> for McpHttpResponse {
            fn from(streamed: $crate::mcp_streams::Streamed) -> Self {
                Self {
                    body: streamed.body,
                    headers: streamed.headers,
                    status: streamed.status,
                }
            }
        }
    };
}

/// The function a node driver reaches an HTTP dispatcher through: the response it answers, or the
/// one the stream it hands in received where it answers `undefined`.
pub const NODE_RESPOND: &str = r#"
const respond = async (dispatch, input) => {
  const seen = [];
  const stream = {
    async begin(status, headers) { seen.push({ status, headers }); },
    async write(event) { seen.push({ event }); },
  };
  const response = await dispatch(input, stream);
  if (response !== undefined) return response;
  const [began, ...events] = seen;
  return { status: began.status, headers: began.headers, body: new TextEncoder().encode(events.map((held) => held.event).join("")) };
};
"#;

/// One call the dispatcher made on an HTTP stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Seen {
    Began(u16, Vec<(String, String)>),
    Wrote(String),
}

/// What a stream received, put together the way a response carries it.
pub struct Streamed {
    pub body: Vec<u8>,
    pub headers: Vec<(String, String)>,
    pub status: u16,
}

/// A stream that keeps every call it takes, and refuses every call once it has taken `lasts` of
/// them.
#[derive(Default)]
pub struct Recorded {
    lasts: Option<usize>,
    log: Mutex<Vec<Seen>>,
}

struct Fed<Change> {
    closed: bool,
    pending: VecDeque<Change>,
    waiting: Option<Waker>,
}

/// A source of changes the test feeds one at a time: `next` waits for the test, with nothing to
/// time out.
pub struct Feed<Change> {
    held: Arc<Mutex<Fed<Change>>>,
}

impl Recorded {
    /// A stream whose client goes away after `calls` calls.
    pub fn gone_after(calls: usize) -> Self {
        Self {
            lasts: Some(calls),
            log: Mutex::new(Vec::new()),
        }
    }

    pub fn record(&self, seen: Seen) -> bool {
        let mut log = self.log.lock().unwrap();
        if self.lasts.is_some_and(|lasts| log.len() >= lasts) {
            return false;
        }
        log.push(seen);
        true
    }

    /// Every call the stream took, in the shape each language's program reports.
    pub fn report(&self) -> Value {
        Value::Array(
            self.seen()
                .into_iter()
                .map(|seen| match seen {
                    Seen::Began(status, headers) => json!({
                        "began": [status, headers.iter().map(|(name, value)| json!([name, value])).collect::<Vec<_>>()],
                    }),
                    Seen::Wrote(text) => json!({ "wrote": text }),
                })
                .collect(),
        )
    }

    /// Every call the stream took, in order.
    pub fn seen(&self) -> Vec<Seen> {
        self.log.lock().unwrap().clone()
    }

    /// What the stream was begun with, and everything written to it.
    pub fn streamed(&self) -> Streamed {
        let (status, headers) = self
            .seen()
            .into_iter()
            .find_map(|seen| match seen {
                Seen::Began(status, headers) => Some((status, headers)),
                Seen::Wrote(_) => None,
            })
            .unwrap();
        Streamed {
            body: self.wrote().concat().into_bytes(),
            headers,
            status,
        }
    }

    /// Every text written, in order.
    pub fn wrote(&self) -> Vec<String> {
        self.seen()
            .into_iter()
            .filter_map(|seen| match seen {
                Seen::Wrote(text) => Some(text),
                Seen::Began(..) => None,
            })
            .collect()
    }
}

impl<Change> Default for Feed<Change> {
    fn default() -> Self {
        Self {
            held: Arc::new(Mutex::new(Fed {
                closed: false,
                pending: VecDeque::new(),
                waiting: None,
            })),
        }
    }
}

impl<Change> Feed<Change> {
    /// Ends the stream the feed is the source of.
    pub fn close(&self) {
        let mut fed = self.held.lock().unwrap();
        fed.closed = true;
        if let Some(waker) = fed.waiting.take() {
            waker.wake();
        }
    }

    pub async fn next(&self) -> Option<Change> {
        poll_fn(|cx| {
            let mut fed = self.held.lock().unwrap();
            if let Some(change) = fed.pending.pop_front() {
                return Poll::Ready(Some(change));
            }
            if fed.closed {
                return Poll::Ready(None);
            }
            fed.waiting = Some(cx.waker().clone());
            Poll::Pending
        })
        .await
    }

    /// Hands one change to the stream.
    pub fn push(&self, change: Change) {
        let mut fed = self.held.lock().unwrap();
        fed.pending.push_back(change);
        if let Some(waker) = fed.waiting.take() {
            waker.wake();
        }
    }
}

/// The Dart function a driver reaches the HTTP dispatcher of the service `prefix` through: the
/// response it answers, or the one the stream it hands in received where it answers `null`.
#[cfg(feature = "dart")]
pub fn dart_respond(prefix: &str) -> String {
    "
class @P@Collected implements @P@McpHttpStream {
  int? status;
  List<(String, String)> headers = const [];
  final List<String> events = <String>[];

  @override
  Future<void> begin(int status, List<(String, String)> headers) async {
    this.status = status;
    this.headers = headers;
  }

  @override
  Future<void> write(String event) async => events.add(event);
}

Future<@P@McpHttpResponse> respond@P@(
  Future<@P@McpHttpResponse?> Function(@P@McpHttpRequest, @P@McpHttpStream) serve,
  @P@McpHttpRequest request,
) async {
  final stream = @P@Collected();
  final response = await serve(request, stream);
  return response ??
      @P@McpHttpResponse(status: stream.status!, headers: stream.headers, body: utf8.encode(stream.events.join()));
}
"
    .replace("@P@", prefix)
}

/// The Kotlin function a driver reaches the HTTP dispatcher of the service `prefix` through: the
/// response it answers, or the one the stream it hands in received where it answers `null`.
#[cfg(feature = "kotlin")]
pub fn kotlin_respond(prefix: &str) -> String {
    r#"
class @P@Collected : @P@McpHttpStream {
    var status = 0
    var headers: List<Pair<String, String>> = emptyList()
    val events = mutableListOf<String>()

    override suspend fun begin(status: Int, headers: List<Pair<String, String>>) {
        this.status = status
        this.headers = headers
    }

    override suspend fun write(event: String) {
        events.add(event)
    }
}

suspend fun respond@P@(
    serve: suspend (@P@McpHttpRequest, @P@McpHttpStream) -> @P@McpHttpResponse?,
    request: @P@McpHttpRequest,
): @P@McpHttpResponse {
    val stream = @P@Collected()
    return serve(request, stream)
        ?: @P@McpHttpResponse(stream.status, stream.headers, stream.events.joinToString("").encodeToByteArray())
}
"#
    .replace("@P@", prefix)
}

/// The response of a call that is handed a fresh stream: what it answers where it answers, and
/// what the stream received where it does not.
pub fn through<Response>(answering: impl FnOnce(&Recorded) -> Option<Response>) -> Response
where
    Response: From<Streamed>,
{
    let stream = Recorded::default();
    answering(&stream).unwrap_or_else(|| stream.streamed().into())
}
