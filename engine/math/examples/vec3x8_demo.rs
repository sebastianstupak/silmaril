//! Demonstration of Vec3x8 SIMD operations

use engine_math::simd::{vec3_aos_to_soa_8, Vec3x8};
use engine_math::Vec3;

fn main() {
    println!("Vec3x8 SIMD Demonstration");
    println!("=========================\n");

    // Create 8 vectors
    let positions = [
        Vec3::new(1.0, 2.0, 3.0),
        Vec3::new(4.0, 5.0, 6.0),
        Vec3::new(7.0, 8.0, 9.0),
        Vec3::new(10.0, 11.0, 12.0),
        Vec3::new(13.0, 14.0, 15.0),
        Vec3::new(16.0, 17.0, 18.0),
        Vec3::new(19.0, 20.0, 21.0),
        Vec3::new(22.0, 23.0, 24.0),
    ];

    println!("Original positions (AoS):");
    for (i, pos) in positions.iter().enumerate() {
        println!("  [{}] = ({:.1}, {:.1}, {:.1})", i, pos.x, pos.y, pos.z);
    }

    // Convert to SIMD (SoA)
    let pos_simd = vec3_aos_to_soa_8(&positions);
    println!("\nConverted to SoA format for SIMD processing");

    // Perform SIMD operations
    println!("\n1. Add operation:");
    let offset = Vec3x8::splat(Vec3::new(100.0, 200.0, 300.0));
    let added = pos_simd + offset;
    let result = added.to_array();
    println!("  Added (100, 200, 300) to all vectors:");
    for (i, v) in result.iter().enumerate() {
        println!("    [{}] = ({:.1}, {:.1}, {:.1})", i, v.x, v.y, v.z);
    }

    // Scalar multiplication
    println!("\n2. Scalar multiplication:");
    let scaled = pos_simd * 2.0;
    let result = scaled.to_array();
    println!("  Multiplied all vectors by 2.0:");
    for (i, v) in result.iter().enumerate() {
        println!("    [{}] = ({:.1}, {:.1}, {:.1})", i, v.x, v.y, v.z);
    }

    // Physics integration (mul_add)
    println!("\n3. Physics integration (pos + vel * dt):");
    let velocities = [
        Vec3::new(0.1, 0.2, 0.3),
        Vec3::new(0.4, 0.5, 0.6),
        Vec3::new(0.7, 0.8, 0.9),
        Vec3::new(1.0, 1.1, 1.2),
        Vec3::new(1.3, 1.4, 1.5),
        Vec3::new(1.6, 1.7, 1.8),
        Vec3::new(1.9, 2.0, 2.1),
        Vec3::new(2.2, 2.3, 2.4),
    ];
    let vel_simd = vec3_aos_to_soa_8(&velocities);
    let dt = 0.016; // ~60 FPS
    let new_pos = pos_simd.mul_add(vel_simd, dt);
    let result = new_pos.to_array();
    println!("  After physics step (dt = {}):", dt);
    for (i, v) in result.iter().enumerate() {
        println!("    [{}] = ({:.4}, {:.4}, {:.4})", i, v.x, v.y, v.z);
    }

    // Dot product
    println!("\n4. Dot product:");
    let a = Vec3x8::splat(Vec3::new(1.0, 2.0, 3.0));
    let b = Vec3x8::splat(Vec3::new(4.0, 5.0, 6.0));
    let dots = a.dot(b);
    let dot_results = dots.to_array();
    println!("  Dot product of (1, 2, 3) · (4, 5, 6):");
    for (i, &dot) in dot_results.iter().enumerate() {
        println!("    [{}] = {:.1}", i, dot);
    }

    println!("\n✓ Vec3x8 processes 8 vectors simultaneously using 256-bit AVX2 instructions");
    println!("✓ Expected speedup: 5-6x over scalar operations");
}
