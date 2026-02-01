//! SIMD-optimized math operations.
//!
//! Process multiple vectors simultaneously using CPU vector instructions.

mod util;
mod vec3x4;
mod vec3x8;

pub use util::{vec3_aos_to_soa_4, vec3_aos_to_soa_8, vec3_soa_to_aos_4, vec3_soa_to_aos_8};
pub use vec3x4::Vec3x4;
pub use vec3x8::Vec3x8;
