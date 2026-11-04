use super::hash::FastMap;
use std::any::TypeId;

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

macro_rules! impl_query_mut {
    ($head:ident $(, $tail:ident)*) => {
        impl<'w, $head: Component $(, $tail: Component)*> QueryMut<'w>
            for (&'w mut $head, $(&'w $tail,)*)
        {
            type Item = (Entity, &'w mut $head, $(&'w $tail,)*);

            #[allow(non_snake_case)]
            fn run(world: &'w mut World) -> impl Iterator<Item = Self::Item> {
                let [$head $(, $tail)*] = world
                    .columns_disjoint_any_mut([TypeId::of::<$head>() $(, TypeId::of::<$tail>())*]);
                let cols = (move || {
                    let $head = $head?.downcast_mut::<FastMap<Entity, $head>>()?;
                    $(
                        let $tail: &FastMap<Entity, $tail> =
                            $tail?.downcast_mut::<FastMap<Entity, $tail>>()?;
                    )*
                    Some(($head, $($tail,)*))
                })();
                cols.into_iter().flat_map(|($head, $($tail,)*)| {
                    $head.iter_mut().filter_map(move |(&e, hv)| {
                        Some((e, hv, $( $tail.get(&e)?, )*))
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
