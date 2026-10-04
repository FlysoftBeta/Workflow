//! Durable dispatch gate keyed by composer revision; transport UUIDs are correlation only.
use crate::{
    error::{ChatError, ErrorKind, Result},
    model::{OpaqueObject, SendMode, TurnSettings},
    ports::{ChatStore, WorkspaceBridge},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposerAttachment {
    pub path: String,
    #[serde(default)]
    pub mime_type: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmittedComposer {
    #[serde(default = "format_one")]
    pub format: u64,
    pub conversation_id: String,
    pub revision: u64,
    pub text: String,
    pub attachments: Vec<ComposerAttachment>,
    #[serde(default, flatten)]
    pub extra: OpaqueObject,
}
fn format_one() -> u64 {
    1
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SendRequest {
    pub operation: String,
    pub submitted: SubmittedComposer,
    pub settings: TurnSettings,
    pub mode: SendMode,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SendState {
    Pending,
    Accepted,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendRecord {
    pub fingerprint: String,
    pub state: SendState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_message_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<u64>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Identity<'a> {
    conversation_id: &'a str,
    revision: u64,
}
#[derive(Serialize)]
struct Fingerprint<'a> {
    submitted: &'a SubmittedComposer,
    settings: &'a TurnSettings,
    mode: SendMode,
}
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub fn send_key(submitted: &SubmittedComposer) -> String {
    format!(
        "send-{}",
        hash(
            &serde_json::to_vec(&Identity {
                conversation_id: &submitted.conversation_id,
                revision: submitted.revision
            })
            .unwrap()
        )
    )
}
fn fingerprint(request: &SendRequest) -> Result<String> {
    let encoded = serde_json::to_vec(&Fingerprint {
        submitted: &request.submitted,
        settings: &request.settings,
        mode: request.mode,
    })?;
    // The named opaque tree canonicalizes object keys without rounding numeric extensions.
    let canonical: crate::model::OpaqueJson = workflow_environment::json::strict_json(&encoded)?;
    Ok(hash(&serde_json::to_vec(&canonical)?))
}
pub struct SendLedger {
    store: Arc<dyn ChatStore>,
    workspace: Arc<dyn WorkspaceBridge>,
    locks: [Mutex<()>; 128],
}
impl SendLedger {
    pub fn new(store: Arc<dyn ChatStore>, workspace: Arc<dyn WorkspaceBridge>) -> Self {
        Self {
            store,
            workspace,
            locks: std::array::from_fn(|_| Mutex::new(())),
        }
    }
    pub fn send(
        &self,
        request: &SendRequest,
        dispatch: impl FnOnce() -> Result<String>,
    ) -> Result<String> {
        if request.operation.trim().is_empty()
            || request.operation.encode_utf16().count() > 200
            || request.submitted.format != 1
            || request.submitted.conversation_id.trim().is_empty()
        {
            return Err(ChatError::new(
                ErrorKind::InvalidArgument,
                "A bounded operation ID and valid submitted composer are required",
            ));
        }
        let key = send_key(&request.submitted);
        let slot = key
            .bytes()
            .fold(0usize, |a, b| a.wrapping_mul(31).wrapping_add(b as usize))
            % self.locks.len();
        let _guard = self.locks[slot].lock().unwrap();
        let fingerprint = fingerprint(request)?;
        if let Some(prior) = self.store.read_send(&key)? {
            if prior.fingerprint != fingerprint {
                return Err(ChatError::new(
                    ErrorKind::SendConflict,
                    "Composer revision was already submitted with different content or settings",
                ));
            }
            if prior.state != SendState::Accepted || prior.client_message_id.is_none() {
                return Err(ChatError::new(
                    ErrorKind::SendAmbiguous,
                    "Submission outcome is ambiguous; inspect the conversation before sending a new message",
                ));
            }
            self.workspace.acknowledge_composer(&request.submitted)?;
            return Ok(prior.client_message_id.unwrap());
        }
        let mut record = SendRecord {
            fingerprint,
            state: SendState::Pending,
            client_message_id: None,
            conversation_id: Some(request.submitted.conversation_id.clone()),
            revision: Some(request.submitted.revision),
        };
        self.store.write_send(&key, &record)?;
        // A failure after this point must never remove the durable pending intent.
        let accepted = dispatch()?;
        record.state = SendState::Accepted;
        record.client_message_id = Some(accepted.clone());
        self.store.write_send(&key, &record)?;
        self.workspace.acknowledge_composer(&request.submitted)?;
        Ok(accepted)
    }
}
