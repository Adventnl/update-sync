pub mod archive;
pub mod build;
pub mod data;
pub mod github;
pub mod manager;
pub mod model;
pub mod platform;
pub mod storage;
pub mod store;
#[cfg(windows)]
mod windows_trust;

pub use manager::Manager;
pub use model::*;

#[cfg(test)]
mod lifecycle_tests;
