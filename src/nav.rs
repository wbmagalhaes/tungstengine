use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use glam::{Vec2, Vec3};

use crate::ecs::prelude::*;
use crate::physics::Footprint;

#[derive(Component, Clone)]
pub struct WalkGrid {
    pub cell: f32,
    pub cols: usize,
    pub rows: usize,
    pub origin: [f32; 2],
    pub blocked: Vec<bool>,
    pub dynamic: Vec<bool>,
}

impl WalkGrid {
    pub fn new(cell: f32, cols: usize, rows: usize, origin: [f32; 2]) -> Self {
        WalkGrid {
            cell,
            cols,
            rows,
            origin,
            blocked: vec![false; cols * rows],
            dynamic: vec![false; cols * rows],
        }
    }

    pub fn clear_dynamic(&mut self) {
        self.dynamic.iter_mut().for_each(|d| *d = false);
    }

    pub fn stamp_aabb(&mut self, center: Vec2, half: Vec2) {
        let eps = self.cell * 0.01;
        let lo_x = ((center.x - half.x + eps - self.origin[0]) / self.cell).floor() as i32;
        let hi_x = ((center.x + half.x - eps - self.origin[0]) / self.cell).floor() as i32;
        let lo_z = ((center.y - half.y + eps - self.origin[1]) / self.cell).floor() as i32;
        let hi_z = ((center.y + half.y - eps - self.origin[1]) / self.cell).floor() as i32;
        for cz in lo_z.max(0)..=hi_z.min(self.rows as i32 - 1) {
            for cx in lo_x.max(0)..=hi_x.min(self.cols as i32 - 1) {
                let i = self.idx(cx as usize, cz as usize);
                if let Some(d) = self.dynamic.get_mut(i) {
                    *d = true;
                }
            }
        }
    }

    pub fn stamp_footprint(&mut self, fp: &Footprint, inflate: f32) {
        if fp.points.is_empty() {
            return;
        }
        let (lo, hi) = fp.bounds();
        let h = self.cell * 0.5 - self.cell * 0.01;
        let to_cell = |v: f32, o: f32| ((v - o) / self.cell).floor() as i32;
        let lo_x = to_cell(lo.x - inflate, self.origin[0]).max(0);
        let hi_x = to_cell(hi.x + inflate, self.origin[0]).min(self.cols as i32 - 1);
        let lo_z = to_cell(lo.y - inflate, self.origin[1]).max(0);
        let hi_z = to_cell(hi.y + inflate, self.origin[1]).min(self.rows as i32 - 1);
        for cz in lo_z..=hi_z {
            for cx in lo_x..=hi_x {
                let c = self.center(cx as usize, cz as usize);
                let square = [
                    Vec2::new(c.x - h, c.z - h),
                    Vec2::new(c.x + h, c.z - h),
                    Vec2::new(c.x + h, c.z + h),
                    Vec2::new(c.x - h, c.z + h),
                ];
                if fp.distance_to(&square) <= inflate {
                    let i = self.idx(cx as usize, cz as usize);
                    if let Some(d) = self.dynamic.get_mut(i) {
                        *d = true;
                    }
                }
            }
        }
    }

    pub fn authored_blocked(&self, cx: usize, cz: usize) -> bool {
        self.blocked.get(self.idx(cx, cz)).copied().unwrap_or(false)
    }

    pub fn nearest_free(&self, p: Vec3) -> Option<Vec3> {
        if !self.is_blocked(p) {
            return Some(p);
        }
        let (cx, cz) = self.cell_of(p.x, p.z)?;
        let (cx, cz) = (cx as i32, cz as i32);
        let max_r = self.cols.max(self.rows) as i32;
        for r in 1..=max_r {
            for dz in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dz.abs() != r {
                        continue;
                    }
                    let (nx, nz) = (cx + dx, cz + dz);
                    if nx < 0 || nz < 0 || nx >= self.cols as i32 || nz >= self.rows as i32 {
                        continue;
                    }
                    if !self.is_blocked_cell(nx as usize, nz as usize) {
                        return Some(self.center(nx as usize, nz as usize));
                    }
                }
            }
        }
        None
    }

    pub fn cell_of(&self, x: f32, z: f32) -> Option<(usize, usize)> {
        let cx = ((x - self.origin[0]) / self.cell).floor();
        let cz = ((z - self.origin[1]) / self.cell).floor();

        if cx < 0.0 || cz < 0.0 {
            return None;
        }

        let (cx, cz) = (cx as usize, cz as usize);
        (cx < self.cols && cz < self.rows).then_some((cx, cz))
    }

    pub fn center(&self, cx: usize, cz: usize) -> Vec3 {
        Vec3::new(
            self.origin[0] + (cx as f32 + 0.5) * self.cell,
            0.0,
            self.origin[1] + (cz as f32 + 0.5) * self.cell,
        )
    }

    fn idx(&self, cx: usize, cz: usize) -> usize {
        cz * self.cols + cx
    }

    pub fn is_blocked_cell(&self, cx: usize, cz: usize) -> bool {
        let i = self.idx(cx, cz);
        match (self.blocked.get(i), self.dynamic.get(i)) {
            (Some(b), Some(d)) => *b || *d,
            _ => true,
        }
    }

    pub fn is_blocked(&self, pos: Vec3) -> bool {
        match self.cell_of(pos.x, pos.z) {
            Some((cx, cz)) => self.is_blocked_cell(cx, cz),
            None => true,
        }
    }

    pub fn set(&mut self, cx: usize, cz: usize, blocked: bool) {
        let i = self.idx(cx, cz);
        if let Some(b) = self.blocked.get_mut(i) {
            *b = blocked;
        }
    }

    fn neighbors(&self, c: (usize, usize)) -> Vec<((usize, usize), i32)> {
        let (x, z) = (c.0 as i32, c.1 as i32);
        let mut out = Vec::new();

        let dirs = [
            (-1, 0, 10),
            (1, 0, 10),
            (0, -1, 10),
            (0, 1, 10),
            (-1, -1, 14),
            (1, -1, 14),
            (-1, 1, 14),
            (1, 1, 14),
        ];

        for (dx, dz, cost) in dirs {
            let (nx, nz) = (x + dx, z + dz);
            if nx < 0 || nz < 0 || nx >= self.cols as i32 || nz >= self.rows as i32 {
                continue;
            }

            let (nx, nz) = (nx as usize, nz as usize);
            if self.is_blocked_cell(nx, nz) {
                continue;
            }

            // no diagonal corner cutting
            if dx != 0
                && dz != 0
                && (self.is_blocked_cell((x + dx) as usize, z as usize)
                    || self.is_blocked_cell(x as usize, (z + dz) as usize))
            {
                continue;
            }

            out.push(((nx, nz), cost));
        }
        out
    }

    fn heuristic(a: (usize, usize), b: (usize, usize)) -> i32 {
        let dx = (a.0 as i32 - b.0 as i32).abs();
        let dz = (a.1 as i32 - b.1 as i32).abs();
        10 * (dx + dz) - 6 * dx.min(dz)
    }

    fn astar(&self, start: (usize, usize), goal: (usize, usize)) -> Option<Vec<(usize, usize)>> {
        let mut open: BinaryHeap<(Reverse<i32>, (usize, usize))> = BinaryHeap::new();
        let mut g: HashMap<(usize, usize), i32> = HashMap::new();
        let mut came: HashMap<(usize, usize), (usize, usize)> = HashMap::new();

        g.insert(start, 0);
        open.push((Reverse(Self::heuristic(start, goal)), start));

        while let Some((_, cur)) = open.pop() {
            if cur == goal {
                let mut path = vec![cur];
                let mut c = cur;
                while let Some(&p) = came.get(&c) {
                    path.push(p);
                    c = p;
                }
                path.reverse();
                path.remove(0);
                return Some(path);
            }

            let cg = *g.get(&cur).unwrap_or(&i32::MAX);
            for (n, cost) in self.neighbors(cur) {
                let ng = cg + cost;
                if ng < *g.get(&n).unwrap_or(&i32::MAX) {
                    came.insert(n, cur);
                    g.insert(n, ng);
                    open.push((Reverse(ng + Self::heuristic(n, goal)), n));
                }
            }
        }
        None
    }

    fn line_of_sight(&self, a: (usize, usize), b: (usize, usize)) -> bool {
        let pa = self.center(a.0, a.1);
        let pb = self.center(b.0, b.1);
        let d = pb - pa;
        let len = (d.x * d.x + d.z * d.z).sqrt();
        let steps = (len / (self.cell * 0.5)).ceil().max(1.0) as i32;
        for k in 0..=steps {
            let p = pa + d * (k as f32 / steps as f32);
            if self.is_blocked(p) {
                return false;
            }
        }
        true
    }

    fn string_pull(&self, cells: &[(usize, usize)]) -> Vec<(usize, usize)> {
        if cells.len() <= 2 {
            return cells.to_vec();
        }
        let mut out = vec![cells[0]];
        let mut anchor = 0;
        for i in 2..cells.len() {
            if !self.line_of_sight(cells[anchor], cells[i]) {
                out.push(cells[i - 1]);
                anchor = i - 1;
            }
        }
        out.push(cells[cells.len() - 1]);
        out
    }

    pub fn path(&self, start: Vec3, goal: Vec3) -> Option<Vec<Vec3>> {
        let s = self.cell_of(start.x, start.z)?;
        let g = self.cell_of(goal.x, goal.z)?;

        if self.is_blocked_cell(g.0, g.1) {
            return None;
        }

        if s == g {
            return Some(vec![Vec3::new(goal.x, 0.0, goal.z)]);
        }

        let cells = self.astar(s, g)?;
        let mut full = Vec::with_capacity(cells.len() + 1);
        full.push(s);
        full.extend_from_slice(&cells);
        let smoothed = self.string_pull(&full);

        let mut pts: Vec<Vec3> = smoothed[1..]
            .iter()
            .map(|&(cx, cz)| self.center(cx, cz))
            .collect();
        if let Some(last) = pts.last_mut() {
            *last = Vec3::new(goal.x, 0.0, goal.z);
        }

        Some(pts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn astar_routes_around_wall() {
        let mut g = WalkGrid::new(1.0, 10, 10, [0.0, 0.0]);
        for cz in 0..8 {
            g.set(5, cz, true);
        }
        let path = g.path(g.center(2, 4), g.center(8, 4)).expect("path exists");
        assert!(!path.is_empty());
        for p in &path {
            assert!(!g.is_blocked(*p), "path crosses blocked cell at {p:?}");
        }
        let last = *path.last().unwrap();
        let goal = g.center(8, 4);
        assert!((last.x - goal.x).abs() < 1.0 && (last.z - goal.z).abs() < 1.0);
    }

    #[test]
    fn blocked_goal_has_no_path() {
        let mut g = WalkGrid::new(1.0, 5, 5, [0.0, 0.0]);
        g.set(2, 2, true);
        assert!(g.path(g.center(0, 0), g.center(2, 2)).is_none());
    }
}

pub mod prelude {
    pub use super::WalkGrid;
}
