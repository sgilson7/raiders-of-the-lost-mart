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
    /// a shell that cannot come off: it soaks most of a hit
    Armour,
    Weak,
    /// one of the drone's breakable parts (`pieces`), by index
    Piece(usize),
}

/// What a breakable part is, which decides what losing it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PieceKind {
    /// a flyer loses lift and drags toward the dead corner
    Rotor,
    /// a plate: soaks hits like armour until it is torn away
    Plate,
    /// the guns: without them a drone rams, flees or stomps
    Gun,
    /// a walker's leg: it limps, and with two or fewer it cannot walk
    Leg,
    /// the moth's beacon: without it, it cannot call the others
    Beacon,
    /// the moth's wing: it flies lopsided and slow, and with both gone it falls
    Wing,
}

impl PieceKind {
    pub fn name(self) -> &'static str {
        match self {
            PieceKind::Rotor => "rotor",
            PieceKind::Plate => "plate",
            PieceKind::Gun => "gun",
            PieceKind::Leg => "leg",
            PieceKind::Beacon => "beacon",
            PieceKind::Wing => "wing",
        }
    }

    /// How much of a hit on this part reaches the body, in percent.
    fn passes(self) -> i32 {
        match self {
            PieceKind::Rotor => 25,
            PieceKind::Plate | PieceKind::Wing => 0,
            PieceKind::Gun | PieceKind::Beacon => 50,
            PieceKind::Leg => 35,
        }
    }
}

/// A breakable part: what it is, its hit sphere in the drone's own space
/// (cm: offset, radius), and its health as a share of the drone's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PieceDef {
    pub kind: PieceKind,
    pub at: (i32, i32, i32),
    pub r: i32,
    pub hp_pct: i32,
}

const fn pc(kind: PieceKind, x: i32, y: i32, z: i32, r: i32, hp_pct: i32) -> PieceDef {
    PieceDef {
        kind,
        at: (x, y, z),
        r,
        hp_pct,
    }
}

use PieceKind::*;

/// The most parts any drone has.
pub const MAX_PIECES: usize = 10;

/// Each drone's breakable parts. Rotors come first, so rotor `i` is part
/// `i`; a walker's legs come first, left side then right.
pub fn pieces(kind: Kind) -> &'static [PieceDef] {
    match kind {
        Kind::Wasp => {
            const P: &[PieceDef] = &[
                pc(Rotor, -44, 14, -44, 20, 50),
                pc(Rotor, 44, 14, -44, 20, 50),
                pc(Rotor, -44, 14, 44, 20, 50),
                pc(Rotor, 44, 14, 44, 20, 50),
                pc(Plate, 0, 0, -30, 26, 60),
                pc(Gun, 0, -22, -36, 12, 40),
            ];
            P
        }
        Kind::Moth => {
            const P: &[PieceDef] = &[
                pc(Rotor, -60, -12, -30, 18, 50),
                pc(Rotor, 60, -12, -30, 18, 50),
                pc(Rotor, -60, -12, 30, 18, 50),
                pc(Rotor, 60, -12, 30, 18, 50),
                pc(Wing, -78, 6, 0, 40, 80),
                pc(Wing, 78, 6, 0, 40, 80),
                pc(Beacon, 0, 32, -10, 12, 30),
            ];
            P
        }
        Kind::Hornet => {
            const P: &[PieceDef] = &[
                pc(Rotor, 0, 28, -40, 30, 45),
                pc(Rotor, 0, 28, 85, 30, 45),
                pc(Plate, 0, 0, -45, 22, 50),
                pc(Gun, 0, -8, -66, 12, 50),
            ];
            P
        }
        Kind::Warden => {
            const P: &[PieceDef] = &[
                pc(Rotor, -125, 20, -125, 44, 22),
                pc(Rotor, 125, 20, -125, 44, 22),
                pc(Rotor, -125, 20, 125, 44, 22),
                pc(Rotor, 125, 20, 125, 44, 22),
                pc(Plate, 0, 30, -78, 45, 30),
                pc(Plate, 0, -60, 0, 52, 30),
                pc(Gun, 0, -30, -92, 22, 25),
            ];
            P
        }
        Kind::Tick => {
            const P: &[PieceDef] = &[
                pc(Leg, -40, 25, -30, 13, 30),
                pc(Leg, -40, 25, 0, 13, 30),
                pc(Leg, -40, 25, 30, 13, 30),
                pc(Leg, 40, 25, -30, 13, 30),
                pc(Leg, 40, 25, 0, 13, 30),
                pc(Leg, 40, 25, 30, 13, 30),
                pc(Plate, 0, 58, 0, 38, 60),
            ];
            P
        }
        Kind::Strider => {
            const P: &[PieceDef] = &[
                pc(Leg, -110, 110, -70, 24, 25),
                pc(Leg, -110, 110, 0, 24, 25),
                pc(Leg, -110, 110, 70, 24, 25),
                pc(Leg, 110, 110, -70, 24, 25),
                pc(Leg, 110, 110, 0, 24, 25),
                pc(Leg, 110, 110, 70, 24, 25),
                pc(Gun, 0, 245, -25, 36, 35),
                pc(Plate, 0, 215, -30, 60, 40),
            ];
            P
        }
    }
}

/// The parts of a drone that cannot come off: hit spheres (cm: offset,
/// radius) and what each is.
pub fn zones(kind: Kind) -> &'static [(i32, i32, i32, i32, Zone)] {
    match kind {
        Kind::Wasp => &[(0, 0, 0, 32, Zone::Body), (0, 4, 34, 17, Zone::Weak)],
        Kind::Moth => &[(0, 18, 0, 20, Zone::Weak), (0, 0, 0, 34, Zone::Body)],
        Kind::Hornet => &[
            (0, -8, -98, 16, Zone::Weak),
            (0, 0, 0, 26, Zone::Body),
            (0, 0, 45, 22, Zone::Body),
        ],
        // the belly plate covers the core: it shows once the plate is gone
        Kind::Warden => &[(0, -62, 0, 36, Zone::Weak), (0, 0, 0, 88, Zone::Armour)],
        Kind::Tick => &[(0, 18, 0, 22, Zone::Weak), (0, 40, 0, 30, Zone::Body)],
        Kind::Strider => &[(0, 140, 95, 55, Zone::Weak), (0, 165, -20, 88, Zone::Armour)],
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
    /// the health of each breakable part (0: torn off), as `pieces` lists them
    pub pieces: [i32; MAX_PIECES],
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
    /// a part this hit tore off
    pub lost: Option<(usize, PieceKind)>,
}

pub fn cm(n: i32) -> i32 {
    n * ONE / 100
}

impl Drone {
    pub fn new(kind: Kind, def: usize, d: &DroneDef, pos: V3, yaw: i32, home: V3) -> Drone {
        let mut parts = [0; MAX_PIECES];
        for (i, p) in pieces(kind).iter().enumerate() {
            parts[i] = match p.kind {
                Rotor if d.rotor_hp > 0 => d.rotor_hp,
                _ => (d.hp * p.hp_pct / 100).max(1),
            };
        }
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
            pieces: parts,
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
            if let Some(t) = sphere(o, d, self.to_world(x, y, z), cm(r), max_t) {
                // weak spots win ties with what wraps them: they are inset
                let t = if zone == Zone::Weak { t - cm(6) } else { t };
                if best.is_none_or(|b| t < b.0) {
                    best = Some((t, zone));
                }
            }
        }
        for (i, p) in pieces(self.kind).iter().enumerate() {
            if self.pieces[i] <= 0 {
                continue;
            }
            let (x, y, z) = p.at;
            if let Some(t) = sphere(o, d, self.to_world(x, y, z), cm(p.r), max_t) {
                // a plate in front of a weak spot shields it
                let t = if p.kind == Plate { t - cm(8) } else { t };
                if best.is_none_or(|b| t < b.0) {
                    best = Some((t, Zone::Piece(i)));
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
            lost: None,
        };
        if !self.active() {
            return out;
        }
        let armour = (d.armour_pct + self.corrode).min(100);
        let dealt = match zone {
            Zone::Body => dmg,
            Zone::Armour => dmg * armour / 100,
            Zone::Weak => dmg * d.weak_pct / 100,
            Zone::Piece(i) => {
                let kind = pieces(self.kind)[i].kind;
                // goo eats plates as it eats armour
                let into = if kind == Plate || kind == Wing {
                    dmg * (100 + self.corrode) / 100
                } else {
                    dmg
                };
                let before = self.pieces[i];
                self.pieces[i] -= into;
                if before > 0 && self.pieces[i] <= 0 {
                    self.pieces[i] = 0;
                    out.lost = Some((i, kind));
                    // tearing a part off jolts the whole drone
                    self.lean_v.0 += deg(6);
                    self.spin += deg(30);
                }
                match kind {
                    Plate | Wing => dmg * armour / 100 / 3,
                    _ => dmg * kind.passes() / 100,
                }
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
        let rotors = self.kind.rotors();
        let dead_rotors = self.pieces[..rotors].iter().filter(|r| **r <= 0).count();
        let wingless = self.kind == Kind::Moth && self.lost_count(Wing) >= 2;
        if self.hp <= 0 || (rotors > 0 && dead_rotors >= rotors.div_ceil(2)) || wingless {
            self.hp = self.hp.max(0);
            self.state = State::Falling(if self.kind.walks() { secs(600) } else { FALL_TIME });
            self.spin += deg(12);
            out.killed = true;
        }
        out
    }

    /// A blast at `at`: every part within reach takes its share; returns the
    /// parts it tore off and where they were.
    pub fn blast(&mut self, at: V3, radius: i32, dmg: i32) -> Vec<(V3, PieceKind)> {
        let mut torn = Vec::new();
        if !self.active() {
            return torn;
        }
        for (i, p) in pieces(self.kind).iter().enumerate() {
            if self.pieces[i] <= 0 {
                continue;
            }
            let (x, y, z) = p.at;
            let w = self.to_world(x, y, z);
            let dist = w.dist(at);
            let reach = radius + cm(p.r);
            if dist > reach {
                continue;
            }
            let fall = ONE - fx::div(dist, reach.max(1)) / 2;
            self.pieces[i] -= fx::mul(dmg, fall);
            if self.pieces[i] <= 0 {
                self.pieces[i] = 0;
                torn.push((w, p.kind));
            }
        }
        if !torn.is_empty() {
            self.spin += deg(40);
            self.lean_v.0 += deg(8);
        }
        torn
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
            let alive = self.pieces[..n].iter().filter(|r| **r > 0).count() as i32;
            pct = pct * (40 + 60 * alive / n as i32) / 100;
        }
        if self.lost_count(Wing) > 0 {
            pct = pct * 65 / 100;
        }
        if self.kind.walks() {
            // a leg short limps; two or fewer and it cannot walk at all
            let legs = self.alive_count(Leg);
            pct = if legs <= 2 {
                0
            } else {
                pct * (25 + 75 * legs / 6) / 100
            };
        }
        pct
    }

    pub fn alive_count(&self, k: PieceKind) -> i32 {
        pieces(self.kind)
            .iter()
            .enumerate()
            .filter(|(i, p)| p.kind == k && self.pieces[*i] > 0)
            .count() as i32
    }

    pub fn lost_count(&self, k: PieceKind) -> i32 {
        pieces(self.kind)
            .iter()
            .enumerate()
            .filter(|(i, p)| p.kind == k && self.pieces[*i] <= 0)
            .count() as i32
    }

    /// Its guns are gone (a drone that never had any is not disarmed).
    pub fn disarmed(&self) -> bool {
        self.lost_count(Gun) > 0 && self.alive_count(Gun) == 0
    }

    /// A walker on two legs or fewer: it stays where it is.
    pub fn lame(&self) -> bool {
        self.kind.walks() && self.alive_count(Leg) <= 2
    }

    /// Legs lost on the left less those on the right: a walker lurches
    /// toward its short side.
    pub fn limp(&self) -> i32 {
        if !self.kind.walks() {
            return 0;
        }
        let side = |lo: usize| (lo..lo + 3).filter(|i| self.pieces[*i] <= 0).count() as i32;
        side(0) - side(3)
    }

    /// Health as a percentage, for the HUD.
    pub fn health_pct(&self) -> i32 {
        (self.hp.max(0) * 100 / self.max_hp.max(1)).min(100)
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
        // a walker short of legs on one side lurches toward it as it steps,
        // and its body sits tilted that way
        let limp = self.limp();
        if self.kind.walks() && limp != 0 && want.len_xz() > 0 {
            let side = crate::geom::right_of(self.yaw).scale(-limp * ONE / 2);
            want = want.add(side.scale(fx::sin(self.gait).abs() * 3));
        }
        if self.kind.walks() {
            self.lean.1 = fx::approach(self.lean.1, -limp * deg(9), deg(1));
        }
        // a moth with a wing gone flies lopsided, turning as it goes
        if self.kind == Kind::Moth && self.lost_count(PieceKind::Wing) > 0 {
            self.spin += deg(4) / TICKS_PER_SECOND;
            self.lean_v.1 += deg(1);
        }
        if self.kind.walks() {
            self.vel.x = fx::approach(self.vel.x, want.x, acc);
            self.vel.z = fx::approach(self.vel.z, want.z, acc);
            self.vel.y -= GRAVITY / TICKS_PER_SECOND;
            // a strider steps about twice a second, a tick's short legs faster
            self.gait = (self.gait + self.vel.len_xz() / 110) & (fx::TURN - 1);
        } else {
            self.vel.x = fx::approach(self.vel.x, want.x, acc);
            self.vel.y = fx::approach(self.vel.y, want.y, acc);
            self.vel.z = fx::approach(self.vel.z, want.z, acc);
            // a dead rotor drags its corner down and the drone sags
            let n = self.kind.rotors();
            let dead = self.pieces[..n].iter().filter(|r| **r <= 0).count() as i32;
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
        // the body a drone collides with spans its rotors: a warden whose
        // box was only its core pushed its rotor pods into the balconies
        let (half, h) = match self.kind {
            Kind::Strider => (cm(110), cm(80)),
            Kind::Tick => (cm(40), cm(80)),
            Kind::Warden => (cm(165), cm(90)),
            Kind::Moth => (cm(95), cm(50)),
            Kind::Hornet => (cm(60), cm(55)),
            Kind::Wasp => (cm(60), cm(50)),
        };
        let lift = if self.kind.walks() { 0 } else { cm(25) };
        let body = |p: V3| Aabb::standing(v3(p.x, p.y - lift, p.z), half, h);
        // already wedged in something (spawned or shoved there): let it move out
        let wedged = map.blocked(&body(self.pos), extra);
        for axis in 0..3 {
            let mut next = self.pos;
            match axis {
                0 => next.x += step.x,
                1 => next.y += step.y,
                _ => next.z += step.z,
            }
            if wedged || !map.blocked(&body(next), extra) {
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
        assert_eq!(from_front.map(|h| h.1), Some(Zone::Piece(4)), "the front plate");
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
        let s = w.strike(&d, Zone::Piece(0), 80, v3(0, 0, -ONE), at, 0);
        assert_eq!(s.lost, Some((0, PieceKind::Rotor)));
        assert!(!s.killed);
        let s = w.strike(&d, Zone::Piece(1), 80, v3(0, 0, -ONE), at, 0);
        assert!(s.killed, "two of four rotors gone: it falls");
        assert!(matches!(w.state, State::Falling(_)));
    }

    #[test]
    fn a_plate_torn_off_bares_what_it_covered() {
        let (mut w, d) = wasp();
        let ray = |w: &Drone| {
            w.ray(v3(0, int(3), -int(10)), v3(0, 0, ONE), int(20))
                .map(|h| h.1)
        };
        assert_eq!(ray(&w), Some(Zone::Piece(4)));
        let mut torn = None;
        for _ in 0..20 {
            let s = w.strike(&d, Zone::Piece(4), 10, v3(0, 0, ONE), w.pos, 0);
            torn = torn.or(s.lost);
        }
        assert_eq!(torn, Some((4, PieceKind::Plate)));
        assert_eq!(ray(&w), Some(Zone::Body), "the plate is gone, the body is open");
        assert!(w.active(), "a plate soaks most of what hits it");
    }

    #[test]
    fn a_shot_off_gun_disarms_and_lost_legs_lame_a_walker() {
        let (mut w, d) = wasp();
        assert!(!w.disarmed());
        w.strike(&d, Zone::Piece(5), 100, v3(0, 0, ONE), w.pos, 0);
        assert!(w.disarmed());
        let c = content();
        let i = c.drone("strider");
        let sd = c.drones[i].clone();
        let mut s = Drone::new(Kind::Strider, i, &sd, V3::ZERO, 0, V3::ZERO);
        let full = s.speed_pct();
        s.strike(&sd, Zone::Piece(0), 60, v3(ONE, 0, 0), s.pos, 0);
        assert!(s.speed_pct() < full && s.speed_pct() > 0, "a leg short: it limps");
        assert_ne!(s.limp(), 0, "and lurches toward the short side");
        for k in 1..4 {
            s.strike(&sd, Zone::Piece(k), 60, v3(ONE, 0, 0), s.pos, 0);
        }
        assert!(s.lame() && s.speed_pct() == 0, "two legs left: it cannot walk");
        assert!(s.active());
    }

    #[test]
    fn a_blast_tears_off_the_parts_near_it() {
        let (mut w, _) = wasp();
        let rotor = w.to_world(44, 14, 44);
        let torn = w.blast(rotor, ONE / 2, 200);
        assert!(torn.iter().any(|t| t.1 == PieceKind::Rotor), "{torn:?}");
        assert!(w.pieces[0] > 0, "the far rotor holds");
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
