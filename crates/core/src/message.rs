//! Translatable text produced by the core (ADR-0014): a key plus named parameters.
//! The shells (UI, CLI, MCP) turn it into a sentence in the user's language.

use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct Message {
    pub key: String,
    pub params: BTreeMap<String, String>,
}

impl Message {
    pub fn new(key: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            params: BTreeMap::new(),
        }
    }

    /// Adds a parameter; builder style.
    pub fn with(mut self, name: impl Into<String>, value: impl ToString) -> Self {
        self.params.insert(name.into(), value.to_string());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_key_and_named_parameters() {
        let message = Message::new("reason.image_unused")
            .with("tag", "node:18")
            .with("days", 90);
        assert_eq!(message.key, "reason.image_unused");
        assert_eq!(message.params["tag"], "node:18");
        assert_eq!(message.params["days"], "90");
    }
}
