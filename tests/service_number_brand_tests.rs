//! A brand, a struct of one slot and an alias over a number, declared above its service, are read
//! from the text of the number in a path, a query, a header and a text part, as the number they
//! wrap is read, and the `http_rest` client writes them as that text. A brand over a `String` is
//! still read as text.

#![cfg(feature = "serde")]

#[cfg(test)]
#[macro_use]
#[path = "service_number_brand_tests/service.rs"]
mod service;

#[cfg(test)]
#[path = "service_number_brand_tests/http_rest_transport.rs"]
mod http_rest_transport;

#[cfg(test)]
#[path = "service_number_brand_tests/http_rest_client.rs"]
mod http_rest_client;

#[cfg(test)]
#[macro_use]
#[path = "service_number_brand_tests/filing.rs"]
mod filing;

#[cfg(test)]
#[path = "service_number_brand_tests/filing_http_rest_transport.rs"]
mod filing_http_rest_transport;

// A transport reaches what the service declared through `$crate`, which is this binary's root.
#[cfg(test)]
use filing::{DockFilingService, dock_filing_service_schema};
#[cfg(test)]
use service::{DockService, dock_service_schema};
