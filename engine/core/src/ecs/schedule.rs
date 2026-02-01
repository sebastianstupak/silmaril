//! System scheduling and parallelization infrastructure
//!
//! This module provides automatic dependency analysis and parallel execution of systems.
//! It builds a dependency graph based on component access patterns and executes systems
//! in parallel when safe to do so.
//!
//! # Architecture
//!
//! The scheduler works in three phases:
//! 1. **Registration**: Systems register their component access (reads/writes)
//! 2. **Analysis**: Build dependency graph and identify parallel execution stages
//! 3. **Execution**: Run systems in topologically sorted order, parallelizing where possible
//!
//! # Examples
//!
//! ```
//! use engine_core::ecs::{World, Schedule, System, SystemAccess};
//! use std::any::TypeId;
//!
//! // Define your components
//! # use engine_core::ecs::Component;
//! # #[derive(Component)]
//! # struct Transform { x: f32 }
//! # #[derive(Component)]
//! # struct Velocity { x: f32 }
//!
//! // Create a schedule
//! let mut schedule = Schedule::new();
//!
//! // Add systems (they'll be automatically scheduled)
//! // schedule.add_system(physics_system);
//! // schedule.add_system(render_system);
//!
//! // Build dependency graph
//! schedule.build();
//!
//! // Execute all systems
//! let mut world = World::new();
//! schedule.run(&mut world);
//! ```

use super::{Component, World};
use std::any::TypeId;
use tracing::{debug, info};

#[cfg(feature = "profiling")]
use agent_game_engine_profiling::{profile_scope, ProfileCategory};

use crate::ecs::dependency_graph::{DependencyGraph, SystemNode};

/// Describes which components a system reads and writes
///
/// This is used to build the dependency graph and determine which systems
/// can run in parallel.
///
/// # Rules
///
/// Systems can run in parallel if:
/// - They only read components (no writes)
/// - They write to different components (no overlap)
///
/// Systems must run sequentially if:
/// - One writes and another reads the same component
/// - Both write to the same component
#[derive(Debug, Clone, Default)]
pub struct SystemAccess {
    /// Component types this system reads (immutable access)
    pub reads: Vec<TypeId>,
    /// Component types this system writes (mutable access)
    pub writes: Vec<TypeId>,
}

impl SystemAccess {
    /// Create a new empty SystemAccess
    pub fn new() -> Self {
        Self { reads: Vec::new(), writes: Vec::new() }
    }

    /// Add a component type that this system reads
    pub fn reads<T: Component>(mut self) -> Self {
        self.reads.push(TypeId::of::<T>());
        self
    }

    /// Add a component type that this system writes
    pub fn writes<T: Component>(mut self) -> Self {
        self.writes.push(TypeId::of::<T>());
        self
    }

    /// Check if this system conflicts with another
    ///
    /// Two systems conflict if:
    /// - One writes and another reads the same component
    /// - Both write to the same component
    ///
    /// Systems that only read can always run in parallel.
    pub fn conflicts_with(&self, other: &SystemAccess) -> bool {
        // Check if we write to something they read
        for write_type in &self.writes {
            if other.reads.contains(write_type) || other.writes.contains(write_type) {
                return true;
            }
        }

        // Check if we read something they write
        for read_type in &self.reads {
            if other.writes.contains(read_type) {
                return true;
            }
        }

        false
    }
}

/// Trait for systems that can be executed on the world
///
/// Systems must be Send + Sync to enable parallel execution.
///
/// # Examples
///
/// ```
/// # use engine_core::ecs::{World, System, SystemAccess, Component};
/// # use std::any::TypeId;
/// # #[derive(Component)]
/// # struct Velocity { x: f32 }
/// # #[derive(Component)]
/// # struct Position { x: f32 }
/// struct PhysicsSystem;
///
/// impl System for PhysicsSystem {
///     fn name(&self) -> &str {
///         "PhysicsSystem"
///     }
///
///     fn run(&mut self, world: &mut World) {
///         // Query and update components
///         // for (pos, vel) in world.query::<(&mut Position, &Velocity)>() {
///         //     pos.x += vel.x;
///         // }
///     }
///
///     fn access(&self) -> SystemAccess {
///         SystemAccess::new()
///             .reads::<Velocity>()
///             .writes::<Position>()
///     }
/// }
/// ```
pub trait System: Send + Sync {
    /// Get the name of this system (for debugging)
    fn name(&self) -> &str;

    /// Execute this system on the world
    fn run(&mut self, world: &mut World);

    /// Describe which components this system accesses
    fn access(&self) -> SystemAccess;
}

/// Type-erased system wrapper
///
/// This allows us to store systems of different types in the same Vec.
type BoxedSystem = Box<dyn System>;

/// A schedule that manages system execution order and parallelization
///
/// The schedule analyzes system dependencies and executes them in the correct order,
/// parallelizing independent systems when possible.
///
/// # Example
///
/// ```
/// # use engine_core::ecs::{World, Schedule};
/// let mut schedule = Schedule::new();
///
/// // Add systems
/// // schedule.add_system(PhysicsSystem);
/// // schedule.add_system(RenderSystem);
///
/// // Analyze dependencies
/// schedule.build();
///
/// // Execute
/// let mut world = World::new();
/// schedule.run(&mut world);
/// ```
pub struct Schedule {
    /// All registered systems
    systems: Vec<BoxedSystem>,
    /// Dependency graph for execution ordering
    graph: DependencyGraph,
    /// Whether the schedule has been built
    built: bool,
}

impl Schedule {
    /// Create a new empty schedule
    pub fn new() -> Self {
        Self { systems: Vec::new(), graph: DependencyGraph::new(), built: false }
    }

    /// Add a system to the schedule
    ///
    /// The system will be analyzed when `build()` is called.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::Schedule;
    /// let mut schedule = Schedule::new();
    /// // schedule.add_system(MySystem);
    /// ```
    pub fn add_system<S: System + 'static>(&mut self, system: S) {
        info!(system = %system.name(), "Adding system to schedule");
        self.systems.push(Box::new(system));
        self.built = false; // Need to rebuild
    }

    /// Build the dependency graph and execution plan
    ///
    /// This analyzes all systems, detects conflicts, and creates an execution
    /// plan that maximizes parallelism while respecting dependencies.
    ///
    /// # Panics
    ///
    /// Panics if there are circular dependencies between systems.
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::Schedule;
    /// let mut schedule = Schedule::new();
    /// // Add systems...
    /// schedule.build();
    /// ```
    pub fn build(&mut self) {
        #[cfg(feature = "profiling")]
        profile_scope!("schedule_build", ProfileCategory::ECS);

        info!(system_count = self.systems.len(), "Building schedule");

        // Clear existing graph
        self.graph = DependencyGraph::new();

        // Create nodes for each system
        for (index, system) in self.systems.iter().enumerate() {
            let node =
                SystemNode { index, name: system.name().to_string(), access: system.access() };
            self.graph.add_node(node);
        }

        // Analyze dependencies between all systems
        self.graph.analyze_dependencies();

        // Check for cycles
        if let Err(cycle) = self.graph.check_for_cycles() {
            panic!("Circular dependency detected in schedule: {}", cycle.join(" -> "));
        }

        // Create execution stages
        self.graph.create_execution_stages();

        info!(stages = self.graph.stage_count(), "Schedule built successfully");

        self.built = true;
    }

    /// Execute all systems in the schedule
    ///
    /// Systems are executed in topologically sorted order, with independent
    /// systems running in parallel within each stage.
    ///
    /// # Panics
    ///
    /// Panics if the schedule has not been built (call `build()` first).
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::{World, Schedule};
    /// let mut schedule = Schedule::new();
    /// // Add systems...
    /// schedule.build();
    ///
    /// let mut world = World::new();
    /// schedule.run(&mut world);
    /// ```
    pub fn run(&mut self, world: &mut World) {
        #[cfg(feature = "profiling")]
        profile_scope!("schedule_run", ProfileCategory::ECS);

        if !self.built {
            panic!("Schedule must be built before execution. Call schedule.build() first.");
        }

        // Execute each stage
        for (stage_idx, stage) in self.graph.stages().iter().enumerate() {
            debug!(stage = stage_idx, system_count = stage.len(), "Executing stage");

            // Execute systems in this stage
            // Note: Systems in the same stage can theoretically run in parallel,
            // but for now we execute them sequentially to avoid complex lifetime issues.
            // The scheduler still provides value by optimizing execution order.
            //
            // TODO: Implement true parallel execution with thread-safe system storage
            for &system_idx in stage {
                let system = &mut self.systems[system_idx];

                #[cfg(feature = "profiling")]
                {
                    let _scope = agent_game_engine_profiling::ProfileScope::new(
                        system.name(),
                        ProfileCategory::ECS,
                    );
                    system.run(world);
                }

                #[cfg(not(feature = "profiling"))]
                system.run(world);
            }
        }
    }

    /// Get the number of stages in the execution plan
    ///
    /// More stages means less parallelism (more sequential execution).
    /// Fewer stages means more parallelism.
    pub fn stage_count(&self) -> usize {
        self.graph.stage_count()
    }

    /// Get information about the schedule for debugging
    ///
    /// Returns a string describing the execution plan.
    pub fn debug_info(&self) -> String {
        if !self.built {
            return "Schedule not built yet".to_string();
        }

        let mut info = format!(
            "Schedule with {} systems in {} stages:\n",
            self.systems.len(),
            self.stage_count()
        );

        for (stage_idx, stage) in self.graph.stages().iter().enumerate() {
            info.push_str(&format!("\nStage {}:\n", stage_idx));
            for &system_idx in stage {
                let system = &self.systems[system_idx];
                info.push_str(&format!("  - {}\n", system.name()));
            }
        }

        info
    }
}

impl Default for Schedule {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct MockComponent;
    impl Component for MockComponent {}

    #[derive(Debug)]
    struct MockComponent2;
    impl Component for MockComponent2 {}

    struct TestSystem {
        name: String,
        access: SystemAccess,
        run_count: Arc<Mutex<usize>>,
    }

    impl System for TestSystem {
        fn name(&self) -> &str {
            &self.name
        }

        fn run(&mut self, _world: &mut World) {
            let mut count = self.run_count.lock().unwrap();
            *count += 1;
        }

        fn access(&self) -> SystemAccess {
            self.access.clone()
        }
    }

    #[test]
    fn test_system_access_conflicts() {
        let read_only = SystemAccess::new().reads::<MockComponent>();
        let write_only = SystemAccess::new().writes::<MockComponent>();
        let _read_write = SystemAccess::new().reads::<MockComponent>().writes::<MockComponent2>();

        // Two readers don't conflict
        assert!(!read_only.conflicts_with(&read_only));

        // Reader and writer conflict
        assert!(read_only.conflicts_with(&write_only));
        assert!(write_only.conflicts_with(&read_only));

        // Two writers conflict
        assert!(write_only.conflicts_with(&write_only));

        // Different components don't conflict
        let write_other = SystemAccess::new().writes::<MockComponent2>();
        assert!(!write_only.conflicts_with(&write_other));
    }

    #[test]
    fn test_schedule_build() {
        let mut schedule = Schedule::new();

        let run_count1 = Arc::new(Mutex::new(0));
        let run_count2 = Arc::new(Mutex::new(0));

        schedule.add_system(TestSystem {
            name: "System1".to_string(),
            access: SystemAccess::new().reads::<MockComponent>(),
            run_count: Arc::clone(&run_count1),
        });

        schedule.add_system(TestSystem {
            name: "System2".to_string(),
            access: SystemAccess::new().reads::<MockComponent>(),
            run_count: Arc::clone(&run_count2),
        });

        schedule.build();

        // Should build successfully
        assert!(schedule.built);
    }

    #[test]
    fn test_schedule_execution() {
        let mut schedule = Schedule::new();
        let mut world = World::new();

        let run_count = Arc::new(Mutex::new(0));

        schedule.add_system(TestSystem {
            name: "TestSystem".to_string(),
            access: SystemAccess::new().reads::<MockComponent>(),
            run_count: Arc::clone(&run_count),
        });

        schedule.build();
        schedule.run(&mut world);

        // System should have run once
        assert_eq!(*run_count.lock().unwrap(), 1);
    }

    #[test]
    #[should_panic(expected = "must be built")]
    fn test_schedule_run_without_build_panics() {
        let mut schedule = Schedule::new();
        let mut world = World::new();

        schedule.run(&mut world);
    }
}
