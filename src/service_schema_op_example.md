```rust
use axum::http::{Request, Response};
use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

#[model_schema()]
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub order_id: String,
    pub total_cents: u32,
}

#[model_schema()]
#[derive(Deserialize, Serialize)]
#[serde(tag = "errorCode", rename_all = "kebab-case")]
pub enum OrderError {
    NotFound,
    AlreadyShipped,
}

#[service_schema(transports = ["http_rest"])]
pub trait Orders<Ctx> {
    /// `include_lines` is no path placeholder, so a `GET` reads it from the query string.
    #[service_schema_op(http(
        method = "GET",
        path = "/orders/{order_id}",
        error_status(NotFound = 404, AlreadyShipped = 409),
    ))]
    async fn get_order(
        &self,
        ctx: &Ctx,
        order_id: String,
        include_lines: Option<bool>,
    ) -> Result<Order, OrderError>;

    /// Named `order-cancellation` on the wire, and bound to the default route.
    #[service_schema_op(message = "order-cancellation")]
    async fn cancel_order(
        &self,
        ctx: &Ctx,
        order_id: String,
        reason: String,
    ) -> Result<Order, OrderError>;

    /// No reply, so no return type.
    #[service_schema_op(one_way)]
    async fn archive_order(&self, ctx: &Ctx, order_id: String, archived_by: String);
}

// `error_status` matches `OrderError` by variant, so the dispatcher's module imports it.
mod orders_http {
    use super::OrderError;

    orders_http_rest_dispatcher!();
}

pub struct Shop;

impl Orders<()> for Shop {
    async fn get_order(
        &self,
        _ctx: &(),
        order_id: String,
        _include_lines: Option<bool>,
    ) -> Result<Order, OrderError> {
        if order_id == "A-1" {
            Ok(Order { order_id, total_cents: 4250 })
        } else {
            Err(OrderError::NotFound)
        }
    }

    async fn cancel_order(
        &self,
        _ctx: &(),
        _order_id: String,
        _reason: String,
    ) -> Result<Order, OrderError> {
        Err(OrderError::AlreadyShipped)
    }

    async fn archive_order(&self, _ctx: &(), _order_id: String, _archived_by: String) {}
}

/// The route an operation answers to, found by its name on the wire.
fn route(operation: &str) -> &'static orders_http::Route {
    orders_http::ROUTES
        .iter()
        .find(|route| route.operation() == operation)
        .unwrap()
}

/// One HTTP handler: the request as an `IncomingRequest`, and the `OutgoingResponse` as the
/// response.
async fn handle(request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let (parts, body) = request.into_parts();
    let headers = parts
        .headers
        .iter()
        .filter_map(|(name, value)| {
            Some((name.as_str().to_owned(), value.to_str().ok()?.to_owned()))
        })
        .collect();
    let incoming = orders_http::IncomingRequest::new(
        parts.method.as_str().to_owned(),
        parts.uri.path().to_owned(),
        parts.uri.query().unwrap_or_default().to_owned(),
        headers,
        body,
    );
    let answered =
        orders_http::dispatch(&Shop, &(), &incoming, &orders_http::DefaultFaultHandler).await;
    let mut response = Response::builder().status(answered.status());
    for (name, value) in answered.headers() {
        response = response.header(name.as_str(), value.as_str());
    }
    response.body(answered.body().to_vec()).unwrap()
}

/// Runs a request that never waits, as every request here is: one poll finishes it.
fn run<F: std::future::Future>(call: F) -> F::Output {
    use std::future::Future;
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    let mut call = pin!(call);
    match call.as_mut().poll(&mut Context::from_waker(Waker::noop())) {
        Poll::Ready(answered) => answered,
        Poll::Pending => panic!("the request waited"),
    }
}

fn main() {
    let get = route("get-order");
    assert_eq!((get.method(), get.path(), get.ok_status()), ("GET", "/orders/{order_id}", 200));
    assert_eq!(get.error_statuses(), [404, 409]);

    let cancel = route("order-cancellation");
    assert_eq!(
        (cancel.method(), cancel.path(), cancel.ok_status()),
        ("POST", "/order-cancellation", 200)
    );
    assert_eq!(cancel.error_statuses(), [422]);

    let archive = route("archive-order");
    assert_eq!(
        (archive.method(), archive.path(), archive.ok_status()),
        ("POST", "/archive-order", 204)
    );
    assert!(archive.error_statuses().is_empty());

    let found = run(handle(Request::get("/orders/A-1").body(Vec::new()).unwrap()));
    assert_eq!(found.status().as_u16(), 200);
    assert_eq!(found.body().as_slice(), br#"{"orderId":"A-1","totalCents":4250}"#);

    let missing = run(handle(Request::get("/orders/B-7").body(Vec::new()).unwrap()));
    assert_eq!(missing.status().as_u16(), 404);
    assert_eq!(missing.body().as_slice(), br#"{"errorCode":"not-found"}"#);
}
```
