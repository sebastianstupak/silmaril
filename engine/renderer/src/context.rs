//! Vulkan context management.
//!
//! Handles Vulkan instance, device, and swapchain creation following best practices
//! from the 2026 Vulkan guide. Implements Phase 1.5: Vulkan Context.

// Tracy profiling macros (no-op when profiling feature disabled)
#[cfg(feature = "profiling")]
macro_rules! profile_scope {
    ($name:expr) => {
        let _tracy_span = tracy_client::span!($name);
    };
}

#[cfg(not(feature = "profiling"))]
macro_rules! profile_scope {
    ($name:expr) => {};
}

use crate::error::RendererError;
use ash::vk;
use gpu_allocator::vulkan as gpu_alloc;
use lazy_static::lazy_static;
use smallvec::SmallVec;
use std::ffi::{CStr, CString};
use std::sync::{Arc, Mutex};
use tracing::{error, info, instrument, warn};

// Cached validation layer names to avoid allocation on every context creation
lazy_static! {
    static ref VALIDATION_LAYERS: Vec<CString> = vec![
        CString::new("VK_LAYER_KHRONOS_validation").unwrap()
    ];

    // Device selection cache for quick re-initialization
    // Stores the UUID of the last selected device to avoid full enumeration
    static ref DEVICE_CACHE: Mutex<Option<[u8; 16]>> = Mutex::new(None);
}

/// Queue family indices for different queue types.
#[derive(Debug, Clone, Copy)]
pub struct QueueFamilies {
    /// Graphics queue family index (required)
    pub graphics: u32,
    /// Transfer queue family index (dedicated if available, otherwise graphics)
    pub transfer: u32,
    /// Compute queue family index (dedicated if available, otherwise graphics)
    pub compute: u32,
    /// Present queue family index (can present to swapchain)
    pub present: u32,
}

impl QueueFamilies {
    /// Check if we have a dedicated transfer queue.
    pub fn has_dedicated_transfer(&self) -> bool {
        self.transfer != self.graphics
    }

    /// Check if we have a dedicated compute queue.
    pub fn has_dedicated_compute(&self) -> bool {
        self.compute != self.graphics
    }

    /// Get unique queue family indices (for device creation).
    pub fn unique_indices(&self) -> Vec<u32> {
        let mut indices = vec![self.graphics, self.transfer, self.compute, self.present];
        indices.sort_unstable();
        indices.dedup();
        indices
    }
}

/// Physical device selection with scoring.
#[derive(Debug)]
struct DeviceCandidate {
    physical_device: vk::PhysicalDevice,
    score: u32,
    properties: vk::PhysicalDeviceProperties,
}

/// Vulkan context containing all initialization state.
pub struct VulkanContext {
    /// Ash entry point
    pub entry: ash::Entry,
    /// Vulkan instance
    pub instance: ash::Instance,
    /// Debug messenger (only in debug builds)
    #[cfg(debug_assertions)]
    pub debug_messenger: Option<vk::DebugUtilsMessengerEXT>,
    /// Debug utils loader (only in debug builds)
    #[cfg(debug_assertions)]
    pub debug_utils_loader: Option<ash::ext::debug_utils::Instance>,
    /// Physical device (GPU)
    pub physical_device: vk::PhysicalDevice,
    /// Physical device properties
    pub physical_device_properties: vk::PhysicalDeviceProperties,
    /// Physical device features
    pub physical_device_features: vk::PhysicalDeviceFeatures,
    /// Physical device memory properties
    pub physical_device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    /// Logical device
    pub device: ash::Device,
    /// Queue families
    pub queue_families: QueueFamilies,
    /// Graphics queue
    pub graphics_queue: vk::Queue,
    /// Transfer queue (may be same as graphics)
    pub transfer_queue: vk::Queue,
    /// Compute queue (may be same as graphics)
    pub compute_queue: vk::Queue,
    /// Present queue
    pub present_queue: vk::Queue,
    /// GPU memory allocator
    pub allocator: Arc<std::sync::Mutex<gpu_alloc::Allocator>>,
}

impl VulkanContext {
    /// Create a new Vulkan context.
    ///
    /// This performs all Vulkan initialization:
    /// - Instance creation with validation layers (debug only)
    /// - Physical device selection with scoring
    /// - Logical device creation with required queues
    /// - Memory allocator setup
    ///
    /// # Arguments
    ///
    /// * `app_name` - Application name for Vulkan metadata
    /// * `surface` - Optional surface for presentation (None for headless)
    /// * `surface_loader` - Surface loader (required if surface is Some)
    #[instrument(skip_all)]
    pub fn new(
        app_name: &str,
        surface: Option<vk::SurfaceKHR>,
        surface_loader: Option<&ash::khr::surface::Instance>,
    ) -> Result<Self, RendererError> {
        profile_scope!("VulkanContext::new");
        info!(app_name = app_name, "Creating Vulkan context");

        // 1. Create Vulkan entry and instance
        // SAFETY: ash::Entry::load() dynamically loads the Vulkan library.
        // This is safe because it only reads from the system's Vulkan loader.
        let entry = unsafe {
            ash::Entry::load().map_err(|e| {
                error!(error = ?e, "Failed to load Vulkan library");
                RendererError::instancecreationfailed(format!(
                    "Failed to load Vulkan library: {:?}",
                    e
                ))
            })?
        };

        let instance = create_instance(&entry, app_name)?;

        // 2. Set up debug messenger (debug builds only, unless disabled for benchmarks)
        #[cfg(debug_assertions)]
        let (debug_messenger, debug_utils_loader) = {
            let disable_validation = std::env::var("DISABLE_VULKAN_VALIDATION").is_ok();
            if disable_validation {
                (None, None)
            } else {
                setup_debug_messenger(&entry, &instance)?
            }
        };

        // 3. Select physical device
        let (physical_device, physical_device_properties) =
            select_physical_device(&instance, surface, surface_loader)?;

        // Get device features and memory properties
        // SAFETY: physical_device is valid, returned from select_physical_device.
        // These Vulkan API calls only query device properties and don't modify state.
        let physical_device_features =
            unsafe { instance.get_physical_device_features(physical_device) };
        let physical_device_memory_properties =
            unsafe { instance.get_physical_device_memory_properties(physical_device) };

        // SAFETY: device_name is a fixed-size array filled by Vulkan driver.
        // The Vulkan spec guarantees it's null-terminated UTF-8.
        info!(
            device_name = unsafe { CStr::from_ptr(physical_device_properties.device_name.as_ptr()) }
                .to_string_lossy()
                .as_ref(),
            device_type = ?physical_device_properties.device_type,
            api_version = physical_device_properties.api_version,
            driver_version = physical_device_properties.driver_version,
            "Selected physical device"
        );

        // 4. Find queue families
        let queue_families =
            find_queue_families(&instance, physical_device, surface, surface_loader)?;

        info!(
            graphics = queue_families.graphics,
            transfer = queue_families.transfer,
            compute = queue_families.compute,
            present = queue_families.present,
            dedicated_transfer = queue_families.has_dedicated_transfer(),
            dedicated_compute = queue_families.has_dedicated_compute(),
            "Found queue families"
        );

        // 5. Create logical device and queues
        let (device, queues) =
            create_logical_device(&instance, physical_device, &queue_families, surface.is_some())?;

        // 6. Set up memory allocator
        let allocator = create_allocator(&instance, physical_device, &device)?;

        info!("Vulkan context created successfully");

        Ok(VulkanContext {
            entry,
            instance,
            #[cfg(debug_assertions)]
            debug_messenger,
            #[cfg(debug_assertions)]
            debug_utils_loader,
            physical_device,
            physical_device_properties,
            physical_device_features,
            physical_device_memory_properties,
            device,
            queue_families,
            graphics_queue: queues.0,
            transfer_queue: queues.1,
            compute_queue: queues.2,
            present_queue: queues.3,
            allocator: Arc::new(std::sync::Mutex::new(allocator)),
        })
    }

    /// Get device name as a string.
    pub fn device_name(&self) -> String {
        // SAFETY: device_name is a fixed-size array filled by Vulkan driver.
        // The Vulkan spec guarantees it's null-terminated UTF-8.
        unsafe {
            CStr::from_ptr(self.physical_device_properties.device_name.as_ptr())
                .to_string_lossy()
                .into_owned()
        }
    }

    /// Create a VulkanContext optimized for benchmarking (no validation layers).
    ///
    /// This constructor disables validation layers regardless of build type,
    /// which is essential for accurate performance measurements.
    ///
    /// # Warning
    /// Only use this for benchmarks! Validation layers catch bugs during development.
    ///
    /// # Example
    /// ```no_run
    /// let context = VulkanContext::new_for_benchmarks("BenchApp", None, None)?;
    /// ```
    pub fn new_for_benchmarks(
        app_name: &str,
        surface_provider: Option<&dyn Fn(&ash::Entry, &ash::Instance) -> Result<vk::SurfaceKHR, RendererError>>,
        preferred_device_uuid: Option<[u8; 16]>,
    ) -> Result<Self, RendererError> {
        // Same as new() but forces validation layers off
        std::env::set_var("DISABLE_VULKAN_VALIDATION", "1");
        let result = Self::new(app_name, surface_provider, preferred_device_uuid);
        std::env::remove_var("DISABLE_VULKAN_VALIDATION");
        result
    }

    /// Wait for the device to become idle.
    pub fn wait_idle(&self) -> Result<(), RendererError> {
        // SAFETY: self.device is valid and we have exclusive access via &self.
        // device_wait_idle() is a standard Vulkan synchronization call.
        unsafe {
            self.device.device_wait_idle().map_err(|e| {
                error!(error = ?e, "Failed to wait for device idle");
                RendererError::queuesubmissionfailed(format!("Device wait idle failed: {:?}", e))
            })
        }
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        info!("Destroying Vulkan context");

        // SAFETY: We're in Drop, so we have exclusive access to all resources.
        // We must ensure proper cleanup order to avoid use-after-free:
        // 1. Wait for GPU idle (ensures no operations are in flight)
        // 2. Free allocator (releases GPU memory allocations)
        // 3. Destroy device (invalidates all device resources)
        // 4. Destroy debug messenger (if present)
        // 5. Destroy instance (final cleanup)
        unsafe {
            // Wait for device to be idle before cleanup
            // Ignore errors - we're already in Drop, nothing we can do
            let _ = self.device.device_wait_idle();

            // Drop allocator first (it owns GPU memory)
            // Note: If the lock is poisoned, we can't recover anyway since we're in Drop
            if let Ok(_allocator) = self.allocator.lock() {
                // The allocator will be dropped when the MutexGuard goes out of scope
                // This ensures memory is freed before destroying the device
            } else {
                // Lock poisoned - log error but continue cleanup
                error!("Allocator lock poisoned during cleanup, memory may leak");
            }

            // Destroy device
            self.device.destroy_device(None);

            // Destroy debug messenger (debug builds only)
            #[cfg(debug_assertions)]
            if let (Some(messenger), Some(ref loader)) =
                (self.debug_messenger, &self.debug_utils_loader)
            {
                loader.destroy_debug_utils_messenger(messenger, None);
            }

            // Destroy instance
            self.instance.destroy_instance(None);
        }

        info!("Vulkan context destroyed");
    }
}

/// Create Vulkan instance with validation layers (debug only).
#[instrument(skip(entry))]
fn create_instance(entry: &ash::Entry, app_name: &str) -> Result<ash::Instance, RendererError> {
    profile_scope!("create_instance");
    // Check API version
    // SAFETY: try_enumerate_instance_version only queries the Vulkan loader.
    // It doesn't modify any state and is safe to call at any time.
    let api_version = unsafe {
        match entry.try_enumerate_instance_version() {
            Ok(Some(version)) => {
                info!(
                    major = vk::api_version_major(version),
                    minor = vk::api_version_minor(version),
                    patch = vk::api_version_patch(version),
                    "Vulkan API version"
                );
                version
            }
            Ok(None) => vk::make_api_version(0, 1, 0, 0),
            Err(e) => {
                warn!(error = ?e, "Failed to query Vulkan version, assuming 1.0");
                vk::make_api_version(0, 1, 0, 0)
            }
        }
    };

    // Require at least Vulkan 1.1
    if vk::api_version_major(api_version) < 1
        || (vk::api_version_major(api_version) == 1 && vk::api_version_minor(api_version) < 1)
    {
        return Err(RendererError::instancecreationfailed(
            "Vulkan 1.1 or higher required".to_string(),
        ));
    }

    // Application info
    let app_name_cstr = CString::new(app_name).unwrap();
    let engine_name = CString::new("Agent Game Engine").unwrap();

    let app_info = vk::ApplicationInfo::default()
        .application_name(&app_name_cstr)
        .application_version(vk::make_api_version(0, 0, 1, 0))
        .engine_name(&engine_name)
        .engine_version(vk::make_api_version(0, 0, 1, 0))
        .api_version(api_version);

    // Required extensions (will be populated by window system)
    #[cfg(any(debug_assertions, target_os = "macos"))]
    let mut extension_names = vec![];
    #[cfg(not(any(debug_assertions, target_os = "macos")))]
    let extension_names = vec![];

    // Add debug utils extension in debug builds (unless disabled for benchmarks)
    #[cfg(debug_assertions)]
    {
        let disable_validation = std::env::var("DISABLE_VULKAN_VALIDATION").is_ok();
        if !disable_validation {
            extension_names.push(ash::ext::debug_utils::NAME.as_ptr());
        }
    }

    // Add portability enumeration for macOS
    #[cfg(target_os = "macos")]
    {
        extension_names.push(ash::khr::portability_enumeration::NAME.as_ptr());
        extension_names.push(ash::khr::get_physical_device_properties2::NAME.as_ptr());
    }

    // Validation layers (debug builds only, unless explicitly disabled for benchmarks)
    #[cfg(debug_assertions)]
    let layer_name_ptrs: Vec<*const i8>;

    #[cfg(debug_assertions)]
    {
        // Check if validation should be disabled (for benchmarks)
        let disable_validation = std::env::var("DISABLE_VULKAN_VALIDATION").is_ok();

        if disable_validation {
            info!("Validation layers disabled for benchmarking");
            layer_name_ptrs = vec![];
        } else {
            layer_name_ptrs = VALIDATION_LAYERS.iter().map(|name| name.as_ptr()).collect();

            // Check if validation layers are available
            // SAFETY: enumerate_instance_layer_properties only queries available layers.
            // It returns a Vec of layer properties, no pointer manipulation needed.
            let available_layers = unsafe {
                entry.enumerate_instance_layer_properties().map_err(|e| {
                    RendererError::instancecreationfailed(format!(
                        "Failed to enumerate layers: {:?}",
                        e
                    ))
                })?
            };

            for required_layer in VALIDATION_LAYERS.iter() {
                let found = available_layers.iter().any(|layer| {
                    // SAFETY: layer_name is a fixed array in VkLayerProperties.
                    // Vulkan spec guarantees it's null-terminated UTF-8.
                    let layer_name = unsafe { CStr::from_ptr(layer.layer_name.as_ptr()) };
                    layer_name == required_layer.as_c_str()
                });

                if !found {
                    warn!(
                        layer = ?required_layer,
                        "Validation layer not available"
                    );
                    return Err(RendererError::validationlayernotavailable(
                        required_layer.to_string_lossy().into_owned(),
                    ));
                }
            }

            info!(layers = ?*VALIDATION_LAYERS, "Enabling validation layers");
        }
    }

    #[cfg(not(debug_assertions))]
    let layer_name_ptrs: Vec<*const i8> = vec![];

    // Create instance
    let create_info = vk::InstanceCreateInfo::default()
        .application_info(&app_info)
        .enabled_extension_names(&extension_names)
        .enabled_layer_names(&layer_name_ptrs);

    // Add portability enumeration flag for macOS
    #[cfg(target_os = "macos")]
    {
        create_info = create_info.flags(vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR);
    }

    // SAFETY: create_info is properly initialized with valid pointers.
    // All string pointers (app_info, extensions, layers) remain valid for the duration of this call.
    // The Vulkan loader will create the instance and return ownership.
    unsafe {
        entry.create_instance(&create_info, None).map_err(|e| {
            error!(error = ?e, "Failed to create Vulkan instance");
            RendererError::instancecreationfailed(format!("{:?}", e))
        })
    }
}

/// Set up debug messenger for validation layers (debug builds only).
#[cfg(debug_assertions)]
#[instrument(skip(entry, instance))]
fn setup_debug_messenger(
    entry: &ash::Entry,
    instance: &ash::Instance,
) -> Result<
    (Option<vk::DebugUtilsMessengerEXT>, Option<ash::ext::debug_utils::Instance>),
    RendererError,
> {
    let debug_utils_loader = ash::ext::debug_utils::Instance::new(entry, instance);

    let create_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
        .message_severity(
            vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
        )
        .message_type(
            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
        )
        .pfn_user_callback(Some(vulkan_debug_callback));

    // SAFETY: create_info is properly initialized with a valid callback function pointer.
    // The callback (vulkan_debug_callback) is a static function with correct signature.
    let debug_messenger = unsafe {
        debug_utils_loader
            .create_debug_utils_messenger(&create_info, None)
            .map_err(|e| {
                warn!(error = ?e, "Failed to create debug messenger");
                RendererError::debugmessengercreationfailed(format!("{:?}", e))
            })?
    };

    info!("Debug messenger created");

    Ok((Some(debug_messenger), Some(debug_utils_loader)))
}

/// Vulkan debug callback (debug builds only).
///
/// # Safety
///
/// This function is called by the Vulkan validation layers. The Vulkan spec guarantees:
/// - p_callback_data is a valid pointer to DebugUtilsMessengerCallbackDataEXT
/// - All string fields are either null or point to valid null-terminated UTF-8
/// - This callback is never called after the messenger is destroyed
#[cfg(debug_assertions)]
unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::ffi::c_void,
) -> vk::Bool32 {
    // SAFETY: Vulkan spec guarantees p_callback_data is valid for the duration of this call
    unsafe {
        // Validate pointer before dereferencing
        if p_callback_data.is_null() {
            error!("Vulkan debug callback received null callback data");
            return vk::FALSE;
        }

        let callback_data = *p_callback_data;

        // SAFETY: Vulkan spec guarantees these pointers are either null or valid UTF-8 C strings
        let message_id_name = if callback_data.p_message_id_name.is_null() {
            std::borrow::Cow::from("")
        } else {
            CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
        };
        let message = if callback_data.p_message.is_null() {
            std::borrow::Cow::from("")
        } else {
            CStr::from_ptr(callback_data.p_message).to_string_lossy()
        };

        match message_severity {
            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
                error!(
                    vuid = %message_id_name,
                    type_ = ?message_type,
                    "{}", message
                );
            }
            vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
                warn!(
                    vuid = %message_id_name,
                    type_ = ?message_type,
                    "{}", message
                );
            }
            vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
                info!(
                    vuid = %message_id_name,
                    type_ = ?message_type,
                    "{}", message
                );
            }
            vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
                tracing::trace!(
                    vuid = %message_id_name,
                    type_ = ?message_type,
                    "{}", message
                );
            }
            _ => {}
        }
    }

    vk::FALSE
}

/// Score a physical device based on its properties and features.
fn score_device(
    properties: &vk::PhysicalDeviceProperties,
    features: &vk::PhysicalDeviceFeatures,
) -> u32 {
    let mut score = 0;

    // Device type priority
    score += match properties.device_type {
        vk::PhysicalDeviceType::DISCRETE_GPU => 1000,
        vk::PhysicalDeviceType::INTEGRATED_GPU => 500,
        vk::PhysicalDeviceType::VIRTUAL_GPU => 100,
        vk::PhysicalDeviceType::CPU => 50,
        _ => 25,
    };

    // Bonus for higher texture dimensions
    if properties.limits.max_image_dimension2_d > 4096 {
        score += 50;
    }
    if properties.limits.max_image_dimension2_d > 16384 {
        score += 100;
    }

    // Bonus for geometry shader support
    if features.geometry_shader == vk::TRUE {
        score += 25;
    }

    // Bonus for tessellation support
    if features.tessellation_shader == vk::TRUE {
        score += 25;
    }

    score
}

/// Select the best physical device (GPU).
#[instrument(skip(instance, surface_loader))]
fn select_physical_device(
    instance: &ash::Instance,
    surface: Option<vk::SurfaceKHR>,
    surface_loader: Option<&ash::khr::surface::Instance>,
) -> Result<(vk::PhysicalDevice, vk::PhysicalDeviceProperties), RendererError> {
    profile_scope!("select_physical_device");
    // SAFETY: instance is valid, enumerate_physical_devices returns handles owned by the instance.
    // The physical device handles remain valid for the lifetime of the instance.
    let physical_devices = unsafe {
        instance
            .enumerate_physical_devices()
            .map_err(|e| RendererError::deviceenumerationfailed(format!("{:?}", e)))?
    };

    if physical_devices.is_empty() {
        return Err(RendererError::nosuitablegpu(0));
    }

    info!(device_count = physical_devices.len(), "Found physical devices");

    // Check device cache for quick re-initialization
    if let Ok(cached_uuid) = DEVICE_CACHE.lock() {
        if let Some(uuid) = *cached_uuid {
            // Try to find the cached device
            for &physical_device in &physical_devices {
                // SAFETY: physical_device is valid, returned from enumerate_physical_devices.
                let properties =
                    unsafe { instance.get_physical_device_properties(physical_device) };

                if properties.pipeline_cache_uuid == uuid {
                    info!(device_uuid = ?uuid, "Found cached device, skipping full enumeration");
                    return Ok((physical_device, properties));
                }
            }
        }
    }

    // Use SmallVec since most systems have ≤4 GPUs (avoids heap allocation in common case)
    let mut candidates: SmallVec<[DeviceCandidate; 4]> = SmallVec::new();

    for &physical_device in &physical_devices {
        // SAFETY: physical_device is valid, returned from enumerate_physical_devices.
        // These query functions only read device properties, they don't modify state.
        let properties = unsafe { instance.get_physical_device_properties(physical_device) };
        let features = unsafe { instance.get_physical_device_features(physical_device) };

        // Check if device has required queue families
        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let has_graphics = queue_families
            .iter()
            .any(|qf| qf.queue_flags.contains(vk::QueueFlags::GRAPHICS));

        if !has_graphics {
            continue; // Skip devices without graphics queue
        }

        // If we have a surface, check for present support
        if let (Some(surface), Some(surface_loader)) = (surface, surface_loader) {
            let has_present = queue_families.iter().enumerate().any(|(index, _)| {
                // SAFETY: physical_device and surface are valid.
                // get_physical_device_surface_support only queries capabilities.
                unsafe {
                    surface_loader
                        .get_physical_device_surface_support(physical_device, index as u32, surface)
                        .unwrap_or(false)
                }
            });

            if !has_present {
                continue; // Skip devices that can't present to surface
            }
        }

        let score = score_device(&properties, &features);

        // SAFETY: device_name is a fixed-size array from VkPhysicalDeviceProperties.
        // Vulkan spec guarantees it's null-terminated UTF-8.
        let device_name = unsafe {
            CStr::from_ptr(properties.device_name.as_ptr()).to_string_lossy().into_owned()
        };

        info!(
            device = device_name,
            device_type = ?properties.device_type,
            score = score,
            "Found suitable device"
        );

        candidates.push(DeviceCandidate { physical_device, score, properties });
    }

    if candidates.is_empty() {
        return Err(RendererError::nosuitablegpu(physical_devices.len()));
    }

    // Sort by score (descending)
    candidates.sort_by(|a, b| b.score.cmp(&a.score));

    let best = candidates.into_iter().next().unwrap();

    // Cache the selected device UUID for quick re-initialization
    if let Ok(mut cache) = DEVICE_CACHE.lock() {
        *cache = Some(best.properties.pipeline_cache_uuid);
        info!(device_uuid = ?best.properties.pipeline_cache_uuid, "Cached device selection");
    }

    Ok((best.physical_device, best.properties))
}

/// Find queue families for graphics, transfer, compute, and present.
#[instrument(skip(instance, surface_loader))]
fn find_queue_families(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    surface: Option<vk::SurfaceKHR>,
    surface_loader: Option<&ash::khr::surface::Instance>,
) -> Result<QueueFamilies, RendererError> {
    profile_scope!("find_queue_families");
    // SAFETY: physical_device is valid. This query only reads queue family properties.
    let queue_families =
        unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

    // Find graphics queue (required)
    let graphics = queue_families
        .iter()
        .position(|qf| qf.queue_flags.contains(vk::QueueFlags::GRAPHICS))
        .ok_or_else(|| RendererError::queuefamilynotfound("graphics".to_string()))?
        as u32;

    // Find dedicated transfer queue (prefer dedicated, fallback to graphics)
    let transfer = queue_families
        .iter()
        .position(|qf| {
            qf.queue_flags.contains(vk::QueueFlags::TRANSFER)
                && !qf.queue_flags.contains(vk::QueueFlags::GRAPHICS)
        })
        .unwrap_or(graphics as usize) as u32;

    // Find dedicated compute queue (prefer dedicated, fallback to graphics)
    let compute = queue_families
        .iter()
        .position(|qf| {
            qf.queue_flags.contains(vk::QueueFlags::COMPUTE)
                && !qf.queue_flags.contains(vk::QueueFlags::GRAPHICS)
        })
        .unwrap_or(graphics as usize) as u32;

    // Find present queue
    let present = if let (Some(surface), Some(surface_loader)) = (surface, surface_loader) {
        let mut present_idx = None;
        for (index, _) in queue_families.iter().enumerate() {
            // SAFETY: physical_device and surface are valid.
            // get_physical_device_surface_support only queries presentation support.
            let supports_present = unsafe {
                surface_loader
                    .get_physical_device_surface_support(physical_device, index as u32, surface)
                    .unwrap_or(false)
            };
            if supports_present {
                present_idx = Some(index as u32);
                break;
            }
        }
        present_idx.ok_or_else(|| RendererError::queuefamilynotfound("present".to_string()))?
    } else {
        graphics // Use graphics queue if no surface
    };

    Ok(QueueFamilies { graphics, transfer, compute, present })
}

/// Create logical device and queues.
#[instrument(skip(instance))]
fn create_logical_device(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    queue_families: &QueueFamilies,
    needs_swapchain: bool,
) -> Result<(ash::Device, (vk::Queue, vk::Queue, vk::Queue, vk::Queue)), RendererError> {
    let queue_priorities = [1.0f32];
    let unique_indices = queue_families.unique_indices();

    let queue_create_infos: Vec<_> = unique_indices
        .iter()
        .map(|&index| {
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(index)
                .queue_priorities(&queue_priorities)
        })
        .collect();

    // Required extensions
    let mut extension_names = vec![];
    if needs_swapchain {
        extension_names.push(ash::khr::swapchain::NAME.as_ptr());
    }

    // Add portability subset for macOS
    #[cfg(target_os = "macos")]
    extension_names.push(vk::KHR_PORTABILITY_SUBSET_NAME.as_ptr());

    let device_features = vk::PhysicalDeviceFeatures::default();

    let create_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(&queue_create_infos)
        .enabled_extension_names(&extension_names)
        .enabled_features(&device_features);

    // SAFETY: physical_device is valid, create_info is properly initialized.
    // All pointers in create_info (queue_create_infos, extensions) remain valid for this call.
    let device = unsafe {
        instance
            .create_device(physical_device, &create_info, None)
            .map_err(|e| RendererError::logicaldevicecreationfailed(format!("{:?}", e)))?
    };

    // Get queue handles
    // SAFETY: device is valid, queue family indices are valid (from find_queue_families).
    // We always request queue index 0, which exists because we verified queue count > 0.
    let graphics_queue = unsafe { device.get_device_queue(queue_families.graphics, 0) };
    let transfer_queue = unsafe { device.get_device_queue(queue_families.transfer, 0) };
    let compute_queue = unsafe { device.get_device_queue(queue_families.compute, 0) };
    let present_queue = unsafe { device.get_device_queue(queue_families.present, 0) };

    info!("Logical device created");

    Ok((device, (graphics_queue, transfer_queue, compute_queue, present_queue)))
}

/// Create GPU memory allocator.
#[instrument(skip(instance, device))]
fn create_allocator(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    device: &ash::Device,
) -> Result<gpu_alloc::Allocator, RendererError> {
    let allocator = gpu_alloc::Allocator::new(&gpu_alloc::AllocatorCreateDesc {
        instance: instance.clone(),
        device: device.clone(),
        physical_device,
        debug_settings: Default::default(),
        buffer_device_address: false,
        allocation_sizes: Default::default(),
    })
    .map_err(|e| RendererError::memoryallocationfailed(0, format!("{:?}", e)))?;

    info!("GPU allocator created");

    Ok(allocator)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_families_unique_indices() {
        let families = QueueFamilies { graphics: 0, transfer: 0, compute: 1, present: 0 };

        let unique = families.unique_indices();
        assert_eq!(unique.len(), 2);
        assert!(unique.contains(&0));
        assert!(unique.contains(&1));
    }

    #[test]
    fn test_queue_families_dedicated() {
        let families = QueueFamilies { graphics: 0, transfer: 1, compute: 2, present: 0 };

        assert!(families.has_dedicated_transfer());
        assert!(families.has_dedicated_compute());
    }

    #[test]
    fn test_device_scoring() {
        let mut properties = vk::PhysicalDeviceProperties::default();
        let features = vk::PhysicalDeviceFeatures::default();

        // Discrete GPU should score higher
        properties.device_type = vk::PhysicalDeviceType::DISCRETE_GPU;
        let discrete_score = score_device(&properties, &features);

        properties.device_type = vk::PhysicalDeviceType::INTEGRATED_GPU;
        let integrated_score = score_device(&properties, &features);

        assert!(discrete_score > integrated_score);
    }
}
