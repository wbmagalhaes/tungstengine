pub const DUAL_FULL: usize = 15;

pub fn neighbor_index(x: i32, y: i32, on: impl Fn(i32, i32) -> bool) -> usize {
    [(0, -1, 1), (1, 0, 2), (0, 1, 4), (-1, 0, 8)]
        .into_iter()
        .filter(|&(dx, dy, _)| on(x + dx, y + dy))
        .map(|(_, _, bit)| bit)
        .sum()
}

pub fn neighbor8_index(x: i32, y: i32, on: impl Fn(i32, i32) -> bool) -> usize {
    let edges = neighbor_index(x, y, &on);
    let (up, right, down, left) = (edges & 1, edges & 2, edges & 4, edges & 8);
    let corner = |sides: bool, dx: i32, dy: i32, bit: usize| {
        if sides && on(x + dx, y + dy) { bit } else { 0 }
    };
    edges
        + corner(up != 0 && right != 0, 1, -1, 16)
        + corner(down != 0 && right != 0, 1, 1, 32)
        + corner(down != 0 && left != 0, -1, 1, 64)
        + corner(up != 0 && left != 0, -1, -1, 128)
}

pub fn nine_index(x: i32, y: i32, on: impl Fn(i32, i32) -> bool) -> (usize, usize) {
    let slice = |before: bool, after: bool| match (before, after) {
        (false, _) => 0,
        (true, false) => 2,
        (true, true) => 1,
    };
    (
        slice(on(x, y - 1), on(x, y + 1)),
        slice(on(x - 1, y), on(x + 1, y)),
    )
}

pub fn corner_index(x: i32, y: i32, on: impl Fn(i32, i32) -> bool) -> usize {
    [(-1, -1, 1), (0, -1, 2), (-1, 0, 4), (0, 0, 8)]
        .into_iter()
        .filter(|&(dx, dy, _)| on(x + dx, y + dy))
        .map(|(_, _, bit)| bit)
        .sum()
}

pub fn dual_cells(
    (x0, x1): (i32, i32),
    (y0, y1): (i32, i32),
    on: impl Fn(i32, i32) -> bool,
) -> Vec<(f32, f32, usize)> {
    let mut out = Vec::new();
    for y in y0..=y1 + 1 {
        for x in x0..=x1 + 1 {
            let index = corner_index(x, y, &on);
            if index != 0 {
                out.push((x as f32 - 0.5, y as f32 - 0.5, index));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(x: i32, y: i32) -> bool {
        (0..2).contains(&x) && (0..2).contains(&y)
    }

    #[test]
    fn neighbor_bits_follow_up_right_down_left() {
        assert_eq!(neighbor_index(0, 0, block), 2 + 4);
        assert_eq!(neighbor_index(1, 1, block), 1 + 8);
    }

    #[test]
    fn neighbor8_counts_a_corner_only_between_two_filled_sides() {
        assert_eq!(neighbor8_index(0, 0, block), 2 + 4 + 32);
        let diagonal = |x: i32, y: i32| (x, y) == (1, 1);
        assert_eq!(neighbor8_index(0, 0, diagonal), 0);
    }

    #[test]
    fn nine_slice_picks_edges_and_center() {
        let wide = |x: i32, y: i32| (0..3).contains(&x) && (0..3).contains(&y);
        assert_eq!(nine_index(0, 0, wide), (0, 0));
        assert_eq!(nine_index(1, 1, wide), (1, 1));
        assert_eq!(nine_index(2, 2, wide), (2, 2));
    }

    #[test]
    fn corner_bits_read_the_four_cells_around_a_vertex() {
        assert_eq!(corner_index(0, 0, block), 8);
        assert_eq!(corner_index(1, 1, block), DUAL_FULL);
        assert_eq!(corner_index(2, 2, block), 1);
    }

    #[test]
    fn dual_cells_are_offset_half_a_cell_and_skip_empty() {
        let cells = dual_cells((0, 1), (0, 1), block);
        assert_eq!(cells.len(), 9);
        assert!(cells.contains(&(0.5, 0.5, DUAL_FULL)));
        assert!(cells.contains(&(-0.5, -0.5, 8)));
    }
}
