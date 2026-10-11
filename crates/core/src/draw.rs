//! What the scene looks like: every raider, drone, gun, shot and the mall
//! itself as boxes, balls and tubes for the renderer (render.rs). Nothing
//! here changes the run; it reads it.

use crate::balance::{secs, TICKS_PER_SECOND};
use crate::content::{Content, Rgb};
use crate::content::{Palette, ShotKind};
use crate::drone::{zones, Drone, Kind, State, Zone};
use crate::fx::{self, deg, int, ONE, QUARTER};
use crate::geom::{facing, v3, Affine, V3};
use crate::gun::Gun;
use crate::mesh::Mesh;
use crate::raider::{gun_frame, Life, Raider};
use crate::render::{mix, shade, Batch, PASS_FACADE, PASS_GLOW, PASS_SOLID};
use crate::run::{Hive, Item, Pickup, Shot, ShotFrom, Van};
use crate::world::{Look, Map};

pub fn cm(n: i32) -> i32 {
    n * ONE / 100
}

/// A shape placed in a base frame: centred at `at`, sized `size`.
pub fn put(b: &mut Batch, mesh: Mesh, base: &Affine, at: V3, size: V3, rgb: Rgb, glow: i32, inked: bool) {
    let a = base.then(&Affine::translate(at)).then(&Affine::scale(size));
    if inked {
        b.inked(mesh, &a, rgb, glow);
    } else {
        b.push(mesh, &a, rgb, glow, PASS_SOLID);
    }
}

/// A box (or tube) from `a` to `b`, `w` thick.
pub fn segment(bt: &mut Batch, mesh: Mesh, a: V3, b: V3, w: i32, rgb: Rgb, glow: i32, inked: bool) {
    let d = b.sub(a);
    let len = d.len();
    if len == 0 {
        return;
    }
    let dir = d.norm();
    let mid = a.add(d.scale(ONE / 2));
    let base = Affine::translate(mid).then(&Affine::looking(dir));
    let size = v3(w, w, len);
    let a = if mesh == Mesh::Cyl {
        base.then(&Affine::rot_x(QUARTER))
            .then(&Affine::scale(v3(size.x, size.z, size.y)))
    } else {
        base.then(&Affine::scale(size))
    };
    if inked {
        bt.inked(mesh, &a, rgb, glow);
    } else {
        bt.push(mesh, &a, rgb, glow, PASS_SOLID);
    }
}

pub fn glow_ball(b: &mut Batch, at: V3, r: i32, rgb: Rgb, strength: i32) {
    let a = Affine::translate(at).then(&Affine::scale(v3(r, r, r)));
    b.glow(Mesh::Sphere, &a, rgb, strength);
}

// ---- the map ----------------------------------------------------------------------

pub fn map_scene(map: &Map, b: &mut Batch) {
    for s in &map.solids {
        let c = s.b.centre();
        let size = s.b.size();
        let a = Affine::translate(c).then(&Affine::scale(size));
        match s.look {
            Look::Hidden => {}
            Look::Flat => b.push(Mesh::Cube, &a, s.rgb, 0, PASS_SOLID),
            Look::Glow => b.inked(Mesh::Cube, &a, s.rgb, ONE * 7 / 10),
            Look::Plain => b.inked(Mesh::Cube, &a, s.rgb, 0),
            Look::Facade => {
                b.inked(Mesh::Cube, &a, s.rgb, 0);
                facades(b, s.b.min, s.b.max, s.rgb);
            }
        }
    }
    for d in &map.decor {
        let a = Affine::translate(d.at)
            .then(&Affine::rot_y(d.yaw))
            .then(&Affine::scale(d.size));
        if d.inked {
            b.inked(d.mesh, &a, d.rgb, d.glow);
        } else {
            b.push(d.mesh, &a, d.rgb, d.glow, PASS_SOLID);
        }
    }
}

/// Windows on a block's four sides, drawn by the shader's facade pass.
fn facades(b: &mut Batch, lo: V3, hi: V3, rgb: Rgb) {
    let h = hi.y - lo.y;
    let mid_y = (lo.y + hi.y) / 2;
    // far enough off the wall that the windows do not flicker against it
    let out = cm(7);
    let lit = mix(rgb, [ONE, ONE * 9 / 10, ONE * 6 / 10], ONE / 2);
    let faces = [
        (
            v3(hi.x + out, mid_y, (lo.z + hi.z) / 2),
            v3(ONE, 0, 0),
            v3(0, 0, hi.z - lo.z),
        ),
        (
            v3(lo.x - out, mid_y, (lo.z + hi.z) / 2),
            v3(-ONE, 0, 0),
            v3(0, 0, hi.z - lo.z),
        ),
        (
            v3((lo.x + hi.x) / 2, mid_y, hi.z + out),
            v3(0, 0, ONE),
            v3(hi.x - lo.x, 0, 0),
        ),
        (
            v3((lo.x + hi.x) / 2, mid_y, lo.z - out),
            v3(0, 0, -ONE),
            v3(hi.x - lo.x, 0, 0),
        ),
    ];
    for (at, normal, along) in faces {
        if along.len() < int(3) || h < int(3) {
            continue;
        }
        let a = Affine {
            c0: v3(0, h, 0),
            c1: normal,
            c2: along,
            t: at,
        };
        b.push(Mesh::Facade, &a, lit, ONE / 3, PASS_FACADE);
    }
}

// ---- raiders --------------------------------------------------------------------------

pub struct Look2 {
    pub jacket: Rgb,
    pub skin: Rgb,
    pub pants: Rgb,
    pub boots: Rgb,
    pub helmet: Rgb,
    pub pack: Rgb,
    pub visor: Rgb,
    /// what is worn that shows: the head piece, a vest, a coat's colour,
    /// and the melee weapon
    pub head: &'static str,
    pub vest: bool,
    pub coat: Option<Rgb>,
    pub melee: &'static str,
}

pub fn raider_look(pal: &Palette, i: usize, c: &Content, wear: &crate::kit::Wear) -> Look2 {
    let id = |slot: usize| c.gear[wear[slot]].id.as_str();
    let head = match id(0) {
        "helmet" => "helmet",
        "headset" => "headset",
        _ => "cap",
    };
    let coat = match id(3) {
        "poncho" | "trench" | "hazmat" => Some(crate::content::hex(&c.gear[wear[3]].colour)),
        _ => None,
    };
    let melee = match id(crate::content::SLOT_MELEE) {
        "sledge" => "sledge",
        "machete" => "machete",
        _ => "bat",
    };
    Look2 {
        head,
        vest: id(2) == "vest",
        coat,
        melee,
        jacket: pal.get(&format!("jacket_{}", i % 4)),
        skin: pal.get(&format!("skin_{}", i % 4)),
        pants: pal.get("pants"),
        boots: pal.get("boots"),
        helmet: pal.get("helmet"),
        pack: pal.get("pack"),
        visor: pal.get("visor"),
    }
}

pub fn raider(b: &mut Batch, r: &Raider, look: &Look2, gun: &Gun, tick: u32) {
    if matches!(r.life, Life::Extracted | Life::Gone | Life::Left) {
        return;
    }
    let downed = matches!(r.life, Life::Downed | Life::Dead);
    let mut base = Affine::translate(r.pos).then(&Affine::rot_y(r.yaw));
    if downed {
        // lying on their back, feet toward the camera
        base = base
            .then(&Affine::translate(v3(0, cm(25), 0)))
            .then(&Affine::rot_x(-QUARTER));
    }
    let swing = if r.moving && r.grounded {
        fx::sin(r.stride) * 35 / 100
    } else {
        0
    };
    let jacket = if r.hurt_flash > 0 && tick % 4 < 2 {
        mix(look.jacket, [ONE, ONE / 3, ONE / 3], ONE / 2)
    } else {
        look.jacket
    };
    // crouched: lower, the thighs forward
    let hip_y = if r.crouch || r.roll_t > 0 || r.slide_t > 0 {
        cm(55)
    } else {
        cm(90)
    };
    let bend = if hip_y < cm(90) { -deg(22) } else { 0 };
    // legs swing about the hips
    for (side, s) in [(-1, swing), (1, -swing)] {
        let hip = Affine::translate(v3(side * cm(13), hip_y, 0)).then(&Affine::rot_x(s + bend));
        let leg = base.then(&hip);
        // crouched, the legs fold: a short thigh down to a knee on the floor
        let len = if bend != 0 { cm(58) } else { cm(84) };
        put(
            b,
            Mesh::Cube,
            &leg,
            v3(0, -len / 2, 0),
            v3(cm(17), len, cm(19)),
            look.pants,
            0,
            true,
        );
        put(
            b,
            Mesh::Cube,
            &leg,
            v3(0, -len, -cm(4)),
            v3(cm(19), cm(12), cm(28)),
            look.boots,
            0,
            true,
        );
    }
    // the body
    let lean = if r.sprinting {
        deg(10)
    } else if hip_y < cm(90) {
        deg(18)
    } else {
        0
    };
    let torso = base
        .then(&Affine::translate(v3(0, hip_y, 0)))
        .then(&Affine::rot_x(-lean));
    let coat = look.coat.unwrap_or(jacket);
    put(
        b,
        Mesh::Cube,
        &torso,
        v3(0, cm(32), 0),
        v3(cm(46), cm(62), cm(28)),
        jacket,
        0,
        true,
    );
    if look.coat.is_some() {
        // a coat over the jacket, longer, open at the front
        put(
            b,
            Mesh::Cube,
            &torso,
            v3(0, cm(20), cm(2)),
            v3(cm(52), cm(80), cm(30)),
            coat,
            0,
            true,
        );
    }
    if look.vest {
        put(
            b,
            Mesh::Cube,
            &torso,
            v3(0, cm(36), -cm(3)),
            v3(cm(50), cm(40), cm(30)),
            shade(look.helmet, 85),
            0,
            true,
        );
    }
    put(
        b,
        Mesh::Cube,
        &torso,
        v3(0, cm(8), 0),
        v3(cm(48), cm(10), cm(30)),
        look.boots,
        0,
        true,
    );
    put(
        b,
        Mesh::Cube,
        &torso,
        v3(0, cm(36), cm(22)),
        v3(cm(38), cm(46), cm(20)),
        look.pack,
        0,
        true,
    );
    put(
        b,
        Mesh::Cube,
        &torso,
        v3(0, cm(64), cm(22)),
        v3(cm(30), cm(10), cm(16)),
        shade(look.pack, 80),
        0,
        true,
    );
    // the head looks where they aim
    let head = torso
        .then(&Affine::translate(v3(0, cm(78), 0)))
        .then(&Affine::rot_x(r.pitch / 2));
    put(
        b,
        Mesh::Sphere,
        &head,
        V3::ZERO,
        v3(cm(26), cm(28), cm(26)),
        look.skin,
        0,
        true,
    );
    match look.head {
        "helmet" => {
            put(
                b,
                Mesh::Sphere,
                &head,
                v3(0, cm(8), cm(2)),
                v3(cm(34), cm(26), cm(34)),
                look.helmet,
                0,
                true,
            );
            put(
                b,
                Mesh::Cube,
                &head,
                v3(0, cm(2), -cm(13)),
                v3(cm(28), cm(14), cm(4)),
                look.visor,
                ONE / 3,
                true,
            );
        }
        "headset" => {
            put(
                b,
                Mesh::Sphere,
                &head,
                v3(0, cm(9), cm(2)),
                v3(cm(30), cm(18), cm(30)),
                look.skin,
                0,
                true,
            );
            for side in [-1, 1] {
                put(
                    b,
                    Mesh::Cyl,
                    &head,
                    v3(side * cm(15), 0, 0),
                    v3(cm(12), cm(6), cm(12)),
                    shade(look.helmet, 50),
                    0,
                    true,
                );
            }
            put(
                b,
                Mesh::Sphere,
                &head,
                v3(cm(10), -cm(6), -cm(14)),
                v3(cm(5), cm(5), cm(5)),
                look.visor,
                ONE,
                false,
            );
        }
        _ => {
            // a cap, its peak forward
            put(
                b,
                Mesh::Sphere,
                &head,
                v3(0, cm(9), cm(1)),
                v3(cm(29), cm(17), cm(29)),
                look.jacket,
                0,
                true,
            );
            put(
                b,
                Mesh::Cube,
                &head,
                v3(0, cm(6), -cm(15)),
                v3(cm(22), cm(3), cm(14)),
                look.jacket,
                0,
                true,
            );
        }
    }
    if downed {
        return;
    }
    let shoulder_r = torso.apply(v3(cm(26), cm(52), 0));
    let shoulder_l = torso.apply(v3(-cm(26), cm(52), 0));
    let (len, w, metal) = match look.melee {
        "sledge" => (cm(95), cm(5), true),
        "machete" => (cm(70), cm(3), false),
        _ => (cm(85), cm(6), false),
    };
    let head_rgb = [ONE * 72 / 100, ONE * 76 / 100, ONE * 74 / 100];
    if r.melee_t > 0 {
        // the swing: from up behind the right shoulder, down across the front
        let t = (ONE * r.melee_t / crate::run::MELEE_HIT).min(ONE * 14 / 10);
        let a = -deg(110) + fx::mul(deg(170), t);
        let dir = torso
            .apply_dir(v3(fx::sin(a) / 2, fx::cos(a), -fx::cos(a / 2).abs()))
            .norm();
        let hand = shoulder_r.add(torso.apply_dir(v3(0, -cm(10), -cm(30))));
        let tip = hand.add(dir.scale(len));
        segment(b, Mesh::Cube, shoulder_r, hand, cm(12), jacket, 0, true);
        segment(b, Mesh::Cyl, hand, tip, w, head_rgb, 0, true);
        if metal {
            put(
                b,
                Mesh::Cube,
                &Affine::translate(tip),
                V3::ZERO,
                v3(cm(24), cm(14), cm(14)),
                shade(head_rgb, 60),
                0,
                true,
            );
        }
        segment(b, Mesh::Cube, shoulder_l, hand, cm(12), jacket, 0, true);
        return;
    }
    // the melee weapon rides on the pack
    let a = torso.apply(v3(-cm(22), cm(10), cm(34)));
    let z = torso.apply(v3(cm(22), cm(10) + len, cm(34)));
    segment(b, Mesh::Cyl, a, z, w, head_rgb, 0, true);
    if metal {
        put(
            b,
            Mesh::Cube,
            &Affine::translate(z),
            V3::ZERO,
            v3(cm(24), cm(14), cm(14)),
            shade(head_rgb, 60),
            0,
            true,
        );
    }
    // the gun, and arms reaching to it
    let g = gun_frame(r);
    gun_model(b, &g, gun);
    let grip = g.apply(v3(0, -cm(3), cm(2)));
    let fore = g.apply(v3(0, 0, -cm(22)));
    segment(b, Mesh::Cube, shoulder_r, grip, cm(12), jacket, 0, true);
    segment(b, Mesh::Cube, shoulder_l, fore, cm(12), jacket, 0, true);
}

pub fn gun_model(b: &mut Batch, frame: &Affine, gun: &Gun) {
    for p in &gun.model {
        let a = if p.along_z {
            frame
                .then(&Affine::translate(p.at))
                .then(&Affine::rot_x(QUARTER))
                .then(&Affine::scale(v3(p.size.x, p.size.y, p.size.z)))
        } else {
            frame.then(&Affine::translate(p.at)).then(&Affine::scale(p.size))
        };
        b.inked(p.mesh, &a, p.rgb, p.glow);
    }
}

/// A muzzle flash at every muzzle of a gun that just fired.
pub fn muzzle_flash(b: &mut Batch, r: &Raider, gun: &Gun, colour: Rgb) {
    if r.firing < 4 {
        return;
    }
    let g = gun_frame(r);
    let muzzles: Vec<V3> = if gun.gatling {
        vec![gun.muzzles[r.next_muzzle.wrapping_sub(1) % gun.muzzles.len()]]
    } else {
        gun.muzzles.clone()
    };
    for m in muzzles {
        glow_ball(b, g.apply(v3(m.x, m.y, m.z - cm(6))), cm(14), colour, ONE);
    }
}

// ---- drones -------------------------------------------------------------------------------

pub struct DroneColours {
    pub body: Rgb,
    pub dark: Rgb,
    pub armour: Rgb,
    pub weak: Rgb,
    pub rotor: Rgb,
    pub eye: Rgb,
    pub beacon: Rgb,
    pub core: Rgb,
    pub goo: Rgb,
    pub ember: Rgb,
    pub shock: Rgb,
    pub laser: Rgb,
}

pub fn drone(b: &mut Batch, d: &Drone, c: &DroneColours, tick: u32) {
    if d.state == State::Wreck {
        return;
    }
    let mut base = Affine::translate(d.pos).then(&Affine::rot_y(d.yaw));
    if !d.kind.walks() {
        base = base
            .then(&Affine::rot_x(-d.lean.0))
            .then(&Affine::rot_z(-d.lean.1));
    }
    let flash = d.flash > 0;
    let body = if flash {
        mix(c.body, [ONE; 3], ONE / 2)
    } else {
        c.body
    };
    let at = |x: i32, y: i32, z: i32| v3(cm(x), cm(y), cm(z));
    let sz = |x: i32, y: i32, z: i32| v3(cm(x), cm(y), cm(z));
    let spin = (tick as i32 * deg(47)) & (fx::TURN - 1);
    let weak_pulse = ONE / 2 + fx::sin((tick as i32 * deg(9)) & (fx::TURN - 1)) / 4;
    let has = |i: usize| d.pieces[i] > 0;
    b.bold = true;
    let rotor = |b: &mut Batch, i: usize, x: i32, y: i32, z: i32, r: i32| {
        if d.pieces[i] <= 0 {
            // a stump where it was
            put(
                b,
                Mesh::Cyl,
                &base,
                at(x, y - 4, z),
                sz(8, 10, 8),
                c.dark,
                0,
                true,
            );
            return;
        }
        put(
            b,
            Mesh::Cyl,
            &base,
            at(x, y - 6, z),
            sz(8, 12, 8),
            c.dark,
            0,
            true,
        );
        let hub = base
            .then(&Affine::translate(at(x, y, z)))
            .then(&Affine::rot_y(spin + i as i32 * deg(30)));
        put(b, Mesh::Cube, &hub, V3::ZERO, sz(r * 2, 2, 7), c.rotor, 0, false);
        put(b, Mesh::Cube, &hub, V3::ZERO, sz(7, 2, r * 2), c.rotor, 0, false);
        // the blur of the blades
        let disc = base
            .then(&Affine::translate(at(x, y, z)))
            .then(&Affine::scale(sz(r * 2, 1, r * 2)));
        b.push(
            Mesh::Cyl,
            &disc,
            mix(c.rotor, [ONE; 3], ONE * 6 / 10),
            ONE / 5,
            PASS_SOLID,
        );
    };
    match d.kind {
        Kind::Wasp => {
            put(b, Mesh::Sphere, &base, at(0, 0, 0), sz(62, 46, 74), body, 0, true);
            for k in [-20, 0, 20] {
                put(b, Mesh::Cube, &base, at(0, 6, k), sz(64, 6, 7), c.dark, 0, false);
            }
            if has(4) {
                put(
                    b,
                    Mesh::Wedge,
                    &base,
                    at(0, 0, -30),
                    sz(50, 38, 20),
                    c.armour,
                    0,
                    true,
                );
            }
            put(
                b,
                Mesh::Sphere,
                &base,
                at(0, -2, -42),
                sz(18, 18, 12),
                c.eye,
                ONE / 2,
                true,
            );
            put(
                b,
                Mesh::Cube,
                &base,
                at(0, 4, 36),
                sz(26, 20, 14),
                c.weak,
                weak_pulse,
                true,
            );
            for x in [-8, 8] {
                if has(5) {
                    segment(
                        b,
                        Mesh::Cube,
                        base.apply(at(x, -22, -20)),
                        base.apply(at(x, -22, -50)),
                        cm(5),
                        c.dark,
                        0,
                        true,
                    );
                }
            }
            for (i, (x, z)) in [(-44, -44), (44, -44), (-44, 44), (44, 44)]
                .into_iter()
                .enumerate()
            {
                segment(
                    b,
                    Mesh::Cube,
                    base.apply(at(0, 8, 0)),
                    base.apply(at(x, 10, z)),
                    cm(7),
                    c.dark,
                    0,
                    true,
                );
                rotor(b, i, x, 16, z, 20);
            }
        }
        Kind::Moth => {
            put(b, Mesh::Sphere, &base, at(0, 0, 0), sz(54, 40, 70), body, 0, true);
            for side in [-1, 1] {
                if has(if side < 0 { 4 } else { 5 }) {
                    put(
                        b,
                        Mesh::Cube,
                        &base,
                        at(side * 78, 6, 0),
                        sz(92, 5, 66),
                        c.dark,
                        0,
                        true,
                    );
                }
                for k in [-20, 0, 20] {
                    if has(if side < 0 { 4 } else { 5 }) {
                        put(
                            b,
                            Mesh::Cube,
                            &base,
                            at(side * 78, 9, k),
                            sz(84, 2, 3),
                            shade(c.dark, 70),
                            0,
                            false,
                        );
                    }
                }
            }
            put(
                b,
                Mesh::Sphere,
                &base,
                at(0, 18, 0),
                sz(26, 22, 26),
                c.weak,
                weak_pulse,
                true,
            );
            let blink = if tick % 40 < 20 { ONE } else { ONE / 5 };
            if has(6) {
                put(
                    b,
                    Mesh::Sphere,
                    &base,
                    at(0, 32, -10),
                    sz(14, 14, 14),
                    c.beacon,
                    blink,
                    true,
                );
            }
            if tick % 40 < 20 && has(6) {
                glow_ball(b, base.apply(at(0, 32, -10)), cm(40), c.beacon, ONE / 2);
            }
            for (i, (x, z)) in [(-60, -30), (60, -30), (-60, 30), (60, 30)]
                .into_iter()
                .enumerate()
            {
                rotor(b, i, x, -12, z, 18);
            }
            for x in [-12, 0, 12] {
                segment(
                    b,
                    Mesh::Cube,
                    base.apply(at(x, -18, -10)),
                    base.apply(at(x, -50, -6)),
                    cm(2),
                    c.dark,
                    0,
                    false,
                );
            }
        }
        Kind::Hornet => {
            put(
                b,
                Mesh::Sphere,
                &base,
                at(0, 0, 0),
                sz(44, 40, 110),
                body,
                0,
                true,
            );
            segment(
                b,
                Mesh::Cube,
                base.apply(at(0, 4, 40)),
                base.apply(at(0, 10, 85)),
                cm(10),
                c.dark,
                0,
                true,
            );
            if has(2) {
                put(
                    b,
                    Mesh::Wedge,
                    &base,
                    at(0, 6, -40),
                    sz(36, 30, 24),
                    c.armour,
                    0,
                    true,
                );
            }
            for side in [-1, 1] {
                put(
                    b,
                    Mesh::Cube,
                    &base,
                    at(side * 26, 0, 10),
                    sz(16, 4, 40),
                    c.armour,
                    0,
                    true,
                );
            }
            if has(3) {
                segment(
                    b,
                    Mesh::Cyl,
                    base.apply(at(0, -8, -30)),
                    base.apply(at(0, -8, -95)),
                    cm(8),
                    c.dark,
                    0,
                    true,
                );
            }
            let charging = d.brain.charge > 0;
            let lens = if charging { ONE } else { ONE / 3 };
            if has(3) {
                put(
                    b,
                    Mesh::Sphere,
                    &base,
                    at(0, -8, -98),
                    sz(16, 16, 10),
                    c.laser,
                    lens,
                    true,
                );
            }
            if has(3) {
                put(
                    b,
                    Mesh::Ring,
                    &base,
                    at(0, -8, -96),
                    sz(22, 8, 22),
                    c.weak,
                    weak_pulse,
                    false,
                );
            }
            rotor(b, 0, 0, 28, -40, 30);
            rotor(b, 1, 0, 28, 85, 30);
            if charging {
                glow_ball(
                    b,
                    base.apply(at(0, -8, -100)),
                    cm(30) + (tick as i32 % 6) * cm(4),
                    c.laser,
                    ONE,
                );
            }
        }
        Kind::Warden => {
            put(
                b,
                Mesh::Sphere,
                &base,
                at(0, 10, 0),
                sz(170, 120, 170),
                body,
                0,
                true,
            );
            if has(4) {
                put(
                    b,
                    Mesh::Cube,
                    &base,
                    at(0, 30, -70),
                    sz(120, 30, 30),
                    c.armour,
                    0,
                    true,
                );
            }
            for k in [-30, 0, 30] {
                if has(5) {
                    put(
                        b,
                        Mesh::Cube,
                        &base,
                        at(k, -40, 0),
                        sz(18, 30, 140),
                        c.armour,
                        0,
                        true,
                    );
                }
            }
            if has(6) {
                put(
                    b,
                    Mesh::Cube,
                    &base,
                    at(0, -30, -92),
                    sz(36, 22, 30),
                    c.dark,
                    0,
                    true,
                );
                for x in [-10, 10] {
                    segment(
                        b,
                        Mesh::Cyl,
                        base.apply(at(x, -32, -100)),
                        base.apply(at(x, -32, -135)),
                        cm(8),
                        c.dark,
                        0,
                        true,
                    );
                }
            }
            put(
                b,
                Mesh::Sphere,
                &base,
                at(0, -58, 0),
                sz(60, 40, 60),
                c.core,
                ONE * 8 / 10,
                true,
            );
            put(
                b,
                Mesh::Ring,
                &base,
                at(0, -66, 0),
                sz(76, 10, 76),
                c.weak,
                weak_pulse,
                false,
            );
            for x in [-36, -12, 12, 36] {
                put(
                    b,
                    Mesh::Sphere,
                    &base,
                    at(x, 22, -82),
                    sz(14, 14, 8),
                    c.eye,
                    ONE / 2,
                    true,
                );
            }
            for (i, (x, z)) in [(-125, -125), (125, -125), (-125, 125), (125, 125)]
                .into_iter()
                .enumerate()
            {
                segment(
                    b,
                    Mesh::Cube,
                    base.apply(at(0, 20, 0)),
                    base.apply(at(x, 20, z)),
                    cm(18),
                    c.dark,
                    0,
                    true,
                );
                if d.pieces[i] > 0 {
                    put(
                        b,
                        Mesh::Ring,
                        &base,
                        at(x, 20, z),
                        sz(92, 24, 92),
                        c.armour,
                        0,
                        true,
                    );
                }
                rotor(b, i, x, 22, z, 40);
            }
        }
        Kind::Tick | Kind::Strider => {
            let big = d.kind == Kind::Strider;
            let k = if big { 100 } else { 40 };
            let s = |n: i32| n * k / 100;
            let ride = if big { 150 } else { 45 };
            let bob = fx::sin(d.gait * 2) * cm(s(4)) / ONE;
            // a walker short of legs on one side sits tilted toward it
            let core = base
                .then(&Affine::translate(v3(0, cm(ride) + bob, 0)))
                .then(&Affine::rot_z(-d.lean.1));
            if big {
                put(
                    b,
                    Mesh::Sphere,
                    &core,
                    at(0, 10, -20),
                    sz(170, 110, 190),
                    body,
                    0,
                    true,
                );
                if has(7) {
                    put(
                        b,
                        Mesh::Cube,
                        &core,
                        at(0, 50, -30),
                        sz(130, 20, 130),
                        c.armour,
                        0,
                        true,
                    );
                }
                put(
                    b,
                    Mesh::Sphere,
                    &core,
                    at(0, -10, 95),
                    sz(100, 90, 110),
                    c.weak,
                    weak_pulse,
                    true,
                );
                if has(6) {
                    put(
                        b,
                        Mesh::Cyl,
                        &core,
                        at(0, 85, -25),
                        sz(64, 40, 64),
                        c.armour,
                        0,
                        true,
                    );
                }
                for x in [-12, 12].into_iter().filter(|_| has(6)) {
                    segment(
                        b,
                        Mesh::Cyl,
                        core.apply(at(x, 95, -40)),
                        core.apply(at(x, 95, -110)),
                        cm(10),
                        c.dark,
                        0,
                        true,
                    );
                }
                for x in [-18, 0, 18] {
                    put(
                        b,
                        Mesh::Sphere,
                        &core,
                        at(x, 20, -105),
                        sz(14, 14, 10),
                        c.eye,
                        ONE / 2,
                        true,
                    );
                }
            } else {
                put(
                    b,
                    Mesh::Sphere,
                    &core,
                    at(0, 10, 0),
                    sz(80, 64, 80),
                    body,
                    0,
                    true,
                );
                if has(6) {
                    put(
                        b,
                        Mesh::Cyl,
                        &core,
                        at(0, -12, 0),
                        sz(86, 14, 86),
                        c.armour,
                        0,
                        true,
                    );
                }
                put(
                    b,
                    Mesh::Sphere,
                    &core,
                    at(0, -26, 0),
                    sz(40, 18, 40),
                    c.weak,
                    weak_pulse,
                    true,
                );
                let armed = d.brain.fuse > 0;
                let blink = if armed && tick % 6 < 3 || !armed && tick % 50 < 10 {
                    ONE
                } else {
                    ONE / 6
                };
                put(
                    b,
                    Mesh::Sphere,
                    &core,
                    at(0, 44, 0),
                    sz(18, 18, 18),
                    c.eye,
                    blink,
                    true,
                );
                if armed {
                    glow_ball(b, core.apply(at(0, 40, 0)), cm(60), c.eye, ONE / 2);
                }
            }
            // six legs: hip, knee raised, foot on the floor, stepping in two sets
            for leg in 0..6 {
                let side = if leg < 3 { -1 } else { 1 };
                let row = (leg % 3) - 1;
                let phase = d.gait + if (leg + (leg / 3)) % 2 == 0 { 0 } else { fx::HALF };
                let lift = (fx::sin(phase).max(0)) * cm(s(24)) / ONE;
                let stride = fx::cos(phase) * cm(s(30)) / ONE;
                let hip = core.apply(at(side * s(50), 0, row * s(50)));
                let foot_local = v3(cm(side * s(150)), lift, cm(row * s(85)) + stride);
                let foot = base.apply(foot_local);
                let knee = base.apply(v3(
                    cm(side * s(110)),
                    cm(ride) + cm(s(60)),
                    cm(row * s(70)) + stride / 2,
                ));
                let w = cm(s(12)).max(cm(4));
                if !has(leg as usize) {
                    // a stump at the hip where the leg was torn off
                    let stub = hip.lerp(knee, ONE / 5);
                    segment(b, Mesh::Cube, hip, stub, w, c.dark, 0, true);
                    continue;
                }
                segment(b, Mesh::Cube, hip, knee, w, c.dark, 0, true);
                segment(b, Mesh::Cube, knee, foot, w * 8 / 10, c.armour, 0, true);
            }
        }
    }
    b.bold = false;
    // what is on it: goo, flames, a stall
    if d.goo > 0 {
        for (k, &(x, y, z, r, _)) in zones(d.kind).iter().enumerate().take(3) {
            let off = (k as i32 * 37) % 20 - 10;
            put(
                b,
                Mesh::Blob,
                &base,
                at(x + off, y + r / 2, z - off),
                sz(r, r / 2, r),
                c.goo,
                ONE / 4,
                false,
            );
        }
    }
    if d.burn > 0 && tick.is_multiple_of(3) {
        glow_ball(b, d.centre().add(v3(0, cm(30), 0)), cm(30), c.ember, ONE * 7 / 10);
    }
    if d.stun > 0 && tick % 4 < 2 {
        glow_ball(b, d.centre(), cm(90), c.shock, ONE / 2);
    }
    if matches!(d.state, State::Falling(_)) && tick.is_multiple_of(2) {
        glow_ball(b, d.centre(), cm(50), c.ember, ONE / 2);
    }
}

/// Every drone's weak spot as a world point, for the scanner sight.
pub fn weak_spot(d: &Drone) -> Option<V3> {
    zones(d.kind)
        .iter()
        .find(|z| z.4 == Zone::Weak)
        .map(|z| d.to_world(z.0, z.1, z.2))
}

// ---- the van, the hive, loot, shots --------------------------------------------------------------

pub fn van(b: &mut Batch, v: &Van, pal: &Palette) {
    let f = facing(v.yaw, 0);
    let base = Affine::translate(v.pos).then(&Affine::rot_y(v.yaw));
    let body = pal.get("van");
    let dark = pal.get("van_dark");
    let stripe = pal.get("van_stripe");
    let inside = pal.get("van_inside");
    let at = |x: i32, y: i32, z: i32| v3(cm(x), cm(y), cm(z));
    let sz = |x: i32, y: i32, z: i32| v3(cm(x), cm(y), cm(z));
    // the van runs backward (+z local) from its back doors
    put(
        b,
        Mesh::Cube,
        &base,
        at(0, 30, 285),
        sz(240, 60, 570),
        dark,
        0,
        true,
    );
    put(
        b,
        Mesh::Cube,
        &base,
        at(-113, 165, 200),
        sz(15, 210, 400),
        body,
        0,
        true,
    );
    put(
        b,
        Mesh::Cube,
        &base,
        at(113, 165, 200),
        sz(15, 210, 400),
        body,
        0,
        true,
    );
    put(
        b,
        Mesh::Cube,
        &base,
        at(0, 280, 200),
        sz(240, 20, 400),
        body,
        0,
        true,
    );
    put(
        b,
        Mesh::Cube,
        &base,
        at(0, 61, 200),
        sz(210, 2, 400),
        inside,
        0,
        false,
    );
    for side in [-1, 1] {
        put(
            b,
            Mesh::Cube,
            &base,
            at(side * 121, 150, 200),
            sz(2, 30, 400),
            stripe,
            0,
            false,
        );
        // benches inside
        put(
            b,
            Mesh::Cube,
            &base,
            at(side * 85, 80, 220),
            sz(40, 40, 300),
            inside,
            0,
            true,
        );
    }
    // the cab
    put(
        b,
        Mesh::Cube,
        &base,
        at(0, 160, 485),
        sz(240, 200, 170),
        body,
        0,
        true,
    );
    put(
        b,
        Mesh::Wedge,
        &base,
        at(0, 225, 560),
        sz(230, 70, 40),
        pal.get("glass"),
        0,
        true,
    );
    // a monitor on the cab wall with the order on it, and a crate of parts
    put(
        b,
        Mesh::Cube,
        &base,
        at(0, 170, 395),
        sz(90, 60, 6),
        pal.get("ink"),
        0,
        true,
    );
    put(
        b,
        Mesh::Cube,
        &base,
        at(0, 170, 392),
        sz(80, 50, 2),
        pal.get("visor"),
        ONE / 2,
        false,
    );
    put(
        b,
        Mesh::Cube,
        &base,
        at(-60, 90, 360),
        sz(50, 40, 40),
        pal.get("wood"),
        0,
        true,
    );
    for (x, z) in [(-115, 80), (115, 80), (-115, 470), (115, 470)] {
        let w = base
            .then(&Affine::translate(at(x, 40, z)))
            .then(&Affine::rot_z(QUARTER));
        put(
            b,
            Mesh::Cyl,
            &w,
            V3::ZERO,
            sz(80, 30, 80),
            pal.get("tyre"),
            0,
            true,
        );
    }
    // the doors, shut or swung right round to the sides
    for side in [-1, 1] {
        let hinge = base.then(&Affine::translate(at(side * 118, 165, 0)));
        let turn = if v.doors_open { -side * deg(100) } else { 0 };
        let door = hinge.then(&Affine::rot_y(turn));
        put(
            b,
            Mesh::Cube,
            &door,
            at(-side * 59, 0, -4),
            sz(118, 210, 8),
            body,
            0,
            true,
        );
        put(
            b,
            Mesh::Cube,
            &door,
            at(-side * 59, 50, -9),
            sz(80, 50, 2),
            pal.get("glass"),
            0,
            true,
        );
    }
    let _ = f;
}

pub fn hive(b: &mut Batch, h: &Hive, pal: &Palette, tick: u32) {
    if h.hp <= 0 {
        // a heap of scrap where it stood
        put(
            b,
            Mesh::Blob,
            &Affine::IDENTITY,
            v3(h.pos.x, ONE / 2, h.pos.z),
            v3(int(5), ONE, int(5)),
            pal.get("rust"),
            0,
            true,
        );
        return;
    }
    let base = Affine::translate(h.pos);
    put(
        b,
        Mesh::Cone,
        &base,
        v3(0, int(2), 0),
        v3(int(5), int(5), int(5)),
        pal.get("hive"),
        0,
        true,
    );
    for k in 0..6 {
        let a = deg(60) * k + deg(30);
        let shelf = Affine::translate(h.pos).then(&Affine::rot_y(a));
        put(
            b,
            Mesh::Cube,
            &shelf,
            v3(0, int(1), int(2)),
            v3(int(1), int(2), ONE / 4),
            pal.get("rack"),
            0,
            true,
        );
    }
    let open = h.pods.iter().all(|p| *p <= 0);
    let pulse = ONE / 2 + fx::sin((tick as i32 * deg(5)) & (fx::TURN - 1)) / 3;
    let core = if open { pal.get("core") } else { pal.get("armour") };
    put(
        b,
        Mesh::Sphere,
        &Affine::IDENTITY,
        h.core(),
        v3(int(3), int(3), int(3)),
        core,
        if open { pulse } else { 0 },
        true,
    );
    for i in 0..6 {
        if h.pods[i] > 0 {
            put(
                b,
                Mesh::Sphere,
                &Affine::IDENTITY,
                h.pod_at(i),
                v3(ONE, ONE, ONE),
                pal.get("hive_pod"),
                pulse,
                true,
            );
        }
    }
}

pub fn pickup(b: &mut Batch, p: &Pickup, c: &Content, tick: u32, pal: &Palette) {
    let bob = fx::sin((tick as i32 * deg(4)) & (fx::TURN - 1)) / 10;
    let spin = Affine::translate(v3(p.pos.x, p.pos.y + cm(30) + bob, p.pos.z))
        .then(&Affine::rot_y(tick as i32 * deg(2)));
    match p.item {
        Item::Mission => {
            put(
                b,
                Mesh::Cube,
                &spin,
                V3::ZERO,
                v3(cm(40), cm(40), cm(14)),
                pal.get("objective"),
                ONE * 6 / 10,
                true,
            );
            glow_ball(b, spin.t, cm(80), pal.get("objective"), ONE / 2);
        }
        Item::Gear(gk) => {
            let gear = &c.gear[gk as usize];
            let colour = crate::content::hex(&gear.colour);
            put(
                b,
                Mesh::Cube,
                &spin,
                V3::ZERO,
                v3(cm(36), cm(18), cm(30)),
                colour,
                ONE / 5,
                true,
            );
            put(
                b,
                Mesh::Cube,
                &spin,
                v3(0, cm(11), 0),
                v3(cm(38), cm(4), cm(32)),
                pal.get("loot_rare"),
                ONE / 2,
                false,
            );
            glow_ball(b, spin.t, cm(50), pal.get("loot_rare"), ONE / 4);
        }
        Item::Ammo(_, _) => {
            for k in [-1, 0, 1] {
                put(
                    b,
                    Mesh::Cyl,
                    &spin,
                    v3(k * cm(9), 0, 0),
                    v3(cm(7), cm(22), cm(7)),
                    pal.get("gold"),
                    ONE / 6,
                    true,
                );
            }
            glow_ball(b, spin.t, cm(35), pal.get("loot"), ONE / 4);
        }
        Item::Part(k) => {
            let part = &c.parts[k as usize];
            let rare = part.rarity >= 3;
            put(
                b,
                Mesh::Cube,
                &spin,
                V3::ZERO,
                v3(cm(34), cm(22), cm(26)),
                part.colour,
                ONE / 5,
                true,
            );
            put(
                b,
                Mesh::Cube,
                &spin,
                v3(0, cm(12), 0),
                v3(cm(36), cm(4), cm(28)),
                pal.get(if rare { "loot_rare" } else { "loot" }),
                ONE / 2,
                false,
            );
            glow_ball(
                b,
                spin.t,
                cm(45),
                pal.get(if rare { "loot_rare" } else { "loot" }),
                ONE / 4,
            );
        }
    }
}

pub fn shot(b: &mut Batch, s: &Shot, pal: &Palette) {
    let dir = s.vel.norm();
    let colour = match s.from {
        ShotFrom::Drone(_) => pal.get("enemy_shot"),
        ShotFrom::Raider(_) => match s.effect {
            crate::content::Effect::Goo => pal.get("goo"),
            crate::content::Effect::Shock => pal.get("shock"),
            crate::content::Effect::Ember => pal.get("ember"),
            crate::content::Effect::None => pal.get("tracer"),
        },
    };
    match (s.from, s.kind) {
        (ShotFrom::Drone(_), _) => {
            let r = if s.heavy { cm(30) } else { cm(12) };
            put(
                b,
                Mesh::Sphere,
                &Affine::IDENTITY,
                s.pos,
                v3(r, r, r),
                colour,
                ONE,
                false,
            );
            glow_ball(b, s.pos, r * 3, colour, ONE * 7 / 10);
        }
        (_, ShotKind::Bullet) => {
            let len = s.vel.len() / 50;
            segment(
                b,
                Mesh::Cube,
                s.pos.sub(dir.scale(len)),
                s.pos,
                cm(4),
                colour,
                ONE,
                false,
            );
        }
        (_, ShotKind::Rocket) => {
            segment(
                b,
                Mesh::Cyl,
                s.pos.sub(dir.scale(cm(60))),
                s.pos,
                cm(14),
                pal.get("metal"),
                0,
                true,
            );
            glow_ball(b, s.pos.sub(dir.scale(cm(70))), cm(40), pal.get("ember"), ONE);
            put(
                b,
                Mesh::Sphere,
                &Affine::IDENTITY,
                s.pos,
                v3(cm(16), cm(16), cm(16)),
                colour,
                ONE / 2,
                true,
            );
        }
        (_, ShotKind::Grenade) => {
            put(
                b,
                Mesh::Sphere,
                &Affine::IDENTITY,
                s.pos,
                v3(cm(18), cm(18), cm(18)),
                colour,
                ONE / 3,
                true,
            );
        }
    }
}

/// A belt gadget out in the world.
pub fn gadget(b: &mut Batch, g: &crate::run::Placed, pal: &Palette, tick: u32) {
    use crate::kit::Gadget;
    let blink = if tick % 30 < 15 { ONE } else { ONE / 6 };
    match g.kind {
        Gadget::Mine => {
            put(
                b,
                Mesh::Cyl,
                &Affine::translate(g.pos),
                v3(0, cm(5), 0),
                v3(cm(40), cm(10), cm(40)),
                pal.get("metal_dark"),
                0,
                true,
            );
            let light = if g.armed > 0 {
                pal.get("gold")
            } else {
                pal.get("red")
            };
            put(
                b,
                Mesh::Sphere,
                &Affine::translate(g.pos),
                v3(0, cm(12), 0),
                v3(cm(8), cm(8), cm(8)),
                light,
                blink,
                false,
            );
        }
        Gadget::Decoy => {
            put(
                b,
                Mesh::Cyl,
                &Affine::translate(g.pos),
                v3(0, cm(10), 0),
                v3(cm(12), cm(20), cm(12)),
                pal.get("gold"),
                0,
                true,
            );
            if g.landed && tick % 60 < 8 {
                glow_ball(b, g.pos.add(v3(0, cm(30), 0)), cm(60), pal.get("gold"), ONE / 2);
            }
        }
        Gadget::Smoke => {
            if !g.landed {
                put(
                    b,
                    Mesh::Sphere,
                    &Affine::translate(g.pos),
                    V3::ZERO,
                    v3(cm(14), cm(14), cm(14)),
                    pal.get("smoke"),
                    0,
                    true,
                );
                return;
            }
            // the cloud: a heap of soft puffs that grows in its first second
            let grow = ONE.min(ONE * (secs(12_000) - g.life).max(0) / TICKS_PER_SECOND.max(1) + ONE / 3);
            for k in 0..7 {
                let a = deg(51) * k + (tick as i32 * deg(1) / 4);
                let r = fx::mul(int(3), grow);
                let at = g
                    .pos
                    .add(facing(a, 0).scale(r / 2))
                    .add(v3(0, int(1) + (k % 3) * ONE * 8 / 10, 0));
                let s = fx::mul(int(4), grow);
                b.push(
                    Mesh::Blob,
                    &Affine::translate(at).then(&Affine::scale(v3(s, s * 3 / 4, s))),
                    pal.get("smoke"),
                    0,
                    PASS_SOLID,
                );
            }
        }
        Gadget::Paint => {
            let base = Affine::translate(g.pos).then(&Affine::rot_y(g.yaw));
            b.push(
                Mesh::Ground,
                &base.then(&Affine::scale(v3(
                    crate::run::PAINT_WIDE,
                    ONE,
                    crate::run::PAINT_LONG,
                ))),
                pal.get("shock"),
                ONE / 2,
                PASS_SOLID,
            );
            for k in 0..4 {
                let z = -crate::run::PAINT_LONG / 2 + (k * 2 + 1) * crate::run::PAINT_LONG / 8;
                let at = base.apply(v3(0, cm(2), z));
                put(
                    b,
                    Mesh::Wedge,
                    &Affine::translate(at).then(&Affine::rot_y(g.yaw)),
                    V3::ZERO,
                    v3(cm(80), cm(2), cm(60)),
                    pal.get("cream"),
                    ONE / 2,
                    false,
                );
            }
        }
    }
}

/// A beam: a laser, a shock's arc, a hornet's aiming line.
pub fn beam(b: &mut Batch, from: V3, to: V3, w: i32, colour: Rgb, strength: i32) {
    segment(b, Mesh::Cube, from, to, w, colour, strength, false);
}

/// Something short-lived: a spark, a puff of smoke, a scrap of a drone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Particle {
    pub pos: V3,
    pub vel: V3,
    pub life: i32,
    pub max: i32,
    pub size: i32,
    pub rgb: Rgb,
    pub glow: i32,
    pub fall: bool,
    pub mesh: Mesh,
}

impl Particle {
    pub fn step(&mut self) {
        self.life -= 1;
        if self.fall {
            self.vel.y -= int(14) / 60;
        }
        self.pos = self.pos.add(self.vel.scale(ONE / 60));
        if self.pos.y < 0 {
            self.pos.y = 0;
            self.vel = v3(self.vel.x / 2, -self.vel.y / 3, self.vel.z / 2);
        }
    }

    pub fn draw(&self, b: &mut Batch) {
        let t = fx::div(self.life, self.max.max(1));
        let s = fx::mul(self.size, if self.glow > 0 { t } else { ONE / 2 + t / 2 });
        let a = Affine::translate(self.pos).then(&Affine::scale(v3(s, s, s)));
        if self.glow > 0 {
            b.glow(self.mesh, &a, self.rgb, fx::mul(self.glow, t));
        } else {
            b.inked(self.mesh, &a, self.rgb, 0);
        }
    }
}

pub fn unused(_: u32) -> u32 {
    PASS_GLOW
}
