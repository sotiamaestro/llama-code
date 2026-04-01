// Copyright 2025 Llama Code Contributors
// SPDX-License-Identifier: Apache-2.0

//! Core agent loop and state machine for Llama Code.
//!
//! This crate contains the main agent loop, configuration, context window
//! management, model backend abstraction, permission system, and session
//! management.

pub mod agent;
pub mod config;
pub mod context;
pub mod errors;
pub mod events;
pub mod history;
pub mod model;
pub mod permissions;
pub mod router;
pub mod session;
