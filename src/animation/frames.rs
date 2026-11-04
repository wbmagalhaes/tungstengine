pub fn frame_at(time: f32, fps: f32, frames: usize) -> usize {
    if frames == 0 {
        return 0;
    }
    (time.max(0.0) * fps) as usize % frames
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loops_through_frames() {
        let seq: Vec<usize> = (0..7).map(|i| frame_at(i as f32 * 0.25, 4.0, 3)).collect();
        assert_eq!(seq, vec![0, 1, 2, 0, 1, 2, 0]);
        assert_eq!(frame_at(1.0, 4.0, 0), 0);
    }
}
