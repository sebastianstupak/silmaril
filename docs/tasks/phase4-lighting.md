# Phase 4.3: Advanced Lighting System

**Status:** ⚪ Not Started
**Estimated Time:** 4-5 days
**Priority:** High (visual quality)

---

## 🎯 **Objective**

Implement advanced lighting system with directional, point, and spot lights. Add shadow mapping with cascaded shadow maps for directional lights and traditional shadow maps for point/spot lights.

**Must support:**
- Directional, point, and spot lights
- Shadow mapping (cascaded for directional)
- Light component for ECS
- Dynamic light creation/modification
- Performance: <2ms shadow map generation per frame

---

## 📋 **Detailed Tasks**

### **1. Light Data Structures** (Day 1)

**File:** `engine/renderer/src/lighting/light.rs`

```rust
use glam::{Vec3, Vec4, Mat4};

/// Light type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LightType {
    Directional,
    Point,
    Spot,
}

/// Light properties
#[derive(Debug, Clone)]
pub struct Light {
    pub light_type: LightType,
    pub color: Vec3,
    pub intensity: f32,

    // Position (point/spot)
    pub position: Vec3,

    // Direction (directional/spot)
    pub direction: Vec3,

    // Point light attenuation
    pub range: f32,

    // Spot light cone
    pub inner_cone_angle: f32, // radians
    pub outer_cone_angle: f32, // radians

    // Shadow properties
    pub cast_shadows: bool,
    pub shadow_map_size: u32,
    pub shadow_bias: f32,
    pub shadow_cascade_count: u32, // For directional lights
}

impl Light {
    /// Create directional light
    pub fn directional(direction: Vec3, color: Vec3, intensity: f32) -> Self {
        Self {
            light_type: LightType::Directional,
            color,
            intensity,
            position: Vec3::ZERO,
            direction: direction.normalize(),
            range: 0.0,
            inner_cone_angle: 0.0,
            outer_cone_angle: 0.0,
            cast_shadows: true,
            shadow_map_size: 2048,
            shadow_bias: 0.005,
            shadow_cascade_count: 4,
        }
    }

    /// Create point light
    pub fn point(position: Vec3, color: Vec3, intensity: f32, range: f32) -> Self {
        Self {
            light_type: LightType::Point,
            color,
            intensity,
            position,
            direction: Vec3::ZERO,
            range,
            inner_cone_angle: 0.0,
            outer_cone_angle: 0.0,
            cast_shadows: true,
            shadow_map_size: 1024,
            shadow_bias: 0.005,
            shadow_cascade_count: 0,
        }
    }

    /// Create spot light
    pub fn spot(
        position: Vec3,
        direction: Vec3,
        color: Vec3,
        intensity: f32,
        range: f32,
        inner_angle: f32,
        outer_angle: f32,
    ) -> Self {
        Self {
            light_type: LightType::Spot,
            color,
            intensity,
            position,
            direction: direction.normalize(),
            range,
            inner_cone_angle: inner_angle,
            outer_cone_angle: outer_angle,
            cast_shadows: true,
            shadow_map_size: 1024,
            shadow_bias: 0.005,
            shadow_cascade_count: 0,
        }
    }

    /// Calculate light attenuation (point/spot)
    pub fn calculate_attenuation(&self, distance: f32) -> f32 {
        match self.light_type {
            LightType::Point | LightType::Spot => {
                let attenuation = 1.0 / (distance * distance).max(0.01);
                let range_factor = (1.0 - (distance / self.range).powi(4)).max(0.0).powi(2);
                attenuation * range_factor
            }
            LightType::Directional => 1.0,
        }
    }

    /// Calculate spot light cone attenuation
    pub fn calculate_cone_attenuation(&self, light_dir: Vec3, surface_to_light: Vec3) -> f32 {
        if self.light_type != LightType::Spot {
            return 1.0;
        }

        let cos_angle = light_dir.dot(surface_to_light);
        let cos_inner = self.inner_cone_angle.cos();
        let cos_outer = self.outer_cone_angle.cos();

        ((cos_angle - cos_outer) / (cos_inner - cos_outer))
            .clamp(0.0, 1.0)
    }
}

/// Light handle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LightHandle(pub u32);

/// GPU light data (upload to shader)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GpuLightData {
    pub position: Vec4, // w = light type (0=directional, 1=point, 2=spot)
    pub direction: Vec4, // w = unused
    pub color: Vec4, // w = intensity
    pub params: Vec4, // x=range, y=inner_cone, z=outer_cone, w=shadow_index
}

impl From<&Light> for GpuLightData {
    fn from(light: &Light) -> Self {
        let light_type = match light.light_type {
            LightType::Directional => 0.0,
            LightType::Point => 1.0,
            LightType::Spot => 2.0,
        };

        Self {
            position: Vec4::new(light.position.x, light.position.y, light.position.z, light_type),
            direction: Vec4::new(light.direction.x, light.direction.y, light.direction.z, 0.0),
            color: Vec4::new(light.color.x, light.color.y, light.color.z, light.intensity),
            params: Vec4::new(
                light.range,
                light.inner_cone_angle.cos(),
                light.outer_cone_angle.cos(),
                -1.0, // Shadow index (set by shadow system)
            ),
        }
    }
}
```

---

### **2. Light Component** (Day 1)

**File:** `engine/ecs/src/components/light.rs`

```rust
use crate::component::Component;

/// Light component for entities
#[derive(Debug, Clone, Component)]
pub struct LightComponent {
    pub light: Light,
}

impl LightComponent {
    pub fn new(light: Light) -> Self {
        Self { light }
    }

    pub fn directional(direction: Vec3, color: Vec3, intensity: f32) -> Self {
        Self {
            light: Light::directional(direction, color, intensity),
        }
    }

    pub fn point(position: Vec3, color: Vec3, intensity: f32, range: f32) -> Self {
        Self {
            light: Light::point(position, color, intensity, range),
        }
    }

    pub fn spot(
        position: Vec3,
        direction: Vec3,
        color: Vec3,
        intensity: f32,
        range: f32,
        inner_angle: f32,
        outer_angle: f32,
    ) -> Self {
        Self {
            light: Light::spot(position, direction, color, intensity, range, inner_angle, outer_angle),
        }
    }
}

/// Light manager
pub struct LightManager {
    lights: Vec<Light>,
    handle_counter: u32,
}

impl LightManager {
    pub fn new() -> Self {
        Self {
            lights: Vec::new(),
            handle_counter: 0,
        }
    }

    /// Add light
    pub fn add_light(&mut self, light: Light) -> LightHandle {
        let handle = LightHandle(self.handle_counter);
        self.handle_counter += 1;

        self.lights.push(light);

        tracing::info!("Light added: {:?} ({:?})", handle, light.light_type);
        handle
    }

    /// Remove light
    pub fn remove_light(&mut self, handle: LightHandle) {
        if let Some(idx) = self.lights.iter().position(|_| true) {
            self.lights.remove(idx);
        }
    }

    /// Get light
    pub fn get_light(&self, handle: LightHandle) -> Option<&Light> {
        self.lights.get(handle.0 as usize)
    }

    /// Get light mut
    pub fn get_light_mut(&mut self, handle: LightHandle) -> Option<&mut Light> {
        self.lights.get_mut(handle.0 as usize)
    }

    /// Get all lights
    pub fn lights(&self) -> &[Light] {
        &self.lights
    }

    /// Get GPU light data for all lights
    pub fn get_gpu_light_data(&self) -> Vec<GpuLightData> {
        self.lights.iter().map(|light| light.into()).collect()
    }
}
```

---

### **3. Shadow Mapping** (Day 2-3)

**File:** `engine/renderer/src/lighting/shadow.rs`

```rust
use glam::{Mat4, Vec3};

/// Shadow map configuration
#[derive(Debug, Clone)]
pub struct ShadowMapConfig {
    pub size: u32,
    pub format: vk::Format,
    pub filter: ShadowFilter,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShadowFilter {
    Nearest,
    Pcf3x3,
    Pcf5x5,
}

impl Default for ShadowMapConfig {
    fn default() -> Self {
        Self {
            size: 2048,
            format: vk::Format::D32_SFLOAT,
            filter: ShadowFilter::Pcf3x3,
        }
    }
}

/// Cascaded shadow map for directional light
#[derive(Debug)]
pub struct CascadedShadowMap {
    pub cascade_count: u32,
    pub cascade_splits: Vec<f32>,
    pub cascade_matrices: Vec<Mat4>,
    pub shadow_maps: Vec<VulkanTexture>,
}

impl CascadedShadowMap {
    /// Create cascaded shadow map
    pub fn new(
        device: &VulkanDevice,
        allocator: &mut VulkanAllocator,
        config: &ShadowMapConfig,
        cascade_count: u32,
    ) -> Result<Self, RendererError> {
        let mut shadow_maps = Vec::new();

        for i in 0..cascade_count {
            let shadow_map = VulkanTexture::create_depth_texture(
                device,
                allocator,
                config.size,
                config.size,
                config.format,
            )?;

            shadow_maps.push(shadow_map);
        }

        Ok(Self {
            cascade_count,
            cascade_splits: Vec::new(),
            cascade_matrices: vec![Mat4::IDENTITY; cascade_count as usize],
            shadow_maps,
        })
    }

    /// Calculate cascade splits (exponential)
    pub fn calculate_cascade_splits(
        &mut self,
        near_plane: f32,
        far_plane: f32,
        lambda: f32, // 0.0 = uniform, 1.0 = exponential
    ) {
        self.cascade_splits.clear();
        self.cascade_splits.push(near_plane);

        for i in 1..self.cascade_count {
            let p = i as f32 / self.cascade_count as f32;

            // Exponential split
            let log_split = near_plane * (far_plane / near_plane).powf(p);

            // Uniform split
            let uniform_split = near_plane + (far_plane - near_plane) * p;

            // Blend
            let split = lambda * log_split + (1.0 - lambda) * uniform_split;

            self.cascade_splits.push(split);
        }

        self.cascade_splits.push(far_plane);
    }

    /// Update cascade matrices
    pub fn update_cascade_matrices(
        &mut self,
        view_matrix: &Mat4,
        projection_matrix: &Mat4,
        light_direction: Vec3,
    ) {
        let view_proj_inv = (projection_matrix * view_matrix).inverse();

        for i in 0..self.cascade_count as usize {
            let near = self.cascade_splits[i];
            let far = self.cascade_splits[i + 1];

            // Get frustum corners in world space
            let frustum_corners = Self::get_frustum_corners_world_space(
                &view_proj_inv,
                near,
                far,
            );

            // Calculate frustum center
            let mut frustum_center = Vec3::ZERO;
            for corner in &frustum_corners {
                frustum_center += *corner;
            }
            frustum_center /= frustum_corners.len() as f32;

            // Calculate light view matrix
            let light_view = Mat4::look_at_rh(
                frustum_center - light_direction * 50.0,
                frustum_center,
                Vec3::Y,
            );

            // Calculate AABB in light space
            let mut min = Vec3::splat(f32::MAX);
            let mut max = Vec3::splat(f32::MIN);

            for corner in &frustum_corners {
                let corner_light_space = light_view.transform_point3(*corner);
                min = min.min(corner_light_space);
                max = max.max(corner_light_space);
            }

            // Expand AABB to stabilize shadows
            let z_mult = 10.0;
            if min.z < 0.0 {
                min.z *= z_mult;
            } else {
                min.z /= z_mult;
            }
            if max.z < 0.0 {
                max.z /= z_mult;
            } else {
                max.z *= z_mult;
            }

            // Create orthographic projection
            let light_proj = Mat4::orthographic_rh(
                min.x,
                max.x,
                min.y,
                max.y,
                min.z,
                max.z,
            );

            self.cascade_matrices[i] = light_proj * light_view;
        }
    }

    /// Get frustum corners in world space
    fn get_frustum_corners_world_space(
        view_proj_inv: &Mat4,
        near: f32,
        far: f32,
    ) -> Vec<Vec3> {
        let mut corners = Vec::new();

        // NDC corners
        for x in &[-1.0, 1.0] {
            for y in &[-1.0, 1.0] {
                for z in &[0.0, 1.0] {
                    let ndc = glam::Vec4::new(*x, *y, *z, 1.0);
                    let world = view_proj_inv * ndc;
                    let world = world / world.w;
                    corners.push(world.xyz());
                }
            }
        }

        // Adjust for near/far planes
        let camera_pos = view_proj_inv.transform_point3(Vec3::ZERO);
        for corner in &mut corners {
            let dir = (*corner - camera_pos).normalize();
            let distance = if corner.z < 0.0 { near } else { far };
            *corner = camera_pos + dir * distance;
        }

        corners
    }
}

/// Point light shadow map (cubemap)
pub struct PointLightShadowMap {
    pub shadow_cubemap: VulkanTexture,
    pub face_matrices: [Mat4; 6],
}

impl PointLightShadowMap {
    /// Create point light shadow map
    pub fn new(
        device: &VulkanDevice,
        allocator: &mut VulkanAllocator,
        size: u32,
    ) -> Result<Self, RendererError> {
        let shadow_cubemap = VulkanTexture::create_depth_cubemap(
            device,
            allocator,
            size,
            vk::Format::D32_SFLOAT,
        )?;

        Ok(Self {
            shadow_cubemap,
            face_matrices: [Mat4::IDENTITY; 6],
        })
    }

    /// Update face matrices
    pub fn update_face_matrices(&mut self, light_position: Vec3, far_plane: f32) {
        let proj = Mat4::perspective_rh(std::f32::consts::FRAC_PI_2, 1.0, 0.1, far_plane);

        let directions = [
            (Vec3::X, Vec3::NEG_Y),   // +X
            (Vec3::NEG_X, Vec3::NEG_Y), // -X
            (Vec3::Y, Vec3::Z),       // +Y
            (Vec3::NEG_Y, Vec3::NEG_Z), // -Y
            (Vec3::Z, Vec3::NEG_Y),   // +Z
            (Vec3::NEG_Z, Vec3::NEG_Y), // -Z
        ];

        for (i, (dir, up)) in directions.iter().enumerate() {
            let view = Mat4::look_at_rh(light_position, light_position + *dir, *up);
            self.face_matrices[i] = proj * view;
        }
    }
}

/// Spot light shadow map
pub struct SpotLightShadowMap {
    pub shadow_map: VulkanTexture,
    pub view_proj_matrix: Mat4,
}

impl SpotLightShadowMap {
    /// Create spot light shadow map
    pub fn new(
        device: &VulkanDevice,
        allocator: &mut VulkanAllocator,
        size: u32,
    ) -> Result<Self, RendererError> {
        let shadow_map = VulkanTexture::create_depth_texture(
            device,
            allocator,
            size,
            size,
            vk::Format::D32_SFLOAT,
        )?;

        Ok(Self {
            shadow_map,
            view_proj_matrix: Mat4::IDENTITY,
        })
    }

    /// Update view-projection matrix
    pub fn update_matrix(
        &mut self,
        light_position: Vec3,
        light_direction: Vec3,
        outer_cone_angle: f32,
        range: f32,
    ) {
        let view = Mat4::look_at_rh(
            light_position,
            light_position + light_direction,
            Vec3::Y,
        );

        let fov = outer_cone_angle * 2.0;
        let proj = Mat4::perspective_rh(fov, 1.0, 0.1, range);

        self.view_proj_matrix = proj * view;
    }
}
```

---

### **4. Shadow Rendering Shader** (Day 3-4)

**File:** `engine/renderer/shaders/shadow.frag`

```glsl
#version 450

// Shadow map sampling
layout(set = 0, binding = 0) uniform sampler2D shadow_map;
layout(set = 0, binding = 1) uniform samplerCube shadow_cubemap;

layout(set = 1, binding = 0) uniform ShadowUniforms {
    mat4 cascade_matrices[4];
    vec4 cascade_splits;
    uint cascade_count;
    uint shadow_filter; // 0=nearest, 1=PCF3x3, 2=PCF5x5
    float bias;
} shadow_uniforms;

// PCF filtering
float pcf_filter_3x3(sampler2D shadow_map, vec2 uv, float depth) {
    float shadow = 0.0;
    vec2 texel_size = 1.0 / textureSize(shadow_map, 0);

    for (int x = -1; x <= 1; x++) {
        for (int y = -1; y <= 1; y++) {
            vec2 offset = vec2(x, y) * texel_size;
            float pcf_depth = texture(shadow_map, uv + offset).r;
            shadow += depth > pcf_depth + shadow_uniforms.bias ? 1.0 : 0.0;
        }
    }

    return shadow / 9.0;
}

float pcf_filter_5x5(sampler2D shadow_map, vec2 uv, float depth) {
    float shadow = 0.0;
    vec2 texel_size = 1.0 / textureSize(shadow_map, 0);

    for (int x = -2; x <= 2; x++) {
        for (int y = -2; y <= 2; y++) {
            vec2 offset = vec2(x, y) * texel_size;
            float pcf_depth = texture(shadow_map, uv + offset).r;
            shadow += depth > pcf_depth + shadow_uniforms.bias ? 1.0 : 0.0;
        }
    }

    return shadow / 25.0;
}

// Calculate shadow for directional light
float calculate_directional_shadow(vec3 world_pos, float view_depth) {
    // Find cascade index
    uint cascade_index = 0;
    for (uint i = 0; i < shadow_uniforms.cascade_count - 1; i++) {
        if (view_depth > shadow_uniforms.cascade_splits[i]) {
            cascade_index = i + 1;
        }
    }

    // Transform to light space
    vec4 light_space_pos = shadow_uniforms.cascade_matrices[cascade_index] * vec4(world_pos, 1.0);
    vec3 proj_coords = light_space_pos.xyz / light_space_pos.w;

    // Transform to [0, 1]
    proj_coords = proj_coords * 0.5 + 0.5;

    // Outside shadow map
    if (proj_coords.x < 0.0 || proj_coords.x > 1.0 ||
        proj_coords.y < 0.0 || proj_coords.y > 1.0 ||
        proj_coords.z > 1.0) {
        return 0.0;
    }

    // Sample shadow map
    if (shadow_uniforms.shadow_filter == 1) {
        return pcf_filter_3x3(shadow_map, proj_coords.xy, proj_coords.z);
    } else if (shadow_uniforms.shadow_filter == 2) {
        return pcf_filter_5x5(shadow_map, proj_coords.xy, proj_coords.z);
    } else {
        float shadow_depth = texture(shadow_map, proj_coords.xy).r;
        return proj_coords.z > shadow_depth + shadow_uniforms.bias ? 1.0 : 0.0;
    }
}

// Calculate shadow for point light
float calculate_point_shadow(vec3 world_pos, vec3 light_pos, float far_plane) {
    vec3 frag_to_light = world_pos - light_pos;
    float current_depth = length(frag_to_light);

    float shadow_depth = texture(shadow_cubemap, frag_to_light).r * far_plane;
    return current_depth > shadow_depth + shadow_uniforms.bias ? 1.0 : 0.0;
}
```

---

### **5. Lighting System** (Day 5)

**File:** `engine/renderer/src/systems/lighting_system.rs`

```rust
/// Lighting system
pub struct LightingSystem {
    light_manager: LightManager,
    cascaded_shadow_maps: Vec<CascadedShadowMap>,
    point_shadow_maps: Vec<PointLightShadowMap>,
    spot_shadow_maps: Vec<SpotLightShadowMap>,
}

impl LightingSystem {
    pub fn new() -> Self {
        Self {
            light_manager: LightManager::new(),
            cascaded_shadow_maps: Vec::new(),
            point_shadow_maps: Vec::new(),
            spot_shadow_maps: Vec::new(),
        }
    }

    /// Update shadow maps
    pub fn update_shadow_maps(
        &mut self,
        view_matrix: &Mat4,
        projection_matrix: &Mat4,
        near_plane: f32,
        far_plane: f32,
    ) {
        // Update directional light cascades
        for (light, csm) in self.light_manager.lights().iter().zip(&mut self.cascaded_shadow_maps) {
            if light.light_type == LightType::Directional && light.cast_shadows {
                csm.calculate_cascade_splits(near_plane, far_plane, 0.9);
                csm.update_cascade_matrices(view_matrix, projection_matrix, light.direction);
            }
        }

        // Update point light shadow maps
        for (light, psm) in self.light_manager.lights().iter().zip(&mut self.point_shadow_maps) {
            if light.light_type == LightType::Point && light.cast_shadows {
                psm.update_face_matrices(light.position, light.range);
            }
        }

        // Update spot light shadow maps
        for (light, ssm) in self.light_manager.lights().iter().zip(&mut self.spot_shadow_maps) {
            if light.light_type == LightType::Spot && light.cast_shadows {
                ssm.update_matrix(
                    light.position,
                    light.direction,
                    light.outer_cone_angle,
                    light.range,
                );
            }
        }
    }

    /// Render shadow maps
    pub fn render_shadow_maps(
        &self,
        device: &VulkanDevice,
        command_buffer: vk::CommandBuffer,
        // ... scene geometry
    ) -> Result<(), RendererError> {
        // Render each shadow map
        // TODO: Implementation
        Ok(())
    }
}
```

---

## ✅ **Acceptance Criteria**

- [ ] Directional, point, and spot light types
- [ ] Light component for ECS
- [ ] Light manager with add/remove/get
- [ ] Cascaded shadow maps for directional lights (4 cascades)
- [ ] Cubemap shadow maps for point lights
- [ ] Traditional shadow maps for spot lights
- [ ] PCF filtering (3x3, 5x5)
- [ ] Shadow bias configuration
- [ ] Performance: <2ms shadow map generation
- [ ] Smooth cascade transitions

---

## 🧪 **Tests**

```rust
#[test]
fn test_light_creation() {
    let light = Light::directional(Vec3::new(0.0, -1.0, 0.0), Vec3::ONE, 1.0);
    assert_eq!(light.light_type, LightType::Directional);
}

#[test]
fn test_light_attenuation() {
    let light = Light::point(Vec3::ZERO, Vec3::ONE, 1.0, 10.0);
    let attenuation = light.calculate_attenuation(5.0);
    assert!(attenuation > 0.0 && attenuation < 1.0);
}

#[test]
fn test_cascade_splits() {
    let mut csm = CascadedShadowMap::new(/* ... */, 4).unwrap();
    csm.calculate_cascade_splits(0.1, 100.0, 0.9);

    assert_eq!(csm.cascade_splits.len(), 5); // 4 cascades + far plane
    assert!(csm.cascade_splits[0] < csm.cascade_splits[4]);
}
```

---

## ⚡ **Performance Targets**

- **Shadow Map Generation:** <2ms per frame (all lights)
- **Cascade Updates:** <0.5ms per frame
- **Memory Usage:** ~16 MB per 2K shadow map
- **Light Count:** 100+ point lights at 60 FPS
- **Shadow Filter:** PCF 3x3 with <1ms overhead

---

**Dependencies:** [phase4-pbr-materials.md](phase4-pbr-materials.md)
**Next:** [phase4-profiling-integration.md](phase4-profiling-integration.md)
