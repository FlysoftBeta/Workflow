//! Rust chat domain. Guest execution and persistence use explicit Environment ports.
pub mod config;
pub mod model;

pub mod error;
pub mod journal;
pub mod ports;
pub mod reducer;
pub mod service;
