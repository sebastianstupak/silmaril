# Engine Observability

## Purpose
The observability crate provides performance monitoring and profiling:
- **Metrics Collection**: CPU, GPU, memory, and network metrics
- **Frame Profiling**: Per-frame timing breakdown
- **Integration**: Integrates with external profilers (Tracy, Optick)
- **Performance Targets**: Validate against performance requirements
- **Real-time Display**: In-game performance overlay

## MUST READ Documentation
Before working on this crate, read these documents in order:

1. **[phase4-profiling-integration.md](../../docs/phase4-profiling-integration.md)** - Profiling integration design
2. **[performance-targets.md](../../docs/performance-targets.md)** - Performance requirements

## Related Crates
- **engine-renderer**: GPU profiling and frame timing
- **engine-networking**: Network bandwidth monitoring
- **engine-core**: ECS system timing

## Quick Example
```rust
use engine_observability::{Profiler, Metrics};

fn game_loop(profiler: &mut Profiler) {
    profiler.begin_frame();

    {
        let _scope = profiler.scope("Physics");
        physics_step();
    }

    {
        let _scope = profiler.scope("Rendering");
        render_frame();
    }

    profiler.end_frame();

    // Display metrics
    let metrics = profiler.get_metrics();
    println!("FPS: {}, Frame Time: {}ms", metrics.fps, metrics.frame_time);
}
```

## Key Dependencies
- `tracy-client` - Tracy profiler integration
- `puffin` - Puffin profiler integration

## Performance Targets
- <0.1ms overhead per frame
- Real-time metrics display at 60 FPS
- Integration with external profilers (Tracy, Optick)
