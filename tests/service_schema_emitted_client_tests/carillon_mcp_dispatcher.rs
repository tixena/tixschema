//! The `mcp` dispatcher for `CarillonMcpService`, expanded out of the transport's macro into a
//! module of its own: the Rust twin the emitted servers are measured against.

mod streaming {
    use super::*;

    streams_here!();
}

carillon_mcp_service_mcp_dispatcher!();
