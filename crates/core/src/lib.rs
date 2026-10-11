//! rm_core: every rule of Raiders of the Lost Mart.
//!
//! Builder requirement: this crate performs no input or output. It takes
//! values and returns values, so `cargo test` reaches every rule without a
//! browser. `tests/boundary.rs` enforces that, along with the dependency
//! allowlist and the reference ban on floats, hash maps and clocks.
#![allow(
    clippy::needless_range_loop,
    clippy::collapsible_match,
    clippy::too_many_arguments
)]
#![recursion_limit = "256"]

pub mod ai;
pub mod balance;
pub mod content;
pub mod draw;
pub mod drone;
pub mod fx;
pub mod game;
pub mod geom;
pub mod gun;
pub mod kit;
pub mod mall;
pub mod mesh;
pub mod online_code;
pub mod raider;
pub mod render;
pub mod rng;
pub mod rollback;
pub mod run;
pub mod world;
