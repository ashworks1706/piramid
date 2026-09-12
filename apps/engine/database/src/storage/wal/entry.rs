//! The operations a write-ahead log records.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use piramid_core::metadata::MetadataValue;

/// One logged operation, replayed in sequence order when a collection opens.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum WalEntry {
    /// A new document.
    Insert {
        /// Document id.
        id: Uuid,
        /// Document vector.
        vector: Vec<f32>,
        /// Document text.
        text: String,
        /// Document metadata.
        metadata: HashMap<String, MetadataValue>,
        /// Sequence number assigned when the entry was logged.
        seq: u64,
    },
    /// A replacement of an existing document, replayed as a delete then an insert.
    Update {
        /// Id of the document replaced.
        id: Uuid,
        /// Replacement vector.
        vector: Vec<f32>,
        /// Replacement text.
        text: String,
        /// Replacement metadata.
        metadata: HashMap<String, MetadataValue>,
        /// Sequence number assigned when the entry was logged.
        seq: u64,
    },
    /// A removal of a document.
    Delete {
        /// Id of the document removed.
        id: Uuid,
        /// Sequence number assigned when the entry was logged.
        seq: u64,
    },
    /// A marker that sidecars were saved.
    Checkpoint {
        /// Time of the checkpoint in Unix seconds.
        timestamp: u64,
        /// Sequence number assigned when the entry was logged.
        seq: u64,
    },
}
