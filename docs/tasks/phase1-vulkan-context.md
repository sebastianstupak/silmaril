# Phase 1.5: Vulkan Context Setup

**Status:** ⚪ Not Started
**Estimated Time:** 4-5 days
**Priority:** Critical (foundation for rendering)

---

## 🎯 **Objective**

Initialize Vulkan instance, select physical device, create logical device, setup memory allocator, and create swapchain. This is the foundation for all rendering.

**Must support:**
- Validation layers (dev builds)
- Multiple GPUs (select best)
- Memory allocation (gpu-allocator)
- Surface/swapchain creation

---

## 📋 **Detailed Tasks**

### **1. Vulkan Instance** (Day 1)

**File:** `engine/renderer/src/vulkan/instance.rs`

```rust
use ash::{vk, Entry};
use std::ffi::{CStr, CString};

/// Vulkan instance wrapper
pub struct VulkanInstance {
    entry: Entry,
    instance: ash::Instance,
    debug_utils: Option<DebugUtils>,
}

/// Debug utilities (validation layers)
struct DebugUtils {
    loader: ash::extensions::ext::DebugUtils,
    messenger: vk::DebugUtilsMessengerEXT,
}

impl VulkanInstance {
    /// Create Vulkan instance
    pub fn new(
        app_name: &str,
        enable_validation: bool,
        required_extensions: &[&str],
    ) -> Result<Self, RendererError> {
        let entry = Entry::linked();

        // Application info
        let app_name_cstr = CString::new(app_name).unwrap();
        let engine_name_cstr = CString::new("Agent Game Engine").unwrap();

        let app_info = vk::ApplicationInfo::builder()
            .application_name(&app_name_cstr)
            .application_version(vk::make_api_version(0, 1, 0, 0))
            .engine_name(&engine_name_cstr)
            .engine_version(vk::make_api_version(0, 1, 0, 0))
            .api_version(vk::API_VERSION_1_3);

        // Validation layers
        let layer_names: Vec<CString> = if enable_validation {
            vec![CString::new("VK_LAYER_KHRONOS_validation").unwrap()]
        } else {
            Vec::new()
        };

        let layer_name_ptrs: Vec<*const i8> = layer_names
            .iter()
            .map(|name| name.as_ptr())
            .collect();

        // Extensions
        let mut extension_names: Vec<CString> = required_extensions
            .iter()
            .map(|&ext| CString::new(ext).unwrap())
            .collect();

        if enable_validation {
            extension_names.push(CString::new("VK_EXT_debug_utils").unwrap());
        }

        let extension_name_ptrs: Vec<*const i8> = extension_names
            .iter()
            .map(|name| name.as_ptr())
            .collect();

        // Create instance
        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_layer_names(&layer_name_ptrs)
            .enabled_extension_names(&extension_name_ptrs);

        let instance = unsafe {
            entry
                .create_instance(&create_info, None)
                .map_err(|e| RendererError::VulkanInit {
                    details: format!("Failed to create instance: {}", e),
                })?
        };

        tracing::info!("Vulkan instance created");

        // Setup debug messenger
        let debug_utils = if enable_validation {
            Some(Self::setup_debug_messenger(&entry, &instance)?)
        } else {
            None
        };

        Ok(Self {
            entry,
            instance,
            debug_utils,
        })
    }

    /// Setup debug messenger for validation layers
    fn setup_debug_messenger(
        entry: &Entry,
        instance: &ash::Instance,
    ) -> Result<DebugUtils, RendererError> {
        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(debug_callback));

        let debug_utils_loader = ash::extensions::ext::DebugUtils::new(entry, instance);
        let messenger = unsafe {
            debug_utils_loader
                .create_debug_utils_messenger(&debug_info, None)
                .map_err(|e| RendererError::VulkanInit {
                    details: format!("Failed to create debug messenger: {}", e),
                })?
        };

        tracing::info!("Vulkan debug messenger created");

        Ok(DebugUtils {
            loader: debug_utils_loader,
            messenger,
        })
    }

    pub fn instance(&self) -> &ash::Instance {
        &self.instance
    }

    pub fn entry(&self) -> &Entry {
        &self.entry
    }
}

impl Drop for VulkanInstance {
    fn drop(&mut self) {
        unsafe {
            if let Some(debug_utils) = &self.debug_utils {
                debug_utils
                    .loader
                    .destroy_debug_utils_messenger(debug_utils.messenger, None);
            }
            self.instance.destroy_instance(None);
        }
        tracing::info!("Vulkan instance destroyed");
    }
}

/// Debug callback for validation layers
unsafe extern "system" fn debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message = CStr::from_ptr(callback_data.p_message).to_string_lossy();

    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            tracing::error!("[Vulkan {:?}] {}", message_type, message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            tracing::warn!("[Vulkan {:?}] {}", message_type, message);
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            tracing::info!("[Vulkan {:?}] {}", message_type, message);
        }
        _ => {
            tracing::debug!("[Vulkan {:?}] {}", message_type, message);
        }
    }

    vk::FALSE
}
```

---

### **2. Physical Device Selection** (Day 1-2)

**File:** `engine/renderer/src/vulkan/device.rs`

```rust
/// Physical device selection criteria
#[derive(Debug, Clone)]
pub struct DeviceRequirements {
    pub discrete_gpu_preferred: bool,
    pub min_vram_mb: u64,
    pub required_extensions: Vec<String>,
}

impl Default for DeviceRequirements {
    fn default() -> Self {
        Self {
            discrete_gpu_preferred: true,
            min_vram_mb: 1024, // 1 GB minimum
            required_extensions: vec![
                "VK_KHR_swapchain".to_string(),
            ],
        }
    }
}

/// Physical device info
#[derive(Debug)]
pub struct PhysicalDeviceInfo {
    pub device: vk::PhysicalDevice,
    pub properties: vk::PhysicalDeviceProperties,
    pub features: vk::PhysicalDeviceFeatures,
    pub memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub queue_families: Vec<QueueFamilyInfo>,
}

#[derive(Debug, Clone)]
pub struct QueueFamilyInfo {
    pub index: u32,
    pub properties: vk::QueueFamilyProperties,
    pub supports_graphics: bool,
    pub supports_compute: bool,
    pub supports_transfer: bool,
    pub supports_present: bool,
}

impl PhysicalDeviceInfo {
    /// Select best physical device
    pub fn select(
        instance: &ash::Instance,
        surface: vk::SurfaceKHR,
        surface_loader: &ash::extensions::khr::Surface,
        requirements: &DeviceRequirements,
    ) -> Result<Self, RendererError> {
        let devices = unsafe {
            instance
                .enumerate_physical_devices()
                .map_err(|e| RendererError::VulkanInit {
                    details: format!("Failed to enumerate devices: {}", e),
                })?
        };

        if devices.is_empty() {
            return Err(RendererError::VulkanInit {
                details: "No Vulkan devices found".to_string(),
            });
        }

        // Score and select best device
        let mut best_device: Option<(vk::PhysicalDevice, i32)> = None;

        for device in devices {
            let score = Self::score_device(instance, device, surface, surface_loader, requirements)?;

            if score > 0 {
                if let Some((_, best_score)) = best_device {
                    if score > best_score {
                        best_device = Some((device, score));
                    }
                } else {
                    best_device = Some((device, score));
                }
            }
        }

        let device = best_device
            .ok_or_else(|| RendererError::VulkanInit {
                details: "No suitable GPU found".to_string(),
            })?
            .0;

        // Get device info
        let properties = unsafe { instance.get_physical_device_properties(device) };
        let features = unsafe { instance.get_physical_device_features(device) };
        let memory_properties = unsafe { instance.get_physical_device_memory_properties(device) };

        let queue_families = Self::get_queue_families(instance, device, surface, surface_loader)?;

        let device_name = unsafe {
            CStr::from_ptr(properties.device_name.as_ptr())
                .to_string_lossy()
        };

        tracing::info!("Selected GPU: {}", device_name);

        Ok(Self {
            device,
            properties,
            features,
            memory_properties,
            queue_families,
        })
    }

    /// Score device (higher = better)
    fn score_device(
        instance: &ash::Instance,
        device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
        surface_loader: &ash::extensions::khr::Surface,
        requirements: &DeviceRequirements,
    ) -> Result<i32, RendererError> {
        let properties = unsafe { instance.get_physical_device_properties(device) };
        let features = unsafe { instance.get_physical_device_features(device) };

        let mut score = 0;

        // Discrete GPU preferred
        if properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
            score += 1000;
        }

        // More VRAM = better
        let memory_properties = unsafe { instance.get_physical_device_memory_properties(device) };
        let vram_mb = memory_properties
            .memory_heaps
            .iter()
            .take(memory_properties.memory_heap_count as usize)
            .filter(|heap| heap.flags.contains(vk::MemoryHeapFlags::DEVICE_LOCAL))
            .map(|heap| heap.size / (1024 * 1024))
            .sum::<u64>();

        if vram_mb < requirements.min_vram_mb {
            return Ok(0); // Not suitable
        }

        score += (vram_mb / 1024) as i32; // Add GB count

        // Check required extensions
        let available_extensions = unsafe {
            instance
                .enumerate_device_extension_properties(device)
                .map_err(|e| RendererError::VulkanInit {
                    details: format!("Failed to enumerate extensions: {}", e),
                })?
        };

        let extension_names: Vec<String> = available_extensions
            .iter()
            .map(|ext| {
                unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) }
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        for required in &requirements.required_extensions {
            if !extension_names.contains(required) {
                return Ok(0); // Not suitable
            }
        }

        // Check queue families
        let queue_families = Self::get_queue_families(instance, device, surface, surface_loader)?;

        let has_graphics = queue_families.iter().any(|qf| qf.supports_graphics);
        let has_present = queue_families.iter().any(|qf| qf.supports_present);

        if !has_graphics || !has_present {
            return Ok(0); // Not suitable
        }

        Ok(score)
    }

    /// Get queue family information
    fn get_queue_families(
        instance: &ash::Instance,
        device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
        surface_loader: &ash::extensions::khr::Surface,
    ) -> Result<Vec<QueueFamilyInfo>, RendererError> {
        let queue_family_properties =
            unsafe { instance.get_physical_device_queue_family_properties(device) };

        let mut queue_families = Vec::new();

        for (index, properties) in queue_family_properties.iter().enumerate() {
            let supports_present = unsafe {
                surface_loader
                    .get_physical_device_surface_support(device, index as u32, surface)
                    .unwrap_or(false)
            };

            queue_families.push(QueueFamilyInfo {
                index: index as u32,
                properties: *properties,
                supports_graphics: properties.queue_flags.contains(vk::QueueFlags::GRAPHICS),
                supports_compute: properties.queue_flags.contains(vk::QueueFlags::COMPUTE),
                supports_transfer: properties.queue_flags.contains(vk::QueueFlags::TRANSFER),
                supports_present,
            });
        }

        Ok(queue_families)
    }

    /// Find queue family index
    pub fn find_queue_family<F>(&self, predicate: F) -> Option<u32>
    where
        F: Fn(&QueueFamilyInfo) -> bool,
    {
        self.queue_families
            .iter()
            .find(|qf| predicate(qf))
            .map(|qf| qf.index)
    }
}
```

---

### **3. Logical Device Creation** (Day 2-3)

**File:** `engine/renderer/src/vulkan/device.rs` (continued)

```rust
/// Logical device wrapper
pub struct VulkanDevice {
    device: ash::Device,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
    graphics_queue_family: u32,
    present_queue_family: u32,
}

impl VulkanDevice {
    /// Create logical device
    pub fn new(
        instance: &ash::Instance,
        physical_device_info: &PhysicalDeviceInfo,
    ) -> Result<Self, RendererError> {
        // Find queue families
        let graphics_queue_family = physical_device_info
            .find_queue_family(|qf| qf.supports_graphics)
            .ok_or_else(|| RendererError::VulkanInit {
                details: "No graphics queue family found".to_string(),
            })?;

        let present_queue_family = physical_device_info
            .find_queue_family(|qf| qf.supports_present)
            .ok_or_else(|| RendererError::VulkanInit {
                details: "No present queue family found".to_string(),
            })?;

        // Queue create infos
        let queue_priorities = [1.0_f32];
        let mut queue_create_infos = vec![vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(graphics_queue_family)
            .queue_priorities(&queue_priorities)
            .build()];

        // If present and graphics are different families, create both
        if graphics_queue_family != present_queue_family {
            queue_create_infos.push(
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(present_queue_family)
                    .queue_priorities(&queue_priorities)
                    .build(),
            );
        }

        // Extensions
        let extension_names = [
            ash::extensions::khr::Swapchain::name().as_ptr(),
        ];

        // Features
        let features = vk::PhysicalDeviceFeatures::builder()
            .sampler_anisotropy(true)
            .fill_mode_non_solid(true);

        // Create device
        let create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&extension_names)
            .enabled_features(&features);

        let device = unsafe {
            instance
                .create_device(physical_device_info.device, &create_info, None)
                .map_err(|e| RendererError::VulkanInit {
                    details: format!("Failed to create device: {}", e),
                })?
        };

        // Get queues
        let graphics_queue = unsafe { device.get_device_queue(graphics_queue_family, 0) };
        let present_queue = unsafe { device.get_device_queue(present_queue_family, 0) };

        tracing::info!("Vulkan logical device created");

        Ok(Self {
            device,
            graphics_queue,
            present_queue,
            graphics_queue_family,
            present_queue_family,
        })
    }

    pub fn device(&self) -> &ash::Device {
        &self.device
    }

    pub fn graphics_queue(&self) -> vk::Queue {
        self.graphics_queue
    }

    pub fn present_queue(&self) -> vk::Queue {
        self.present_queue
    }

    pub fn graphics_queue_family(&self) -> u32 {
        self.graphics_queue_family
    }
}

impl Drop for VulkanDevice {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_device(None);
        }
        tracing::info!("Vulkan logical device destroyed");
    }
}
```

---

### **4. Memory Allocator** (Day 3)

**File:** `engine/renderer/src/vulkan/allocator.rs`

```rust
use gpu_allocator::vulkan::*;
use gpu_allocator::MemoryLocation;

/// Memory allocator wrapper
pub struct VulkanAllocator {
    allocator: Allocator,
}

impl VulkanAllocator {
    /// Create allocator
    pub fn new(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
    ) -> Result<Self, RendererError> {
        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: instance.clone(),
            device: device.clone(),
            physical_device,
            debug_settings: AllocatorDebugSettings {
                log_memory_information: cfg!(debug_assertions),
                log_leaks_on_shutdown: cfg!(debug_assertions),
                ..Default::default()
            },
            buffer_device_address: false,
        })
        .map_err(|e| RendererError::AllocationFailed {
            details: format!("Failed to create allocator: {}", e),
        })?;

        tracing::info!("Vulkan memory allocator created");

        Ok(Self { allocator })
    }

    /// Allocate buffer
    pub fn allocate_buffer(
        &mut self,
        device: &ash::Device,
        size: u64,
        usage: vk::BufferUsageFlags,
        location: MemoryLocation,
    ) -> Result<(vk::Buffer, Allocation), RendererError> {
        // Create buffer
        let buffer_info = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe {
            device
                .create_buffer(&buffer_info, None)
                .map_err(|e| RendererError::BufferCreationFailed {
                    details: e.to_string(),
                })?
        };

        // Allocate memory
        let requirements = unsafe { device.get_buffer_memory_requirements(buffer) };

        let allocation = self
            .allocator
            .allocate(&AllocationCreateDesc {
                name: "buffer",
                requirements,
                location,
                linear: true,
            })
            .map_err(|e| RendererError::AllocationFailed {
                details: e.to_string(),
            })?;

        // Bind memory
        unsafe {
            device
                .bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
                .map_err(|e| RendererError::AllocationFailed {
                    details: e.to_string(),
                })?;
        }

        Ok((buffer, allocation))
    }

    /// Free buffer
    pub fn free_buffer(
        &mut self,
        device: &ash::Device,
        buffer: vk::Buffer,
        allocation: Allocation,
    ) {
        unsafe {
            device.destroy_buffer(buffer, None);
        }
        self.allocator.free(allocation).unwrap();
    }
}
```

---

### **5. Swapchain** (Day 4-5)

**File:** `engine/renderer/src/vulkan/swapchain.rs`

```rust
/// Swapchain wrapper
pub struct Swapchain {
    loader: ash::extensions::khr::Swapchain,
    swapchain: vk::SwapchainKHR,
    images: Vec<vk::Image>,
    image_views: Vec<vk::ImageView>,
    format: vk::SurfaceFormatKHR,
    extent: vk::Extent2D,
}

impl Swapchain {
    /// Create swapchain
    pub fn new(
        instance: &ash::Instance,
        device: &ash::Device,
        physical_device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
        surface_loader: &ash::extensions::khr::Surface,
        width: u32,
        height: u32,
    ) -> Result<Self, RendererError> {
        // Query surface capabilities
        let capabilities = unsafe {
            surface_loader
                .get_physical_device_surface_capabilities(physical_device, surface)
                .map_err(|e| RendererError::SwapchainCreationFailed {
                    details: e.to_string(),
                })?
        };

        // Choose format
        let formats = unsafe {
            surface_loader
                .get_physical_device_surface_formats(physical_device, surface)
                .map_err(|e| RendererError::SwapchainCreationFailed {
                    details: e.to_string(),
                })?
        };

        let format = Self::choose_surface_format(&formats);

        // Choose present mode
        let present_modes = unsafe {
            surface_loader
                .get_physical_device_surface_present_modes(physical_device, surface)
                .map_err(|e| RendererError::SwapchainCreationFailed {
                    details: e.to_string(),
                })?
        };

        let present_mode = Self::choose_present_mode(&present_modes);

        // Choose extent
        let extent = Self::choose_extent(&capabilities, width, height);

        // Image count (triple buffering preferred)
        let mut image_count = capabilities.min_image_count + 1;
        if capabilities.max_image_count > 0 && image_count > capabilities.max_image_count {
            image_count = capabilities.max_image_count;
        }

        // Create swapchain
        let create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surface)
            .min_image_count(image_count)
            .image_format(format.format)
            .image_color_space(format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true);

        let swapchain_loader = ash::extensions::khr::Swapchain::new(instance, device);

        let swapchain = unsafe {
            swapchain_loader
                .create_swapchain(&create_info, None)
                .map_err(|e| RendererError::SwapchainCreationFailed {
                    details: e.to_string(),
                })?
        };

        // Get images
        let images = unsafe {
            swapchain_loader
                .get_swapchain_images(swapchain)
                .map_err(|e| RendererError::SwapchainCreationFailed {
                    details: e.to_string(),
                })?
        };

        // Create image views
        let image_views = Self::create_image_views(device, &images, format.format)?;

        tracing::info!(
            "Swapchain created: {}x{}, {} images",
            extent.width,
            extent.height,
            images.len()
        );

        Ok(Self {
            loader: swapchain_loader,
            swapchain,
            images,
            image_views,
            format,
            extent,
        })
    }

    fn choose_surface_format(formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
        // Prefer SRGB
        for format in formats {
            if format.format == vk::Format::B8G8R8A8_SRGB
                && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            {
                return *format;
            }
        }
        formats[0]
    }

    fn choose_present_mode(present_modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
        // Prefer mailbox (triple buffering)
        if present_modes.contains(&vk::PresentModeKHR::MAILBOX) {
            vk::PresentModeKHR::MAILBOX
        } else {
            vk::PresentModeKHR::FIFO // VSync (always available)
        }
    }

    fn choose_extent(
        capabilities: &vk::SurfaceCapabilitiesKHR,
        width: u32,
        height: u32,
    ) -> vk::Extent2D {
        if capabilities.current_extent.width != u32::MAX {
            capabilities.current_extent
        } else {
            vk::Extent2D {
                width: width.clamp(
                    capabilities.min_image_extent.width,
                    capabilities.max_image_extent.width,
                ),
                height: height.clamp(
                    capabilities.min_image_extent.height,
                    capabilities.max_image_extent.height,
                ),
            }
        }
    }

    fn create_image_views(
        device: &ash::Device,
        images: &[vk::Image],
        format: vk::Format,
    ) -> Result<Vec<vk::ImageView>, RendererError> {
        images
            .iter()
            .map(|&image| {
                let create_info = vk::ImageViewCreateInfo::builder()
                    .image(image)
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(format)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    });

                unsafe {
                    device
                        .create_image_view(&create_info, None)
                        .map_err(|e| RendererError::ImageViewCreationFailed {
                            details: e.to_string(),
                        })
                }
            })
            .collect()
    }

    pub fn images(&self) -> &[vk::Image] {
        &self.images
    }

    pub fn image_views(&self) -> &[vk::ImageView] {
        &self.image_views
    }

    pub fn format(&self) -> vk::Format {
        self.format.format
    }

    pub fn extent(&self) -> vk::Extent2D {
        self.extent
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            for &image_view in &self.image_views {
                self.loader.destroy_image_view(image_view, None);
            }
            self.loader.destroy_swapchain(self.swapchain, None);
        }
        tracing::info!("Swapchain destroyed");
    }
}
```

---

## ✅ **Acceptance Criteria**

- [ ] Vulkan instance created with validation layers
- [ ] Debug messenger logs validation errors
- [ ] Physical device selection works (best GPU)
- [ ] Logical device created with queues
- [ ] Memory allocator created (gpu-allocator)
- [ ] Swapchain created with proper format/mode
- [ ] All platforms supported (Windows, Linux, macOS)
- [ ] Proper cleanup (no leaks)
- [ ] Error handling with custom error types
- [ ] Structured logging (tracing)

---

## 🧪 **Tests**

```rust
#[test]
fn test_instance_creation() {
    let instance = VulkanInstance::new("Test", true, &[]).unwrap();
    // Instance should be valid
    assert!(!instance.instance().handle().is_null());
}

#[test]
fn test_device_selection() {
    let instance = VulkanInstance::new("Test", false, &[]).unwrap();
    // Create dummy surface for testing
    // ... device selection test
}
```

---

**Dependencies:** [phase1-platform.md](phase1-platform.md)
**Next:** [phase1-basic-rendering.md](phase1-basic-rendering.md)
