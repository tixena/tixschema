//! A service with a streamed reply and a file part, which only `http_rest` carries, beside an
//! operation every transport carries, and a service of a streamed reply alone: the `ws_rpc` client
//! and dispatcher of each language hold no member for either and compile under the language's own
//! toolchain, where the `http_rest` client keeps both.

#![cfg(any(
    all(feature = "typescript", feature = "zod"),
    feature = "dart",
    feature = "swift",
    feature = "kotlin"
))]

use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use std::io::Cursor;

use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

#[cfg(feature = "kotlin")]
use super::runtime::ran_kotlin;

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum GranaryError {
    Empty,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GranaryReceipt {
    pub label: String,
}

#[service_schema(transports = [])]
pub trait GranaryClientService<Ctx> {
    #[service_schema_op(http(method = "GET", path = "/drawn/{bin}", body = "stream"))]
    async fn draw(
        &self,
        ctx: &Ctx,
        bin: String,
    ) -> Result<granary_client_service_schema::StreamedAnswer, GranaryError>;

    #[service_schema_op(http(
        method = "POST",
        path = "/poured",
        body = "multipart",
        part("grain" = grain),
    ))]
    async fn pour(
        &self,
        ctx: &Ctx,
        title: String,
        note: String,
        grain: Box<dyn granary_client_service_schema::BodySource + Send>,
    ) -> Result<GranaryReceipt, GranaryError>;

    #[service_schema_op(http(method = "GET", path = "/sifted/{bin}"))]
    async fn sift(&self, ctx: &Ctx, bin: String) -> Result<u32, GranaryError>;
}

#[service_schema(transports = [])]
pub trait SiloClientService<Ctx> {
    #[service_schema_op(http(method = "GET", path = "/drawn/{bin}", body = "stream"))]
    async fn draw(
        &self,
        ctx: &Ctx,
        bin: String,
    ) -> Result<silo_client_service_schema::StreamedAnswer, GranaryError>;
}

/// An implementation of both services, answering each operation from what it is handed.
pub struct Granary;

impl GranaryClientService<()> for Granary {
    async fn draw(
        &self,
        _ctx: &(),
        bin: String,
    ) -> Result<granary_client_service_schema::StreamedAnswer, GranaryError> {
        ready(()).await;
        Ok(granary_client_service_schema::StreamedAnswer::Full {
            source: Box::new(Cursor::new(bin.into_bytes())),
            content_type: "text/plain".to_owned(),
        })
    }

    async fn pour(
        &self,
        _ctx: &(),
        title: String,
        note: String,
        mut grain: Box<dyn granary_client_service_schema::BodySource + Send>,
    ) -> Result<GranaryReceipt, GranaryError> {
        ready(()).await;
        let mut held = [0_u8; 8];
        let pulled = grain.pull(&mut held).unwrap_or_default();
        Ok(GranaryReceipt {
            label: format!("{title}{note}{pulled}"),
        })
    }

    async fn sift(&self, _ctx: &(), bin: String) -> Result<u32, GranaryError> {
        ready(()).await;
        u32::try_from(bin.len()).or(Err(GranaryError::Empty))
    }
}

impl SiloClientService<()> for Granary {
    async fn draw(
        &self,
        _ctx: &(),
        bin: String,
    ) -> Result<silo_client_service_schema::StreamedAnswer, GranaryError> {
        ready(()).await;
        if bin.is_empty() {
            return Err(GranaryError::Empty);
        }
        Ok(silo_client_service_schema::StreamedAnswer::Full {
            source: Box::new(Cursor::new(bin.into_bytes())),
            content_type: "text/plain".to_owned(),
        })
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

/// Holds `ws` to no member for `draw` and `pour` and to the one for `sift`, and `http` to all
/// three.
fn held_apart(ws: &str, http: &str, sift: &str, draw: &str, pour: &str) {
    assert!(
        ws.contains(sift) && !ws.contains(draw) && !ws.contains(pour),
        "the `ws_rpc` client holds a member for an operation only `http_rest` carries:\n{ws}"
    );
    assert!(
        !ws.contains("StreamedAnswer"),
        "the `ws_rpc` client names `StreamedAnswer`:\n{ws}"
    );
    assert!(
        http.contains(sift) && http.contains(draw) && http.contains(pour),
        "the `http_rest` client lost an operation:\n{http}"
    );
}

/// The services stand on their own: each is implemented, and answers.
#[test]
fn both_services_are_implementable_and_callable() {
    let sifted = poll_once(GranaryClientService::sift(&Granary, &(), "oats".to_owned()));
    assert_eq!(sifted, Some(Ok(4)), "the sift answers another value");
    let poured = poll_once(Granary.pour(
        &(),
        "t".to_owned(),
        "n".to_owned(),
        Box::new(Cursor::new(b"rye".to_vec())),
    ));
    assert_eq!(
        poured,
        Some(Ok(GranaryReceipt {
            label: "tn3".to_owned()
        })),
        "the pour answers another value"
    );
    let drawn = poll_once(GranaryClientService::draw(&Granary, &(), "b".to_owned()));
    assert!(
        matches!(drawn, Some(Ok(_))),
        "the granary's draw answers no stream"
    );
    let empty = poll_once(SiloClientService::draw(&Granary, &(), String::new()));
    assert!(
        matches!(empty, Some(Err(GranaryError::Empty))),
        "the silo's draw answers no refusal"
    );
}

/// The transport-agnostic and `ws_rpc` TypeScript clients hold no method and no schema for
/// `draw` and `pour`, and the `http_rest` client keeps both.
#[cfg(all(feature = "typescript", feature = "zod"))]
#[test]
fn the_typescript_clients_hold_no_operation_only_http_rest_carries() {
    held_apart(
        &GranaryClientServiceSchema::ts_client(),
        &GranaryClientServiceSchema::ts_http_client(),
        "sift(req",
        "draw(",
        "pour(",
    );
    let tables = GranaryClientServiceSchema::ts_ws_client();
    assert!(
        tables.contains("  \"sift\": ")
            && !tables.contains("\"draw\"")
            && !tables.contains("\"pour\"")
            && !tables.contains("StreamedAnswer"),
        "the `ws_rpc` reply tables hold an operation only `http_rest` carries:\n{tables}"
    );
    for written in [
        SiloClientServiceSchema::ts_client(),
        SiloClientServiceSchema::ts_ws_client(),
    ] {
        assert!(
            !written.contains("draw(")
                && !written.contains("\"draw\"")
                && !written.contains("StreamedAnswer"),
            "a client of a service of one streamed operation names it:\n{written}"
        );
    }
}

/// The Dart `ws_rpc` client and attachment hold no member for `draw` and `pour`, and pass
/// `dart analyze` with no issue beside the `http_rest` client, which keeps both.
#[cfg(feature = "dart")]
#[test]
fn the_dart_ws_client_holds_no_operation_only_http_rest_carries_and_compiles() {
    use super::run_dart_validate::dart;

    held_apart(
        &GranaryClientServiceSchema::dart_ws_client(),
        &GranaryClientServiceSchema::dart_http_client(),
        "sift(String req)",
        "draw(",
        "pour(",
    );
    let module = [
        "import 'dart:async';".to_owned(),
        "import 'dart:convert';".to_owned(),
        granary_error_dart::dart_definition(),
        granary_receipt_dart::dart_definition(),
        GranaryClientServiceSchema::dart_definition(),
        GranaryClientServiceSchema::dart_http_client(),
        GranaryClientServiceSchema::dart_ws_client(),
        SiloClientServiceSchema::dart_definition(),
        SiloClientServiceSchema::dart_http_client(),
        SiloClientServiceSchema::dart_ws_client(),
        "void main() {}".to_owned(),
    ]
    .join("\n\n");
    let Some((clean, said)) = dart("http-alone-analyze", &["analyze"], &module) else {
        return;
    };
    assert!(clean && said.contains("No issues found!"), "{said}");
}

/// The Swift `ws_rpc` actor holds no method for `draw` and `pour`, and compiles with warnings as
/// errors beside the `http_rest` client, which keeps both.
#[cfg(feature = "swift")]
#[test]
fn the_swift_ws_client_holds_no_operation_only_http_rest_carries_and_compiles() {
    use super::runtime::judged;
    use granary_client_service_schema::{
        granary_client_service_fault_fields_swift, granary_client_service_fault_kind_swift,
    };
    use silo_client_service_schema::{
        silo_client_service_fault_fields_swift, silo_client_service_fault_kind_swift,
    };

    held_apart(
        &GranaryClientServiceSchema::swift_ws_client(),
        &GranaryClientServiceSchema::swift_http_client(),
        "func sift(",
        "func draw(",
        "func pour(",
    );
    let module = [
        "import Foundation".to_owned(),
        granary_error_swift::swift_definition(),
        granary_receipt_swift::swift_definition(),
        pour_request_swift::swift_definition(),
        granary_client_service_fault_fields_swift::swift_definition(),
        granary_client_service_fault_kind_swift::swift_definition(),
        GranaryClientServiceSchema::swift_http_client(),
        GranaryClientServiceSchema::swift_ws_client(),
        silo_client_service_fault_fields_swift::swift_definition(),
        silo_client_service_fault_kind_swift::swift_definition(),
        SiloClientServiceSchema::swift_http_client(),
        SiloClientServiceSchema::swift_ws_client(),
        "print(\"compiled\")".to_owned(),
    ]
    .join("\n\n");
    let Some(run) = judged(
        "swift-http-alone",
        "TIXSCHEMA_SWIFT",
        "swift",
        &["-warnings-as-errors"],
        "main.swift",
        &module,
    ) else {
        return;
    };
    assert!(
        run.succeeded && run.stdout == "compiled\n",
        "`swift -warnings-as-errors main.swift` failed.\n--- stdout ---\n{}\n--- stderr ---\n{}",
        run.stdout,
        run.stderr
    );
}

/// The Kotlin `ws_rpc` client and attachment hold no member for `draw` and `pour`, and compile
/// with `-Werror` beside the `http_rest` client, which keeps both.
#[cfg(feature = "kotlin")]
#[test]
fn the_kotlin_ws_client_holds_no_operation_only_http_rest_carries_and_compiles() {
    use granary_client_service_schema::{
        granary_client_service_fault_fields_kotlin, granary_client_service_fault_kind_kotlin,
    };
    use silo_client_service_schema::{
        silo_client_service_fault_fields_kotlin, silo_client_service_fault_kind_kotlin,
    };

    held_apart(
        &GranaryClientServiceSchema::kotlin_ws_client(),
        &GranaryClientServiceSchema::kotlin_http_client(),
        "suspend fun sift(",
        "suspend fun draw(",
        "suspend fun pour(",
    );
    let module = [
        "import kotlinx.coroutines.*\n\
         import kotlinx.coroutines.flow.*\n\
         import kotlinx.serialization.*\n\
         import kotlinx.serialization.json.*\n\
         import kotlinx.serialization.descriptors.*\n\
         import kotlinx.serialization.encoding.*\n\
         import kotlinx.serialization.builtins.*"
            .to_owned(),
        granary_error_kotlin::kotlin_definition(),
        granary_receipt_kotlin::kotlin_definition(),
        pour_request_kotlin::kotlin_definition(),
        granary_client_service_fault_fields_kotlin::kotlin_definition(),
        granary_client_service_fault_kind_kotlin::kotlin_definition(),
        GranaryClientServiceSchema::kotlin_http_client(),
        GranaryClientServiceSchema::kotlin_ws_client(),
        silo_client_service_fault_fields_kotlin::kotlin_definition(),
        silo_client_service_fault_kind_kotlin::kotlin_definition(),
        SiloClientServiceSchema::kotlin_http_client(),
        SiloClientServiceSchema::kotlin_ws_client(),
        "fun main() = println(\"compiled\")".to_owned(),
    ]
    .join("\n\n");
    let Some(printed) = ran_kotlin(&module) else {
        return;
    };
    assert_eq!(
        printed, "compiled\n",
        "the Kotlin module printed another line"
    );
}
