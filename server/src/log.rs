use serde::{Deserialize, Serialize};

use crate::models::Term;

pub type LogIndex = u128;

// TODO: add more types of log entries like config changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogEntryContent {
    Null,
    Kv(KvChange),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KvChange {
    Set(String, String),
    Delete(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub term: Term,
    pub content: LogEntryContent,
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
            content: LogEntryContent::Null,
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
        self.entries.len() as LogIndex - 1
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

    // TODO - this feels really leaky?
    pub fn kv_get(&self, key: String) -> Option<String> {
        self.entries
            .iter()
            .rev()
            .find_map(|entry| match &entry.content {
                LogEntryContent::Kv(change) => match change {
                    KvChange::Set(entry_key, value) if *entry_key == key => Some(value.clone()),
                    KvChange::Delete(entry_key) if *entry_key == key => None,
                    _ => None,
                },
                _ => None,
            })
    }
}
