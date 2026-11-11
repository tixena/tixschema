//! `CarillonService` on `ws_rpc`, whose one operation takes a message that holds a struct in a
//! list, in a map, in an externally, an adjacently and an internally tagged enum, beside a
//! flattened member and in a struct with its own tag, numbers, a member with a bound, and answers
//! the message it was handed. What the Rust dispatcher answers
//! to each payload names the field every emitted dispatcher and client is measured against.
//! Nothing here waits on a clock.

use alloc::collections::BTreeMap;
use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context, Poll, Waker};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Mutex;
use tixschema::{model_schema, service_schema};

use super::carillon_ws_rpc_transport;

/// The message read whole.
pub const STRUCK: &str = r#"{"bells":[{"weight":3},{"weight":4}],"towers":{"north":{"weight":5}},"peal":{"Swung":{"arc":1}},"swing":{"kind":"Wide","data":{"arc":2}},"turn":{"kind":"Half","arc":3},"strokes":2,"label":"a","frame":{"span":1,"oak":2},"yoke":{"kind":"CarillonYoke","pin":3}}"#;

/// What each case of [`CASES`] changes in [`STRUCK`]: the dotted place, and the value written
/// there, `None` taking the key out.
pub const CASES: [(&str, &str, Option<&str>); 23] = [
    ("read whole", "", None),
    ("a key beside a list item", "bells.1.stray", Some("1")),
    ("a key beside a map value", "towers.north.stray", Some("1")),
    (
        "a key beside an external variant",
        "peal.Swung.stray",
        Some("1"),
    ),
    (
        "a key beside an adjacent variant",
        "swing.data.stray",
        Some("1"),
    ),
    ("a key beside an internal variant", "turn.stray", Some("1")),
    ("a list item without its member", "bells.0.weight", None),
    ("an internal variant without its tag", "turn.kind", None),
    (
        "an adjacent variant without its content",
        "swing.data",
        None,
    ),
    ("a text for a number", "strokes", Some(r#""x""#)),
    (
        "a text for a number in a list item",
        "bells.1.weight",
        Some(r#""x""#),
    ),
    (
        "a negative number for a u32",
        "towers.north.weight",
        Some("-1"),
    ),
    ("a null for a number", "strokes", Some("null")),
    ("a list for a number", "strokes", Some("[1]")),
    (
        "a boolean for a number in a variant",
        "peal.Swung.arc",
        Some("true"),
    ),
    ("a number for a text", "label", Some("7")),
    ("a text for a list", "bells", Some(r#""x""#)),
    ("a label below its bound", "label", Some(r#""""#)),
    ("a weight above its bound", "bells.1.weight", Some("101")),
    ("an unknown variant", "peal", Some(r#"{"Rung":{}}"#)),
    (
        "a text for a number in a flattened member",
        "frame.oak",
        Some(r#""x""#),
    ),
    (
        "a text for a number beside a flattened member",
        "frame.span",
        Some(r#""x""#),
    ),
    (
        "a text for a number in a tagged struct",
        "yoke.pin",
        Some(r#""x""#),
    ),
];

/// The cases where the Rust dispatcher names a place above the one the case changes: serde reads a
/// flattened member from the keys its holder buffers, and names the holder. The emitted reads take
/// the member's keys off the holder's own object, and name the place the case changes.
pub const NAMED_ABOVE: [(&str, &str); 1] = [("a text for a number in a flattened member", "frame")];

#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CarillonBell {
    #[model_schema_prop(maximum = 100)]
    pub weight: u32,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CarillonPeal {
    Still,
    Swung { arc: u32 },
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data")]
pub enum CarillonSwing {
    Wide { arc: u32 },
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum CarillonTurn {
    Half { arc: u32 },
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CarillonBeam {
    pub oak: u32,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CarillonFrame {
    #[serde(flatten)]
    pub beam: CarillonBeam,
    pub span: u32,
}

#[model_schema(enable = ["jsonschema"], nonstrict)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub struct CarillonYoke {
    pub pin: u32,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CarillonChime {
    pub bells: Vec<CarillonBell>,
    pub frame: CarillonFrame,
    #[model_schema_prop(minLength = 1)]
    pub label: String,
    pub peal: CarillonPeal,
    pub strokes: u32,
    pub swing: CarillonSwing,
    pub towers: BTreeMap<String, CarillonBell>,
    pub turn: CarillonTurn,
    pub yoke: CarillonYoke,
}

#[model_schema(enable = ["jsonschema"])]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum CarillonError {
    Cracked,
}

/// Strikes the bells of a carillon.
#[service_schema(transports = ["ws_rpc"])]
pub trait CarillonService<Ctx> {
    /// Strikes the chime the message names, and answers it.
    async fn strike(&self, ctx: &Ctx, req: CarillonChime) -> Result<CarillonChime, CarillonError>;
}

/// Counts the strikes it is handed.
#[derive(Default)]
pub struct Carillon {
    struck: Mutex<u32>,
}

impl Carillon {
    /// How many strikes reached the handler.
    pub fn struck(&self) -> u32 {
        *self.struck.lock().unwrap()
    }
}

impl CarillonService<()> for Carillon {
    fn strike(
        &self,
        _ctx: &(),
        req: CarillonChime,
    ) -> impl Future<Output = Result<CarillonChime, CarillonError>> {
        *self.struck.lock().unwrap() += 1;
        ready(Ok(req))
    }
}

/// [`STRUCK`] with the change `case` names.
pub fn payload((_, place, value): (&str, &str, Option<&str>)) -> String {
    let mut changed: Value = serde_json::from_str(STRUCK).unwrap();
    if place.is_empty() {
        return changed.to_string();
    }
    let steps: Vec<&str> = place.split('.').collect();
    let (last, ahead) = steps.split_last().unwrap();
    let mut at = &mut changed;
    for step in ahead {
        at = match step.parse::<usize>() {
            Ok(index) => &mut at[index],
            Err(_) => &mut at[*step],
        };
    }
    match value {
        Some(written) => at[*last] = serde_json::from_str(written).unwrap(),
        None => {
            at.as_object_mut().unwrap().remove(*last);
        }
    }
    changed.to_string()
}

/// The request frame that carries `payload` to `strike`.
pub fn frame(payload: &str) -> String {
    json!({
        "kind": "request",
        "id": "ID",
        "service": "CarillonService",
        "operation": "strike",
        "payload": serde_json::from_str::<Value>(payload).unwrap(),
    })
    .to_string()
}

/// The reply the Rust `ws_rpc` dispatcher writes to `payload`, and how many strikes reached the
/// handler.
pub fn over_ws(payload: &str) -> (Value, u32) {
    let carillon = Carillon::default();
    let framed = frame(payload);
    let mut answering = pin!(carillon_ws_rpc_transport::answer(&framed, &carillon, &()));
    let polled = answering
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()));
    // The dispatcher answers on its first poll: the handler never suspends.
    let reply = match polled {
        Poll::Ready(answered) => answered,
        Poll::Pending => None,
    };
    (
        serde_json::from_str(&reply.unwrap()).unwrap(),
        carillon.struck(),
    )
}

/// The field the Rust dispatcher names for each case, `null` for the message it reads.
pub fn rust_fields() -> Vec<Value> {
    CASES
        .into_iter()
        .map(|case| {
            let (reply, _) = over_ws(&payload(case));
            reply["error"]["fault"]["field"].clone()
        })
        .collect()
}

#[test]
fn the_rust_dispatcher_names_the_place_of_each_case() {
    let (read, struck) = over_ws(STRUCK);
    assert_eq!(read["ok"], true, "{read}");
    assert_eq!(
        read["value"],
        serde_json::from_str::<Value>(STRUCK).unwrap()
    );
    assert_eq!(struck, 1);
    for (case, field) in CASES.into_iter().zip(rust_fields()).skip(1) {
        let (refused, strikes) = over_ws(&payload(case));
        assert_eq!(
            refused["error"]["fault"]["kind"], "failed-validation",
            "{refused}"
        );
        assert_eq!(strikes, 0, "{}", case.0);
        let place = NAMED_ABOVE
            .into_iter()
            .find_map(|(named, above)| (named == case.0).then_some(above))
            .unwrap_or(case.1);
        assert_eq!(field, place, "{}", case.0);
    }
}
