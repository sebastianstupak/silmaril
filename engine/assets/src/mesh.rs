//! Mesh data structures (pure data, no GPU/rendering dependencies)
//!
//! Can be used by:
//! - Server for procedural generation
//! - Physics for collision detection
//! - Tools for asset processing
//! - Client for rendering (via engine-renderer)

use crate::validation::{check_f32, compute_hash, AssetValidator, ValidationError};
use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use glam::{Vec2, Vec3};
use tracing::{info, instrument};

define_error! {
    pub enum MeshError {
        InvalidObjFormat { reason: String } = ErrorCode::MeshLoadFailed, ErrorSeverity::Error,
        InvalidVertexData { reason: String } = ErrorCode::MeshLoadFailed, ErrorSeverity::Error,
        InvalidGltfFormat { reason: String } = ErrorCode::MeshLoadFailed, ErrorSeverity::Error,
        InvalidBinaryFormat { reason: String } = ErrorCode::MeshLoadFailed, ErrorSeverity::Error,
        InvalidFbxFormat { reason: String } = ErrorCode::MeshLoadFailed, ErrorSeverity::Error,
    }
}

/// Vertex data with position, normal, and UV coordinates
///
/// This is a pure data structure - rendering backends define their own
/// vertex input layouts based on this data.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Vertex {
    /// 3D position
    pub position: Vec3,
    /// Surface normal
    pub normal: Vec3,
    /// Texture coordinates
    pub uv: Vec2,
}

impl Vertex {
    /// Create a new vertex
    pub fn new(position: Vec3, normal: Vec3, uv: Vec2) -> Self {
        Self { position, normal, uv }
    }
}

/// Mesh data (CPU-side geometry)
///
/// Pure data structure - no GPU buffers or rendering state.
/// Rendering backends create GPU resources from this data.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MeshData {
    /// Vertex data
    pub vertices: Vec<Vertex>,
    /// Index data (triangles)
    pub indices: Vec<u32>,
}

impl MeshData {
    /// Create an empty mesh
    pub fn new() -> Self {
        Self { vertices: Vec::new(), indices: Vec::new() }
    }

    /// Create a mesh with pre-allocated capacity
    pub fn with_capacity(vertex_count: usize, index_count: usize) -> Self {
        Self {
            vertices: Vec::with_capacity(vertex_count),
            indices: Vec::with_capacity(index_count),
        }
    }

    /// Create a unit cube mesh (2x2x2, centered at origin)
    #[instrument]
    pub fn cube() -> Self {
        info!("Creating cube mesh");

        let vertices = vec![
            // Front face (Z+)
            Vertex::new(Vec3::new(-1.0, -1.0, 1.0), Vec3::Z, Vec2::new(0.0, 0.0)),
            Vertex::new(Vec3::new(1.0, -1.0, 1.0), Vec3::Z, Vec2::new(1.0, 0.0)),
            Vertex::new(Vec3::new(1.0, 1.0, 1.0), Vec3::Z, Vec2::new(1.0, 1.0)),
            Vertex::new(Vec3::new(-1.0, 1.0, 1.0), Vec3::Z, Vec2::new(0.0, 1.0)),
            // Back face (Z-)
            Vertex::new(Vec3::new(1.0, -1.0, -1.0), Vec3::NEG_Z, Vec2::new(0.0, 0.0)),
            Vertex::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::NEG_Z, Vec2::new(1.0, 0.0)),
            Vertex::new(Vec3::new(-1.0, 1.0, -1.0), Vec3::NEG_Z, Vec2::new(1.0, 1.0)),
            Vertex::new(Vec3::new(1.0, 1.0, -1.0), Vec3::NEG_Z, Vec2::new(0.0, 1.0)),
            // Top face (Y+)
            Vertex::new(Vec3::new(-1.0, 1.0, 1.0), Vec3::Y, Vec2::new(0.0, 0.0)),
            Vertex::new(Vec3::new(1.0, 1.0, 1.0), Vec3::Y, Vec2::new(1.0, 0.0)),
            Vertex::new(Vec3::new(1.0, 1.0, -1.0), Vec3::Y, Vec2::new(1.0, 1.0)),
            Vertex::new(Vec3::new(-1.0, 1.0, -1.0), Vec3::Y, Vec2::new(0.0, 1.0)),
            // Bottom face (Y-)
            Vertex::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::NEG_Y, Vec2::new(0.0, 0.0)),
            Vertex::new(Vec3::new(1.0, -1.0, -1.0), Vec3::NEG_Y, Vec2::new(1.0, 0.0)),
            Vertex::new(Vec3::new(1.0, -1.0, 1.0), Vec3::NEG_Y, Vec2::new(1.0, 1.0)),
            Vertex::new(Vec3::new(-1.0, -1.0, 1.0), Vec3::NEG_Y, Vec2::new(0.0, 1.0)),
            // Right face (X+)
            Vertex::new(Vec3::new(1.0, -1.0, 1.0), Vec3::X, Vec2::new(0.0, 0.0)),
            Vertex::new(Vec3::new(1.0, -1.0, -1.0), Vec3::X, Vec2::new(1.0, 0.0)),
            Vertex::new(Vec3::new(1.0, 1.0, -1.0), Vec3::X, Vec2::new(1.0, 1.0)),
            Vertex::new(Vec3::new(1.0, 1.0, 1.0), Vec3::X, Vec2::new(0.0, 1.0)),
            // Left face (X-)
            Vertex::new(Vec3::new(-1.0, -1.0, -1.0), Vec3::NEG_X, Vec2::new(0.0, 0.0)),
            Vertex::new(Vec3::new(-1.0, -1.0, 1.0), Vec3::NEG_X, Vec2::new(1.0, 0.0)),
            Vertex::new(Vec3::new(-1.0, 1.0, 1.0), Vec3::NEG_X, Vec2::new(1.0, 1.0)),
            Vertex::new(Vec3::new(-1.0, 1.0, -1.0), Vec3::NEG_X, Vec2::new(0.0, 1.0)),
        ];

        #[rustfmt::skip]
        let indices = vec![
            // Front
            0, 1, 2, 2, 3, 0,
            // Back
            4, 5, 6, 6, 7, 4,
            // Top
            8, 9, 10, 10, 11, 8,
            // Bottom
            12, 13, 14, 14, 15, 12,
            // Right
            16, 17, 18, 18, 19, 16,
            // Left
            20, 21, 22, 22, 23, 20,
        ];

        Self { vertices, indices }
    }

    /// Create a triangle mesh (simple test geometry)
    pub fn triangle() -> Self {
        let vertices = vec![
            Vertex::new(Vec3::new(0.0, -0.5, 0.0), Vec3::Z, Vec2::new(0.5, 1.0)),
            Vertex::new(Vec3::new(0.5, 0.5, 0.0), Vec3::Z, Vec2::new(1.0, 0.0)),
            Vertex::new(Vec3::new(-0.5, 0.5, 0.0), Vec3::Z, Vec2::new(0.0, 0.0)),
        ];

        let indices = vec![0, 1, 2];

        Self { vertices, indices }
    }

    /// Load mesh from OBJ file (simple parser, no materials)
    #[instrument(skip(obj_data))]
    pub fn from_obj(obj_data: &str) -> Result<Self, MeshError> {
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for line in obj_data.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "v" if parts.len() >= 4 => {
                    let x = parts[1].parse().map_err(|_| {
                        MeshError::invalidobjformat("Invalid vertex X coordinate".to_string())
                    })?;
                    let y = parts[2].parse().map_err(|_| {
                        MeshError::invalidobjformat("Invalid vertex Y coordinate".to_string())
                    })?;
                    let z = parts[3].parse().map_err(|_| {
                        MeshError::invalidobjformat("Invalid vertex Z coordinate".to_string())
                    })?;
                    positions.push(Vec3::new(x, y, z));
                }
                "vn" if parts.len() >= 4 => {
                    let x = parts[1].parse().map_err(|_| {
                        MeshError::invalidobjformat("Invalid normal X coordinate".to_string())
                    })?;
                    let y = parts[2].parse().map_err(|_| {
                        MeshError::invalidobjformat("Invalid normal Y coordinate".to_string())
                    })?;
                    let z = parts[3].parse().map_err(|_| {
                        MeshError::invalidobjformat("Invalid normal Z coordinate".to_string())
                    })?;
                    normals.push(Vec3::new(x, y, z));
                }
                "vt" if parts.len() >= 3 => {
                    let u = parts[1].parse().map_err(|_| {
                        MeshError::invalidobjformat("Invalid UV U coordinate".to_string())
                    })?;
                    let v = parts[2].parse().map_err(|_| {
                        MeshError::invalidobjformat("Invalid UV V coordinate".to_string())
                    })?;
                    uvs.push(Vec2::new(u, v));
                }
                "f" if parts.len() >= 4 => {
                    for i in 1..parts.len() {
                        let vert_parts: Vec<&str> = parts[i].split('/').collect();

                        let pos_idx: usize = vert_parts[0].parse::<usize>().map_err(|_| {
                            MeshError::invalidobjformat("Invalid face position index".to_string())
                        })? - 1;

                        let uv_idx = if vert_parts.len() > 1 && !vert_parts[1].is_empty() {
                            vert_parts[1].parse::<usize>().ok().map(|idx| idx - 1)
                        } else {
                            None
                        };

                        let norm_idx = if vert_parts.len() > 2 {
                            vert_parts[2].parse::<usize>().ok().map(|idx| idx - 1)
                        } else {
                            None
                        };

                        let position = positions.get(pos_idx).copied().unwrap_or(Vec3::ZERO);
                        let uv = uv_idx.and_then(|idx| uvs.get(idx).copied()).unwrap_or(Vec2::ZERO);
                        let normal =
                            norm_idx.and_then(|idx| normals.get(idx).copied()).unwrap_or(Vec3::Z);

                        let index = vertices.len() as u32;
                        vertices.push(Vertex::new(position, normal, uv));
                        indices.push(index);
                    }
                }
                _ => {}
            }
        }

        info!(vertices = vertices.len(), indices = indices.len(), "OBJ mesh loaded");

        Ok(Self { vertices, indices })
    }

    /// Get vertex count
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Get index count
    pub fn index_count(&self) -> usize {
        self.indices.len()
    }

    /// Get triangle count
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Calculate axis-aligned bounding box
    pub fn bounding_box(&self) -> (Vec3, Vec3) {
        if self.vertices.is_empty() {
            return (Vec3::ZERO, Vec3::ZERO);
        }

        let mut min = self.vertices[0].position;
        let mut max = self.vertices[0].position;

        for vertex in &self.vertices {
            min = min.min(vertex.position);
            max = max.max(vertex.position);
        }

        (min, max)
    }

    /// Calculate mesh centroid
    pub fn centroid(&self) -> Vec3 {
        if self.vertices.is_empty() {
            return Vec3::ZERO;
        }

        let sum: Vec3 = self.vertices.iter().map(|v| v.position).sum();
        sum / self.vertices.len() as f32
    }

    /// Load mesh from glTF file
    ///
    /// # Arguments
    /// * `gltf_data` - The glTF JSON data
    /// * `bin_data` - Optional external binary buffer data (for non-embedded buffers)
    #[instrument(skip(gltf_data, bin_data))]
    pub fn from_gltf(gltf_data: &[u8], bin_data: Option<&[u8]>) -> Result<Self, MeshError> {
        use gltf::Gltf;

        let gltf = Gltf::from_slice(gltf_data)
            .map_err(|e| MeshError::invalidgltfformat(format!("Failed to parse glTF: {}", e)))?;

        let mut vertices = Vec::new();

        // Get the first mesh primitive (simplified - assumes single mesh)
        let mesh = gltf
            .meshes()
            .next()
            .ok_or_else(|| MeshError::invalidgltfformat("No meshes found in glTF".to_string()))?;

        let primitive = mesh.primitives().next().ok_or_else(|| {
            MeshError::invalidgltfformat("No primitives found in mesh".to_string())
        })?;

        // Get buffer data
        let buffer_data = if let Some(buffer) = gltf.buffers().next() {
            match buffer.source() {
                gltf::buffer::Source::Uri(uri) if uri.starts_with("data:") => {
                    // Embedded base64 data
                    Self::decode_data_uri(uri)?
                }
                gltf::buffer::Source::Uri(_) => {
                    // External binary file
                    bin_data
                        .ok_or_else(|| {
                            MeshError::invalidgltfformat(
                                "External buffer required but not provided".to_string(),
                            )
                        })?
                        .to_vec()
                }
                gltf::buffer::Source::Bin => {
                    return Err(MeshError::invalidgltfformat(
                        "GLB format not supported yet".to_string(),
                    ));
                }
            }
        } else {
            return Err(MeshError::invalidgltfformat("No buffer found".to_string()));
        };

        // Read positions
        let positions = if let Some(accessor) = primitive.get(&gltf::Semantic::Positions) {
            Self::read_vec3_accessor(&accessor, &buffer_data)?
        } else {
            return Err(MeshError::invalidgltfformat("No positions found".to_string()));
        };

        // Read normals (or generate default)
        let normals = if let Some(accessor) = primitive.get(&gltf::Semantic::Normals) {
            Self::read_vec3_accessor(&accessor, &buffer_data)?
        } else {
            vec![Vec3::Z; positions.len()]
        };

        // Read UVs (or generate default)
        let uvs = if let Some(accessor) = primitive.get(&gltf::Semantic::TexCoords(0)) {
            Self::read_vec2_accessor(&accessor, &buffer_data)?
        } else {
            vec![Vec2::ZERO; positions.len()]
        };

        // Read indices
        let gltf_indices = if let Some(accessor) = primitive.indices() {
            Self::read_indices_accessor(&accessor, &buffer_data)?
        } else {
            // No indices - generate sequential
            (0..positions.len() as u32).collect()
        };

        // Build vertices
        for i in 0..positions.len() {
            vertices.push(Vertex::new(
                positions[i],
                normals.get(i).copied().unwrap_or(Vec3::Z),
                uvs.get(i).copied().unwrap_or(Vec2::ZERO),
            ));
        }

        let indices = gltf_indices;

        info!(vertices = vertices.len(), indices = indices.len(), "glTF mesh loaded");

        Ok(Self { vertices, indices })
    }

    /// Serialize mesh to custom binary format
    ///
    /// Format:
    /// ```text
    /// [4 bytes: magic "MESH"]
    /// [4 bytes: version (1)]
    /// [4 bytes: vertex_count]
    /// [4 bytes: index_count]
    /// [vertex_count * 32 bytes: vertices (pos + normal + uv)]
    /// [index_count * 4 bytes: indices]
    /// ```
    #[instrument(skip(self))]
    pub fn to_binary(&self) -> Vec<u8> {
        let mut data = Vec::new();

        // Magic number
        data.extend_from_slice(b"MESH");

        // Version
        data.extend_from_slice(&1u32.to_le_bytes());

        // Counts
        data.extend_from_slice(&(self.vertices.len() as u32).to_le_bytes());
        data.extend_from_slice(&(self.indices.len() as u32).to_le_bytes());

        // Vertices
        for vertex in &self.vertices {
            data.extend_from_slice(&vertex.position.x.to_le_bytes());
            data.extend_from_slice(&vertex.position.y.to_le_bytes());
            data.extend_from_slice(&vertex.position.z.to_le_bytes());
            data.extend_from_slice(&vertex.normal.x.to_le_bytes());
            data.extend_from_slice(&vertex.normal.y.to_le_bytes());
            data.extend_from_slice(&vertex.normal.z.to_le_bytes());
            data.extend_from_slice(&vertex.uv.x.to_le_bytes());
            data.extend_from_slice(&vertex.uv.y.to_le_bytes());
        }

        // Indices
        for &index in &self.indices {
            data.extend_from_slice(&index.to_le_bytes());
        }

        info!(
            size = data.len(),
            vertices = self.vertices.len(),
            indices = self.indices.len(),
            "Binary mesh serialized"
        );

        data
    }

    /// Deserialize mesh from custom binary format
    #[instrument(skip(data))]
    pub fn from_binary(data: &[u8]) -> Result<Self, MeshError> {
        if data.len() < 16 {
            return Err(MeshError::invalidbinaryformat("Data too small for header".to_string()));
        }

        // Check magic
        if &data[0..4] != b"MESH" {
            return Err(MeshError::invalidbinaryformat("Invalid magic number".to_string()));
        }

        // Check version
        let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        if version != 1 {
            return Err(MeshError::invalidbinaryformat(format!(
                "Unsupported version: {}",
                version
            )));
        }

        // Read counts
        let vertex_count = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
        let index_count = u32::from_le_bytes([data[12], data[13], data[14], data[15]]) as usize;

        // Validate size
        let expected_size = 16 + vertex_count * 32 + index_count * 4;
        if data.len() != expected_size {
            return Err(MeshError::invalidbinaryformat(format!(
                "Data size mismatch: expected {}, got {}",
                expected_size,
                data.len()
            )));
        }

        let mut vertices = Vec::with_capacity(vertex_count);
        let mut indices = Vec::with_capacity(index_count);

        // Read vertices
        let mut offset = 16;
        for _ in 0..vertex_count {
            let px = f32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            let py = f32::from_le_bytes([
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]);
            let pz = f32::from_le_bytes([
                data[offset + 8],
                data[offset + 9],
                data[offset + 10],
                data[offset + 11],
            ]);

            let nx = f32::from_le_bytes([
                data[offset + 12],
                data[offset + 13],
                data[offset + 14],
                data[offset + 15],
            ]);
            let ny = f32::from_le_bytes([
                data[offset + 16],
                data[offset + 17],
                data[offset + 18],
                data[offset + 19],
            ]);
            let nz = f32::from_le_bytes([
                data[offset + 20],
                data[offset + 21],
                data[offset + 22],
                data[offset + 23],
            ]);

            let u = f32::from_le_bytes([
                data[offset + 24],
                data[offset + 25],
                data[offset + 26],
                data[offset + 27],
            ]);
            let v = f32::from_le_bytes([
                data[offset + 28],
                data[offset + 29],
                data[offset + 30],
                data[offset + 31],
            ]);

            vertices.push(Vertex::new(
                Vec3::new(px, py, pz),
                Vec3::new(nx, ny, nz),
                Vec2::new(u, v),
            ));

            offset += 32;
        }

        // Read indices
        for _ in 0..index_count {
            let index = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            indices.push(index);
            offset += 4;
        }

        info!(vertices = vertices.len(), indices = indices.len(), "Binary mesh loaded");

        Ok(Self { vertices, indices })
    }

    #[cfg(feature = "fbx-support")]
    /// Load mesh from FBX file
    #[instrument(skip(fbx_data))]
    pub fn from_fbx(fbx_data: &[u8]) -> Result<Self, MeshError> {
        use fbxcel_dom::any::AnyDocument;

        let doc = AnyDocument::from_seekable_reader(std::io::Cursor::new(fbx_data))
            .map_err(|e| MeshError::invalidfbxformat(format!("Failed to parse FBX: {}", e)))?;

        // Simplified FBX loading - just extract first mesh geometry
        // In a real implementation, you'd traverse the scene graph properly

        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        // This is a placeholder - full FBX support requires complex scene graph traversal
        // For now, just return an error indicating it's not fully implemented
        Err(MeshError::invalidfbxformat("FBX loading not yet fully implemented".to_string()))
    }

    // ========================================================================
    // Helper methods for glTF loading
    // ========================================================================

    fn decode_data_uri(uri: &str) -> Result<Vec<u8>, MeshError> {
        // Parse data URI: data:application/octet-stream;base64,<data>
        let parts: Vec<&str> = uri.split(',').collect();
        if parts.len() != 2 {
            return Err(MeshError::invalidgltfformat("Invalid data URI format".to_string()));
        }

        // Decode base64
        let decoded = base64_decode(parts[1])?;
        Ok(decoded)
    }

    fn read_vec3_accessor(
        accessor: &gltf::Accessor,
        buffer_data: &[u8],
    ) -> Result<Vec<Vec3>, MeshError> {
        let view = accessor.view().ok_or_else(|| {
            MeshError::invalidgltfformat("Accessor missing buffer view".to_string())
        })?;

        let offset = view.offset() + accessor.offset();
        let stride = view.stride().unwrap_or(12); // VEC3 = 12 bytes by default

        let mut result = Vec::with_capacity(accessor.count());

        for i in 0..accessor.count() {
            let start = offset + i * stride;
            let x = f32::from_le_bytes([
                buffer_data[start],
                buffer_data[start + 1],
                buffer_data[start + 2],
                buffer_data[start + 3],
            ]);
            let y = f32::from_le_bytes([
                buffer_data[start + 4],
                buffer_data[start + 5],
                buffer_data[start + 6],
                buffer_data[start + 7],
            ]);
            let z = f32::from_le_bytes([
                buffer_data[start + 8],
                buffer_data[start + 9],
                buffer_data[start + 10],
                buffer_data[start + 11],
            ]);

            result.push(Vec3::new(x, y, z));
        }

        Ok(result)
    }

    fn read_vec2_accessor(
        accessor: &gltf::Accessor,
        buffer_data: &[u8],
    ) -> Result<Vec<Vec2>, MeshError> {
        let view = accessor.view().ok_or_else(|| {
            MeshError::invalidgltfformat("Accessor missing buffer view".to_string())
        })?;

        let offset = view.offset() + accessor.offset();
        let stride = view.stride().unwrap_or(8); // VEC2 = 8 bytes by default

        let mut result = Vec::with_capacity(accessor.count());

        for i in 0..accessor.count() {
            let start = offset + i * stride;
            let x = f32::from_le_bytes([
                buffer_data[start],
                buffer_data[start + 1],
                buffer_data[start + 2],
                buffer_data[start + 3],
            ]);
            let y = f32::from_le_bytes([
                buffer_data[start + 4],
                buffer_data[start + 5],
                buffer_data[start + 6],
                buffer_data[start + 7],
            ]);

            result.push(Vec2::new(x, y));
        }

        Ok(result)
    }

    fn read_indices_accessor(
        accessor: &gltf::Accessor,
        buffer_data: &[u8],
    ) -> Result<Vec<u32>, MeshError> {
        let view = accessor.view().ok_or_else(|| {
            MeshError::invalidgltfformat("Accessor missing buffer view".to_string())
        })?;

        let offset = view.offset() + accessor.offset();
        let mut result = Vec::with_capacity(accessor.count());

        match accessor.data_type() {
            gltf::accessor::DataType::U16 => {
                for i in 0..accessor.count() {
                    let start = offset + i * 2;
                    let index = u16::from_le_bytes([buffer_data[start], buffer_data[start + 1]]);
                    result.push(index as u32);
                }
            }
            gltf::accessor::DataType::U32 => {
                for i in 0..accessor.count() {
                    let start = offset + i * 4;
                    let index = u32::from_le_bytes([
                        buffer_data[start],
                        buffer_data[start + 1],
                        buffer_data[start + 2],
                        buffer_data[start + 3],
                    ]);
                    result.push(index);
                }
            }
            _ => {
                return Err(MeshError::invalidgltfformat(
                    "Unsupported index data type".to_string(),
                ));
            }
        }

        Ok(result)
    }
}

// Base64 decoder (simple implementation)
fn base64_decode(input: &str) -> Result<Vec<u8>, MeshError> {
    let input = input.trim();
    let mut output = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0;

    for &ch in input.as_bytes() {
        if ch == b'=' {
            break;
        }

        let value = if (b'A'..=b'Z').contains(&ch) {
            ch - b'A'
        } else if (b'a'..=b'z').contains(&ch) {
            ch - b'a' + 26
        } else if (b'0'..=b'9').contains(&ch) {
            ch - b'0' + 52
        } else if ch == b'+' {
            62
        } else if ch == b'/' {
            63
        } else {
            continue; // Skip whitespace
        };

        buffer = (buffer << 6) | value as u32;
        bits += 6;

        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }

    Ok(output)
}

impl Default for MeshData {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Validation Implementation
// ============================================================================

impl AssetValidator for MeshData {
    /// Validate binary format (magic "MESH", version 1)
    fn validate_format(data: &[u8]) -> Result<(), ValidationError> {
        if data.len() < 16 {
            return Err(ValidationError::invalidvertexdata(
                "Data too small for header".to_string(),
            ));
        }

        // Check magic number
        if &data[0..4] != b"MESH" {
            let got = String::from_utf8_lossy(&data[0..4.min(data.len())]).to_string();
            return Err(ValidationError::invalidmagic("MESH".to_string(), got));
        }

        // Check version
        let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        if version > 1 {
            return Err(ValidationError::unsupportedversion(version, 1));
        }

        Ok(())
    }

    /// Validate mesh data integrity
    fn validate_data(&self) -> Result<(), ValidationError> {
        // Check for empty mesh
        if self.vertices.is_empty() {
            return Err(ValidationError::emptydata());
        }

        // Validate all vertices for NaN/Inf
        for (i, vertex) in self.vertices.iter().enumerate() {
            // Check position
            check_f32(vertex.position.x, &format!("vertex[{}].position.x", i))?;
            check_f32(vertex.position.y, &format!("vertex[{}].position.y", i))?;
            check_f32(vertex.position.z, &format!("vertex[{}].position.z", i))?;

            // Check normal
            check_f32(vertex.normal.x, &format!("vertex[{}].normal.x", i))?;
            check_f32(vertex.normal.y, &format!("vertex[{}].normal.y", i))?;
            check_f32(vertex.normal.z, &format!("vertex[{}].normal.z", i))?;

            // Check UV
            check_f32(vertex.uv.x, &format!("vertex[{}].uv.x", i))?;
            check_f32(vertex.uv.y, &format!("vertex[{}].uv.y", i))?;
        }

        // Validate indices are in bounds
        let vertex_count = self.vertices.len();
        for &index in self.indices.iter() {
            if index as usize >= vertex_count {
                return Err(ValidationError::indexoutofbounds(index, vertex_count));
            }
        }

        Ok(())
    }

    /// Validate checksum
    fn validate_checksum(&self, expected: &[u8; 32]) -> Result<(), ValidationError> {
        let actual = self.compute_checksum();
        if &actual != expected {
            return Err(ValidationError::checksummismatch(*expected, actual));
        }
        Ok(())
    }

    /// Compute Blake3 checksum of mesh data
    fn compute_checksum(&self) -> [u8; 32] {
        let binary = self.to_binary();
        compute_hash(&binary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_size() {
        assert_eq!(std::mem::size_of::<Vertex>(), 32); // 3*4 + 3*4 + 2*4
    }

    #[test]
    fn test_mesh_data_new() {
        let mesh = MeshData::new();
        assert_eq!(mesh.vertices.len(), 0);
        assert_eq!(mesh.indices.len(), 0);
    }

    #[test]
    fn test_mesh_data_cube() {
        let cube = MeshData::cube();
        assert_eq!(cube.vertex_count(), 24);
        assert_eq!(cube.index_count(), 36);
        assert_eq!(cube.triangle_count(), 12);
    }

    #[test]
    fn test_mesh_data_triangle() {
        let tri = MeshData::triangle();
        assert_eq!(tri.vertex_count(), 3);
        assert_eq!(tri.index_count(), 3);
        assert_eq!(tri.triangle_count(), 1);
    }

    #[test]
    fn test_obj_simple_triangle() {
        let obj = r#"
            v 0.0 -0.5 0.0
            v 0.5 0.5 0.0
            v -0.5 0.5 0.0
            f 1 2 3
        "#;

        let mesh = MeshData::from_obj(obj).unwrap();
        assert_eq!(mesh.vertex_count(), 3);
        assert_eq!(mesh.index_count(), 3);
    }

    #[test]
    fn test_obj_with_normals_and_uvs() {
        let obj = r#"
            v 0.0 0.0 0.0
            v 1.0 0.0 0.0
            v 0.0 1.0 0.0
            vn 0.0 0.0 1.0
            vt 0.0 0.0
            vt 1.0 0.0
            vt 0.0 1.0
            f 1/1/1 2/2/1 3/3/1
        "#;

        let mesh = MeshData::from_obj(obj).unwrap();
        assert_eq!(mesh.vertex_count(), 3);
        assert_eq!(mesh.vertices[0].normal, Vec3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn test_bounding_box() {
        let cube = MeshData::cube();
        let (min, max) = cube.bounding_box();
        assert_eq!(min, Vec3::new(-1.0, -1.0, -1.0));
        assert_eq!(max, Vec3::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn test_centroid() {
        let cube = MeshData::cube();
        let centroid = cube.centroid();
        // Cube is centered, so centroid should be near origin
        assert!((centroid - Vec3::ZERO).length() < 0.01);
    }

    // ========================================================================
    // Validation Tests
    // ========================================================================

    use crate::validation::{AssetValidator, ValidationError};

    #[test]
    fn test_valid_mesh_passes_all_validations() {
        let mesh = MeshData::cube();
        let report = mesh.validate_all();
        assert!(report.is_valid(), "Cube mesh should pass validation");
        assert!(report.errors.is_empty());
    }

    #[test]
    fn test_validate_format_valid_magic() {
        let mesh = MeshData::cube();
        let binary = mesh.to_binary();
        assert!(MeshData::validate_format(&binary).is_ok());
    }

    #[test]
    fn test_validate_format_invalid_magic() {
        let mut data = vec![0u8; 16];
        data[0..4].copy_from_slice(b"BADM"); // Wrong magic
        data[4..8].copy_from_slice(&1u32.to_le_bytes()); // Version 1

        let result = MeshData::validate_format(&data);
        assert!(result.is_err());
        match result {
            Err(ValidationError::InvalidMagic { expected, got }) => {
                assert_eq!(expected, "MESH");
                assert_eq!(got, "BADM");
            }
            _ => panic!("Expected InvalidMagic error"),
        }
    }

    #[test]
    fn test_validate_format_unsupported_version() {
        let mut data = vec![0u8; 16];
        data[0..4].copy_from_slice(b"MESH");
        data[4..8].copy_from_slice(&999u32.to_le_bytes()); // Version 999

        let result = MeshData::validate_format(&data);
        assert!(result.is_err());
        match result {
            Err(ValidationError::UnsupportedVersion { version, max_supported }) => {
                assert_eq!(version, 999);
                assert_eq!(max_supported, 1);
            }
            _ => panic!("Expected UnsupportedVersion error"),
        }
    }

    #[test]
    fn test_validate_data_nan_in_position() {
        let mut mesh = MeshData::triangle();
        mesh.vertices[0].position.x = f32::NAN;

        let result = mesh.validate_data();
        assert!(result.is_err());
        match result {
            Err(ValidationError::NaNDetected { field }) => {
                assert!(field.contains("position.x"));
            }
            _ => panic!("Expected NaNDetected error"),
        }
    }

    #[test]
    fn test_validate_data_infinity_in_normal() {
        let mut mesh = MeshData::triangle();
        mesh.vertices[1].normal.y = f32::INFINITY;

        let result = mesh.validate_data();
        assert!(result.is_err());
        match result {
            Err(ValidationError::InfinityDetected { field }) => {
                assert!(field.contains("normal.y"));
            }
            _ => panic!("Expected InfinityDetected error"),
        }
    }

    #[test]
    fn test_validate_data_neg_infinity_in_uv() {
        let mut mesh = MeshData::triangle();
        mesh.vertices[2].uv.x = f32::NEG_INFINITY;

        let result = mesh.validate_data();
        assert!(result.is_err());
        match result {
            Err(ValidationError::InfinityDetected { .. }) => {}
            _ => panic!("Expected InfinityDetected error"),
        }
    }

    #[test]
    fn test_validate_data_index_out_of_bounds() {
        let mut mesh = MeshData::triangle();
        mesh.indices.push(999); // Out of bounds

        let result = mesh.validate_data();
        assert!(result.is_err());
        match result {
            Err(ValidationError::IndexOutOfBounds { index, vertex_count }) => {
                assert_eq!(index, 999);
                assert_eq!(vertex_count, 3);
            }
            _ => panic!("Expected IndexOutOfBounds error"),
        }
    }

    #[test]
    fn test_validate_data_empty_mesh_fails() {
        let mesh = MeshData::new();
        let result = mesh.validate_data();
        assert!(result.is_err());
        match result {
            Err(ValidationError::EmptyData {}) => {}
            _ => panic!("Expected EmptyData error"),
        }
    }

    #[test]
    fn test_checksum_validation_passes() {
        let mesh = MeshData::cube();
        let checksum = mesh.compute_checksum();
        assert!(mesh.validate_checksum(&checksum).is_ok());
    }

    #[test]
    fn test_checksum_validation_fails() {
        let mesh = MeshData::cube();
        let wrong_checksum = [0u8; 32];
        let result = mesh.validate_checksum(&wrong_checksum);
        assert!(result.is_err());
        match result {
            Err(ValidationError::ChecksumMismatch { expected, actual }) => {
                assert_eq!(expected, wrong_checksum);
                assert_ne!(actual, wrong_checksum);
            }
            _ => panic!("Expected ChecksumMismatch error"),
        }
    }

    #[test]
    fn test_checksum_deterministic() {
        let mesh = MeshData::cube();
        let checksum1 = mesh.compute_checksum();
        let checksum2 = mesh.compute_checksum();
        assert_eq!(checksum1, checksum2);
    }

    #[test]
    fn test_validation_report_aggregation() {
        // Valid mesh
        let valid_mesh = MeshData::cube();
        let report = valid_mesh.validate_all();
        assert!(report.is_valid());
        assert_eq!(report.errors.len(), 0);

        // Invalid mesh (NaN)
        let mut invalid_mesh = MeshData::triangle();
        invalid_mesh.vertices[0].position.x = f32::NAN;
        let report = invalid_mesh.validate_all();
        assert!(!report.is_valid());
        assert!(!report.errors.is_empty());
    }
}
