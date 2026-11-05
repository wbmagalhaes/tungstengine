use super::hash::FastMap;
use std::any::{Any, TypeId};
use std::marker::PhantomData;

use super::entity::Entity;
use super::marker::Component;
use super::world::World;

pub trait Query<'w> {
    type Item;
    fn run(world: &'w World) -> impl Iterator<Item = Self::Item>;
}

pub trait QueryMut<'w> {
    type Item;
    fn run(world: &'w mut World) -> impl Iterator<Item = Self::Item>;
}

macro_rules! impl_query_shared {
    ($head:ident $(, $tail:ident)*) => {
        impl<'w, $head: Component $(, $tail: Component)*> Query<'w>
            for (&'w $head, $(&'w $tail,)*)
        {
            type Item = (Entity, &'w $head, $(&'w $tail,)*);

            #[allow(non_snake_case)]
            fn run(world: &'w World) -> impl Iterator<Item = Self::Item> {
                let cols = (move || {
                    let $head = world.storage::<$head>()?;
                    $( let $tail = world.storage::<$tail>()?; )*
                    Some(($head, $($tail,)*))
                })();
                cols.into_iter().flat_map(|($head, $($tail,)*)| {
                    $head.iter().filter_map(move |(&e, hv)| {
                        Some((e, hv, $( $tail.get(&e)?, )*))
                    })
                })
            }
        }
    };
}

pub trait QueryTail<'w> {
    type Item;
    type Column;
    fn id() -> TypeId;
    fn column(any: &'w mut dyn Any) -> Option<Self::Column>;
    fn fetch(column: &mut Self::Column, e: Entity) -> Option<Self::Item>;
}

impl<'w, T: Component> QueryTail<'w> for &'w T {
    type Item = &'w T;
    type Column = &'w FastMap<Entity, T>;

    fn id() -> TypeId {
        TypeId::of::<T>()
    }

    fn column(any: &'w mut dyn Any) -> Option<Self::Column> {
        any.downcast_mut::<FastMap<Entity, T>>().map(|c| &*c)
    }

    fn fetch(column: &mut Self::Column, e: Entity) -> Option<Self::Item> {
        column.get(&e)
    }
}

pub struct MutColumn<'w, T> {
    map: *mut FastMap<Entity, T>,
    world: PhantomData<&'w mut FastMap<Entity, T>>,
}

impl<'w, T: Component> QueryTail<'w> for &'w mut T {
    type Item = &'w mut T;
    type Column = MutColumn<'w, T>;

    fn id() -> TypeId {
        TypeId::of::<T>()
    }

    fn column(any: &'w mut dyn Any) -> Option<Self::Column> {
        let map = any.downcast_mut::<FastMap<Entity, T>>()?;
        Some(MutColumn {
            map,
            world: PhantomData,
        })
    }

    fn fetch(column: &mut Self::Column, e: Entity) -> Option<Self::Item> {
        // SAFETY: the column is borrowed exclusively for 'w, the map is never resized
        // during the query, and the head iterator yields each entity only once, so no
        // two returned references point at the same value.
        unsafe { (*column.map).get_mut(&e) }
    }
}

macro_rules! impl_query_mut {
    ($head:ident $(, $tail:ident)*) => {
        impl<'w, $head: Component $(, $tail: QueryTail<'w>)*> QueryMut<'w>
            for (&'w mut $head, $($tail,)*)
        {
            type Item = (Entity, &'w mut $head, $($tail::Item,)*);

            #[allow(non_snake_case)]
            fn run(world: &'w mut World) -> impl Iterator<Item = Self::Item> {
                let [$head $(, $tail)*] = world
                    .columns_disjoint_any_mut([TypeId::of::<$head>() $(, $tail::id())*]);
                let cols = (move || {
                    let $head = $head?.downcast_mut::<FastMap<Entity, $head>>()?;
                    $( let $tail = $tail::column($tail?)?; )*
                    Some(($head, $($tail,)*))
                })();
                cols.into_iter().flat_map(|($head, $(mut $tail,)*)| {
                    $head.iter_mut().filter_map(move |(&e, hv)| {
                        Some((e, hv, $( $tail::fetch(&mut $tail, e)?, )*))
                    })
                })
            }
        }
    };
}

macro_rules! impl_query_all {
    ($head:ident) => {
        impl_query_shared!($head);
        impl_query_mut!($head);
    };
    ($head:ident $(, $rest:ident)+) => {
        impl_query_shared!($head $(, $rest)+);
        impl_query_mut!($head $(, $rest)+);
        impl_query_all!($($rest),+);
    };
}

impl_query_all!(A, B, C, D, E, F, G, H);

impl World {
    pub fn query<'w, Q: Query<'w>>(&'w self) -> impl Iterator<Item = Q::Item> {
        Q::run(self)
    }

    pub fn query_mut<'w, Q: QueryMut<'w>>(&'w mut self) -> impl Iterator<Item = Q::Item> {
        Q::run(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct A(i32);
    struct B(i32);
    struct C(i32);

    impl Component for A {}
    impl Component for B {}
    impl Component for C {}

    #[test]
    fn query_mut_takes_several_mutable_columns() {
        let mut world = World::new();
        for i in 0..10 {
            let e = world.spawn();
            world.insert(e, A(i));
            world.insert(e, B(i));
            if i % 2 == 0 {
                world.insert(e, C(1));
            }
        }
        for (_, a, b, c) in world.query_mut::<(&mut A, &mut B, &C)>() {
            a.0 += c.0;
            b.0 -= c.0;
        }
        let sum: i32 = world.query::<(&A, &B)>().map(|(_, a, b)| a.0 - b.0).sum();
        assert_eq!(sum, 10);
    }

    #[test]
    fn query_shared_two() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, A(1));
        world.insert(e, B(2));
        let lone = world.spawn();
        world.insert(lone, A(9));

        let mut hits: Vec<_> = world
            .query::<(&A, &B)>()
            .map(|(_, a, b)| (a.0, b.0))
            .collect();
        hits.sort();
        assert_eq!(hits, vec![(1, 2)]);
    }

    #[test]
    fn query_mut_three_writes_first_reads_rest() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, A(0));
        world.insert(e, B(10));
        world.insert(e, C(100));

        for (_, a, b, c) in world.query_mut::<(&mut A, &B, &C)>() {
            a.0 += b.0 + c.0;
        }

        assert_eq!(world.get::<A>(e).unwrap().0, 110);
    }

    #[test]
    fn query_skips_entities_missing_a_component() {
        let mut world = World::new();
        let both = world.spawn();
        world.insert(both, A(1));
        world.insert(both, B(1));
        let only_a = world.spawn();
        world.insert(only_a, A(1));

        let count = world.query_mut::<(&mut A, &B)>().count();
        assert_eq!(count, 1);
    }
}
