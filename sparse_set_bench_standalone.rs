// Standalone benchmark comparing original vs optimized SparseSet
// Run with: rustc -O sparse_set_bench_standalone.rs && ./sparse_set_bench_standalone

use std::time::Instant;

// Minimal Entity implementation
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Entity(u32);

impl Entity {
    fn new(id: u32, _generation: u32) -> Self {
        Entity(id)
    }

    fn id(&self) -> u32 {
        self.0
    }
}

// Minimal Component trait
trait Component: 'static {}

// Original SparseSet implementation
mod original {
    use super::*;

    pub struct SparseSet<T: Component> {
        sparse: Vec<Option<usize>>,
        dense: Vec<Entity>,
        components: Vec<T>,
    }

    impl<T: Component> SparseSet<T> {
        pub fn new() -> Self {
            Self {
                sparse: Vec::new(),
                dense: Vec::new(),
                components: Vec::new(),
            }
        }

        pub fn with_capacity(capacity: usize) -> Self {
            Self {
                sparse: Vec::new(),
                dense: Vec::with_capacity(capacity),
                components: Vec::with_capacity(capacity),
            }
        }

        pub fn insert(&mut self, entity: Entity, component: T) {
            let idx = entity.id() as usize;
            if idx >= self.sparse.len() {
                self.sparse.resize(idx + 1, None);
            }

            if let Some(dense_idx) = self.sparse[idx] {
                self.components[dense_idx] = component;
            } else {
                let dense_idx = self.dense.len();
                self.sparse[idx] = Some(dense_idx);
                self.dense.push(entity);
                self.components.push(component);
            }
        }

        pub fn remove(&mut self, entity: Entity) -> Option<T> {
            let idx = entity.id() as usize;
            let dense_idx = self.sparse.get_mut(idx)?.take()?;

            let last_idx = self.dense.len() - 1;
            if dense_idx != last_idx {
                self.dense.swap(dense_idx, last_idx);
                self.components.swap(dense_idx, last_idx);
                let swapped_entity = self.dense[dense_idx];
                let swapped_id = swapped_entity.id() as usize;
                self.sparse[swapped_id] = Some(dense_idx);
            }

            self.dense.pop();
            Some(self.components.pop().unwrap())
        }

        #[inline]
        pub fn get(&self, entity: Entity) -> Option<&T> {
            let idx = entity.id() as usize;
            let dense_idx = *self.sparse.get(idx)?.as_ref()?;
            Some(&self.components[dense_idx])
        }

        #[inline]
        pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
            let idx = entity.id() as usize;
            let dense_idx = *self.sparse.get(idx)?.as_ref()?;
            Some(&mut self.components[dense_idx])
        }

        pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
            self.dense.iter().copied().zip(self.components.iter())
        }

        pub fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
            self.dense.iter().copied().zip(self.components.iter_mut())
        }

        pub fn len(&self) -> usize {
            self.dense.len()
        }
    }
}

// Optimized SparseSet implementation
mod optimized {
    use super::*;

    pub struct SparseSet<T: Component> {
        sparse: Vec<Option<usize>>,
        dense: Vec<Entity>,
        components: Vec<T>,
    }

    impl<T: Component> SparseSet<T> {
        #[inline]
        pub fn new() -> Self {
            Self {
                sparse: Vec::new(),
                dense: Vec::new(),
                components: Vec::new(),
            }
        }

        #[inline]
        pub fn with_capacity(capacity: usize) -> Self {
            Self {
                sparse: Vec::new(),
                dense: Vec::with_capacity(capacity),
                components: Vec::with_capacity(capacity),
            }
        }

        #[inline]
        pub fn insert(&mut self, entity: Entity, component: T) {
            let idx = entity.id() as usize;

            if idx >= self.sparse.len() {
                let new_capacity = (idx + 1).max(self.sparse.len() * 2);
                self.sparse.resize(new_capacity, None);
            }

            if let Some(dense_idx) = self.sparse[idx] {
                unsafe {
                    *self.components.get_unchecked_mut(dense_idx) = component;
                }
                return;
            }

            let dense_idx = self.dense.len();
            self.sparse[idx] = Some(dense_idx);
            self.dense.push(entity);
            self.components.push(component);
        }

        #[inline]
        pub fn remove(&mut self, entity: Entity) -> Option<T> {
            let idx = entity.id() as usize;
            let dense_idx = self.sparse.get_mut(idx)?.take()?;

            let last_idx = self.dense.len() - 1;
            if dense_idx != last_idx {
                unsafe {
                    self.dense.swap(dense_idx, last_idx);
                    self.components.swap(dense_idx, last_idx);
                    let swapped_entity = *self.dense.get_unchecked(dense_idx);
                    let swapped_id = swapped_entity.id() as usize;
                    *self.sparse.get_unchecked_mut(swapped_id) = Some(dense_idx);
                }
            }

            self.dense.pop();
            Some(self.components.pop().unwrap())
        }

        #[inline]
        pub fn get(&self, entity: Entity) -> Option<&T> {
            let idx = entity.id() as usize;
            let dense_idx = *self.sparse.get(idx)?.as_ref()?;
            unsafe { Some(self.components.get_unchecked(dense_idx)) }
        }

        #[inline]
        pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
            let idx = entity.id() as usize;
            let dense_idx = *self.sparse.get(idx)?.as_ref()?;
            unsafe { Some(self.components.get_unchecked_mut(dense_idx)) }
        }

        #[inline]
        pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
            self.dense.iter().copied().zip(self.components.iter())
        }

        #[inline]
        pub fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
            self.dense.iter().copied().zip(self.components.iter_mut())
        }

        #[inline]
        pub fn len(&self) -> usize {
            self.dense.len()
        }
    }
}

// Test components
#[derive(Clone, Copy)]
struct Position {
    x: f32,
    y: f32,
    z: f32,
}
impl Component for Position {}

#[derive(Clone, Copy)]
struct Velocity {
    x: f32,
    y: f32,
    z: f32,
}
impl Component for Velocity {}

// Benchmark utilities
fn bench<F: FnMut()>(name: &str, mut f: F, iterations: usize) -> u128 {
    // Warmup
    for _ in 0..10 {
        f();
    }

    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    let elapsed = start.elapsed().as_nanos();
    let avg_ns = elapsed / iterations as u128;

    println!("{}: {} ns/op", name, avg_ns);
    avg_ns
}

fn main() {
    println!("=== SparseSet Benchmark: Original vs Optimized ===\n");

    let sizes = [100, 1000, 10000, 100000];

    for &size in &sizes {
        println!("--- SIZE: {} ---", size);

        // Benchmark: Bulk Insert
        let orig_insert = bench(
            "  Original insert",
            || {
                let mut storage = original::SparseSet::<Position>::with_capacity(size);
                for i in 0..size {
                    storage.insert(
                        Entity::new(i as u32, 0),
                        Position { x: i as f32, y: i as f32, z: i as f32 },
                    );
                }
                std::hint::black_box(storage);
            },
            10,
        );

        let opt_insert = bench(
            "  Optimized insert",
            || {
                let mut storage = optimized::SparseSet::<Position>::with_capacity(size);
                for i in 0..size {
                    storage.insert(
                        Entity::new(i as u32, 0),
                        Position { x: i as f32, y: i as f32, z: i as f32 },
                    );
                }
                std::hint::black_box(storage);
            },
            10,
        );

        println!("  Speedup: {:.2}x\n", orig_insert as f64 / opt_insert as f64);

        // Benchmark: Get operations
        let mut orig_storage = original::SparseSet::<Position>::with_capacity(size);
        let mut opt_storage = optimized::SparseSet::<Position>::with_capacity(size);
        for i in 0..size {
            let pos = Position { x: i as f32, y: i as f32, z: i as f32 };
            orig_storage.insert(Entity::new(i as u32, 0), pos);
            opt_storage.insert(Entity::new(i as u32, 0), pos);
        }

        let orig_get = bench(
            "  Original get",
            || {
                let mut sum = 0.0;
                for i in 0..size {
                    if let Some(pos) = orig_storage.get(Entity::new(i as u32, 0)) {
                        sum += pos.x;
                    }
                }
                std::hint::black_box(sum);
            },
            100,
        );

        let opt_get = bench(
            "  Optimized get",
            || {
                let mut sum = 0.0;
                for i in 0..size {
                    if let Some(pos) = opt_storage.get(Entity::new(i as u32, 0)) {
                        sum += pos.x;
                    }
                }
                std::hint::black_box(sum);
            },
            100,
        );

        println!("  Speedup: {:.2}x\n", orig_get as f64 / opt_get as f64);

        // Benchmark: Iteration
        let orig_iter = bench(
            "  Original iter",
            || {
                let mut sum = 0.0;
                for (_entity, pos) in orig_storage.iter() {
                    sum += pos.x + pos.y + pos.z;
                }
                std::hint::black_box(sum);
            },
            100,
        );

        let opt_iter = bench(
            "  Optimized iter",
            || {
                let mut sum = 0.0;
                for (_entity, pos) in opt_storage.iter() {
                    sum += pos.x + pos.y + pos.z;
                }
                std::hint::black_box(sum);
            },
            100,
        );

        println!("  Speedup: {:.2}x\n", orig_iter as f64 / opt_iter as f64);

        // Benchmark: Random removal (smaller sizes only)
        if size <= 10000 {
            let indices: Vec<u32> = (0..size as u32)
                .map(|i| (i * 2654435761) % (size as u32))
                .collect();

            let orig_remove = bench(
                "  Original remove",
                || {
                    let mut storage = original::SparseSet::<Position>::with_capacity(size);
                    for i in 0..size {
                        storage.insert(
                            Entity::new(i as u32, 0),
                            Position { x: i as f32, y: i as f32, z: i as f32 },
                        );
                    }
                    for &idx in &indices {
                        storage.remove(Entity::new(idx, 0));
                    }
                    std::hint::black_box(storage);
                },
                10,
            );

            let opt_remove = bench(
                "  Optimized remove",
                || {
                    let mut storage = optimized::SparseSet::<Position>::with_capacity(size);
                    for i in 0..size {
                        storage.insert(
                            Entity::new(i as u32, 0),
                            Position { x: i as f32, y: i as f32, z: i as f32 },
                        );
                    }
                    for &idx in &indices {
                        storage.remove(Entity::new(idx, 0));
                    }
                    std::hint::black_box(storage);
                },
                10,
            );

            println!("  Speedup: {:.2}x\n", orig_remove as f64 / opt_remove as f64);
        }

        println!();
    }

    println!("=== Benchmark Complete ===");
}
