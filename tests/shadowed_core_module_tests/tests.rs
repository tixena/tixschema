mod core;
mod std;

use ::core::future::{Future as _, ready};
use ::core::pin::pin;
use ::core::task::{Context as PollContext, Poll, Waker};
use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BalanceAnswer {
    pub credits: u32,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum BalanceFailure {
    DbError,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceRequest {
    #[model_schema_prop(minLength = 1)]
    pub note: String,
    pub organization_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

/// Answers a balance, and refuses organization zero.
pub struct BalanceBackEnd;

#[service_schema(transports = ["amqp_rpc", "http_rest", "ws_rpc"])]
pub trait BalanceService<Ctx> {
    #[service_schema_op(http(method = "POST", path = "/balance"))]
    async fn read_balance(
        &self,
        ctx: &Ctx,
        req: BalanceRequest,
    ) -> Result<BalanceAnswer, BalanceFailure>;
}

impl BalanceService<()> for BalanceBackEnd {
    async fn read_balance(
        &self,
        _ctx: &(),
        req: BalanceRequest,
    ) -> Result<BalanceAnswer, BalanceFailure> {
        ready(()).await;
        if req.organization_id == 0 {
            return Err(BalanceFailure::DbError);
        }
        Ok(BalanceAnswer {
            credits: req.organization_id,
        })
    }
}

/// Nothing above suspends, so one poll answers it.
fn balance_of(organization_id: u32) -> Poll<Result<BalanceAnswer, BalanceFailure>> {
    let req = BalanceRequest {
        note: "x".to_owned(),
        organization_id,
        tags: None,
    };
    let call = pin!(BalanceBackEnd.read_balance(&(), req));
    call.poll(&mut PollContext::from_waker(Waker::noop()))
}

#[test]
fn a_type_declared_beside_them_reads_and_validates() {
    let read: BalanceRequest =
        serde_json::from_value(serde_json::json!({ "note": "", "organizationId": 3_u32 })).unwrap();
    assert_eq!(read.validate().unwrap_err().len(), 1);
    assert_eq!(read.tags, None);
}

#[test]
fn the_modules_in_scope_are_the_ones_declared_here() {
    assert_eq!(core::answer(), 42);
    assert_eq!(std::answer(), 7);
}

#[test]
fn the_service_declared_beside_them_answers() {
    assert_eq!(balance_of(7), Poll::Ready(Ok(BalanceAnswer { credits: 7 })));
    assert_eq!(balance_of(0), Poll::Ready(Err(BalanceFailure::DbError)));
}
