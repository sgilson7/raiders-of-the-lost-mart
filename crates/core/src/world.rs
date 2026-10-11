//! The map as the rules see it: solid boxes in a grid, the ground plane at
//! y = 0, named zones, spots, entrances and a waypoint graph. Everything a
//! body or a shot asks of the level goes through here: where it may move,
//! what a ray meets, and how to get from one place to another.

use crate::content::Rgb;
use crate::fx::{self, int, ONE};
use crate::geom::{v3, V3};

/// An axis-aligned box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aabb {
    pub min: V3,
    pub max: V3,
}

impl Aabb {
    pub fn new(a: V3, b: V3) -> Aabb {
        Aabb {
            min: v3(a.x.min(b.x), a.y.min(b.y), a.z.min(b.z)),
            max: v3(a.x.max(b.x), a.y.max(b.y), a.z.max(b.z)),
        }
    }

    /// A box around a point: half sizes, standing on `feet`.
    pub fn standing(feet: V3, half: i32, height: i32) -> Aabb {
        Aabb {
            min: v3(feet.x - half, feet.y, feet.z - half),
            max: v3(feet.x + half, feet.y + height, feet.z + half),
        }
    }

    pub fn overlaps(&self, o: &Aabb) -> bool {
        self.min.x < o.max.x
            && self.max.x > o.min.x
            && self.min.y < o.max.y
            && self.max.y > o.min.y
            && self.min.z < o.max.z
            && self.max.z > o.min.z
    }

    pub fn contains(&self, p: V3) -> bool {
        p.x >= self.min.x
            && p.x <= self.max.x
            && p.y >= self.min.y
            && p.y <= self.max.y
            && p.z >= self.min.z
            && p.z <= self.max.z
    }

    pub fn centre(&self) -> V3 {
        v3(
            (self.min.x + self.max.x) / 2,
            (self.min.y + self.max.y) / 2,
            (self.min.z + self.max.z) / 2,
        )
    }

    pub fn size(&self) -> V3 {
        self.max.sub(self.min)
    }

    /// Where a ray `o + d·t` (d a unit vector) enters this box, if it does
    /// within `max_t`, and the face's axis (0 x, 1 y, 2 z) and sign.
    pub fn ray(&self, o: V3, d: V3, max_t: i32) -> Option<(i32, usize, i32)> {
        let mut t0: i64 = 0;
        let mut t1: i64 = max_t as i64;
        let mut axis = 0usize;
        let mut sign = 0i32;
        let oo = [o.x, o.y, o.z];
        let dd = [d.x, d.y, d.z];
        let lo = [self.min.x, self.min.y, self.min.z];
        let hi = [self.max.x, self.max.y, self.max.z];
        for a in 0..3 {
            if dd[a].abs() < 4 {
                if oo[a] < lo[a] || oo[a] > hi[a] {
                    return None;
                }
                continue;
            }
            let ta = (((lo[a] - oo[a]) as i64) << fx::FRAC_BITS) / dd[a] as i64;
            let tb = (((hi[a] - oo[a]) as i64) << fx::FRAC_BITS) / dd[a] as i64;
            let (near, far, s) = if ta < tb { (ta, tb, -1) } else { (tb, ta, 1) };
            if near > t0 {
                t0 = near;
                axis = a;
                sign = s;
            }
            t1 = t1.min(far);
            if t0 > t1 {
                return None;
            }
        }
        Some((t0 as i32, axis, sign))
    }
}

/// How a solid is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Look {
    /// a plain box in one colour, outlined
    Plain,
    /// a facade: windows drawn by the shader on its faces
    Facade,
    /// glowing (signs, skylights, lamps)
    Glow,
    /// a box drawn without an outline (floors, roofs, big flats)
    Flat,
    /// collides but is not drawn (the map's edge)
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Solid {
    pub b: Aabb,
    pub rgb: Rgb,
    pub look: Look,
    /// shots pass through it (railings' glass, chain fences): bodies do not
    pub open: bool,
}

/// Something drawn that nothing collides with: lines, plants, lamps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decor {
    pub mesh: crate::mesh::Mesh,
    pub at: V3,
    pub size: V3,
    pub yaw: i32,
    pub rgb: Rgb,
    pub glow: i32,
    pub inked: bool,
}

/// A named area, for the HUD's "you are in" and for missions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zone {
    pub key: String,
    pub min: V3,
    pub max: V3,
    pub indoors: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpotKind {
    /// a container of loot to search: a register, a crate, a rack
    Loot,
    /// a place a mission uses (its key says which)
    Mission,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spot {
    pub kind: SpotKind,
    pub key: String,
    pub at: V3,
    /// the zone it is in
    pub zone: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entrance {
    pub key: String,
    /// the doorway, at the floor
    pub door: V3,
    /// where the van parks, and the way its back doors face (toward the mall)
    pub van: V3,
    pub van_yaw: i32,
}

pub const CELL: i32 = int(8);

#[derive(Debug, Clone)]
pub struct Map {
    pub solids: Vec<Solid>,
    pub decor: Vec<Decor>,
    pub zones: Vec<Zone>,
    pub spots: Vec<Spot>,
    pub entrances: Vec<Entrance>,
    pub nodes: Vec<V3>,
    pub links: Vec<Vec<(u16, i32)>>,
    /// the playable area; the map's edge is hidden walls
    pub lo: V3,
    pub hi: V3,
    cols: i32,
    rows: i32,
    cells: Vec<Vec<u16>>,
}

/// What a ray met.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    pub t: i32,
    pub at: V3,
    pub normal: V3,
}

impl Map {
    pub fn new(lo: V3, hi: V3) -> Map {
        let cols = (hi.x - lo.x) / CELL + 1;
        let rows = (hi.z - lo.z) / CELL + 1;
        Map {
            solids: Vec::new(),
            decor: Vec::new(),
            zones: Vec::new(),
            spots: Vec::new(),
            entrances: Vec::new(),
            nodes: Vec::new(),
            links: Vec::new(),
            lo,
            hi,
            cols,
            rows,
            cells: vec![Vec::new(); (cols * rows) as usize],
        }
    }

    fn cell_of(&self, x: i32, z: i32) -> (i32, i32) {
        (
            ((x - self.lo.x) / CELL).clamp(0, self.cols - 1),
            ((z - self.lo.z) / CELL).clamp(0, self.rows - 1),
        )
    }

    pub fn add(&mut self, s: Solid) {
        let i = self.solids.len() as u16;
        let (c0, r0) = self.cell_of(s.b.min.x, s.b.min.z);
        let (c1, r1) = self.cell_of(s.b.max.x, s.b.max.z);
        for r in r0..=r1 {
            for c in c0..=c1 {
                self.cells[(r * self.cols + c) as usize].push(i);
            }
        }
        self.solids.push(s);
    }

    /// The indices of solids whose cells overlap a box (may repeat).
    fn near(&self, b: &Aabb, out: &mut Vec<u16>) {
        out.clear();
        let (c0, r0) = self.cell_of(b.min.x, b.min.z);
        let (c1, r1) = self.cell_of(b.max.x, b.max.z);
        for r in r0..=r1 {
            for c in c0..=c1 {
                out.extend_from_slice(&self.cells[(r * self.cols + c) as usize]);
            }
        }
    }

    /// Does a body's box meet anything solid (or one of the extra boxes)?
    pub fn blocked(&self, b: &Aabb, extra: &[Aabb]) -> bool {
        if b.min.x < self.lo.x || b.max.x > self.hi.x || b.min.z < self.lo.z || b.max.z > self.hi.z {
            return true;
        }
        let mut near = Vec::new();
        self.near(b, &mut near);
        near.iter().any(|&i| self.solids[i as usize].b.overlaps(b)) || extra.iter().any(|e| e.overlaps(b))
    }

    /// The highest top of anything solid under a body's box that it could
    /// stand on (the ground is 0), searching down from `from_y`.
    pub fn floor_under(&self, b: &Aabb, from_y: i32, extra: &[Aabb]) -> i32 {
        let mut near = Vec::new();
        self.near(b, &mut near);
        let mut best = 0;
        let flat =
            |s: &Aabb| s.min.x < b.max.x && s.max.x > b.min.x && s.min.z < b.max.z && s.max.z > b.min.z;
        for &i in near.iter() {
            let s = &self.solids[i as usize].b;
            if flat(s) && s.max.y <= from_y && s.max.y > best {
                best = s.max.y;
            }
        }
        for s in extra {
            if flat(s) && s.max.y <= from_y && s.max.y > best {
                best = s.max.y;
            }
        }
        best
    }

    /// The first thing a ray meets within `max_t`: a solid, an extra box,
    /// or the ground. `shots` skips solids that shots pass through.
    pub fn ray(&self, o: V3, d: V3, max_t: i32, extra: &[Aabb], shots: bool) -> Option<Hit> {
        let mut best: Option<(i32, usize, i32)> = None;
        // the ground
        if d.y < 0 && o.y >= 0 {
            let t = (((o.y as i64) << fx::FRAC_BITS) / (-d.y) as i64).min(i32::MAX as i64) as i32;
            if t <= max_t {
                best = Some((t, 1, 1));
            }
        }
        for e in extra {
            if let Some(h) = e.ray(o, d, max_t) {
                if best.is_none_or(|b| h.0 < b.0) {
                    best = Some(h);
                }
            }
        }
        // walk the grid cells the ray crosses, in order
        let (mut cx, mut cz) = self.cell_of(o.x, o.z);
        let step_x = if d.x > 0 { 1 } else { -1 };
        let step_z = if d.z > 0 { 1 } else { -1 };
        let edge = |c: i32, s: i32, lo: i32| -> i64 { (lo + (c + if s > 0 { 1 } else { 0 }) * CELL) as i64 };
        let big: i64 = i64::MAX / 4;
        let t_to = |edge: i64, o: i32, d: i32| -> i64 {
            if d.abs() < 4 {
                big
            } else {
                ((edge - o as i64) << fx::FRAC_BITS) / d as i64
            }
        };
        let mut tx = t_to(edge(cx, step_x, self.lo.x), o.x, d.x);
        let mut tz = t_to(edge(cz, step_z, self.lo.z), o.z, d.z);
        let dx = if d.x.abs() < 4 {
            big
        } else {
            ((CELL as i64) << fx::FRAC_BITS) / d.x.abs() as i64
        };
        let dz = if d.z.abs() < 4 {
            big
        } else {
            ((CELL as i64) << fx::FRAC_BITS) / d.z.abs() as i64
        };
        let mut guard = 0;
        loop {
            let cell = &self.cells[(cz * self.cols + cx) as usize];
            for &i in cell.iter() {
                let s = &self.solids[i as usize];
                if shots && s.open {
                    continue;
                }
                if let Some(h) = s.b.ray(o, d, max_t) {
                    if best.is_none_or(|b| h.0 < b.0) {
                        best = Some(h);
                    }
                }
            }
            let leave = tx.min(tz);
            if let Some(b) = best {
                if (b.0 as i64) <= leave {
                    break;
                }
            }
            if leave > max_t as i64 {
                break;
            }
            if tx < tz {
                cx += step_x;
                tx += dx;
            } else {
                cz += step_z;
                tz += dz;
            }
            guard += 1;
            if cx < 0 || cz < 0 || cx >= self.cols || cz >= self.rows || guard > 400 {
                break;
            }
        }
        best.map(|(t, axis, sign)| {
            let n = match axis {
                0 => v3(sign * ONE, 0, 0),
                1 => v3(0, sign * ONE, 0),
                _ => v3(0, 0, sign * ONE),
            };
            Hit {
                t,
                at: o.add(d.scale(t)),
                normal: n,
            }
        })
    }

    /// Can a point see another (nothing that stops shots between them)?
    pub fn clear(&self, a: V3, b: V3, extra: &[Aabb]) -> bool {
        let d = b.sub(a);
        let len = d.len();
        if len < ONE / 8 {
            return true;
        }
        self.ray(a, d.norm(), len - ONE / 8, extra, true).is_none()
    }

    /// The smallest zone holding a point: a store before the mall, the mall
    /// before the parking lot.
    pub fn zone_at(&self, p: V3) -> Option<&Zone> {
        self.zones
            .iter()
            .filter(|z| p.x >= z.min.x && p.x <= z.max.x && p.z >= z.min.z && p.z <= z.max.z)
            .min_by_key(|z| ((z.max.x - z.min.x) as i64 >> 8) * ((z.max.z - z.min.z) as i64 >> 8))
    }

    pub fn zone(&self, key: &str) -> Option<&Zone> {
        self.zones.iter().find(|z| z.key == key)
    }

    pub fn entrance(&self, key: &str) -> &Entrance {
        self.entrances
            .iter()
            .find(|e| e.key == key)
            .unwrap_or(&self.entrances[0])
    }

    // ---- the waypoint graph ------------------------------------------------

    pub fn node(&mut self, p: V3) -> usize {
        self.nodes.push(p);
        self.links.push(Vec::new());
        self.nodes.len() - 1
    }

    /// Links every pair of nodes closer than `reach` that a body can walk
    /// between: clear at knee height and at head height.
    pub fn link_nodes(&mut self, reach: i32) {
        let n = self.nodes.len();
        for a in 0..n {
            for b in a + 1..n {
                let (pa, pb) = (self.nodes[a], self.nodes[b]);
                if (pa.x - pb.x).abs() > reach || (pa.z - pb.z).abs() > reach {
                    continue;
                }
                let d = pa.dist(pb);
                if d > reach {
                    continue;
                }
                let up = |p: V3, h: i32| v3(p.x, p.y + h, p.z);
                if self.walkable(up(pa, ONE * 9 / 10), up(pb, ONE * 9 / 10))
                    && self.clear(up(pa, int(2)), up(pb, int(2)), &[])
                {
                    self.links[a].push((b as u16, d));
                    self.links[b].push((a as u16, d));
                }
            }
        }
    }

    /// A body's width clears the way, not only a line.
    fn walkable(&self, a: V3, b: V3) -> bool {
        let side = crate::geom::right_of(crate::geom::yaw_pitch_of(b.sub(a)).0).scale(ONE * 4 / 10);
        self.clear(a, b, &[])
            && self.clear(a.add(side), b.add(side), &[])
            && self.clear(a.sub(side), b.sub(side), &[])
    }

    /// The node nearest a point that it can see.
    pub fn nearest_node(&self, p: V3) -> Option<usize> {
        let mut best: Option<(i32, usize)> = None;
        for (i, n) in self.nodes.iter().enumerate() {
            let dx = (n.x - p.x).abs();
            let dz = (n.z - p.z).abs();
            if dx > int(40) || dz > int(40) {
                continue;
            }
            let d = dx + dz + (n.y - p.y).abs() * 3;
            if best.is_none_or(|b| d < b.0) {
                let eye = v3(p.x, p.y + ONE, p.z);
                if self.clear(eye, v3(n.x, n.y + ONE, n.z), &[]) {
                    best = Some((d, i));
                }
            }
        }
        best.map(|b| b.1)
    }

    /// The nodes from `from` to `to` (both included), cheapest first found
    /// by A*; empty when there is no way, at most `limit` nodes searched.
    pub fn path(&self, from: usize, to: usize, limit: usize) -> Vec<usize> {
        let n = self.nodes.len();
        if from >= n || to >= n {
            return Vec::new();
        }
        let mut cost = vec![i32::MAX; n];
        let mut prev = vec![usize::MAX; n];
        let mut open: Vec<(i32, usize)> = vec![(0, from)];
        cost[from] = 0;
        let goal = self.nodes[to];
        let h = |i: usize| self.nodes[i].sub(goal).len_xz();
        let mut searched = 0;
        while let Some(k) = (0..open.len()).min_by_key(|&k| (open[k].0, open[k].1)) {
            let (_, cur) = open.swap_remove(k);
            if cur == to {
                break;
            }
            searched += 1;
            if searched > limit {
                return Vec::new();
            }
            for &(nb, d) in &self.links[cur] {
                let nb = nb as usize;
                let c = cost[cur].saturating_add(d / 64);
                if c < cost[nb] {
                    cost[nb] = c;
                    prev[nb] = cur;
                    open.push((c + h(nb) / 64, nb));
                }
            }
        }
        if cost[to] == i32::MAX {
            return Vec::new();
        }
        let mut out = vec![to];
        let mut cur = to;
        while cur != from {
            cur = prev[cur];
            if cur == usize::MAX {
                return Vec::new();
            }
            out.push(cur);
        }
        out.reverse();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxy() -> Map {
        let mut m = Map::new(v3(-int(50), 0, -int(50)), v3(int(50), int(20), int(50)));
        m.add(Solid {
            b: Aabb::new(v3(int(5), 0, -int(5)), v3(int(6), int(4), int(5))),
            rgb: [0; 3],
            look: Look::Plain,
            open: false,
        });
        m
    }

    #[test]
    fn a_ray_stops_at_the_first_wall_and_the_ground() {
        let m = boxy();
        let h = m.ray(v3(0, ONE, 0), v3(ONE, 0, 0), int(40), &[], true).unwrap();
        assert!((h.t - int(5)).abs() < ONE / 50, "{}", h.t);
        assert_eq!(h.normal, v3(-ONE, 0, 0));
        let down = m
            .ray(v3(0, int(3), 0), v3(0, -ONE, 0), int(10), &[], true)
            .unwrap();
        assert!((down.t - int(3)).abs() < 4);
        assert!(m.ray(v3(0, ONE, 0), v3(-ONE, 0, 0), int(40), &[], true).is_none());
        // over the wall's top
        assert!(m.clear(v3(0, int(5), 0), v3(int(10), int(5), 0), &[]));
        assert!(!m.clear(v3(0, int(2), 0), v3(int(10), int(2), 0), &[]));
    }

    #[test]
    fn a_ray_crossing_many_cells_finds_a_far_box() {
        let mut m = boxy();
        m.add(Solid {
            b: Aabb::new(v3(-int(40), 0, int(30)), v3(-int(38), int(4), int(34))),
            rgb: [0; 3],
            look: Look::Plain,
            open: false,
        });
        let d = v3(-int(39), 0, int(32)).sub(v3(0, 0, 0)).norm();
        let h = m.ray(v3(0, ONE, 0), d, int(80), &[], true).unwrap();
        assert!(h.t > int(40) && h.t < int(52), "{}", h.t);
    }

    #[test]
    fn a_box_blocks_a_body_and_its_top_is_a_floor() {
        let m = boxy();
        let body = Aabb::standing(v3(int(5), 0, 0), ONE / 2, int(2));
        assert!(m.blocked(&body, &[]));
        let clear = Aabb::standing(v3(0, 0, 0), ONE / 2, int(2));
        assert!(!m.blocked(&clear, &[]));
        assert_eq!(
            m.floor_under(
                &Aabb::standing(v3(int(5), int(5), 0), ONE / 2, int(2)),
                int(5),
                &[]
            ),
            int(4)
        );
    }

    #[test]
    fn a_path_goes_round_a_wall() {
        let mut m = boxy();
        let a = m.node(v3(0, 0, 0));
        let b = m.node(v3(int(12), 0, 0));
        let c = m.node(v3(int(3), 0, int(9)));
        let _d = m.node(v3(int(9), 0, int(9)));
        m.link_nodes(int(13));
        assert!(!m.links[a].iter().any(|l| l.0 as usize == b), "the wall cuts a-b");
        let p = m.path(a, b, 100);
        assert!(
            p.len() >= 3 && p[0] == a && p[p.len() - 1] == b && p.contains(&c),
            "{p:?}"
        );
    }
}
