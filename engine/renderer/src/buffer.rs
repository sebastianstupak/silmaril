//! GPU buffer management (Vulkan buffers with gpu-allocator)

use crate::context::VulkanContext;
use crate::error::RendererError;
use ash::vk;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme};
use gpu_allocator::MemoryLocation;
use tracing::{info, instrument};

/// GPU buffer with automatic memory management
pub struct GpuBuffer {
    /// Vulkan buffer handle
    pub buffer: vk::Buffer,
    /// GPU memory allocation
    pub allocation: Option<Allocation>,
    /// Buffer size in bytes
    pub size: u64,
    /// Vulkan device (for cleanup)
    device: ash::Device,
}

impl std::fmt::Debug for GpuBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GpuBuffer")
            .field("buffer", &self.buffer)
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

impl GpuBuffer {
    /// Create a new GPU buffer
    ///
    /// # Arguments
    /// * `context` - Vulkan context
    /// * `size` - Buffer size in bytes
    /// * `usage` - Buffer usage flags
    /// * `location` - Memory location (GPU only, CPU visible, etc.)
    #[instrument(skip(context))]
    pub fn new(
        context: &VulkanContext,
        size: u64,
        usage: vk::BufferUsageFlags,
        location: MemoryLocation,
    ) -> Result<Self, RendererError> {
        // Create buffer
        let buffer_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe {
            context.device.create_buffer(&buffer_info, None).map_err(|e| {
                RendererError::buffercreationfailed(size, format!("vkCreateBuffer failed: {:?}", e))
            })?
        };

        // Get memory requirements
        let requirements = unsafe { context.device.get_buffer_memory_requirements(buffer) };

        // Allocate memory
        let allocation = context
            .allocator
            .lock()
            .map_err(|e| {
                unsafe { context.device.destroy_buffer(buffer, None) };
                RendererError::memoryallocationfailed(
                    size,
                    format!("Failed to lock allocator: {:?}", e),
                )
            })?
            .allocate(&AllocationCreateDesc {
                name: "gpu_buffer",
                requirements,
                location,
                linear: true,
                allocation_scheme: AllocationScheme::GpuAllocatorManaged,
            })
            .map_err(|e| {
                unsafe { context.device.destroy_buffer(buffer, None) };
                RendererError::memoryallocationfailed(size, format!("Allocation failed: {:?}", e))
            })?;

        // Bind buffer memory
        unsafe {
            context
                .device
                .bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
                .map_err(|e| {
                    context.device.destroy_buffer(buffer, None);
                    RendererError::memoryallocationfailed(
                        size,
                        format!("bind_buffer_memory failed: {:?}", e),
                    )
                })?;
        }

        info!(size = size, usage = ?usage, "GPU buffer created");

        Ok(Self { buffer, allocation: Some(allocation), size, device: context.device.clone() })
    }

    /// Upload data to buffer (for CPU-visible buffers)
    ///
    /// # Safety
    /// Buffer must be CPU-visible (MemoryLocation::CpuToGpu)
    pub fn upload<T: Copy>(&mut self, data: &[T]) -> Result<(), RendererError> {
        let data_size = std::mem::size_of_val(data) as u64;
        if data_size > self.size {
            return Err(RendererError::memoryallocationfailed(
                data_size,
                format!("Data size {} exceeds buffer size {}", data_size, self.size),
            ));
        }

        if let Some(allocation) = &mut self.allocation {
            let mapped_ptr = allocation.mapped_ptr().ok_or_else(|| {
                RendererError::memorymappingfailed("Buffer is not CPU-visible".to_string())
            })?;

            unsafe {
                std::ptr::copy_nonoverlapping(
                    data.as_ptr() as *const u8,
                    mapped_ptr.as_ptr() as *mut u8,
                    data_size as usize,
                );
            }

            Ok(())
        } else {
            Err(RendererError::memorymappingfailed("No allocation".to_string()))
        }
    }

    /// Get buffer handle
    pub fn handle(&self) -> vk::Buffer {
        self.buffer
    }

    /// Get buffer size
    pub fn size(&self) -> u64 {
        self.size
    }
}

impl Drop for GpuBuffer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_buffer(self.buffer, None);
        }
        // Allocation is automatically freed when dropped
    }
}

/// Vertex buffer (convenience wrapper)
#[derive(Debug)]
pub struct VertexBuffer {
    buffer: GpuBuffer,
    vertex_count: u32,
}

impl VertexBuffer {
    /// Create a vertex buffer from data
    pub fn from_data<T: Copy>(
        context: &VulkanContext,
        vertices: &[T],
    ) -> Result<Self, RendererError> {
        let size = std::mem::size_of_val(vertices) as u64;
        let usage = vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST;

        let mut buffer = GpuBuffer::new(context, size, usage, MemoryLocation::CpuToGpu)?;

        buffer.upload(vertices)?;

        Ok(Self { buffer, vertex_count: vertices.len() as u32 })
    }

    /// Get buffer handle
    pub fn handle(&self) -> vk::Buffer {
        self.buffer.handle()
    }

    /// Get vertex count
    pub fn vertex_count(&self) -> u32 {
        self.vertex_count
    }
}

/// Index buffer (convenience wrapper)
#[derive(Debug)]
pub struct IndexBuffer {
    buffer: GpuBuffer,
    index_count: u32,
}

impl IndexBuffer {
    /// Create an index buffer from data
    pub fn from_data(context: &VulkanContext, indices: &[u32]) -> Result<Self, RendererError> {
        let size = std::mem::size_of_val(indices) as u64;
        let usage = vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST;

        let mut buffer = GpuBuffer::new(context, size, usage, MemoryLocation::CpuToGpu)?;

        buffer.upload(indices)?;

        Ok(Self { buffer, index_count: indices.len() as u32 })
    }

    /// Get buffer handle
    pub fn handle(&self) -> vk::Buffer {
        self.buffer.handle()
    }

    /// Get index count
    pub fn index_count(&self) -> u32 {
        self.index_count
    }
}

/// GPU mesh (vertex + index buffers)
pub struct GpuMesh {
    /// Vertex buffer
    pub vertex_buffer: VertexBuffer,
    /// Index buffer
    pub index_buffer: IndexBuffer,
}

impl GpuMesh {
    /// Create a GPU mesh from MeshData
    pub fn from_mesh_data(
        context: &VulkanContext,
        mesh_data: &engine_assets::MeshData,
    ) -> Result<Self, RendererError> {
        let vertex_buffer = VertexBuffer::from_data(context, &mesh_data.vertices)?;
        let index_buffer = IndexBuffer::from_data(context, &mesh_data.indices)?;

        info!(
            vertices = vertex_buffer.vertex_count(),
            indices = index_buffer.index_count(),
            "GPU mesh created"
        );

        Ok(Self { vertex_buffer, index_buffer })
    }

    /// Get vertex count
    pub fn vertex_count(&self) -> u32 {
        self.vertex_buffer.vertex_count()
    }

    /// Get index count
    pub fn index_count(&self) -> u32 {
        self.index_buffer.index_count()
    }

    /// Get triangle count
    pub fn triangle_count(&self) -> u32 {
        self.index_count() / 3
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_buffer_sizes() {
        // Just test size calculations (no Vulkan required)
        let vertex_size = std::mem::size_of::<engine_assets::Vertex>();
        assert_eq!(vertex_size, 32);

        let vertices_count = 100;
        let expected_size = vertex_size * vertices_count;
        assert_eq!(expected_size, 3200);
    }
}
