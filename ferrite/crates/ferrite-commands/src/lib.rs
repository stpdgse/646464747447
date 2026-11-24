//! Commands the way the official server structures them.
//!
//! * [`tree`]: the command tree of the `commands` packet: literals, arguments with their parsers,
//!   permissions. The trees for every permission level come from the official server (see
//!   `ferrite-datagen`), so the tree Ferrite sends is vanilla's byte for byte.

#![forbid(unsafe_code)]

pub mod args;
pub mod blockstate;
pub mod components;
pub mod coordinates;
pub mod dispatch;
pub mod identifier;
pub mod item;
pub mod nbt;
pub mod range;
pub mod reader;
pub mod selector;
pub mod tree;

pub use reader::{Message, Reader, SyntaxError};
pub use tree::{Node, NodeKind, Parser, Tree, TreeError};
