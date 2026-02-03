//! Phase 1.6 Rendering Pipeline Benchmarks
//!
//! Benchmarks for all Phase 1.6 rendering components:
//! - Command pool allocation
//! - Command buffer recording
//! - Synchronization (fence wait/reset)
//! - Framebuffer creation
//! - Full frame rendering cycle
//!
//! These benchmarks validate that the rendering pipeline meets performance targets.

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use engine_renderer::*;

fn bench_command_pool_allocation(c: &mut Criterion) {
    let context = VulkanContext::new("CommandPoolBench", None, None)
        .expect("Failed to create Vulkan context");

    c.bench_function("command_pool_creation", |b| {
        b.iter(|| {
            let pool = CommandPool::new(
                &context.device,
                context.queue_families.graphics,
                ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            )
            .expect("Failed to create command pool");

            black_box(pool);
        });
    });

    context.wait_idle().unwrap();
}

fn bench_command_buffer_allocation(c: &mut Criterion) {
    let context = VulkanContext::new("CommandBufferAllocBench", None, None)
        .expect("Failed to create Vulkan context");

    let command_pool = CommandPool::new(
        &context.device,
        context.queue_families.graphics,
        ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
    )
    .expect("Failed to create command pool");

    c.bench_function("command_buffer_allocation_2x", |b| {
        b.iter_batched(
            || &command_pool,
            |pool| {
                let buffers = pool
                    .allocate(&context.device, ash::vk::CommandBufferLevel::PRIMARY, 2)
                    .expect("Failed to allocate command buffers");
                black_box(buffers);
            },
            BatchSize::SmallInput,
        );
    });

    context.wait_idle().unwrap();
}

fn bench_command_buffer_recording(c: &mut Criterion) {
    let context = VulkanContext::new("CommandBufferRecordBench", None, None)
        .expect("Failed to create Vulkan context");

    let command_pool = CommandPool::new(
        &context.device,
        context.queue_families.graphics,
        ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
    )
    .expect("Failed to create command pool");

    let buffers = command_pool
        .allocate(&context.device, ash::vk::CommandBufferLevel::PRIMARY, 1)
        .expect("Failed to allocate command buffers");

    let cmd = CommandBuffer::from_handle(buffers[0]);

    // Create render pass for testing
    let render_pass = RenderPass::new(
        &context.device,
        RenderPassConfig {
            color_format: ash::vk::Format::B8G8R8A8_UNORM,
            depth_format: None,
            samples: ash::vk::SampleCountFlags::TYPE_1,
            load_op: ash::vk::AttachmentLoadOp::CLEAR,
            store_op: ash::vk::AttachmentStoreOp::STORE,
        },
    )
    .expect("Failed to create render pass");

    // Create dummy framebuffer
    let image_create_info = ash::vk::ImageCreateInfo::default()
        .image_type(ash::vk::ImageType::TYPE_2D)
        .format(ash::vk::Format::B8G8R8A8_UNORM)
        .extent(ash::vk::Extent3D { width: 800, height: 600, depth: 1 })
        .mip_levels(1)
        .array_layers(1)
        .samples(ash::vk::SampleCountFlags::TYPE_1)
        .usage(ash::vk::ImageUsageFlags::COLOR_ATTACHMENT);

    let image = unsafe { context.device.create_image(&image_create_info, None).unwrap() };

    let mem_requirements = unsafe { context.device.get_image_memory_requirements(image) };

    let mem_type_index = find_memory_type(
        &context,
        mem_requirements.memory_type_bits,
        ash::vk::MemoryPropertyFlags::DEVICE_LOCAL,
    );

    let alloc_info = ash::vk::MemoryAllocateInfo::default()
        .allocation_size(mem_requirements.size)
        .memory_type_index(mem_type_index);

    let memory = unsafe { context.device.allocate_memory(&alloc_info, None).unwrap() };
    unsafe { context.device.bind_image_memory(image, memory, 0).unwrap() };

    let view_create_info = ash::vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(ash::vk::ImageViewType::TYPE_2D)
        .format(ash::vk::Format::B8G8R8A8_UNORM)
        .subresource_range(ash::vk::ImageSubresourceRange {
            aspect_mask: ash::vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        });

    let image_view = unsafe { context.device.create_image_view(&view_create_info, None).unwrap() };

    let framebuffer = Framebuffer::new(
        &context.device,
        render_pass.handle(),
        image_view,
        ash::vk::Extent2D { width: 800, height: 600 },
    )
    .expect("Failed to create framebuffer");

    c.bench_function("command_buffer_record_clear", |b| {
        b.iter(|| {
            // Reset command buffer
            unsafe {
                context
                    .device
                    .reset_command_buffer(
                        cmd.handle(),
                        ash::vk::CommandBufferResetFlags::RELEASE_RESOURCES,
                    )
                    .unwrap();
            }

            // Record commands
            cmd.begin(&context.device, ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
                .unwrap();

            cmd.begin_render_pass(
                &context.device,
                render_pass.handle(),
                framebuffer.handle(),
                ash::vk::Extent2D { width: 800, height: 600 },
                [0.1, 0.2, 0.3, 1.0],
            );

            cmd.end_render_pass(&context.device);

            cmd.end(&context.device).unwrap();

            black_box(cmd.handle());
        });
    });

    // Cleanup
    unsafe {
        context.device.destroy_image_view(image_view, None);
        context.device.destroy_image(image, None);
        context.device.free_memory(memory, None);
    }

    context.wait_idle().unwrap();
}

fn bench_sync_objects_creation(c: &mut Criterion) {
    let context =
        VulkanContext::new("SyncObjectBench", None, None).expect("Failed to create Vulkan context");

    c.bench_function("sync_objects_create_2_frames", |b| {
        b.iter(|| {
            let sync =
                create_sync_objects(&context.device, 2).expect("Failed to create sync objects");
            black_box(sync);
        });
    });

    context.wait_idle().unwrap();
}

fn bench_fence_operations(c: &mut Criterion) {
    let context =
        VulkanContext::new("FenceBench", None, None).expect("Failed to create Vulkan context");

    let sync_objects =
        create_sync_objects(&context.device, 1).expect("Failed to create sync objects");
    let sync = &sync_objects[0];

    c.bench_function("fence_wait_and_reset", |b| {
        b.iter(|| {
            sync.wait(&context.device, u64::MAX).unwrap();
            sync.reset(&context.device).unwrap();
            black_box(sync.fence());
        });
    });

    context.wait_idle().unwrap();
}

fn bench_framebuffer_creation(c: &mut Criterion) {
    let context = VulkanContext::new("FramebufferBench", None, None)
        .expect("Failed to create Vulkan context");

    let render_pass = RenderPass::new(
        &context.device,
        RenderPassConfig {
            color_format: ash::vk::Format::B8G8R8A8_UNORM,
            depth_format: None,
            samples: ash::vk::SampleCountFlags::TYPE_1,
            load_op: ash::vk::AttachmentLoadOp::CLEAR,
            store_op: ash::vk::AttachmentStoreOp::STORE,
        },
    )
    .expect("Failed to create render pass");

    // Create dummy image view for framebuffer
    let image_create_info = ash::vk::ImageCreateInfo::default()
        .image_type(ash::vk::ImageType::TYPE_2D)
        .format(ash::vk::Format::B8G8R8A8_UNORM)
        .extent(ash::vk::Extent3D { width: 1920, height: 1080, depth: 1 })
        .mip_levels(1)
        .array_layers(1)
        .samples(ash::vk::SampleCountFlags::TYPE_1)
        .usage(ash::vk::ImageUsageFlags::COLOR_ATTACHMENT);

    let image = unsafe { context.device.create_image(&image_create_info, None).unwrap() };

    let mem_requirements = unsafe { context.device.get_image_memory_requirements(image) };
    let mem_type_index = find_memory_type(
        &context,
        mem_requirements.memory_type_bits,
        ash::vk::MemoryPropertyFlags::DEVICE_LOCAL,
    );

    let alloc_info = ash::vk::MemoryAllocateInfo::default()
        .allocation_size(mem_requirements.size)
        .memory_type_index(mem_type_index);

    let memory = unsafe { context.device.allocate_memory(&alloc_info, None).unwrap() };
    unsafe { context.device.bind_image_memory(image, memory, 0).unwrap() };

    let view_create_info = ash::vk::ImageViewCreateInfo::default()
        .image(image)
        .view_type(ash::vk::ImageViewType::TYPE_2D)
        .format(ash::vk::Format::B8G8R8A8_UNORM)
        .subresource_range(ash::vk::ImageSubresourceRange {
            aspect_mask: ash::vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        });

    let image_view = unsafe { context.device.create_image_view(&view_create_info, None).unwrap() };

    c.bench_function("framebuffer_creation_1080p", |b| {
        b.iter(|| {
            let framebuffer = Framebuffer::new(
                &context.device,
                render_pass.handle(),
                image_view,
                ash::vk::Extent2D { width: 1920, height: 1080 },
            )
            .expect("Failed to create framebuffer");
            black_box(framebuffer);
        });
    });

    // Cleanup
    unsafe {
        context.device.destroy_image_view(image_view, None);
        context.device.destroy_image(image, None);
        context.device.free_memory(memory, None);
    }

    context.wait_idle().unwrap();
}

fn bench_render_pass_creation(c: &mut Criterion) {
    let context =
        VulkanContext::new("RenderPassBench", None, None).expect("Failed to create Vulkan context");

    c.bench_function("render_pass_creation", |b| {
        b.iter(|| {
            let render_pass = RenderPass::new(
                &context.device,
                RenderPassConfig {
                    color_format: ash::vk::Format::B8G8R8A8_UNORM,
                    depth_format: None,
                    samples: ash::vk::SampleCountFlags::TYPE_1,
                    load_op: ash::vk::AttachmentLoadOp::CLEAR,
                    store_op: ash::vk::AttachmentStoreOp::STORE,
                },
            )
            .expect("Failed to create render pass");
            black_box(render_pass);
        });
    });

    context.wait_idle().unwrap();
}

// Helper function to find suitable memory type
fn find_memory_type(
    context: &VulkanContext,
    type_filter: u32,
    properties: ash::vk::MemoryPropertyFlags,
) -> u32 {
    let mem_properties =
        unsafe { context.instance.get_physical_device_memory_properties(context.physical_device) };

    for i in 0..mem_properties.memory_type_count {
        if (type_filter & (1 << i)) != 0
            && mem_properties.memory_types[i as usize].property_flags.contains(properties)
        {
            return i;
        }
    }

    panic!("Failed to find suitable memory type");
}

criterion_group!(
    rendering_pipeline_benches,
    bench_command_pool_allocation,
    bench_command_buffer_allocation,
    bench_command_buffer_recording,
    bench_sync_objects_creation,
    bench_fence_operations,
    bench_framebuffer_creation,
    bench_render_pass_creation,
);

criterion_main!(rendering_pipeline_benches);
