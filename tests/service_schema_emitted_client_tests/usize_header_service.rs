//! A brand over a `usize` answered in a reply header, the one place an `http_rest` client reads a
//! brand from the text of a header.

#![cfg(any(feature = "swift", feature = "kotlin"))]

use core::future::ready;

use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

/// Each value the dispatcher writes in `x-quota`: the bottom of the range, `i64::MAX`, the value
/// above it, and `usize::MAX`.
pub const WRITTEN: [&str; 4] = [
    "0",
    "9223372036854775807",
    "9223372036854775808",
    "18446744073709551615",
];

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Quota(usize);

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct QuotaAsk {
    pub label: String,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum QuotaError {
    Missing,
}

#[service_schema(transports = ["http_rest"])]
pub trait QuotaClientService<Ctx> {
    #[service_schema_op(http(method = "POST", path = "/quota", header_out("x-quota")))]
    async fn ask(&self, ctx: &Ctx, req: QuotaAsk) -> Result<(QuotaAsk, Quota), QuotaError>;
}

pub struct QuotaBackEnd;

impl QuotaClientService<()> for QuotaBackEnd {
    async fn ask(&self, _ctx: &(), req: QuotaAsk) -> Result<(QuotaAsk, Quota), QuotaError> {
        ready(()).await;
        if req.label.is_empty() {
            return Err(QuotaError::Missing);
        }
        Ok((req, Quota::new(usize::MAX).unwrap()))
    }
}

#[test]
fn the_quota_backend_answers_the_widest_value() {
    let ask = |label: &str| QuotaAskDraft {
        label: label.to_owned(),
    };
    let answered =
        super::tests::poll_once(QuotaBackEnd.ask(&(), ask("a").validate().unwrap())).unwrap();
    assert_eq!(
        answered,
        Ok((
            ask("a").validate().unwrap(),
            Quota::new(usize::MAX).unwrap()
        ))
    );
    let refused =
        super::tests::poll_once(QuotaBackEnd.ask(&(), ask("").validate().unwrap())).unwrap();
    assert_eq!(refused, Err(QuotaError::Missing));
}
