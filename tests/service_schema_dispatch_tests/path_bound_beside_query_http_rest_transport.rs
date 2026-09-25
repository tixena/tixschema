//! The `http_rest` dispatcher for `PathBoundBesideQueryService`, expanded out of the transport's
//! macro into a module of its own. Unlike `solo_path_bound_http_rest_transport`, this service
//! does declare a query-reading operation, so `parse_query` is reachable here — `get_document`'s
//! own arm still must not bind it.

use crate::path_bound_beside_query_service::PathBoundError;

path_bound_beside_query_service_http_rest_dispatcher!();
