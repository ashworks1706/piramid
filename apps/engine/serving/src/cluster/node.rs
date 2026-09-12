//! Node identity, capabilities and runtime state.

use std::fmt::{Display, Formatter};

/// Name of a node in the cluster. The default is local.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(String);

impl NodeId {
    /// Node id with the given name.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The node name.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self("local".to_string())
    }
}

impl Display for NodeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Hardware a node offers.
#[derive(Debug, Clone)]
pub struct NodeCapabilities {
    /// Worker threads of the node. None means one per core.
    pub cpu_threads: Option<usize>,
    /// Host memory budget of the node, in bytes. None when not configured.
    pub memory_budget_bytes: Option<u64>,
    /// Whether the node has GPU compute enabled.
    pub gpu_enabled: bool,
}

impl Default for NodeCapabilities {
    fn default() -> Self {
        Self {
            cpu_threads: Some(num_cpus::get()),
            memory_budget_bytes: None,
            gpu_enabled: false,
        }
    }
}

/// Identity, capabilities and health of one node.
#[derive(Debug, Clone)]
pub struct NodeRuntimeState {
    /// Node name.
    pub id: NodeId,
    /// Hardware the node offers.
    pub capabilities: NodeCapabilities,
    /// Whether the node reports itself healthy.
    pub healthy: bool,
}

impl Default for NodeRuntimeState {
    fn default() -> Self {
        Self {
            id: NodeId::default(),
            capabilities: NodeCapabilities::default(),
            healthy: true,
        }
    }
}
