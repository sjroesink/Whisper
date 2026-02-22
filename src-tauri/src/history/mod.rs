use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::providers::{ProviderId, TranscriptionResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionEntry {
    pub id: String,
    pub text: String,
    pub provider: ProviderId,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: u64,
    pub language: Option<String>,
}

pub struct TranscriptionHistory {
    entries: Vec<TranscriptionEntry>,
    max_entries: usize,
}

impl TranscriptionHistory {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries,
        }
    }

    pub fn add(&mut self, result: &TranscriptionResult) {
        let entry = TranscriptionEntry {
            id: Uuid::new_v4().to_string(),
            text: result.text.clone(),
            provider: result.provider.clone(),
            timestamp: Utc::now(),
            duration_ms: result.duration_ms,
            language: result.language.clone(),
        };
        self.entries.insert(0, entry);
        self.entries.truncate(self.max_entries);
    }

    pub fn get_all(&self) -> &[TranscriptionEntry] {
        &self.entries
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
