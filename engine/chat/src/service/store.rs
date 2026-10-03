//! Typed Chat documents through Environment's atomic document store.
use crate::{
    error::Result,
    ports::ChatStore,
    service::{index::IndexDocument, ledger::SendRecord},
};
use workflow_environment::{
    Store,
    documents::{self, DocumentParams, Operation},
    store::keys,
};
pub struct EnvironmentChatStore(pub Store);
impl EnvironmentChatStore {
    fn read(&self, key: &str) -> Result<Option<String>> {
        Ok(documents::call(
            &self.0,
            Operation::Read,
            &DocumentParams {
                namespace: "chat".into(),
                key: key.into(),
                document: None,
                expected_revision: None,
                extra: Default::default(),
            },
            true,
        )?
        .document
        .document)
    }
    fn write<T: serde::Serialize>(&self, key: &str, body: &T) -> Result<()> {
        documents::call(
            &self.0,
            Operation::Write,
            &DocumentParams {
                namespace: "chat".into(),
                key: key.into(),
                document: Some(serde_json::to_string(body)?),
                expected_revision: None,
                extra: Default::default(),
            },
            true,
        )?;
        Ok(())
    }
}
impl ChatStore for EnvironmentChatStore {
    fn read_index(&self) -> Result<Option<IndexDocument>> {
        self.read("conversations")?
            .map(|s| IndexDocument::decode(s.as_bytes()))
            .transpose()
    }
    fn write_index(&self, document: &IndexDocument) -> Result<()> {
        self.write("conversations", document)
    }
    fn quarantine_index(&self) -> Result<()> {
        self.0
            .quarantine(&keys::document("chat", "conversations")?)?;
        Ok(())
    }
    fn read_send(&self, key: &str) -> Result<Option<SendRecord>> {
        self.read(key)?
            .map(|s| workflow_environment::json::strict_json(s.as_bytes()).map_err(Into::into))
            .transpose()
    }
    fn write_send(&self, key: &str, record: &SendRecord) -> Result<()> {
        self.write(key, record)
    }
    fn remove_sends(&self, conversation: &str) -> Result<()> {
        for path in self.0.list("documents/chat")? {
            let Some(name) = path
                .rsplit('/')
                .next()
                .and_then(|s| s.strip_suffix(".json"))
                .filter(|s| s.starts_with("send-"))
            else {
                continue;
            };
            if self
                .read_send(name)?
                .is_some_and(|r| r.conversation_id.as_deref() == Some(conversation))
            {
                self.0.remove(&path)?;
            }
        }
        Ok(())
    }
}
