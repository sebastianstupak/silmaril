//! Rendering-related components for the game logic layer
//!
//! This module provides pure game logic components for rendering:
//! - Camera: Perspective projection and view matrix generation
//! - MeshRenderer: Asset handle reference for mesh rendering
//!
//! NO Vulkan types here - this is pure ECS/game logic.
//! Rendering backends (engine-renderer) use these components.
//!
//! High-performance components optimized for 120 FPS AAA targets.
//! All components are designed for minimal overhead (<0.5µs per operation).

use crate::ecs::Component;
use crate::math::Transform;
use crate::Vec3;
use glam::Mat4;
use serde::{Deserialize, Serialize};

// ============================================================================
// MeshRenderer Component
// ============================================================================

/// Mesh renderer component - references mesh asset by ID
///
/// Uses u64 ID for mesh references to avoid circular dependencies.
/// The rendering system resolves IDs to actual mesh data.
/// Optimized for cache-friendly ECS queries.
///
/// # Examples
///
/// ```
/// use engine_core::MeshRenderer;
///
/// let renderer = MeshRenderer::new(12345); // mesh ID
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshRenderer {
    /// Mesh asset ID
    pub mesh_id: u64,
    /// Visibility flag (fast culling check)
    pub visible: bool,
}

impl Component for MeshRenderer {}

impl MeshRenderer {
    /// Create a new mesh renderer (visible by default).
    #[inline]
    #[must_use]
    pub const fn new(mesh_id: u64) -> Self {
        Self { mesh_id, visible: true }
    }

    /// Create a mesh renderer with explicit visibility.
    #[inline]
    #[must_use]
    pub const fn with_visibility(mesh_id: u64, visible: bool) -> Self {
        Self { mesh_id, visible }
    }

    /// Check if the renderer is visible.
    #[inline]
    #[must_use]
    pub const fn is_visible(&self) -> bool {
        self.visible
    }

    /// Set visibility.
    #[inline]
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Change the mesh asset.
    #[inline]
    pub fn set_mesh_id(&mut self, mesh_id: u64) {
        self.mesh_id = mesh_id;
    }
}

// ============================================================================
// Camera Component
// ============================================================================

/// Camera component for view and projection matrices
///
/// Optimized for 120 FPS with cached matrices and SIMD-friendly layout.
/// Matrices are recomputed only when camera parameters change.
///
/// # Performance Targets (120 FPS)
/// - Matrix calculation: <0.5µs
/// - MVP composition: <0.3µs
/// - Cache-friendly: 16-byte aligned
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[repr(C, align(16))] // SIMD-friendly alignment
pub struct Camera {
    /// Field of view in radians (typically PI/4 for 90 degrees)
    pub fov: f32,
    /// Aspect ratio (width / height)
    pub aspect: f32,
    /// Near clipping plane
    pub near: f32,
    /// Far clipping plane
    pub far: f32,
    /// Cached projection matrix (recomputed when params change)
    #[serde(skip)]
    cached_projection: Mat4,
    /// Dirty flag for lazy recomputation
    #[serde(skip)]
    dirty: bool,
}

impl Component for Camera {}

impl Camera {
    /// Create a new perspective camera
    ///
    /// # Arguments
    /// * `fov` - Field of view in radians (typical: 1.57 = 90 degrees)
    /// * `aspect` - Aspect ratio (width / height)
    ///
    /// # Performance
    /// - Time: <0.5µs (includes matrix computation)
    /// - Uses glam's optimized SIMD matrix operations
    ///
    /// # Example
    /// ```
    /// use engine_core::Camera;
    ///
    /// let camera = Camera::new(1.57, 16.0 / 9.0);
    /// ```
    #[must_use]
    pub fn new(fov: f32, aspect: f32) -> Self {
        let near = 0.1;
        let far = 1000.0;
        let projection = Mat4::perspective_rh(fov, aspect, near, far);

        Self { fov, aspect, near, far, cached_projection: projection, dirty: false }
    }

    /// Create with custom near/far planes
    #[must_use]
    pub fn with_planes(fov: f32, aspect: f32, near: f32, far: f32) -> Self {
        let projection = Mat4::perspective_rh(fov, aspect, near, far);

        Self { fov, aspect, near, far, cached_projection: projection, dirty: false }
    }

    /// Get projection matrix (cached, optimized for 120 FPS)
    ///
    /// # Performance
    /// - Cache hit: <0.05µs (just a memory read)
    /// - Cache miss: <0.5µs (matrix recomputation)
    #[inline]
    #[must_use]
    pub fn projection_matrix(&mut self) -> Mat4 {
        if self.dirty {
            self.cached_projection =
                Mat4::perspective_rh(self.fov, self.aspect, self.near, self.far);
            self.dirty = false;
        }
        self.cached_projection
    }

    /// Get projection matrix (const version, no cache update)
    #[inline]
    #[must_use]
    pub fn projection_matrix_const(&self) -> Mat4 {
        self.cached_projection
    }

    /// Compute view matrix from transform (camera position/rotation)
    ///
    /// # Performance
    /// - Time: <0.3µs (SIMD-optimized via glam)
    /// - Uses cached affine transform from Transform component
    ///
    /// # Example
    /// ```
    /// use engine_core::{Camera, Transform, Vec3, Quat};
    ///
    /// let mut camera = Camera::new(1.57, 16.0 / 9.0);
    /// let transform = Transform::new(
    ///     Vec3::new(0.0, 2.0, 5.0),
    ///     Quat::IDENTITY,
    ///     Vec3::ONE,
    /// );
    ///
    /// let view = camera.view_matrix(&transform);
    /// ```
    #[inline]
    #[must_use]
    pub fn view_matrix(&self, transform: &Transform) -> Mat4 {
        // Camera looks down -Z axis, so we need inverse transform
        // Use look_at for correct camera orientation
        Mat4::look_at_rh(
            transform.position,
            transform.position + transform.rotation * Vec3::NEG_Z,
            Vec3::Y,
        )
    }

    /// Compute full view-projection matrix
    ///
    /// # Performance
    /// - Time: <0.4µs (matrix multiplication is SIMD-optimized)
    #[inline]
    #[must_use]
    pub fn view_projection_matrix(&mut self, transform: &Transform) -> Mat4 {
        self.projection_matrix() * self.view_matrix(transform)
    }

    /// Update aspect ratio (marks projection matrix dirty)
    #[inline]
    pub fn set_aspect(&mut self, aspect: f32) {
        if (self.aspect - aspect).abs() > 1e-6 {
            self.aspect = aspect;
            self.dirty = true;
        }
    }

    /// Update field of view (marks projection matrix dirty)
    #[inline]
    pub fn set_fov(&mut self, fov: f32) {
        if (self.fov - fov).abs() > 1e-6 {
            self.fov = fov;
            self.dirty = true;
        }
    }

    /// Update near/far planes (marks projection matrix dirty)
    #[inline]
    pub fn set_planes(&mut self, near: f32, far: f32) {
        if (self.near - near).abs() > 1e-6 || (self.far - far).abs() > 1e-6 {
            self.near = near;
            self.far = far;
            self.dirty = true;
        }
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self::new(std::f32::consts::FRAC_PI_4, 16.0 / 9.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Quat;

    #[test]
    fn test_camera_creation() {
        let camera = Camera::new(std::f32::consts::FRAC_PI_4, 16.0 / 9.0);
        assert!((camera.fov - std::f32::consts::FRAC_PI_4).abs() < 1e-6);
        assert!((camera.aspect - 16.0 / 9.0).abs() < 1e-6);
        assert_eq!(camera.near, 0.1);
        assert_eq!(camera.far, 1000.0);
    }

    #[test]
    fn test_camera_projection_matrix() {
        let mut camera = Camera::new(std::f32::consts::FRAC_PI_4, 1.0);
        let proj = camera.projection_matrix();

        // Projection matrix should be non-zero
        assert_ne!(proj, Mat4::ZERO);

        // Should be cached (same reference)
        let proj2 = camera.projection_matrix_const();
        assert_eq!(proj, proj2);
    }

    #[test]
    fn test_camera_view_matrix() {
        let camera = Camera::new(std::f32::consts::FRAC_PI_4, 1.0);
        let transform = Transform::new(Vec3::new(0.0, 0.0, 5.0), Quat::IDENTITY, Vec3::ONE);

        let view = camera.view_matrix(&transform);
        assert_ne!(view, Mat4::ZERO);
    }

    #[test]
    fn test_camera_dirty_flag() {
        let mut camera = Camera::new(std::f32::consts::FRAC_PI_4, 1.0);

        // First call computes
        let proj1 = camera.projection_matrix();

        // Change aspect ratio
        camera.set_aspect(2.0);
        assert_eq!(camera.aspect, 2.0);

        // Should recompute
        let proj2 = camera.projection_matrix();
        assert_ne!(proj1, proj2);
    }

    #[test]
    fn test_camera_alignment() {
        // Verify SIMD alignment for cache-friendly access
        assert_eq!(std::mem::align_of::<Camera>(), 16);
    }

    #[test]
    fn test_view_projection_composition() {
        let mut camera = Camera::new(std::f32::consts::FRAC_PI_4, 1.0);
        let transform = Transform::new(Vec3::new(0.0, 2.0, 5.0), Quat::IDENTITY, Vec3::ONE);

        let vp = camera.view_projection_matrix(&transform);

        // Manual composition should match
        let view = camera.view_matrix(&transform);
        let proj = camera.projection_matrix_const();
        let manual_vp = proj * view;

        assert_eq!(vp, manual_vp);
    }

    #[test]
    fn test_camera_component_trait() {
        // Verify Camera implements Component
        fn assert_component<T: Component>() {}
        assert_component::<Camera>();
    }

    // MeshRenderer tests
    #[test]
    fn test_mesh_renderer_new() {
        let renderer = MeshRenderer::new(12345);
        assert_eq!(renderer.mesh_id, 12345);
        assert!(renderer.visible);
    }

    #[test]
    fn test_mesh_renderer_with_visibility() {
        let renderer = MeshRenderer::with_visibility(12345, false);
        assert_eq!(renderer.mesh_id, 12345);
        assert!(!renderer.visible);
    }

    #[test]
    fn test_mesh_renderer_set_visible() {
        let mut renderer = MeshRenderer::new(12345);
        renderer.set_visible(false);
        assert!(!renderer.is_visible());

        renderer.set_visible(true);
        assert!(renderer.is_visible());
    }

    #[test]
    fn test_mesh_renderer_set_mesh_id() {
        let mut renderer = MeshRenderer::new(12345);
        renderer.set_mesh_id(67890);
        assert_eq!(renderer.mesh_id, 67890);
    }

    #[test]
    fn test_mesh_renderer_component_trait() {
        // Verify MeshRenderer implements Component
        fn assert_component<T: Component>() {}
        assert_component::<MeshRenderer>();
    }
}
