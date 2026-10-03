//! Protocol document operations over the Environment-owned atomic store.
use crate::protocol::{DocumentParams, DocumentResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use workflow_environment::{Error, Result};
use workflow_environment::{Store, json::OpaqueObject, persist, store::keys};
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
struct DocumentRecord {
    format: u32,
    revision: u64,
    document: Option<String>,
    #[serde(flatten)]
    extra: OpaqueObject,
}
impl Default for DocumentRecord {
    fn default() -> Self {
        Self {
            format: 1,
            revision: 0,
            document: None,
            extra: Default::default(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
struct Sidecar {
    format: u32,
    revision: u64,
    sha256: Option<String>,
    #[serde(flatten)]
    extra: OpaqueObject,
}
impl Default for Sidecar {
    fn default() -> Self {
        Self {
            format: 1,
            revision: 0,
            sha256: None,
            extra: Default::default(),
        }
    }
}
#[derive(Clone, Copy)]
pub enum Operation {
    Read,
    Write,
    Quarantine,
}
pub struct Change {
    pub document: DocumentResult,
    pub changed: bool,
}
fn next(revision: u64) -> Result<u64> {
    revision
        .checked_add(1)
        .ok_or_else(|| Error::business("overflow", "document revision overflow"))
}
fn expected(expected: Option<u64>, revision: u64) -> Result<()> {
    if expected.is_some_and(|v| v != revision) {
        Err(Error::business("conflict", "document revision changed"))
    } else {
        Ok(())
    }
}
fn text(a: &DocumentParams) -> Result<&str> {
    let text = a
        .document
        .as_deref()
        .ok_or_else(|| Error::invalid("document must be a string"))?;
    if text.len() > 16 * 1024 * 1024 {
        return Err(Error::invalid("document exceeds 16 MiB"));
    }
    Ok(text)
}
pub fn call(store: &Store, op: Operation, a: &DocumentParams, writable: bool) -> Result<Change> {
    if let Some(service) = a.namespace.strip_prefix("services.") {
        return service_document(store, op, service, a, writable);
    }
    let key = keys::document(&a.namespace, &a.key)?;
    if matches!(op, Operation::Quarantine) {
        store.quarantine(&key)?;
        return Ok(Change {
            document: DocumentResult {
                document: None,
                revision: 0,
            },
            changed: true,
        });
    }
    let mut record: DocumentRecord = store.read(&key)?.unwrap_or_default();
    if record.format != 1 {
        return Err(Error::business(
            "unsupported_format",
            "invalid opaque document envelope",
        ));
    }
    if matches!(op, Operation::Read) {
        return Ok(Change {
            document: DocumentResult {
                document: record.document,
                revision: record.revision,
            },
            changed: false,
        });
    }
    expected(a.expected_revision, record.revision)?;
    record.document = Some(text(a)?.into());
    record.revision = next(record.revision)?;
    store.write(&key, &record)?;
    Ok(Change {
        document: DocumentResult {
            document: record.document,
            revision: record.revision,
        },
        changed: true,
    })
}
fn service_document(
    store: &Store,
    op: Operation,
    service: &str,
    a: &DocumentParams,
    writable: bool,
) -> Result<Change> {
    let key = keys::service(service, &a.key)?;
    let sidecar_key = keys::service_sidecar(service, &a.key)?;
    let mut document = store.read_text(&key)?;
    let hash = document.as_ref().map(|d| persist::hash(d.as_bytes()));
    let mut meta: Sidecar = store.read(&sidecar_key)?.unwrap_or_default();
    if meta.format != 1 {
        return Err(Error::business(
            "unsupported_format",
            "invalid service document sidecar",
        ));
    }
    let mut changed = false;
    if meta.sha256 != hash {
        meta.revision = next(meta.revision)?;
        meta.sha256 = hash;
        if writable {
            store.write(&sidecar_key, &meta)?;
            changed = true;
        }
    }
    if matches!(op, Operation::Read) {
        return Ok(Change {
            document: DocumentResult {
                document,
                revision: meta.revision,
            },
            changed,
        });
    }
    expected(a.expected_revision, meta.revision)?;
    if matches!(op, Operation::Quarantine) {
        store.quarantine(&key)?;
        meta.sha256 = None;
        document = None;
    } else {
        let value = text(a)?;
        store.write_text(&key, value)?;
        meta.sha256 = Some(persist::hash(value.as_bytes()));
        document = Some(value.into());
    }
    meta.revision = next(meta.revision)?;
    store.write(&sidecar_key, &meta)?;
    Ok(Change {
        document: DocumentResult {
            document,
            revision: meta.revision,
        },
        changed: true,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_service_edit_invalidates_revision_and_unknown_sidecar_fields_survive() {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::open(temp.path()).unwrap();
        let mut a = DocumentParams {
            namespace: "services.proxy".into(),
            key: "config.yaml".into(),
            document: Some("mode: direct\n".into()),
            expected_revision: Some(0),
            extra: Default::default(),
        };
        let first = call(&store, Operation::Write, &a, true).unwrap();
        assert_eq!(first.document.revision, 1);
        store
            .write_text(
                &keys::service("proxy", "config.yaml").unwrap(),
                "mode: rule\n",
            )
            .unwrap();
        a.expected_revision = Some(1);
        assert_eq!(
            call(&store, Operation::Write, &a, true).err().unwrap().kind,
            "conflict"
        );
        let now = call(&store, Operation::Read, &a, true).unwrap();
        assert_eq!(now.document.revision, 2);
        assert_eq!(now.document.document.as_deref(), Some("mode: rule\n"));
    }
}
