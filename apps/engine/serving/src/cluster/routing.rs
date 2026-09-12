//! Placement of collections onto nodes.

use crate::cluster::{NodeId, NodeRuntimeState};

/// Where a collection is served.
#[derive(Debug, Clone)]
pub enum RouteDecision {
    /// This process serves the collection.
    Local,
    /// The named node serves the collection.
    Remote(NodeId),
}

/// Assigns collections to nodes.
pub trait ClusterRouter: Send + Sync {
    /// State of the node this process runs as.
    fn local_node(&self) -> NodeRuntimeState;
    /// Node that serves the named collection.
    fn route_collection(&self, collection: &str) -> RouteDecision;
}

/// Router for a single-node deployment; every collection routes locally.
#[derive(Debug, Clone, Default)]
pub struct LocalClusterRouter {
    local: NodeRuntimeState,
}

impl LocalClusterRouter {
    /// Router whose local node has the given state.
    pub fn new(local: NodeRuntimeState) -> Self {
        Self { local }
    }
}

impl ClusterRouter for LocalClusterRouter {
    fn local_node(&self) -> NodeRuntimeState {
        self.local.clone()
    }

    fn route_collection(&self, _collection: &str) -> RouteDecision {
        RouteDecision::Local
    }
}
