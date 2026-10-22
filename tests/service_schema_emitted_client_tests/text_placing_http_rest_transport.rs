//! The `http_rest` dispatcher for `TextPlacingClientService`, expanded out of the transport's macro
//! into a module of its own: the Rust twin that reads each request a client writes a value as text
//! in.

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};

use crate::wire_forms_models::ConveyedSlot;

text_placing_client_service_http_rest_dispatcher!();
