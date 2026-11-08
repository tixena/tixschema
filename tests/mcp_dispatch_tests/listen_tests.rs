//! `subscriptions/listen`: the acknowledgment first, then only what the client asked for, the
//! graceful end, and the event stream over HTTP. Nothing here waits on a clock; each stream ends
//! on the changes it is handed.

listens_through_every_entry_point_of!(
    the_document_dispatcher_listens_through_every_entry_point,
    document_mcp,
    DocumentServiceSchema,
    Archive::default(),
    |_call| ()
);
listens_through_every_entry_point_of!(
    the_billing_dispatcher_listens_through_every_entry_point,
    billing_mcp,
    BillingServiceSchema,
    Archive::default(),
    |_call| ()
);
listens_through_every_entry_point_of!(
    the_export_dispatcher_listens_through_every_entry_point,
    export_mcp,
    ExportServiceSchema,
    Archive::default(),
    |_call| Reporting::new(|_, _, _| {}, || false)
);
listens_through_every_entry_point_of!(
    the_approval_dispatcher_listens_through_every_entry_point,
    approval_mcp,
    ApprovalServiceSchema,
    Approvals::default(),
    |_call| ()
);
listens_through_every_entry_point_of!(
    the_counsel_dispatcher_listens_through_every_entry_point,
    counsel_mcp,
    CounselServiceSchema,
    Counsels::default(),
    |_call| ()
);

use crate::approval::Approvals;
use crate::counsel::Counsels;
use crate::document_mcp::{self, McpChange, McpEndpoint, McpHttpRequest, McpHttpSettings};
use crate::support::{
    Feed, Lines, Recorded, Seen, Tasks, answer, assert_error, assert_result, assert_valid,
    block_on, id, request, server_info, through,
};
use crate::tests::{Archive, Reporting};
use crate::{
    ApprovalServiceSchema, BillingServiceSchema, CounselServiceSchema, DocumentServiceSchema,
    ExportServiceSchema, approval_mcp, billing_mcp, counsel_mcp, export_mcp,
};
use alloc::collections::VecDeque;
use core::future::pending;
use serde_json::{Value, json};
use std::sync::Mutex;

const WATCHED: &str = "workspace://documents/64de3d95ff45b119e5b53a01";
const OTHER: &str = "workspace://documents/another";

const SETTINGS: McpHttpSettings<'static> = McpHttpSettings::new(&[], None);

fn listen(filter: &Value) -> Value {
    let asked = request(
        &id(1),
        "subscriptions/listen",
        json!({ "notifications": filter }),
    );
    assert_valid("SubscriptionsListenRequest", &asked);
    asked
}

fn updated(uri: &str) -> McpChange {
    McpChange::ResourceUpdated {
        uri: uri.to_owned(),
    }
}

fn listened(asked: &Value, changes: Vec<McpChange>, lines: &Lines) -> Option<Value> {
    let mut queue = VecDeque::from(changes);
    let mut source = async || queue.pop_front();
    block_on(document_mcp::answer_listening(
        &asked.to_string(),
        &Archive::default(),
        |_call| (),
        lines,
        &document_mcp::McpInFlight::default(),
        &mut source,
    ))
    .map(|text| serde_json::from_str(&text).unwrap())
}

fn written(lines: &Lines) -> Vec<Value> {
    lines
        .written()
        .iter()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn acknowledgment(notifications: &Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "method": "notifications/subscriptions/acknowledged",
        "params": {
            "_meta": { "io.modelcontextprotocol/subscriptionId": 1_u64 },
            "notifications": notifications,
        },
    })
}

fn post(asked: &Value) -> McpHttpRequest {
    McpHttpRequest::new(
        "POST".to_owned(),
        "/mcp".to_owned(),
        vec![
            ("MCP-Protocol-Version".to_owned(), "2026-07-28".to_owned()),
            ("Mcp-Method".to_owned(), "subscriptions/listen".to_owned()),
        ],
        asked.to_string().into_bytes(),
    )
}

#[test]
fn the_acknowledgment_comes_first_and_only_what_was_asked_follows() {
    let asked = listen(&json!({ "resourceSubscriptions": [WATCHED] }));
    let lines = Lines::default();
    let answered = listened(
        &asked,
        vec![
            updated(OTHER),
            McpChange::ResourcesListChanged,
            updated(WATCHED),
            McpChange::KeepAlive,
            McpChange::Task {
                task_id: "export:1".to_owned(),
            },
        ],
        &lines,
    )
    .unwrap();
    let notifications = written(&lines);
    assert_eq!(
        notifications,
        vec![
            acknowledgment(&json!({ "resourceSubscriptions": [WATCHED] })),
            json!({
                "jsonrpc": "2.0",
                "method": "notifications/resources/updated",
                "params": {
                    "_meta": { "io.modelcontextprotocol/subscriptionId": 1_u64 },
                    "uri": WATCHED,
                },
            }),
        ]
    );
    assert_valid("SubscriptionsAcknowledgedNotification", &notifications[0]);
    assert_valid("ResourceUpdatedNotification", &notifications[1]);
    assert_valid("SubscriptionsListenResultResponse", &answered);
    assert_result("SubscriptionsListenResult", &answered);
    let mut meta = server_info("DocumentService");
    meta["io.modelcontextprotocol/subscriptionId"] = json!(1_u64);
    assert_eq!(
        answered["result"],
        json!({ "_meta": meta, "resultType": "complete" })
    );
}

#[test]
fn a_list_change_is_written_where_the_client_asked_for_it() {
    let asked = listen(&json!({ "resourcesListChanged": true }));
    let lines = Lines::default();
    listened(&asked, vec![McpChange::ResourcesListChanged], &lines).unwrap();
    let notifications = written(&lines);
    assert_eq!(
        notifications[0],
        acknowledgment(&json!({ "resourcesListChanged": true }))
    );
    assert_eq!(
        notifications[1],
        json!({
            "jsonrpc": "2.0",
            "method": "notifications/resources/list_changed",
            "params": { "_meta": { "io.modelcontextprotocol/subscriptionId": 1_u64 } },
        })
    );
    assert_valid("ResourceListChangedNotification", &notifications[1]);
    assert_eq!(notifications.len(), 2);
}

#[test]
fn the_tool_list_and_the_prompt_list_are_never_acknowledged() {
    let asked = listen(&json!({ "toolsListChanged": true, "promptsListChanged": true }));
    let lines = Lines::default();
    listened(&asked, vec![McpChange::ResourcesListChanged], &lines).unwrap();
    assert_eq!(written(&lines), vec![acknowledgment(&json!({}))]);
}

#[test]
fn a_source_with_nothing_to_say_ends_the_stream_gracefully() {
    let asked = listen(&json!({ "resourceSubscriptions": [] }));
    let lines = Lines::default();
    let answered = listened(&asked, Vec::new(), &lines).unwrap();
    assert_eq!(written(&lines), vec![acknowledgment(&json!({}))]);
    assert_valid("SubscriptionsListenResultResponse", &answered);
    assert_eq!(answered["id"], 1_u64);
}

#[test]
fn a_request_that_names_no_notifications_is_refused_before_anything_is_written() {
    let mut asked = request(&id(1), "subscriptions/listen", json!({}));
    let lines = Lines::default();
    let missing = listened(&asked, Vec::new(), &lines).unwrap();
    assert_error("JSONRPCErrorResponse", &missing);
    assert_eq!(missing["error"]["code"], -32_602_i64);
    asked["params"]["notifications"] = json!({ "resourceSubscriptions": [7_u8] });
    let not_text = listened(&asked, Vec::new(), &lines).unwrap();
    assert_eq!(not_text["error"]["code"], -32_602_i64);
    asked["params"]["notifications"] = json!({ "resourcesListChanged": "yes" });
    let not_boolean = listened(&asked, Vec::new(), &lines).unwrap();
    assert_eq!(not_boolean["error"]["code"], -32_602_i64);
    assert_eq!(lines.written(), Vec::<String>::new());
}

#[test]
fn discover_offers_list_changes_and_subscriptions_only_where_changes_are_handed_in() {
    let asked = request(&id(1), "server/discover", json!({}));
    let mut queue = VecDeque::<McpChange>::new();
    let mut source = async || queue.pop_front();
    let listening: Value = serde_json::from_str(
        &block_on(document_mcp::answer_listening(
            &asked.to_string(),
            &Archive::default(),
            |_call| (),
            &Lines::default(),
            &document_mcp::McpInFlight::default(),
            &mut source,
        ))
        .unwrap(),
    )
    .unwrap();
    assert_result("DiscoverResult", &listening);
    assert_eq!(
        listening["result"]["capabilities"]["resources"],
        json!({ "listChanged": true, "subscribe": true })
    );
    assert_eq!(listening["result"]["capabilities"]["tools"], json!({}));
    let plain = answer(&Archive::default(), &asked).unwrap();
    assert_eq!(plain["result"]["capabilities"]["resources"], json!({}));
}

#[test]
fn a_service_that_is_handed_no_changes_does_not_know_the_method() {
    let asked = listen(&json!({ "resourcesListChanged": true }));
    let answered = answer(&Archive::default(), &asked).unwrap();
    assert_error("JSONRPCErrorResponse", &answered);
    assert_eq!(answered["error"]["code"], -32_601_i64);
}

#[test]
fn a_cancellation_ends_the_stream_and_drops_its_answer() {
    let asked = listen(&json!({ "resourcesListChanged": true }));
    let lines = Lines::default();
    let in_flight = document_mcp::McpInFlight::default();
    let archive = Archive::default();
    let mut source = async || pending::<Option<McpChange>>().await;
    let cancelled = json!({
        "jsonrpc": "2.0",
        "method": "notifications/cancelled",
        "params": { "requestId": 1_u64 },
    });
    assert_valid("CancelledNotification", &cancelled);
    let mut tasks = Tasks::default();
    tasks.spawn(async {
        let answered = document_mcp::answer_listening(
            &asked.to_string(),
            &archive,
            |_call| (),
            &lines,
            &in_flight,
            &mut source,
        )
        .await;
        assert_eq!(answered, None);
    });
    tasks.run_until_idle();
    assert_eq!((lines.written().len(), tasks.running()), (1, 1));
    tasks.spawn(async {
        let answered = document_mcp::answer(
            &cancelled.to_string(),
            &archive,
            |_call| (),
            &Lines::default(),
            &in_flight,
        )
        .await;
        assert_eq!(answered, None);
    });
    tasks.run_until_idle();
    assert_eq!((lines.written().len(), tasks.running()), (1, 0));
}

#[test]
fn a_write_that_fails_ends_the_stream() {
    let asked = listen(&json!({ "resourcesListChanged": true }));
    let lines = Lines::gone();
    let answered = listened(&asked, vec![McpChange::ResourcesListChanged], &lines);
    assert_eq!(answered, None);
    assert_eq!(lines.written(), Vec::<String>::new());
}

fn listen_over_http(asked: &Value, changes: Vec<McpChange>) -> document_mcp::McpHttpResponse {
    let mut queue = VecDeque::from(changes);
    let mut source = async || queue.pop_front();
    through(|stream| {
        block_on(document_mcp::answer_http_listening(
            &post(asked),
            &SETTINGS,
            stream,
            &Archive::default(),
            async |_request: &McpHttpRequest, _call: document_mcp::McpCall| {
                Ok::<_, document_mcp::McpDenied>(())
            },
            &mut source,
        ))
    })
}

#[test]
fn over_http_the_stream_holds_the_events_a_comment_and_the_end() {
    let asked = listen(&json!({ "resourceSubscriptions": [WATCHED] }));
    let response = listen_over_http(
        &asked,
        vec![McpChange::KeepAlive, updated(WATCHED), updated(OTHER)],
    );
    assert_eq!(response.status(), 200);
    assert_eq!(
        response.headers(),
        vec![
            ("Content-Type".to_owned(), "text/event-stream".to_owned()),
            ("X-Accel-Buffering".to_owned(), "no".to_owned()),
        ]
    );
    let body = String::from_utf8(response.body().to_vec()).unwrap();
    let events: Vec<&str> = body.split_terminator("\n\n").collect();
    assert_eq!(events.len(), 4, "{body}");
    assert_eq!(events[1], ":");
    let data = |event: &str| -> Value {
        serde_json::from_str(event.strip_prefix("event: message\ndata: ").unwrap()).unwrap()
    };
    assert_eq!(
        data(events[0]),
        acknowledgment(&json!({ "resourceSubscriptions": [WATCHED] }))
    );
    assert_eq!(data(events[2])["method"], "notifications/resources/updated");
    assert_valid("ResourceUpdatedNotification", &data(events[2]));
    assert_valid("SubscriptionsListenResultResponse", &data(events[3]));
}

#[test]
fn over_http_a_service_handed_no_changes_does_not_know_the_method() {
    let asked = listen(&json!({ "resourcesListChanged": true }));
    let response = through(|stream| {
        block_on(document_mcp::answer_http(
            &post(&asked),
            &SETTINGS,
            stream,
            &Archive::default(),
            async |_request: &McpHttpRequest, _call: document_mcp::McpCall| {
                Ok::<_, document_mcp::McpDenied>(())
            },
        ))
    });
    assert_eq!(response.status(), 404);
}

#[test]
fn an_endpoint_of_two_services_honors_resources_of_either() {
    static ENDPOINT: McpEndpoint = McpEndpoint::new(
        &[
            DocumentServiceSchema::MCP_NAMES,
            BillingServiceSchema::MCP_NAMES,
        ],
        &[document_mcp::catalog, billing_mcp_catalog],
    );
    fn billing_mcp_catalog() -> Value {
        billing_mcp::catalog()
    }
    let asked = listen(&json!({ "resourcesListChanged": true }));
    let lines = Lines::default();
    let archive = Archive::default();
    let mut queue = VecDeque::from([McpChange::ResourcesListChanged]);
    let mut source = async || queue.pop_front();
    let answered = block_on(document_mcp::serve_listening(
        &asked.to_string(),
        &document_mcp::McpServer::new(&ENDPOINT, None, "Workspace"),
        |_call| (),
        document_mcp::McpHost::new(&document_mcp::McpInFlight::default(), &lines),
        &|ctx: &(), part, method: &str, params: &Value| {
            let held = archive.clone();
            Box::pin(async move {
                match part {
                    0 => document_mcp::route(method, params, &held, ctx).await,
                    1 => billing_mcp::route(method, params, &held, ctx).await,
                    _ => None,
                }
            })
        },
        &mut source,
    ))
    .unwrap();
    assert_valid(
        "SubscriptionsListenResultResponse",
        &serde_json::from_str(&answered).unwrap(),
    );
    assert_eq!(
        written(&lines),
        vec![
            acknowledgment(&json!({ "resourcesListChanged": true })),
            json!({
                "jsonrpc": "2.0",
                "method": "notifications/resources/list_changed",
                "params": { "_meta": { "io.modelcontextprotocol/subscriptionId": 1_u64 } },
            }),
        ]
    );
}

fn change_event(text: &str) -> Value {
    let data = text
        .strip_prefix("event: message\ndata: ")
        .and_then(|rest| rest.strip_suffix("\n\n"))
        .unwrap();
    serde_json::from_str(data).unwrap()
}

#[test]
fn over_http_the_acknowledgment_and_each_change_are_written_as_they_come() {
    let filter = json!({ "resourcesListChanged": true, "resourceSubscriptions": [WATCHED] });
    let asked = listen(&filter);
    let stream = Recorded::default();
    let feed = Feed::default();
    let mut source = async || feed.next().await;
    let mut tasks = Tasks::default();
    tasks.spawn(async {
        let answered = document_mcp::answer_http_listening(
            &post(&asked),
            &SETTINGS,
            &stream,
            &Archive::default(),
            async |_request: &McpHttpRequest, _call: document_mcp::McpCall| {
                Ok::<_, document_mcp::McpDenied>(())
            },
            &mut source,
        )
        .await;
        assert!(answered.is_none());
    });
    tasks.run_until_idle();
    assert_eq!(
        stream.seen()[0],
        Seen::Began(
            200,
            vec![
                ("Content-Type".to_owned(), "text/event-stream".to_owned()),
                ("X-Accel-Buffering".to_owned(), "no".to_owned()),
            ]
        )
    );
    assert_eq!(stream.seen().len(), 2);
    let acknowledged = change_event(&stream.wrote()[0]);
    assert_eq!(acknowledged, acknowledgment(&filter));
    assert_valid("SubscriptionsAcknowledgedNotification", &acknowledged);
    feed.push(McpChange::ResourcesListChanged);
    tasks.run_until_idle();
    assert_eq!(stream.seen().len(), 3);
    let listed = change_event(&stream.wrote()[1]);
    assert_eq!(listed["method"], "notifications/resources/list_changed");
    assert_valid("ResourceListChangedNotification", &listed);
    feed.push(updated(WATCHED));
    tasks.run_until_idle();
    assert_eq!(stream.seen().len(), 4);
    let changed = change_event(&stream.wrote()[2]);
    assert_eq!(changed["method"], "notifications/resources/updated");
    assert_valid("ResourceUpdatedNotification", &changed);
    assert_eq!(tasks.running(), 1);
    feed.push(McpChange::KeepAlive);
    tasks.run_until_idle();
    assert_eq!(stream.wrote()[3], ":\n\n");
    feed.close();
    tasks.run_until_idle();
    assert_eq!(tasks.running(), 0);
    assert_eq!(stream.seen().len(), 6);
    assert_valid(
        "SubscriptionsListenResultResponse",
        &change_event(&stream.wrote()[4]),
    );
}

#[test]
fn over_http_a_listen_whose_write_fails_is_cancelled_and_ends() {
    let asked = listen(&json!({ "resourcesListChanged": true }));
    let stream = Recorded::gone_after(1);
    let held_call = Mutex::new(None);
    let mut source = async || pending::<Option<McpChange>>().await;
    let answered = block_on(document_mcp::answer_http_listening(
        &post(&asked),
        &SETTINGS,
        &stream,
        &Archive::default(),
        async |_request: &McpHttpRequest, call: document_mcp::McpCall| {
            *held_call.lock().unwrap() = Some(call);
            Ok::<_, document_mcp::McpDenied>(())
        },
        &mut source,
    ));
    assert!(answered.is_none());
    assert_eq!(stream.seen().len(), 1);
    assert!(held_call.lock().unwrap().as_ref().unwrap().cancelled());
}
