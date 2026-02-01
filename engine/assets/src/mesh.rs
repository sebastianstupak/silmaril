//! Mesh data structures (pure data, no GPU/rendering dependencies)
//!
//! Can be used by:
//! - Server for procedural generation
//! - Physics for collision detection
//! - Tools for asset processing
//! - Client for rendering (via engine-renderer)

use engine_core::{EngineError, ErrorCode, ErrorSeverity};
use engine_macros::define_error;
use glam::{Vec2, Vec3};
use tracing::{info, instrument};

define_error! {
    pub enum MeshError {
        InvalidObjFormat { reason: String } = ErrorCode::MeshLoadFailed, ErrorSeverity::Error,
        InvalidVertexData { reason: String } = ErrorCode::MeshLoadFailed, ErrorSeverity::Error,
    }
}

/// Vertex data with position, normal, and UV coordinates
///
/// This is a pure data structure - rendering backends define their own
/// vertex input layouts based on this data.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
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
#[derive(Debug, Clone)]
pub struct MeshData {
    /// Vertex data
    pub vertices: Vec<Vertex>,
    /// Index data (triangles)
    pub indices: Vec<u32>,
}

impl MeshData {
    /// Create an empty mesh
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
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
                        let normal = norm_idx
                            .and_then(|idx| normals.get(idx).copied())
                            .unwrap_or(Vec3::Z);

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
}

impl Default for MeshData {
    fn default() -> Self {
        Self::new()
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
}
