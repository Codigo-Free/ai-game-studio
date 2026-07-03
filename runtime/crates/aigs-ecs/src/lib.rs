//! Entity Component System of the AI Game Studio runtime.
//!
//! Milestone M0 seeds the entity allocator with generational indices so that
//! stale [`Entity`] handles can never address a recycled slot. Component
//! storage, queries and systems land in milestone M1 (see `docs/plan.md`).

/// Handle to an entity. Cheap to copy; safe against slot reuse thanks to the
/// generation counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity {
    index: u32,
    generation: u32,
}

impl Entity {
    pub fn index(self) -> u32 {
        self.index
    }

    pub fn generation(self) -> u32 {
        self.generation
    }
}

/// Container of all entities of a running scene.
#[derive(Debug, Default)]
pub struct World {
    generations: Vec<u32>,
    alive: Vec<bool>,
    free: Vec<u32>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new entity, reusing a freed slot when available.
    pub fn spawn(&mut self) -> Entity {
        match self.free.pop() {
            Some(index) => {
                self.alive[index as usize] = true;
                Entity {
                    index,
                    generation: self.generations[index as usize],
                }
            }
            None => {
                let index = self.generations.len() as u32;
                self.generations.push(0);
                self.alive.push(true);
                Entity {
                    index,
                    generation: 0,
                }
            }
        }
    }

    /// Removes an entity. Returns `false` if the handle was already stale.
    pub fn despawn(&mut self, entity: Entity) -> bool {
        if !self.is_alive(entity) {
            return false;
        }
        let slot = entity.index as usize;
        self.alive[slot] = false;
        self.generations[slot] = self.generations[slot].wrapping_add(1);
        self.free.push(entity.index);
        true
    }

    pub fn is_alive(&self, entity: Entity) -> bool {
        let slot = entity.index as usize;
        slot < self.generations.len()
            && self.alive[slot]
            && self.generations[slot] == entity.generation
    }

    /// Number of live entities.
    pub fn len(&self) -> usize {
        self.alive.iter().filter(|alive| **alive).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_creates_live_entities() {
        let mut world = World::new();
        let a = world.spawn();
        let b = world.spawn();
        assert_ne!(a, b);
        assert!(world.is_alive(a));
        assert!(world.is_alive(b));
        assert_eq!(world.len(), 2);
    }

    #[test]
    fn despawn_invalidates_handle() {
        let mut world = World::new();
        let a = world.spawn();
        assert!(world.despawn(a));
        assert!(!world.is_alive(a));
        assert!(!world.despawn(a), "double despawn must be a no-op");
        assert_eq!(world.len(), 0);
    }

    #[test]
    fn recycled_slot_gets_new_generation() {
        let mut world = World::new();
        let a = world.spawn();
        world.despawn(a);
        let b = world.spawn();
        assert_eq!(a.index(), b.index(), "slot must be reused");
        assert_ne!(a.generation(), b.generation());
        assert!(!world.is_alive(a), "stale handle must stay dead");
        assert!(world.is_alive(b));
    }
}
