//! Procedural asset generation implementation
//!
//! NOTE: Procedural generation is part of Wave 2 and not yet integrated.
//! This is a simplified implementation for basic shapes.

use anyhow::{Context, Result};
use engine_assets::{AudioData, AudioFormat, MeshData, TextureData, TextureFormat, Vertex};
use glam::{Vec2, Vec3};
use std::f32::consts::PI;
use std::fs;
use std::path::PathBuf;
use tracing::info;

/// Generate a procedural asset
pub fn run(asset_type: &str, params: Vec<String>, output_path: PathBuf) -> Result<()> {
    info!(
        asset_type = asset_type,
        params = ?params,
        output = ?output_path,
        "Generating procedural asset"
    );

    match asset_type.to_lowercase().as_str() {
        "mesh" => generate_mesh(params, output_path),
        "texture" => generate_texture(params, output_path),
        "audio" => generate_audio(params, output_path),
        _ => anyhow::bail!("Unknown asset type: {}", asset_type),
    }
}

/// Generate a procedural mesh
fn generate_mesh(params: Vec<String>, output_path: PathBuf) -> Result<()> {
    if params.is_empty() {
        anyhow::bail!("Missing mesh type. Usage: generate mesh <type> [params...]");
    }

    let mesh_type = &params[0];

    let mesh = match mesh_type.to_lowercase().as_str() {
        "cube" => {
            let size = parse_f32(&params, 1, "size", 1.0)?;
            generate_cube(size)
        }
        "sphere" => {
            let radius = parse_f32(&params, 1, "radius", 1.0)?;
            let subdivisions_lat = parse_u32(&params, 2, "subdivisions_lat", 16)?;
            let subdivisions_lon = parse_u32(&params, 3, "subdivisions_lon", 32)?;
            generate_sphere(radius, subdivisions_lat, subdivisions_lon)
        }
        "plane" => {
            let width = parse_f32(&params, 1, "width", 10.0)?;
            let height = parse_f32(&params, 2, "height", 10.0)?;
            let subdivisions_x = parse_u32(&params, 3, "subdivisions_x", 10)?;
            let subdivisions_y = parse_u32(&params, 4, "subdivisions_y", 10)?;
            generate_plane(width, height, subdivisions_x, subdivisions_y)
        }
        "cylinder" => {
            let radius = parse_f32(&params, 1, "radius", 1.0)?;
            let height = parse_f32(&params, 2, "height", 2.0)?;
            let segments = parse_u32(&params, 3, "segments", 16)?;
            generate_cylinder(radius, height, segments)
        }
        _ => anyhow::bail!("Unknown mesh type: {}. Supported: cube, sphere, plane, cylinder", mesh_type),
    };

    info!(mesh_type = mesh_type, "Generating mesh");

    // Save to binary format
    let binary_data = mesh.to_binary();
    fs::write(&output_path, binary_data)
        .with_context(|| format!("Failed to write mesh: {}", output_path.display()))?;

    println!("✓ Generated mesh:");
    println!("  Type:      {}", mesh_type);
    println!("  Vertices:  {}", mesh.vertex_count());
    println!("  Indices:   {}", mesh.index_count());
    println!("  Triangles: {}", mesh.triangle_count());
    println!("  Output:    {}", output_path.display());

    Ok(())
}

/// Generate a cube mesh
fn generate_cube(size: f32) -> MeshData {
    let half = size / 2.0;

    let vertices = vec![
        // Front face (Z+)
        Vertex::new(Vec3::new(-half, -half, half), Vec3::Z, Vec2::new(0.0, 0.0)),
        Vertex::new(Vec3::new(half, -half, half), Vec3::Z, Vec2::new(1.0, 0.0)),
        Vertex::new(Vec3::new(half, half, half), Vec3::Z, Vec2::new(1.0, 1.0)),
        Vertex::new(Vec3::new(-half, half, half), Vec3::Z, Vec2::new(0.0, 1.0)),
        // Back face (Z-)
        Vertex::new(Vec3::new(half, -half, -half), Vec3::NEG_Z, Vec2::new(0.0, 0.0)),
        Vertex::new(Vec3::new(-half, -half, -half), Vec3::NEG_Z, Vec2::new(1.0, 0.0)),
        Vertex::new(Vec3::new(-half, half, -half), Vec3::NEG_Z, Vec2::new(1.0, 1.0)),
        Vertex::new(Vec3::new(half, half, -half), Vec3::NEG_Z, Vec2::new(0.0, 1.0)),
        // Top face (Y+)
        Vertex::new(Vec3::new(-half, half, half), Vec3::Y, Vec2::new(0.0, 0.0)),
        Vertex::new(Vec3::new(half, half, half), Vec3::Y, Vec2::new(1.0, 0.0)),
        Vertex::new(Vec3::new(half, half, -half), Vec3::Y, Vec2::new(1.0, 1.0)),
        Vertex::new(Vec3::new(-half, half, -half), Vec3::Y, Vec2::new(0.0, 1.0)),
        // Bottom face (Y-)
        Vertex::new(Vec3::new(-half, -half, -half), Vec3::NEG_Y, Vec2::new(0.0, 0.0)),
        Vertex::new(Vec3::new(half, -half, -half), Vec3::NEG_Y, Vec2::new(1.0, 0.0)),
        Vertex::new(Vec3::new(half, -half, half), Vec3::NEG_Y, Vec2::new(1.0, 1.0)),
        Vertex::new(Vec3::new(-half, -half, half), Vec3::NEG_Y, Vec2::new(0.0, 1.0)),
        // Right face (X+)
        Vertex::new(Vec3::new(half, -half, half), Vec3::X, Vec2::new(0.0, 0.0)),
        Vertex::new(Vec3::new(half, -half, -half), Vec3::X, Vec2::new(1.0, 0.0)),
        Vertex::new(Vec3::new(half, half, -half), Vec3::X, Vec2::new(1.0, 1.0)),
        Vertex::new(Vec3::new(half, half, half), Vec3::X, Vec2::new(0.0, 1.0)),
        // Left face (X-)
        Vertex::new(Vec3::new(-half, -half, -half), Vec3::NEG_X, Vec2::new(0.0, 0.0)),
        Vertex::new(Vec3::new(-half, -half, half), Vec3::NEG_X, Vec2::new(1.0, 0.0)),
        Vertex::new(Vec3::new(-half, half, half), Vec3::NEG_X, Vec2::new(1.0, 1.0)),
        Vertex::new(Vec3::new(-half, half, -half), Vec3::NEG_X, Vec2::new(0.0, 1.0)),
    ];

    #[rustfmt::skip]
    let indices = vec![
        0, 1, 2, 2, 3, 0,    // Front
        4, 5, 6, 6, 7, 4,    // Back
        8, 9, 10, 10, 11, 8, // Top
        12, 13, 14, 14, 15, 12, // Bottom
        16, 17, 18, 18, 19, 16, // Right
        20, 21, 22, 22, 23, 20, // Left
    ];

    MeshData { vertices, indices }
}

/// Generate a sphere mesh
fn generate_sphere(radius: f32, subdivisions_lat: u32, subdivisions_lon: u32) -> MeshData {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // Generate vertices
    for lat in 0..=subdivisions_lat {
        let theta = lat as f32 * PI / subdivisions_lat as f32;
        let sin_theta = theta.sin();
        let cos_theta = theta.cos();

        for lon in 0..=subdivisions_lon {
            let phi = lon as f32 * 2.0 * PI / subdivisions_lon as f32;
            let sin_phi = phi.sin();
            let cos_phi = phi.cos();

            let x = sin_theta * cos_phi;
            let y = cos_theta;
            let z = sin_theta * sin_phi;

            let position = Vec3::new(x, y, z) * radius;
            let normal = Vec3::new(x, y, z);
            let uv = Vec2::new(lon as f32 / subdivisions_lon as f32, lat as f32 / subdivisions_lat as f32);

            vertices.push(Vertex::new(position, normal, uv));
        }
    }

    // Generate indices
    for lat in 0..subdivisions_lat {
        for lon in 0..subdivisions_lon {
            let current = lat * (subdivisions_lon + 1) + lon;
            let next = current + subdivisions_lon + 1;

            indices.push(current);
            indices.push(next);
            indices.push(current + 1);

            indices.push(current + 1);
            indices.push(next);
            indices.push(next + 1);
        }
    }

    MeshData { vertices, indices }
}

/// Generate a plane mesh
fn generate_plane(width: f32, height: f32, subdivisions_x: u32, subdivisions_y: u32) -> MeshData {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let half_width = width / 2.0;
    let half_height = height / 2.0;

    // Generate vertices
    for y in 0..=subdivisions_y {
        for x in 0..=subdivisions_x {
            let px = (x as f32 / subdivisions_x as f32) * width - half_width;
            let py = (y as f32 / subdivisions_y as f32) * height - half_height;

            let position = Vec3::new(px, 0.0, py);
            let normal = Vec3::Y;
            let uv = Vec2::new(x as f32 / subdivisions_x as f32, y as f32 / subdivisions_y as f32);

            vertices.push(Vertex::new(position, normal, uv));
        }
    }

    // Generate indices
    for y in 0..subdivisions_y {
        for x in 0..subdivisions_x {
            let current = y * (subdivisions_x + 1) + x;
            let next = current + subdivisions_x + 1;

            indices.push(current);
            indices.push(next);
            indices.push(current + 1);

            indices.push(current + 1);
            indices.push(next);
            indices.push(next + 1);
        }
    }

    MeshData { vertices, indices }
}

/// Generate a cylinder mesh
fn generate_cylinder(radius: f32, height: f32, segments: u32) -> MeshData {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let half_height = height / 2.0;

    // Side vertices
    for i in 0..=segments {
        let angle = 2.0 * PI * i as f32 / segments as f32;
        let x = angle.cos() * radius;
        let z = angle.sin() * radius;

        // Bottom
        vertices.push(Vertex::new(
            Vec3::new(x, -half_height, z),
            Vec3::new(x, 0.0, z).normalize(),
            Vec2::new(i as f32 / segments as f32, 0.0),
        ));

        // Top
        vertices.push(Vertex::new(
            Vec3::new(x, half_height, z),
            Vec3::new(x, 0.0, z).normalize(),
            Vec2::new(i as f32 / segments as f32, 1.0),
        ));
    }

    // Side indices
    for i in 0..segments {
        let base = i * 2;
        indices.push(base);
        indices.push(base + 2);
        indices.push(base + 1);

        indices.push(base + 1);
        indices.push(base + 2);
        indices.push(base + 3);
    }

    // Add caps (simplified - center vertex + ring)
    let bottom_center = vertices.len() as u32;
    vertices.push(Vertex::new(
        Vec3::new(0.0, -half_height, 0.0),
        Vec3::NEG_Y,
        Vec2::new(0.5, 0.5),
    ));

    let top_center = vertices.len() as u32;
    vertices.push(Vertex::new(
        Vec3::new(0.0, half_height, 0.0),
        Vec3::Y,
        Vec2::new(0.5, 0.5),
    ));

    for i in 0..segments {
        let angle = 2.0 * PI * i as f32 / segments as f32;
        let x = angle.cos() * radius;
        let z = angle.sin() * radius;

        // Bottom cap
        vertices.push(Vertex::new(
            Vec3::new(x, -half_height, z),
            Vec3::NEG_Y,
            Vec2::new(0.5 + x / (2.0 * radius), 0.5 + z / (2.0 * radius)),
        ));

        // Top cap
        vertices.push(Vertex::new(
            Vec3::new(x, half_height, z),
            Vec3::Y,
            Vec2::new(0.5 + x / (2.0 * radius), 0.5 + z / (2.0 * radius)),
        ));
    }

    // Cap indices
    for i in 0..segments {
        let next = (i + 1) % segments;

        // Bottom cap
        indices.push(bottom_center);
        indices.push(bottom_center + 1 + i * 2);
        indices.push(bottom_center + 1 + next * 2);

        // Top cap
        indices.push(top_center);
        indices.push(top_center + 2 + next * 2);
        indices.push(top_center + 2 + i * 2);
    }

    MeshData { vertices, indices }
}

/// Generate a procedural texture
fn generate_texture(params: Vec<String>, output_path: PathBuf) -> Result<()> {
    if params.is_empty() {
        anyhow::bail!("Missing texture type. Usage: generate texture <type> [params...]");
    }

    let texture_type = &params[0];

    let texture = match texture_type.to_lowercase().as_str() {
        "checkerboard" => {
            let width = parse_u32(&params, 1, "width", 256)?;
            let height = parse_u32(&params, 2, "height", 256)?;
            let tile_size = parse_u32(&params, 3, "tile_size", 32)?;
            generate_checkerboard(width, height, tile_size)?
        }
        "gradient" => {
            let width = parse_u32(&params, 1, "width", 256)?;
            let height = parse_u32(&params, 2, "height", 256)?;
            generate_gradient(width, height)?
        }
        "noise" => {
            let width = parse_u32(&params, 1, "width", 256)?;
            let height = parse_u32(&params, 2, "height", 256)?;
            generate_noise(width, height)?
        }
        _ => anyhow::bail!("Unknown texture type: {}. Supported: checkerboard, gradient, noise", texture_type),
    };

    info!(texture_type = texture_type, "Generating texture");

    // Save to binary format
    let binary_data = bincode::serialize(&texture)?;
    fs::write(&output_path, binary_data)
        .with_context(|| format!("Failed to write texture: {}", output_path.display()))?;

    println!("✓ Generated texture:");
    println!("  Type:       {}", texture_type);
    println!("  Dimensions: {}x{}", texture.width, texture.height);
    println!("  Format:     {:?}", texture.format);
    println!("  Output:     {}", output_path.display());

    Ok(())
}

/// Generate a checkerboard texture
fn generate_checkerboard(width: u32, height: u32, tile_size: u32) -> Result<TextureData> {
    let mut data = vec![0u8; (width * height * 4) as usize];

    for y in 0..height {
        for x in 0..width {
            let is_white = ((x / tile_size) + (y / tile_size)) % 2 == 0;
            let color = if is_white { 255 } else { 0 };

            let index = ((y * width + x) * 4) as usize;
            data[index] = color;
            data[index + 1] = color;
            data[index + 2] = color;
            data[index + 3] = 255;
        }
    }

    TextureData::new(width, height, TextureFormat::RGBA8Unorm, data)
        .context("Failed to create checkerboard texture")
}

/// Generate a gradient texture
fn generate_gradient(width: u32, height: u32) -> Result<TextureData> {
    let mut data = vec![0u8; (width * height * 4) as usize];

    for y in 0..height {
        for x in 0..width {
            let index = ((y * width + x) * 4) as usize;
            data[index] = ((x as f32 / width as f32) * 255.0) as u8;
            data[index + 1] = ((y as f32 / height as f32) * 255.0) as u8;
            data[index + 2] = 128;
            data[index + 3] = 255;
        }
    }

    TextureData::new(width, height, TextureFormat::RGBA8Unorm, data)
        .context("Failed to create gradient texture")
}

/// Generate a simple noise texture (pseudorandom)
fn generate_noise(width: u32, height: u32) -> Result<TextureData> {
    let mut data = vec![0u8; (width * height * 4) as usize];

    // Simple pseudorandom noise
    for y in 0..height {
        for x in 0..width {
            let hash = ((x * 374761393) + (y * 668265263)) % 255;
            let value = hash as u8;

            let index = ((y * width + x) * 4) as usize;
            data[index] = value;
            data[index + 1] = value;
            data[index + 2] = value;
            data[index + 3] = 255;
        }
    }

    TextureData::new(width, height, TextureFormat::RGBA8Unorm, data)
        .context("Failed to create noise texture")
}

/// Generate procedural audio
fn generate_audio(params: Vec<String>, output_path: PathBuf) -> Result<()> {
    if params.is_empty() {
        anyhow::bail!("Missing audio type. Usage: generate audio <type> [params...]");
    }

    let audio_type = &params[0];

    let audio = match audio_type.to_lowercase().as_str() {
        "sine" => {
            let frequency = parse_f32(&params, 1, "frequency", 440.0)?;
            let duration = parse_f32(&params, 2, "duration", 1.0)?;
            generate_sine_wave(frequency, duration)
        }
        "whitenoise" => {
            let duration = parse_f32(&params, 1, "duration", 1.0)?;
            generate_white_noise(duration)
        }
        _ => anyhow::bail!("Unknown audio type: {}. Supported: sine, whitenoise", audio_type),
    };

    info!(audio_type = audio_type, "Generating audio");

    // Save to binary format
    let binary_data = bincode::serialize(&audio)?;
    fs::write(&output_path, binary_data)
        .with_context(|| format!("Failed to write audio: {}", output_path.display()))?;

    println!("✓ Generated audio:");
    println!("  Type:        {}", audio_type);
    println!("  Sample Rate: {}", audio.sample_rate);
    println!("  Channels:    {}", audio.channels);
    println!("  Format:      {:?}", audio.format);
    println!("  Output:      {}", output_path.display());

    Ok(())
}

/// Generate a sine wave
fn generate_sine_wave(frequency: f32, duration_secs: f32) -> AudioData {
    const SAMPLE_RATE: u32 = 44100;
    let samples_count = (SAMPLE_RATE as f32 * duration_secs) as usize;
    let mut samples = Vec::with_capacity(samples_count * 2); // Stereo

    for i in 0..samples_count {
        let t = i as f32 / SAMPLE_RATE as f32;
        let value = (2.0 * PI * frequency * t).sin();
        let sample = (value * 32767.0) as i16;

        // Write as stereo (left and right)
        samples.push(sample.to_le_bytes()[0]);
        samples.push(sample.to_le_bytes()[1]);
        samples.push(sample.to_le_bytes()[0]);
        samples.push(sample.to_le_bytes()[1]);
    }

    AudioData {
        sample_rate: SAMPLE_RATE,
        channels: 2,
        format: AudioFormat::PCM16,
        data: samples,
    }
}

/// Generate white noise
fn generate_white_noise(duration_secs: f32) -> AudioData {
    const SAMPLE_RATE: u32 = 44100;
    let samples_count = (SAMPLE_RATE as f32 * duration_secs) as usize;
    let mut samples = Vec::with_capacity(samples_count * 2); // Stereo

    for i in 0..samples_count {
        // Simple pseudorandom noise
        let hash = ((i * 374761393) % 65535) as i32 - 32768;
        let sample = hash as i16;

        // Write as stereo
        samples.push(sample.to_le_bytes()[0]);
        samples.push(sample.to_le_bytes()[1]);
        samples.push(sample.to_le_bytes()[0]);
        samples.push(sample.to_le_bytes()[1]);
    }

    AudioData {
        sample_rate: SAMPLE_RATE,
        channels: 2,
        format: AudioFormat::PCM16,
        data: samples,
    }
}

/// Parse f32 parameter from string array
fn parse_f32(params: &[String], index: usize, name: &str, default: f32) -> Result<f32> {
    if index >= params.len() {
        return Ok(default);
    }

    params[index]
        .parse::<f32>()
        .with_context(|| format!("Failed to parse {} as f32", name))
}

/// Parse u32 parameter from string array
fn parse_u32(params: &[String], index: usize, name: &str, default: u32) -> Result<u32> {
    if index >= params.len() {
        return Ok(default);
    }

    params[index]
        .parse::<u32>()
        .with_context(|| format!("Failed to parse {} as u32", name))
}
