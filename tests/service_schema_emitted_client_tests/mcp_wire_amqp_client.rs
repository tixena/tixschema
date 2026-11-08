//! Expanded out of the transport's macro into a module this harness names.

use crate::mcp_wire_service::{ShelfCount, ShelfWireError, ShelfWireRequest};
use crate::shelf_wire_service_schema;

shelf_wire_service_amqp_rpc_client!();
