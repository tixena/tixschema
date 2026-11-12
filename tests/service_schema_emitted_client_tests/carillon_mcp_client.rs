//! The `mcp` client for `CarillonMcpService`, expanded out of the transport's macro into a module
//! of its own: the Rust twin the emitted clients are measured against.
//!
//! The `use` is what resolves the types the author declared: the macro spells them exactly as
//! they were written.

use super::carillon_mcp_service::CarillonVerse;
use super::carillon_service::{CarillonBell, CarillonChime, CarillonError};

carillon_mcp_service_mcp_client!();
