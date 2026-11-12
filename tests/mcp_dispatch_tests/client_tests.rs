//! The generated client, driven over a seam that hands each message to the dispatcher, and over
//! seams that answer canned text.

use crate::document_client::{DocumentServiceClient, McpMessage, Transport};
use crate::document_mcp;
use crate::document_service_schema::{CallError, ServiceFault, ServiceFaultKind};
use crate::support::{Lines, assert_valid, block_on};
use crate::tests::{
    Archive, DocumentId, DraftBatchDraft, DraftDraft, GetVersionRequest, GetVersionRequestDraft,
    VersionError, VersionErrorDraft, VersionResponse, VersionResponseDraft,
};
use core::fmt::Debug;
use core::future::Future;
use core::mem::take;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::panic::resume_unwind;
use std::sync::Mutex;

/// What a seam was handed.
#[derive(Debug, Eq, PartialEq)]
struct Sent {
    method: &'static str,
    name: Option<String>,
    protocol_version: &'static str,
    text: String,
}

impl Sent {
    fn arguments(&self) -> Value {
        serde_json::from_str::<Value>(&self.text).unwrap()["params"]["arguments"].clone()
    }
}

/// Hands each message to the dispatcher of the same service, and writes down what it was handed.
struct Dispatching<'archive> {
    archive: &'archive Archive,
    sent: Mutex<Vec<Sent>>,
}

impl<'archive> Dispatching<'archive> {
    fn over(archive: &'archive Archive) -> Self {
        Self {
            archive,
            sent: Mutex::new(Vec::new()),
        }
    }
}

/// Answers the same text to every message, or fails to send it.
struct Canned {
    answer: Result<String, String>,
    sent: Mutex<Vec<Sent>>,
}

impl Canned {
    fn answering(text: &str) -> Self {
        Self {
            answer: Ok(text.to_owned()),
            sent: Mutex::new(Vec::new()),
        }
    }

    fn failing(reason: &str) -> Self {
        Self {
            answer: Err(reason.to_owned()),
            sent: Mutex::new(Vec::new()),
        }
    }
}

impl Transport for Canned {
    fn send(&self, message: McpMessage) -> impl Future<Output = Result<String, String>> + Send {
        written_down(&self.sent, &message);
        let answer = self.answer.clone();
        async move { answer }
    }
}

impl Transport for Dispatching<'_> {
    fn send(&self, message: McpMessage) -> impl Future<Output = Result<String, String>> + Send {
        let text = written_down(&self.sent, &message);
        let archive = self.archive;
        async move {
            let answered = document_mcp::answer(
                &text,
                archive,
                |_call| (),
                &Lines::default(),
                &document_mcp::McpInFlight::default(),
            )
            .await;
            Ok(answered.unwrap())
        }
    }
}

fn written_down(sent: &Mutex<Vec<Sent>>, message: &McpMessage) -> String {
    let (method, name, protocol_version, text) = (
        message.method(),
        message.name().map(str::to_owned),
        message.protocol_version(),
        message.text().to_owned(),
    );
    assert_valid("CallToolRequest", &serde_json::from_str(&text).unwrap());
    sent.lock().unwrap().push(Sent {
        method,
        name,
        protocol_version,
        text: text.clone(),
    });
    text
}

fn taken(sent: &Mutex<Vec<Sent>>) -> Vec<Sent> {
    take(&mut *sent.lock().unwrap())
}

fn result_text(result: &Value) -> String {
    json!({ "jsonrpc": "2.0", "id": 1_u64, "result": result }).to_string()
}

fn fault_of<Answered>(answered: Result<Answered, CallError<VersionError>>) -> ServiceFault
where
    Answered: Debug,
{
    match answered.unwrap_err() {
        CallError::Fault(fault) => fault,
        CallError::Operation(declared) => resume_unwind(Box::new(declared)),
    }
}

fn ask(
    client: &DocumentServiceClient<impl Transport + Sync>,
    document_id: &str,
) -> Result<VersionResponse, CallError<VersionError>> {
    block_on(
        client.get_version(GetVersionRequest::model_schema_wrap_unjudged(
            GetVersionRequestDraft {
                document_id: document_id.to_owned(),
            },
        )),
    )
}

#[test]
fn a_success_is_read_from_the_structured_content() {
    let archive = Archive::default();
    let client = DocumentServiceClient::new(Dispatching::over(&archive));
    assert_eq!(
        ask(&client, "doc-1"),
        Ok(VersionResponseDraft { version: 7 }.validate().unwrap())
    );
}

#[test]
fn a_declared_error_is_the_error_the_operation_declared() {
    let archive = Archive::default();
    let client = DocumentServiceClient::new(Dispatching::over(&archive));
    assert_eq!(
        ask(&client, "missing"),
        Err(CallError::Operation(
            VersionErrorDraft::NotFound.validate().unwrap()
        ))
    );
}

#[test]
fn a_message_the_validator_refuses_is_refused_before_the_seam_is_reached() {
    let archive = Archive::default();
    let client = DocumentServiceClient::new(Dispatching::over(&archive));
    let fault = fault_of(ask(&client, ""));
    assert_eq!(fault.kind(), ServiceFaultKind::FailedValidation);
    assert_eq!(fault.field(), Some("document_id"));
    assert_eq!(taken(&client.transport().sent), Vec::<Sent>::new());
}

#[test]
fn a_list_a_map_and_an_optional_message_are_refused_for_the_item_that_breaks_a_bound() {
    let archive = Archive::default();
    let client = DocumentServiceClient::new(Dispatching::over(&archive));
    let broken = || {
        GetVersionRequest::model_schema_wrap_unjudged(GetVersionRequestDraft {
            document_id: String::new(),
        })
    };
    let refusals = [
        fault_of(block_on(client.check_many(vec![broken()]))),
        fault_of(block_on(
            client.check_named(HashMap::from([("first".to_owned(), broken())])),
        )),
        fault_of(block_on(client.check_maybe(Some(broken())))),
    ];
    for fault in refusals {
        assert_eq!(fault.kind(), ServiceFaultKind::FailedValidation);
        assert_eq!(fault.field(), Some("document_id"));
    }
    assert_eq!(taken(&client.transport().sent), Vec::<Sent>::new());
    let read = block_on(client.check_many(vec![GetVersionRequestDraft {
        document_id: "doc-1".to_owned(),
    }.validate().unwrap()]));
    assert_eq!(
        read,
        Ok(VersionResponseDraft { version: 7 }.validate().unwrap())
    );
}

#[test]
fn a_pair_is_refused_for_the_member_whose_request_breaks_a_bound() {
    let archive = Archive::default();
    let client = DocumentServiceClient::new(Dispatching::over(&archive));
    let whole = || {
        GetVersionRequestDraft {
            document_id: "doc-1".to_owned(),
        }
        .validate()
        .unwrap()
    };
    let broken = || {
        GetVersionRequest::model_schema_wrap_unjudged(GetVersionRequestDraft {
            document_id: String::new(),
        })
    };
    let fault = fault_of(block_on(client.check_pair(whole(), broken())));
    assert_eq!(fault.kind(), ServiceFaultKind::FailedValidation);
    assert_eq!(fault.field(), Some("second.document_id"));
    assert_eq!(taken(&client.transport().sent), Vec::<Sent>::new());
    assert_eq!(
        block_on(client.check_pair(whole(), whole())),
        Ok(VersionResponseDraft { version: 7 }.validate().unwrap())
    );
}

#[test]
fn a_list_of_drafts_is_read_by_the_server_and_counted() {
    let archive = Archive::default();
    let client = DocumentServiceClient::new(Dispatching::over(&archive));
    let counted = block_on(
        client.check_drafts(
            DraftBatchDraft {
                drafts: vec![
                    DraftDraft {
                        references: Vec::new(),
                        title: "Plan".to_owned(),
                    }
                    .validate()
                    .unwrap(),
                ],
            }
            .validate()
            .unwrap(),
        ),
    );
    assert_eq!(
        counted,
        Ok(VersionResponseDraft { version: 1 }.validate().unwrap())
    );
}

#[test]
fn an_answer_the_read_refuses_names_the_path_of_the_value() {
    let result = json!({ "resultType": "complete", "content": [], "structuredContent": { "version": "seven" }, "isError": false });
    let client = DocumentServiceClient::new(Canned::answering(&result_text(&result)));
    assert_eq!(
        fault_of(ask(&client, "doc-1")).detail(),
        "'version': invalid type: string \"seven\", expected u32"
    );
}

#[test]
fn a_fault_of_the_server_reaches_the_caller_as_that_fault() {
    let archive = Archive::default();
    let client = DocumentServiceClient::new(Dispatching::over(&archive));
    let panicked = fault_of(ask(&client, "panic"));
    assert_eq!(panicked.kind(), ServiceFaultKind::HandlerPanic);
    assert_eq!(panicked.detail(), "the handler came apart");
    assert_eq!(panicked.operation(), "get-version");
}

#[test]
fn a_unit_success_a_wrapped_message_and_a_one_way_call_are_answered() {
    let archive = Archive::default();
    let client = DocumentServiceClient::new(Dispatching::over(&archive));
    assert_eq!(
        block_on(client.rename_document("doc-1".to_owned(), "Plan".to_owned())),
        Ok(())
    );
    assert_eq!(
        block_on(client.lookup(DocumentId::new("doc-1".to_owned()).unwrap())),
        Ok(VersionResponseDraft { version: 7 }.validate().unwrap())
    );
    assert_eq!(
        block_on(client.touch_document("doc-1".to_owned(), "reviewer-7".to_owned())),
        Ok(())
    );
    assert_eq!(
        block_on(client.touch_document("doc-1".to_owned(), "panic".to_owned())),
        Ok(())
    );
    let arguments: Vec<Value> = taken(&client.transport().sent)
        .iter()
        .map(Sent::arguments)
        .collect();
    assert_eq!(
        arguments,
        [
            json!({ "documentId": "doc-1", "title": "Plan" }),
            json!({ "value": "doc-1" }),
            json!({ "documentId": "doc-1", "reader": "reviewer-7" }),
            json!({ "documentId": "doc-1", "reader": "panic" }),
        ]
    );
}

#[test]
fn a_header_goes_beside_the_wrapped_message_and_comes_back_beside_the_value() {
    let archive = Archive::default();
    let client = DocumentServiceClient::new(Dispatching::over(&archive));
    assert_eq!(
        block_on(client.get_tagged("doc-1".to_owned(), Some("\"v6\"".to_owned()))),
        Ok((
            VersionResponseDraft { version: 7 }.validate().unwrap(),
            "\"v7\"".to_owned()
        ))
    );
    assert_eq!(
        block_on(client.get_tagged("doc-1".to_owned(), None)),
        Ok((
            VersionResponseDraft { version: 7 }.validate().unwrap(),
            "\"v7\"".to_owned()
        ))
    );
    let arguments: Vec<Value> = taken(&client.transport().sent)
        .iter()
        .map(Sent::arguments)
        .collect();
    assert_eq!(
        arguments,
        [
            json!({ "value": "doc-1", "if-none-match": "\"v6\"" }),
            json!({ "value": "doc-1" }),
        ]
    );
    assert_eq!(
        *archive.reached.lock().unwrap(),
        [
            "get-tagged doc-1 Some(\"\\\"v6\\\"\")",
            "get-tagged doc-1 None"
        ]
    );
}

#[test]
fn each_message_names_the_version_the_method_and_the_tool_and_numbers_its_request() {
    let archive = Archive::default();
    let client = DocumentServiceClient::new(Dispatching::over(&archive));
    ask(&client, "doc-1").unwrap();
    ask(&client, "doc-1").unwrap();
    let sent = taken(&client.transport().sent);
    let texts: Vec<Value> = sent
        .iter()
        .map(|message| serde_json::from_str(&message.text).unwrap())
        .collect();
    assert_eq!(
        sent.iter()
            .map(|message| (
                message.method,
                message.name.as_deref(),
                message.protocol_version
            ))
            .collect::<Vec<_>>(),
        [("tools/call", Some("get-version"), "2026-07-28"); 2]
    );
    assert_eq!(
        [&texts[0]["id"], &texts[1]["id"]],
        [&json!(1_u64), &json!(2_u64)]
    );
    assert_eq!(
        texts[0]["params"]["_meta"],
        json!({
            "io.modelcontextprotocol/protocolVersion": "2026-07-28",
            "io.modelcontextprotocol/clientCapabilities": {},
            "io.modelcontextprotocol/clientInfo": {
                "name": env!("CARGO_PKG_NAME"),
                "version": env!("CARGO_PKG_VERSION"),
            },
        })
    );
}

#[test]
fn a_seam_that_fails_is_a_transport_failure() {
    let client = DocumentServiceClient::new(Canned::failing("connection refused"));
    let fault = fault_of(ask(&client, "doc-1"));
    assert_eq!(fault.kind(), ServiceFaultKind::TransportFailure);
    assert_eq!(fault.detail(), "connection refused");
    let one_way = block_on(client.touch_document("doc-1".to_owned(), "reviewer-7".to_owned()));
    assert_eq!(
        one_way.unwrap_err().kind(),
        ServiceFaultKind::TransportFailure
    );
}

#[test]
fn text_that_is_no_json_is_an_undeserializable_payload() {
    let client = DocumentServiceClient::new(Canned::answering("not json"));
    assert_eq!(
        fault_of(ask(&client, "doc-1")).kind(),
        ServiceFaultKind::UndeserializablePayload
    );
}

#[test]
fn a_value_the_read_refuses_is_an_undeserializable_payload() {
    for result in [
        json!({ "resultType": "complete", "content": [], "isError": false }),
        json!({ "resultType": "complete", "content": [], "structuredContent": { "version": "seven" }, "isError": false }),
        json!({ "resultType": "complete", "content": [], "isError": true }),
        json!({ "resultType": "complete", "content": [{ "type": "text", "text": "not json" }], "isError": true }),
    ] {
        let client = DocumentServiceClient::new(Canned::answering(&result_text(&result)));
        assert_eq!(
            fault_of(ask(&client, "doc-1")).kind(),
            ServiceFaultKind::UndeserializablePayload,
            "{result}"
        );
    }
    let client = DocumentServiceClient::new(Canned::answering("{\"jsonrpc\":\"2.0\",\"id\":1}"));
    assert_eq!(
        fault_of(ask(&client, "doc-1")).kind(),
        ServiceFaultKind::UndeserializablePayload
    );
}

#[test]
fn a_fault_in_a_result_is_that_fault() {
    let text = "{\"fault\":{\"detail\":\"'document_id': too short\",\"field\":\"document_id\",\"kind\":\"failed-validation\",\"operation\":\"get-version\"},\"isServiceFault\":true}";
    let result = json!({ "resultType": "complete", "content": [{ "type": "text", "text": text }], "isError": true });
    let client = DocumentServiceClient::new(Canned::answering(&result_text(&result)));
    let fault = fault_of(ask(&client, "doc-1"));
    assert_eq!(fault.kind(), ServiceFaultKind::FailedValidation);
    assert_eq!(fault.field(), Some("document_id"));
    assert_eq!(fault.detail(), "'document_id': too short");
    assert_eq!(fault.issues(), []);
}

#[test]
fn a_fault_that_lists_its_problems_keeps_each_one_at_its_field() {
    let text = "{\"fault\":{\"detail\":\"'drafts.0.title': missing field `title`; 'drafts.1.title': missing field `title`\",\"field\":\"drafts.0.title\",\"issues\":[{\"detail\":\"missing field `title`\",\"field\":\"drafts.0.title\"},{\"detail\":\"missing field `title`\",\"field\":\"drafts.1.title\"}],\"kind\":\"failed-validation\",\"operation\":\"check-drafts\"},\"isServiceFault\":true}";
    let result = json!({ "resultType": "complete", "content": [{ "type": "text", "text": text }], "isError": true });
    let client = DocumentServiceClient::new(Canned::answering(&result_text(&result)));
    let fault = fault_of(ask(&client, "doc-1"));
    assert_eq!(fault.field(), Some("drafts.0.title"));
    let issues: Vec<(Option<&str>, &str)> = fault
        .issues()
        .iter()
        .map(|issue| (issue.field(), issue.detail()))
        .collect();
    assert_eq!(
        issues,
        [
            (Some("drafts.0.title"), "missing field `title`"),
            (Some("drafts.1.title"), "missing field `title`"),
        ]
    );
}

#[test]
fn a_json_rpc_error_with_a_fault_envelope_is_that_fault() {
    let answer = json!({
        "jsonrpc": "2.0",
        "id": 1_u64,
        "error": {
            "code": -32603_i64,
            "message": "get-version: the handler came apart",
            "data": { "isServiceFault": true, "fault": {
                "detail": "the handler came apart",
                "kind": "handler-panic",
                "operation": "get-version",
            } },
        },
    });
    let client = DocumentServiceClient::new(Canned::answering(&answer.to_string()));
    assert_eq!(
        fault_of(ask(&client, "doc-1")).kind(),
        ServiceFaultKind::HandlerPanic
    );
}

#[test]
fn any_other_json_rpc_error_is_a_transport_failure_naming_it() {
    let answer = json!({ "jsonrpc": "2.0", "id": 1_u64, "error": { "code": -32022_i64, "message": "Unsupported protocol version" } });
    let client = DocumentServiceClient::new(Canned::answering(&answer.to_string()));
    let fault = fault_of(ask(&client, "doc-1"));
    assert_eq!(fault.kind(), ServiceFaultKind::TransportFailure);
    assert_eq!(fault.detail(), "-32022: Unsupported protocol version");
}
