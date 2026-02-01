# Phase 4.4: Tracy Profiling Integration

**Status:** ⚪ Not Started
**Estimated Time:** 2-3 days
**Priority:** High (performance monitoring)

---

## 🎯 **Objective**

Integrate Tracy profiler for real-time performance analysis. Add profiling zones throughout the engine, built-in profiling UI, performance metrics dashboard, and data export capabilities.

**Must support:**
- Tracy profiling zones (manual and automatic)
- Built-in profiling UI with frame graphs
- Performance metrics dashboard
- Export profiling data (JSON, CSV)
- Memory tracking
- GPU profiling integration

---

## 📋 **Detailed Tasks**

### **1. Tracy Integration** (Day 1)

**File:** `engine/profiling/Cargo.toml`

```toml
[package]
name = "profiling"
version = "0.1.0"
edition = "2021"

[dependencies]
tracy-client = { version = "0.17", optional = true }
tracing = "0.1"

[features]
default = []
tracy = ["tracy-client"]
```

**File:** `engine/profiling/src/lib.rs`

```rust
#[cfg(feature = "tracy")]
pub use tracy_client;

/// Initialize profiler
pub fn init_profiler() {
    #[cfg(feature = "tracy")]
    {
        tracy_client::Client::start();
        tracing::info!("Tracy profiler initialized");
    }

    #[cfg(not(feature = "tracy"))]
    {
        tracing::info!("Tracy profiler disabled (compile with --features tracy)");
    }
}

/// Profiling zone macro
#[macro_export]
macro_rules! profile_zone {
    ($name:expr) => {
        #[cfg(feature = "tracy")]
        let _tracy_zone = tracy_client::span!($name);
    };
}

/// Profiling function macro
#[macro_export]
macro_rules! profile_function {
    () => {
        #[cfg(feature = "tracy")]
        let _tracy_zone = tracy_client::span!();
    };
}

/// Frame mark (call once per frame)
#[inline]
pub fn frame_mark() {
    #[cfg(feature = "tracy")]
    tracy_client::frame_mark();
}

/// Plot value (for graphs)
#[inline]
pub fn plot(name: &str, value: f64) {
    #[cfg(feature = "tracy")]
    tracy_client::plot!(name, value);
}

/// Memory allocation tracking
#[inline]
pub fn alloc(ptr: *const u8, size: usize) {
    #[cfg(feature = "tracy")]
    tracy_client::alloc(ptr, size);
}

#[inline]
pub fn free(ptr: *const u8) {
    #[cfg(feature = "tracy")]
    tracy_client::free(ptr);
}

/// Message (appears in timeline)
#[inline]
pub fn message(text: &str) {
    #[cfg(feature = "tracy")]
    tracy_client::message(text, 0);
}

/// Colored message
#[inline]
pub fn message_color(text: &str, color: u32) {
    #[cfg(feature = "tracy")]
    tracy_client::message(text, color);
}
```

---

### **2. Profiling Zones** (Day 1)

**File:** `engine/renderer/src/renderer.rs` (example usage)

```rust
use profiling::{profile_function, profile_zone};

impl Renderer {
    pub fn render_frame(&mut self) -> Result<(), RendererError> {
        profile_function!();

        {
            profile_zone!("Acquire Image");
            self.acquire_swapchain_image()?;
        }

        {
            profile_zone!("Record Commands");
            self.record_command_buffer()?;
        }

        {
            profile_zone!("Submit Queue");
            self.submit_commands()?;
        }

        {
            profile_zone!("Present");
            self.present_image()?;
        }

        profiling::frame_mark();

        Ok(())
    }

    fn update_scene(&mut self) {
        profile_function!();

        // Update logic...
    }
}
```

---

### **3. Built-in Profiling UI** (Day 2)

**File:** `engine/profiling/src/ui.rs`

```rust
use egui::{Context, Window, plot::*};
use std::collections::VecDeque;

/// Profiling metrics
#[derive(Debug, Clone, Default)]
pub struct FrameMetrics {
    pub frame_time_ms: f32,
    pub fps: f32,
    pub cpu_time_ms: f32,
    pub gpu_time_ms: f32,
    pub draw_calls: u32,
    pub triangles: u32,
    pub memory_used_mb: f32,
}

/// Profiling UI
pub struct ProfilingUI {
    show_window: bool,
    frame_history: VecDeque<FrameMetrics>,
    max_history_size: usize,
    current_metrics: FrameMetrics,
}

impl ProfilingUI {
    pub fn new() -> Self {
        Self {
            show_window: false,
            frame_history: VecDeque::new(),
            max_history_size: 300, // 5 seconds at 60 FPS
            current_metrics: FrameMetrics::default(),
        }
    }

    /// Toggle window visibility
    pub fn toggle(&mut self) {
        self.show_window = !self.show_window;
    }

    /// Update metrics
    pub fn update_metrics(&mut self, metrics: FrameMetrics) {
        self.current_metrics = metrics.clone();

        self.frame_history.push_back(metrics);
        if self.frame_history.len() > self.max_history_size {
            self.frame_history.pop_front();
        }
    }

    /// Render UI
    pub fn render(&mut self, ctx: &Context) {
        if !self.show_window {
            return;
        }

        Window::new("Performance Profiler")
            .default_width(600.0)
            .default_height(400.0)
            .show(ctx, |ui| {
                // Current frame stats
                ui.heading("Current Frame");
                ui.separator();

                ui.horizontal(|ui| {
                    ui.label(format!("FPS: {:.1}", self.current_metrics.fps));
                    ui.label(format!("Frame Time: {:.2} ms", self.current_metrics.frame_time_ms));
                });

                ui.horizontal(|ui| {
                    ui.label(format!("CPU: {:.2} ms", self.current_metrics.cpu_time_ms));
                    ui.label(format!("GPU: {:.2} ms", self.current_metrics.gpu_time_ms));
                });

                ui.horizontal(|ui| {
                    ui.label(format!("Draw Calls: {}", self.current_metrics.draw_calls));
                    ui.label(format!("Triangles: {}", self.current_metrics.triangles));
                });

                ui.label(format!("Memory: {:.2} MB", self.current_metrics.memory_used_mb));

                ui.separator();

                // Frame time graph
                ui.heading("Frame Time History");

                let frame_times: PlotPoints = self
                    .frame_history
                    .iter()
                    .enumerate()
                    .map(|(i, m)| [i as f64, m.frame_time_ms as f64])
                    .collect();

                Plot::new("frame_time_plot")
                    .height(150.0)
                    .allow_drag(false)
                    .allow_zoom(false)
                    .show(ui, |plot_ui| {
                        plot_ui.line(Line::new(frame_times).name("Frame Time (ms)"));

                        // 60 FPS line
                        plot_ui.hline(HLine::new(16.67).name("60 FPS").color(egui::Color32::GREEN));

                        // 30 FPS line
                        plot_ui.hline(HLine::new(33.33).name("30 FPS").color(egui::Color32::YELLOW));
                    });

                ui.separator();

                // FPS graph
                ui.heading("FPS History");

                let fps_points: PlotPoints = self
                    .frame_history
                    .iter()
                    .enumerate()
                    .map(|(i, m)| [i as f64, m.fps as f64])
                    .collect();

                Plot::new("fps_plot")
                    .height(150.0)
                    .allow_drag(false)
                    .allow_zoom(false)
                    .show(ui, |plot_ui| {
                        plot_ui.line(Line::new(fps_points).name("FPS"));
                        plot_ui.hline(HLine::new(60.0).name("60 FPS").color(egui::Color32::GREEN));
                    });

                ui.separator();

                // Memory graph
                ui.heading("Memory Usage");

                let memory_points: PlotPoints = self
                    .frame_history
                    .iter()
                    .enumerate()
                    .map(|(i, m)| [i as f64, m.memory_used_mb as f64])
                    .collect();

                Plot::new("memory_plot")
                    .height(150.0)
                    .allow_drag(false)
                    .allow_zoom(false)
                    .show(ui, |plot_ui| {
                        plot_ui.line(Line::new(memory_points).name("Memory (MB)"));
                    });

                ui.separator();

                // Export button
                if ui.button("Export Data").clicked() {
                    self.export_data();
                }
            });
    }

    /// Export profiling data
    fn export_data(&self) {
        use std::fs::File;
        use std::io::Write;

        let path = format!("profiling_data_{}.csv", chrono::Utc::now().timestamp());
        let mut file = File::create(&path).unwrap();

        writeln!(file, "Frame,FPS,FrameTime,CPU,GPU,DrawCalls,Triangles,Memory").unwrap();

        for (i, metrics) in self.frame_history.iter().enumerate() {
            writeln!(
                file,
                "{},{:.2},{:.2},{:.2},{:.2},{},{},{:.2}",
                i,
                metrics.fps,
                metrics.frame_time_ms,
                metrics.cpu_time_ms,
                metrics.gpu_time_ms,
                metrics.draw_calls,
                metrics.triangles,
                metrics.memory_used_mb
            )
            .unwrap();
        }

        tracing::info!("Profiling data exported to {}", path);
    }
}
```

---

### **4. Performance Metrics Collector** (Day 2)

**File:** `engine/profiling/src/metrics.rs`

```rust
use std::time::{Duration, Instant};

/// Performance metrics collector
pub struct MetricsCollector {
    frame_start: Instant,
    cpu_start: Instant,
    draw_calls: u32,
    triangles: u32,
    frame_times: Vec<Duration>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            frame_start: Instant::now(),
            cpu_start: Instant::now(),
            draw_calls: 0,
            triangles: 0,
            frame_times: Vec::new(),
        }
    }

    /// Begin frame
    pub fn begin_frame(&mut self) {
        self.frame_start = Instant::now();
        self.cpu_start = Instant::now();
        self.draw_calls = 0;
        self.triangles = 0;
    }

    /// End frame
    pub fn end_frame(&mut self) -> FrameMetrics {
        let frame_time = self.frame_start.elapsed();
        self.frame_times.push(frame_time);

        // Keep last 100 frames
        if self.frame_times.len() > 100 {
            self.frame_times.remove(0);
        }

        let frame_time_ms = frame_time.as_secs_f32() * 1000.0;
        let fps = if frame_time_ms > 0.0 {
            1000.0 / frame_time_ms
        } else {
            0.0
        };

        let cpu_time_ms = self.cpu_start.elapsed().as_secs_f32() * 1000.0;

        FrameMetrics {
            frame_time_ms,
            fps,
            cpu_time_ms,
            gpu_time_ms: 0.0, // TODO: GPU timing queries
            draw_calls: self.draw_calls,
            triangles: self.triangles,
            memory_used_mb: self.get_memory_usage_mb(),
        }
    }

    /// Record draw call
    pub fn record_draw_call(&mut self, triangle_count: u32) {
        self.draw_calls += 1;
        self.triangles += triangle_count;
    }

    /// Get average frame time
    pub fn average_frame_time(&self) -> Duration {
        if self.frame_times.is_empty() {
            return Duration::ZERO;
        }

        let total: Duration = self.frame_times.iter().sum();
        total / self.frame_times.len() as u32
    }

    /// Get average FPS
    pub fn average_fps(&self) -> f32 {
        let avg_time = self.average_frame_time();
        if avg_time.as_secs_f32() > 0.0 {
            1.0 / avg_time.as_secs_f32()
        } else {
            0.0
        }
    }

    /// Get memory usage in MB (platform-specific)
    fn get_memory_usage_mb(&self) -> f32 {
        #[cfg(target_os = "windows")]
        {
            use winapi::um::processthreadsapi::GetCurrentProcess;
            use winapi::um::psapi::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};

            unsafe {
                let mut pmc: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
                pmc.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;

                if GetProcessMemoryInfo(
                    GetCurrentProcess(),
                    &mut pmc as *mut _,
                    pmc.cb,
                ) != 0
                {
                    return pmc.WorkingSetSize as f32 / (1024.0 * 1024.0);
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                for line in status.lines() {
                    if line.starts_with("VmRSS:") {
                        if let Some(value) = line.split_whitespace().nth(1) {
                            if let Ok(kb) = value.parse::<f32>() {
                                return kb / 1024.0;
                            }
                        }
                    }
                }
            }
        }

        0.0
    }
}
```

---

### **5. GPU Timing Queries** (Day 3)

**File:** `engine/renderer/src/profiling/gpu_timer.rs`

```rust
use ash::vk;

/// GPU timer using timestamp queries
pub struct GpuTimer {
    query_pool: vk::QueryPool,
    query_count: u32,
    current_query: u32,
}

impl GpuTimer {
    /// Create GPU timer
    pub fn new(device: &ash::Device, max_queries: u32) -> Result<Self, RendererError> {
        let create_info = vk::QueryPoolCreateInfo::builder()
            .query_type(vk::QueryType::TIMESTAMP)
            .query_count(max_queries * 2); // Start and end for each query

        let query_pool = unsafe {
            device
                .create_query_pool(&create_info, None)
                .map_err(|e| RendererError::VulkanInit {
                    details: format!("Failed to create query pool: {}", e),
                })?
        };

        Ok(Self {
            query_pool,
            query_count: max_queries,
            current_query: 0,
        })
    }

    /// Reset queries
    pub fn reset(&mut self, device: &ash::Device, command_buffer: vk::CommandBuffer) {
        unsafe {
            device.cmd_reset_query_pool(command_buffer, self.query_pool, 0, self.query_count * 2);
        }
        self.current_query = 0;
    }

    /// Begin GPU timing
    pub fn begin(
        &mut self,
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
    ) -> GpuTimingScope {
        let query_id = self.current_query;
        self.current_query += 1;

        unsafe {
            device.cmd_write_timestamp(
                command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                self.query_pool,
                query_id * 2,
            );
        }

        GpuTimingScope {
            query_pool: self.query_pool,
            query_id,
        }
    }

    /// End GPU timing
    pub fn end(
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
        scope: GpuTimingScope,
    ) {
        unsafe {
            device.cmd_write_timestamp(
                command_buffer,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                scope.query_pool,
                scope.query_id * 2 + 1,
            );
        }
    }

    /// Get timing results
    pub fn get_results(
        &self,
        device: &ash::Device,
        timestamp_period: f32, // nanoseconds per timestamp unit
    ) -> Result<Vec<f32>, RendererError> {
        let mut results = vec![0u64; (self.current_query * 2) as usize];

        unsafe {
            device
                .get_query_pool_results(
                    self.query_pool,
                    0,
                    self.current_query * 2,
                    &mut results,
                    vk::QueryResultFlags::TYPE_64,
                )
                .map_err(|e| RendererError::QueryFailed {
                    details: e.to_string(),
                })?;
        }

        let mut timings = Vec::new();

        for i in 0..self.current_query {
            let start = results[(i * 2) as usize];
            let end = results[(i * 2 + 1) as usize];
            let duration_ns = (end - start) as f32 * timestamp_period;
            let duration_ms = duration_ns / 1_000_000.0;
            timings.push(duration_ms);
        }

        Ok(timings)
    }
}

impl Drop for GpuTimer {
    fn drop(&mut self) {
        // Query pool destroyed by device
    }
}

/// GPU timing scope (RAII)
pub struct GpuTimingScope {
    query_pool: vk::QueryPool,
    query_id: u32,
}
```

---

## ✅ **Acceptance Criteria**

- [ ] Tracy profiler integration with feature flag
- [ ] Profiling macros (zone, function, frame_mark)
- [ ] Built-in profiling UI with egui
- [ ] Frame time graph (last 300 frames)
- [ ] FPS graph
- [ ] Memory usage graph
- [ ] Export profiling data (CSV)
- [ ] GPU timing queries
- [ ] Memory allocation tracking
- [ ] <0.1ms overhead when profiling enabled

---

## 🧪 **Tests**

```rust
#[test]
fn test_metrics_collector() {
    let mut collector = MetricsCollector::new();

    collector.begin_frame();
    std::thread::sleep(Duration::from_millis(16));
    collector.record_draw_call(1000);

    let metrics = collector.end_frame();

    assert!(metrics.frame_time_ms > 15.0);
    assert_eq!(metrics.draw_calls, 1);
    assert_eq!(metrics.triangles, 1000);
}

#[test]
fn test_gpu_timer() {
    // Create device and command buffer
    // ...

    let mut timer = GpuTimer::new(&device, 10).unwrap();
    timer.reset(&device, command_buffer);

    let scope = timer.begin(&device, command_buffer);
    // ... GPU work
    GpuTimer::end(&device, command_buffer, scope);

    let results = timer.get_results(&device, 1.0).unwrap();
    assert!(results.len() > 0);
}
```

---

## ⚡ **Performance Targets**

- **Profiling Overhead:** <0.1ms per frame with Tracy enabled
- **UI Rendering:** <1ms per frame
- **Memory Overhead:** <10 MB for profiling data
- **Export Time:** <100ms for 300 frames

---

## 📚 **Dependencies**

```toml
[dependencies]
tracy-client = { version = "0.17", optional = true }
egui = "0.28"
chrono = "0.4"

[target.'cfg(windows)'.dependencies]
winapi = { version = "0.3", features = ["processthreadsapi", "psapi"] }
```

---

**Dependencies:** [phase4-lighting.md](phase4-lighting.md)
**Next:** [phase4-hot-reload.md](phase4-hot-reload.md)
