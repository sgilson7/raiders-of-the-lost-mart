//! A drone's body: where it is, how it flies or walks, where it can be hit
//! and what each hit does to it. What it decides to do is ai.rs.
//!
//! Flyers move like quadcopters. They lean into every change of speed, so
//! a drone that brakes tips back; a hit pushes them and sets them spinning
//! and rocking, and the rocking springs back; a rotor shot off tips them
//! toward the dead corner and lets them sag. Walkers keep their feet on the
//! floor and turn on the spot.

use crate::balance::*;
use crate::content::{DroneDef, Effect};
use crate::fx::{self, deg, int, ONE};
use crate::geom::{v3, V3};
use crate::world::{Aabb, Map};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Wasp,
    Moth,
    Hornet,
    Warden,
    Tick,
    Strider,
}

impl Kind {
    pub fn from_id(id: &str) -> Kind {
        match id {
            "moth" => Kind::Moth,
            "hornet" => Kind::Hornet,
            "warden" => Kind::Warden,
            "tick" => Kind::Tick,
            "strider" => Kind::Strider,
            _ => Kind::Wasp,
        }
    }

    pub fn walks(self) -> bool {
        matches!(self, Kind::Tick | Kind::Strider)
    }

    pub fn rotors(self) -> usize {
        match self {
            Kind::Wasp | Kind::Moth | Kind::Warden => 4,
            Kind::Hornet => 2,
            _ => 0,
        }
    }
}

/// What part of a drone a hit landed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Zone {
    Body,
    Armour,
    Weak,
    Rotor(usize),
}

/// A hit sphere in the drone's own space (x right, y up, −z forward), in
/// centimetres: offset, radius, zone.
pub fn zones(kind: Kind) -> &'static [(i32, i32, i32, i32, Zone)] {
    match kind {
        Kind::Wasp => &[
            (0, 0, -28, 26, Zone::Armour),
            (0, 0, 0, 32, Zone::Body),
            (0, 4, 34, 17, Zone::Weak),
            (-44, 14, -44, 20, Zone::Rotor(0)),
            (44, 14, -44, 20, Zone::Rotor(1)),
            (-44, 14, 44, 20, Zone::Rotor(2)),
            (44, 14, 44, 20, Zone::Rotor(3)),
        ],
        Kind::Moth => &[
            (0, 18, 0, 20, Zone::Weak),
            (0, 0, 0, 34, Zone::Body),
            (-78, 6, 0, 40, Zone::Armour),
            (78, 6, 0, 40, Zone::Armour),
            (-60, -12, -30, 18, Zone::Rotor(0)),
            (60, -12, -30, 18, Zone::Rotor(1)),
            (-60, -12, 30, 18, Zone::Rotor(2)),
            (60, -12, 30, 18, Zone::Rotor(3)),
        ],
        Kind::Hornet => &[
            (0, -8, -98, 16, Zone::Weak),
            (0, 0, -45, 22, Zone::Armour),
            (0, 0, 0, 26, Zone::Body),
            (0, 0, 45, 22, Zone::Body),
            (0, 28, -40, 30, Zone::Rotor(0)),
            (0, 28, 85, 30, Zone::Rotor(1)),
        ],
        Kind::Warden => &[
            (0, -62, 0, 36, Zone::Weak),
            (0, 0, 0, 92, Zone::Armour),
            (-125, 20, -125, 44, Zone::Rotor(0)),
            (125, 20, -125, 44, Zone::Rotor(1)),
            (-125, 20, 125, 44, Zone::Rotor(2)),
            (125, 20, 125, 44, Zone::Rotor(3)),
        ],
        Kind::Tick => &[(0, 18, 0, 22, Zone::Weak), (0, 45, 0, 40, Zone::Armour)],
        Kind::Strider => &[
            (0, 140, 95, 55, Zone::Weak),
            (0, 165, -20, 88, Zone::Armour),
            (0, 245, -25, 36, Zone::Body),
        ],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Active,
    /// shot down, falling for this many ticks before it breaks up
    Falling(i32),
    /// gone from the run
    Wreck,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drone {
    pub kind: Kind,
    pub def: usize,
    pub pos: V3,
    pub vel: V3,
    pub yaw: i32,
    /// lean forward (+) and right (+), and how fast each is changing
    pub lean: (i32, i32),
    pub lean_v: (i32, i32),
    pub spin: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub rotors: [i32; 4],
    pub state: State,
    pub goo: i32,
    /// armour eaten by goo, in percentage points
    pub corrode: i32,
    pub stun: i32,
    pub burn: i32,
    pub burn_by: u8,
    pub flash: i32,
    /// legs: the gait's phase
    pub gait: i32,
    /// part of a mission's count (the food court's nest)
    pub nest: bool,
    /// the last raider who hit it
    pub hit_by: u8,
    pub brain: crate::ai::Brain,
}

/// What a hit did, for the shot's owner and the effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Struck {
    pub zone: Zone,
    pub dealt: i32,
    pub killed: bool,
    pub rotor_lost: bool,
}

pub fn cm(n: i32) -> i32 {
    n * ONE / 100
}

impl Drone {
    pub fn new(kind: Kind, def: usize, d: &DroneDef, pos: V3, yaw: i32, home: V3) -> Drone {
        let rotor = if d.rotor_hp > 0 { d.rotor_hp } else { d.hp / 2 };
        Drone {
            kind,
            def,
            pos,
            vel: V3::ZERO,
            yaw,
            lean: (0, 0),
            lean_v: (0, 0),
            spin: 0,
            hp: d.hp,
            max_hp: d.hp,
            rotors: [rotor; 4],
            state: State::Active,
            goo: 0,
            corrode: 0,
            stun: 0,
            burn: 0,
            burn_by: 0,
            flash: 0,
            gait: 0,
            nest: false,
            hit_by: 0,
            brain: crate::ai::Brain::new(home),
        }
    }

    pub fn active(&self) -> bool {
        self.state == State::Active
    }

    /// The world position of a point given in the drone's space (cm),
    /// turned by its heading (lean is drawn, not hit-tested).
    pub fn to_world(&self, x: i32, y: i32, z: i32) -> V3 {
        let (s, c) = (fx::sin(self.yaw), fx::cos(self.yaw));
        let (lx, ly, lz) = (cm(x), cm(y), cm(z));
        // yaw turns −z (forward) toward −x when positive
        let wx = fx::mul(lx, c) + fx::mul(lz, s);
        let wz = -fx::mul(lx, s) + fx::mul(lz, c);
        v3(self.pos.x + wx, self.pos.y + ly, self.pos.z + wz)
    }

    /// Where it looks from: its eye.
    pub fn eye(&self) -> V3 {
        match self.kind {
            Kind::Strider => self.to_world(0, 170, -80),
            Kind::Tick => self.to_world(0, 50, -30),
            _ => self.pos,
        }
    }

    pub fn centre(&self) -> V3 {
        match self.kind {
            Kind::Strider => self.to_world(0, 160, 0),
            Kind::Tick => self.to_world(0, 40, 0),
            _ => self.pos,
        }
    }

    /// The nearest zone a ray `o + d·t` meets within `max_t`.
    pub fn ray(&self, o: V3, d: V3, max_t: i32) -> Option<(i32, Zone)> {
        // a quick test against the whole drone first
        let reach = cm(match self.kind {
            Kind::Warden => 190,
            Kind::Strider => 260,
            Kind::Moth => 130,
            Kind::Hornet => 130,
            _ => 80,
        });
        sphere(o, d, self.centre(), reach + cm(100), max_t)?;
        let mut best: Option<(i32, Zone)> = None;
        for &(x, y, z, r, zone) in zones(self.kind) {
            if let Zone::Rotor(i) = zone {
                if self.rotors[i] <= 0 {
                    continue;
                }
            }
            if let Some(t) = sphere(o, d, self.to_world(x, y, z), cm(r), max_t) {
                // weak spots win ties with what wraps them: they are inset
                let t = if zone == Zone::Weak { t - cm(12) } else { t };
                if best.is_none_or(|b| t < b.0) {
                    best = Some((t, zone));
                }
            }
        }
        best
    }

    /// Damage to this drone. `push` is the shot's direction (unit), `at`
    /// where it landed; the hit shoves and turns the body.
    pub fn strike(&mut self, d: &DroneDef, zone: Zone, dmg: i32, push: V3, at: V3, by: u8) -> Struck {
        let mut out = Struck {
            zone,
            dealt: 0,
            killed: false,
            rotor_lost: false,
        };
        if !self.active() {
            return out;
        }
        let armour = (d.armour_pct + self.corrode).min(100);
        let dealt = match zone {
            Zone::Body => dmg,
            Zone::Armour => dmg * armour / 100,
            Zone::Weak => dmg * d.weak_pct / 100,
            Zone::Rotor(i) => {
                let before = self.rotors[i];
                self.rotors[i] -= dmg;
                if before > 0 && self.rotors[i] <= 0 {
                    out.rotor_lost = true;
                }
                dmg / 4
            }
        };
        self.hp -= dealt;
        self.hit_by = by;
        self.flash = 6;
        out.dealt = dealt;
        // the shove: lighter drones move more
        let mass = d.hp.max(30);
        let kick = (dmg.max(4) * ONE * 3 / mass).min(int(6));
        if self.kind.walks() {
            self.vel = self.vel.add(v3(push.x, 0, push.z).scale(kick / 4));
        } else {
            self.vel = self.vel.add(push.scale(kick));
            // torque: off-centre hits spin and rock it
            let off = at.sub(self.pos);
            let torque = off.cross(push);
            self.spin += torque.y.clamp(-ONE, ONE) * deg(40) / ONE * kick / ONE;
            self.lean_v.0 += (torque.x.clamp(-ONE, ONE) / 6) * kick / ONE + deg(2);
            self.lean_v.1 += (torque.z.clamp(-ONE, ONE) / 6) * kick / ONE;
        }
        let dead_rotors = self.rotors[..self.kind.rotors()]
            .iter()
            .filter(|r| **r <= 0)
            .count();
        if self.hp <= 0 || (self.kind.rotors() > 0 && dead_rotors >= self.kind.rotors().div_ceil(2)) {
            self.hp = self.hp.max(0);
            self.state = State::Falling(if self.kind.walks() { secs(600) } else { FALL_TIME });
            self.spin += deg(12);
            out.killed = true;
        }
        out
    }

    pub fn apply_effect(&mut self, e: Effect, power: i32, by: u8) {
        match e {
            Effect::Goo => {
                self.goo = self.goo.max(power * 3);
                self.corrode = (self.corrode + power / 4).min(100);
            }
            Effect::Shock => {
                self.stun = self.stun.max(power / 2 + secs(350));
            }
            Effect::Ember => {
                self.burn = self.burn.max(power);
                self.burn_by = by;
            }
            Effect::None => {}
        }
    }

    pub fn speed_pct(&self) -> i32 {
        let mut pct = 100;
        if self.goo > 0 {
            pct = pct * (100 - GOO_SLOW_PCT) / 100;
        }
        let n = self.kind.rotors();
        if n > 0 {
            let alive = self.rotors[..n].iter().filter(|r| **r > 0).count() as i32;
            pct = pct * (40 + 60 * alive / n as i32) / 100;
        }
        pct
    }

    /// One tick of the body. `want` is the velocity the brain asks for and
    /// `face` the heading it wants; the body gets there as it can.
    pub fn step_body(&mut self, d: &DroneDef, want: V3, face: i32, map: &Map, extra: &[Aabb]) {
        self.flash = (self.flash - 1).max(0);
        self.goo = (self.goo - 1).max(0);
        self.stun = (self.stun - 1).max(0);
        if self.burn > 0 {
            self.burn -= 1;
        }
        let dt = ONE / TICKS_PER_SECOND;
        match self.state {
            State::Wreck => return,
            State::Falling(t) => {
                self.state = if t <= 1 {
                    State::Wreck
                } else {
                    State::Falling(t - 1)
                };
                if !self.kind.walks() {
                    self.vel.y -= GRAVITY / TICKS_PER_SECOND;
                    self.yaw = fx::wrap(self.yaw + self.spin / TICKS_PER_SECOND);
                    self.lean.0 += self.lean_v.0 / TICKS_PER_SECOND + deg(1);
                    self.lean.1 += self.spin / 200;
                }
                self.integrate(dt, map, extra);
                if self.pos.y <= self.floor(map, extra) + cm(10) && !self.kind.walks() {
                    // it hit the ground: break up now
                    self.state = State::Wreck;
                }
                return;
            }
            State::Active => {}
        }
        let r = self.radius(d);
        let top = d.speed * ONE / 10 * self.speed_pct() / 100;
        let mut want = want.clamp_len(top);
        if self.stun > 0 {
            // rotors stalled: it drops and shudders
            want = v3(0, -int(2), 0);
        }
        let acc = d.accel * ONE / 10 * 60 / TICKS_PER_SECOND / 10;
        let before = self.vel;
        if self.kind.walks() {
            self.vel.x = fx::approach(self.vel.x, want.x, acc);
            self.vel.z = fx::approach(self.vel.z, want.z, acc);
            self.vel.y -= GRAVITY / TICKS_PER_SECOND;
            self.gait = (self.gait + self.vel.len_xz() / 14) & (fx::TURN - 1);
        } else {
            self.vel.x = fx::approach(self.vel.x, want.x, acc);
            self.vel.y = fx::approach(self.vel.y, want.y, acc);
            self.vel.z = fx::approach(self.vel.z, want.z, acc);
            // a dead rotor drags its corner down and the drone sags
            let n = self.kind.rotors();
            let dead = self.rotors[..n].iter().filter(|r| **r <= 0).count() as i32;
            if dead > 0 {
                self.vel.y -= int(1) * dead / TICKS_PER_SECOND;
                self.spin += deg(8) * dead / TICKS_PER_SECOND;
            }
            // the lean follows the acceleration, in the drone's own frame
            let a = self.vel.sub(before);
            let (s, c) = (fx::sin(self.yaw), fx::cos(self.yaw));
            let fwd_acc = -fx::mul(a.x, s) - fx::mul(a.z, c);
            let right_acc = fx::mul(a.x, c) - fx::mul(a.z, s);
            let target = (
                (fwd_acc * 60).clamp(-deg(28), deg(28)),
                (right_acc * 60).clamp(-deg(28), deg(28)),
            );
            // a spring toward the target lean, damped
            for (l, (lv, t)) in [
                (&mut self.lean.0, (&mut self.lean_v.0, target.0)),
                (&mut self.lean.1, (&mut self.lean_v.1, target.1)),
            ] {
                *lv += (t - *l) / 6;
                *lv = *lv * 82 / 100;
                *l += *lv / 4;
                *l = (*l).clamp(-deg(60), deg(60));
            }
            if self.stun > 0 {
                self.lean_v.0 += deg(2) * if self.stun % 8 < 4 { 1 } else { -1 };
            }
        }
        // turning: toward the wanted heading, plus any spin from hits
        let turn = fx::wrap(face - self.yaw);
        let rate = if self.kind.walks() { deg(3) } else { deg(5) };
        self.yaw = fx::wrap(self.yaw + turn.clamp(-rate, rate) + self.spin / TICKS_PER_SECOND);
        self.spin = self.spin * 93 / 100;
        let _ = r;
        self.integrate(dt, map, extra);
    }

    pub fn radius(&self, d: &DroneDef) -> i32 {
        cm(d.radius)
    }

    fn floor(&self, map: &Map, extra: &[Aabb]) -> i32 {
        let b = Aabb::standing(self.pos, cm(30), cm(50));
        map.floor_under(&b, self.pos.y + cm(40), extra)
    }

    fn integrate(&mut self, dt: i32, map: &Map, extra: &[Aabb]) {
        let step = self.vel.scale(dt);
        let (half, h) = if self.kind.walks() {
            (
                if self.kind == Kind::Strider {
                    cm(110)
                } else {
                    cm(40)
                },
                cm(80),
            )
        } else {
            (cm(45), cm(50))
        };
        let lift = if self.kind.walks() { 0 } else { cm(25) };
        let body = |p: V3| Aabb::standing(v3(p.x, p.y - lift, p.z), half, h);
        for axis in 0..3 {
            let mut next = self.pos;
            match axis {
                0 => next.x += step.x,
                1 => next.y += step.y,
                _ => next.z += step.z,
            }
            if !map.blocked(&body(next), extra) {
                self.pos = next;
                continue;
            }
            if self.kind.walks() && axis != 1 {
                // walkers climb low things
                let up = v3(next.x, next.y + cm(60), next.z);
                if !map.blocked(&body(up), extra) {
                    self.pos = v3(next.x, self.pos.y + cm(20), next.z);
                    continue;
                }
            }
            // bounce off what it met, a little
            match axis {
                0 => self.vel.x = -self.vel.x / 3,
                1 => self.vel.y = -self.vel.y / 4,
                _ => self.vel.z = -self.vel.z / 3,
            }
        }
        if self.kind.walks() {
            let f = map.floor_under(&body(self.pos), self.pos.y + cm(30), extra);
            if self.pos.y <= f {
                self.pos.y = f;
                self.vel.y = 0;
            }
        } else if self.pos.y < cm(60) {
            self.pos.y = cm(60);
            self.vel.y = self.vel.y.max(0);
        }
    }
}

/// Where a ray `o + d·t` first meets a sphere, if within `max_t`.
pub fn sphere(o: V3, d: V3, c: V3, r: i32, max_t: i32) -> Option<i32> {
    let oc = c.sub(o);
    let t = oc.dot(d);
    if t < -r || t > max_t + r {
        return None;
    }
    let closest = o.add(d.scale(t));
    let off = closest.sub(c).len();
    if off > r {
        return None;
    }
    let back = fx::isqrt((r as i64 * r as i64 - off as i64 * off as i64).max(0) as u64) as i32;
    let t0 = t - back;
    if t0 > max_t {
        return None;
    }
    Some(t0.max(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::content;

    fn wasp() -> (Drone, DroneDef) {
        let c = content();
        let i = c.drone("wasp");
        let d = c.drones[i].clone();
        (Drone::new(Kind::Wasp, i, &d, v3(0, int(3), 0), 0, V3::ZERO), d)
    }

    #[test]
    fn a_shot_from_behind_finds_the_weak_battery_and_from_the_front_the_armour() {
        let (w, _) = wasp();
        // facing −z; the battery is at +z
        let from_behind = w.ray(v3(0, int(3), int(10)), v3(0, 0, -ONE), int(20));
        assert_eq!(from_behind.map(|h| h.1), Some(Zone::Weak));
        let from_front = w.ray(v3(0, int(3), -int(10)), v3(0, 0, ONE), int(20));
        assert_eq!(from_front.map(|h| h.1), Some(Zone::Armour));
        assert!(w
            .ray(v3(int(5), int(3), -int(10)), v3(0, 0, ONE), int(20))
            .is_none());
    }

    #[test]
    fn weak_spots_take_more_and_goo_eats_armour() {
        let (mut w, d) = wasp();
        let weak = w
            .clone()
            .strike(&d, Zone::Weak, 10, v3(0, 0, -ONE), w.pos, 0)
            .dealt;
        let armour = w
            .clone()
            .strike(&d, Zone::Armour, 10, v3(0, 0, -ONE), w.pos, 0)
            .dealt;
        assert!(weak > 10 && armour < 10, "{weak} {armour}");
        w.apply_effect(Effect::Goo, 100, 0);
        w.apply_effect(Effect::Goo, 100, 0);
        let gooed = w.strike(&d, Zone::Armour, 10, v3(0, 0, -ONE), w.pos, 0).dealt;
        assert!(gooed > armour, "{gooed} {armour}");
        assert!(w.speed_pct() < 100);
    }

    #[test]
    fn a_hit_shoves_and_rocks_a_flyer_and_losing_rotors_brings_it_down() {
        let (mut w, d) = wasp();
        let at = w.to_world(30, 0, 0);
        w.strike(&d, Zone::Body, 20, v3(0, 0, -ONE), at, 0);
        assert!(w.vel.z < 0, "pushed along the shot");
        assert!(w.spin != 0 || w.lean_v != (0, 0), "an off-centre hit turns it");
        let s = w.strike(&d, Zone::Rotor(0), 80, v3(0, 0, -ONE), at, 0);
        assert!(s.rotor_lost && !s.killed);
        let s = w.strike(&d, Zone::Rotor(1), 80, v3(0, 0, -ONE), at, 0);
        assert!(s.killed, "two of four rotors gone: it falls");
        assert!(matches!(w.state, State::Falling(_)));
    }

    #[test]
    fn a_flyer_leans_into_its_acceleration() {
        let (mut w, d) = wasp();
        let m = Map::new(v3(-int(50), -int(5), -int(50)), v3(int(50), int(20), int(50)));
        for _ in 0..20 {
            w.step_body(&d, v3(0, 0, -int(8)), 0, &m, &[]);
        }
        assert!(w.lean.0 > 0, "nose down going forward: {}", w.lean.0);
        assert!(w.pos.z < 0);
    }
}
