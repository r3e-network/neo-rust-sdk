#![allow(warnings)]

//! # neo_builder::script
//!
//! This module provides utilities for building and reading Neo smart contract scripts.
//!
//! ## Modules
//!
//! ### `interop_service`
//!
//! Contains the [`InteropService`] enum, which represents various system calls available in the Neo virtual machine.
//!
//! ### `script_builder`
//!
//! Provides the [`ScriptBuilder`] struct for constructing Neo smart contract scripts programmatically.
//!
//! ### `script_reader`
//!
//! Offers the [`ScriptReader`] struct for parsing and interpreting Neo smart contract scripts.
//!
//! ## Usage
//!
//! To use the functionality provided by this module, you can import the necessary components:
//!
//! ```rust
//! use neo3::neo_builder::script::{InteropService, ScriptBuilder, ScriptReader};
//! ```
//!
//! [`InteropService`]: interop_service::InteropService
//! [`ScriptBuilder`]: script_builder::ScriptBuilder
//! [`ScriptReader`]: script_reader::ScriptReader

pub use interop_service::*;
pub use script_builder::*;
pub use script_reader::*;

mod interop_service;
mod script_builder;
mod script_reader;
