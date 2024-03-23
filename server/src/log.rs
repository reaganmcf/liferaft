use serde::{Deserialize, Serialize};

use crate::models::Term;

pub type LogIndex = u128;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub term: Term,
    pub key: String,
    pub value: String,
}

// The first entry in the log is at index 1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
    pub entries: Vec<LogEntry>,
}

impl Log {
    pub fn new() -> Self {
        let null_entry = LogEntry {
            term: 0,
            key: String::from(""),
            value: "".to_string(),
        };

        Log {
            entries: vec![null_entry],
        }
    }

    pub fn append(&mut self, entry: LogEntry) {
        self.entries.push(entry);
    }

    pub fn last_term(&self) -> Term {
        self.entries.last().map(|entry| entry.term).unwrap_or(0)
    }

    pub fn last_index(&self) -> LogIndex {
        self.entries.len() as LogIndex
    }

    pub fn get(&self, index: LogIndex) -> Option<&LogEntry> {
        self.entries.get(index as usize)
    }

    pub fn truncate(&mut self, index: LogIndex) {
        self.entries.truncate(index as usize);
    }

    pub fn len(&self) -> usize {
        // Subtract 1 to account for the NULL_ENTRY
        self.entries.len() - 1
    }

    pub fn last(&self) -> Option<&LogEntry> {
        self.entries.last()
    }
}