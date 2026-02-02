//! Binary patching using bsdiff algorithm for differential updates.

pub mod apply;
pub mod diff;

pub use apply::apply_patch;
pub use diff::create_patch;
