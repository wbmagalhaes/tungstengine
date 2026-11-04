pub fn fan_split(from: (f32, f32), to: &[(f32, f32)]) -> (f32, f32) {
    let n = to.len().max(1) as f32;
    let (sx, sy) = to.iter().fold((0.0, 0.0), |(x, y), p| (x + p.0, y + p.1));
    let (cx, cy) = (sx / n, sy / n);
    (from.0 + (cx - from.0) * 0.5, from.1 + (cy - from.1) * 0.5)
}
