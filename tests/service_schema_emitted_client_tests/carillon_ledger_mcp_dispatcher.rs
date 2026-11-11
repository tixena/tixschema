//! The `mcp` dispatcher for `CarillonLedgerMcpService`, expanded out of the transport's macro into
//! a module of its own: the Rust twin the emitted TypeScript server is measured against.

mod streaming {
    use super::*;

    streams_here!();
}

carillon_ledger_mcp_service_mcp_dispatcher!();
