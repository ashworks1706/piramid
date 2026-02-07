use std::collections::HashMap;
use uuid::Uuid;
use serde::{Serialize, Deserialize};

use crate::metadata::MetadataValue;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum WalEntry{
    Insert { id : Uuid, vector : Vec<f32>, text: String, metadata: HashMap<String, MetadataValue> },
    Update { id: Uuid, vector : Vec<f32>, text: String, metadata: HashMap<String, MetadataValue> },
    Delete { id: Uuid },
    Checkpoint { timestamp: u64}
}
