//! Four services whose TypeScript surfaces call a different set of the module-private helpers each,
//! compiled with every other service of the group under `--noUnusedLocals`.

use super::super::{HeaderProbeDocument, HeaderProbeError, poll_once};
use core::future::ready;
use tixschema::service_schema;

/// No header binding and a JSON body.
#[service_schema(transports = [])]
pub trait PlainService<Ctx> {
    #[service_schema_op(http(method = "POST", path = "/read"))]
    async fn read(&self, ctx: &Ctx, id: String) -> Result<HeaderProbeDocument, HeaderProbeError>;
}

/// A `header_in` binding alone.
#[service_schema(transports = [])]
pub trait InboundService<Ctx> {
    #[service_schema_op(http(method = "POST", path = "/read", header_in("x-tenant" = tenant)))]
    async fn read(
        &self,
        ctx: &Ctx,
        id: String,
        tenant: String,
    ) -> Result<HeaderProbeDocument, HeaderProbeError>;
}

/// A `header_out` binding alone.
#[service_schema(transports = [])]
pub trait OutboundService<Ctx> {
    #[service_schema_op(http(method = "POST", path = "/read", header_out("etag")))]
    async fn read(
        &self,
        ctx: &Ctx,
        id: String,
    ) -> Result<(HeaderProbeDocument, String), HeaderProbeError>;
}

/// A `GET` with a path placeholder and no body.
#[service_schema(transports = [])]
pub trait BodilessService<Ctx> {
    #[service_schema_op(http(method = "GET", path = "/notes/{id}"))]
    async fn read(&self, ctx: &Ctx, id: String) -> Result<HeaderProbeDocument, HeaderProbeError>;
}

/// The implementation of each service above, which the macro needs to see the trait used.
pub struct NoteBackEnd;

impl PlainService<()> for NoteBackEnd {
    async fn read(&self, _ctx: &(), id: String) -> Result<HeaderProbeDocument, HeaderProbeError> {
        ready(()).await;
        Ok(HeaderProbeDocument { title: id })
    }
}

impl InboundService<()> for NoteBackEnd {
    async fn read(
        &self,
        _ctx: &(),
        id: String,
        _tenant: String,
    ) -> Result<HeaderProbeDocument, HeaderProbeError> {
        ready(()).await;
        Ok(HeaderProbeDocument { title: id })
    }
}

impl OutboundService<()> for NoteBackEnd {
    async fn read(
        &self,
        _ctx: &(),
        id: String,
    ) -> Result<(HeaderProbeDocument, String), HeaderProbeError> {
        ready(()).await;
        Ok((HeaderProbeDocument { title: id.clone() }, id))
    }
}

impl BodilessService<()> for NoteBackEnd {
    async fn read(&self, _ctx: &(), id: String) -> Result<HeaderProbeDocument, HeaderProbeError> {
        ready(()).await;
        Ok(HeaderProbeDocument { title: id })
    }
}

#[test]
fn the_four_services_are_implementable_and_callable() {
    let read = |answered: Result<HeaderProbeDocument, HeaderProbeError>| answered.unwrap().title;
    assert_eq!(
        read(poll_once(PlainService::read(&NoteBackEnd, &(), "a".to_owned())).unwrap()),
        "a"
    );
    assert_eq!(
        read(
            poll_once(InboundService::read(
                &NoteBackEnd,
                &(),
                "b".to_owned(),
                "t".to_owned()
            ))
            .unwrap()
        ),
        "b"
    );
    assert_eq!(
        poll_once(OutboundService::read(&NoteBackEnd, &(), "c".to_owned()))
            .unwrap()
            .unwrap()
            .1,
        "c"
    );
    assert_eq!(
        read(poll_once(BodilessService::read(&NoteBackEnd, &(), "d".to_owned())).unwrap()),
        "d"
    );
}
