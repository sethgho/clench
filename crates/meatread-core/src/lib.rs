//! # meatread-core
//!
//! A portable, sans-I/O engine for RSVP (rapid serial visual presentation)
//! speed reading.
#![no_std]
#![warn(missing_docs)]

extern crate alloc;

mod document;
#[allow(dead_code)]
mod text;

pub use document::{Block, BlockKind, Document, Source};
