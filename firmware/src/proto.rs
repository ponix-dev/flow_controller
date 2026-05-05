//! Generated wire types from `proto/flow_controller/v1/*.proto`.
//!
//! Regenerate via `mise run proto:gen` after editing `.proto` files.
//! The generated file `proto/valve.rs` is committed; do not hand-edit.

#[allow(warnings, clippy::all)]
mod generated {
    include!("proto/valve.rs");
}

// Phase 3 wires these into the uplink/downlink path; until then the re-export
// has no in-tree caller.
#[allow(unused_imports)]
pub use generated::flow_controller_::v1_::*;
