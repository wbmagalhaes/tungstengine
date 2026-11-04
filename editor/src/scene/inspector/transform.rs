use glam::Vec3;

use super::form::Form;

const SLOT_BASE: u64 = 100;

pub fn draw(f: &mut Form, pos: &mut Vec3, rot: &mut Vec3, scale: &mut Vec3) {
    f.component_header("transform", None, None);
    f.vec3(SLOT_BASE, "position", pos);
    f.vec3(SLOT_BASE + 1, "rotation", rot);
    f.vec3_lockable(SLOT_BASE + 2, "scale", scale);
    f.gap();
}
