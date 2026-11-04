use ecs_macros::Resource;

#[derive(Resource, Default, Clone, Copy, Debug)]
#[resource(core)]
pub struct Time {
    pub dt: f32,
    pub elapsed: f32,
}

impl Time {
    pub fn advance(&mut self, dt: f32) {
        self.dt = dt;
        self.elapsed += dt;
    }
}
