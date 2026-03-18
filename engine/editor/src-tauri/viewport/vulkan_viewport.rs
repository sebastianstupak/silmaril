//! Minimal Vulkan clear-colour renderer for the editor viewport.
//!
//! Creates a Vulkan surface from a Win32 HWND (child window), initialises
//! instance/device/swapchain, and renders each frame by clearing to a dark
//! background colour.  Handles swapchain recreation on resize.
//!
//! This module intentionally does NOT depend on `engine-renderer` so that
//! the editor stays lean and avoids pulling in the full renderer dependency
//! graph.  Once the engine renderer API supports externally-created surfaces,
//! this will be replaced with a proper integration.

#![cfg(windows)]

use ash::vk;
use std::ffi::CStr;
use tracing::{debug, info};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

/// Clear colour for the viewport background: dark blue-gray (#1a1a26).
const CLEAR_COLOR: [f32; 4] = [0.102, 0.102, 0.149, 1.0];

/// All Vulkan state needed for a minimal clear-colour renderer.
pub struct VulkanViewport {
    _entry: ash::Entry,
    instance: ash::Instance,
    surface: vk::SurfaceKHR,
    surface_loader: ash::khr::surface::Instance,
    physical_device: vk::PhysicalDevice,
    device: ash::Device,
    graphics_queue: vk::Queue,
    graphics_queue_family: u32,
    swapchain_loader: ash::khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,
    image_available_semaphore: vk::Semaphore,
    render_finished_semaphore: vk::Semaphore,
    in_flight_fence: vk::Fence,
    width: u32,
    height: u32,
    /// Set when the swapchain needs recreation (e.g. after resize).
    needs_recreate: bool,
}

impl VulkanViewport {
    /// Initialise Vulkan for a child HWND.
    ///
    /// Creates instance, surface, device, swapchain, command pool/buffer,
    /// and synchronisation primitives.
    pub fn new(hwnd: isize, width: u32, height: u32) -> Result<Self, String> {
        // Guard against zero-size viewports
        let width = width.max(1);
        let height = height.max(1);

        // 1. Load Vulkan
        let entry = unsafe { ash::Entry::load() }.map_err(|e| format!("Vulkan load failed: {e}"))?;

        // 2. Create instance
        let instance = create_instance(&entry)?;

        // 3. Create Win32 surface
        let (surface, surface_loader) = create_surface(&entry, &instance, hwnd)?;

        // 4. Pick physical device
        let (physical_device, graphics_queue_family) =
            pick_physical_device(&instance, surface, &surface_loader)?;

        // 5. Create logical device + queue
        let (device, graphics_queue) =
            create_device(&instance, physical_device, graphics_queue_family)?;

        // 6. Create swapchain
        let swapchain_loader = ash::khr::swapchain::Device::new(&instance, &device);
        let (swapchain, swapchain_images) = create_swapchain(
            &swapchain_loader,
            &surface_loader,
            physical_device,
            surface,
            &device,
            graphics_queue_family,
            width,
            height,
            vk::SwapchainKHR::null(),
        )?;

        // 7. Command pool + buffer
        let (command_pool, command_buffer) =
            create_command_resources(&device, graphics_queue_family)?;

        // 8. Sync objects
        let (image_available_semaphore, render_finished_semaphore, in_flight_fence) =
            create_sync_objects(&device)?;

        info!(
            width,
            height,
            images = swapchain_images.len(),
            "Vulkan viewport initialised"
        );

        Ok(Self {
            _entry: entry,
            instance,
            surface,
            surface_loader,
            physical_device,
            device,
            graphics_queue,
            graphics_queue_family,
            swapchain_loader,
            swapchain,
            swapchain_images,
            command_pool,
            command_buffer,
            image_available_semaphore,
            render_finished_semaphore,
            in_flight_fence,
            width,
            height,
            needs_recreate: false,
        })
    }

    /// Notify the renderer that the viewport has been resized.
    ///
    /// The actual swapchain recreation happens lazily at the start of the
    /// next `render_frame` call.
    pub fn notify_resize(&mut self, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        if width != self.width || height != self.height {
            self.width = width;
            self.height = height;
            self.needs_recreate = true;
        }
    }

    /// Render a single frame (clear to background colour).
    ///
    /// Returns `Ok(true)` if a frame was presented, `Ok(false)` if the
    /// swapchain was out of date and will be recreated next call.
    pub fn render_frame(&mut self) -> Result<bool, String> {
        // Recreate swapchain if needed
        if self.needs_recreate {
            self.recreate_swapchain()?;
            self.needs_recreate = false;
        }

        unsafe {
            // Wait for previous frame
            self.device
                .wait_for_fences(&[self.in_flight_fence], true, u64::MAX)
                .map_err(|e| format!("wait_for_fences: {e}"))?;

            // Acquire next image
            let acquire_result = self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                self.image_available_semaphore,
                vk::Fence::null(),
            );

            let image_index = match acquire_result {
                Ok((index, _suboptimal)) => index,
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    self.needs_recreate = true;
                    return Ok(false);
                }
                Err(e) => return Err(format!("acquire_next_image: {e}")),
            };

            self.device
                .reset_fences(&[self.in_flight_fence])
                .map_err(|e| format!("reset_fences: {e}"))?;

            // Record command buffer
            self.record_clear_commands(image_index)?;

            // Submit
            let wait_semaphores = [self.image_available_semaphore];
            let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let signal_semaphores = [self.render_finished_semaphore];
            let command_buffers = [self.command_buffer];

            let submit_info = vk::SubmitInfo::default()
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_stages)
                .command_buffers(&command_buffers)
                .signal_semaphores(&signal_semaphores);

            self.device
                .queue_submit(self.graphics_queue, &[submit_info], self.in_flight_fence)
                .map_err(|e| format!("queue_submit: {e}"))?;

            // Present
            let swapchains = [self.swapchain];
            let image_indices = [image_index];
            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(&signal_semaphores)
                .swapchains(&swapchains)
                .image_indices(&image_indices);

            match self
                .swapchain_loader
                .queue_present(self.graphics_queue, &present_info)
            {
                Ok(_suboptimal) => {}
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR | vk::Result::SUBOPTIMAL_KHR) => {
                    self.needs_recreate = true;
                }
                Err(e) => return Err(format!("queue_present: {e}")),
            }
        }

        Ok(true)
    }

    /// Record clear-colour commands into the command buffer.
    unsafe fn record_clear_commands(&self, image_index: u32) -> Result<(), String> {
        let image = self.swapchain_images[image_index as usize];

        self.device
            .reset_command_buffer(self.command_buffer, vk::CommandBufferResetFlags::empty())
            .map_err(|e| format!("reset_command_buffer: {e}"))?;

        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        self.device
            .begin_command_buffer(self.command_buffer, &begin_info)
            .map_err(|e| format!("begin_command_buffer: {e}"))?;

        // Transition UNDEFINED -> TRANSFER_DST_OPTIMAL
        let barrier_to_transfer = vk::ImageMemoryBarrier::default()
            .image(image)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .src_access_mask(vk::AccessFlags::empty())
            .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        self.device.cmd_pipeline_barrier(
            self.command_buffer,
            vk::PipelineStageFlags::TOP_OF_PIPE,
            vk::PipelineStageFlags::TRANSFER,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier_to_transfer],
        );

        // Clear
        let clear_color = vk::ClearColorValue {
            float32: CLEAR_COLOR,
        };
        let range = vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        };
        self.device.cmd_clear_color_image(
            self.command_buffer,
            image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &clear_color,
            &[range],
        );

        // Transition TRANSFER_DST_OPTIMAL -> PRESENT_SRC_KHR
        let barrier_to_present = vk::ImageMemoryBarrier::default()
            .image(image)
            .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags::empty())
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        self.device.cmd_pipeline_barrier(
            self.command_buffer,
            vk::PipelineStageFlags::TRANSFER,
            vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            vk::DependencyFlags::empty(),
            &[],
            &[],
            &[barrier_to_present],
        );

        self.device
            .end_command_buffer(self.command_buffer)
            .map_err(|e| format!("end_command_buffer: {e}"))?;

        Ok(())
    }

    /// Recreate the swapchain after a resize.
    fn recreate_swapchain(&mut self) -> Result<(), String> {
        unsafe {
            self.device
                .device_wait_idle()
                .map_err(|e| format!("device_wait_idle: {e}"))?;
        }

        let old_swapchain = self.swapchain;

        let (new_swapchain, new_images) = create_swapchain(
            &self.swapchain_loader,
            &self.surface_loader,
            self.physical_device,
            self.surface,
            &self.device,
            self.graphics_queue_family,
            self.width,
            self.height,
            old_swapchain,
        )?;

        // Destroy old swapchain
        unsafe {
            self.swapchain_loader
                .destroy_swapchain(old_swapchain, None);
        }

        self.swapchain = new_swapchain;
        self.swapchain_images = new_images;

        debug!(
            width = self.width,
            height = self.height,
            images = self.swapchain_images.len(),
            "Swapchain recreated"
        );

        Ok(())
    }
}

impl Drop for VulkanViewport {
    fn drop(&mut self) {
        unsafe {
            let _ = self.device.device_wait_idle();

            self.device
                .destroy_semaphore(self.image_available_semaphore, None);
            self.device
                .destroy_semaphore(self.render_finished_semaphore, None);
            self.device.destroy_fence(self.in_flight_fence, None);
            self.device.destroy_command_pool(self.command_pool, None);
            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.device.destroy_device(None);
            self.instance.destroy_instance(None);
        }
        info!("Vulkan viewport destroyed");
    }
}

// ---------------------------------------------------------------------------
// Vulkan initialisation helpers
// ---------------------------------------------------------------------------

fn create_instance(entry: &ash::Entry) -> Result<ash::Instance, String> {
    let app_name = c"SilmarilEditor";
    let engine_name = c"Silmaril";

    let app_info = vk::ApplicationInfo::default()
        .application_name(app_name)
        .application_version(vk::make_api_version(0, 0, 1, 0))
        .engine_name(engine_name)
        .engine_version(vk::make_api_version(0, 0, 1, 0))
        .api_version(vk::API_VERSION_1_2);

    let mut extensions = vec![
        ash::khr::surface::NAME.as_ptr(),
        ash::khr::win32_surface::NAME.as_ptr(),
    ];

    // Enable debug utils in debug builds
    #[cfg(debug_assertions)]
    extensions.push(ash::ext::debug_utils::NAME.as_ptr());

    let create_info = vk::InstanceCreateInfo::default()
        .application_info(&app_info)
        .enabled_extension_names(&extensions);

    unsafe {
        entry
            .create_instance(&create_info, None)
            .map_err(|e| format!("vkCreateInstance: {e}"))
    }
}

fn create_surface(
    entry: &ash::Entry,
    instance: &ash::Instance,
    hwnd: isize,
) -> Result<(vk::SurfaceKHR, ash::khr::surface::Instance), String> {
    let win32_loader = ash::khr::win32_surface::Instance::new(entry, instance);

    let hinstance = unsafe {
        GetModuleHandleW(None).map_err(|e| format!("GetModuleHandleW: {e}"))?
    };

    let create_info = vk::Win32SurfaceCreateInfoKHR::default()
        .hwnd(hwnd as vk::HWND)
        .hinstance(hinstance.0 as vk::HINSTANCE);

    let surface = unsafe {
        win32_loader
            .create_win32_surface(&create_info, None)
            .map_err(|e| format!("vkCreateWin32SurfaceKHR: {e}"))?
    };

    let surface_loader = ash::khr::surface::Instance::new(entry, instance);

    Ok((surface, surface_loader))
}

fn pick_physical_device(
    instance: &ash::Instance,
    surface: vk::SurfaceKHR,
    surface_loader: &ash::khr::surface::Instance,
) -> Result<(vk::PhysicalDevice, u32), String> {
    let devices = unsafe {
        instance
            .enumerate_physical_devices()
            .map_err(|e| format!("enumerate_physical_devices: {e}"))?
    };

    if devices.is_empty() {
        return Err("No Vulkan-capable GPUs found".into());
    }

    let mut best: Option<(vk::PhysicalDevice, u32, u32)> = None; // (device, queue_family, score)

    for &pd in &devices {
        let props = unsafe { instance.get_physical_device_properties(pd) };
        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(pd) };

        // Find a graphics queue family that also supports present
        for (idx, qf) in queue_families.iter().enumerate() {
            if !qf.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                continue;
            }

            let supports_present = unsafe {
                surface_loader
                    .get_physical_device_surface_support(pd, idx as u32, surface)
                    .unwrap_or(false)
            };
            if !supports_present {
                continue;
            }

            let score = match props.device_type {
                vk::PhysicalDeviceType::DISCRETE_GPU => 1000,
                vk::PhysicalDeviceType::INTEGRATED_GPU => 500,
                vk::PhysicalDeviceType::VIRTUAL_GPU => 100,
                _ => 10,
            };

            if best.map_or(true, |(_, _, s)| score > s) {
                best = Some((pd, idx as u32, score));
            }

            break; // first suitable queue family is fine
        }
    }

    let (pd, qf, _) = best.ok_or("No GPU with graphics+present support found")?;

    let name = unsafe {
        let props = instance.get_physical_device_properties(pd);
        CStr::from_ptr(props.device_name.as_ptr())
            .to_string_lossy()
            .into_owned()
    };
    info!(device = %name, queue_family = qf, "Selected GPU for viewport");

    Ok((pd, qf))
}

fn create_device(
    instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    queue_family: u32,
) -> Result<(ash::Device, vk::Queue), String> {
    let queue_priorities = [1.0f32];
    let queue_create_info = vk::DeviceQueueCreateInfo::default()
        .queue_family_index(queue_family)
        .queue_priorities(&queue_priorities);

    let extensions = [ash::khr::swapchain::NAME.as_ptr()];

    let create_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(std::slice::from_ref(&queue_create_info))
        .enabled_extension_names(&extensions);

    let device = unsafe {
        instance
            .create_device(physical_device, &create_info, None)
            .map_err(|e| format!("vkCreateDevice: {e}"))?
    };

    let queue = unsafe { device.get_device_queue(queue_family, 0) };

    Ok((device, queue))
}

#[allow(clippy::too_many_arguments)]
fn create_swapchain(
    swapchain_loader: &ash::khr::swapchain::Device,
    surface_loader: &ash::khr::surface::Instance,
    physical_device: vk::PhysicalDevice,
    surface: vk::SurfaceKHR,
    _device: &ash::Device,
    queue_family: u32,
    width: u32,
    height: u32,
    old_swapchain: vk::SwapchainKHR,
) -> Result<(vk::SwapchainKHR, Vec<vk::Image>), String> {
    let capabilities = unsafe {
        surface_loader
            .get_physical_device_surface_capabilities(physical_device, surface)
            .map_err(|e| format!("get_surface_capabilities: {e}"))?
    };

    let formats = unsafe {
        surface_loader
            .get_physical_device_surface_formats(physical_device, surface)
            .map_err(|e| format!("get_surface_formats: {e}"))?
    };

    // Pick format: prefer B8G8R8A8_SRGB, fall back to first available
    let format = formats
        .iter()
        .find(|f| {
            f.format == vk::Format::B8G8R8A8_SRGB
                && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
        })
        .or_else(|| {
            formats.iter().find(|f| {
                f.format == vk::Format::B8G8R8A8_UNORM
                    && f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
        })
        .unwrap_or(&formats[0]);

    // Extent
    let extent = if capabilities.current_extent.width != u32::MAX {
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
    };

    // Image count: prefer triple buffering
    let mut image_count = capabilities.min_image_count + 1;
    if capabilities.max_image_count > 0 && image_count > capabilities.max_image_count {
        image_count = capabilities.max_image_count;
    }

    // We need TRANSFER_DST for vkCmdClearColorImage
    let usage = vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST;

    let queue_family_indices = [queue_family];
    let create_info = vk::SwapchainCreateInfoKHR::default()
        .surface(surface)
        .min_image_count(image_count)
        .image_format(format.format)
        .image_color_space(format.color_space)
        .image_extent(extent)
        .image_array_layers(1)
        .image_usage(usage)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .queue_family_indices(&queue_family_indices)
        .pre_transform(capabilities.current_transform)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .present_mode(vk::PresentModeKHR::FIFO) // guaranteed available
        .clipped(true)
        .old_swapchain(old_swapchain);

    let swapchain = unsafe {
        swapchain_loader
            .create_swapchain(&create_info, None)
            .map_err(|e| format!("vkCreateSwapchainKHR: {e}"))?
    };

    let images = unsafe {
        swapchain_loader
            .get_swapchain_images(swapchain)
            .map_err(|e| format!("get_swapchain_images: {e}"))?
    };

    Ok((swapchain, images))
}

fn create_command_resources(
    device: &ash::Device,
    queue_family: u32,
) -> Result<(vk::CommandPool, vk::CommandBuffer), String> {
    let pool_info = vk::CommandPoolCreateInfo::default()
        .queue_family_index(queue_family)
        .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

    let pool = unsafe {
        device
            .create_command_pool(&pool_info, None)
            .map_err(|e| format!("vkCreateCommandPool: {e}"))?
    };

    let alloc_info = vk::CommandBufferAllocateInfo::default()
        .command_pool(pool)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_buffer_count(1);

    let buffers = unsafe {
        device
            .allocate_command_buffers(&alloc_info)
            .map_err(|e| format!("vkAllocateCommandBuffers: {e}"))?
    };

    Ok((pool, buffers[0]))
}

fn create_sync_objects(
    device: &ash::Device,
) -> Result<(vk::Semaphore, vk::Semaphore, vk::Fence), String> {
    let sem_info = vk::SemaphoreCreateInfo::default();
    let fence_info = vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED);

    unsafe {
        let image_available = device
            .create_semaphore(&sem_info, None)
            .map_err(|e| format!("create_semaphore(image_available): {e}"))?;
        let render_finished = device
            .create_semaphore(&sem_info, None)
            .map_err(|e| format!("create_semaphore(render_finished): {e}"))?;
        let fence = device
            .create_fence(&fence_info, None)
            .map_err(|e| format!("create_fence: {e}"))?;

        Ok((image_available, render_finished, fence))
    }
}
