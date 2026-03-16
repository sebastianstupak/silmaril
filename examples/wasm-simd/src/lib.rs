//! WASM SIMD benchmark demo
//!
//! This module demonstrates SIMD performance in WebAssembly.
//! It exports functions that can be called from JavaScript to benchmark
//! scalar vs SIMD performance in the browser.

use wasm_bindgen::prelude::*;
use engine_math::{Vec3, simd::{vec3_aos_to_soa_4, vec3_aos_to_soa_8}};

/// Physics integration benchmark - scalar version
///
/// Integrates positions with velocities over multiple iterations.
/// Returns the total time in milliseconds.
#[wasm_bindgen]
pub fn bench_scalar(iterations: usize, entity_count: usize) -> f64 {
    let start = instant::now();

    let mut positions: Vec<Vec3> = (0..entity_count)
        .map(|i| Vec3::new(i as f32, i as f32, i as f32))
        .collect();

    let velocities: Vec<Vec3> = (0..entity_count)
        .map(|i| Vec3::new(i as f32 * 0.1, i as f32 * 0.1, i as f32 * 0.1))
        .collect();

    let dt = 0.016;

    for _ in 0..iterations {
        for i in 0..entity_count {
            positions[i] = positions[i] + velocities[i] * dt;
        }
    }

    // Prevent optimization
    let _ = positions;

    instant::now() - start
}

/// Physics integration benchmark - SIMD 4-wide version
///
/// Same as scalar but processes 4 entities at once using SIMD.
/// Returns the total time in milliseconds.
#[wasm_bindgen]
pub fn bench_simd_4wide(iterations: usize, entity_count: usize) -> f64 {
    let start = instant::now();

    // Ensure entity count is divisible by 4
    let entity_count = (entity_count / 4) * 4;

    let mut positions: Vec<Vec3> = (0..entity_count)
        .map(|i| Vec3::new(i as f32, i as f32, i as f32))
        .collect();

    let velocities: Vec<Vec3> = (0..entity_count)
        .map(|i| Vec3::new(i as f32 * 0.1, i as f32 * 0.1, i as f32 * 0.1))
        .collect();

    let dt = 0.016;

    for _ in 0..iterations {
        for chunk_idx in (0..entity_count).step_by(4) {
            // Convert AoS to SoA
            let pos_aos = [
                positions[chunk_idx],
                positions[chunk_idx + 1],
                positions[chunk_idx + 2],
                positions[chunk_idx + 3],
            ];
            let vel_aos = [
                velocities[chunk_idx],
                velocities[chunk_idx + 1],
                velocities[chunk_idx + 2],
                velocities[chunk_idx + 3],
            ];

            let pos_soa = vec3_aos_to_soa_4(&pos_aos);
            let vel_soa = vec3_aos_to_soa_4(&vel_aos);

            // SIMD operation
            let new_pos = pos_soa.mul_add(vel_soa, dt);

            // Convert back to AoS
            let result = new_pos.to_array();
            positions[chunk_idx] = result[0];
            positions[chunk_idx + 1] = result[1];
            positions[chunk_idx + 2] = result[2];
            positions[chunk_idx + 3] = result[3];
        }
    }

    // Prevent optimization
    let _ = positions;

    instant::now() - start
}

/// Physics integration benchmark - SIMD 8-wide version
///
/// Same as scalar but processes 8 entities at once using SIMD.
/// Returns the total time in milliseconds.
#[wasm_bindgen]
pub fn bench_simd_8wide(iterations: usize, entity_count: usize) -> f64 {
    let start = instant::now();

    // Ensure entity count is divisible by 8
    let entity_count = (entity_count / 8) * 8;

    let mut positions: Vec<Vec3> = (0..entity_count)
        .map(|i| Vec3::new(i as f32, i as f32, i as f32))
        .collect();

    let velocities: Vec<Vec3> = (0..entity_count)
        .map(|i| Vec3::new(i as f32 * 0.1, i as f32 * 0.1, i as f32 * 0.1))
        .collect();

    let dt = 0.016;

    for _ in 0..iterations {
        for chunk_idx in (0..entity_count).step_by(8) {
            // Convert AoS to SoA
            let pos_aos = [
                positions[chunk_idx],
                positions[chunk_idx + 1],
                positions[chunk_idx + 2],
                positions[chunk_idx + 3],
                positions[chunk_idx + 4],
                positions[chunk_idx + 5],
                positions[chunk_idx + 6],
                positions[chunk_idx + 7],
            ];
            let vel_aos = [
                velocities[chunk_idx],
                velocities[chunk_idx + 1],
                velocities[chunk_idx + 2],
                velocities[chunk_idx + 3],
                velocities[chunk_idx + 4],
                velocities[chunk_idx + 5],
                velocities[chunk_idx + 6],
                velocities[chunk_idx + 7],
            ];

            let pos_soa = vec3_aos_to_soa_8(&pos_aos);
            let vel_soa = vec3_aos_to_soa_8(&vel_aos);

            // SIMD operation
            let new_pos = pos_soa.mul_add(vel_soa, dt);

            // Convert back to AoS
            let result = new_pos.to_array();
            positions[chunk_idx] = result[0];
            positions[chunk_idx + 1] = result[1];
            positions[chunk_idx + 2] = result[2];
            positions[chunk_idx + 3] = result[3];
            positions[chunk_idx + 4] = result[4];
            positions[chunk_idx + 5] = result[5];
            positions[chunk_idx + 6] = result[6];
            positions[chunk_idx + 7] = result[7];
        }
    }

    // Prevent optimization
    let _ = positions;

    instant::now() - start
}

/// Simple wrapper for getting current timestamp
mod instant {
    use std::cell::Cell;

    thread_local! {
        static START: Cell<f64> = Cell::new(0.0);
    }

    pub fn now() -> f64 {
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::window()
                .expect("should have a window")
                .performance()
                .expect("should have performance")
                .now()
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::time::SystemTime;
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64() * 1000.0
        }
    }
}
