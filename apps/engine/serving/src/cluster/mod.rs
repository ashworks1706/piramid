//! Node identity and the routing of collections onto nodes. Every collection routes locally.

mod node;
mod routing;

pub use node::{NodeCapabilities, NodeId, NodeRuntimeState};
pub use routing::{ClusterRouter, LocalClusterRouter, RouteDecision};
