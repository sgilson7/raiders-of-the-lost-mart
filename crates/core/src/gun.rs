//! Guns built from parts, like Banjo-Kazooie: Nuts & Bolts' vehicles: a
//! frame, barrels (the pieces that do the damage), a mount that sets out how
//! many barrels and where, a payload every shot carries, a sight and a grip.
//! Composing them gives the gun's numbers, its model as a list of boxes and
//! tubes, and the muzzles: every shot leaves from the end of a barrel the
//! player can see, so four double-barrelled shotguns in a square fire eight
//! loads of pellets from eight muzzles.
//!
//! Gun space: x right, y up, forward is −z (as a heading of 0 faces); the
//! origin is the hand on the grip, at the bottom of the frame.

use crate::content::{Cat, Content, Effect, Part, Rgb, ShotKind};
use crate::fx::{self, ONE};
use crate::geom::{v3, V3};
use crate::mesh::Mesh;

/// One part from each category, as indices into `Content::parts`, in
/// `Cat::ALL` order.
pub type Build = [usize; 6];

pub fn basic_build(c: &Content) -> Build {
    let mut b = [0; 6];
    for cat in Cat::ALL {
        b[cat.index()] = c.basic(cat);
    }
    b
}

/// A build is sound when each slot holds a part of that slot's category.
pub fn valid(c: &Content, b: &Build) -> bool {
    Cat::ALL
        .iter()
        .all(|cat| c.parts.get(b[cat.index()]).is_some_and(|p| p.cat == *cat))
}

/// One shape of the gun's model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub mesh: Mesh,
    pub at: V3,
    pub size: V3,
    /// a tube lies along z (turned a quarter about x)
    pub along_z: bool,
    pub rgb: Rgb,
    pub glow: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gun {
    pub auto: bool,
    /// ticks between trigger pulls (a gatling's between barrels)
    pub cooldown: i32,
    /// volleys in a magazine
    pub mag: i32,
    pub reload: i32,
    pub kind: ShotKind,
    /// per projectile
    pub damage: i32,
    pub pellets: i32,
    /// metres a second, Q16
    pub speed: i32,
    /// cone half-angles (angle units) from the hip and aimed
    pub hip_spread: i32,
    pub aim_spread: i32,
    /// pitch kick per volley, angle units
    pub recoil: i32,
    /// blast radius, Q16 metres (0: none)
    pub splash: i32,
    pub pierce: i32,
    pub effect: Effect,
    pub power: i32,
    pub move_pct: i32,
    /// field of view while aiming, as a share of the normal one
    pub zoom_pct: i32,
    pub scanner: bool,
    pub gatling: bool,
    pub muzzles: Vec<V3>,
    pub model: Vec<Piece>,
    /// where the sight's eye line sits, for the aimed camera
    pub sight_at: V3,
}

/// Centimetres to Q16 metres.
fn cm(n: i32) -> i32 {
    n * ONE / 100
}

/// Tenths of a degree to angle units.
fn tenths(n: i32) -> i32 {
    (n as i64 * fx::TURN as i64 / 3600) as i32
}

fn piece(mesh: Mesh, at: V3, size: V3, rgb: Rgb) -> Piece {
    Piece {
        mesh,
        at,
        size,
        along_z: false,
        rgb,
        glow: 0,
    }
}

fn tube(at: V3, width: i32, len: i32, rgb: Rgb) -> Piece {
    Piece {
        mesh: Mesh::Cyl,
        at,
        size: v3(width, len, width),
        along_z: true,
        rgb,
        glow: 0,
    }
}

pub fn compose(c: &Content, b: &Build) -> Gun {
    let p = |cat: Cat| -> &Part { &c.parts[b[cat.index()]] };
    let (frame, barrel, mount, payload, sight, grip) = (
        p(Cat::Frame),
        p(Cat::Barrel),
        p(Cat::Mount),
        p(Cat::Payload),
        p(Cat::Sight),
        p(Cat::Grip),
    );

    // ---- the muzzles ----------------------------------------------------------
    let axis_y = cm(frame.height) * 65 / 100;
    let front = -cm(frame.len);
    let spacing_pct = (barrel.width * 100 / 8).max(100);
    let bore_dx = cm(barrel.width) / 4;
    let mut barrels: Vec<V3> = Vec::new();
    for &(sx, sy) in &mount.slots {
        barrels.push(v3(
            cm(sx) * spacing_pct / 100,
            axis_y + cm(sy) * spacing_pct / 100,
            front,
        ));
    }
    let tip = -cm(frame.len + barrel.len);
    let mut muzzles = Vec::new();
    for at in &barrels {
        for k in 0..barrel.bores {
            let dx = if barrel.bores > 1 {
                bore_dx * (2 * k - (barrel.bores - 1))
            } else {
                0
            };
            muzzles.push(v3(at.x + dx, at.y, tip));
        }
    }
    let n = muzzles.len() as i32;
    let per_volley = if mount.gatling { barrel.bores } else { n };

    // ---- the numbers ------------------------------------------------------------
    // more barrels a volley drain the magazine faster, but not one for one
    let mag = if mount.gatling {
        frame.mag * 3 / 2
    } else if per_volley == 1 {
        frame.mag
    } else {
        (frame.mag * 2 / (per_volley + 1)).max(2)
    };
    let base_cd = frame.cooldown.max(barrel.min_cd);
    let cooldown = if mount.gatling {
        (base_cd / 2).max(2)
    } else {
        base_cd
    };
    let damage = barrel.damage * frame.dmg_pct / 100 * payload.dmg_pct / 100;
    let spread = (frame.spread + barrel.spread) * grip.spread_pct / 100;
    let recoil_tenths = frame.recoil * grip.recoil_pct / 100 * (4 + per_volley - 1) / 4;
    let splash = cm(barrel.splash * 10 + payload.splash * 10);

    // ---- the model --------------------------------------------------------------
    let mut model = Vec::new();
    let fh = cm(frame.height);
    let fl = cm(frame.len);
    let fw = cm(6) + cm(frame.height) / 4;
    model.push(piece(
        Mesh::Cube,
        v3(0, fh / 2, -fl / 2),
        v3(fw, fh, fl),
        frame.colour,
    ));
    model.push(piece(
        Mesh::Cube,
        v3(0, fh + cm(1), -fl / 2),
        v3(fw * 7 / 10, cm(2), fl * 8 / 10),
        frame.trim,
    ));
    // the frame's own furniture: a magazine of its shape
    let mag_piece = match frame.id.as_str() {
        "ak" => piece(
            Mesh::Wedge,
            v3(0, -cm(8), -fl * 6 / 10),
            v3(cm(4), cm(16), cm(7)),
            frame.trim,
        ),
        "heavy" => piece(
            Mesh::Cube,
            v3(cm(3), -cm(6), -fl / 2),
            v3(cm(10), cm(12), cm(12)),
            frame.trim,
        ),
        "pump" => tube(v3(0, cm(2), -fl - cm(2)), cm(4), cm(20), frame.trim),
        "smg" => piece(
            Mesh::Cube,
            v3(0, -cm(9), -fl * 6 / 10),
            v3(cm(3), cm(18), cm(4)),
            frame.trim,
        ),
        _ => piece(
            Mesh::Cube,
            v3(0, -cm(5), -cm(3)),
            v3(cm(3), cm(8), cm(4)),
            frame.trim,
        ),
    };
    model.push(mag_piece);
    // the mount: a plate the barrels sit in
    let (mut lo, mut hi) = (v3(i32::MAX, i32::MAX, 0), v3(i32::MIN, i32::MIN, 0));
    for at in &barrels {
        lo = v3(lo.x.min(at.x), lo.y.min(at.y), 0);
        hi = v3(hi.x.max(at.x), hi.y.max(at.y), 0);
    }
    let bw = cm(barrel.width);
    model.push(piece(
        Mesh::Cube,
        v3((lo.x + hi.x) / 2, (lo.y + hi.y) / 2, front - cm(2)),
        v3(hi.x - lo.x + bw + cm(3), hi.y - lo.y + bw + cm(3), cm(4)),
        mount.colour,
    ));
    if mount.gatling {
        model.push(tube(
            v3(0, axis_y, front - cm(barrel.len) / 2),
            cm(3),
            cm(barrel.len),
            mount.colour,
        ));
    }
    // the barrels, each with its bores
    let bl = cm(barrel.len);
    for at in &barrels {
        let mid = v3(at.x, at.y, front - bl / 2);
        match barrel.kind {
            ShotKind::Rocket => {
                model.push(tube(mid, bw, bl, barrel.colour));
                model.push(tube(
                    v3(at.x, at.y, tip + cm(4)),
                    bw + cm(2),
                    cm(6),
                    payload.colour,
                ));
            }
            ShotKind::Grenade => {
                model.push(tube(mid, bw, bl, barrel.colour));
                model.push(tube(v3(at.x, at.y, front - cm(3)), bw + cm(3), cm(6), frame.trim));
            }
            ShotKind::Bullet => {
                for k in 0..barrel.bores {
                    let dx = if barrel.bores > 1 {
                        bore_dx * (2 * k - (barrel.bores - 1))
                    } else {
                        0
                    };
                    let w = if barrel.bores > 1 { bw / 2 } else { bw };
                    model.push(tube(v3(at.x + dx, at.y, mid.z), w, bl, barrel.colour));
                }
                if barrel.id == "rifle" {
                    model.push(tube(v3(at.x, at.y, tip + cm(3)), bw + cm(2), cm(6), frame.colour));
                }
            }
        }
    }
    // the payload shows as a canister on the side
    model.push(tube(
        v3(fw / 2 + cm(2), fh / 2, -fl / 3),
        cm(4),
        cm(9),
        payload.colour,
    ));
    // the sight on top
    let top = fh + cm(2);
    let sight_at = match sight.id.as_str() {
        "scope" => {
            model.push(tube(v3(0, top + cm(4), -fl / 2), cm(5), cm(26), sight.colour));
            model.push(tube(
                v3(0, top + cm(4), -fl / 2 - cm(13)),
                cm(7),
                cm(3),
                sight.colour,
            ));
            v3(0, top + cm(4), 0)
        }
        "dot" => {
            model.push(piece(
                Mesh::Cube,
                v3(0, top + cm(3), -fl / 3),
                v3(cm(4), cm(5), cm(7)),
                sight.colour,
            ));
            v3(0, top + cm(4), 0)
        }
        "laser" => {
            model.push(piece(
                Mesh::Cube,
                v3(fw / 2 + cm(2), fh / 3, -fl + cm(6)),
                v3(cm(3), cm(3), cm(10)),
                sight.colour,
            ));
            model.push(piece(
                Mesh::Cube,
                v3(0, top + cm(2), -fl + cm(3)),
                v3(cm(1), cm(4), cm(1)),
                crate::content::hex("#3B2340"),
            ));
            v3(0, top + cm(2), 0)
        }
        "scanner" => {
            model.push(piece(
                Mesh::Cube,
                v3(0, top + cm(3), -fl / 3),
                v3(cm(7), cm(6), cm(8)),
                sight.trim,
            ));
            let mut screen = piece(
                Mesh::Cube,
                v3(0, top + cm(4), -fl / 3 + cm(4)),
                v3(cm(5), cm(4), cm(1)),
                sight.colour,
            );
            screen.glow = ONE / 2;
            model.push(screen);
            v3(0, top + cm(4), 0)
        }
        _ => {
            model.push(piece(
                Mesh::Cube,
                v3(0, top + cm(1), -fl + cm(2)),
                v3(cm(1), cm(3), cm(1)),
                sight.colour,
            ));
            model.push(piece(
                Mesh::Cube,
                v3(0, top + cm(1), -cm(2)),
                v3(cm(3), cm(3), cm(1)),
                sight.colour,
            ));
            v3(0, top + cm(2), 0)
        }
    };
    // the grip below the hand, and a stock or a foregrip
    model.push(piece(
        Mesh::Cube,
        v3(0, -cm(6), cm(1)),
        v3(cm(3), cm(12), cm(4)),
        if grip.id == "wood" {
            grip.colour
        } else {
            frame.trim
        },
    ));
    match grip.id.as_str() {
        "fore" => model.push(piece(
            Mesh::Cube,
            v3(0, -cm(6), -fl + cm(6)),
            v3(cm(3), cm(11), cm(3)),
            grip.colour,
        )),
        "stock" => model.push(piece(
            Mesh::Wedge,
            v3(0, fh / 2 - cm(2), cm(14)),
            v3(cm(4), fh + cm(4), cm(24)),
            grip.colour,
        )),
        "skeleton" => {
            model.push(tube(v3(0, fh - cm(2), cm(12)), cm(2), cm(22), grip.colour));
            model.push(tube(v3(0, cm(1), cm(12)), cm(2), cm(22), grip.colour));
            model.push(piece(
                Mesh::Cube,
                v3(0, fh / 2, cm(23)),
                v3(cm(3), fh, cm(2)),
                grip.colour,
            ));
        }
        "brace" => {
            model.push(piece(
                Mesh::Cube,
                v3(0, fh / 2, cm(14)),
                v3(cm(6), fh + cm(4), cm(24)),
                grip.colour,
            ));
            model.push(piece(
                Mesh::Cube,
                v3(0, fh / 2, cm(27)),
                v3(cm(7), fh + cm(8), cm(3)),
                crate::content::hex("#E47A7E"),
            ));
        }
        _ => {}
    }

    Gun {
        auto: frame.auto,
        cooldown,
        mag,
        reload: frame.reload + 12 * (per_volley - 1).min(10),
        kind: barrel.kind,
        damage,
        pellets: barrel.pellets,
        speed: barrel.speed * ONE,
        hip_spread: tenths(spread * sight.hip_spread_pct / 100),
        aim_spread: tenths(spread * sight.aim_spread_pct / 100),
        recoil: tenths(recoil_tenths),
        splash,
        pierce: barrel.pierce,
        effect: payload.effect,
        power: payload.power,
        move_pct: frame.move_pct * grip.move_pct / 100,
        zoom_pct: sight.zoom_pct,
        scanner: sight.scanner,
        gatling: mount.gatling,
        muzzles,
        model,
        sight_at,
    }
}

/// Damage a volley does if every projectile lands, for the workshop.
pub fn volley_damage(g: &Gun) -> i32 {
    let n = if g.gatling { 1 } else { g.muzzles.len() as i32 };
    g.damage * g.pellets * n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::content;

    fn build(c: &Content, ids: [&str; 6]) -> Build {
        let mut b = [0; 6];
        for (i, id) in ids.iter().enumerate() {
            b[i] = c.part(id).unwrap();
        }
        assert!(valid(c, &b));
        b
    }

    #[test]
    fn four_double_barrels_in_a_square_make_eight_muzzles_at_the_barrels_ends() {
        let c = content();
        let g = compose(
            &c,
            &build(&c, ["pistol", "double", "square", "slug", "dot", "wood"]),
        );
        assert_eq!(g.muzzles.len(), 8);
        let pistol = &c.parts[c.part("pistol").unwrap()];
        let double = &c.parts[c.part("double").unwrap()];
        for m in &g.muzzles {
            assert_eq!(
                m.z,
                -cm(pistol.len + double.len),
                "every muzzle at the barrels' tip"
            );
        }
        // four distinct heights-and-sides pairs, two bores each
        let mut xs: Vec<i32> = g.muzzles.iter().map(|m| m.x).collect();
        xs.sort();
        xs.dedup();
        assert_eq!(xs.len(), 4);
        assert_eq!(g.pellets, 6);
        assert_eq!(volley_damage(&g), 6 * 6 * 8);
    }

    #[test]
    fn a_rocket_on_an_ak_full_of_goo_does_no_damage_but_carries_goo() {
        let c = content();
        let g = compose(&c, &build(&c, ["ak", "rocket", "single", "goo", "iron", "stock"]));
        assert_eq!(g.kind, ShotKind::Rocket);
        assert_eq!(g.damage, 0);
        assert_eq!(g.effect, Effect::Goo);
        assert!(g.cooldown >= 34, "the rocket tube slows the AK's rate");
        assert!(g.splash > 0);
    }

    #[test]
    fn the_basic_build_is_a_working_pistol_and_more_barrels_drain_the_magazine_faster() {
        let c = content();
        let basic = compose(&c, &basic_build(&c));
        assert_eq!(basic.muzzles.len(), 1);
        assert!(basic.damage > 0 && basic.mag >= 10);
        let square = compose(
            &c,
            &build(&c, ["pistol", "short", "square", "slug", "iron", "wood"]),
        );
        assert!(square.mag < basic.mag);
        assert!(square.mag >= 2);
        assert!(square.recoil > basic.recoil);
    }

    #[test]
    fn a_gatling_fires_one_barrel_at_a_time_quicker_than_its_frame() {
        let c = content();
        let g = compose(
            &c,
            &build(&c, ["smg", "short", "gatling", "slug", "iron", "wood"]),
        );
        assert_eq!(g.muzzles.len(), 6);
        assert!(g.gatling);
        assert!(g.cooldown < 5);
        assert_eq!(volley_damage(&g), g.damage);
    }
}
