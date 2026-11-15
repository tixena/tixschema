//! `Shipping`: one operation whose message holds a bounded brand in each shape a member holds a
//! model in.

use crate::shipping_ws;
use alloc::collections::BTreeMap;
use core::future::{Future as _, ready};
use core::pin::pin;
use core::task::{Context, Poll, Waker};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::{model_schema, service_schema};

/// A label of one to eighty characters.
#[model_schema(minLength = 1, maxLength = 80)]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(transparent)]
pub struct LabelText(String);

/// One shipment, holding labels in each shape a member holds a model in.
#[model_schema()]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Shipment {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub alias: Option<LabelText>,
    #[serde(default)]
    pub grid: Vec<Vec<LabelText>>,
    pub label: LabelText,
    #[serde(default)]
    pub named: BTreeMap<String, LabelText>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub pair: Option<(LabelText, u32)>,
    #[model_schema_prop(minimum = 1, maximum = 100)]
    pub parcel_count: u32,
    pub tags: Vec<LabelText>,
}

/// A shipment and the shipments noted beside it.
#[model_schema()]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Manifest {
    pub notes: Vec<Shipment>,
    pub shipment: Shipment,
}

/// The one error `ship` declares.
#[model_schema()]
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "errorCode", rename_all = "kebab-case")]
pub enum ShipError {
    Refused,
}

#[service_schema(transports = ["ws_rpc"])]
pub trait Shipping<Ctx> {
    async fn ship(&self, ctx: &Ctx, req: Manifest) -> Result<u32, ShipError>;
}

struct Dock;

impl Shipping<()> for Dock {
    async fn ship(&self, _ctx: &(), _req: Manifest) -> Result<u32, ShipError> {
        ready(Ok(1_u32)).await
    }
}

fn fault_for(payload: &Value) -> Value {
    let frame = json!({
        "kind": "request",
        "id": "1",
        "service": "Shipping",
        "operation": "ship",
        "payload": payload,
    })
    .to_string();
    let mut answering = pin!(shipping_ws::answer(&frame, &Dock, &()));
    let Poll::Ready(Some(reply)) = answering
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    else {
        return Value::Null;
    };
    let answered: Value = serde_json::from_str(&reply).unwrap();
    answered["error"]["fault"].clone()
}

fn shipment(extra: &Value) -> Value {
    let mut base = json!({ "label": "ok", "parcelCount": 3_u32, "tags": [] });
    for (key, value) in extra.as_object().unwrap() {
        base[key] = value.clone();
    }
    base
}

#[test]
fn each_broken_label_is_named_at_its_path() {
    let cases = [
        (
            json!({ "shipment": shipment(&json!({ "parcelCount": 200_u32 })), "notes": [] }),
            "shipment.parcelCount",
        ),
        (
            json!({ "shipment": shipment(&json!({ "label": "" })), "notes": [] }),
            "shipment.label",
        ),
        (
            json!({ "shipment": shipment(&json!({ "tags": ["ok", ""] })), "notes": [] }),
            "shipment.tags.1",
        ),
        (
            json!({ "shipment": shipment(&json!({ "alias": "" })), "notes": [] }),
            "shipment.alias",
        ),
        (
            json!({ "shipment": shipment(&json!({ "named": { "a": "" } })), "notes": [] }),
            "shipment.named.a",
        ),
        (
            json!({ "shipment": shipment(&json!({ "grid": [["ok", ""]] })), "notes": [] }),
            "shipment.grid.0.1",
        ),
        (
            json!({ "shipment": shipment(&json!({ "pair": ["", 1_u32] })), "notes": [] }),
            "shipment.pair.0",
        ),
        (
            json!({ "shipment": shipment(&json!({})), "notes": [shipment(&json!({ "label": "" }))] }),
            "notes.0.label",
        ),
    ];
    for (payload, field) in cases {
        let fault = fault_for(&payload);
        assert_eq!(fault["kind"], "failed-validation", "{payload}");
        assert_eq!(fault["issues"][0]["field"], field, "{payload}");
        assert!(
            !fault["issues"][0]["detail"].as_str().unwrap().is_empty(),
            "{fault}"
        );
    }
}

#[test]
fn every_break_of_a_checked_holder_reaches_the_issues() {
    let payload = json!({ "shipment": shipment(&json!({ "label": "", "parcelCount": 200_u32 })), "notes": [] });
    assert_eq!(
        fault_for(&payload)["issues"],
        json!([
            { "detail": "too short: minimum length is 1, got 0", "field": "shipment.label" },
            { "detail": "too large: maximum is 100, got 200", "field": "shipment.parcelCount" },
        ])
    );
}

#[test]
fn a_refusal_raised_while_serde_reads_is_placed_at_the_path_serde_tracked() {
    let cases = [
        // A held brand refuses inside a list: the holders' hooks name no index.
        (
            "shipment.tags.1",
            "'shipment.tags': too short: minimum length is 1, got 0",
            "shipment.tags.1",
        ),
        // A held model's own rule, named below the place serde stopped at.
        (
            "shipment",
            "'shipment.parcelCount': too large: maximum is 100, got 200",
            "shipment.parcelCount",
        ),
        // An optional member no hook names.
        (
            "shipment.alias",
            "'shipment': too short: minimum length is 1, got 0",
            "shipment.alias",
        ),
        // A model in a list, refusing a brand of its own.
        (
            "notes.0.label",
            "'notes.label': too short: minimum length is 1, got 0",
            "notes.0.label",
        ),
        // Nothing tracked: the message names the whole path.
        (
            "",
            "'shipment.label': too short: minimum length is 1, got 0",
            "shipment.label",
        ),
    ];
    for (path, said, field) in cases {
        let (_, words) = said.split_once("': ").unwrap();
        assert_eq!(
            shipping_schema::refused_at(path, said),
            [shipping_schema::ServiceFaultIssue::new(Some(field), words)],
            "{path} / {said}"
        );
    }
}

#[test]
fn each_violation_of_a_refusal_is_placed() {
    let issues = shipping_schema::refused_at(
        "notes.0",
        "'notes.label': too short: minimum length is 1, got 0; 'notes.parcelCount': too large: maximum is 100, got 200",
    );
    assert_eq!(
        issues,
        [
            shipping_schema::ServiceFaultIssue::new(
                Some("notes.0.label"),
                "too short: minimum length is 1, got 0"
            ),
            shipping_schema::ServiceFaultIssue::new(
                Some("notes.0.parcelCount"),
                "too large: maximum is 100, got 200"
            ),
        ]
    );
}
