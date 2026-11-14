//! The calling crate: it places the client the declaring crate publishes, and calls it over seams
//! that answer canned text.

use contract::document_service_schema::{CallError, ServiceFault, ServiceFaultKind};
use contract::{GetVersionRequest, GetVersionRequestDraft, VersionError, VersionErrorDraft, VersionResponse};
use std::future::Future;
use std::pin::pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Waker};

mod client {
    use contract::{GetVersionRequest, VersionError, VersionResponse};

    contract::document_service_mcp_client!();
}

use client::{DocumentServiceClient, McpMessage, Transport};

/// Answers one text to every message, or fails to send it, and counts the messages it was handed.
struct Canned {
    answer: Result<String, String>,
    sent: AtomicUsize,
}

impl Transport for Canned {
    fn send(&self, message: McpMessage) -> impl Future<Output = Result<String, String>> + Send {
        assert_eq!(message.method(), "tools/call");
        assert_eq!(message.name(), Some("get-version"));
        self.sent.fetch_add(1, Ordering::Relaxed);
        let answer = self.answer.clone();
        async move { answer }
    }
}

fn block_on<Answering: Future>(answering: Answering) -> Answering::Output {
    let mut pinned = pin!(answering);
    match pinned.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(output) => output,
        Poll::Pending => unreachable!("a canned seam answers at once"),
    }
}

fn kebab(fault: &ServiceFault) -> &'static str {
    match fault.kind() {
        ServiceFaultKind::FailedValidation => "failed-validation",
        ServiceFaultKind::HandlerPanic => "handler-panic",
        ServiceFaultKind::TransportFailure => "transport-failure",
        ServiceFaultKind::UndeserializablePayload => "undeserializable-payload",
        ServiceFaultKind::UnknownOperation => "unknown-operation",
    }
}

fn outcome(answered: Result<VersionResponse, CallError<VersionError>>) -> String {
    match answered {
        Ok(version) => format!("ok version={}", version.version),
        Err(CallError::Operation(error)) => match &*error {
            VersionErrorDraft::NotFound => "declared not-found".to_owned(),
        },
        Err(CallError::Fault(fault)) => format!("fault {}", kebab(&fault)),
    }
}

fn call(answer: Result<String, String>, document_id: &str) -> (String, usize) {
    let client = DocumentServiceClient::new(Canned {
        answer,
        sent: AtomicUsize::new(0),
    });
    let answered = block_on(client.get_version(GetVersionRequest::model_schema_wrap_unjudged(
        GetVersionRequestDraft {
            document_id: document_id.to_owned(),
        },
    )));
    (
        outcome(answered),
        client.transport().sent.load(Ordering::Relaxed),
    )
}

fn result(content: &str, is_error: bool, structured: Option<&str>) -> String {
    let structured = structured.map_or_else(String::new, |value| {
        format!(",\"structuredContent\":{value}")
    });
    let text = serde_json::to_string(content).unwrap();
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{{\"resultType\":\"complete\",\"content\":[{{\"type\":\"text\",\"text\":{text}}}],\"isError\":{is_error}{structured}}}}}"
    )
}

fn main() {
    let fault = "{\"fault\":{\"issues\":[{\"detail\":\"too short\",\"field\":\"document_id\"}],\"kind\":\"failed-validation\",\"operation\":\"get-version\"},\"isServiceFault\":true}";
    let panicked = "{\"jsonrpc\":\"2.0\",\"id\":1,\"error\":{\"code\":-32603,\"message\":\"get-version: the handler came apart\",\"data\":{\"isServiceFault\":true,\"fault\":{\"detail\":\"the handler came apart\",\"kind\":\"handler-panic\",\"operation\":\"get-version\"}}}}";
    let unsupported = "{\"jsonrpc\":\"2.0\",\"id\":1,\"error\":{\"code\":-32022,\"message\":\"Unsupported protocol version\",\"data\":{\"supported\":[\"2026-07-28\"],\"requested\":\"2025-11-25\"}}}";
    let answers = [
        Ok(result("{\"version\":7}", false, Some("{\"version\":7}"))),
        Ok(result("{\"errorCode\":\"not-found\"}", true, None)),
        Ok(result(fault, true, None)),
        Ok(panicked.to_owned()),
        Ok(unsupported.to_owned()),
        Ok("not json".to_owned()),
        Err("connection refused".to_owned()),
    ];
    for answer in answers {
        println!("{}", call(answer, "doc-1").0);
    }
    let (refused, sent) = call(Err("never reached".to_owned()), "");
    println!("refused {} sent={sent}", refused.trim_start_matches("fault "));
}
