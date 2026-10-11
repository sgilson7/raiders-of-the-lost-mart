//! The one map: The Streets at Southpoint, Durham, as an abandoned mall, and
//! the parking lots and streets around it. Built from boxes by code, the
//! way Ribbit King's holes were laid out by tools/courses.py, so a change
//! is one line here and the same map comes out every time.
//!
//! The layout follows the real mall loosely, scaled to play (metres, x east,
//! z south, the floor at y = 0):
//!
//! * a long two-level concourse running north and south, stores both sides,
//!   balconies overhead joined by two bridges;
//! * a centre court with a dry fountain, two escalators to the balconies and
//!   a skylight, opening east into the food court;
//! * three anchor department stores (north, south and west, the west one
//!   reached down a short wing);
//! * an outdoor brick "Main Street" to the south-east with a cinema at its
//!   end, reached from the concourse down a side corridor;
//! * five entrances (north, south, west, east through the food court, and the
//!   head of Main Street), each with a parking spot for the van far out in
//!   the lot, so the mall is a long walk away when the doors open.
//!
//! Words (zone and store names) live in data/copy.json under `zone_<key>`.

use crate::content::{Palette, Rgb};
use crate::fx::{deg, int, ONE};
use crate::geom::{v3, V3};
use crate::mesh::Mesh;
use crate::rng::Rng;
use crate::world::{Aabb, Decor, Entrance, Look, Map, Solid, Spot, SpotKind, Zone};

/// Metres with a fraction: `m(12, 5)` is 12.5 m.
const fn m(whole: i32, tenths: i32) -> i32 {
    whole * ONE + tenths * ONE / 10
}

/// A roof's thickness: a wall that stands on a roof starts at its top.
const ROOF_T: i32 = ONE * 6 / 10;

/// The height every indoor floor is drawn at, clear of the ground under it.
const FLOOR_Y: i32 = ONE * 4 / 100;

/// Centimetres.
const fn cm(n: i32) -> i32 {
    n * ONE / 100
}

const fn mm(n: i32) -> i32 {
    n * ONE
}

/// The playable area.
pub const LO_X: i32 = -280;
pub const HI_X: i32 = 280;
pub const LO_Z: i32 = -300;
pub const HI_Z: i32 = 300;

/// The concourse's half width, the stores' depth line and heights.
const HALL: i32 = 9;
const STORE_BACK: i32 = 33;
const STORE_H: i32 = 6;
const DOOR_H: i32 = m(4, 5);
const HALL_H: i32 = 12;
const BALC_Y: i32 = m(5, 6);
const BALC_T: i32 = m(0, 4);
const BALC_IN: i32 = 5;
const RAIL_H: i32 = m(1, 0);
const WALL_T: i32 = m(0, 4);

/// What a store holds, which decides its fittings.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Racks,
    Shelves,
    HotTopic,
    Security,
    Electronics,
    Jewelry,
    Arcade,
    Food,
    Books,
}

struct B<'a> {
    map: Map,
    pal: &'a Palette,
    rng: Rng,
}

impl<'a> B<'a> {
    fn c(&self, name: &str) -> Rgb {
        self.pal.get(name)
    }

    fn solid(&mut self, a: V3, b: V3, colour: &str, look: Look) {
        let rgb = self.c(colour);
        self.map.add(Solid {
            b: Aabb::new(a, b),
            rgb,
            look,
            open: false,
        });
    }

    /// A box shots fly through: railings, fences.
    fn open_solid(&mut self, a: V3, b: V3, colour: &str) {
        let rgb = self.c(colour);
        self.map.add(Solid {
            b: Aabb::new(a, b),
            rgb,
            look: Look::Plain,
            open: true,
        });
    }

    fn decor(&mut self, mesh: Mesh, at: V3, size: V3, yaw: i32, colour: &str, glow: i32, inked: bool) {
        let rgb = self.c(colour);
        self.map.decor.push(Decor {
            mesh,
            at,
            size,
            yaw,
            rgb,
            glow,
            inked,
        });
    }

    /// A flat patch of floor or paint, drawn just above the ground.
    fn patch(&mut self, x0: i32, z0: i32, x1: i32, z1: i32, y: i32, colour: &str) {
        let at = v3((x0 + x1) / 2, y, (z0 + z1) / 2);
        let size = v3((x1 - x0).abs(), ONE, (z1 - z0).abs());
        self.decor(Mesh::Ground, at, size, 0, colour, 0, false);
    }

    /// A wall along z at x, from z0 to z1, with doorways (z ranges, opening up
    /// to `door_h`) cut out of it.
    fn wall_x(
        &mut self,
        x: i32,
        z0: i32,
        z1: i32,
        y0: i32,
        y1: i32,
        gaps: &[(i32, i32)],
        door_h: i32,
        colour: &str,
        look: Look,
    ) {
        let h = WALL_T / 2;
        let mut z = z0;
        let mut gs: Vec<(i32, i32)> = gaps.to_vec();
        gs.sort();
        for (g0, g1) in gs {
            if g0 > z {
                self.solid(v3(x - h, y0, z), v3(x + h, y1, g0), colour, look);
            }
            if door_h < y1 {
                self.solid(v3(x - h, door_h, g0), v3(x + h, y1, g1), colour, look);
            }
            z = g1;
        }
        if z < z1 {
            self.solid(v3(x - h, y0, z), v3(x + h, y1, z1), colour, look);
        }
    }

    fn wall_z(
        &mut self,
        z: i32,
        x0: i32,
        x1: i32,
        y0: i32,
        y1: i32,
        gaps: &[(i32, i32)],
        door_h: i32,
        colour: &str,
        look: Look,
    ) {
        let h = WALL_T / 2;
        let mut x = x0;
        let mut gs: Vec<(i32, i32)> = gaps.to_vec();
        gs.sort();
        for (g0, g1) in gs {
            if g0 > x {
                self.solid(v3(x, y0, z - h), v3(g0, y1, z + h), colour, look);
            }
            if door_h < y1 {
                self.solid(v3(g0, door_h, z - h), v3(g1, y1, z + h), colour, look);
            }
            x = g1;
        }
        if x < x1 {
            self.solid(v3(x, y0, z - h), v3(x1, y1, z + h), colour, look);
        }
    }

    fn zone(&mut self, key: &str, x0: i32, z0: i32, x1: i32, z1: i32, indoors: bool) {
        self.map.zones.push(Zone {
            key: key.to_string(),
            min: v3(mm(x0.min(x1)), -int(10), mm(z0.min(z1))),
            max: v3(mm(x0.max(x1)), int(40), mm(z0.max(z1))),
            indoors,
        });
    }

    fn loot(&mut self, at: V3, zone: &str) {
        self.map.spots.push(Spot {
            kind: SpotKind::Loot,
            key: String::new(),
            at,
            zone: zone.to_string(),
        });
    }

    fn mission_spot(&mut self, key: &str, at: V3, zone: &str) {
        self.map.spots.push(Spot {
            kind: SpotKind::Mission,
            key: key.to_string(),
            at,
            zone: zone.to_string(),
        });
    }

    fn node(&mut self, x: i32, y: i32, z: i32) {
        let p = v3(x, y, z);
        let probe = Aabb::standing(v3(p.x, p.y + ONE / 10, p.z), ONE * 4 / 10, int(1));
        if !self.map.blocked(&probe, &[]) {
            self.map.node(p);
        }
    }

    /// A crate, a register or a bin: something to search, drawn as a box.
    fn container(&mut self, x: i32, z: i32, y: i32, colour: &str, zone: &str) {
        let h = m(0, 9);
        self.solid(
            v3(x - m(0, 5), y, z - m(0, 4)),
            v3(x + m(0, 5), y + h, z + m(0, 4)),
            colour,
            Look::Plain,
        );
        self.loot(v3(x, y + h, z), zone);
    }

    // ---- stores ---------------------------------------------------------------

    /// A store on the west (side −1) or east (+1) of the concourse, between
    /// z0 and z1 (metres), its front open to the concourse.
    fn store(&mut self, key: &str, side: i32, z0: i32, z1: i32, kind: Kind, wall: &str, sign: &str) {
        let (zf0, zf1) = (mm(z0), mm(z1));
        let front = mm(HALL) * side;
        let back = mm(STORE_BACK) * side;
        let (xa, xb) = (front.min(back), front.max(back));
        let cz = (zf0 + zf1) / 2;
        let wall_c = if kind == Kind::HotTopic { "goth" } else { wall };
        // side and back walls, and the roof
        self.wall_z(zf0, xa, xb, 0, mm(STORE_H), &[], 0, wall_c, Look::Plain);
        self.wall_z(zf1, xa, xb, 0, mm(STORE_H), &[], 0, wall_c, Look::Plain);
        self.wall_x(back, zf0, zf1, 0, mm(STORE_H), &[], 0, "stucco_dark", Look::Plain);
        // the roof stops at the front wall rather than running into it
        let (ra, rb) = if side > 0 {
            (xa + WALL_T / 2, xb)
        } else {
            (xa, xb - WALL_T / 2)
        };
        self.solid(
            v3(ra, mm(STORE_H), zf0),
            v3(rb, mm(STORE_H) + m(0, 6), zf1),
            "roof",
            Look::Flat,
        );
        // the front: two piers, a doorway as wide as the store allows, a sign
        let gap = (zf0 + int(2), zf1 - int(2));
        self.wall_x(
            front,
            zf0,
            zf1,
            0,
            mm(STORE_H),
            &[gap],
            DOOR_H,
            wall_c,
            Look::Plain,
        );
        // a hand's breadth off the lintel's face, so the two do not flicker
        let sx = front - side * cm(36);
        self.solid(
            v3(sx - m(0, 1), DOOR_H + m(0, 2), cz - int(4)),
            v3(sx + m(0, 1), DOOR_H + m(1, 0), cz + int(4)),
            sign,
            Look::Glow,
        );
        // the floor
        let floor = if kind == Kind::HotTopic || kind == Kind::Arcade {
            "black"
        } else {
            "carpet"
        };
        self.patch(xa, zf0, xb, zf1, FLOOR_Y, floor);
        self.zone(
            key,
            (HALL * side).min(STORE_BACK * side),
            z0,
            (HALL * side).max(STORE_BACK * side),
            z1,
            true,
        );
        // fittings
        let inner = |t: i32| front + side * mm(t);
        match kind {
            Kind::Racks | Kind::HotTopic => {
                let rack = if kind == Kind::HotTopic { "black" } else { "rack" };
                for row in 0..3 {
                    let x = inner(6 + row * 6);
                    for k in 0..2 {
                        let z = cz - int(4) + k * int(8);
                        self.solid(
                            v3(x - m(0, 3), 0, z - int(2)),
                            v3(x + m(0, 3), m(1, 5), z + int(2)),
                            rack,
                            Look::Plain,
                        );
                        // clothes on the rail
                        let cloth = if kind == Kind::HotTopic {
                            "shirt"
                        } else {
                            ["wallA", "teal", "sage", "wallD"][(row + k) as usize % 4]
                        };
                        self.decor(
                            Mesh::Cube,
                            v3(x, m(1, 0), z),
                            v3(m(0, 8), m(0, 9), int(3)),
                            0,
                            cloth,
                            0,
                            true,
                        );
                    }
                }
                if kind == Kind::HotTopic {
                    // the display case at the back, and band posters
                    let x = inner(20);
                    self.solid(
                        v3(x - int(1), 0, cz - int(1)),
                        v3(x + int(1), m(1, 0), cz + int(1)),
                        "black",
                        Look::Plain,
                    );
                    self.decor(
                        Mesh::Cube,
                        v3(x, m(1, 6), cz),
                        v3(m(1, 8), m(1, 2), m(1, 8)),
                        0,
                        "case",
                        ONE / 3,
                        true,
                    );
                    self.mission_spot("hottopic", v3(x, m(1, 0), cz), key);
                    for k in 0..4 {
                        let z = zf0 + int(2) + k * int(4);
                        self.decor(
                            Mesh::Panel,
                            v3(back - side * m(0, 3), m(2, 6), z),
                            v3(m(1, 4), ONE, m(2, 0)),
                            deg(90) * side,
                            ["sign_d", "sign_e", "sign_a", "purple"][k as usize],
                            ONE / 4,
                            true,
                        );
                    }
                }
                self.container(inner(19), zf0 + int(2), 0, "counter", key);
            }
            Kind::Shelves | Kind::Books => {
                let c = if kind == Kind::Books { "wood" } else { "shelf" };
                for row in 0..3 {
                    let x = inner(6 + row * 5);
                    self.solid(
                        v3(x - m(0, 5), 0, cz - int(5)),
                        v3(x + m(0, 5), m(2, 2), cz + int(5)),
                        c,
                        Look::Plain,
                    );
                }
                self.container(inner(20), cz + int(5), 0, "counter", key);
                self.container(inner(3), zf1 - int(2), 0, "shelf", key);
            }
            Kind::Security => {
                // desks, a wall of dead monitors with one still lit, lockers
                for k in 0..2 {
                    let z = cz - int(3) + k * int(6);
                    self.solid(
                        v3(inner(8) - int(1), 0, z - m(1, 5)),
                        v3(inner(8) + int(1), m(0, 8), z + m(1, 5)),
                        "wood",
                        Look::Plain,
                    );
                }
                for k in 0..6 {
                    let z = zf0 + int(2) + k * int(2);
                    let lit = if k == 3 { ONE / 2 } else { 0 };
                    let colour = if k == 3 { "sign_b" } else { "black" };
                    self.decor(
                        Mesh::Cube,
                        v3(back - side * m(0, 4), m(2, 2), z),
                        v3(m(0, 3), m(1, 4), m(1, 8)),
                        0,
                        colour,
                        lit,
                        true,
                    );
                }
                self.solid(
                    v3(inner(14) - int(1), 0, zf1 - int(2)),
                    v3(inner(14) + int(1), m(2, 2), zf1 - m(0, 5)),
                    "metal",
                    Look::Plain,
                );
                self.mission_spot("security", v3(inner(8), m(0, 8), cz - int(3)), key);
                self.container(inner(14), zf0 + int(2), 0, "metal", key);
            }
            Kind::Electronics => {
                for row in 0..2 {
                    let x = inner(7 + row * 7);
                    for k in 0..2 {
                        let z = cz - int(5) + k * int(10);
                        self.solid(
                            v3(x - int(1), 0, z - int(2)),
                            v3(x + int(1), m(0, 9), z + int(2)),
                            "cream",
                            Look::Plain,
                        );
                    }
                }
                for k in 0..5 {
                    let z = zf0 + int(3) + k * int(4);
                    self.decor(
                        Mesh::Cube,
                        v3(back - side * m(0, 3), m(2, 4), z),
                        v3(m(0, 2), m(1, 8), int(3)),
                        0,
                        "black",
                        0,
                        true,
                    );
                }
                self.mission_spot("electronics", v3(inner(14), m(0, 9), cz + int(5)), key);
                self.container(inner(20), cz, 0, "counter", key);
            }
            Kind::Jewelry => {
                // a U of glass counters
                self.solid(
                    v3(inner(6) - m(0, 5), 0, zf0 + int(3)),
                    v3(inner(6) + m(0, 5), m(1, 0), zf1 - int(3)),
                    "glass",
                    Look::Plain,
                );
                self.solid(
                    v3(inner(6).min(inner(16)), 0, zf0 + int(3)),
                    v3(inner(6).max(inner(16)), m(1, 0), zf0 + int(4)),
                    "glass",
                    Look::Plain,
                );
                self.solid(
                    v3(inner(6).min(inner(16)), 0, zf1 - int(4)),
                    v3(inner(6).max(inner(16)), m(1, 0), zf1 - int(3)),
                    "glass",
                    Look::Plain,
                );
                self.mission_spot("jewelry", v3(inner(6), m(1, 0), cz), key);
                self.container(inner(20), cz, 0, "counter", key);
            }
            Kind::Arcade => {
                for row in 0..2 {
                    let x = inner(6 + row * 8);
                    for k in 0..4 {
                        let z = zf0 + int(3) + k * int(3);
                        self.solid(
                            v3(x - m(0, 6), 0, z - m(0, 6)),
                            v3(x + m(0, 6), m(1, 9), z + m(0, 6)),
                            ["sign_a", "sign_b", "sign_d", "sign_c"][k as usize],
                            Look::Plain,
                        );
                    }
                }
                self.container(inner(20), cz, 0, "metal", key);
            }
            Kind::Food => {
                self.solid(
                    v3(inner(4) - m(0, 5), 0, zf0 + int(2)),
                    v3(inner(4) + m(0, 5), m(1, 1), zf1 - int(2)),
                    "counter",
                    Look::Plain,
                );
                self.container(inner(10), cz, 0, "metal", key);
            }
        }
        // a nav node at the door and two inside
        self.node(front - side * int(3), 0, cz);
        self.node(inner(4), 0, cz);
        self.node(inner(16), 0, cz);
        if kind != Kind::Racks {
            self.loot(v3(inner(12), ONE / 2, cz), key);
        }
    }

    // ---- the concourse and the courts --------------------------------------

    fn concourse(&mut self) {
        // floor and the upper storefronts above the stores' roofs
        // the court has its own floor: two floors in one plane flicker
        self.patch(-mm(HALL), -int(120), mm(HALL), -int(16), FLOOR_Y, "tile");
        self.patch(-mm(HALL), int(16), mm(HALL), int(120), FLOOR_Y, "tile");
        for side in [-1, 1] {
            let x = mm(HALL) * side;
            for (z0, z1) in [(-120, -16), (16, 120)] {
                self.solid(
                    v3(x - WALL_T / 2, mm(STORE_H), mm(z0)),
                    v3(x + WALL_T / 2, mm(HALL_H), mm(z1)),
                    "stucco",
                    Look::Facade,
                );
                // the balcony and its railing
                let xi = mm(BALC_IN) * side;
                // the balcony stops at the face of the wall behind it
                let xw = (mm(HALL) - WALL_T / 2) * side;
                self.solid(
                    v3(xi.min(xw), BALC_Y - BALC_T, mm(z0)),
                    v3(xi.max(xw), BALC_Y, mm(z1)),
                    "stucco_trim",
                    Look::Plain,
                );
                self.open_solid(
                    v3(xi - ONE / 20, BALC_Y, mm(z0)),
                    v3(xi + ONE / 20, BALC_Y + RAIL_H, mm(z1)),
                    "rail",
                );
                let mut z = z0 + 6;
                while z < z1 {
                    self.node(mm(7) * side, BALC_Y, mm(z));
                    z += 12;
                }
            }
        }
        // two bridges across
        for cz in [-64, 64] {
            let (z0, z1) = (mm(cz - 2), mm(cz + 2));
            self.solid(
                v3(-mm(BALC_IN), BALC_Y - BALC_T, z0),
                v3(mm(BALC_IN), BALC_Y, z1),
                "stucco_trim",
                Look::Plain,
            );
            self.open_solid(
                v3(-mm(BALC_IN), BALC_Y, z0 - ONE / 20),
                v3(mm(BALC_IN), BALC_Y + RAIL_H, z0 + ONE / 20),
                "rail",
            );
            self.open_solid(
                v3(-mm(BALC_IN), BALC_Y, z1 - ONE / 20),
                v3(mm(BALC_IN), BALC_Y + RAIL_H, z1 + ONE / 20),
                "rail",
            );
            self.node(0, BALC_Y, mm(cz));
        }
        // the roof with skylights every 24 m
        for (z0, z1) in [(-120, -16), (16, 120)] {
            let mut z = z0;
            while z < z1 {
                let zn = (z + 24).min(z1);
                self.solid(
                    v3(-mm(HALL), mm(HALL_H), mm(z)),
                    v3(mm(HALL), mm(HALL_H) + m(0, 6), mm(zn - 6)),
                    "roof",
                    Look::Flat,
                );
                self.solid(
                    v3(-mm(HALL), mm(HALL_H) + m(0, 3), mm(zn - 6)),
                    v3(mm(HALL), mm(HALL_H) + m(0, 5), mm(zn)),
                    "skylight",
                    Look::Glow,
                );
                z = zn;
            }
        }
        // the spine of nodes, benches and planters down the middle
        let mut z = -114;
        while z <= 114 {
            if !(-16..=16).contains(&z) {
                self.node(0, 0, mm(z));
                if (z + 120) % 24 == 6 && z.abs() > 30 {
                    // a planter with a tree, and a loot bin by it
                    self.solid(
                        v3(-int(2), 0, mm(z) - int(2)),
                        v3(int(2), m(0, 7), mm(z) + int(2)),
                        "stucco_dark",
                        Look::Plain,
                    );
                    self.decor(
                        Mesh::Cyl,
                        v3(0, m(2, 0), mm(z)),
                        v3(m(0, 4), int(3), m(0, 4)),
                        0,
                        "trunk",
                        0,
                        true,
                    );
                    self.decor(
                        Mesh::Blob,
                        v3(0, m(4, 2), mm(z)),
                        v3(int(4), int(3), int(4)),
                        0,
                        "plant",
                        0,
                        true,
                    );
                }
            }
            z += 12;
        }
        self.zone("concourse", -HALL, -120, HALL, 120, true);
    }

    fn centre_court(&mut self) {
        let (x0, x1, z0, z1) = (-24, 24, -16, 16);
        self.patch(mm(x0), mm(z0), mm(x1), mm(z1), FLOOR_Y, "court");
        // north and south walls beside the concourse mouth, and the west wall
        // with the wing corridor's mouth
        // (the stores' own side walls stand below the stores' roofs: two walls
        // in one plane in different colours flicker, so these start above them)
        for z in [z0, z1] {
            self.wall_z(
                mm(z),
                mm(x0),
                mm(x1),
                mm(STORE_H) + ROOF_T,
                mm(14),
                &[(-mm(HALL), mm(HALL))],
                mm(HALL_H) + ROOF_T,
                "stucco",
                Look::Plain,
            );
        }
        self.wall_x(
            mm(x0),
            mm(z0),
            mm(z1),
            0,
            mm(14),
            &[(-int(6), int(6))],
            int(8) + ROOF_T,
            "stucco",
            Look::Plain,
        );
        // east: open below to the food court, wall above it
        self.wall_x(
            mm(x1),
            mm(z0),
            mm(z1),
            int(9) + ROOF_T,
            mm(14),
            &[],
            0,
            "stucco",
            Look::Facade,
        );
        // the high roof and its skylight
        self.solid(
            v3(mm(x0), mm(14), mm(z0)),
            v3(mm(x1), mm(14) + m(0, 6), -int(6)),
            "roof",
            Look::Flat,
        );
        self.solid(
            v3(mm(x0), mm(14), int(6)),
            v3(mm(x1), mm(14) + m(0, 6), mm(z1)),
            "roof",
            Look::Flat,
        );
        self.solid(
            v3(mm(x0), mm(14) + m(0, 3), -int(6)),
            v3(mm(x1), mm(14) + m(0, 5), int(6)),
            "skylight",
            Look::Glow,
        );
        // balconies round the court: north and south bands, and west and east strips
        let band = |b: &mut B, a: V3, c: V3| b.solid(a, c, "stucco_trim", Look::Plain);
        band(
            self,
            v3(mm(x0), BALC_Y - BALC_T, mm(z0)),
            v3(mm(x1), BALC_Y, -int(12)),
        );
        band(
            self,
            v3(mm(x0), BALC_Y - BALC_T, int(12)),
            v3(mm(x1), BALC_Y, mm(z1)),
        );
        band(
            self,
            v3(mm(x0), BALC_Y - BALC_T, -int(12)),
            v3(-int(20), BALC_Y, int(12)),
        );
        band(
            self,
            v3(int(20), BALC_Y - BALC_T, -int(12)),
            v3(mm(x1), BALC_Y, int(12)),
        );
        // railings on the inner edges, leaving room for the escalators' tops
        let rail = |b: &mut B, a: V3, c: V3| b.open_solid(a, c, "rail");
        rail(
            self,
            v3(-int(20), BALC_Y, -int(12) - ONE / 20),
            v3(int(20), BALC_Y + RAIL_H, -int(12) + ONE / 20),
        );
        rail(
            self,
            v3(-int(20), BALC_Y, int(12) - ONE / 20),
            v3(int(20), BALC_Y + RAIL_H, int(12) + ONE / 20),
        );
        rail(
            self,
            v3(-int(20) - ONE / 20, BALC_Y, -int(8)),
            v3(-int(20) + ONE / 20, BALC_Y + RAIL_H, int(12)),
        );
        rail(
            self,
            v3(int(20) - ONE / 20, BALC_Y, -int(12)),
            v3(int(20) + ONE / 20, BALC_Y + RAIL_H, int(8)),
        );
        // two escalators: west one rises westward along z −11..−9, east one
        // eastward along z 9..11, sixteen steps each
        self.escalator(-1, -int(11));
        self.escalator(1, int(9));
        // the dry fountain
        self.solid(
            v3(-int(3), 0, -int(7)),
            v3(int(3), m(0, 6), -int(1)),
            "stucco_dark",
            Look::Plain,
        );
        self.decor(
            Mesh::Cyl,
            v3(0, m(1, 0), -int(4)),
            v3(m(0, 8), int(2), m(0, 8)),
            0,
            "stucco_trim",
            0,
            true,
        );
        self.decor(
            Mesh::Ground,
            v3(0, cm(62), -int(4)),
            v3(int(5), ONE, int(5)),
            0,
            "dry",
            0,
            false,
        );
        self.mission_spot("generator", v3(0, 0, int(8)), "centercourt");
        self.loot(v3(int(18), ONE / 2, -int(14)), "centercourt");
        self.loot(v3(-int(16), BALC_Y, int(14)), "centercourt");
        for x in [-18, -8, 0, 8, 18] {
            for z in [-12, 0, 12] {
                if !(x == 0 && z == 0) {
                    self.node(mm(x), 0, mm(z));
                }
            }
        }
        for x in [-16, -4, 4, 16] {
            self.node(mm(x), BALC_Y, -int(14));
            self.node(mm(x), BALC_Y, int(14));
        }
        self.node(-int(22), BALC_Y, 0);
        self.node(int(22), BALC_Y, 0);
        self.zone("centercourt", x0, z0, x1, z1, true);
    }

    fn escalator(&mut self, dir: i32, z0: i32) {
        let z1 = z0 + int(2);
        let steps = 16;
        let run = int(14) / steps;
        let rise = BALC_Y / steps;
        for i in 0..steps {
            let xa = dir * (int(6) + run * i);
            let xb = dir * (int(6) + run * (i + 1));
            self.solid(
                v3(xa.min(xb), 0, z0),
                v3(xa.max(xb), rise * (i + 1), z1),
                "metal",
                Look::Plain,
            );
        }
        let (xa, xb) = (dir * int(6), dir * int(20));
        for z in [z0 - ONE / 10, z1] {
            self.open_solid(
                v3(xa.min(xb), int(1), z),
                v3(xa.max(xb), BALC_Y + RAIL_H, z + ONE / 10),
                "rail",
            );
        }
        self.node(dir * int(4), 0, z0 + ONE);
        self.node(dir * int(13), rise * 8, z0 + ONE);
        self.node(dir * int(22), BALC_Y, z0 + ONE);
    }

    fn food_court(&mut self) {
        let (x0, x1, z0, z1) = (24, 64, -16, 16);
        self.patch(mm(x0), mm(z0), mm(x1), mm(z1), FLOOR_Y, "tile_dark");
        // west of x 33 the east stores' side walls stand to their roofs
        for z in [z0, z1] {
            self.wall_z(mm(z), int(33), mm(x1), 0, int(9), &[], 0, "stucco", Look::Plain);
            self.wall_z(
                mm(z),
                mm(x0),
                int(33),
                mm(STORE_H) + ROOF_T,
                int(9),
                &[],
                0,
                "stucco",
                Look::Plain,
            );
        }
        self.wall_x(
            mm(x1),
            mm(z0),
            mm(z1),
            0,
            int(9),
            &[(-int(4), int(4))],
            int(4),
            "stucco",
            Look::Plain,
        );
        // roof with a long skylight
        self.solid(
            v3(mm(x0), int(9), mm(z0)),
            v3(mm(x1), int(9) + m(0, 6), -int(4)),
            "roof",
            Look::Flat,
        );
        self.solid(
            v3(mm(x0), int(9), int(4)),
            v3(mm(x1), int(9) + m(0, 6), mm(z1)),
            "roof",
            Look::Flat,
        );
        self.solid(
            v3(mm(x0), int(9) + m(0, 3), -int(4)),
            v3(mm(x1), int(9) + m(0, 5), int(4)),
            "skylight",
            Look::Glow,
        );
        // stalls along both long walls: a counter, a back board, a lit sign
        let signs = ["sign_a", "sign_c", "sign_e", "sign_b", "sign_d"];
        for (k, x) in (0..5).zip([28, 35, 42, 49, 56]) {
            for side in [-1, 1] {
                let zw = mm(if side < 0 { z0 } else { z1 });
                let zc = zw - side * int(3);
                self.solid(
                    v3(mm(x), 0, zc - m(0, 4)),
                    v3(mm(x + 6), m(1, 1), zc + m(0, 4)),
                    "counter",
                    Look::Plain,
                );
                self.solid(
                    v3(mm(x), int(4), zw - side * m(0, 4)),
                    v3(mm(x + 6), int(5), zw - side * m(0, 2)),
                    signs[(k + (side + 1) as usize) % 5],
                    Look::Glow,
                );
                if k % 2 == 0 {
                    self.loot(v3(mm(x + 3), m(1, 1), zc), "foodcourt");
                }
            }
        }
        // tables and chairs, some knocked over
        let mut rng = Rng::new(31);
        for tx in 0..4 {
            for tz in 0..3 {
                let x = mm(32 + tx * 7);
                let z = mm(-6 + tz * 6);
                if rng.chance(80) {
                    self.solid(
                        v3(x - m(0, 6), 0, z - m(0, 6)),
                        v3(x + m(0, 6), cm(75), z + m(0, 6)),
                        "table",
                        Look::Plain,
                    );
                }
                for (dx, dz) in [(-1, 0), (1, 0)] {
                    let tipped = rng.chance(30);
                    let h = if tipped { m(0, 3) } else { m(0, 9) };
                    self.decor(
                        Mesh::Cube,
                        v3(x + dx * m(1, 1), h / 2, z + dz * m(1, 1)),
                        v3(m(0, 5), h, m(0, 5)),
                        rng.range(0, deg(90)),
                        "chair",
                        0,
                        true,
                    );
                }
            }
        }
        for x in [30, 40, 50, 60] {
            for z in [-8, 0, 8] {
                self.node(mm(x), 0, mm(z));
            }
        }
        self.node(int(68), 0, 0);
        self.zone("foodcourt", x0, z0, x1, z1, true);
    }

    fn west_wing(&mut self) {
        let (x0, x1) = (-60, -24);
        self.patch(mm(x0), -int(6), mm(x1), int(6), FLOOR_Y, "tile");
        self.wall_z(-int(6), mm(x0), mm(x1), 0, int(8), &[], 0, "stucco", Look::Plain);
        self.wall_z(int(6), mm(x0), mm(x1), 0, int(8), &[], 0, "stucco", Look::Plain);
        self.solid(
            v3(mm(x0), int(8), -int(6)),
            v3(mm(x1), int(8) + m(0, 6), int(6)),
            "roof",
            Look::Flat,
        );
        for x in [-54, -44, -34] {
            self.node(mm(x), 0, 0);
        }
        self.loot(v3(-int(40), ONE / 2, -int(5)), "westwing");
        self.zone("westwing", x0, -6, x1, 6, true);
    }

    /// A department store: a big box with racks, counters and doors.
    #[allow(clippy::too_many_arguments)]
    fn anchor(
        &mut self,
        key: &str,
        x0: i32,
        z0: i32,
        x1: i32,
        z1: i32,
        band: &str,
        doors: &[(char, i32, i32)],
        inner: char,
    ) {
        let h = int(10);
        let (fx0, fz0, fx1, fz1) = (mm(x0), mm(z0), mm(x1), mm(z1));
        self.patch(fx0, fz0, fx1, fz1, FLOOR_Y, "tile_dark");
        let gaps = |side: char| -> Vec<(i32, i32)> {
            doors
                .iter()
                .filter(|d| d.0 == side)
                .map(|d| (mm(d.1), mm(d.2)))
                .collect()
        };
        self.wall_z(fz0, fx0, fx1, 0, h, &gaps('n'), int(5), "stucco", Look::Plain);
        self.wall_z(fz1, fx0, fx1, 0, h, &gaps('s'), int(5), "stucco", Look::Plain);
        self.wall_x(fx0, fz0, fz1, 0, h, &gaps('w'), int(5), "stucco", Look::Plain);
        self.wall_x(fx1, fz0, fz1, 0, h, &gaps('e'), int(5), "stucco", Look::Plain);
        self.solid(v3(fx0, h, fz0), v3(fx1, h + m(0, 8), fz1), "roof", Look::Flat);
        // the store's colour band round the top of the outside: not on the
        // side that faces into the mall
        for (side, (a, b)) in ['n', 's', 'w', 'e'].into_iter().zip([
            (
                v3(fx0 - m(0, 3), h - int(2), fz0 - m(0, 3)),
                v3(fx1 + m(0, 3), h - int(1), fz0),
            ),
            (
                v3(fx0 - m(0, 3), h - int(2), fz1),
                v3(fx1 + m(0, 3), h - int(1), fz1 + m(0, 3)),
            ),
            (v3(fx0 - m(0, 3), h - int(2), fz0), v3(fx0, h - int(1), fz1)),
            (v3(fx1, h - int(2), fz0), v3(fx1 + m(0, 3), h - int(1), fz1)),
        ]) {
            if side != inner {
                self.solid(a, b, band, Look::Plain);
            }
        }
        // racks and counters in a grid, every other one a container
        let mut rng = Rng::new((x0 * 7 + z0 * 13) as u64);
        let mut x = x0 + 7;
        let mut k = 0;
        while x < x1 - 5 {
            let mut z = z0 + 7;
            while z < z1 - 5 {
                let (cx, cz) = (mm(x), mm(z));
                let roll = rng.range(0, 9);
                if roll < 5 {
                    self.solid(
                        v3(cx - m(1, 5), 0, cz - m(0, 3)),
                        v3(cx + m(1, 5), m(1, 5), cz + m(0, 3)),
                        "rack",
                        Look::Plain,
                    );
                    let cloth = ["wallA", "teal", "sage", "wallD", "purple"][rng.range(0, 4) as usize];
                    self.decor(
                        Mesh::Cube,
                        v3(cx, m(1, 0), cz),
                        v3(int(3), m(0, 9), m(0, 8)),
                        0,
                        cloth,
                        0,
                        true,
                    );
                } else if roll < 7 {
                    self.solid(
                        v3(cx - int(2), 0, cz - int(1)),
                        v3(cx + int(2), m(1, 0), cz + int(1)),
                        "glass",
                        Look::Plain,
                    );
                    if k % 2 == 0 {
                        self.loot(v3(cx, m(1, 0), cz), key);
                    }
                    k += 1;
                } else if roll < 8 {
                    // a mannequin, standing or knocked over
                    if rng.chance(50) {
                        self.decor(
                            Mesh::Cyl,
                            v3(cx, m(0, 9), cz),
                            v3(m(0, 4), m(1, 8), m(0, 4)),
                            0,
                            "cream",
                            0,
                            true,
                        );
                        self.decor(
                            Mesh::Sphere,
                            v3(cx, m(2, 0), cz),
                            v3(m(0, 3), m(0, 3), m(0, 3)),
                            0,
                            "cream",
                            0,
                            true,
                        );
                    } else {
                        self.decor(
                            Mesh::Cyl,
                            v3(cx, m(0, 2), cz),
                            v3(m(1, 8), m(0, 4), m(0, 4)),
                            rng.range(0, deg(180)),
                            "cream",
                            0,
                            true,
                        );
                    }
                }
                self.node(cx + int(3), 0, cz + int(3));
                z += 10;
            }
            x += 10;
        }
        self.zone(key, x0, z0, x1, z1, true);
    }

    // ---- outside ----------------------------------------------------------------

    fn main_street(&mut self) {
        // the corridor from the concourse out to the street
        self.patch(mm(HALL), int(80), int(40), int(96), FLOOR_Y, "tile");
        // the stores beside it give its walls up to their roofs
        for z in [int(80), int(96)] {
            self.wall_z(z, int(33), int(40), 0, int(7), &[], 0, "stucco", Look::Plain);
            self.wall_z(
                z,
                mm(HALL),
                int(33),
                mm(STORE_H) + ROOF_T,
                int(7),
                &[],
                0,
                "stucco",
                Look::Plain,
            );
        }
        self.wall_x(
            mm(HALL),
            int(80),
            int(96),
            0,
            mm(STORE_H),
            &[(int(82), int(94))],
            DOOR_H,
            "stucco",
            Look::Plain,
        );
        self.solid(
            v3(mm(HALL), int(7), int(80)),
            v3(int(40), int(7) + m(0, 6), int(96)),
            "roof",
            Look::Flat,
        );
        for x in [16, 26, 36] {
            self.node(mm(x), 0, int(88));
        }
        self.zone("sidehall", HALL, 80, 40, 96, true);
        // brick buildings on both sides of a pedestrian street; the west row
        // leaves a gap where the corridor comes out
        self.patch(int(58), int(50), int(82), int(180), ONE / 40, "brick_dark");
        let west = [
            (50, 78, "brick", 9),
            (98, 128, "wallA", 8),
            (132, 180, "wallC", 10),
        ];
        let east = [
            (50, 90, "wallE", 9),
            (94, 130, "brick", 11),
            (134, 180, "wallF", 14),
        ];
        for (row, x0, x1) in [(&west, 40, 58), (&east, 82, 100)] {
            // the west row stands a little off the department store beside it
            let off = if x0 == 40 { m(0, 6) } else { 0 };
            for (z0, z1, c, h) in row.iter() {
                self.solid(
                    v3(mm(x0) + off, 0, mm(*z0)),
                    v3(mm(x1), mm(*h), mm(*z1)),
                    c,
                    Look::Facade,
                );
                self.solid(
                    v3(mm(x0) + off - m(0, 3), mm(*h), mm(*z0) - m(0, 3)),
                    v3(mm(x1) + m(0, 3), mm(*h) + m(0, 6), mm(*z1) + m(0, 3)),
                    "stucco_trim",
                    Look::Plain,
                );
                // an awning over the street side
                let street_x = if x0 == 40 {
                    mm(x1) + int(1)
                } else {
                    mm(x0) - int(1)
                };
                self.decor(
                    Mesh::Wedge,
                    v3(street_x, m(3, 4), (mm(*z0) + mm(*z1)) / 2),
                    v3(int(2), m(0, 6), mm(*z1 - *z0) - int(4)),
                    if x0 == 40 { deg(90) } else { -deg(90) },
                    "teal",
                    0,
                    true,
                );
            }
        }
        // the cinema's marquee
        self.solid(
            v3(int(80), int(6), int(138)),
            v3(int(82), int(9), int(176)),
            "sign_c",
            Look::Glow,
        );
        // lamps, planters and a few loot spots on the street
        for z in (56..180).step_by(16) {
            for x in [60, 80] {
                self.lamp(mm(x), mm(z), int(5));
            }
            self.node(int(70), 0, mm(z));
        }
        self.container(int(66), int(70), 0, "wood", "mainstreet");
        self.container(int(74), int(118), 0, "wood", "mainstreet");
        self.container(int(64), int(160), 0, "metal", "mainstreet");
        self.zone("mainstreet", 40, 50, 100, 180, false);
    }

    fn lamp(&mut self, x: i32, z: i32, h: i32) {
        self.solid(
            v3(x - m(0, 2), 0, z - m(0, 2)),
            v3(x + m(0, 2), h, z + m(0, 2)),
            "pole",
            Look::Plain,
        );
        self.decor(
            Mesh::Sphere,
            v3(x, h + m(0, 3), z),
            v3(m(0, 9), m(0, 6), m(0, 9)),
            0,
            "lamp",
            ONE / 2,
            true,
        );
    }

    /// The parking lots: paint, light poles, dead cars, tree islands, carts.
    fn lots(&mut self, rng: &mut Rng) {
        // the whole ground is asphalt; grass verges round the edge
        self.patch(
            mm(LO_X) - int(200),
            mm(LO_Z) - int(200),
            mm(HI_X) + int(200),
            mm(HI_Z) + int(200),
            0,
            "asphalt",
        );
        for (x0, z0, x1, z1) in [
            (LO_X - 200, LO_Z - 200, HI_X + 200, LO_Z + 14),
            (LO_X - 200, HI_Z - 14, HI_X + 200, HI_Z + 200),
            (LO_X - 200, LO_Z, LO_X + 14, HI_Z),
            (HI_X - 14, LO_Z, HI_X + 200, HI_Z),
        ] {
            self.patch(mm(x0), mm(z0), mm(x1), mm(z1), ONE / 100, "grass");
        }
        // the ring road, a darker band
        for (x0, z0, x1, z1) in [
            (-240, -260, 240, -248),
            (-240, 248, 240, 260),
            (-240, -260, -228, 260),
            (228, -260, 240, 260),
        ] {
            self.patch(mm(x0), mm(z0), mm(x1), mm(z1), ONE / 100, "asphalt_dark");
        }
        let inside_building = |x: i32, z: i32, pad: i32| -> bool {
            let boxes = [
                (-40, -170, 40, -120),
                (-40, 120, 40, 170),
                (-110, -40, -60, 40),
                (-33, -120, 33, 120),
                (-60, -16, -24, 16),
                (24, -16, 64, 16),
                (9, 80, 40, 96),
                (40, 50, 100, 180),
            ];
            boxes
                .iter()
                .any(|b| x > b.0 - pad && x < b.2 + pad && z > b.1 - pad && z < b.3 + pad)
        };
        // stall rows every 18 m, lines across them
        let mut z = LO_Z + 30;
        while z < HI_Z - 30 {
            let mut x = LO_X + 30;
            while x < HI_X - 30 {
                let by_door = self
                    .map
                    .entrances
                    .iter()
                    .any(|e| (e.door.x - mm(x)).abs() < int(26) && (e.door.z - mm(z)).abs() < int(26));
                if !inside_building(x, z, 10) && !by_door {
                    for k in 0..4 {
                        let lx = mm(x) + k * int(3);
                        self.decor(
                            Mesh::Ground,
                            v3(lx, ONE / 30, mm(z)),
                            v3(cm(15), ONE, int(5)),
                            0,
                            "line",
                            0,
                            false,
                        );
                    }
                    if rng.chance(22) {
                        let cx = mm(x) + rng.range(0, 2) * int(3) + int(1) + ONE / 2;
                        self.car(cx, mm(z), rng);
                    } else if rng.chance(6) {
                        self.container(mm(x) + int(2), mm(z) + int(4), 0, "metal", "parking");
                    }
                }
                x += 14;
            }
            z += 18;
        }
        // light poles on a 36 m grid; trees on islands between rows
        let mut z = LO_Z + 24;
        while z < HI_Z - 20 {
            let mut x = LO_X + 24;
            while x < HI_X - 20 {
                if !inside_building(x, z, 8) {
                    self.lamp(mm(x), mm(z) + int(9), int(9));
                    if rng.chance(40) {
                        let tx = mm(x) + int(14);
                        let tz = mm(z) + int(9);
                        self.solid(
                            v3(tx - m(0, 3), 0, tz - m(0, 3)),
                            v3(tx + m(0, 3), int(3), tz + m(0, 3)),
                            "trunk",
                            Look::Plain,
                        );
                        let s = int(4) + rng.range(0, int(2));
                        self.decor(
                            Mesh::Blob,
                            v3(tx, int(4), tz),
                            v3(s, s * 3 / 4, s),
                            0,
                            if rng.chance(50) { "plant" } else { "plant_dark" },
                            0,
                            true,
                        );
                    }
                }
                x += 36;
            }
            z += 36;
        }
        // the parking grid of nodes
        let mut z = LO_Z + 20;
        while z < HI_Z - 20 {
            let mut x = LO_X + 20;
            while x < HI_X - 20 {
                if !inside_building(x, z, 4) {
                    self.node(mm(x), 0, mm(z));
                }
                x += 24;
            }
            z += 24;
        }
        // trees all round the edge, and the hidden wall beyond them
        let mut t = LO_X;
        while t < HI_X {
            for z in [LO_Z + 6, HI_Z - 6] {
                let s = int(5) + rng.range(0, int(3));
                self.decor(
                    Mesh::Blob,
                    v3(mm(t), int(5), mm(z)),
                    v3(s, s, s),
                    0,
                    "plant_dark",
                    0,
                    true,
                );
            }
            t += 9;
        }
        let mut t = LO_Z;
        while t < HI_Z {
            for x in [LO_X + 6, HI_X - 6] {
                let s = int(5) + rng.range(0, int(3));
                self.decor(
                    Mesh::Blob,
                    v3(mm(x), int(5), mm(t)),
                    v3(s, s, s),
                    0,
                    "plant_dark",
                    0,
                    true,
                );
            }
            t += 9;
        }
        // the interstate to the north, beyond the trees
        self.decor(
            Mesh::Cube,
            v3(0, int(9), mm(LO_Z) - int(60)),
            v3(int(900), int(3), int(24)),
            0,
            "concrete_far",
            0,
            true,
        );
        for x in (-400..=400).step_by(50) {
            self.decor(
                Mesh::Cube,
                v3(mm(x), int(4), mm(LO_Z) - int(60)),
                v3(int(3), int(8), int(6)),
                0,
                "concrete_far",
                0,
                true,
            );
        }
        self.zone("parking", LO_X, LO_Z, HI_X, HI_Z, false);
    }

    fn car(&mut self, x: i32, z: i32, rng: &mut Rng) {
        let colours = ["car_a", "car_b", "car_c", "car_d", "car_e", "rust"];
        let c = colours[rng.range(0, 5) as usize];
        let (w, l) = (m(1, 8), m(4, 4));
        self.solid(
            v3(x - w / 2, m(0, 3), z - l / 2),
            v3(x + w / 2, m(1, 1), z + l / 2),
            c,
            Look::Plain,
        );
        self.decor(
            Mesh::Cube,
            v3(x, m(1, 4), z + m(0, 3)),
            v3(w - m(0, 2), m(0, 6), m(2, 2)),
            0,
            "glass",
            0,
            true,
        );
        for (dx, dz) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
            self.decor(
                Mesh::Cube,
                v3(x + dx * (w / 2), cm(35), z + dz * m(1, 4)),
                v3(cm(25), m(0, 7), m(0, 7)),
                deg(90),
                "tyre",
                0,
                true,
            );
        }
    }

    /// Walls round the outside of the mall that the anchors and courts do
    /// not already give, the entrance towers, and the roof edge.
    fn shell(&mut self) {
        // the store rows' outer ends where they meet the anchors are walls already;
        // close the gaps between the rows and the courts
        for side in [-1, 1] {
            let x = mm(STORE_BACK) * side;
            let _ = x;
        }
        // entrance towers: tall glass blocks flanking each outside door
        let towers = [(0, -170, 'z'), (0, 170, 'z'), (-110, 0, 'x'), (64, 0, 'x')];
        for (x, z, axis) in towers {
            for s in [-1, 1] {
                let (cx, cz) = if axis == 'z' {
                    (mm(x) + s * int(9), mm(z))
                } else {
                    (mm(x), mm(z) + s * int(8))
                };
                self.solid(
                    v3(cx - int(2), 0, cz - int(2)),
                    v3(cx + int(2), int(15), cz + int(2)),
                    "stucco_trim",
                    Look::Facade,
                );
                self.solid(
                    v3(cx - m(2, 3), int(15), cz - m(2, 3)),
                    v3(cx + m(2, 3), m(15, 8), cz + m(2, 3)),
                    "teal",
                    Look::Plain,
                );
            }
        }
    }

    fn entrances(&mut self) {
        // the doorway, the van's spot far out in the lot and the way the van's
        // back faces (yaw: 0 faces −z): toward the door
        let list = [
            ("north", v3(0, 0, -int(172)), v3(int(30), 0, -int(276))),
            ("south", v3(0, 0, int(172)), v3(-int(36), 0, int(276))),
            ("west", v3(-int(112), 0, 0), v3(-int(240), 0, -int(40))),
            ("east", v3(int(66), 0, 0), v3(int(200), 0, int(36))),
            ("southeast", v3(int(70), 0, int(48)), v3(int(150), 0, int(-110))),
        ];
        for (key, door, van) in list {
            let d = door.sub(van);
            let yaw = crate::geom::yaw_pitch_of(d).0;
            self.map.entrances.push(Entrance {
                key: key.to_string(),
                door,
                van,
                van_yaw: yaw,
            });
            self.node(door.x, 0, door.z);
        }
    }
}

/// Builds the whole map.
pub fn build(pal: &Palette) -> Map {
    let map = Map::new(v3(mm(LO_X), -int(10), mm(LO_Z)), v3(mm(HI_X), int(60), mm(HI_Z)));
    let mut b = B {
        map,
        pal,
        rng: Rng::new(2026),
    };
    b.zone("mall", -110, -170, 64, 170, true);
    b.concourse();
    // west side stores, north to south
    use Kind::*;
    let west: [(&str, i32, i32, Kind, &str, &str); 12] = [
        ("security", -120, -104, Security, "wallE", "sign_b"),
        ("shoes", -104, -88, Shelves, "wallA", "sign_e"),
        ("books", -88, -72, Books, "wallD", "sign_c"),
        ("toys", -72, -56, Shelves, "wallB", "sign_a"),
        ("jewelry", -56, -40, Jewelry, "cream", "sign_c"),
        ("apparel", -40, -16, Racks, "wallF", "sign_d"),
        ("candles", 16, 32, Shelves, "wallB", "sign_a"),
        ("games", 32, 48, Shelves, "wallC", "sign_b"),
        ("sports", 48, 64, Racks, "wallE", "sign_e"),
        ("hottopic", 64, 80, HotTopic, "goth", "sign_d"),
        ("phones", 80, 96, Shelves, "wallA", "sign_b"),
        ("electronics", 96, 120, Electronics, "wallD", "sign_f"),
    ];
    let east: [(&str, i32, i32, Kind, &str, &str); 11] = [
        ("pretzels", -120, -104, Food, "wallD", "sign_c"),
        ("beauty", -104, -88, Shelves, "wallF", "sign_a"),
        ("denim", -88, -72, Racks, "wallB", "sign_f"),
        ("arcade", -72, -56, Arcade, "goth", "sign_a"),
        ("music", -56, -40, Shelves, "wallC", "sign_d"),
        ("outdoor", -40, -16, Racks, "sage", "sign_f"),
        ("tea", 16, 32, Food, "wallA", "sign_e"),
        ("eyewear", 32, 48, Shelves, "cream", "sign_b"),
        ("kids", 48, 64, Racks, "wallD", "sign_c"),
        ("cards", 64, 80, Shelves, "wallB", "sign_e"),
        ("furniture", 96, 120, Racks, "wallE", "sign_d"),
    ];
    for (key, z0, z1, kind, wall, sign) in west {
        b.store(key, -1, z0, z1, kind, wall, sign);
    }
    for (key, z0, z1, kind, wall, sign) in east {
        b.store(key, 1, z0, z1, kind, wall, sign);
    }
    b.centre_court();
    b.food_court();
    b.west_wing();
    b.anchor(
        "northanchor",
        -40,
        -170,
        40,
        -120,
        "sign_e",
        &[('n', -6, 6), ('s', -7, 7)],
        's',
    );
    b.anchor(
        "southanchor",
        -40,
        120,
        40,
        170,
        "sign_b",
        &[('s', -6, 6), ('n', -7, 7)],
        'n',
    );
    b.anchor(
        "westanchor",
        -110,
        -40,
        -60,
        40,
        "sign_d",
        &[('w', -5, 5), ('e', -6, 6)],
        'e',
    );
    b.mission_spot("hive", v3(-int(85), 0, 0), "westanchor");
    b.main_street();
    b.shell();
    b.entrances();
    let mut rng = std::mem::replace(&mut b.rng, Rng::new(0));
    b.lots(&mut rng);
    b.map.drop_hidden_solids();
    b.map.link_nodes(int(26));
    b.map
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::content::tests::content;

    pub fn map() -> Map {
        build(&content().palette)
    }

    #[test]
    fn the_mall_has_five_entrances_and_every_mission_spot() {
        let m = map();
        assert_eq!(m.entrances.len(), 5);
        for key in [
            "hottopic",
            "security",
            "electronics",
            "jewelry",
            "generator",
            "hive",
        ] {
            assert!(m.spots.iter().any(|s| s.key == key), "{key}");
        }
        assert!(m.spots.iter().filter(|s| s.kind == SpotKind::Loot).count() > 60);
        assert!(m.solids.len() > 500, "{}", m.solids.len());
    }

    #[test]
    fn a_van_can_walk_to_its_entrance_and_on_to_the_centre_court() {
        let m = map();
        for e in &m.entrances {
            let a = m.nearest_node(e.van).expect("a node by the van");
            let b = m.nearest_node(e.door).expect("a node at the door");
            let c = m.nearest_node(v3(0, 0, int(12))).unwrap();
            assert!(!m.path(a, b, 4000).is_empty(), "{}: van to door", e.key);
            assert!(!m.path(b, c, 4000).is_empty(), "{}: door to the court", e.key);
            // the van's spot is far out: the mall is a walk away
            assert!(e.van.dist(e.door) > int(90), "{}", e.key);
        }
    }

    /// Pairs of visible faces in one plane, facing the same way, overlapping,
    /// in different colours: what flickers as the camera moves.
    pub fn flickering(m: &Map) -> Vec<String> {
        use crate::world::Look;
        let shown: Vec<_> = m.solids.iter().filter(|s| s.look != Look::Hidden).collect();
        let mut out = Vec::new();
        let lo = |b: &crate::world::Aabb, a: usize| [b.min.x, b.min.y, b.min.z][a];
        let hi = |b: &crate::world::Aabb, a: usize| [b.max.x, b.max.y, b.max.z][a];
        for i in 0..shown.len() {
            for j in i + 1..shown.len() {
                let (a, b) = (&shown[i].b, &shown[j].b);
                if shown[i].rgb == shown[j].rgb {
                    continue;
                }
                for axis in 0..3 {
                    let (u, v) = ((axis + 1) % 3, (axis + 2) % 3);
                    let overlap = |k: usize| hi(a, k).min(hi(b, k)) - lo(a, k).max(lo(b, k));
                    if overlap(u) <= 0 || overlap(v) <= 0 {
                        continue;
                    }
                    // big enough to see, and not the undersides on the ground
                    let area = (overlap(u) as i64 * overlap(v) as i64) >> 16;
                    let on_ground = axis == 1 && lo(a, 1) <= 0 && lo(b, 1) <= 0 && hi(a, 1) != hi(b, 1);
                    if area < (ONE / 4) as i64 || on_ground {
                        continue;
                    }
                    if lo(a, axis) == lo(b, axis) || hi(a, axis) == hi(b, axis) {
                        let f = |b: &crate::world::Aabb| {
                            format!(
                                "[{},{},{}..{},{},{}]",
                                (b.min.x * 10) >> 16,
                                (b.min.y * 10) >> 16,
                                (b.min.z * 10) >> 16,
                                (b.max.x * 10) >> 16,
                                (b.max.y * 10) >> 16,
                                (b.max.z * 10) >> 16
                            )
                        };
                        out.push(format!("axis {axis} {} {}", f(a), f(b)));
                    }
                }
            }
        }
        out
    }

    #[test]
    fn no_two_walls_share_a_face_in_different_colours() {
        let m = map();
        let f = flickering(&m);
        assert!(f.is_empty(), "{} faces fight: {:?}", f.len(), f);
    }

    #[test]
    fn the_balconies_are_reached_by_the_escalators() {
        let m = map();
        let floor = m.nearest_node(v3(0, 0, int(40))).unwrap();
        let up = m.nearest_node(v3(int(7), BALC_Y, int(40))).unwrap();
        assert!(m.nodes[up].y == BALC_Y);
        assert!(!m.path(floor, up, 4000).is_empty());
    }
}
