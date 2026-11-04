const DEFAULT_STEP: f32 = 1.0 / 120.0;
const MAX_CATCH_UP: f32 = 0.25;

#[derive(Clone, Copy, Debug)]
pub struct FixedStep {
    pub step: f32,
    accumulator: f32,
}

impl Default for FixedStep {
    fn default() -> Self {
        FixedStep {
            step: DEFAULT_STEP,
            accumulator: 0.0,
        }
    }
}

impl FixedStep {
    pub fn advance(&mut self, dt: f32) -> u32 {
        self.accumulator = (self.accumulator + dt).min(MAX_CATCH_UP);
        let steps = (self.accumulator / self.step).floor();
        self.accumulator -= steps * self.step;
        steps as u32
    }

    pub fn alpha(&self) -> f32 {
        self.accumulator / self.step
    }
}
