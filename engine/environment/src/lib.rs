//! Workspace persistence, environment lifecycle and the sole guest execution API.
//! Wire formats and client transport are owned by the Server.
pub mod config;
pub mod documents;
pub mod services;
pub mod error;
mod home_stage;
pub mod json;
mod lifecycle;
mod model;
pub mod persist;
pub mod runtime;
pub mod store;
pub mod tools;
pub use error::{Error, Result};
pub use lifecycle::{Environment, Options, validate};
pub use model::*;
pub use store::{Store, StoreLock, StoredMetadata, UploadStage};

/// Schemas are derived from the same models used to read and write documents.
pub fn schemas() -> std::collections::BTreeMap<String, schemars::Schema> {
    let mut schemas = std::collections::BTreeMap::from([
        (
            "EnvironmentSpec".into(),
            schemars::schema_for!(EnvironmentSpec),
        ),
        (
            "EnvironmentState".into(),
            schemars::schema_for!(EnvironmentState),
        ),
        (
            "EnvironmentStatus".into(),
            schemars::schema_for!(EnvironmentStatus),
        ),
        (
            "ImageIndex".into(),
            schemars::schema_for!(model::ImageIndex),
        ),
    ]);
    schemas.extend(tools::schemas());
    schemas.insert("HomeBaseline".into(), home_stage::schema());
    schemas
}
