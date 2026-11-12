//! `ShelfService`, whose template and completion take messages a request can fail to read or the
//! validator can refuse, and the stand-in that answers it: the Rust twin the TypeScript and Dart
//! servers are held to the fault table by.

use super::mcp_library_rows::{assert_answer_valid, request};
use super::mcp_library_service::{BookError, Sink};
use super::mcp_shelf_dispatcher;
use super::mcp_writer_rows::ready_output;
use super::resource_cases::RESOURCE_FAULT_CASES;
use super::tests::poll_once;
use core::future::{Future, ready};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tixschema::{model_schema, service_schema};

#[model_schema(enable = ["jsonschema"], maxLength = 4)]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ShelfLabel(String);

/// Keeps goods in the slots of aisles.
#[service_schema(transports = ["mcp"])]
pub trait ShelfService<Ctx> {
    /// One level of one bay, two numbers no captured text reads as.
    #[service_schema_op(mcp(resource = "shelf://bays/{bay}/levels/{level}"))]
    async fn get_bay(&self, ctx: &Ctx, bay: u32, level: u32) -> Result<String, BookError>;

    /// The count of one shelf, a number no captured text reads as.
    #[service_schema_op(mcp(resource = "shelf://counts/{bin}"))]
    async fn get_count(&self, ctx: &Ctx, bin: u32) -> Result<String, BookError>;

    /// One slot of one aisle.
    #[service_schema_op(mcp(resource = "shelf://aisles/{aisle}/slots/{slot}"))]
    async fn get_slot(
        &self,
        ctx: &Ctx,
        aisle: ShelfLabel,
        slot: String,
    ) -> Result<String, BookError>;

    /// Suggests the slots whose number starts with what the user typed.
    #[service_schema_op(mcp(completes(resource = "get_slot", parameter = "slot")))]
    async fn suggest_slots(
        &self,
        ctx: &Ctx,
        typed: ShelfLabel,
        chosen: HashMap<String, String>,
    ) -> Result<Vec<String>, BookError>;
}

/// Answers every operation of `ShelfService`.
#[derive(Default)]
pub struct Shelves;

impl ShelfService<()> for Shelves {
    fn get_bay(
        &self,
        _ctx: &(),
        bay: u32,
        level: u32,
    ) -> impl Future<Output = Result<String, BookError>> {
        ready(Ok(format!("{bay}/{level}")))
    }

    fn get_count(&self, _ctx: &(), bin: u32) -> impl Future<Output = Result<String, BookError>> {
        ready(Ok(bin.to_string()))
    }

    fn get_slot(
        &self,
        _ctx: &(),
        aisle: ShelfLabel,
        slot: String,
    ) -> impl Future<Output = Result<String, BookError>> {
        ready(Ok(format!("{}/{slot}", aisle.as_inner())))
    }

    fn suggest_slots(
        &self,
        _ctx: &(),
        typed: ShelfLabel,
        _chosen: HashMap<String, String>,
    ) -> impl Future<Output = Result<Vec<String>, BookError>> {
        ready(Ok(vec![typed.into_inner()]))
    }
}

impl super::mcp_shelf_dispatcher::McpEventWriter for Sink {
    fn write(
        &self,
        _message: String,
    ) -> impl Future<Output = Result<(), super::mcp_shelf_dispatcher::McpClientGone>> {
        ready(Ok(()))
    }
}

/// The requests of the fault table, one text for each case, numbered from 100.
pub fn fault_requests() -> Vec<String> {
    (100_u64..)
        .zip(RESOURCE_FAULT_CASES.iter())
        .map(|(id, case)| request(&Value::from(id), case.method, &case.params()).to_string())
        .collect()
}

/// What the Rust dispatcher of `ShelfService` answers to `text`.
pub fn shelf_answer(text: &str) -> Option<String> {
    poll_once(super::mcp_shelf_dispatcher::answer(
        text,
        &Shelves,
        |call| call.progress(0.0_f64, None, None),
        &Sink,
        &super::mcp_shelf_dispatcher::McpInFlight::default(),
    ))
    .flatten()
}

/// Holds each answer of a server, `answers` in the order of [`fault_requests`], to the table.
pub fn assert_faults(which: &str, answers: &[Option<String>]) {
    assert_eq!(answers.len(), RESOURCE_FAULT_CASES.len(), "{which}");
    for (case, answered) in RESOURCE_FAULT_CASES.iter().zip(answers) {
        let text = answered.as_deref().unwrap();
        assert_answer_valid(case.method, text);
        case.holds(&serde_json::from_str(text).unwrap());
    }
}

#[test]
fn the_rust_dispatcher_answers_every_fault_of_the_table() {
    let answers: Vec<Option<String>> = fault_requests()
        .iter()
        .map(|text| shelf_answer(text))
        .collect();
    assert_faults("rust", &answers);
}

#[test]
fn the_rust_dispatcher_listens_alike_through_every_entry_point() {
    let (alone, served) = rust_listens_alike!(
        mcp_shelf_dispatcher,
        ShelfServiceSchema,
        Shelves,
        ready_output
    );
    assert_eq!(alone, served);
    assert!(alone.is_some());
}
