use core::future::{Future as _, ready};
use core::pin::pin;
use core::task::{Context as PollContext, Poll, Waker};
use serde::{Deserialize, Serialize};
use tixschema::{model_schema, service_schema};

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ShadowAnswer {
    pub credits: u32,
}

#[model_schema()]
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case", tag = "errorCode")]
pub enum ShadowFailure {
    DbError,
}

#[model_schema()]
#[derive(Debug, Deserialize, Serialize)]
pub struct ShadowRequest {
    pub organization_id: u32,
}

/// Answers a balance, and refuses organization zero.
pub struct ShadowBackEnd;

pub struct Box;
pub struct Clone;
pub struct Default;
pub struct Err;
pub struct None;
pub struct Ok;
pub struct Option;
pub struct Send;
pub struct Sized;
pub struct Some;
pub struct String;
pub struct Sync;
pub struct Vec;

#[service_schema(transports = ["amqp_rpc", "http_rest", "ws_rpc"])]
pub trait ShadowService<Ctx> {
    #[service_schema_op(http(method = "POST", path = "/balance"))]
    async fn read_balance(
        &self,
        ctx: &Ctx,
        req: ShadowRequest,
    ) -> Result<ShadowAnswer, ShadowFailure>;
}

impl ShadowService<()> for ShadowBackEnd {
    async fn read_balance(
        &self,
        _ctx: &(),
        req: ShadowRequest,
    ) -> Result<ShadowAnswer, ShadowFailure> {
        ready(()).await;
        if req.organization_id == 0 {
            return Result::Err(ShadowFailure::DbError);
        }
        Result::Ok(ShadowAnswer {
            credits: req.organization_id,
        })
    }
}

/// Nothing above suspends, so one poll answers it.
fn balance_of(organization_id: u32) -> Poll<Result<ShadowAnswer, ShadowFailure>> {
    let call = pin!(ShadowBackEnd.read_balance(&(), ShadowRequest { organization_id }));
    call.poll(&mut PollContext::from_waker(Waker::noop()))
}

#[test]
fn the_service_declared_beside_them_answers() {
    assert_eq!(
        balance_of(7),
        Poll::Ready(Result::Ok(ShadowAnswer { credits: 7 }))
    );
    assert_eq!(
        balance_of(0),
        Poll::Ready(Result::Err(ShadowFailure::DbError))
    );
}

#[test]
fn the_names_in_scope_are_the_ones_declared_here() {
    let declared = (
        Box, Clone, Default, Err, None, Ok, Option, Send, Sized, Some, String, Sync, Vec,
    );
    assert_eq!(size_of_val(&declared), 0);
}
