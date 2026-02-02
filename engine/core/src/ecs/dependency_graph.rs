//! Dependency graph analysis for system scheduling
//!
//! This module provides the dependency graph data structure and algorithms for:
//! - Building dependency graphs from system access patterns
//! - Detecting circular dependencies
//! - Topological sorting for execution order
//! - Identifying parallel execution stages
//!
//! # Algorithm
//!
//! 1. Build directed graph where edge A -> B means "A must run before B"
//! 2. Detect cycles using depth-first search
//! 3. Topologically sort systems into execution order
//! 4. Group independent systems into parallel stages
//!
//! # Examples
//!
//! ```
//! use engine_core::ecs::dependency_graph::{DependencyGraph, SystemNode};
//! use engine_core::ecs::SystemAccess;
//!
//! let mut graph = DependencyGraph::new();
//!
//! // Add systems
//! // graph.add_node(SystemNode { ... });
//!
//! // Analyze dependencies
//! graph.analyze_dependencies();
//!
//! // Check for cycles
//! if let Err(cycle) = graph.check_for_cycles() {
//!     panic!("Circular dependency: {:?}", cycle);
//! }
//!
//! // Create execution stages
//! graph.create_execution_stages();
//! ```

use super::schedule::SystemAccess;
use std::collections::{HashMap, HashSet, VecDeque};
use tracing::{debug, warn};

/// A node in the dependency graph representing a system
#[derive(Debug, Clone)]
pub struct SystemNode {
    /// Index of the system in the schedule
    pub index: usize,
    /// Name of the system (for debugging)
    pub name: String,
    /// Component access pattern
    pub access: SystemAccess,
}

/// Dependency graph for system scheduling
///
/// This graph tracks which systems must run before others based on
/// their component access patterns.
#[derive(Debug)]
pub struct DependencyGraph {
    /// All system nodes
    nodes: Vec<SystemNode>,
    /// Adjacency list: node index -> list of dependent node indices
    /// If edges[A] contains B, then A must run before B
    edges: HashMap<usize, Vec<usize>>,
    /// Execution stages (groups of systems that can run in parallel)
    stages: Vec<Vec<usize>>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        Self { nodes: Vec::new(), edges: HashMap::new(), stages: Vec::new() }
    }

    /// Add a system node to the graph
    pub fn add_node(&mut self, node: SystemNode) {
        debug!(
            system = %node.name,
            index = node.index,
            "Adding system to dependency graph"
        );
        self.nodes.push(node);
    }

    /// Analyze dependencies between all systems
    ///
    /// This builds the dependency graph by checking component access conflicts.
    /// The key insight: if two systems conflict, we need an ordering between them.
    /// We add an edge from the first system to the second, ensuring deterministic ordering.
    pub fn analyze_dependencies(&mut self) {
        debug!("Analyzing system dependencies");

        self.edges.clear();

        // Check each ordered pair of systems (i < j) to avoid bidirectional edges
        for i in 0..self.nodes.len() {
            for j in (i + 1)..self.nodes.len() {
                let node_i = &self.nodes[i];
                let node_j = &self.nodes[j];

                // Check if there's a conflict between i and j
                if self.systems_conflict(&node_i.access, &node_j.access) {
                    // Systems conflict - add edge to enforce ordering
                    // By convention, lower index runs first
                    self.edges.entry(i).or_default().push(j);
                    debug!(
                        from = %node_i.name,
                        to = %node_j.name,
                        "Dependency edge added (systems conflict)"
                    );
                }
            }
        }
    }

    /// Check if two systems conflict and need to be ordered
    ///
    /// Systems conflict if:
    /// - One writes and the other reads the same component
    /// - Both write to the same component
    ///
    /// Systems that only read the same components don't conflict.
    fn systems_conflict(&self, access_a: &SystemAccess, access_b: &SystemAccess) -> bool {
        // Check if A writes something that B reads or writes
        for write_type in &access_a.writes {
            if access_b.reads.contains(write_type) || access_b.writes.contains(write_type) {
                return true;
            }
        }

        // Check if B writes something that A reads or writes
        for write_type in &access_b.writes {
            if access_a.reads.contains(write_type) || access_a.writes.contains(write_type) {
                return true;
            }
        }

        false
    }

    /// Check for circular dependencies in the graph
    ///
    /// Returns Ok(()) if no cycles found, or Err(cycle) with the cycle path.
    pub fn check_for_cycles(&self) -> Result<(), Vec<String>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for node in &self.nodes {
            if !visited.contains(&node.index) {
                if let Some(cycle) = self.detect_cycle_dfs(node.index, &mut visited, &mut rec_stack)
                {
                    // Build cycle path with system names
                    let cycle_names: Vec<String> =
                        cycle.iter().map(|&idx| self.nodes[idx].name.clone()).collect();
                    return Err(cycle_names);
                }
            }
        }

        Ok(())
    }

    /// Depth-first search to detect cycles
    ///
    /// Returns Some(cycle) if a cycle is found, None otherwise.
    fn detect_cycle_dfs(
        &self,
        node: usize,
        visited: &mut HashSet<usize>,
        rec_stack: &mut HashSet<usize>,
    ) -> Option<Vec<usize>> {
        visited.insert(node);
        rec_stack.insert(node);

        if let Some(neighbors) = self.edges.get(&node) {
            for &neighbor in neighbors {
                if !visited.contains(&neighbor) {
                    if let Some(cycle) = self.detect_cycle_dfs(neighbor, visited, rec_stack) {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(&neighbor) {
                    // Found a cycle
                    return Some(vec![node, neighbor]);
                }
            }
        }

        rec_stack.remove(&node);
        None
    }

    /// Create execution stages using topological sort and level assignment
    ///
    /// Systems are grouped into stages where:
    /// - All systems in a stage can run in parallel (no conflicts)
    /// - Stage N runs before stage N+1
    /// - Systems are assigned to the earliest stage possible
    pub fn create_execution_stages(&mut self) {
        debug!("Creating execution stages");

        // Calculate in-degree for each node
        let mut in_degree = HashMap::new();
        for node in &self.nodes {
            in_degree.insert(node.index, 0);
        }

        for neighbors in self.edges.values() {
            for &neighbor in neighbors {
                *in_degree.entry(neighbor).or_insert(0) += 1;
            }
        }

        // Start with nodes that have no dependencies
        let mut queue: VecDeque<usize> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(&idx, _)| idx)
            .collect();

        self.stages.clear();
        let mut current_stage = Vec::new();

        while !queue.is_empty() {
            // Process all nodes at the current level
            let stage_size = queue.len();
            current_stage.clear();

            for _ in 0..stage_size {
                let node = queue.pop_front().unwrap();
                current_stage.push(node);

                // Reduce in-degree of neighbors
                if let Some(neighbors) = self.edges.get(&node) {
                    for &neighbor in neighbors {
                        let degree = in_degree.get_mut(&neighbor).unwrap();
                        *degree -= 1;

                        if *degree == 0 {
                            queue.push_back(neighbor);
                        }
                    }
                }
            }

            // Add stage
            if !current_stage.is_empty() {
                debug!(
                    stage = self.stages.len(),
                    system_count = current_stage.len(),
                    "Created execution stage"
                );
                self.stages.push(current_stage.clone());
            }
        }

        // Verify all nodes were processed
        if self.stages.iter().flatten().count() != self.nodes.len() {
            warn!("Not all systems were scheduled. This may indicate a circular dependency.");
        }
    }

    /// Get the execution stages
    pub fn stages(&self) -> &[Vec<usize>] {
        &self.stages
    }

    /// Get the number of execution stages
    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    /// Get the maximum parallelism (largest stage size)
    pub fn max_parallelism(&self) -> usize {
        self.stages.iter().map(|s| s.len()).max().unwrap_or(0)
    }

    /// Get information about the graph for debugging
    pub fn debug_info(&self) -> String {
        let mut info = format!("DependencyGraph with {} systems:\n", self.nodes.len());

        info.push_str("\nDependencies:\n");
        for (from, tos) in &self.edges {
            let from_name = &self.nodes[*from].name;
            for to in tos {
                let to_name = &self.nodes[*to].name;
                info.push_str(&format!("  {} -> {}\n", from_name, to_name));
            }
        }

        info.push_str(&format!("\nExecution Stages ({}): \n", self.stages.len()));
        for (stage_idx, stage) in self.stages.iter().enumerate() {
            info.push_str(&format!("  Stage {}: ", stage_idx));
            let names: Vec<&str> = stage.iter().map(|&idx| self.nodes[idx].name.as_str()).collect();
            info.push_str(&names.join(", "));
            info.push('\n');
        }

        info
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_node(index: usize, name: &str) -> SystemNode {
        SystemNode { index, name: name.to_string(), access: SystemAccess::new() }
    }

    #[test]
    fn test_empty_graph() {
        let graph = DependencyGraph::new();
        assert_eq!(graph.nodes.len(), 0);
        assert_eq!(graph.stage_count(), 0);
    }

    #[test]
    fn test_single_node() {
        let mut graph = DependencyGraph::new();
        graph.add_node(create_test_node(0, "System1"));
        graph.analyze_dependencies();
        graph.create_execution_stages();

        assert_eq!(graph.stage_count(), 1);
        assert_eq!(graph.stages()[0].len(), 1);
    }

    #[test]
    fn test_independent_systems() {
        let mut graph = DependencyGraph::new();

        // Two systems with no conflicts
        graph.add_node(create_test_node(0, "System1"));
        graph.add_node(create_test_node(1, "System2"));

        graph.analyze_dependencies();
        graph.create_execution_stages();

        // Should be in one stage (parallel)
        assert_eq!(graph.stage_count(), 1);
        assert_eq!(graph.stages()[0].len(), 2);
    }

    #[test]
    fn test_sequential_systems() {
        let mut graph = DependencyGraph::new();

        // Add edge manually for testing
        let mut node1 = create_test_node(0, "System1");
        let mut node2 = create_test_node(1, "System2");

        // System1 writes, System2 reads -> dependency
        use crate::ecs::Component;

        #[derive(Debug)]
        struct TestComp;
        impl Component for TestComp {}

        node1.access = SystemAccess::new().writes::<TestComp>();
        node2.access = SystemAccess::new().reads::<TestComp>();

        graph.add_node(node1);
        graph.add_node(node2);

        graph.analyze_dependencies();
        graph.create_execution_stages();

        // Should be in two stages (sequential)
        assert_eq!(graph.stage_count(), 2);
        assert_eq!(graph.stages()[0].len(), 1);
        assert_eq!(graph.stages()[1].len(), 1);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::new();

        graph.add_node(create_test_node(0, "System1"));
        graph.add_node(create_test_node(1, "System2"));

        // Create a cycle manually
        graph.edges.insert(0, vec![1]);
        graph.edges.insert(1, vec![0]);

        let result = graph.check_for_cycles();
        assert!(result.is_err());
    }

    #[test]
    fn test_no_cycle() {
        let mut graph = DependencyGraph::new();

        graph.add_node(create_test_node(0, "System1"));
        graph.add_node(create_test_node(1, "System2"));
        graph.add_node(create_test_node(2, "System3"));

        // Linear dependency chain
        graph.edges.insert(0, vec![1]);
        graph.edges.insert(1, vec![2]);

        let result = graph.check_for_cycles();
        assert!(result.is_ok());
    }

    #[test]
    fn test_complex_graph() {
        let mut graph = DependencyGraph::new();

        // Create a complex dependency structure:
        //   0 -> 2
        //   1 -> 2
        //   2 -> 3
        //   2 -> 4
        //
        // Expected stages:
        // Stage 0: [0, 1] (parallel)
        // Stage 1: [2]
        // Stage 2: [3, 4] (parallel)

        for i in 0..5 {
            graph.add_node(create_test_node(i, &format!("System{}", i)));
        }

        graph.edges.insert(0, vec![2]);
        graph.edges.insert(1, vec![2]);
        graph.edges.insert(2, vec![3, 4]);

        graph.create_execution_stages();

        assert_eq!(graph.stage_count(), 3);
        assert_eq!(graph.stages()[0].len(), 2); // 0 and 1
        assert_eq!(graph.stages()[1].len(), 1); // 2
        assert_eq!(graph.stages()[2].len(), 2); // 3 and 4
    }

    #[test]
    fn test_max_parallelism() {
        let mut graph = DependencyGraph::new();

        for i in 0..5 {
            graph.add_node(create_test_node(i, &format!("System{}", i)));
        }

        graph.create_execution_stages();

        // All independent systems should be in one stage
        assert_eq!(graph.max_parallelism(), 5);
    }
}
