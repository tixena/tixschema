//! A multipart operation whose text parts are a brand over a `u32`, a brand over an `i64` and a
//! brand over a `String`, and whose path binds a brand over a `u32`.

use core::future::ready;
use std::io;

use tixschema::service_schema;

use crate::filing_http_rest_transport;
use crate::service::*;

#[service_schema(transports = ["http_rest"])]
pub trait DockFilingService<Ctx> {
    #[service_schema_op(http(
        method = "POST",
        path = "/filed/{slip}",
        body = "multipart",
        part("file" = attachment),
    ))]
    async fn file(
        &self,
        ctx: &Ctx,
        slip: DockSlipNumber,
        ticket: DockTicketNumber,
        label: DockSlipLabel,
        attachment: Box<dyn dock_filing_service_schema::BodySource + Send>,
    ) -> Result<String, DockError>;
}

pub struct FilingBackEnd;

impl DockFilingService<()> for FilingBackEnd {
    async fn file(
        &self,
        _ctx: &(),
        slip: DockSlipNumber,
        ticket: DockTicketNumber,
        label: DockSlipLabel,
        mut attachment: Box<dyn dock_filing_service_schema::BodySource + Send>,
    ) -> Result<String, DockError> {
        ready(()).await;
        let mut drained = Vec::new();
        let mut buf = [0_u8; 64];
        loop {
            let read = attachment.pull(&mut buf).unwrap();
            if read == 0 {
                break;
            }
            drained.extend_from_slice(&buf[..read]);
        }
        Ok(format!(
            "{} {} {} {}",
            slip.0,
            ticket.0,
            label.0,
            String::from_utf8(drained).unwrap()
        ))
    }
}

/// An owner's `FaultHandler`, which answers every fault with a status and a header of its own.
struct Refusing;

impl filing_http_rest_transport::FaultHandler for Refusing {
    fn on_fault(
        &self,
        fault: &dock_filing_service_schema::ServiceFault,
    ) -> filing_http_rest_transport::OutgoingResponse {
        filing_http_rest_transport::OutgoingResponse::new(
            499,
            vec![("x-fault".to_owned(), format!("{}", fault.kind()))],
            Vec::new(),
        )
    }
}

fn request(path: &str, ticket: &str, label: &str) -> String {
    let incoming = filing_http_rest_transport::IncomingRequest::new(
        "POST".to_owned(),
        path.to_owned(),
        String::new(),
        Vec::new(),
        Vec::new(),
    );
    let parts = vec![
        (
            "ticket".to_owned(),
            filing_http_rest_transport::IncomingPart::Text(ticket.to_owned()),
        ),
        (
            "label".to_owned(),
            filing_http_rest_transport::IncomingPart::Text(label.to_owned()),
        ),
        (
            "file".to_owned(),
            filing_http_rest_transport::IncomingPart::File(Box::new(io::Cursor::new(
                b"body".to_vec(),
            ))),
        ),
    ];
    let answered = poll_once(filing_http_rest_transport::dispatch(
        &FilingBackEnd,
        &(),
        &incoming,
        parts,
        &filing_http_rest_transport::DefaultFaultHandler,
    ))
    .unwrap();
    format!(
        "{} {}",
        answered.status(),
        String::from_utf8(answered.body().to_vec()).unwrap()
    )
}

/// The dispatcher reads a text part of a brand over a number from the text of the number, and a
/// brand over a `String` of digits as text.
#[test]
fn a_brand_over_a_number_in_a_text_part_is_read_from_the_text_of_the_number() {
    assert_eq!(
        request("/filed/7", "9223372036854775807", "007"),
        format!("200 \"7 {} 007 body\"", i64::MAX)
    );
    assert_eq!(request("/filed/7", "-5", "-5"), "200 \"7 -5 -5 body\"");
    assert!(request("/filed/7", "five", "x").starts_with("400 "));
}

/// The route table lists the one route, and a request reads back what it was built with.
#[test]
fn the_route_table_lists_the_route_and_a_request_reads_back_what_it_was_built_with() {
    let routes = filing_http_rest_transport::ROUTES;
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].operation(), "file");
    assert_eq!(routes[0].method(), "POST");
    assert_eq!(routes[0].path(), "/filed/{slip}");
    assert_eq!(routes[0].ok_status(), 200);
    assert_eq!(routes[0].error_statuses(), &[422]);
    let incoming = filing_http_rest_transport::IncomingRequest::new(
        "POST".to_owned(),
        "/filed/7".to_owned(),
        "q=1".to_owned(),
        vec![("x-trace".to_owned(), "t".to_owned())],
        b"b".to_vec(),
    );
    let refused = poll_once(filing_http_rest_transport::dispatch(
        &FilingBackEnd,
        &(),
        &incoming,
        Vec::new(),
        &Refusing,
    ))
    .unwrap();
    assert_eq!(
        (refused.status(), refused.headers()),
        (
            499,
            &[("x-fault".to_owned(), "failed validation".to_owned())][..]
        )
    );
    assert_eq!(
        (
            incoming.body(),
            incoming.query(),
            incoming.header("x-trace"),
            incoming.headers().len()
        ),
        (&b"b"[..], "q=1", Some("t"), 1)
    );
}
