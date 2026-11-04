use super::components::{Children, Parent};
use super::prelude::*;

pub fn set_parent(world: &mut World, child: Entity, parent: Entity) {
    world.insert(child, Parent(parent));
    if let Some(c) = world.get_mut::<Children>(parent) {
        if !c.0.contains(&child) {
            c.0.push(child);
        }
    } else {
        world.insert(parent, Children(vec![child]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn despawns_whole_subtree() {
        let mut world = World::new();
        let root = world.spawn();
        let a = world.spawn();
        let b = world.spawn();
        let grandchild = world.spawn();

        set_parent(&mut world, a, root);
        set_parent(&mut world, b, root);
        set_parent(&mut world, grandchild, a);

        let outsider = world.spawn();

        world.despawn(root);

        assert!(!world.is_alive(root));
        assert!(!world.is_alive(a));
        assert!(!world.is_alive(b));
        assert!(!world.is_alive(grandchild));
        assert!(world.is_alive(outsider));
    }
}
