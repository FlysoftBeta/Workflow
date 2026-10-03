mod pairs;
mod types;
mod wire;
pub use pairs::Pairs;
pub use types::*;
pub use wire::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SendMode {
    #[default]
    Auto,
    Start,
    Steer,
    Queue,
}
