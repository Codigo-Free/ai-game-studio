//! Entity Component System of the AI Game Studio runtime.
//!
//! A pragmatic ECS: entities are generational handles, each component type
//! lives in its own column indexed by entity slot, and queries iterate the
//! intersection of columns. Systems are plain functions grouped in a
//! [`Schedule`]. Optimized storage (archetypes/sparse sets) can replace the
//! internals later without changing this API (see `docs/runtime.md`).

mod storage;

use std::any::TypeId;
use std::cell::{Ref, RefMut};
use std::collections::HashMap;

use storage::{AnyColumn, Column};

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

/// Container of all entities and components of a running scene.
#[derive(Default)]
pub struct World {
    generations: Vec<u32>,
    alive: Vec<bool>,
    free: Vec<u32>,
    columns: HashMap<TypeId, Box<dyn AnyColumn>>,
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

    /// Removes an entity and all its components. Returns `false` if the
    /// handle was already stale.
    pub fn despawn(&mut self, entity: Entity) -> bool {
        if !self.is_alive(entity) {
            return false;
        }
        let slot = entity.index as usize;
        self.alive[slot] = false;
        self.generations[slot] = self.generations[slot].wrapping_add(1);
        self.free.push(entity.index);
        for column in self.columns.values() {
            column.clear_slot(slot);
        }
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

    /// Iterates every live entity.
    pub fn entities(&self) -> impl Iterator<Item = Entity> + '_ {
        self.alive
            .iter()
            .enumerate()
            .filter(|(_, alive)| **alive)
            .map(|(index, _)| Entity {
                index: index as u32,
                generation: self.generations[index],
            })
    }

    // -- components --------------------------------------------------------

    /// Attaches (or replaces) a component on a live entity.
    pub fn insert<T: 'static>(&mut self, entity: Entity, value: T) {
        assert!(self.is_alive(entity), "insert on dead entity");
        let column = self
            .columns
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(Column::<T>::new()));
        column
            .as_any()
            .downcast_ref::<Column<T>>()
            .expect("column type mismatch")
            .set(entity.index as usize, value);
    }

    /// Detaches a component; returns it if it was present.
    pub fn remove<T: 'static>(&mut self, entity: Entity) -> Option<T> {
        if !self.is_alive(entity) {
            return None;
        }
        self.column::<T>()?
            .cells
            .borrow_mut()
            .get_mut(entity.index as usize)?
            .take()
    }

    /// Shared borrow of a component.
    ///
    /// Panics if the same component type is currently mutably borrowed by an
    /// ongoing `for_each*` call.
    pub fn get<T: 'static>(&self, entity: Entity) -> Option<Ref<'_, T>> {
        if !self.is_alive(entity) {
            return None;
        }
        let cells = self.column::<T>()?.cells.borrow();
        Ref::filter_map(cells, |cells| {
            cells.get(entity.index as usize).and_then(Option::as_ref)
        })
        .ok()
    }

    /// Mutable borrow of a component. Same panic rules as [`World::get`].
    pub fn get_mut<T: 'static>(&self, entity: Entity) -> Option<RefMut<'_, T>> {
        if !self.is_alive(entity) {
            return None;
        }
        let cells = self.column::<T>()?.cells.borrow_mut();
        RefMut::filter_map(cells, |cells| {
            cells
                .get_mut(entity.index as usize)
                .and_then(Option::as_mut)
        })
        .ok()
    }

    pub fn has<T: 'static>(&self, entity: Entity) -> bool {
        self.get::<T>(entity).is_some()
    }

    // -- queries ------------------------------------------------------------

    /// Runs `f` over every live entity that has a `T`.
    pub fn for_each<T: 'static>(&self, mut f: impl FnMut(Entity, &mut T)) {
        let Some(column) = self.column::<T>() else {
            return;
        };
        let mut cells = column.cells.borrow_mut();
        for entity in self.entities() {
            if let Some(Some(value)) = cells.get_mut(entity.index as usize) {
                f(entity, value);
            }
        }
    }

    /// Runs `f` over every live entity that has both an `A` and a `B`.
    ///
    /// Panics if `A` and `B` are the same type.
    pub fn for_each2<A: 'static, B: 'static>(&self, mut f: impl FnMut(Entity, &mut A, &mut B)) {
        assert_ne!(
            TypeId::of::<A>(),
            TypeId::of::<B>(),
            "for_each2 requires two distinct component types"
        );
        let (Some(col_a), Some(col_b)) = (self.column::<A>(), self.column::<B>()) else {
            return;
        };
        let mut cells_a = col_a.cells.borrow_mut();
        let mut cells_b = col_b.cells.borrow_mut();
        for entity in self.entities() {
            let slot = entity.index as usize;
            if let (Some(Some(a)), Some(Some(b))) = (
                cells_a.get_mut(slot).map(Option::as_mut),
                cells_b.get_mut(slot).map(Option::as_mut),
            ) {
                f(entity, a, b);
            }
        }
    }

    /// Runs `f` over every live entity that has an `A`, a `B` and a `C`.
    ///
    /// Panics if any two of the component types coincide.
    pub fn for_each3<A: 'static, B: 'static, C: 'static>(
        &self,
        mut f: impl FnMut(Entity, &mut A, &mut B, &mut C),
    ) {
        let ids = [TypeId::of::<A>(), TypeId::of::<B>(), TypeId::of::<C>()];
        assert!(
            ids[0] != ids[1] && ids[0] != ids[2] && ids[1] != ids[2],
            "for_each3 requires three distinct component types"
        );
        let (Some(col_a), Some(col_b), Some(col_c)) =
            (self.column::<A>(), self.column::<B>(), self.column::<C>())
        else {
            return;
        };
        let mut cells_a = col_a.cells.borrow_mut();
        let mut cells_b = col_b.cells.borrow_mut();
        let mut cells_c = col_c.cells.borrow_mut();
        for entity in self.entities() {
            let slot = entity.index as usize;
            if let (Some(Some(a)), Some(Some(b)), Some(Some(c))) = (
                cells_a.get_mut(slot).map(Option::as_mut),
                cells_b.get_mut(slot).map(Option::as_mut),
                cells_c.get_mut(slot).map(Option::as_mut),
            ) {
                f(entity, a, b, c);
            }
        }
    }

    fn column<T: 'static>(&self) -> Option<&Column<T>> {
        self.columns
            .get(&TypeId::of::<T>())?
            .as_any()
            .downcast_ref::<Column<T>>()
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// A system is a function over the world plus an app-defined context (time,
/// input, …).
pub type System<Ctx> = Box<dyn FnMut(&mut World, &Ctx)>;

/// Ordered list of systems executed once per simulation tick.
pub struct Schedule<Ctx> {
    systems: Vec<System<Ctx>>,
}

impl<Ctx> Default for Schedule<Ctx> {
    fn default() -> Self {
        Self {
            systems: Vec::new(),
        }
    }
}

impl<Ctx> Schedule<Ctx> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_system(&mut self, system: impl FnMut(&mut World, &Ctx) + 'static) -> &mut Self {
        self.systems.push(Box::new(system));
        self
    }

    /// Runs every system in registration order.
    pub fn run(&mut self, world: &mut World, ctx: &Ctx) {
        for system in &mut self.systems {
            system(world, ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Position(f32, f32);
    #[derive(Debug, PartialEq)]
    struct Velocity(f32, f32);
    #[derive(Debug, PartialEq)]
    struct Tag;

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
    fn despawn_invalidates_handle_and_drops_components() {
        let mut world = World::new();
        let a = world.spawn();
        world.insert(a, Position(1.0, 2.0));
        assert!(world.despawn(a));
        assert!(!world.is_alive(a));
        assert!(!world.despawn(a), "double despawn must be a no-op");
        assert!(world.get::<Position>(a).is_none());

        // The recycled slot must not inherit the old component.
        let b = world.spawn();
        assert_eq!(a.index(), b.index(), "slot must be reused");
        assert_ne!(a.generation(), b.generation());
        assert!(world.get::<Position>(b).is_none());
    }

    #[test]
    fn insert_get_remove_roundtrip() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, Position(3.0, 4.0));
        assert_eq!(*world.get::<Position>(e).unwrap(), Position(3.0, 4.0));
        world.get_mut::<Position>(e).unwrap().0 = 9.0;
        assert_eq!(world.get::<Position>(e).unwrap().0, 9.0);
        assert_eq!(world.remove::<Position>(e), Some(Position(9.0, 4.0)));
        assert!(!world.has::<Position>(e));
    }

    #[test]
    fn queries_visit_only_matching_entities() {
        let mut world = World::new();
        let moving = world.spawn();
        world.insert(moving, Position(0.0, 0.0));
        world.insert(moving, Velocity(1.0, 2.0));
        let still = world.spawn();
        world.insert(still, Position(10.0, 10.0));
        let dead = world.spawn();
        world.insert(dead, Position(0.0, 0.0));
        world.insert(dead, Velocity(5.0, 5.0));
        world.despawn(dead);

        let mut visited = 0;
        world.for_each2::<Position, Velocity>(|_, pos, vel| {
            pos.0 += vel.0;
            pos.1 += vel.1;
            visited += 1;
        });
        assert_eq!(visited, 1);
        assert_eq!(*world.get::<Position>(moving).unwrap(), Position(1.0, 2.0));
        assert_eq!(*world.get::<Position>(still).unwrap(), Position(10.0, 10.0));
    }

    #[test]
    fn for_each3_intersects_three_columns() {
        let mut world = World::new();
        let full = world.spawn();
        world.insert(full, Position(0.0, 0.0));
        world.insert(full, Velocity(1.0, 1.0));
        world.insert(full, Tag);
        let partial = world.spawn();
        world.insert(partial, Position(0.0, 0.0));
        world.insert(partial, Tag);

        let mut visited = Vec::new();
        world.for_each3::<Position, Velocity, Tag>(|entity, _, _, _| visited.push(entity));
        assert_eq!(visited, vec![full]);
    }

    #[test]
    fn schedule_runs_systems_in_order() {
        struct Ctx {
            dt: f32,
        }
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, Position(0.0, 0.0));
        world.insert(e, Velocity(10.0, 0.0));

        let mut schedule: Schedule<Ctx> = Schedule::new();
        schedule.add_system(|world, ctx: &Ctx| {
            world.for_each2::<Position, Velocity>(|_, pos, vel| {
                pos.0 += vel.0 * ctx.dt;
            });
        });
        schedule.add_system(|world, _| {
            world.for_each::<Position>(|_, pos| pos.1 = pos.0 * 2.0);
        });

        schedule.run(&mut world, &Ctx { dt: 0.5 });
        assert_eq!(*world.get::<Position>(e).unwrap(), Position(5.0, 10.0));
    }
}
