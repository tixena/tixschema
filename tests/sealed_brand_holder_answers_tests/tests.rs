//! `Probe`: one operation whose message holds a bounded brand and a brand under a check in each
//! shape the answer of a holder was measured for.

use crate::probe_ws;
use alloc::collections::BTreeMap;
use core::future::{Future as _, ready};
use core::pin::pin;
use core::task::{Context, Poll, Waker};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tixschema::{model_schema, service_schema};

#[model_schema(minLength = 1, maxLength = 80)]
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct LabelText(String);

#[model_schema(check = ["audible"])]
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ToneText(String);

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Either {
    Audible(ToneText),
    Plain(String),
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum Tag {
    One { tone: ToneText },
}

#[model_schema()]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Probe {
    pub either: Either,
    pub keyed: BTreeMap<ToneText, u32>,
    pub labels: Vec<LabelText>,
    pub tag: Tag,
    pub text: LabelText,
    pub tone: ToneText,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum ProbeError {
    Cracked,
}

#[service_schema(transports = ["ws_rpc"])]
pub trait ProbeService<Ctx> {
    async fn strike(&self, ctx: &Ctx, req: Probe) -> Result<Probe, ProbeError>;
}

struct Echo;

impl ProbeService<()> for Echo {
    async fn strike(&self, _ctx: &(), req: Probe) -> Result<Probe, ProbeError> {
        ready(Ok(req)).await
    }
}

fn audible(value: &ToneText) -> Result<(), Vec<String>> {
    if value.as_inner() == "mute" {
        Err(vec!["mute is silent".to_owned()])
    } else {
        Ok(())
    }
}

fn answered(payload: &Value) -> Value {
    let frame = json!({
        "kind": "request",
        "id": "1",
        "service": "ProbeService",
        "operation": "strike",
        "payload": payload,
    })
    .to_string();
    let mut answering = pin!(probe_ws::answer(&frame, &Echo, &()));
    let Poll::Ready(Some(reply)) = answering
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    else {
        return Value::Null;
    };
    serde_json::from_str(&reply).unwrap()
}

fn payload_with(place: &str, value: Value) -> Value {
    let mut payload = json!({
        "either": "fine",
        "keyed": { "fine": 1_u32 },
        "labels": ["a", "b"],
        "tag": { "kind": "One", "tone": "fine" },
        "text": "a",
        "tone": "ok",
    });
    let steps: Vec<&str> = place.split('.').collect();
    let (last, ahead) = steps.split_last().unwrap();
    let mut at = &mut payload;
    for step in ahead {
        at = &mut at[*step];
    }
    match last.parse::<usize>() {
        Ok(index) if at.is_array() => at[index] = value,
        _ => at[*last] = value,
    }
    payload
}

fn fault_of(place: &str, value: Value) -> Value {
    answered(&payload_with(place, value))["error"]["fault"].clone()
}

#[test]
fn a_whole_payload_is_read_and_echoed() {
    let reply = answered(&payload_with("text", json!("a")));
    assert_eq!(reply["ok"], true, "{reply}");
}

#[test]
fn an_item_of_a_list_below_the_bound_is_named_at_its_index() {
    let fault = fault_of("labels", json!(["a", ""]));
    assert_eq!(fault["kind"], "failed-validation");
    assert_eq!(fault["field"], "labels.1");
    assert_eq!(
        fault["detail"],
        "'labels.1': too short: minimum length is 1, got 0"
    );
}

#[test]
fn a_tagged_variants_brand_is_named_at_its_member() {
    let fault = fault_of("tag", json!({ "kind": "One", "tone": "mute" }));
    assert_eq!(fault["kind"], "failed-validation");
    assert_eq!(fault["field"], "tag.tone");
    assert_eq!(fault["detail"], "'tag.tone': mute is silent");
}

#[test]
fn an_untagged_variants_brand_refuses_by_its_check() {
    let fault = fault_of("either", json!("mute"));
    assert_eq!(fault["kind"], "failed-validation");
    assert_eq!(fault["field"], "either");
    assert_eq!(fault["detail"], "'either': mute is silent");
}
