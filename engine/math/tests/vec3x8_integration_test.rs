//! Integration test for Vec3x8 SIMD operations

#[cfg(feature = "simd")]
use engine_math::simd::{vec3_aos_to_soa_8, Vec3x8};

#[cfg(feature = "simd")]
use engine_math::Vec3;

#[cfg(feature = "simd")]
#[test]
fn test_vec3x8_aos_to_soa_conversion() {
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

    let pos_simd = vec3_aos_to_soa_8(&positions);
    let result = pos_simd.to_array();

    // Verify roundtrip conversion
    for (i, (original, converted)) in positions.iter().zip(result.iter()).enumerate() {
        assert!((original.x - converted.x).abs() < 1e-6, "Position {} x mismatch", i);
        assert!((original.y - converted.y).abs() < 1e-6, "Position {} y mismatch", i);
        assert!((original.z - converted.z).abs() < 1e-6, "Position {} z mismatch", i);
    }
}

#[cfg(feature = "simd")]
#[test]
fn test_vec3x8_add() {
    let positions = [Vec3::new(1.0, 2.0, 3.0); 8];
    let pos_simd = vec3_aos_to_soa_8(&positions);

    let offset = Vec3x8::splat(Vec3::new(100.0, 200.0, 300.0));
    let added = pos_simd + offset;
    let result = added.to_array();

    for vec in &result {
        assert!((vec.x - 101.0).abs() < 1e-6);
        assert!((vec.y - 202.0).abs() < 1e-6);
        assert!((vec.z - 303.0).abs() < 1e-6);
    }
}

#[cfg(feature = "simd")]
#[test]
fn test_vec3x8_scalar_mul() {
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
    let pos_simd = vec3_aos_to_soa_8(&positions);

    let scaled = pos_simd * 2.0;
    let result = scaled.to_array();

    for (i, (original, scaled_vec)) in positions.iter().zip(result.iter()).enumerate() {
        assert!((original.x * 2.0 - scaled_vec.x).abs() < 1e-6, "Vector {} x mismatch", i);
        assert!((original.y * 2.0 - scaled_vec.y).abs() < 1e-6, "Vector {} y mismatch", i);
        assert!((original.z * 2.0 - scaled_vec.z).abs() < 1e-6, "Vector {} z mismatch", i);
    }
}

#[cfg(feature = "simd")]
#[test]
fn test_vec3x8_physics_integration() {
    let positions = [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 1.0, 1.0),
        Vec3::new(2.0, 2.0, 2.0),
        Vec3::new(3.0, 3.0, 3.0),
        Vec3::new(4.0, 4.0, 4.0),
        Vec3::new(5.0, 5.0, 5.0),
        Vec3::new(6.0, 6.0, 6.0),
        Vec3::new(7.0, 7.0, 7.0),
    ];

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

    let pos_simd = vec3_aos_to_soa_8(&positions);
    let vel_simd = vec3_aos_to_soa_8(&velocities);
    let dt = 0.016;

    let new_pos = pos_simd.mul_add(vel_simd, dt);
    let result = new_pos.to_array();

    for (i, (pos, vel)) in positions.iter().zip(velocities.iter()).enumerate() {
        let expected = *pos + *vel * dt;
        let actual = result[i];

        assert!((expected.x - actual.x).abs() < 1e-4, "Vector {} x mismatch", i);
        assert!((expected.y - actual.y).abs() < 1e-4, "Vector {} y mismatch", i);
        assert!((expected.z - actual.z).abs() < 1e-4, "Vector {} z mismatch", i);
    }
}
