//! `DocumentService` on `amqp_rpc`, `http_rest` and `ws_rpc`: one operation taking a list of
//! drafts, and one operation for each shape serde writes a member under.

#![cfg(feature = "serde")]

use crate::amqp_transport;
use crate::http_rest_transport;
use crate::ws_transport;
use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Mutex;
use tixschema::{model_schema, service_schema};

macro_rules! reads {
    ($($method:ident: $message:ty),* $(,)?) => {$(
        fn $method(
            &self,
            _ctx: &(),
            _req: $message,
        ) -> impl Future<Output = Result<Tally, TallyError>> {
            self.reached.lock().unwrap().push(stringify!($method).to_owned());
            ready(Ok(Tally { drafts: 0 }))
        }
    )*};
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Reference {
    pub name: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    pub references: Vec<Reference>,
    #[model_schema_prop(minLength = 1)]
    pub title: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DraftBatch {
    pub drafts: Vec<Draft>,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Tally {
    pub drafts: u32,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum TallyError {
    Uncountable,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Struct {
    #[model_schema_prop(minLength = 1)]
    pub label: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Slots(pub String, #[model_schema_prop(minLength = 1)] pub String);

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Newtype(#[model_schema_prop(minLength = 1)] pub String);

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Variants {
    Boxed(#[model_schema_prop(minLength = 1)] String),
    Loose {
        #[model_schema_prop(minLength = 1)]
        label: String,
    },
    Split(String, #[model_schema_prop(minLength = 1)] String),
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "held")]
pub enum AdjacentlyTagged {
    Boxed(#[model_schema_prop(minLength = 1)] String),
    Loose {
        #[model_schema_prop(minLength = 1)]
        label: String,
    },
    Split(String, #[model_schema_prop(minLength = 1)] String),
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum InternallyTagged {
    Boxed(Struct),
    Loose {
        #[model_schema_prop(minLength = 1)]
        label: String,
    },
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Untagged {
    Loose {
        #[model_schema_prop(minLength = 1)]
        label: String,
    },
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Flattened {
    #[serde(flatten)]
    pub inner: Struct,
    #[serde(default)]
    pub note: String,
}

/// Counts what it is handed.
#[service_schema(transports = ["amqp_rpc", "http_rest", "ws_rpc"])]
pub trait DocumentService<Ctx> {
    /// Counts the drafts.
    async fn check_drafts(&self, ctx: &Ctx, req: DraftBatch) -> Result<Tally, TallyError>;

    /// Reads a struct.
    async fn read_adjacently_tagged(
        &self,
        ctx: &Ctx,
        req: AdjacentlyTagged,
    ) -> Result<Tally, TallyError>;

    /// Reads a struct holding a flattened struct.
    async fn read_flattened(&self, ctx: &Ctx, req: Flattened) -> Result<Tally, TallyError>;

    /// Reads an internally tagged enum.
    async fn read_internally_tagged(
        &self,
        ctx: &Ctx,
        req: InternallyTagged,
    ) -> Result<Tally, TallyError>;

    /// Reads a tuple struct of one slot.
    async fn read_newtype(&self, ctx: &Ctx, req: Newtype) -> Result<Tally, TallyError>;

    /// Reads a tuple struct of several slots.
    async fn read_slots(&self, ctx: &Ctx, req: Slots) -> Result<Tally, TallyError>;

    /// Reads a struct.
    async fn read_struct(&self, ctx: &Ctx, req: Struct) -> Result<Tally, TallyError>;

    /// Reads an untagged enum.
    async fn read_untagged(&self, ctx: &Ctx, req: Untagged) -> Result<Tally, TallyError>;

    /// Reads an externally tagged enum.
    async fn read_variants(&self, ctx: &Ctx, req: Variants) -> Result<Tally, TallyError>;

    /// Tags a draft, taking its arguments one by one.
    async fn tag_draft(&self, ctx: &Ctx, title: String, count: u32) -> Result<Tally, TallyError>;
}

#[derive(Default)]
pub struct Back {
    reached: Mutex<Vec<String>>,
}

impl DocumentService<()> for Back {
    fn check_drafts(
        &self,
        _ctx: &(),
        req: DraftBatch,
    ) -> impl Future<Output = Result<Tally, TallyError>> {
        self.reached.lock().unwrap().push("check_drafts".to_owned());
        ready(Ok(Tally {
            drafts: u32::try_from(req.drafts.len()).unwrap_or(u32::MAX),
        }))
    }

    reads!(
        read_adjacently_tagged: AdjacentlyTagged,
        read_flattened: Flattened,
        read_internally_tagged: InternallyTagged,
        read_newtype: Newtype,
        read_slots: Slots,
        read_struct: Struct,
        read_untagged: Untagged,
        read_variants: Variants,
    );

    fn tag_draft(
        &self,
        _ctx: &(),
        title: String,
        count: u32,
    ) -> impl Future<Output = Result<Tally, TallyError>> {
        self.reached.lock().unwrap().push("tag_draft".to_owned());
        ready(Ok(Tally {
            drafts: count + u32::try_from(title.len()).unwrap_or(u32::MAX),
        }))
    }
}

struct Captured(Mutex<Option<Value>>);

impl amqp_transport::Reply for Captured {
    async fn fault(&self, fault: document_service_schema::ServiceFault) {
        ready(()).await;
        *self.0.lock().unwrap() = Some(serde_json::to_value(&fault).unwrap());
    }

    async fn send<T>(&self, value: T, _headers: Vec<(String, String)>)
    where
        T: Serialize + Send,
    {
        ready(()).await;
        *self.0.lock().unwrap() =
            Some(json!({ "answered": serde_json::to_value(&value).unwrap() }));
    }
}

/// Answers every fault as its own JSON.
struct FaultAsJson;

impl http_rest_transport::FaultHandler for FaultAsJson {
    fn on_fault(
        &self,
        fault: &document_service_schema::ServiceFault,
    ) -> http_rest_transport::OutgoingResponse {
        http_rest_transport::OutgoingResponse::new(
            422,
            vec![("content-type".to_owned(), "application/json".to_owned())],
            serde_json::to_vec(fault).unwrap(),
        )
    }
}

fn poll_once<Answered>(answering: Answered) -> Option<Answered::Output>
where
    Answered: Future,
{
    let mut pinned = pin!(answering);
    let mut polling = PollContext::from_waker(Waker::noop());
    match pinned.as_mut().poll(&mut polling) {
        Poll::Ready(answer) => Some(answer),
        Poll::Pending => None,
    }
}

/// The fault `payload` earns on `amqp_rpc` for `operation`.
fn over_amqp(operation: &str, payload: &Value) -> Value {
    let captured = Captured(Mutex::new(None));
    poll_once(amqp_transport::dispatch(
        &Back::default(),
        &(),
        &amqp_transport::IncomingMessage::new(
            operation.to_owned(),
            serde_json::to_vec(payload).unwrap(),
            Vec::new(),
        ),
        &captured,
    ))
    .unwrap();
    captured.0.lock().unwrap().take().unwrap()
}

/// The fault `payload` earns on `http_rest` for `operation`, as `FaultAsJson` writes it.
fn over_http(operation: &str, payload: &Value) -> Value {
    let request = http_rest_transport::IncomingRequest::new(
        "POST".to_owned(),
        format!("/{operation}"),
        String::new(),
        Vec::new(),
        serde_json::to_vec(payload).unwrap(),
    );
    assert_eq!(request.headers(), &[] as &[(String, String)]);
    assert_eq!(request.query(), "");
    assert_eq!(request.header("content-type"), None);
    let answered = poll_once(http_rest_transport::dispatch(
        &Back::default(),
        &(),
        &request,
        &FaultAsJson,
    ))
    .unwrap();
    assert_eq!(answered.status(), 422);
    assert!(
        answered
            .headers()
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("content-type"))
    );
    serde_json::from_slice(answered.body()).unwrap()
}

/// The fault `payload` earns on `ws_rpc` for `operation`.
fn over_ws(operation: &str, payload: &Value) -> Value {
    let frame = json!({
        "kind": "request",
        "id": "1",
        "service": "DocumentService",
        "operation": operation,
        "payload": payload,
    });
    let reply = poll_once(ws_transport::answer(
        &frame.to_string(),
        &Back::default(),
        &(),
    ))
    .flatten()
    .unwrap();
    let parsed: Value = serde_json::from_str(&reply).unwrap();
    parsed["error"]["fault"].clone()
}

/// The fault of every transport for the same request, which must be one fault.
fn faults(operation: &str, payload: &Value) -> Value {
    let amqp = over_amqp(operation, payload);
    let ws = over_ws(operation, payload);
    assert_eq!(amqp, ws, "amqp_rpc and ws_rpc disagree");
    amqp
}

#[test]
fn a_missing_key_inside_a_list_answers_failed_validation_with_its_full_path_on_every_transport() {
    let payload =
        json!({ "drafts": [{ "references": [], "title": "Plan" }, { "references": [] }] });
    let expected = json!({
        "detail": "'drafts.1.title': missing field `title`",
        "field": "drafts.1.title",
        "kind": "failed-validation",
        "operation": "check-drafts",
    });
    assert_eq!(over_amqp("check-drafts", &payload), expected);
    assert_eq!(over_ws("check-drafts", &payload), expected);
    assert_eq!(over_http("check-drafts", &payload), expected);
}

#[test]
fn a_value_of_the_wrong_type_two_lists_deep_names_its_full_path_on_every_transport() {
    let payload = json!({ "drafts": [{ "references": [{ "name": 5_u8 }], "title": "Plan" }] });
    let expected = json!({
        "detail": "'drafts.0.references.0.name': invalid type: integer `5`, expected a string",
        "field": "drafts.0.references.0.name",
        "kind": "failed-validation",
        "operation": "check-drafts",
    });
    assert_eq!(over_amqp("check-drafts", &payload), expected);
    assert_eq!(over_ws("check-drafts", &payload), expected);
    assert_eq!(over_http("check-drafts", &payload), expected);
}

#[test]
fn an_unknown_key_inside_a_list_names_the_key_once_on_every_transport() {
    let payload = json!({ "drafts": [
        { "references": [], "title": "Plan" },
        { "references": [], "subtitle": "Draft", "title": "Plan" },
    ] });
    let expected = json!({
        "detail": "'drafts.1.subtitle': unknown field `subtitle`, expected `references` or `title`",
        "field": "drafts.1.subtitle",
        "kind": "failed-validation",
        "operation": "check-drafts",
    });
    assert_eq!(over_amqp("check-drafts", &payload), expected);
    assert_eq!(over_ws("check-drafts", &payload), expected);
    assert_eq!(over_http("check-drafts", &payload), expected);
}

/// The fault `payload` earns for `operation` on every transport, which must be one fault.
fn faults_everywhere(operation: &str, payload: &Value) -> Value {
    let fault = faults(operation, payload);
    assert_eq!(over_http(operation, payload), fault, "http_rest disagrees");
    fault
}

#[test]
fn an_unknown_key_in_a_generated_message_is_refused_and_the_handler_is_not_called() {
    let payload = json!({ "title": "Plan", "count": 1_u8, "stray": 1_u8 });
    let back = Back::default();
    let captured = Captured(Mutex::new(None));
    poll_once(amqp_transport::dispatch(
        &back,
        &(),
        &amqp_transport::IncomingMessage::new(
            "tag-draft".to_owned(),
            serde_json::to_vec(&payload).unwrap(),
            Vec::new(),
        ),
        &captured,
    ))
    .unwrap();
    assert!(back.reached.lock().unwrap().is_empty());
    let expected = json!({
        "detail": "'stray': unknown field `stray`, expected `title` or `count`",
        "field": "stray",
        "kind": "failed-validation",
        "operation": "tag-draft",
    });
    assert_eq!(captured.0.lock().unwrap().take().unwrap(), expected);
    assert_eq!(faults_everywhere("tag-draft", &payload), expected);
}

#[test]
fn an_unknown_key_in_a_model_nobody_marked_is_refused_inside_a_list() {
    let payload = json!({ "drafts": [
        { "references": [{ "name": "a", "stray": 1_u8 }], "title": "Plan" },
    ] });
    let expected = json!({
        "detail": "'drafts.0.references.0.stray': unknown field `stray`, expected `name`",
        "field": "drafts.0.references.0.stray",
        "kind": "failed-validation",
        "operation": "check-drafts",
    });
    assert_eq!(faults_everywhere("check-drafts", &payload), expected);
}

#[test]
fn an_unknown_key_under_a_flattened_struct_or_a_tag_alone_is_refused_at_the_key() {
    for (operation, payload) in [
        ("read-flattened", json!({ "label": "a", "stray": 1_u8 })),
        (
            "read-internally-tagged",
            json!({ "kind": "Loose", "label": "a", "stray": 1_u8 }),
        ),
    ] {
        let fault = faults_everywhere(operation, &payload);
        assert_eq!(fault["kind"], "failed-validation", "{operation}");
        assert_eq!(fault["field"], "stray", "{operation}");
        assert!(
            fault["detail"]
                .as_str()
                .unwrap()
                .starts_with("'stray': unknown field `stray`"),
            "{operation}: {fault}"
        );
    }
}

#[test]
fn text_that_is_no_json_still_answers_undeserializable_payload() {
    let captured = Captured(Mutex::new(None));
    poll_once(amqp_transport::dispatch(
        &Back::default(),
        &(),
        &amqp_transport::IncomingMessage::new(
            "check-drafts".to_owned(),
            b"not json".to_vec(),
            Vec::new(),
        ),
        &captured,
    ))
    .unwrap();
    let fault = captured.0.lock().unwrap().take().unwrap();
    assert_eq!(fault["kind"], "undeserializable-payload");
    assert_eq!(fault.get("field"), None);

    let request = http_rest_transport::IncomingRequest::new(
        "POST".to_owned(),
        "/check-drafts".to_owned(),
        String::new(),
        Vec::new(),
        b"not json".to_vec(),
    );
    let answered = poll_once(http_rest_transport::dispatch(
        &Back::default(),
        &(),
        &request,
        &http_rest_transport::DefaultFaultHandler,
    ))
    .unwrap();
    assert_eq!(answered.status(), 400);
    let body: Value = serde_json::from_slice(answered.body()).unwrap();
    assert_eq!(body["kind"], "undeserializable-payload");
}

/// What a row of the table names for a member serde refused and for a bound it broke, as the
/// `field` of the fault each answers.
fn named(operation: &str, refused: &Value, broken: &Value) -> (Option<String>, Option<String>) {
    let field = |fault: Value| {
        fault
            .get("field")
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    (
        field(faults(operation, refused)),
        field(faults(operation, broken)),
    )
}

#[test]
fn the_path_of_a_refusal_is_the_path_validate_names_for_the_same_member() {
    let rows = [
        (
            "read-struct",
            json!({ "label": 5_u8 }),
            json!({ "label": "" }),
            Some("label"),
        ),
        (
            "read-slots",
            json!(["a", 5_u8]),
            json!(["a", ""]),
            Some("1"),
        ),
        ("read-newtype", json!(5_u8), json!(""), None),
        (
            "read-variants",
            json!({ "Loose": { "label": 5_u8 } }),
            json!({ "Loose": { "label": "" } }),
            Some("Loose.label"),
        ),
        (
            "read-variants",
            json!({ "Boxed": 5_u8 }),
            json!({ "Boxed": "" }),
            Some("Boxed"),
        ),
        (
            "read-variants",
            json!({ "Split": ["a", 5_u8] }),
            json!({ "Split": ["a", ""] }),
            Some("Split.1"),
        ),
        (
            "read-adjacently-tagged",
            json!({ "kind": "Loose", "held": { "label": 5_u8 } }),
            json!({ "kind": "Loose", "held": { "label": "" } }),
            Some("held.label"),
        ),
        (
            "read-adjacently-tagged",
            json!({ "kind": "Boxed", "held": 5_u8 }),
            json!({ "kind": "Boxed", "held": "" }),
            Some("held"),
        ),
        (
            "read-adjacently-tagged",
            json!({ "kind": "Split", "held": ["a", 5_u8] }),
            json!({ "kind": "Split", "held": ["a", ""] }),
            Some("held.1"),
        ),
    ];
    for (operation, refused, broken, expected) in rows {
        let (by_serde, by_validate) = named(operation, &refused, &broken);
        assert_eq!(by_validate.as_deref(), expected, "{operation} {broken}");
        assert_eq!(by_serde, by_validate, "{operation} {refused}");
    }
}

#[test]
fn a_member_serde_reads_from_a_buffer_names_what_serde_path_to_error_reaches() {
    let internally = |refused: Value| {
        faults("read-internally-tagged", &refused)
            .get("field")
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    assert_eq!(internally(json!({ "kind": "Loose", "label": 5_u8 })), None);
    assert_eq!(internally(json!({ "kind": "Boxed", "label": 5_u8 })), None);
    let flattened = faults("read-flattened", &json!({ "label": 5_u8 }));
    assert_eq!(flattened.get("field"), None);
    assert_eq!(
        flattened["detail"],
        "invalid type: integer `5`, expected a string"
    );
    let untagged = faults("read-untagged", &json!({ "label": 5_u8 }));
    assert_eq!(untagged.get("field"), None);
    assert_eq!(
        untagged["detail"],
        "data did not match any variant of untagged enum Untagged"
    );
}

#[test]
fn every_operation_is_routed_at_a_post_of_its_wire_name() {
    for route in http_rest_transport::ROUTES {
        assert_eq!(route.method(), "POST");
        assert_eq!(route.path(), format!("/{}", route.operation()));
        assert_eq!(route.ok_status(), 200);
        assert_eq!(route.error_statuses(), &[422_u16]);
    }
}
