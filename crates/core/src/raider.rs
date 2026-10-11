//! A raider: how one moves, aims and holds a gun, and the shape of the
//! third-person camera behind them. Movement is Long Time Comin's habit:
//! one axis at a time, stopping at whatever is met, stepping up anything low
//! enough to walk onto.

use crate::balance::*;
use crate::fx::{self, ONE};
use crate::geom::{facing, right_of, v3, Affine, V3};
use crate::world::{Aabb, Map};

/// Buttons, one bit each. data/controls.json names keys for the page's
/// actions; the game turns those into these.
pub const B_FWD: u32 = 1;
pub const B_BACK: u32 = 2;
pub const B_LEFT: u32 = 4;
pub const B_RIGHT: u32 = 8;
pub const B_FIRE: u32 = 16;
pub const B_AIM: u32 = 32;
pub const B_RELOAD: u32 = 64;
pub const B_USE: u32 = 128;
pub const B_SPRINT: u32 = 256;
pub const B_JUMP: u32 = 512;
/// this raider's person has left the game
pub const B_GONE: u32 = 1 << 15;
pub const B_CROUCH: u32 = 1 << 16;
pub const B_MELEE: u32 = 1 << 17;
pub const B_ROLL: u32 = 1 << 18;
pub const B_GADGET: u32 = 1 << 19;
/// Every bit a raider's input carries.
pub const B_ALL: u32 = 0x3ff | B_CROUCH | B_MELEE | B_ROLL | B_GADGET;

pub const CROUCH_H: i32 = ONE * 115 / 100;
const ROLL_TICKS: i32 = secs(380);
const ROLL_AGAIN: i32 = secs(1200);
const ROLL_SPEED: i32 = ONE * 85 / 10;
const SLIDE_TICKS: i32 = secs(650);

/// One raider's input on one tick: the held buttons and where they look.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct Input {
    #[serde(default)]
    pub b: u32,
    #[serde(default)]
    pub y: i32,
    #[serde(default)]
    pub p: i32,
    /// drop the pack item in this slot, plus one (0: drop nothing)
    #[serde(default, skip_serializing_if = "is_zero")]
    pub d: u8,
}

fn is_zero(v: &u8) -> bool {
    *v == 0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Life {
    Alive,
    /// down, bleeding out; a teammate can bring them back
    Downed,
    Dead,
    /// in the van when it left
    Extracted,
    /// left behind when the van went
    Left,
    /// their person quit
    Gone,
}

impl Life {
    pub fn name(self) -> &'static str {
        match self {
            Life::Alive => "alive",
            Life::Downed => "downed",
            Life::Dead => "dead",
            Life::Extracted => "extracted",
            Life::Left => "left",
            Life::Gone => "gone",
        }
    }

    pub fn active(self) -> bool {
        matches!(self, Life::Alive | Life::Downed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raider {
    pub pos: V3,
    pub vel: V3,
    pub yaw: i32,
    pub pitch: i32,
    pub grounded: bool,
    pub hp: i32,
    pub life: Life,
    pub bleed: i32,
    pub since_hurt: i32,
    pub regen_t: i32,
    pub prev: u32,
    pub ammo: i32,
    pub cooldown: i32,
    pub reloading: i32,
    pub spin: i32,
    pub next_muzzle: usize,
    /// what the E key is being held for, and for how long
    pub use_t: i32,
    pub use_on: i32,
    pub stride: i32,
    pub moving: bool,
    pub aiming: bool,
    pub sprinting: bool,
    pub firing: i32,
    pub kills: i32,
    pub hurt_flash: i32,
    pub max_hp: i32,
    pub crouch: bool,
    pub roll_t: i32,
    pub roll_cd: i32,
    pub slide_t: i32,
    /// the way a roll or a slide carries them, at its speed
    pub dash: V3,
    /// a melee swing: ticks into it (0: none), and ticks until the next
    pub melee_t: i32,
    pub melee_cd: i32,
    pub charges: i32,
    /// they landed from a jump this tick (a thump drones hear)
    pub landed: bool,
}

impl Raider {
    pub fn new(pos: V3, yaw: i32, ammo: i32) -> Raider {
        Raider {
            pos,
            vel: V3::ZERO,
            yaw,
            pitch: 0,
            grounded: true,
            hp: RAIDER_HP,
            life: Life::Alive,
            bleed: 0,
            since_hurt: REGEN_AFTER,
            regen_t: 0,
            prev: 0,
            ammo,
            cooldown: 0,
            reloading: 0,
            spin: 0,
            next_muzzle: 0,
            use_t: 0,
            use_on: -1,
            stride: 0,
            moving: false,
            aiming: false,
            sprinting: false,
            firing: 0,
            kills: 0,
            hurt_flash: 0,
            max_hp: RAIDER_HP,
            crouch: false,
            roll_t: 0,
            roll_cd: 0,
            slide_t: 0,
            dash: V3::ZERO,
            melee_t: 0,
            melee_cd: 0,
            charges: 0,
            landed: false,
        }
    }

    /// Where the third-person camera's pivot sits above the feet.
    pub fn eye_up(&self) -> i32 {
        match self.height() {
            h if h < CROUCH_H => ONE * 9 / 10,
            CROUCH_H => CAM_UP - ONE * 45 / 100,
            _ => CAM_UP,
        }
    }

    /// How tall they stand now.
    pub fn height(&self) -> i32 {
        if self.life == Life::Downed {
            ONE * 6 / 10
        } else if self.crouch || self.roll_t > 0 || self.slide_t > 0 {
            CROUCH_H
        } else {
            RAIDER_H
        }
    }

    pub fn body(&self) -> Aabb {
        Aabb::standing(self.pos, RAIDER_HALF, self.height())
    }

    pub fn chest(&self) -> V3 {
        let h = match self.height() {
            h if h < CROUCH_H => ONE * 4 / 10,
            CROUCH_H => ONE * 8 / 10,
            _ => ONE * 13 / 10,
        };
        v3(self.pos.x, self.pos.y + h, self.pos.z)
    }

    pub fn head(&self) -> V3 {
        let h = self.height() - ONE / 10;
        v3(self.pos.x, self.pos.y + h, self.pos.z)
    }

    /// Takes damage; true if it put them down.
    pub fn hurt(&mut self, dmg: i32) -> bool {
        if self.life != Life::Alive {
            if self.life == Life::Downed {
                self.bleed = (self.bleed - dmg * 6).max(1);
            }
            return false;
        }
        self.hp -= dmg;
        self.since_hurt = 0;
        self.hurt_flash = 12;
        if self.hp <= 0 {
            self.hp = 0;
            self.life = Life::Downed;
            self.bleed = BLEED_OUT;
            self.reloading = 0;
            return true;
        }
        false
    }

    /// Movement for one tick from held buttons, against the map and the
    /// extra boxes (the van). `speed_pct` is the gun's weight.
    pub fn step_move(&mut self, inp: &Input, map: &Map, extra: &[Aabb], speed_pct: i32) {
        self.step_move_kit(inp, map, extra, speed_pct, &crate::kit::NEUTRAL);
    }

    /// Movement for one tick, with what the gear changes: speeds, the jump,
    /// a roll, a slide, how fast they move crouched.
    pub fn step_move_kit(
        &mut self,
        inp: &Input,
        map: &Map,
        extra: &[Aabb],
        speed_pct: i32,
        kit: &crate::kit::Moves,
    ) {
        let held = |b: u32| inp.b & b != 0;
        let pressed = |b: u32| inp.b & b != 0 && self.prev & b == 0;
        let alive = self.life == Life::Alive;
        self.roll_t = (self.roll_t - 1).max(0);
        self.roll_cd = (self.roll_cd - 1).max(0);
        self.slide_t = (self.slide_t - 1).max(0);
        // crouch while held; stand only where there is room overhead
        if held(B_CROUCH) && alive {
            if !self.crouch && kit.slide && self.sprinting && self.grounded && self.vel.len_xz() > WALK {
                self.slide_t = SLIDE_TICKS;
                self.dash = v3(self.vel.x, 0, self.vel.z).norm().scale(SPRINT * 13 / 10);
            }
            self.crouch = true;
        } else if self.crouch && !map.blocked(&Aabb::standing(self.pos, RAIDER_HALF, RAIDER_H), extra) {
            self.crouch = false;
        }
        self.yaw = inp.y;
        self.pitch = inp.p.clamp(-MAX_PITCH, MAX_PITCH);
        self.aiming = held(B_AIM) && self.life == Life::Alive;
        let mut mx = 0;
        let mut mz = 0;
        if held(B_FWD) {
            mz += ONE;
        }
        if held(B_BACK) {
            mz -= ONE;
        }
        if held(B_RIGHT) {
            mx += ONE;
        }
        if held(B_LEFT) {
            mx -= ONE;
        }
        self.sprinting = held(B_SPRINT) && mz > 0 && !self.aiming && !self.crouch && alive;
        let crouch_pct = if kit.crouch_full { 90 } else { 50 };
        let top = match self.life {
            Life::Downed => CRAWL,
            _ if self.crouch => WALK * crouch_pct / 100,
            _ if self.aiming => AIM_WALK,
            _ if self.sprinting => SPRINT * kit.sprint_pct / 100,
            _ => WALK,
        } * speed_pct
            / 100
            * kit.speed_pct
            / 100;
        let fwd = facing(self.yaw, 0);
        let right = right_of(self.yaw);
        let wish = fwd.scale(mz).add(right.scale(mx));
        let wish = if wish.len() > 0 {
            wish.norm().scale(top)
        } else {
            V3::ZERO
        };
        // a roll: a quick dash the way they are moving, ducked
        if kit.roll && pressed(B_ROLL) && self.roll_cd == 0 && self.grounded && alive {
            let dir = if wish.len() > 0 { wish.norm() } else { fwd };
            self.roll_t = ROLL_TICKS;
            self.roll_cd = ROLL_AGAIN;
            self.dash = dir.scale(ROLL_SPEED);
        }
        let dv = ACCEL / TICKS_PER_SECOND;
        if self.roll_t > 0 || self.slide_t > 0 {
            // carried by the dash; a slide slows as it goes
            let k = if self.slide_t > 0 {
                ONE * self.slide_t / SLIDE_TICKS / 2 + ONE / 2
            } else {
                ONE
            };
            self.vel.x = fx::mul(self.dash.x, k);
            self.vel.z = fx::mul(self.dash.z, k);
        } else {
            self.vel.x = fx::approach(self.vel.x, wish.x, dv);
            self.vel.z = fx::approach(self.vel.z, wish.z, dv);
        }
        let was_grounded = self.grounded;
        if pressed(B_JUMP) && self.grounded && alive && !self.crouch {
            // twice as high asks for about root two the speed: half the percentage over
            self.vel.y = JUMP * (100 + (kit.jump_pct - 100) / 2) / 100;
            self.grounded = false;
        }
        self.vel.y -= GRAVITY / TICKS_PER_SECOND;
        self.moving = wish.len() > 0;
        if self.moving && self.grounded {
            // about one and a half strides a second at a walk, two and a half sprinting
            self.stride = (self.stride + self.vel.len_xz() / STRIDE_PER) & (fx::TURN - 1);
        }
        let step = self.vel.scale(ONE / TICKS_PER_SECOND);
        self.move_axis(0, step.x, map, extra);
        self.move_axis(2, step.z, map, extra);
        // vertical
        let mut next = self.pos;
        next.y += step.y;
        let b = Aabb::standing(next, RAIDER_HALF, self.height());
        if self.vel.y <= 0 {
            let floor = map.floor_under(&b, self.pos.y + STEP_UP / 4, extra);
            if next.y <= floor {
                next.y = floor;
                self.vel.y = 0;
                self.grounded = true;
            } else {
                self.grounded = false;
            }
            self.pos = next;
        } else if map.blocked(&b, extra) {
            self.vel.y = 0;
        } else {
            self.pos = next;
            self.grounded = false;
        }
        self.landed = self.grounded && !was_grounded;
    }

    fn move_axis(&mut self, axis: usize, d: i32, map: &Map, extra: &[Aabb]) {
        if d == 0 {
            return;
        }
        let mut next = self.pos;
        if axis == 0 {
            next.x += d;
        } else {
            next.z += d;
        }
        let h = self.height();
        if !map.blocked(&Aabb::standing(next, RAIDER_HALF, h), extra) {
            self.pos = next;
            return;
        }
        // a step up onto something low
        if self.grounded {
            let up = v3(next.x, next.y + STEP_UP, next.z);
            if !map.blocked(&Aabb::standing(up, RAIDER_HALF, h), extra) {
                let floor = map.floor_under(&Aabb::standing(up, RAIDER_HALF, h), up.y, extra);
                self.pos = v3(next.x, floor.max(self.pos.y), next.z);
                return;
            }
        }
        if axis == 0 {
            self.vel.x = 0;
        } else {
            self.vel.z = 0;
        }
    }
}

/// Where the third-person camera sits for a raider looking along yaw and
/// pitch: over the right shoulder, pulled in front of any wall. Returns the
/// eye and its forward direction.
pub fn camera(
    pos: V3,
    yaw: i32,
    pitch: i32,
    aiming: bool,
    downed: bool,
    map: &Map,
    extra: &[Aabb],
) -> (V3, V3) {
    camera_at(
        pos,
        yaw,
        pitch,
        aiming,
        if downed { ONE * 9 / 10 } else { CAM_UP },
        map,
        extra,
    )
}

/// The camera for a raider whose eyes are `up` above their feet (lower
/// crouched, lower still when down).
pub fn camera_at(
    pos: V3,
    yaw: i32,
    pitch: i32,
    aiming: bool,
    up: i32,
    map: &Map,
    extra: &[Aabb],
) -> (V3, V3) {
    let fwd = facing(yaw, pitch);
    let pivot = v3(pos.x, pos.y + up, pos.z).add(right_of(yaw).scale(CAM_RIGHT));
    let back = if aiming { CAM_BACK_AIM } else { CAM_BACK };
    let want = pivot.sub(fwd.scale(back)).add(v3(0, ONE / 5, 0));
    let d = want.sub(pivot);
    let len = d.len();
    let dir = d.norm();
    let mut eye = match map.ray(pivot, dir, len + ONE / 4, extra, false) {
        Some(h) => pivot.add(dir.scale((h.t - ONE * 3 / 10).max(ONE / 5))),
        None => want,
    };
    // keep a little room round the lens, pulling in toward the head
    for _ in 0..6 {
        let room = Aabb::new(
            eye.sub(v3(ONE / 4, ONE / 4, ONE / 4)),
            eye.add(v3(ONE / 4, ONE / 4, ONE / 4)),
        );
        if !map.blocked(&room, extra) {
            break;
        }
        eye = eye.lerp(pivot, ONE * 35 / 100);
    }
    (eye, fwd)
}

/// Where a raider's gun hand is, and the gun's frame (columns: right, up,
/// back) as an affine transform from gun space to the world.
pub fn gun_frame(r: &Raider) -> Affine {
    let fwd = facing(r.yaw, r.pitch);
    let right = right_of(r.yaw);
    let up = right.cross(fwd).neg().norm();
    let up = if up.y < 0 { up.neg() } else { up };
    let crouched = if r.crouch || r.roll_t > 0 || r.slide_t > 0 {
        ONE * 35 / 100
    } else {
        0
    };
    let lift = if r.life == Life::Downed {
        ONE / 3
    } else if r.aiming {
        ONE * 14 / 10 - crouched
    } else {
        ONE * 12 / 10 - crouched
    };
    let side = if r.aiming { ONE / 6 } else { ONE * 3 / 10 };
    let hand = v3(r.pos.x, r.pos.y + lift, r.pos.z)
        .add(right.scale(side))
        .add(facing(r.yaw, 0).scale(ONE * 3 / 10));
    Affine {
        c0: right,
        c1: up,
        c2: fwd.neg(),
        t: hand,
    }
}

/// Speed (Q16 m/s) per angle unit of the walk cycle a tick.
pub const STRIDE_PER: i32 = 170;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fx::int;
    use crate::world::{Look, Solid};

    fn flat() -> Map {
        let mut m = Map::new(v3(-int(50), -int(5), -int(50)), v3(int(50), int(20), int(50)));
        // a wall ahead at z = −5 and a low step to the right
        m.add(Solid {
            b: Aabb::new(v3(-int(5), 0, -int(6)), v3(int(5), int(3), -int(5))),
            rgb: [0; 3],
            look: Look::Plain,
            open: false,
        });
        m.add(Solid {
            b: Aabb::new(v3(int(2), 0, -int(2)), v3(int(6), ONE * 3 / 10, int(2))),
            rgb: [0; 3],
            look: Look::Plain,
            open: false,
        });
        m
    }

    #[test]
    fn a_raider_walks_forward_and_stops_at_a_wall() {
        let m = flat();
        let mut r = Raider::new(V3::ZERO, 0, 10);
        let inp = Input {
            b: B_FWD,
            y: 0,
            p: 0,
            d: 0,
        };
        for _ in 0..240 {
            r.step_move(&inp, &m, &[], 100);
            r.prev = inp.b;
        }
        assert!(r.pos.z < -int(4) && r.pos.z > -int(5), "{}", r.pos.z);
    }

    #[test]
    fn walking_legs_swing_at_a_human_pace() {
        let m = flat();
        let mut r = Raider::new(v3(0, 0, int(20)), 0, 10);
        let inp = Input {
            b: B_FWD,
            y: 0,
            p: 0,
            d: 0,
        };
        let mut cycles = 0i64;
        let mut last = r.stride;
        for _ in 0..60 {
            r.step_move(&inp, &m, &[], 100);
            cycles += (r.stride - last).rem_euclid(fx::TURN) as i64;
            last = r.stride;
        }
        // strides in one second at a walk: between one and two full cycles
        assert!(
            cycles > fx::TURN as i64 && cycles < 2 * fx::TURN as i64,
            "{cycles}"
        );
    }

    #[test]
    fn a_raider_steps_up_a_low_box_and_jumps_and_lands() {
        let m = flat();
        let mut r = Raider::new(V3::ZERO, 0, 10);
        let right = Input {
            b: B_RIGHT,
            y: 0,
            p: 0,
            d: 0,
        };
        for _ in 0..60 {
            r.step_move(&right, &m, &[], 100);
        }
        assert_eq!(r.pos.y, ONE * 3 / 10, "on the step");
        let jump = Input {
            b: B_JUMP,
            y: 0,
            p: 0,
            d: 0,
        };
        r.prev = 0;
        r.step_move(&jump, &m, &[], 100);
        r.prev = B_JUMP;
        assert!(r.pos.y > ONE * 3 / 10);
        for _ in 0..120 {
            r.step_move(&jump, &m, &[], 100);
        }
        assert_eq!(r.pos.y, ONE * 3 / 10, "landed again");
    }

    #[test]
    fn the_camera_sits_behind_and_is_pulled_in_by_a_wall() {
        let m = flat();
        let (eye, _) = camera(v3(0, 0, int(10)), 0, 0, false, false, &m, &[]);
        assert!(eye.z > int(12), "behind (+z when facing −z): {}", eye.z);
        let (eye, _) = camera(v3(0, 0, -ONE * 3), fx::HALF, 0, false, false, &m, &[]);
        assert!(eye.z > -int(5), "kept in front of the wall: {}", eye.z);
    }
}
