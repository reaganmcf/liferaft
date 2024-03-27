use std::path::PathBuf;

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
    pub path: PathBuf,
}

impl Log {
    pub fn new(path: PathBuf) -> Self {
        let entries = {
            if path.exists() {
                let serialized = std::fs::read_to_string(path.clone()).expect("Failed to read log");
                
                // serialize into vec
                if serialized.len() > 0 {
                    let entries: Vec<LogEntry> = serde_json::from_str(&serialized).unwrap();
                    entries
                } else {
                    let null_entry = LogEntry {
                        term: 0,
                        content: LogEntryContent::Null,
                    };
                    vec![null_entry]
                }
            } else {
                let null_entry = LogEntry {
                    term: 0,
                    content: LogEntryContent::Null,
                };

                vec![null_entry]
            }
        };

        let log = Log { entries, path };
        log.save().expect("Failed to save log");

        log
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

    pub fn save(&self) -> std::io::Result<()> {
        let serialized = serde_json::to_string(&self.entries)?;
        std::fs::write(&self.path, serialized)
    }
}
