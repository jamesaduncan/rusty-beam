//! Rusty Beam v2 - Plugin-based HTTP server architecture
//! 
//! This module contains the next-generation architecture for rusty-beam,
//! featuring a unified plugin system where all request processing is handled
//! through configurable plugin pipelines.

pub mod plugin;
pub mod pipeline;
pub mod config;
pub mod examples;
pub mod demo;
pub mod loader;
pub mod wasm;
pub mod config_loader;
pub mod dynamic;

pub use plugin::{Plugin, PluginRequest, PluginContext};
pub use pipeline::{Pipeline, NestedPipeline, PipelineItem, PipelineResult};