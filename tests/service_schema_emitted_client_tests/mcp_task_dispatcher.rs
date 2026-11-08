//! The `JobService` dispatcher, expanded out of the transport's macro into a module this
//! harness names.

mod streaming {
    use super::*;

    streams_here!();
}

job_service_mcp_dispatcher!();
