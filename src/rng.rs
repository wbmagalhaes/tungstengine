use crate::ecs::prelude::*;

#[derive(Resource)]
#[resource(core)]
pub struct Rng(pub u32);

impl Default for Rng {
    fn default() -> Self {
        let seed = web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0x9e3779b9);
        Rng(seed | 1)
    }
}

impl Rng {
    pub fn seeded(seed: u32) -> Self {
        Rng(seed.wrapping_mul(0x9e3779b9) | 1)
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.unit() < p
    }

    pub fn unit(&mut self) -> f32 {
        (self.next() >> 8) as f32 / (1u32 << 24) as f32
    }

    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { self.next() as usize % n }
    }
}

pub mod prelude {
    pub use super::Rng;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_sequences_repeat() {
        let (mut a, mut b) = (Rng::seeded(7), Rng::seeded(7));
        assert_eq!(a.next(), b.next());
        assert!(a.below(5) < 5);
        assert!((0.0..1.0).contains(&a.unit()));
    }
}
