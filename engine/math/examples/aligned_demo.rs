//! Demonstration of cache-aligned memory allocations for SIMD performance.
//!
//! This example shows how to use AlignedVec with SIMD types to achieve
//! better cache performance and prevent false sharing.

use engine_math::aligned::AlignedVec;
use engine_math::simd::{vec3_aos_to_soa_4, Vec3x4};
use engine_math::Vec3;

fn main() {
    println!("=== Cache-Aligned Memory Allocation Demo ===\n");

    // Create a cache-line aligned vector
    let mut positions: AlignedVec<Vec3x4, 64> = AlignedVec::new();

    println!("1. Created AlignedVec<Vec3x4, 64>");
    println!("   Empty vector capacity: {}", positions.capacity());

    // Add some data
    for i in 0..10 {
        let vec = Vec3x4::splat(Vec3::new(i as f32, i as f32 * 2.0, i as f32 * 3.0));
        positions.push(vec);
    }

    println!("\n2. Added 10 Vec3x4 elements");
    println!("   Current length: {}", positions.len());
    println!("   Current capacity: {}", positions.capacity());

    // Check alignment
    let ptr = positions.as_ptr() as usize;
    println!("\n3. Memory alignment check:");
    println!("   Pointer address: 0x{:X}", ptr);
    println!("   Is 64-byte aligned: {}", ptr % 64 == 0);
    println!("   Is 16-byte aligned: {}", ptr % 16 == 0);

    // Demonstrate SIMD operations on aligned data
    println!("\n4. Performing SIMD physics integration:");

    let mut velocities: AlignedVec<Vec3x4, 64> = AlignedVec::new();
    for _ in 0..10 {
        velocities.push(Vec3x4::splat(Vec3::new(0.1, 0.2, 0.3)));
    }

    let dt = 0.016; // 60 FPS timestep

    // Physics integration: pos = pos + vel * dt
    for i in 0..positions.len() {
        positions[i] = positions[i].mul_add(velocities[i], dt);
    }

    println!("   Integrated {} chunks (40 vectors total)", positions.len());
    println!("   Each SIMD operation processes 4 vectors at once");

    // Convert one back to see results
    let result = positions[0].to_array();
    println!("\n5. Sample result (first chunk):");
    for (i, vec) in result.iter().enumerate() {
        println!("   Vector {}: ({:.3}, {:.3}, {:.3})", i, vec.x, vec.y, vec.z);
    }

    // Demonstrate aligned load/store
    println!("\n6. Aligned load/store operations:");

    let mut buffer: AlignedVec<f32, 16> = AlignedVec::with_capacity(12);
    buffer.resize(12, 0.0);

    let test_vec = Vec3x4::splat(Vec3::new(1.0, 2.0, 3.0));

    unsafe {
        // Store to aligned buffer
        test_vec.store_aligned(buffer.as_mut_ptr());
        println!("   Stored Vec3x4 to aligned buffer");

        // Load back
        let loaded = Vec3x4::load_aligned(buffer.as_ptr());
        println!("   Loaded Vec3x4 from aligned buffer");

        // Verify
        let loaded_array = loaded.to_array();
        println!(
            "   First element: ({:.1}, {:.1}, {:.1})",
            loaded_array[0].x, loaded_array[0].y, loaded_array[0].z
        );
    }

    // Compare standard Vec vs AlignedVec
    println!("\n7. Memory layout comparison:");

    let standard_vec: Vec<Vec3x4> = vec![Vec3x4::splat(Vec3::new(1.0, 2.0, 3.0)); 5];
    let standard_ptr = standard_vec.as_ptr() as usize;

    let mut aligned_vec: AlignedVec<Vec3x4, 64> = AlignedVec::new();
    for _ in 0..5 {
        aligned_vec.push(Vec3x4::splat(Vec3::new(1.0, 2.0, 3.0)));
    }
    let aligned_ptr = aligned_vec.as_ptr() as usize;

    println!(
        "   Standard Vec pointer: 0x{:X} (alignment: {} bytes)",
        standard_ptr,
        if standard_ptr % 64 == 0 {
            "64"
        } else if standard_ptr % 16 == 0 {
            "16"
        } else {
            "< 16"
        }
    );
    println!("   AlignedVec<T, 64> pointer: 0x{:X} (alignment: 64 bytes)", aligned_ptr);

    println!("\n=== Benefits of Cache-Line Alignment ===");
    println!("1. Prevents cache line splits (data spanning two cache lines)");
    println!("2. Enables faster aligned SIMD load/store instructions");
    println!("3. Prevents false sharing in multi-threaded scenarios");
    println!("4. Better utilization of CPU cache bandwidth");

    println!("\n=== Use Cases ===");
    println!("- Bulk physics integration (positions, velocities)");
    println!("- Particle systems with thousands of particles");
    println!("- Spatial data structures (BVH nodes, octree data)");
    println!("- Any scenario with large arrays of SIMD types");
}
