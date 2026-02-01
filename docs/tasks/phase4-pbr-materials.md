# Phase 4.2: PBR Materials System

**Status:** ⚪ Not Started
**Estimated Time:** 4-5 days
**Priority:** High (visual quality)

---

## 🎯 **Objective**

Implement physically-based rendering (PBR) material system with metallic-roughness workflow. Support albedo, normal, metallic, roughness, and ambient occlusion texture maps.

**Must support:**
- PBR metallic-roughness workflow
- Multi-texture loading (albedo, normal, metallic, roughness, AO)
- Material component for ECS
- Shader updates for PBR lighting
- Material library system

---

## 📋 **Detailed Tasks**

### **1. Material Data Structures** (Day 1)

**File:** `engine/renderer/src/material.rs`

```rust
use glam::{Vec3, Vec4};
use std::sync::Arc;

/// PBR material properties
#[derive(Debug, Clone)]
pub struct PbrMaterial {
    pub name: String,

    // Base color
    pub albedo: Vec4, // RGB + alpha
    pub albedo_texture: Option<TextureHandle>,

    // Normal mapping
    pub normal_texture: Option<TextureHandle>,
    pub normal_scale: f32,

    // Metallic-roughness
    pub metallic: f32,
    pub roughness: f32,
    pub metallic_roughness_texture: Option<TextureHandle>, // R=unused, G=roughness, B=metallic

    // Ambient occlusion
    pub ao_texture: Option<TextureHandle>,
    pub ao_strength: f32,

    // Emissive
    pub emissive: Vec3,
    pub emissive_texture: Option<TextureHandle>,
    pub emissive_strength: f32,

    // Flags
    pub double_sided: bool,
    pub alpha_mode: AlphaMode,
    pub alpha_cutoff: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlphaMode {
    Opaque,
    Mask,
    Blend,
}

impl Default for PbrMaterial {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            albedo: Vec4::new(1.0, 1.0, 1.0, 1.0),
            albedo_texture: None,
            normal_texture: None,
            normal_scale: 1.0,
            metallic: 0.0,
            roughness: 1.0,
            metallic_roughness_texture: None,
            ao_texture: None,
            ao_strength: 1.0,
            emissive: Vec3::ZERO,
            emissive_texture: None,
            emissive_strength: 1.0,
            double_sided: false,
            alpha_mode: AlphaMode::Opaque,
            alpha_cutoff: 0.5,
        }
    }
}

impl PbrMaterial {
    /// Create new material
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Set albedo color
    pub fn with_albedo(mut self, color: Vec4) -> Self {
        self.albedo = color;
        self
    }

    /// Set albedo texture
    pub fn with_albedo_texture(mut self, texture: TextureHandle) -> Self {
        self.albedo_texture = Some(texture);
        self
    }

    /// Set normal texture
    pub fn with_normal_texture(mut self, texture: TextureHandle) -> Self {
        self.normal_texture = Some(texture);
        self
    }

    /// Set metallic and roughness
    pub fn with_metallic_roughness(mut self, metallic: f32, roughness: f32) -> Self {
        self.metallic = metallic;
        self.roughness = roughness;
        self
    }

    /// Set metallic-roughness texture
    pub fn with_metallic_roughness_texture(mut self, texture: TextureHandle) -> Self {
        self.metallic_roughness_texture = Some(texture);
        self
    }

    /// Set AO texture
    pub fn with_ao_texture(mut self, texture: TextureHandle) -> Self {
        self.ao_texture = Some(texture);
        self
    }

    /// Set emissive properties
    pub fn with_emissive(mut self, color: Vec3, strength: f32) -> Self {
        self.emissive = color;
        self.emissive_strength = strength;
        self
    }
}

/// Material handle (GPU resource)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialHandle(pub u32);

/// Texture handle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle(pub u32);
```

---

### **2. Texture Loading System** (Day 1-2)

**File:** `engine/renderer/src/texture.rs`

```rust
use image::{DynamicImage, GenericImageView, ImageFormat};
use std::path::Path;

/// Texture descriptor
#[derive(Debug, Clone)]
pub struct TextureDescriptor {
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub mip_levels: u32,
    pub usage: TextureUsage,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextureFormat {
    R8Unorm,
    Rg8Unorm,
    Rgba8Srgb,
    Rgba8Unorm,
    Bc7Srgb, // Compressed
    Bc7Unorm,
}

bitflags::bitflags! {
    pub struct TextureUsage: u32 {
        const SAMPLED = 1 << 0;
        const STORAGE = 1 << 1;
        const TRANSFER_DST = 1 << 2;
        const TRANSFER_SRC = 1 << 3;
    }
}

/// Texture data
pub struct TextureData {
    pub descriptor: TextureDescriptor,
    pub data: Vec<u8>,
}

impl TextureData {
    /// Load texture from file
    pub fn load_from_file(path: &Path, format: TextureFormat) -> Result<Self, RendererError> {
        let img = image::open(path).map_err(|e| RendererError::TextureLoadFailed {
            details: format!("Failed to load image: {}", e),
        })?;

        Self::from_image(img, format)
    }

    /// Create texture from image
    pub fn from_image(img: DynamicImage, format: TextureFormat) -> Result<Self, RendererError> {
        let (width, height) = img.dimensions();

        // Convert to RGBA8
        let rgba = img.to_rgba8();
        let data = rgba.into_raw();

        // Calculate mip levels
        let mip_levels = (width.max(height) as f32).log2().floor() as u32 + 1;

        Ok(Self {
            descriptor: TextureDescriptor {
                width,
                height,
                format,
                mip_levels,
                usage: TextureUsage::SAMPLED | TextureUsage::TRANSFER_DST,
            },
            data,
        })
    }

    /// Generate mipmaps (simple box filter)
    pub fn generate_mipmaps(&mut self) {
        let mut mip_data = Vec::new();
        let mut current_width = self.descriptor.width;
        let mut current_height = self.descriptor.height;
        let mut current_data = self.data.clone();

        // Add base mip level
        mip_data.extend_from_slice(&current_data);

        // Generate each mip level
        for _ in 1..self.descriptor.mip_levels {
            let next_width = (current_width / 2).max(1);
            let next_height = (current_height / 2).max(1);

            let next_data = Self::downsample_2x(
                &current_data,
                current_width,
                current_height,
                next_width,
                next_height,
            );

            mip_data.extend_from_slice(&next_data);

            current_width = next_width;
            current_height = next_height;
            current_data = next_data;
        }

        self.data = mip_data;
    }

    /// Downsample image by 2x (simple box filter)
    fn downsample_2x(
        data: &[u8],
        src_width: u32,
        src_height: u32,
        dst_width: u32,
        dst_height: u32,
    ) -> Vec<u8> {
        let mut result = vec![0u8; (dst_width * dst_height * 4) as usize];

        for y in 0..dst_height {
            for x in 0..dst_width {
                let src_x = x * 2;
                let src_y = y * 2;

                // Sample 4 pixels and average
                let mut r = 0u32;
                let mut g = 0u32;
                let mut b = 0u32;
                let mut a = 0u32;
                let mut count = 0u32;

                for dy in 0..2 {
                    for dx in 0..2 {
                        let sx = (src_x + dx).min(src_width - 1);
                        let sy = (src_y + dy).min(src_height - 1);
                        let idx = ((sy * src_width + sx) * 4) as usize;

                        r += data[idx] as u32;
                        g += data[idx + 1] as u32;
                        b += data[idx + 2] as u32;
                        a += data[idx + 3] as u32;
                        count += 1;
                    }
                }

                let dst_idx = ((y * dst_width + x) * 4) as usize;
                result[dst_idx] = (r / count) as u8;
                result[dst_idx + 1] = (g / count) as u8;
                result[dst_idx + 2] = (b / count) as u8;
                result[dst_idx + 3] = (a / count) as u8;
            }
        }

        result
    }
}

/// Texture manager
pub struct TextureManager {
    textures: Vec<VulkanTexture>,
    handle_counter: u32,
}

impl TextureManager {
    pub fn new() -> Self {
        Self {
            textures: Vec::new(),
            handle_counter: 0,
        }
    }

    /// Create texture from data
    pub fn create_texture(
        &mut self,
        device: &VulkanDevice,
        allocator: &mut VulkanAllocator,
        texture_data: TextureData,
    ) -> Result<TextureHandle, RendererError> {
        let texture = VulkanTexture::new(device, allocator, texture_data)?;

        let handle = TextureHandle(self.handle_counter);
        self.handle_counter += 1;

        self.textures.push(texture);

        tracing::info!("Texture created: {:?}", handle);
        Ok(handle)
    }

    /// Load texture from file
    pub fn load_texture(
        &mut self,
        device: &VulkanDevice,
        allocator: &mut VulkanAllocator,
        path: &Path,
        format: TextureFormat,
    ) -> Result<TextureHandle, RendererError> {
        let mut texture_data = TextureData::load_from_file(path, format)?;
        texture_data.generate_mipmaps();

        self.create_texture(device, allocator, texture_data)
    }

    pub fn get_texture(&self, handle: TextureHandle) -> Option<&VulkanTexture> {
        self.textures.get(handle.0 as usize)
    }
}
```

---

### **3. Material Component** (Day 2)

**File:** `engine/ecs/src/components/material.rs`

```rust
use crate::component::Component;

/// Material component for entities
#[derive(Debug, Clone, Component)]
pub struct MaterialComponent {
    pub material: MaterialHandle,
}

impl MaterialComponent {
    pub fn new(material: MaterialHandle) -> Self {
        Self { material }
    }
}

/// Material library
pub struct MaterialLibrary {
    materials: Vec<PbrMaterial>,
    handle_counter: u32,
}

impl MaterialLibrary {
    pub fn new() -> Self {
        Self {
            materials: Vec::new(),
            handle_counter: 0,
        }
    }

    /// Add material
    pub fn add_material(&mut self, material: PbrMaterial) -> MaterialHandle {
        let handle = MaterialHandle(self.handle_counter);
        self.handle_counter += 1;

        self.materials.push(material);

        tracing::info!("Material added: {:?} ({})", handle, self.materials.last().unwrap().name);
        handle
    }

    /// Get material
    pub fn get_material(&self, handle: MaterialHandle) -> Option<&PbrMaterial> {
        self.materials.get(handle.0 as usize)
    }

    /// Get material mut
    pub fn get_material_mut(&mut self, handle: MaterialHandle) -> Option<&mut PbrMaterial> {
        self.materials.get_mut(handle.0 as usize)
    }

    /// Load material from glTF format
    pub fn load_from_gltf(&mut self, path: &Path) -> Result<Vec<MaterialHandle>, RendererError> {
        // Parse glTF and extract materials
        let (document, buffers, _) = gltf::import(path).map_err(|e| RendererError::MaterialLoadFailed {
            details: format!("Failed to load glTF: {}", e),
        })?;

        let mut handles = Vec::new();

        for material in document.materials() {
            let pbr = material.pbr_metallic_roughness();

            let mut mat = PbrMaterial::new(material.name().unwrap_or("Unnamed"));

            // Base color
            let base_color = pbr.base_color_factor();
            mat.albedo = Vec4::from_array(base_color);

            // Metallic/roughness
            mat.metallic = pbr.metallic_factor();
            mat.roughness = pbr.roughness_factor();

            // Normal scale
            if let Some(normal) = material.normal_texture() {
                mat.normal_scale = normal.scale();
            }

            // Emissive
            mat.emissive = Vec3::from_array(material.emissive_factor());

            // Alpha mode
            mat.alpha_mode = match material.alpha_mode() {
                gltf::material::AlphaMode::Opaque => AlphaMode::Opaque,
                gltf::material::AlphaMode::Mask => AlphaMode::Mask,
                gltf::material::AlphaMode::Blend => AlphaMode::Blend,
            };

            mat.alpha_cutoff = material.alpha_cutoff().unwrap_or(0.5);
            mat.double_sided = material.double_sided();

            let handle = self.add_material(mat);
            handles.push(handle);
        }

        Ok(handles)
    }
}
```

---

### **4. PBR Shader** (Day 3-4)

**File:** `engine/renderer/shaders/pbr.frag`

```glsl
#version 450

// Inputs
layout(location = 0) in vec3 in_position;
layout(location = 1) in vec3 in_normal;
layout(location = 2) in vec2 in_uv;
layout(location = 3) in vec3 in_tangent;
layout(location = 4) in vec3 in_bitangent;

// Outputs
layout(location = 0) out vec4 out_color;

// Material uniforms
layout(set = 1, binding = 0) uniform MaterialUniforms {
    vec4 albedo;
    vec3 emissive;
    float metallic;
    float roughness;
    float normal_scale;
    float ao_strength;
    float emissive_strength;
    uint flags; // bit 0: has_albedo_tex, bit 1: has_normal_tex, etc.
} material;

// Textures
layout(set = 1, binding = 1) uniform sampler2D albedo_texture;
layout(set = 1, binding = 2) uniform sampler2D normal_texture;
layout(set = 1, binding = 3) uniform sampler2D metallic_roughness_texture;
layout(set = 1, binding = 4) uniform sampler2D ao_texture;
layout(set = 1, binding = 5) uniform sampler2D emissive_texture;

// Scene uniforms
layout(set = 0, binding = 0) uniform SceneUniforms {
    vec3 camera_position;
    vec3 light_direction;
    vec3 light_color;
    float light_intensity;
} scene;

const float PI = 3.14159265359;

// PBR functions
vec3 fresnelSchlick(float cosTheta, vec3 F0) {
    return F0 + (1.0 - F0) * pow(1.0 - cosTheta, 5.0);
}

float DistributionGGX(vec3 N, vec3 H, float roughness) {
    float a = roughness * roughness;
    float a2 = a * a;
    float NdotH = max(dot(N, H), 0.0);
    float NdotH2 = NdotH * NdotH;

    float num = a2;
    float denom = (NdotH2 * (a2 - 1.0) + 1.0);
    denom = PI * denom * denom;

    return num / denom;
}

float GeometrySchlickGGX(float NdotV, float roughness) {
    float r = (roughness + 1.0);
    float k = (r * r) / 8.0;

    float num = NdotV;
    float denom = NdotV * (1.0 - k) + k;

    return num / denom;
}

float GeometrySmith(vec3 N, vec3 V, vec3 L, float roughness) {
    float NdotV = max(dot(N, V), 0.0);
    float NdotL = max(dot(N, L), 0.0);
    float ggx2 = GeometrySchlickGGX(NdotV, roughness);
    float ggx1 = GeometrySchlickGGX(NdotL, roughness);

    return ggx1 * ggx2;
}

void main() {
    // Sample textures
    vec4 albedo = material.albedo;
    if ((material.flags & 1u) != 0) {
        albedo *= texture(albedo_texture, in_uv);
    }

    // Normal mapping
    vec3 N = normalize(in_normal);
    if ((material.flags & 2u) != 0) {
        vec3 tangent_normal = texture(normal_texture, in_uv).xyz * 2.0 - 1.0;
        tangent_normal.xy *= material.normal_scale;

        mat3 TBN = mat3(
            normalize(in_tangent),
            normalize(in_bitangent),
            N
        );
        N = normalize(TBN * tangent_normal);
    }

    // Metallic and roughness
    float metallic = material.metallic;
    float roughness = material.roughness;
    if ((material.flags & 4u) != 0) {
        vec3 mr = texture(metallic_roughness_texture, in_uv).rgb;
        roughness *= mr.g;
        metallic *= mr.b;
    }

    // Ambient occlusion
    float ao = 1.0;
    if ((material.flags & 8u) != 0) {
        ao = texture(ao_texture, in_uv).r;
        ao = mix(1.0, ao, material.ao_strength);
    }

    // Emissive
    vec3 emissive = material.emissive * material.emissive_strength;
    if ((material.flags & 16u) != 0) {
        emissive *= texture(emissive_texture, in_uv).rgb;
    }

    // PBR calculation
    vec3 V = normalize(scene.camera_position - in_position);
    vec3 L = -scene.light_direction;
    vec3 H = normalize(V + L);

    vec3 F0 = vec3(0.04);
    F0 = mix(F0, albedo.rgb, metallic);

    // Cook-Torrance BRDF
    float NDF = DistributionGGX(N, H, roughness);
    float G = GeometrySmith(N, V, L, roughness);
    vec3 F = fresnelSchlick(max(dot(H, V), 0.0), F0);

    vec3 kS = F;
    vec3 kD = vec3(1.0) - kS;
    kD *= 1.0 - metallic;

    vec3 numerator = NDF * G * F;
    float denominator = 4.0 * max(dot(N, V), 0.0) * max(dot(N, L), 0.0) + 0.0001;
    vec3 specular = numerator / denominator;

    float NdotL = max(dot(N, L), 0.0);
    vec3 Lo = (kD * albedo.rgb / PI + specular) * scene.light_color * scene.light_intensity * NdotL;

    // Ambient
    vec3 ambient = vec3(0.03) * albedo.rgb * ao;

    vec3 color = ambient + Lo + emissive;

    // Tone mapping and gamma correction
    color = color / (color + vec3(1.0));
    color = pow(color, vec3(1.0 / 2.2));

    out_color = vec4(color, albedo.a);
}
```

---

### **5. Material System Integration** (Day 5)

**File:** `engine/renderer/src/systems/material_system.rs`

```rust
use crate::ecs::{System, World, Query};

/// Material rendering system
pub struct MaterialRenderSystem {
    material_library: MaterialLibrary,
    texture_manager: TextureManager,
}

impl MaterialRenderSystem {
    pub fn new() -> Self {
        Self {
            material_library: MaterialLibrary::new(),
            texture_manager: TextureManager::new(),
        }
    }

    /// Bind material for rendering
    pub fn bind_material(
        &self,
        device: &VulkanDevice,
        command_buffer: vk::CommandBuffer,
        material_handle: MaterialHandle,
    ) -> Result<(), RendererError> {
        let material = self
            .material_library
            .get_material(material_handle)
            .ok_or_else(|| RendererError::MaterialNotFound)?;

        // Update material uniforms
        let uniforms = MaterialUniforms {
            albedo: material.albedo,
            emissive: material.emissive,
            metallic: material.metallic,
            roughness: material.roughness,
            normal_scale: material.normal_scale,
            ao_strength: material.ao_strength,
            emissive_strength: material.emissive_strength,
            flags: self.calculate_material_flags(material),
        };

        // TODO: Push constants or descriptor set update

        // Bind textures
        // TODO: Bind texture descriptor sets

        Ok(())
    }

    fn calculate_material_flags(&self, material: &PbrMaterial) -> u32 {
        let mut flags = 0u32;

        if material.albedo_texture.is_some() {
            flags |= 1 << 0;
        }
        if material.normal_texture.is_some() {
            flags |= 1 << 1;
        }
        if material.metallic_roughness_texture.is_some() {
            flags |= 1 << 2;
        }
        if material.ao_texture.is_some() {
            flags |= 1 << 3;
        }
        if material.emissive_texture.is_some() {
            flags |= 1 << 4;
        }

        flags
    }
}

#[repr(C)]
struct MaterialUniforms {
    albedo: Vec4,
    emissive: Vec3,
    metallic: f32,
    roughness: f32,
    normal_scale: f32,
    ao_strength: f32,
    emissive_strength: f32,
    flags: u32,
}
```

---

## ✅ **Acceptance Criteria**

- [ ] PBR material data structure with all properties
- [ ] Texture loading from common formats (PNG, JPG, etc.)
- [ ] Mipmap generation
- [ ] Material component for ECS
- [ ] Material library with add/get/update
- [ ] PBR shader with Cook-Torrance BRDF
- [ ] Normal mapping support
- [ ] Emissive materials
- [ ] Alpha blending modes (opaque, mask, blend)
- [ ] glTF material import

---

## 🧪 **Tests**

```rust
#[test]
fn test_material_creation() {
    let material = PbrMaterial::new("Test")
        .with_albedo(Vec4::new(1.0, 0.0, 0.0, 1.0))
        .with_metallic_roughness(0.8, 0.2);

    assert_eq!(material.name, "Test");
    assert_eq!(material.metallic, 0.8);
    assert_eq!(material.roughness, 0.2);
}

#[test]
fn test_texture_loading() {
    let texture_data = TextureData::load_from_file(
        Path::new("test_assets/albedo.png"),
        TextureFormat::Rgba8Srgb,
    )
    .unwrap();

    assert!(texture_data.descriptor.width > 0);
    assert!(texture_data.descriptor.height > 0);
}

#[test]
fn test_material_library() {
    let mut library = MaterialLibrary::new();

    let mat = PbrMaterial::new("Test");
    let handle = library.add_material(mat);

    assert!(library.get_material(handle).is_some());
    assert_eq!(library.get_material(handle).unwrap().name, "Test");
}
```

---

## ⚡ **Performance Targets**

- **Texture Loading:** <100ms for 2K texture with mipmaps
- **Material Switching:** <0.1ms per material
- **Memory Usage:** ~10 MB per 2K texture with mipmaps
- **Shader Performance:** 60 FPS at 1080p with 100+ materials
- **Texture Limit:** 1000+ unique textures supported

---

## 📚 **Dependencies**

```toml
[dependencies]
glam = "0.24"
image = "0.24"
gltf = "1.0"
bitflags = "2.0"
```

---

**Dependencies:** [phase4-auto-update.md](phase4-auto-update.md)
**Next:** [phase4-lighting.md](phase4-lighting.md)
