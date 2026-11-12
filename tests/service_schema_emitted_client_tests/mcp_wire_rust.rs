//! What the Rust dispatchers and clients of `ShelfWireService` write and read on every transport
//! for an operation answering content blocks and one answering prompt messages: the block form
//! the `mcp` transport writes. Nothing here waits on a clock.

use super::ShelfWireServiceSchema;
use super::mcp_streams::through;
use super::mcp_wire_service::{
    ShelfCountDraft, ShelfWireRequestDraft, Stand, blocks, messages, page, task,
};
use super::mcp_wire_service::{ShelfWireErrorDraft, shelf_wire_service_schema};
use super::{
    mcp_wire_amqp_client, mcp_wire_amqp_transport, mcp_wire_http_rest_client,
    mcp_wire_http_rest_transport, mcp_wire_mcp_transport, mcp_wire_ws_rpc_client,
    mcp_wire_ws_rpc_transport,
};
use alloc::sync::Arc;
use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context, Poll, Waker};
use serde::Serialize;
use serde_json::{Value, json};
use shelf_wire_service_schema::{McpContent, McpPromptMessage};
use std::sync::Mutex;

pub const BLOCKS: &str = concat!(
    r#"[{"text":"Eleven books.","type":"text"},"#,
    r#"{"data":"iVBORw==","mimeType":"image/png","type":"image"},"#,
    r#"{"data":"//uQ","mimeType":"audio/mpeg","type":"audio"},"#,
    r#"{"mimeType":"text/markdown","name":"history","title":"History","type":"resource_link","uri":"shelf://history"},"#,
    r##"{"resource":{"mimeType":"text/markdown","text":"# History","uri":"shelf://history/index"},"type":"resource"},"##,
    r#"{"resource":{"blob":"CQgH","uri":"shelf://history/cover"},"type":"resource"}]"#,
);

pub const PAGE: &str = r#"{"resources":[{"uri":"shelf://history","name":"history","mimeType":"text/markdown","size":11}],"nextCursor":"page-2"}"#;

pub const MESSAGES: &str = concat!(
    r#"[{"content":{"text":"Review the history shelf.","type":"text"},"role":"user"},"#,
    r#"{"content":{"text":"Which books?","type":"text"},"role":"assistant"}]"#,
);

struct HttpLoop;

impl mcp_wire_http_rest_client::Transport for HttpLoop {
    async fn send(
        &self,
        request: mcp_wire_http_rest_client::OutgoingRequest,
    ) -> Result<mcp_wire_http_rest_client::IncomingResponse, String> {
        let incoming = mcp_wire_http_rest_transport::IncomingRequest::new(
            request.method().to_owned(),
            request.path().to_owned(),
            request.query().to_owned(),
            request.headers().to_vec(),
            request.body().to_vec(),
        );
        let answered = mcp_wire_http_rest_transport::dispatch(
            &Stand,
            &(),
            &incoming,
            &mcp_wire_http_rest_transport::DefaultFaultHandler,
        )
        .await;
        Ok(mcp_wire_http_rest_client::IncomingResponse::new(
            answered.status(),
            answered.headers().to_vec(),
            answered.body().to_vec(),
        ))
    }
}

/// Answers every fault with the kind it names, so an owner's override is what a loop built on it
/// answers with.
struct NamedFaultHandler;

impl mcp_wire_http_rest_transport::FaultHandler for NamedFaultHandler {
    fn on_fault(
        &self,
        fault: &shelf_wire_service_schema::ServiceFault,
    ) -> mcp_wire_http_rest_transport::OutgoingResponse {
        mcp_wire_http_rest_transport::OutgoingResponse::new(
            490,
            Vec::new(),
            format!("{}", fault.kind()).into_bytes(),
        )
    }
}

/// What one AMQP reply carries.
type AmqpAnswer = (Vec<u8>, Vec<(String, String)>);

struct AmqpCapture {
    settled: Mutex<Option<AmqpAnswer>>,
}

impl mcp_wire_amqp_transport::Reply for AmqpCapture {
    async fn fault(&self, fault: shelf_wire_service_schema::ServiceFault) {
        ready(()).await;
        let framed = json!({ "ok": false, "error": { "isServiceFault": true, "fault": fault } });
        *self.settled.lock().unwrap() = Some((serde_json::to_vec(&framed).unwrap(), Vec::new()));
    }

    async fn send<T>(&self, value: T, headers: Vec<(String, String)>)
    where
        T: Serialize + Send,
    {
        ready(()).await;
        *self.settled.lock().unwrap() = Some((serde_json::to_vec(&value).unwrap(), headers));
    }
}

/// Joins the `amqp_rpc` client and dispatcher: a call is turned straight into a dispatch, and the
/// answer handed back is what the dispatcher wrote.
struct AmqpLoop {
    written: Mutex<Vec<String>>,
}

impl mcp_wire_amqp_client::Transport for AmqpLoop {
    async fn notify<T>(
        &self,
        _operation: &str,
        _payload: T,
        _headers: Vec<(String, String)>,
    ) -> Result<(), String>
    where
        T: Serialize + Send,
    {
        ready(()).await;
        Ok(())
    }

    async fn request<T>(
        &self,
        operation: &str,
        payload: T,
        headers: Vec<(String, String)>,
    ) -> Result<AmqpAnswer, String>
    where
        T: Serialize + Send,
    {
        let capture = AmqpCapture {
            settled: Mutex::new(None),
        };
        mcp_wire_amqp_transport::dispatch(
            &Stand,
            &(),
            &mcp_wire_amqp_transport::IncomingMessage::new(
                operation.to_owned(),
                serde_json::to_vec(&payload).unwrap(),
                headers,
            ),
            &capture,
        )
        .await;
        let answer = capture.settled.lock().unwrap().take().unwrap();
        self.written
            .lock()
            .unwrap()
            .push(String::from_utf8(answer.0.clone()).unwrap());
        Ok(answer)
    }
}

struct Nowhere;

impl mcp_wire_mcp_transport::McpEventWriter for Nowhere {
    fn write(
        &self,
        _message: String,
    ) -> impl Future<Output = Result<(), mcp_wire_mcp_transport::McpClientGone>> {
        ready(Ok(()))
    }
}

pub fn once<Answering>(answering: Answering) -> Answering::Output
where
    Answering: Future,
{
    match pin!(answering).poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(output) => Some(output),
        Poll::Pending => None,
    }
    .unwrap()
}

/// The frame a client sends for `operation`, and the reply the Rust `ws_rpc` dispatcher gives it.
pub fn over_ws(operation: &str, payload: &Value) -> (String, String) {
    let frame = json!({
        "kind": "request",
        "id": "ID",
        "service": "ShelfWireService",
        "operation": operation,
        "payload": payload,
    })
    .to_string();
    let reply = once(mcp_wire_ws_rpc_transport::answer(&frame, &Stand, &())).unwrap();
    (frame, reply)
}

/// The status and body the Rust `http_rest` dispatcher gives a `POST` of `payload` to `path`.
pub fn over_http(path: &str, payload: &Value) -> (u16, String) {
    let incoming = mcp_wire_http_rest_transport::IncomingRequest::new(
        "POST".to_owned(),
        path.to_owned(),
        String::new(),
        vec![("content-type".to_owned(), "application/json".to_owned())],
        payload.to_string().into_bytes(),
    );
    let response = once(mcp_wire_http_rest_transport::dispatch(
        &Stand,
        &(),
        &incoming,
        &mcp_wire_http_rest_transport::DefaultFaultHandler,
    ));
    (
        response.status(),
        String::from_utf8(response.body().to_vec()).unwrap(),
    )
}

fn over_mcp(method: &str, params: Value) -> Value {
    let mut carried = params;
    carried["_meta"] = json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientCapabilities": {},
    });
    let request = json!({ "jsonrpc": "2.0", "id": 1_u8, "method": method, "params": carried });
    let reply = once(mcp_wire_mcp_transport::answer(
        &request.to_string(),
        &Stand,
        |_call| (),
        &Nowhere,
        &mcp_wire_mcp_transport::McpInFlight::default(),
    ))
    .unwrap();
    serde_json::from_str(&reply).unwrap()
}

fn written(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

#[test]
fn the_ws_rpc_dispatcher_answers_the_blocks_in_the_mcp_block_form() {
    let (_, reply) = over_ws("describe-shelf", &json!({ "shelf": "history" }));
    assert!(
        reply.contains(&format!(r#""ok":true,"value":{BLOCKS},"#)),
        "{reply}"
    );
}

#[test]
fn the_ws_rpc_dispatcher_answers_the_messages_in_the_mcp_message_form() {
    let (_, reply) = over_ws("review-shelf", &json!("history"));
    assert!(
        reply.contains(&format!(r#""ok":true,"value":{MESSAGES},"#)),
        "{reply}"
    );
}

#[test]
fn the_http_rest_dispatcher_answers_the_blocks_in_the_mcp_block_form() {
    let (status, body) = over_http("/describe-shelf", &json!({ "shelf": "history" }));
    assert_eq!((status, body.as_str()), (200, BLOCKS));
}

#[test]
fn the_http_rest_dispatcher_answers_the_messages_in_the_mcp_message_form() {
    let (status, body) = over_http("/review-shelf", &json!("history"));
    assert_eq!((status, body.as_str()), (200, MESSAGES));
}

#[test]
fn the_http_rest_client_reads_the_blocks_and_the_messages_the_dispatcher_writes() {
    let client = mcp_wire_http_rest_client::ShelfWireServiceClient::new(HttpLoop);
    let described = once(
        client.describe_shelf(
            ShelfWireRequestDraft {
                shelf: "history".to_owned(),
            }
            .validate()
            .unwrap(),
        ),
    );
    assert_eq!(described, Ok(blocks()));
    let reviewed = once(client.review_shelf("history".to_owned()));
    assert_eq!(reviewed, Ok(messages()));
    let missing = once(client.review_shelf("fiction".to_owned()));
    assert_eq!(
        missing,
        Err(shelf_wire_service_schema::CallError::Operation(
            ShelfWireErrorDraft::NotFound.validate().unwrap()
        ))
    );
}

#[test]
fn the_amqp_rpc_dispatcher_and_client_carry_the_blocks_and_the_messages() {
    let transport = AmqpLoop {
        written: Mutex::new(Vec::new()),
    };
    let client = mcp_wire_amqp_client::ShelfWireServiceClient::new(transport);
    let described = once(
        client.describe_shelf(
            ShelfWireRequestDraft {
                shelf: "history".to_owned(),
            }
            .validate()
            .unwrap(),
        ),
    );
    assert_eq!(described, Ok(blocks()));
    let reviewed = once(client.review_shelf("history".to_owned()));
    assert_eq!(reviewed, Ok(messages()));
}

#[test]
fn the_ws_rpc_client_reads_the_blocks_and_the_messages_the_dispatcher_writes() {
    let sent = Arc::new(Mutex::new(Vec::<String>::new()));
    let mailbox = Arc::clone(&sent);
    let session = mcp_wire_ws_rpc_client::FrameSession::new(move |text| {
        mailbox.lock().unwrap().push(text);
        ready(Ok(()))
    });
    let client = mcp_wire_ws_rpc_client::ShelfWireServiceClient::new(session.clone());
    let mut described = pin!(
        client.describe_shelf(
            ShelfWireRequestDraft {
                shelf: "history".to_owned(),
            }
            .validate()
            .unwrap()
        )
    );
    let context = &mut Context::from_waker(Waker::noop());
    assert!(described.as_mut().poll(context).is_pending());
    let frame = sent.lock().unwrap().remove(0);
    let reply = once(mcp_wire_ws_rpc_transport::answer(&frame, &Stand, &())).unwrap();
    once(session.deliver(&reply));
    assert_eq!(described.as_mut().poll(context), Poll::Ready(Ok(blocks())));
    let mut reviewed = pin!(client.review_shelf("history".to_owned()));
    assert!(reviewed.as_mut().poll(context).is_pending());
    let review_frame = sent.lock().unwrap().remove(0);
    let review_reply = once(mcp_wire_ws_rpc_transport::answer(
        &review_frame,
        &Stand,
        &(),
    ))
    .unwrap();
    once(session.deliver(&review_reply));
    assert_eq!(reviewed.as_mut().poll(context), Poll::Ready(Ok(messages())));
    let mut listed = pin!(client.list_shelves(None));
    assert!(listed.as_mut().poll(context).is_pending());
    let list_frame = sent.lock().unwrap().remove(0);
    let list_reply = once(mcp_wire_ws_rpc_transport::answer(&list_frame, &Stand, &())).unwrap();
    once(session.deliver(&list_reply));
    assert_eq!(listed.as_mut().poll(context), Poll::Ready(Ok(page())));
}

#[test]
fn the_mcp_dispatcher_writes_the_blocks_and_the_messages_in_the_same_form() {
    let called = over_mcp(
        "tools/call",
        json!({ "name": "describe-shelf", "arguments": { "shelf": "history" } }),
    );
    assert_eq!(called["result"]["content"], written(BLOCKS));
    let got = over_mcp(
        "prompts/get",
        json!({ "name": "review-shelf", "arguments": { "shelfName": "history" } }),
    );
    assert_eq!(got["result"]["messages"], written(MESSAGES));
}

#[test]
fn a_block_that_is_no_block_is_refused_with_the_reason() {
    for (text, reason) in [
        (r#"{"type":"video"}"#, "`video` is no content block"),
        (r#"{"text":"x"}"#, "a content block holds no `type`"),
        (r#"{"type":"text"}"#, "a content block holds no `text`"),
        (
            r#"{"type":"image","data":"%","mimeType":"image/png"}"#,
            "`data` is not base64",
        ),
    ] {
        let refused = serde_json::from_str::<McpContent>(text).unwrap_err();
        assert!(refused.to_string().contains(reason), "{text}: {refused}");
    }
    serde_json::from_str::<McpPromptMessage>(
        r#"{"content":{"type":"text","text":"x"},"role":"system"}"#,
    )
    .unwrap_err();
}

#[test]
fn the_mcp_dispatcher_answers_alone_beside_an_endpoint_and_over_http_alike() {
    let (alone, served, status) =
        rust_serves_alike!(mcp_wire_mcp_transport, ShelfWireServiceSchema, Stand, once);
    assert_eq!(alone, served);
    assert!(alone.is_some());
    assert_eq!(status, 200);
}

#[test]
fn the_mcp_dispatcher_listens_alike_through_every_entry_point() {
    let (alone, served) =
        rust_listens_alike!(mcp_wire_mcp_transport, ShelfWireServiceSchema, Stand, once);
    assert_eq!(alone, served);
    assert!(alone.is_some());
}

#[test]
fn an_admit_that_denies_stops_a_call_before_its_handler_runs() {
    let settings = mcp_wire_mcp_transport::McpHttpSettings::new(&[], None);
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1_u8,
        "method": "tools/call",
        "params": {
            "name": "describe-shelf",
            "arguments": { "shelf": "history" },
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientCapabilities": {},
            },
        },
    })
    .to_string();
    let asked = mcp_wire_mcp_transport::McpHttpRequest::new(
        "POST".to_owned(),
        "/mcp".to_owned(),
        vec![
            ("Content-Type".to_owned(), "application/json".to_owned()),
            ("MCP-Protocol-Version".to_owned(), "2026-07-28".to_owned()),
            ("Mcp-Method".to_owned(), "tools/call".to_owned()),
            ("Mcp-Name".to_owned(), "describe-shelf".to_owned()),
        ],
        body.into_bytes(),
    );
    for (denied, status) in [
        (mcp_wire_mcp_transport::McpDenied::Forbidden, 403_u16),
        (
            mcp_wire_mcp_transport::McpDenied::InsufficientScope {
                scope: "shelves:read".to_owned(),
            },
            403,
        ),
        (
            mcp_wire_mcp_transport::McpDenied::NoValidCredentials { scope: None },
            401,
        ),
    ] {
        let response = through(|stream| {
            once(mcp_wire_mcp_transport::answer_http(
                &asked,
                &settings,
                stream,
                &Stand,
                async |request: &mcp_wire_mcp_transport::McpHttpRequest,
                       _call: mcp_wire_mcp_transport::McpCall| {
                    assert_eq!(request.mcp_name().as_deref(), Some("describe-shelf"));
                    assert_eq!(request.bearer_token(), None);
                    assert_eq!(request.header("mcp-method"), Some("tools/call"));
                    Err::<mcp_wire_mcp_transport::McpAdmitted<()>, _>(denied.clone())
                },
            ))
        });
        assert_eq!(response.status(), status);
    }
}

#[test]
fn the_http_rest_route_table_names_each_operation_by_its_wire_name() {
    let routes: Vec<_> = mcp_wire_http_rest_transport::ROUTES
        .iter()
        .map(|route| {
            (
                route.operation(),
                route.method(),
                route.path(),
                route.ok_status(),
                route.error_statuses().to_vec(),
            )
        })
        .collect();
    assert_eq!(
        routes,
        [
            ("count-status", "POST", "/count-status", 200, vec![422]),
            ("describe-shelf", "POST", "/describe-shelf", 200, vec![422]),
            ("list-shelves", "POST", "/list-shelves", 200, vec![422]),
            ("review-shelf", "POST", "/review-shelf", 200, vec![422]),
            ("start-count", "POST", "/start-count", 200, vec![422]),
        ]
    );
}

#[test]
fn the_transports_hand_their_parts_to_the_caller() {
    let request = mcp_wire_http_rest_transport::IncomingRequest::new(
        "POST".to_owned(),
        "/review-shelf".to_owned(),
        "a=1".to_owned(),
        vec![("x-trace".to_owned(), "7".to_owned())],
        Vec::new(),
    );
    assert_eq!(request.query(), "a=1");
    assert_eq!(request.headers().len(), 1);
    assert_eq!(request.header("x-trace"), Some("7"));
    let response = mcp_wire_http_rest_client::IncomingResponse::new(
        200,
        vec![("etag".to_owned(), "v1".to_owned())],
        Vec::new(),
    );
    assert_eq!(response.header("etag"), Some("v1"));
    let amqp = AmqpLoop {
        written: Mutex::new(Vec::new()),
    };
    let client = mcp_wire_amqp_client::ShelfWireServiceClient::new(amqp);
    assert!(client.transport().written.lock().unwrap().is_empty());
    let sent = once(mcp_wire_amqp_client::Transport::notify(
        client.transport(),
        "purge-shelf",
        json!("history"),
        Vec::new(),
    ));
    assert_eq!(sent, Ok(()));
    let http = mcp_wire_http_rest_client::ShelfWireServiceClient::new(HttpLoop);
    let _bound: &HttpLoop = http.transport();
    let ping = mcp_wire_ws_rpc_client::ping_frame();
    assert!(ping.contains("ping"), "{ping}");
    let frames = Arc::new(Mutex::new(Vec::<String>::new()));
    let mailbox = Arc::clone(&frames);
    let writer = mcp_wire_ws_rpc_client::FrameWriter::new(move |text| {
        mailbox.lock().unwrap().push(text);
        ready(Ok(()))
    });
    let pushing = mcp_wire_ws_rpc_client::ShelfWireServiceClient::new(writer);
    let notified = once(mcp_wire_ws_rpc_client::Transport::notify(
        pushing.transport(),
        "purge-shelf",
        json!("history"),
        Vec::new(),
    ));
    assert_eq!(notified, Ok(()));
    assert_eq!(frames.lock().unwrap().len(), 1);
}

#[test]
fn an_owner_s_fault_handler_answers_an_operation_nobody_declared_and_a_closed_session_refuses() {
    let incoming = mcp_wire_http_rest_transport::IncomingRequest::new(
        "GET".to_owned(),
        "/nowhere".to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let response = once(mcp_wire_http_rest_transport::dispatch(
        &Stand,
        &(),
        &incoming,
        &NamedFaultHandler,
    ));
    assert_eq!(response.status(), 490);
    let session = mcp_wire_ws_rpc_client::FrameSession::new(|_text| ready(Ok(())));
    let client = mcp_wire_ws_rpc_client::ShelfWireServiceClient::new(session.clone());
    let mut waiting = pin!(client.review_shelf("history".to_owned()));
    let context = &mut Context::from_waker(Waker::noop());
    assert!(waiting.as_mut().poll(context).is_pending());
    session.close("the socket closed");
    assert!(matches!(
        waiting.as_mut().poll(context),
        Poll::Ready(Err(shelf_wire_service_schema::CallError::Fault(_)))
    ));
}

#[test]
fn the_listing_is_answered_on_every_transport_in_the_page_form() {
    let (status, body) = over_http("/list-shelves", &Value::Null);
    assert_eq!((status, body.as_str()), (200, PAGE));
    let (_, reply) = over_ws("list-shelves", &Value::Null);
    assert!(
        reply.contains(&format!(r#""ok":true,"value":{PAGE},"#)),
        "{reply}"
    );
    let http = mcp_wire_http_rest_client::ShelfWireServiceClient::new(HttpLoop);
    assert_eq!(once(http.list_shelves(None)), Ok(page()));
    let amqp = mcp_wire_amqp_client::ShelfWireServiceClient::new(AmqpLoop {
        written: Mutex::new(Vec::new()),
    });
    assert_eq!(once(amqp.list_shelves(None)), Ok(page()));
}

#[test]
fn a_task_operation_starts_and_reports_on_every_transport() {
    let started = json!({
        "taskId": "count-1",
        "createdAt": "2026-07-28T09:00:00Z",
        "lastUpdatedAt": "2026-07-28T09:00:00Z",
        "ttlMs": null,
    });
    let (status, body) = over_http("/start-count", &json!({ "shelf": "history" }));
    assert_eq!(status, 200);
    assert_eq!(written(&body), started);
    let (_, reply) = over_ws("start-count", &json!({ "shelf": "history" }));
    assert!(reply.contains(r#""ok":true"#), "{reply}");
    let http = mcp_wire_http_rest_client::ShelfWireServiceClient::new(HttpLoop);
    let amqp = mcp_wire_amqp_client::ShelfWireServiceClient::new(AmqpLoop {
        written: Mutex::new(Vec::new()),
    });
    let counted = || {
        Ok(shelf_wire_service_schema::McpTaskState::Completed {
            task: task(),
            result: ShelfCountDraft { books: 11 }.validate().unwrap(),
        })
    };
    let request = || ShelfWireRequestDraft {
        shelf: "history".to_owned(),
    };
    assert_eq!(
        once(http.start_count(request().validate().unwrap())),
        Ok(task())
    );
    assert_eq!(
        once(amqp.start_count(request().validate().unwrap())),
        Ok(task())
    );
    assert_eq!(once(http.count_status("count-1".to_owned())), counted());
    assert_eq!(once(amqp.count_status("count-1".to_owned())), counted());
    let sent = Arc::new(Mutex::new(Vec::<String>::new()));
    let mailbox = Arc::clone(&sent);
    let session = mcp_wire_ws_rpc_client::FrameSession::new(move |text| {
        mailbox.lock().unwrap().push(text);
        ready(Ok(()))
    });
    let client = mcp_wire_ws_rpc_client::ShelfWireServiceClient::new(session.clone());
    let context = &mut Context::from_waker(Waker::noop());
    let mut starting = pin!(client.start_count(request().validate().unwrap()));
    assert!(starting.as_mut().poll(context).is_pending());
    let frame = sent.lock().unwrap().remove(0);
    once(session.deliver(&once(mcp_wire_ws_rpc_transport::answer(&frame, &Stand, &())).unwrap()));
    assert_eq!(starting.as_mut().poll(context), Poll::Ready(Ok(task())));
    let mut counting = pin!(client.count_status("count-1".to_owned()));
    assert!(counting.as_mut().poll(context).is_pending());
    let count_frame = sent.lock().unwrap().remove(0);
    let count_reply = once(mcp_wire_ws_rpc_transport::answer(&count_frame, &Stand, &())).unwrap();
    once(session.deliver(&count_reply));
    assert_eq!(counting.as_mut().poll(context), Poll::Ready(counted()));
}
