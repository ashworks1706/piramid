use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::metadata::MetadataValue;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum WalEntry {
    Insert {
        id: Uuid,
        vector: Vec<f32>,
        text: String,
        metadata: HashMap<String, MetadataValue>,
        seq: u64,
    },
    Update {
        id: Uuid,
        vector: Vec<f32>,
        text: String,
        metadata: HashMap<String, MetadataValue>,
        seq: u64,
    },
    Delete {
        id: Uuid,
        seq: u64,
    },
    Checkpoint {
        timestamp: u64,
        seq: u64,
    },
}
