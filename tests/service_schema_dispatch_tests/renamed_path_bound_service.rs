//! Operations whose path binds a field of a declared message, under each way serde names the
//! field: the implementation gets the path's value, whatever the body carries.

#![cfg(feature = "serde")]

use core::future::{Future, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tixschema::{model_schema, service_schema};

use crate::renamed_path_bound_http_rest_client::{
    IncomingResponse, OutgoingRequest, RenamedPathBoundServiceClient, Transport,
};
use crate::renamed_path_bound_http_rest_transport::{
    DefaultFaultHandler, FaultHandler, IncomingRequest, OutgoingResponse, ROUTES, Route, dispatch,
};
use crate::renamed_path_bound_service_schema::ServiceFault;

/// An alias of a renamed message, which reads the keys of the type it names.
pub type BoundAliased = BoundRenamed;

/// A message serde names as Rust does.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct BoundPlain {
    pub document_id: String,
    pub title: String,
}

/// A message under a `rename_all`.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundRenamed {
    pub document_id: String,
    pub title: String,
}

/// A message whose member has a key of its own, which is no identifier.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct BoundOwnKey {
    #[serde(rename = "doc-id")]
    pub document_id: String,
    pub title: String,
}

/// A renamed message that declares a parameter.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundGeneric<Held> {
    pub document_id: String,
    pub held: Held,
}

/// What `BoundBesideFlatten` flattens.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct BoundExtra {
    pub title: String,
}

/// A renamed member beside a flattened one.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct BoundBesideFlatten {
    #[serde(rename = "docId")]
    pub document_id: String,
    #[serde(flatten)]
    pub extra: BoundExtra,
}

/// A member Rust writes as a raw identifier, under a `rename_all`.
#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct BoundKeyword {
    pub title: String,
    pub r#type: String,
}

/// A message no `#[model_schema]` wrote, which publishes no key of its members: its placeholder's
/// own text is the key it is read from.
#[derive(Debug, Deserialize, Serialize)]
pub struct BoundUnlisted {
    pub document_id: String,
    pub title: String,
}

/// The id an implementation was handed.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BoundAnswer {
    pub id: String,
}

/// The error each operation declares.
#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "errorCode")]
pub enum BoundRefused {
    Nope,
}

/// One operation for each way serde names a member, one whose path binds two, and one whose
/// message the macro generates.
#[service_schema(transports = ["http_rest"])]
pub trait RenamedPathBoundService<Ctx> {
    #[service_schema_op(http(method = "PUT", path = "/aliased/{document_id}"))]
    async fn aliased(&self, ctx: &Ctx, req: BoundAliased) -> Result<BoundAnswer, BoundRefused>;

    #[service_schema_op(http(method = "PUT", path = "/beside/{document_id}"))]
    async fn beside_flatten(
        &self,
        ctx: &Ctx,
        req: BoundBesideFlatten,
    ) -> Result<BoundAnswer, BoundRefused>;

    #[service_schema_op(http(method = "PUT", path = "/generated/{document_id}"))]
    async fn generated(
        &self,
        ctx: &Ctx,
        document_id: String,
        title: String,
    ) -> Result<BoundAnswer, BoundRefused>;

    #[service_schema_op(http(method = "PUT", path = "/generic/{document_id}"))]
    async fn generic(
        &self,
        ctx: &Ctx,
        req: BoundGeneric<String>,
    ) -> Result<BoundAnswer, BoundRefused>;

    #[service_schema_op(http(method = "PUT", path = "/keyword/{type}"))]
    async fn keyword(&self, ctx: &Ctx, req: BoundKeyword) -> Result<BoundAnswer, BoundRefused>;

    #[service_schema_op(http(method = "PUT", path = "/own/{document_id}"))]
    async fn own_key(&self, ctx: &Ctx, req: BoundOwnKey) -> Result<BoundAnswer, BoundRefused>;

    #[service_schema_op(http(
        method = "PUT",
        path = "/plain/{document_id}",
        error_status(Nope = 409),
    ))]
    async fn plain(&self, ctx: &Ctx, req: BoundPlain) -> Result<BoundAnswer, BoundRefused>;

    #[service_schema_op(http(
        method = "PUT",
        path = "/renamed/{document_id}",
        error_status(Nope = 409),
    ))]
    async fn renamed(&self, ctx: &Ctx, req: BoundRenamed) -> Result<BoundAnswer, BoundRefused>;

    #[service_schema_op(http(method = "PUT", path = "/two/{document_id}/{title}"))]
    async fn two_placeholders(
        &self,
        ctx: &Ctx,
        req: BoundRenamed,
    ) -> Result<BoundAnswer, BoundRefused>;

    #[service_schema_op(http(method = "PUT", path = "/unlisted/{document_id}"))]
    async fn unlisted(&self, ctx: &Ctx, req: BoundUnlisted) -> Result<BoundAnswer, BoundRefused>;
}

/// Answers each operation with the id its implementation was handed.
pub struct BoundBackEnd;

impl RenamedPathBoundService<()> for BoundBackEnd {
    async fn aliased(&self, _ctx: &(), req: BoundAliased) -> Result<BoundAnswer, BoundRefused> {
        ready(()).await;
        answered(req.document_id)
    }

    async fn beside_flatten(
        &self,
        _ctx: &(),
        req: BoundBesideFlatten,
    ) -> Result<BoundAnswer, BoundRefused> {
        ready(()).await;
        answered(req.document_id)
    }

    async fn generated(
        &self,
        _ctx: &(),
        document_id: String,
        _title: String,
    ) -> Result<BoundAnswer, BoundRefused> {
        ready(()).await;
        answered(document_id)
    }

    async fn generic(
        &self,
        _ctx: &(),
        req: BoundGeneric<String>,
    ) -> Result<BoundAnswer, BoundRefused> {
        ready(()).await;
        answered(req.document_id)
    }

    async fn keyword(&self, _ctx: &(), req: BoundKeyword) -> Result<BoundAnswer, BoundRefused> {
        ready(()).await;
        answered(req.r#type)
    }

    async fn own_key(&self, _ctx: &(), req: BoundOwnKey) -> Result<BoundAnswer, BoundRefused> {
        ready(()).await;
        answered(req.document_id)
    }

    async fn plain(&self, _ctx: &(), req: BoundPlain) -> Result<BoundAnswer, BoundRefused> {
        ready(()).await;
        answered(req.document_id)
    }

    async fn renamed(&self, _ctx: &(), req: BoundRenamed) -> Result<BoundAnswer, BoundRefused> {
        ready(()).await;
        answered(req.document_id)
    }

    async fn two_placeholders(
        &self,
        _ctx: &(),
        req: BoundRenamed,
    ) -> Result<BoundAnswer, BoundRefused> {
        ready(()).await;
        answered(format!("{}|{}", req.document_id, req.title))
    }

    async fn unlisted(&self, _ctx: &(), req: BoundUnlisted) -> Result<BoundAnswer, BoundRefused> {
        ready(()).await;
        answered(req.document_id)
    }
}

/// Carries a request the Rust client writes to the Rust dispatcher, and holds each path it carried.
#[derive(Default)]
struct Dispatched {
    carried: Mutex<Vec<String>>,
}

impl Transport for Dispatched {
    async fn send(&self, request: OutgoingRequest) -> Result<IncomingResponse, String> {
        self.carried.lock().unwrap().push(request.path().to_owned());
        let incoming = IncomingRequest::new(
            request.method().to_owned(),
            request.path().to_owned(),
            request.query().to_owned(),
            request.headers().to_vec(),
            request.body().to_vec(),
        );
        let response = dispatch(&BoundBackEnd, &(), &incoming, &DefaultFaultHandler).await;
        let read = IncomingResponse::new(
            response.status(),
            response.headers().to_vec(),
            response.body().to_vec(),
        );
        assert_eq!(read.header("X-None"), None, "no response names this header");
        Ok(read)
    }
}

/// Answers each fault `499`, through the response an adapter would build.
struct Noted;

impl FaultHandler for Noted {
    fn on_fault(&self, fault: &ServiceFault) -> OutgoingResponse {
        OutgoingResponse::new(
            499,
            vec![("x-fault".to_owned(), format!("{}", fault.kind()))],
            fault.detail().as_bytes().to_vec(),
        )
    }
}

fn answered(id: String) -> Result<BoundAnswer, BoundRefused> {
    if id == "refused" {
        return Err(BoundRefused::Nope);
    }
    Ok(BoundAnswer { id })
}

/// The back end never suspends, so one poll answers it.
fn poll_once<Answered>(answering: Answered) -> Option<Answered::Output>
where
    Answered: Future,
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

fn put(path: &str, body: &str) -> (u16, String) {
    let request = IncomingRequest::new(
        "PUT".to_owned(),
        path.to_owned(),
        String::new(),
        Vec::new(),
        body.as_bytes().to_vec(),
    );
    let response = poll_once(dispatch(&BoundBackEnd, &(), &request, &DefaultFaultHandler)).unwrap();
    (
        response.status(),
        String::from_utf8(response.body().to_vec()).unwrap(),
    )
}

/// Every operation with a body that carries another value under the key the operation reads the
/// member from, and a body that carries none.
#[test]
fn the_path_value_reaches_the_implementation_whatever_the_body_carries() {
    for (path, elsewhere, without) in [
        (
            "/plain/d1",
            r#"{"document_id":"elsewhere","title":"t"}"#,
            r#"{"title":"t"}"#,
        ),
        (
            "/renamed/d1",
            r#"{"documentId":"elsewhere","title":"t"}"#,
            r#"{"title":"t"}"#,
        ),
        (
            "/own/d1",
            r#"{"doc-id":"elsewhere","title":"t"}"#,
            r#"{"title":"t"}"#,
        ),
        (
            "/generic/d1",
            r#"{"documentId":"elsewhere","held":"t"}"#,
            r#"{"held":"t"}"#,
        ),
        (
            "/aliased/d1",
            r#"{"documentId":"elsewhere","title":"t"}"#,
            r#"{"title":"t"}"#,
        ),
        (
            "/beside/d1",
            r#"{"docId":"elsewhere","title":"t"}"#,
            r#"{"title":"t"}"#,
        ),
        (
            "/keyword/d1",
            r#"{"TYPE":"elsewhere","TITLE":"t"}"#,
            r#"{"TITLE":"t"}"#,
        ),
        (
            "/unlisted/d1",
            r#"{"document_id":"elsewhere","title":"t"}"#,
            r#"{"title":"t"}"#,
        ),
        (
            "/generated/d1",
            r#"{"documentId":"elsewhere","title":"t"}"#,
            r#"{"title":"t"}"#,
        ),
    ] {
        for body in [elsewhere, without] {
            assert_eq!(
                put(path, body),
                (200, r#"{"id":"d1"}"#.to_owned()),
                "PUT {path} {body}"
            );
        }
    }
}

#[test]
fn each_placeholder_of_a_path_reaches_its_own_member() {
    assert_eq!(
        put(
            "/two/d1/t1",
            r#"{"documentId":"elsewhere","title":"other"}"#
        ),
        (200, r#"{"id":"d1|t1"}"#.to_owned())
    );
}

#[test]
fn a_struct_publishes_the_key_serde_reads_each_member_from() {
    assert_eq!(
        BoundRenamed::WIRE_KEYS,
        Some(&[("document_id", "documentId"), ("title", "title")][..])
    );
    assert_eq!(
        BoundOwnKey::WIRE_KEYS,
        Some(&[("document_id", "doc-id"), ("title", "title")][..])
    );
    assert_eq!(
        BoundBesideFlatten::WIRE_KEYS,
        Some(&[("document_id", "docId")][..])
    );
    assert_eq!(
        BoundKeyword::WIRE_KEYS,
        Some(&[("title", "TITLE"), ("type", "TYPE")][..])
    );
    assert_eq!(
        BoundGeneric::<String>::WIRE_KEYS,
        Some(&[("document_id", "documentId"), ("held", "held")][..])
    );
}

#[test]
fn the_rust_client_and_dispatcher_round_trip_each_member() {
    let client = RenamedPathBoundServiceClient::new(Dispatched::default());
    let (id, title) = ("d1".to_owned(), "t".to_owned());
    let renamed = || BoundRenamed {
        document_id: id.clone(),
        title: title.clone(),
    };
    let called = [
        poll_once(client.plain(BoundPlain {
            document_id: id.clone(),
            title: title.clone(),
        })),
        poll_once(client.renamed(renamed())),
        poll_once(client.own_key(BoundOwnKey {
            document_id: id.clone(),
            title: title.clone(),
        })),
        poll_once(client.generic(BoundGeneric {
            document_id: id.clone(),
            held: title.clone(),
        })),
        poll_once(client.aliased(renamed())),
        poll_once(client.beside_flatten(BoundBesideFlatten {
            document_id: id.clone(),
            extra: BoundExtra {
                title: title.clone(),
            },
        })),
        poll_once(client.keyword(BoundKeyword {
            title: title.clone(),
            r#type: id.clone(),
        })),
        poll_once(client.unlisted(BoundUnlisted {
            document_id: id.clone(),
            title: title.clone(),
        })),
        poll_once(client.generated(id.clone(), title.clone())),
    ];
    for answer in called {
        assert_eq!(
            answer.unwrap().ok(),
            Some(BoundAnswer { id: id.clone() }),
            "a client call answered another id"
        );
    }
    assert_eq!(
        poll_once(client.two_placeholders(renamed())).unwrap().ok(),
        Some(BoundAnswer {
            id: "d1|t".to_owned()
        })
    );
}

#[test]
fn a_renamed_member_still_answers_the_declared_error() {
    assert_eq!(put("/renamed/refused", r#"{"title":"t"}"#).0, 409);
    let client = RenamedPathBoundServiceClient::new(Dispatched::default());
    let answer = poll_once(client.renamed(BoundRenamed {
        document_id: "refused".to_owned(),
        title: "t".to_owned(),
    }))
    .unwrap();
    assert!(answer.is_err(), "the declared error did not come back");
    assert_eq!(
        *client.transport().carried.lock().unwrap(),
        ["/renamed/refused"]
    );
}

/// A body whose key is the member's name in Rust does not reach the member: the path's value is
/// over the key the member is read from.
#[test]
fn a_body_under_the_rust_name_does_not_reach_the_member() {
    let request = IncomingRequest::new(
        "PUT".to_owned(),
        "/renamed/d1".to_owned(),
        "unused=1".to_owned(),
        vec![("x-trace".to_owned(), "abc".to_owned())],
        br#"{"document_id":"elsewhere","title":"t"}"#.to_vec(),
    );
    assert_eq!(request.query(), "unused=1");
    assert_eq!(request.header("X-Trace"), Some("abc"));
    assert_eq!(request.headers().len(), 1);
    let response = poll_once(dispatch(&BoundBackEnd, &(), &request, &Noted)).unwrap();
    assert_eq!(response.status(), 499);
    let body = String::from_utf8_lossy(response.body());
    assert!(body.contains("unknown field `document_id`"), "got: {body}");
}

#[test]
fn a_body_that_is_no_object_is_refused_in_the_handler_it_is_given() {
    let request = IncomingRequest::new(
        "PUT".to_owned(),
        "/renamed/d1".to_owned(),
        String::new(),
        Vec::new(),
        b"[1]".to_vec(),
    );
    let response = poll_once(dispatch(&BoundBackEnd, &(), &request, &Noted)).unwrap();
    assert_eq!(response.status(), 499, "{:?}", response.headers());
}

#[test]
fn the_route_table_names_each_placeholder_as_the_path_writes_it() {
    let renamed = ROUTES
        .iter()
        .find(|route: &&Route| route.operation() == "renamed")
        .unwrap();
    assert_eq!(renamed.method(), "PUT");
    assert_eq!(renamed.path(), "/renamed/{document_id}");
    assert_eq!(renamed.ok_status(), 200);
    assert_eq!(renamed.error_statuses(), &[409]);
}
