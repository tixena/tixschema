//! The `http_rest` dispatcher for `SoloPathBoundService`, expanded out of the transport's macro
//! into a module of its own — the same placement every other transport macro in this harness
//! gets. This service declares no query-reading operation at all, so its dispatcher must never
//! reach for `parse_query`.

use crate::solo_path_bound_service::PathBoundError;

solo_path_bound_service_http_rest_dispatcher!();
