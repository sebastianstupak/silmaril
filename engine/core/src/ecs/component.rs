//! Component trait and metadata
//!
//! Components are pure data structures that can be attached to entities.
//! They must be 'static, Send, and Sync for safe multithreading.

use std::any::TypeId;

/// Marker trait for components
///
/// Components are pure data, no methods (use systems for behavior).
/// They must be 'static (no lifetimes), Send + Sync for thread safety.
///
/// # Examples
///
/// ```
/// use engine_core::ecs::Component;
///
/// #[derive(Debug, Clone)]
/// struct Position {
///     x: f32,
///     y: f32,
///     z: f32,
/// }
///
/// impl Component for Position {}
/// ```
pub trait Component: 'static + Send + Sync {}

/// Component metadata for debugging and introspection
///
/// Stores type information about a component including its TypeId,
/// name, size, and alignment.
#[derive(Debug, Clone, Copy)]
pub struct ComponentDescriptor {
    /// The TypeId of the component
    pub type_id: TypeId,
    /// The type name as a string (for debugging)
    pub type_name: &'static str,
    /// Size of the component in bytes
    pub size: usize,
    /// Alignment requirement of the component
    pub align: usize,
}

impl ComponentDescriptor {
    /// Create a new component descriptor for type T
    ///
    /// # Examples
    ///
    /// ```
    /// # use engine_core::ecs::{Component, ComponentDescriptor};
    /// # struct Health { current: f32, max: f32 }
    /// # impl Component for Health {}
    /// let descriptor = ComponentDescriptor::new::<Health>();
    /// println!("Component: {}", descriptor.type_name);
    /// ```
    pub fn new<T: Component>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
            size: std::mem::size_of::<T>(),
            align: std::mem::align_of::<T>(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy)]
    #[allow(dead_code)]
    struct TestComponent {
        value: i32,
    }

    impl Component for TestComponent {}

    #[test]
    fn test_component_descriptor() {
        let descriptor = ComponentDescriptor::new::<TestComponent>();

        assert_eq!(descriptor.type_id, TypeId::of::<TestComponent>());
        assert_eq!(descriptor.size, std::mem::size_of::<TestComponent>());
        assert_eq!(descriptor.align, std::mem::align_of::<TestComponent>());
        assert!(descriptor.type_name.contains("TestComponent"));
    }

    #[derive(Debug)]
    #[allow(dead_code)]
    struct AnotherTestComponent {
        x: f64,
        y: f64,
    }

    impl Component for AnotherTestComponent {}

    #[test]
    fn test_component_descriptor_different_types() {
        let desc1 = ComponentDescriptor::new::<TestComponent>();
        let desc2 = ComponentDescriptor::new::<AnotherTestComponent>();

        assert_ne!(desc1.type_id, desc2.type_id);
        assert_ne!(desc1.size, desc2.size);
    }
}
