//! Typed `chat.command` arguments and results in the retained `ChatWire` shape (phase one).
use super::{
    hub::{Hub, backend_of},
    ledger::{ComposerAttachment, SendRequest, SubmittedComposer},
};
use crate::{
    error::{ChatError, ErrorKind, Result},
    model::*,
    service::launch_env::Secret,
    wire,
};
use serde::{Deserialize, de::DeserializeOwned};

#[derive(Deserialize)]
struct Id {
    id: String,
}
#[derive(Deserialize)]
struct NewConversation {
    #[serde(default)]
    backend: Option<String>,
    #[serde(default)]
    id: Option<String>,
}
#[derive(Deserialize)]
struct Kind {
    kind: String,
}
#[derive(Deserialize)]
struct IdKind {
    id: String,
    kind: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Send {
    id: String,
    text: String,
    #[serde(default)]
    attachments: Vec<ComposerAttachment>,
    submitted: SubmittedComposer,
    #[serde(default)]
    settings: Option<TurnSettings>,
    #[serde(default)]
    mode: Option<String>,
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CancelQueued {
    id: String,
    client_message_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Respond {
    key: RequestKey,
    response: RequestResponse,
    process_epoch: String,
}
#[derive(Deserialize)]
struct Rename {
    id: String,
    title: String,
}
#[derive(Deserialize)]
struct Archive {
    id: String,
    archived: bool,
}
#[derive(Deserialize)]
struct Raw {
    id: String,
    method: String,
    #[serde(default)]
    params: Option<OpaqueJson>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Fork {
    id: String,
    #[serde(default)]
    at_turn_id: Option<String>,
}
#[derive(Deserialize)]
struct Preset {
    id: String,
    preset: String,
}
#[derive(Deserialize)]
struct Selection {
    id: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    effort: Option<String>,
}
#[derive(Deserialize)]
struct Login {
    kind: String,
    method: String,
    #[serde(default)]
    secret: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CancelLogin {
    kind: String,
    login_id: String,
}

fn args<T: DeserializeOwned>(args: &OpaqueObject) -> Result<T> {
    wire::decode(&wire::encode(args)).map_err(|e| ChatError::invalid(e.to_string()))
}
fn backend(value: &str) -> Result<BackendKind> {
    backend_of(&value.to_ascii_lowercase()).ok_or_else(|| ChatError::invalid("Unknown backend"))
}
/// Kotlin `Enum.valueOf(value.uppercase())` over the wire's uppercase enum names.
fn named<T: DeserializeOwned>(value: &str, what: &str) -> Result<T> {
    wire::decode(&wire::string_value(&value.to_ascii_uppercase()))
        .map_err(|_| ChatError::invalid(format!("Unknown {what} {value}")))
}
fn null() -> OpaqueJson {
    OpaqueJson::default()
}

pub(super) fn dispatch(hub: &Hub, name: &str, a: &OpaqueObject) -> Result<OpaqueJson> {
    Ok(match name {
        "newConversation" => {
            let p: NewConversation = args(a)?;
            let kind = p.backend.as_deref().map(backend).transpose()?;
            let created =
                hub.new_conversation(kind, p.id.unwrap_or_else(|| hub.ports.ids.new_id()))?;
            hub.refresh_tools(hub.entry(&created).map(|e| e.backend))?;
            wire::string_value(&created)
        }
        "ensureConversation" => {
            let p: Id = args(a)?;
            let entry = hub.ensure_conversation(&p.id)?;
            hub.refresh_tools(Some(entry.backend))?;
            wire::encode(&entry)
        }
        "open" => {
            let p: Id = args(a)?;
            let entry = hub.ensure_conversation(&p.id)?;
            hub.refresh_tools(Some(entry.backend))?;
            hub.open(&p.id)?;
            null()
        }
        "loadEarlier" => {
            hub.load_earlier(&args::<Id>(a)?.id)?;
            null()
        }
        "setBackend" => {
            let p: IdKind = args(a)?;
            let kind = backend(&p.kind)?;
            hub.set_backend(&p.id, kind)?;
            hub.refresh_tools(Some(kind))?;
            null()
        }
        "send" => {
            let p: Send = args(a)?;
            if p.submitted.conversation_id != p.id
                || p.submitted.text != p.text
                || p.submitted.attachments != p.attachments
            {
                return Err(ChatError::invalid(
                    "Submitted composer does not match the message",
                ));
            }
            let settings = p.settings.unwrap_or_default();
            let mode = match p.mode.as_deref() {
                Some(m) => named::<SendMode>(m, "send mode")?,
                None => SendMode::Auto,
            };
            let request = SendRequest {
                operation: p.operation_id,
                submitted: p.submitted,
                settings: settings.clone(),
                mode,
            };
            let accepted = hub.ledger.send(&request, || {
                hub.send(&p.id, &p.text, &p.attachments, &settings, mode)
            })?;
            hub.remember_selection(&p.id, settings.model, settings.effort)?;
            wire::string_value(&accepted)
        }
        "interrupt" => {
            hub.with_thread(&args::<Id>(a)?.id, |b, thread| b.interrupt(thread, false))?;
            null()
        }
        "cancelQueued" => {
            let p: CancelQueued = args(a)?;
            hub.with_thread(&p.id, |b, thread| {
                b.cancel_queued(thread, &p.client_message_id)
            })?;
            null()
        }
        "respond" => {
            let p: Respond = args(a)?;
            hub.journal.validate_response(&p.key, &p.process_epoch)?;
            hub.respond(&p.key, &p.response)?;
            null()
        }
        "rename" => {
            let p: Rename = args(a)?;
            hub.rename(&p.id, &p.title)?;
            null()
        }
        "archive" => {
            let p: Archive = args(a)?;
            hub.archive(&p.id, p.archived)?;
            null()
        }
        "deleteConversation" => {
            hub.delete_conversation(&args::<Id>(a)?.id)?;
            null()
        }
        "compact" => {
            hub.compact(&args::<Id>(a)?.id)?;
            null()
        }
        "refreshUsage" => {
            hub.connect(backend(&args::<Kind>(a)?.kind)?)?
                .refresh_rate_limits()?;
            null()
        }
        "refreshModels" => wire::encode(
            &hub.connect(backend(&args::<Kind>(a)?.kind)?)?
                .refresh_models()?,
        ),
        "rawRequest" => {
            let p: Raw = args(a)?;
            hub.raw_request(&p.id, &p.method, p.params.filter(|v| !wire::is_null(v)))?
        }
        "fork" => {
            let p: Fork = args(a)?;
            wire::string_value(&hub.fork(&p.id, p.at_turn_id.as_deref())?)
        }
        "setPermissions" => {
            let p: Preset = args(a)?;
            hub.set_permissions(&p.id, named(&p.preset, "permission preset")?)?;
            null()
        }
        "rememberSelection" => {
            let p: Selection = args(a)?;
            hub.remember_selection(&p.id, p.model, p.effort)?;
            null()
        }
        "login" => {
            let p: Login = args(a)?;
            let method: LoginMethod = named(&p.method, "login method")?;
            wire::encode(&hub.login(backend(&p.kind)?, method, p.secret.map(Secret))?)
        }
        "cancelLogin" => {
            let p: CancelLogin = args(a)?;
            hub.connect(backend(&p.kind)?)?.cancel_login(&p.login_id)?;
            null()
        }
        "logout" => {
            hub.connect(backend(&args::<Kind>(a)?.kind)?)?.logout()?;
            null()
        }
        "refreshAccount" => {
            hub.connect(backend(&args::<Kind>(a)?.kind)?)?
                .refresh_account()?;
            null()
        }
        "warmUp" => {
            let kind = backend(&args::<Kind>(a)?.kind)?;
            hub.refresh_tools(Some(kind))?;
            hub.connect(kind)?;
            null()
        }
        // Index writes are synchronous; nothing is buffered.
        "flush" => null(),
        other => {
            return Err(ChatError::new(
                ErrorKind::InvalidArgument,
                format!("Unknown chat command: {other}"),
            ));
        }
    })
}
