//! The `http_rest` client for `CarillonHttpService`, expanded out of the transport's macro into a
//! module of its own: the Rust twin the emitted clients are measured against.

use super::carillon_service::{CarillonChime, CarillonError};

carillon_http_service_http_rest_client!();
