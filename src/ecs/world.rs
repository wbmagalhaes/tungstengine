use super::hash::FastMap;
use std::any::{Any, TypeId};
use std::error::Error;

use super::components::{Children, Parent};
use super::entity::Entity;
use super::marker::{Component, Resource, ResourceCore};
use super::time::Time;

trait Column: 'static {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn erase(&mut self, e: Entity);
}

impl<T: 'static> Column for FastMap<Entity, T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn erase(&mut self, e: Entity) {
        self.remove(&e);
    }
}

pub type SysResult = Result<(), Box<dyn Error>>;

type System = fn(&mut World) -> SysResult;
type Condition = Box<dyn Fn(&World) -> bool>;

fn missing_resource<T>() -> Box<dyn Error> {
    format!("resource `{}` not registered", std::any::type_name::<T>()).into()
}

pub fn in_state<S: Resource + PartialEq + Copy>(target: S) -> impl Fn(&World) -> bool {
    move |world| world.resource::<S>().map(|s| *s == target).unwrap_or(false)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum Stage {
    PreUpdate,
    #[default]
    Update,
    Physics,
    AfterPhysics,
    Late,
}

struct Scheduled {
    stage: Stage,
    condition: Option<Condition>,
    system: System,
}

#[derive(Default)]
pub struct World {
    generations: Vec<u32>,
    free: Vec<u32>,
    columns: FastMap<TypeId, Box<dyn Column>>,
    resources: FastMap<TypeId, Box<dyn Any>>,
    systems: Vec<Scheduled>,
    failures: FastMap<usize, String>,
}

impl World {
    pub fn new() -> Self {
        let mut world = Self::default();
        world.insert_resource(Time::default());
        world
    }

    pub fn insert_resource<T: Resource>(&mut self, value: T) {
        self.resources.insert(TypeId::of::<T>(), Box::new(value));
    }

    pub fn resource<T: Resource>(&self) -> Result<&T, Box<dyn Error>> {
        self.resources
            .get(&TypeId::of::<T>())
            .and_then(|r| r.downcast_ref())
            .ok_or_else(missing_resource::<T>)
    }

    pub fn resource_mut<T: Resource>(&mut self) -> Result<&mut T, Box<dyn Error>> {
        self.resources
            .get_mut(&TypeId::of::<T>())
            .and_then(|r| r.downcast_mut())
            .ok_or_else(missing_resource::<T>)
    }

    pub fn resource_core<T: ResourceCore + Default>(&self) -> &T {
        self.resources
            .get(&TypeId::of::<T>())
            .and_then(|r| r.downcast_ref())
            .unwrap_or_else(|| panic!("missing core resource: {}", std::any::type_name::<T>()))
    }

    pub fn resource_core_mut<T: ResourceCore + Default>(&mut self) -> &mut T {
        self.resources
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(T::default()))
            .downcast_mut()
            .expect("core resource type mismatch")
    }

    fn schedule(&mut self, entry: Scheduled) {
        let at = self.systems.partition_point(|s| s.stage <= entry.stage);
        self.systems.insert(at, entry);
    }

    pub fn add_system(&mut self, system: System) {
        self.add_system_to(Stage::Update, system);
    }

    pub fn add_system_to(&mut self, stage: Stage, system: System) {
        self.schedule(Scheduled {
            stage,
            condition: None,
            system,
        });
    }

    pub fn add_system_if(&mut self, condition: impl Fn(&World) -> bool + 'static, system: System) {
        self.add_system_to_if(Stage::Update, condition, system);
    }

    pub fn add_system_to_if(
        &mut self,
        stage: Stage,
        condition: impl Fn(&World) -> bool + 'static,
        system: System,
    ) {
        self.schedule(Scheduled {
            stage,
            condition: Some(Box::new(condition)),
            system,
        });
    }

    pub fn init_state<S: Resource + Default>(&mut self) {
        self.insert_resource(S::default());
    }

    pub fn dt(&self) -> f32 {
        match self.resource::<Time>() {
            Ok(t) => t.dt,
            _ => 0.0,
        }
    }

    pub fn elapsed(&self) -> f32 {
        match self.resource::<Time>() {
            Ok(t) => t.elapsed,
            _ => 0.0,
        }
    }

    pub fn run(&mut self) {
        let systems = std::mem::take(&mut self.systems);
        for entry in &systems {
            if entry.condition.as_ref().is_none_or(|c| c(self)) {
                let result = (entry.system)(self).map_err(|e| e.to_string());
                if let Some(message) = self.note_failure(entry.system as usize, result) {
                    log::error!("system error: {message}");
                }
            }
        }
        let added = std::mem::replace(&mut self.systems, systems);
        for entry in added {
            self.schedule(entry);
        }
    }

    fn note_failure(&mut self, system: usize, result: Result<(), String>) -> Option<String> {
        let Err(message) = result else {
            self.failures.remove(&system);
            return None;
        };
        let repeated = self.failures.get(&system) == Some(&message);
        self.failures.insert(system, message.clone());
        (!repeated).then_some(message)
    }

    pub fn spawn(&mut self) -> Entity {
        if let Some(id) = self.free.pop() {
            Entity {
                id,
                generation: self.generations[id as usize],
            }
        } else {
            let id = self.generations.len() as u32;
            self.generations.push(0);
            Entity { id, generation: 0 }
        }
    }

    pub fn despawn(&mut self, e: Entity) -> bool {
        if !self.is_alive(e) {
            return false;
        }
        if let Some(p) = self.get::<Parent>(e).map(|p| p.0)
            && let Some(c) = self.get_mut::<Children>(p)
        {
            c.0.retain(|&x| x != e);
        }
        let mut stack = vec![e];
        while let Some(n) = stack.pop() {
            if let Some(c) = self.get::<Children>(n) {
                stack.extend(c.0.iter().copied());
            }
            self.despawn_one(n);
        }
        true
    }

    fn despawn_one(&mut self, e: Entity) {
        if !self.is_alive(e) {
            return;
        }
        for col in self.columns.values_mut() {
            col.erase(e);
        }
        let g = &mut self.generations[e.id as usize];
        *g = g.wrapping_add(1);
        self.free.push(e.id);
    }

    pub fn is_alive(&self, e: Entity) -> bool {
        (e.id as usize) < self.generations.len() && self.generations[e.id as usize] == e.generation
    }

    pub fn insert<T: Component>(&mut self, e: Entity, value: T) {
        self.columns
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(FastMap::<Entity, T>::default()))
            .as_any_mut()
            .downcast_mut::<FastMap<Entity, T>>()
            .unwrap()
            .insert(e, value);
    }

    pub fn remove<T: Component>(&mut self, e: Entity) -> Option<T> {
        self.columns
            .get_mut(&TypeId::of::<T>())?
            .as_any_mut()
            .downcast_mut::<FastMap<Entity, T>>()?
            .remove(&e)
    }

    pub(super) fn storage<T: Component>(&self) -> Option<&FastMap<Entity, T>> {
        self.columns
            .get(&TypeId::of::<T>())?
            .as_any()
            .downcast_ref()
    }

    pub(super) fn storage_mut<T: Component>(&mut self) -> Option<&mut FastMap<Entity, T>> {
        self.columns
            .get_mut(&TypeId::of::<T>())?
            .as_any_mut()
            .downcast_mut()
    }

    pub fn get<T: Component>(&self, e: Entity) -> Option<&T> {
        self.storage::<T>()?.get(&e)
    }

    pub fn get_mut<T: Component>(&mut self, e: Entity) -> Option<&mut T> {
        self.storage_mut::<T>()?.get_mut(&e)
    }

    pub fn has<T: Component>(&self, e: Entity) -> bool {
        self.storage::<T>().is_some_and(|s| s.contains_key(&e))
    }

    pub fn columns_disjoint_any_mut<const N: usize>(
        &mut self,
        ids: [TypeId; N],
    ) -> [Option<&mut dyn Any>; N] {
        let repeated = (1..N).any(|i| ids[..i].contains(&ids[i]));
        if repeated {
            log::error!("query_mut repeats a component; the query returns nothing");
            return [(); N].map(|_| None);
        }
        self.columns
            .get_disjoint_mut(ids.each_ref())
            .map(|slot| slot.map(|col| col.as_any_mut()))
    }
}

#[cfg(test)]
mod failure_tests {
    use super::*;

    #[derive(Default)]
    struct Trace(Vec<u8>);
    impl Resource for Trace {}

    fn mark(world: &mut World, n: u8) -> SysResult {
        world.resource_mut::<Trace>()?.0.push(n);
        Ok(())
    }

    #[test]
    fn stages_run_in_order_regardless_of_registration() {
        let mut w = World::new();
        w.insert_resource(Trace::default());
        w.add_system_to(Stage::Late, |w| mark(w, 4));
        w.add_system_to(Stage::Physics, |w| mark(w, 2));
        w.add_system(|w| mark(w, 0));
        w.add_system_to(Stage::AfterPhysics, |w| mark(w, 3));
        w.add_system(|w| mark(w, 1));
        w.run();
        assert_eq!(w.resource::<Trace>().unwrap().0, [0, 1, 2, 3, 4]);
    }

    #[test]
    fn repeated_system_errors_are_reported_once() {
        let mut w = World::new();
        let fail = || Err("boom".to_string());
        assert!(w.note_failure(1, fail()).is_some());
        assert!(w.note_failure(1, fail()).is_none());
        assert!(w.note_failure(2, fail()).is_some());
        assert!(w.note_failure(1, Ok(())).is_none());
        assert!(w.note_failure(1, fail()).is_some());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Position(f32, f32);

    impl Component for Position {}

    #[test]
    fn spawn_insert_query() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, Position(1.0, 2.0));
        let p = world.get::<Position>(e).unwrap();
        assert_eq!((p.0, p.1), (1.0, 2.0));
    }

    #[test]
    fn despawn_clears_components() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, Position(0.0, 0.0));
        assert!(world.despawn(e));
        assert!(!world.is_alive(e));
        assert!(world.get::<Position>(e).is_none());
    }

    #[test]
    fn generation_invalidates_recycled_id() {
        let mut world = World::new();
        let a = world.spawn();
        world.despawn(a);
        let b = world.spawn();
        assert_eq!(a.id, b.id);
        assert_ne!(a.generation, b.generation);
        assert!(!world.is_alive(a));
        assert!(world.is_alive(b));
    }
}

#[cfg(test)]
mod core_tests {
    use crate::ecs::{Resource, World};

    #[derive(Resource, Default)]
    #[resource(core)]
    struct Counter(u32);

    #[test]
    fn core_mut_creates_default_when_missing() {
        let mut world = World::new();
        world.resource_core_mut::<Counter>().0 += 2;
        assert_eq!(world.resource_core::<Counter>().0, 2);
    }

    #[test]
    #[should_panic(expected = "missing core resource")]
    fn core_ref_panics_with_name_when_missing() {
        let world = World::new();
        let _ = world.resource_core::<Counter>();
    }
}
