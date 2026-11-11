//! The `http_rest` client for `RenamedPathBoundService`, in a module of its own.

use crate::renamed_path_bound_service::{
    BoundAliased, BoundAnswer, BoundBesideFlatten, BoundGeneric, BoundKeyword, BoundOwnKey,
    BoundPlain, BoundRefused, BoundRenamed, BoundTagged, BoundTyped, BoundUnlisted,
};

renamed_path_bound_service_http_rest_client!();
