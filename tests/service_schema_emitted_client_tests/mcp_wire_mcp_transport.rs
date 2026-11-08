//! Expanded out of the transport's macro into a module this harness names.

mod streaming {
    use super::*;

    streams_here!();
}

shelf_wire_service_mcp_dispatcher!();
