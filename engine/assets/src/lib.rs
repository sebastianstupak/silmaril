//! Asset data structures and loaders
//!
//! Pure data structures for game assets (meshes, textures, materials).
//! No rendering or GPU dependencies - can be used by server, tools, or client.

#![warn(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(clippy::print_stdout)]
#![warn(clippy::print_stderr)]

pub mod mesh;

pub use mesh::{MeshData, Vertex};
