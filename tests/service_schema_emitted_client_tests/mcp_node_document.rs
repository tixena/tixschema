//! The `DocumentService` dispatcher, expanded out of the transport's macro into a module this
//! harness names.

mod streaming {
    use super::*;

    streams_here!();
}

document_service_mcp_dispatcher!();
