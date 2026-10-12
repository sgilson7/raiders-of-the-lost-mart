//! What a drone decides: a behaviour tree per kind, run every few ticks.
//!
//! The tree is data (a nested `Bt` built from `&'static` slices) and is
//! ticked from the root each think: selectors try children in order until
//! one does not fail, sequences run children in order until one does not
//! succeed. Long actions keep their progress in the drone's `Brain` (its
//! blackboard), so a re-run of the tree picks up where the action was.
//!
//! The behaviours the design asks for:
//! * hovering and patrolling in patterns (every flyer's `Patrol`, a loop of
//!   waypoints with a slow bob);
//! * circling a raider while firing (the wasp's `Orbit`);
//! * peeking: duck into cover when seen, wait, pop out somewhere else,
//!   charge a shot, duck back (the hornet's `SeekCover`, `Peek`, `Charge`);
//! * baiting a raider out of cover: show itself, linger, then back away
//!   toward a waypoint where wasps it has called are waiting (the moth's
//!   `Call` and `Bait`, the wasps' `Ambush`);
//! * holding range with the armour turned to the threat (the warden),
//!   rushing and detonating (the tick), keeping distance, strafing and
//!   stomping (the strider);
//! * the queen: stamping on whoever is under her, launching wasps from her
//!   back, a rocket barrage from her shoulder pods and her eye cannon.

use crate::balance::*;
use crate::content::DroneDef;
use crate::drone::{Drone, Kind};
use crate::fx::{self, deg, int, ONE};
use crate::geom::{facing, right_of, v3, yaw_pitch_of, V3};
use crate::rng::Rng;
use crate::world::{Aabb, Map};

/// Ticks between a drone's thinks.
pub const THINK: u32 = THINK_EVERY;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Success,
    Failure,
    Running,
}

/// Conditions: questions about the drone and its world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum C {
    Stunned,
    HasTarget,
    Sees,
    /// the target is within this many metres
    Near(i32),
    /// the target can see this drone and is facing it
    Exposed,
    InCover,
    PeekDue,
    Charging,
    Armed,
    Ambushing,
    Baiting,
    /// the drone's target is out of its sight (in cover)
    TargetHidden,
    /// its guns have been shot off
    Disarmed,
    /// a walker down to two legs or fewer
    Lame,
    /// it noticed something it has not made out: a sound, a shape
    Suspicious,
}

/// Actions: each sets the drone's goal, heading or orders and says how it went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum A {
    Wobble,
    Patrol,
    Investigate,
    Approach,
    Orbit,
    Shoot,
    /// hover at this many metres from the target, turning about it slowly
    Hold(i32),
    /// back off to at least this many metres
    Keep(i32),
    SeekCover,
    Peek,
    Charge,
    Call,
    Bait,
    Flee,
    Ambush,
    Rush,
    Arm,
    Stomp,
    /// fly or run straight into the target, and bounce off
    Ram,
    /// stay put and turn to face the target (a walker that cannot walk)
    Anchor,
    /// go, slowly, to where it noticed something, and look about
    Search,
    /// the queen: a salvo of rockets from each pod she still has
    Barrage,
    /// the queen: launch wasps from her back
    Brood,
}

#[derive(Debug, Clone, Copy)]
pub enum Bt {
    Sel(&'static [Bt]),
    Seq(&'static [Bt]),
    If(C),
    Not(C),
    Do(A),
}

use Bt::*;

pub const WASP: Bt = Sel(&[
    Seq(&[If(C::Stunned), Do(A::Wobble)]),
    // its guns shot off, a wasp throws itself at you
    Seq(&[If(C::HasTarget), If(C::Disarmed), Do(A::Ram)]),
    Seq(&[If(C::Ambushing), Not(C::Sees), Do(A::Ambush)]),
    Seq(&[
        If(C::HasTarget),
        If(C::Sees),
        Sel(&[
            Seq(&[If(C::Near(26)), Do(A::Orbit), Do(A::Shoot)]),
            Do(A::Approach),
        ]),
    ]),
    Seq(&[If(C::HasTarget), Do(A::Investigate)]),
    Seq(&[If(C::Suspicious), Do(A::Search)]),
    Do(A::Patrol),
]);

pub const MOTH: Bt = Sel(&[
    Seq(&[If(C::Stunned), Do(A::Wobble)]),
    Seq(&[
        If(C::HasTarget),
        Sel(&[
            Seq(&[If(C::Baiting), Do(A::Bait)]),
            Seq(&[If(C::Sees), If(C::Near(12)), Do(A::Flee)]),
            Seq(&[If(C::Sees), Do(A::Call), Do(A::Hold(18))]),
            Seq(&[If(C::TargetHidden), Do(A::Call), Do(A::Bait)]),
        ]),
    ]),
    Seq(&[If(C::Suspicious), Do(A::Search)]),
    Do(A::Patrol),
]);

pub const HORNET: Bt = Sel(&[
    Seq(&[If(C::Stunned), Do(A::Wobble)]),
    // a hornet without its barrel has nothing left to do but keep away
    Seq(&[If(C::HasTarget), If(C::Disarmed), Do(A::Flee)]),
    Seq(&[
        If(C::HasTarget),
        Sel(&[
            Seq(&[If(C::Charging), Do(A::Charge)]),
            Seq(&[If(C::Exposed), Do(A::SeekCover)]),
            Seq(&[If(C::InCover), If(C::PeekDue), Do(A::Peek)]),
            Seq(&[If(C::InCover), Do(A::SeekCover)]),
            Seq(&[If(C::Sees), Do(A::Charge)]),
            Do(A::Investigate),
        ]),
    ]),
    Seq(&[If(C::Suspicious), Do(A::Search)]),
    Do(A::Patrol),
]);

pub const WARDEN: Bt = Sel(&[
    Seq(&[If(C::Stunned), Do(A::Wobble)]),
    Seq(&[If(C::HasTarget), If(C::Disarmed), Do(A::Ram)]),
    Seq(&[
        If(C::HasTarget),
        If(C::Sees),
        Sel(&[
            Seq(&[If(C::Near(8)), Do(A::Keep(16))]),
            Seq(&[Do(A::Hold(18)), Do(A::Shoot)]),
        ]),
    ]),
    Seq(&[If(C::HasTarget), Do(A::Investigate)]),
    Seq(&[If(C::Suspicious), Do(A::Search)]),
    Do(A::Patrol),
]);

pub const TICK: Bt = Sel(&[
    Seq(&[If(C::Armed), Do(A::Arm)]),
    Seq(&[If(C::Stunned), Do(A::Wobble)]),
    Seq(&[
        If(C::HasTarget),
        Sel(&[Seq(&[If(C::Near(2)), Do(A::Arm)]), Do(A::Rush)]),
    ]),
    Seq(&[If(C::Suspicious), Do(A::Search)]),
    Do(A::Patrol),
]);

pub const STRIDER: Bt = Sel(&[
    Seq(&[If(C::Stunned), Do(A::Wobble)]),
    // on two legs or fewer it plants itself and becomes a turret
    Seq(&[If(C::HasTarget), If(C::Lame), Do(A::Anchor), Do(A::Shoot)]),
    // its turret gone, it charges and stomps
    Seq(&[
        If(C::HasTarget),
        If(C::Disarmed),
        Sel(&[Seq(&[If(C::Near(5)), Do(A::Stomp)]), Do(A::Rush)]),
    ]),
    Seq(&[
        If(C::HasTarget),
        If(C::Sees),
        Sel(&[
            Seq(&[If(C::Near(5)), Do(A::Stomp)]),
            Seq(&[Do(A::Keep(15)), Do(A::Shoot)]),
        ]),
    ]),
    Seq(&[If(C::HasTarget), Do(A::Investigate)]),
    Seq(&[If(C::Suspicious), Do(A::Search)]),
    Do(A::Patrol),
]);

pub const QUEEN: Bt = Sel(&[
    // on two legs or fewer she squats where she is and keeps firing
    Seq(&[
        If(C::HasTarget),
        If(C::Lame),
        Do(A::Anchor),
        Sel(&[Do(A::Brood), Do(A::Barrage), Do(A::Shoot)]),
    ]),
    Seq(&[
        If(C::HasTarget),
        Sel(&[
            Seq(&[If(C::Near(11)), Do(A::Stomp)]),
            Seq(&[If(C::Sees), Do(A::Brood)]),
            Seq(&[If(C::Sees), Do(A::Barrage)]),
            Seq(&[If(C::Sees), Do(A::Keep(24)), Do(A::Shoot)]),
            Do(A::Investigate),
        ]),
    ]),
    Seq(&[If(C::Suspicious), Do(A::Search)]),
    Do(A::Patrol),
]);

pub fn tree(kind: Kind) -> &'static Bt {
    match kind {
        Kind::Wasp => &WASP,
        Kind::Moth => &MOTH,
        Kind::Hornet => &HORNET,
        Kind::Warden => &WARDEN,
        Kind::Tick => &TICK,
        Kind::Strider => &STRIDER,
        Kind::Queen => &QUEEN,
    }
}

/// The drone's blackboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Brain {
    pub home: V3,
    pub patrol: Vec<V3>,
    pub patrol_i: usize,
    pub target: Option<u8>,
    pub seen_at: V3,
    pub seen_t: i32,
    pub sees: bool,
    /// where it wants to be and which way it wants to face
    pub goal: V3,
    pub face: i32,
    /// the waypoint path toward the goal, when the way is not clear
    pub path: Vec<V3>,
    pub path_goal: V3,
    pub replan: i32,
    pub orbit_dir: i32,
    pub burst: i32,
    pub gap: i32,
    pub reload: i32,
    pub charge: i32,
    pub fuse: i32,
    pub cover: Option<V3>,
    pub cover_t: i32,
    pub peek: Option<V3>,
    pub ambush: Option<V3>,
    pub bait: i32,
    pub bait_at: Option<V3>,
    pub called: i32,
    pub stomp: i32,
    /// the queen: ticks until her next barrage, and her next brood
    pub barrage: i32,
    pub brood: i32,
    /// what the drone is doing, for the HUD's scanner and for tests
    pub doing: A,
    /// how aware it is of each raider, in thousandths (run.rs: AWARE_*)
    pub aware: [i32; 4],
    /// where it noticed something, and how long it will keep looking
    pub suspect: Option<V3>,
    pub suspect_t: i32,
    /// stuck: where it was when last checked, and for how long it has barely moved
    pub last_pos: V3,
    pub stuck_t: i32,
}

impl Brain {
    pub fn new(home: V3) -> Brain {
        Brain {
            home,
            patrol: vec![home],
            patrol_i: 0,
            target: None,
            seen_at: home,
            seen_t: FORGET,
            sees: false,
            goal: home,
            face: 0,
            path: Vec::new(),
            path_goal: home,
            replan: 0,
            orbit_dir: 1,
            burst: 0,
            gap: 0,
            reload: 0,
            charge: 0,
            fuse: 0,
            cover: None,
            cover_t: 0,
            peek: None,
            ambush: None,
            bait: 0,
            bait_at: None,
            called: 0,
            stomp: 0,
            barrage: QUEEN_FIRST_BARRAGE,
            brood: QUEEN_FIRST_BROOD,
            doing: A::Patrol,
            aware: [0; 4],
            suspect: None,
            suspect_t: 0,
            last_pos: home,
            stuck_t: 0,
        }
    }
}

/// A raider as a drone sees them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seen {
    pub pos: V3,
    pub chest: V3,
    pub head: V3,
    pub yaw: i32,
    pub pitch: i32,
    pub vel: V3,
    /// how much they stand out, in percent: stance times gear
    pub notice: i32,
    pub active: bool,
}

/// What a think asks of the run, beyond moving.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    /// fire a bolt from the drone toward a point
    Bolt(V3),
    /// the hornet's charged shot lands now, along the line to this point
    Laser(V3),
    /// wake drones nearby to this target and send wasps to wait at a point
    Call(u8, V3),
    Detonate,
    Stomp,
    /// it hit the target body first
    Ram,
    /// the queen's rockets, at this point
    Barrage(V3),
    /// the queen's wasps, out of her back
    Brood,
}

pub struct Ctx<'a> {
    pub map: &'a Map,
    pub extra: &'a [Aabb],
    pub def: &'a DroneDef,
    pub raiders: &'a [Seen],
    pub rng: &'a mut Rng,
    pub tick: u32,
    pub id: usize,
    pub orders: Vec<Order>,
    pub outdoors: bool,
}

impl<'a> Ctx<'a> {
    fn target(&self, d: &Drone) -> Option<Seen> {
        d.brain
            .target
            .and_then(|t| self.raiders.get(t as usize).copied())
            .filter(|s| s.active)
    }
}

pub fn run(bt: &Bt, d: &mut Drone, cx: &mut Ctx) -> Status {
    match bt {
        Sel(children) => {
            for c in children.iter() {
                let s = run(c, d, cx);
                if s != Status::Failure {
                    return s;
                }
            }
            Status::Failure
        }
        Seq(children) => {
            for c in children.iter() {
                let s = run(c, d, cx);
                if s != Status::Success {
                    return s;
                }
            }
            Status::Success
        }
        If(c) => {
            if cond(*c, d, cx) {
                Status::Success
            } else {
                Status::Failure
            }
        }
        Not(c) => {
            if cond(*c, d, cx) {
                Status::Failure
            } else {
                Status::Success
            }
        }
        Do(a) => {
            d.brain.doing = *a;
            act(*a, d, cx)
        }
    }
}

fn cond(c: C, d: &Drone, cx: &Ctx) -> bool {
    let b = &d.brain;
    match c {
        C::Stunned => d.stun > 0,
        C::HasTarget => cx.target(d).is_some() && b.seen_t < FORGET,
        C::Sees => b.sees,
        C::Near(m) => cx.target(d).is_some_and(|t| t.pos.dist(d.pos) < int(m)),
        C::Exposed => {
            let Some(t) = cx.target(d) else { return false };
            if d.flash > 0 {
                return true;
            }
            if !b.sees {
                return false;
            }
            let to_me = d.pos.sub(t.head).norm();
            to_me.dot(facing(t.yaw, t.pitch)) > ONE * 95 / 100
        }
        C::InCover => b.cover.is_some_and(|c| c.dist(d.pos) < int(2)),
        C::PeekDue => b.cover_t <= 0,
        C::Charging => b.charge > 0,
        C::Armed => b.fuse > 0,
        C::Ambushing => b.ambush.is_some(),
        C::Baiting => b.bait > 0,
        C::TargetHidden => !b.sees,
        C::Disarmed => d.disarmed(),
        C::Suspicious => b.target.is_none() && b.suspect.is_some(),
        C::Lame => d.lame(),
    }
}

/// The height a flyer keeps over the floor under it.
fn hover(cx: &Ctx, d: &Drone) -> i32 {
    if cx.outdoors {
        OUTDOOR_HOVER
    } else {
        match d.kind {
            // under the balconies (5.2 m), rotors and all
            Kind::Warden => int(3),
            Kind::Hornet => int(4),
            _ => HOVER_LOW + (d.brain.home.x & (ONE * 2 - 1)),
        }
    }
}

fn aim_at(from: V3, to: V3) -> i32 {
    yaw_pitch_of(to.sub(from)).0
}

fn act(a: A, d: &mut Drone, cx: &mut Ctx) -> Status {
    let flyer = !d.kind.walks();
    let target = cx.target(d);
    match a {
        A::Wobble => {
            d.brain.goal = d.pos;
            Status::Running
        }
        A::Patrol => {
            let b = &mut d.brain;
            let n = b.patrol.len().max(1);
            let p = b.patrol[b.patrol_i % n];
            let at = if flyer {
                v3(p.x, p.y + hover(cx, d), p.z)
            } else {
                p
            };
            if d.pos.sub(at).len_xz() < int(2) {
                d.brain.patrol_i = (d.brain.patrol_i + 1) % n;
            }
            // a slow bob while it hovers along
            let bob = fx::sin((cx.tick as i32 * 600 + cx.id as i32 * 7919) & (fx::TURN - 1)) / 3;
            d.brain.goal = v3(at.x, at.y + if flyer { bob } else { 0 }, at.z);
            d.brain.face = aim_at(d.pos, at);
            Status::Running
        }
        A::Investigate => {
            let seen = d.brain.seen_at;
            let at = if flyer {
                v3(seen.x, seen.y + hover(cx, d), seen.z)
            } else {
                seen
            };
            d.brain.goal = at;
            if d.pos.sub(at).len_xz() < int(3) {
                // look round where it last saw them
                d.brain.face = fx::wrap(d.brain.face + deg(20));
            } else {
                d.brain.face = aim_at(d.pos, at);
            }
            Status::Running
        }
        A::Approach => {
            let Some(t) = target else { return Status::Failure };
            let at = if flyer {
                v3(t.pos.x, t.pos.y + hover(cx, d) + int(1), t.pos.z)
            } else {
                t.pos
            };
            d.brain.goal = at;
            d.brain.face = aim_at(d.pos, t.chest);
            Status::Running
        }
        A::Orbit => {
            let Some(t) = target else { return Status::Failure };
            // circle at a radius, a quarter-turn ahead of where it is now
            let rel = d.pos.sub(t.pos);
            let ang = yaw_pitch_of(v3(rel.x, 0, rel.z)).0;
            if cx.rng.chance(2) {
                d.brain.orbit_dir = -d.brain.orbit_dir;
            }
            let next = fx::wrap(ang + deg(25) * d.brain.orbit_dir);
            let r = ORBIT_RADIUS + (cx.id as i32 % 3) * int(1);
            let ring = t.pos.add(facing(next, 0).scale(r));
            d.brain.goal = v3(ring.x, t.pos.y + hover(cx, d) + int(1), ring.z);
            d.brain.face = aim_at(d.pos, t.chest);
            Status::Success
        }
        A::Hold(m) => {
            let Some(t) = target else { return Status::Failure };
            let rel = d.pos.sub(t.pos);
            let ang = yaw_pitch_of(v3(rel.x, 0, rel.z)).0;
            let next = fx::wrap(ang + deg(6) * d.brain.orbit_dir);
            let ring = t.pos.add(facing(next, 0).scale(int(m)));
            let y = if flyer {
                t.pos.y + hover(cx, d) + int(2)
            } else {
                ring.y
            };
            d.brain.goal = v3(ring.x, y, ring.z);
            d.brain.face = aim_at(d.pos, t.chest);
            Status::Success
        }
        A::Keep(m) => {
            let Some(t) = target else { return Status::Failure };
            let rel = d.pos.sub(t.pos);
            let dist = rel.len_xz();
            let away = v3(rel.x, 0, rel.z).norm();
            // walk sideways as well as back, so it is not a straight line
            let side = right_of(aim_at(d.pos, t.pos)).scale(int(4) * d.brain.orbit_dir);
            let want = if dist < int(m) {
                t.pos.add(away.scale(int(m)))
            } else {
                d.pos
            };
            d.brain.goal = want.add(side);
            if cx.rng.chance(3) {
                d.brain.orbit_dir = -d.brain.orbit_dir;
            }
            d.brain.face = aim_at(d.pos, t.chest);
            Status::Success
        }
        A::Shoot => {
            let Some(t) = target else { return Status::Failure };
            // the queen shoots with her eye cannon, while she has it
            let eye_gone = d.kind == Kind::Queen && d.pieces[crate::drone::QUEEN_EYE] <= 0;
            if d.disarmed() || eye_gone {
                return Status::Failure;
            }
            if !d.brain.sees || cx.def.burst == 0 {
                return Status::Running;
            }
            let b = &mut d.brain;
            let facing_ok = fx::wrap(aim_at(d.pos, t.chest) - d.yaw).abs() < deg(35);
            if b.reload > 0 || b.gap > 0 || !facing_ok {
                return Status::Running;
            }
            if b.burst <= 0 {
                b.burst = cx.def.burst;
            }
            b.burst -= 1;
            b.gap = cx.def.burst_gap.max(1);
            if b.burst == 0 {
                b.reload = cx.def.reload + cx.rng.range(0, secs(600));
            }
            // lead the target a little
            let lead = t.vel.scale(ONE * 2 / 10);
            cx.orders.push(Order::Bolt(t.chest.add(lead)));
            Status::Running
        }
        A::SeekCover => {
            let Some(t) = target else { return Status::Failure };
            let need_new = match d.brain.cover {
                None => true,
                Some(c) => cx.map.clear(t.head, c, cx.extra),
            };
            if need_new {
                d.brain.cover = find_spot(d, cx, t.head, false, int(7));
                d.brain.cover_t = secs(1200) + cx.rng.range(0, secs(1800));
                d.brain.peek = None;
            }
            match d.brain.cover {
                Some(c) => {
                    d.brain.goal = c;
                    if c.dist(d.pos) < int(2) {
                        d.brain.cover_t -= THINK_EVERY as i32;
                    }
                }
                None => {
                    // nowhere hidden: back off and keep low
                    let away = d.pos.sub(t.pos).norm().scale(int(6));
                    d.brain.goal = d.pos.add(v3(away.x, 0, away.z));
                }
            }
            d.brain.face = aim_at(d.pos, t.chest);
            Status::Running
        }
        A::Peek => {
            let Some(t) = target else { return Status::Failure };
            if d.brain.peek.is_none() {
                d.brain.peek = find_spot(d, cx, t.head, true, int(6));
                if d.brain.peek.is_none() {
                    d.brain.cover_t = secs(800);
                    return Status::Failure;
                }
            }
            let p = d.brain.peek.unwrap_or(d.pos);
            d.brain.goal = p;
            d.brain.face = aim_at(d.pos, t.chest);
            if d.brain.sees && d.brain.reload <= 0 {
                d.brain.charge = cx.def.charge.max(1);
                d.brain.cover = None;
            }
            Status::Running
        }
        A::Charge => {
            let Some(t) = target else {
                d.brain.charge = 0;
                return Status::Failure;
            };
            if d.brain.reload > 0 && d.brain.charge == 0 {
                return Status::Failure;
            }
            if d.brain.charge == 0 {
                d.brain.charge = cx.def.charge.max(1);
            }
            d.brain.goal = d.pos;
            d.brain.face = aim_at(d.pos, t.chest);
            d.brain.charge -= THINK_EVERY as i32;
            if d.brain.charge <= 0 {
                d.brain.charge = 0;
                d.brain.reload = cx.def.reload;
                d.brain.peek = None;
                d.brain.cover = None;
                if d.brain.sees {
                    cx.orders.push(Order::Laser(t.chest));
                }
                return Status::Success;
            }
            Status::Running
        }
        A::Call => {
            // no beacon, no call
            if d.brain.called > 0 || d.alive_count(crate::drone::PieceKind::Beacon) == 0 {
                return Status::Success;
            }
            d.brain.called = secs(6000);
            let ambush = d.brain.home;
            d.brain.bait_at = Some(ambush);
            if let Some(t) = d.brain.target {
                cx.orders.push(Order::Call(t, ambush));
            }
            Status::Success
        }
        A::Bait => {
            let Some(t) = target else { return Status::Failure };
            if d.brain.bait <= 0 {
                // start: find a place the raider can see, not too close
                let show = find_spot(d, cx, t.head, true, int(10));
                let Some(show) = show else { return Status::Failure };
                d.brain.peek = Some(show);
                d.brain.bait = secs(7000);
            }
            d.brain.bait -= THINK_EVERY as i32;
            let shown = d.brain.bait < secs(4500);
            let goal = if shown {
                // linger, blinking, then drift back toward the ambush
                let back = d.brain.bait_at.unwrap_or(d.brain.home);
                if d.brain.bait < secs(3000) {
                    v3(back.x, back.y + hover(cx, d), back.z)
                } else {
                    d.pos
                }
            } else {
                d.brain.peek.unwrap_or(d.pos)
            };
            d.brain.goal = goal;
            d.brain.face = aim_at(d.pos, t.chest);
            if d.brain.bait <= 0 {
                d.brain.peek = None;
                return Status::Success;
            }
            Status::Running
        }
        A::Flee => {
            let Some(t) = target else { return Status::Failure };
            let back = d.brain.bait_at.unwrap_or(d.brain.home);
            let away = d.pos.sub(t.pos);
            let away = v3(away.x, 0, away.z).norm().scale(int(10));
            let to = if back.dist(t.pos) > int(15) {
                back
            } else {
                d.pos.add(away)
            };
            d.brain.goal = v3(to.x, to.y.max(t.pos.y) + hover(cx, d), to.z);
            d.brain.face = aim_at(d.pos, t.chest);
            Status::Running
        }
        A::Ambush => {
            let Some(p) = d.brain.ambush else {
                return Status::Failure;
            };
            // wait low beside the spot, facing the way the bait comes from
            let side = right_of(d.brain.home.x & 0xffff).scale(int(4));
            d.brain.goal = v3(p.x + side.x, p.y + HOVER_LOW, p.z + side.z);
            if let Some(t) = target {
                d.brain.face = aim_at(d.pos, t.pos);
            }
            Status::Running
        }
        A::Rush => {
            let Some(t) = target else { return Status::Failure };
            d.brain.goal = t.pos;
            d.brain.face = aim_at(d.pos, t.pos);
            Status::Running
        }
        A::Arm => {
            if d.brain.fuse == 0 {
                d.brain.fuse = cx.def.fuse.max(1);
            }
            d.brain.goal = target.map(|t| t.pos).unwrap_or(d.pos);
            d.brain.fuse -= THINK_EVERY as i32;
            if d.brain.fuse <= 0 {
                d.brain.fuse = 0;
                cx.orders.push(Order::Detonate);
                return Status::Success;
            }
            Status::Running
        }
        A::Ram => {
            let Some(t) = target else { return Status::Failure };
            d.brain.goal = t.chest;
            d.brain.face = aim_at(d.pos, t.chest);
            if d.brain.stomp <= 0 && d.centre().dist(t.chest) < ONE * 16 / 10 {
                d.brain.stomp = secs(1500);
                cx.orders.push(Order::Ram);
            }
            Status::Running
        }
        A::Search => {
            let Some(p) = d.brain.suspect else {
                return Status::Failure;
            };
            let at = if flyer {
                v3(p.x, p.y + hover(cx, d), p.z)
            } else {
                p
            };
            d.brain.goal = at;
            if d.pos.sub(at).len_xz() < int(3) {
                // there: turn about, looking
                d.brain.face = fx::wrap(d.brain.face + deg(15));
                d.brain.suspect_t -= THINK_EVERY as i32 * 2;
            } else {
                d.brain.face = aim_at(d.pos, at);
                d.brain.suspect_t -= THINK_EVERY as i32 / 2;
            }
            if d.brain.suspect_t <= 0 {
                d.brain.suspect = None;
                return Status::Success;
            }
            Status::Running
        }
        A::Barrage => {
            let Some(t) = target else { return Status::Failure };
            let pods = crate::drone::QUEEN_PODS.iter().any(|&i| d.pieces[i] > 0);
            if d.brain.barrage > 0 || !pods || !d.brain.sees {
                return Status::Failure;
            }
            d.brain.barrage = QUEEN_BARRAGE_EVERY + cx.rng.range(0, secs(2000));
            d.brain.goal = d.pos;
            d.brain.face = aim_at(d.pos, t.pos);
            cx.orders.push(Order::Barrage(t.pos.add(t.vel.scale(ONE / 2))));
            Status::Success
        }
        A::Brood => {
            if d.brain.brood > 0 || target.is_none() {
                return Status::Failure;
            }
            d.brain.brood = QUEEN_BROOD_EVERY;
            d.brain.goal = d.pos;
            cx.orders.push(Order::Brood);
            Status::Success
        }
        A::Anchor => {
            d.brain.goal = d.pos;
            if let Some(t) = target {
                d.brain.face = aim_at(d.pos, t.chest);
            }
            Status::Success
        }
        A::Stomp => {
            let Some(t) = target else { return Status::Failure };
            d.brain.goal = d.pos;
            d.brain.face = aim_at(d.pos, t.pos);
            if d.brain.stomp <= 0 {
                d.brain.stomp = secs(1600);
                cx.orders.push(Order::Stomp);
            }
            Status::Running
        }
    }
}

/// A spot near the drone that `from` can see (`visible`) or cannot, at
/// least `min` from `from`. Eight bearings at two distances, nearest first.
fn find_spot(d: &Drone, cx: &mut Ctx, from: V3, visible: bool, min: i32) -> Option<V3> {
    let flyer = !d.kind.walks();
    let start = cx.rng.range(0, 7);
    let mut best: Option<(i32, V3)> = None;
    for ring in [int(5), int(9), int(14)] {
        for k in 0..8 {
            let ang = fx::wrap((start + k) * deg(45));
            let mut p = d.pos.add(facing(ang, 0).scale(ring));
            if flyer {
                p.y = (from.y + int(2)).max(d.pos.y - int(1));
            }
            if p.dist(from) < min {
                continue;
            }
            if !cx.map.clear(d.pos, p, cx.extra) {
                continue;
            }
            let probe = Aabb::standing(v3(p.x, p.y - ONE / 2, p.z), ONE / 2, ONE);
            if cx.map.blocked(&probe, cx.extra) {
                continue;
            }
            if cx.map.clear(from, p, cx.extra) == visible {
                let cost = p.dist(d.pos);
                if best.is_none_or(|b| cost < b.0) {
                    best = Some((cost, p));
                }
            }
        }
        if best.is_some() {
            break;
        }
    }
    best.map(|b| b.1)
}

/// One think: perception has already been done; the timers tick and the
/// tree runs.
pub fn think(d: &mut Drone, cx: &mut Ctx) {
    let b = &mut d.brain;
    let dt = THINK_EVERY as i32;
    b.gap = (b.gap - dt).max(0);
    b.reload = (b.reload - dt).max(0);
    b.called = (b.called - dt).max(0);
    b.stomp = (b.stomp - dt).max(0);
    b.barrage = (b.barrage - dt).max(0);
    b.brood = (b.brood - dt).max(0);
    b.seen_t = (b.seen_t + dt).min(FORGET * 2);
    if b.seen_t >= FORGET && b.fuse == 0 {
        b.target = None;
        b.ambush = None;
        b.bait = 0;
    }
    let bt = tree(d.kind);
    run(bt, d, cx);
}

/// Where a drone should move this tick toward its goal: straight when the
/// way is clear, else along the waypoint graph. Returns the wanted velocity.
pub fn steer(d: &mut Drone, map: &Map, extra: &[Aabb], top: i32, outdoors: bool) -> V3 {
    let flyer = !d.kind.walks();
    let goal = d.brain.goal;
    let from = if flyer {
        d.pos
    } else {
        v3(d.pos.x, d.pos.y + ONE, d.pos.z)
    };
    let to = if flyer {
        goal
    } else {
        v3(goal.x, goal.y + ONE, goal.z)
    };
    let direct = d.brain.replan > 0 && d.brain.path.is_empty() || map.clear(from, to, extra);
    let aim = if direct || d.pos.dist(goal) < int(3) {
        d.brain.path.clear();
        goal
    } else {
        if d.brain.path.is_empty() || d.brain.path_goal.dist(goal) > int(6) || d.brain.replan <= 0 {
            d.brain.replan = secs(1500);
            d.brain.path_goal = goal;
            d.brain.path.clear();
            if let (Some(a), Some(b)) = (map.nearest_node(d.pos), map.nearest_node(goal)) {
                let hover = if outdoors { OUTDOOR_HOVER } else { HOVER_LOW + ONE };
                for n in map.path(a, b, 600) {
                    let p = map.nodes[n];
                    d.brain
                        .path
                        .push(if flyer { v3(p.x, p.y + hover, p.z) } else { p });
                }
            }
        }
        while d.brain.path.len() > 1 && d.brain.path[0].dist(d.pos) < int(2) {
            d.brain.path.remove(0);
        }
        d.brain.path.first().copied().unwrap_or(goal)
    };
    d.brain.replan -= 1;
    let mut want = aim.sub(d.pos);
    if !flyer {
        want.y = 0;
    }
    let dist = want.len();
    if dist < ONE / 4 {
        return V3::ZERO;
    }
    // arrive: slow down inside 3 m
    let speed = if dist < int(3) {
        fx::mul(top, fx::div(dist, int(3)))
    } else {
        top
    };
    want.norm().scale(speed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::content;
    use crate::world::{Look, Solid};

    fn room() -> Map {
        let mut m = Map::new(v3(-int(60), -int(5), -int(60)), v3(int(60), int(30), int(60)));
        // a pillar to hide behind
        m.add(Solid {
            b: Aabb::new(v3(-int(2), 0, -int(12)), v3(int(2), int(12), -int(8))),
            rgb: [0; 3],
            look: Look::Plain,
            open: false,
        });
        m
    }

    fn seen(pos: V3, yaw: i32) -> Seen {
        Seen {
            pos,
            chest: v3(pos.x, pos.y + ONE * 13 / 10, pos.z),
            head: v3(pos.x, pos.y + ONE * 17 / 10, pos.z),
            yaw,
            pitch: 0,
            vel: V3::ZERO,
            notice: 100,
            active: true,
        }
    }

    fn drone(kind: &str, pos: V3) -> (Drone, DroneDef) {
        let c = content();
        let i = c.drone(kind);
        let d = c.drones[i].clone();
        (Drone::new(Kind::from_id(kind), i, &d, pos, 0, pos), d)
    }

    #[test]
    fn a_wasp_that_sees_a_raider_circles_and_fires() {
        let m = room();
        let (mut w, def) = drone("wasp", v3(0, int(4), int(14)));
        w.brain.target = Some(0);
        w.brain.seen_t = 0;
        w.brain.sees = true;
        w.yaw = yaw_pitch_of(v3(0, 0, -ONE)).0;
        let raiders = [seen(V3::ZERO, 0)];
        let mut rng = Rng::new(1);
        let mut fired = 0;
        for tick in 0..40 {
            let mut cx = Ctx {
                map: &m,
                extra: &[],
                def: &def,
                raiders: &raiders,
                rng: &mut rng,
                tick,
                id: 0,
                orders: Vec::new(),
                outdoors: false,
            };
            think(&mut w, &mut cx);
            fired += cx.orders.iter().filter(|o| matches!(o, Order::Bolt(_))).count();
        }
        assert_eq!(w.brain.doing, A::Shoot);
        assert!(fired >= 2, "{fired}");
        let r = w.brain.goal.sub(V3::ZERO).len_xz();
        assert!(r > int(9) && r < int(15), "an orbit's radius: {r}");
    }

    #[test]
    fn a_hornet_that_is_looked_at_ducks_behind_the_pillar() {
        let m = room();
        let (mut h, def) = drone("hornet", v3(0, int(4), -int(16)));
        h.brain.target = Some(0);
        h.brain.seen_t = 0;
        h.brain.sees = true;
        // the raider at the origin faces the hornet (−z)
        let raiders = [seen(V3::ZERO, 0)];
        let mut rng = Rng::new(3);
        let mut cx = Ctx {
            map: &m,
            extra: &[],
            def: &def,
            raiders: &raiders,
            rng: &mut rng,
            tick: 0,
            id: 0,
            orders: Vec::new(),
            outdoors: false,
        };
        think(&mut h, &mut cx);
        assert_eq!(h.brain.doing, A::SeekCover);
        let c = h.brain.cover.expect("a cover spot");
        assert!(
            !m.clear(raiders[0].head, c, &[]),
            "the spot is hidden from the raider"
        );
    }

    #[test]
    fn a_hornet_in_cover_peeks_then_charges_then_fires_a_laser() {
        let m = room();
        let (mut h, def) = drone("hornet", v3(0, int(3), -int(14)));
        h.brain.target = Some(0);
        h.brain.seen_t = 0;
        h.brain.sees = false;
        h.brain.cover = Some(h.pos);
        h.brain.cover_t = 0;
        // facing away: not exposed
        let raiders = [seen(V3::ZERO, fx::HALF)];
        let mut rng = Rng::new(5);
        let mut cx = Ctx {
            map: &m,
            extra: &[],
            def: &def,
            raiders: &raiders,
            rng: &mut rng,
            tick: 0,
            id: 0,
            orders: Vec::new(),
            outdoors: false,
        };
        think(&mut h, &mut cx);
        assert_eq!(h.brain.doing, A::Peek);
        let p = h.brain.peek.expect("a peek spot");
        assert!(m.clear(raiders[0].head, p, &[]), "the peek spot sees the raider");
        // arrive and see: it charges, then the laser goes
        h.pos = p;
        h.brain.sees = true;
        let mut lasers = 0;
        for tick in 0..60 {
            let mut cx = Ctx {
                map: &m,
                extra: &[],
                def: &def,
                raiders: &raiders,
                rng: &mut rng,
                tick,
                id: 0,
                orders: Vec::new(),
                outdoors: false,
            };
            think(&mut h, &mut cx);
            lasers += cx.orders.iter().filter(|o| matches!(o, Order::Laser(_))).count();
        }
        assert_eq!(lasers, 1);
    }

    #[test]
    fn a_moth_calls_for_an_ambush_and_baits_a_raider_in_cover() {
        let m = room();
        let (mut moth, def) = drone("moth", v3(0, int(5), -int(30)));
        moth.brain.target = Some(0);
        moth.brain.seen_t = 0;
        moth.brain.sees = false;
        let raiders = [seen(v3(0, 0, -int(3)), 0)];
        let mut rng = Rng::new(9);
        let mut cx = Ctx {
            map: &m,
            extra: &[],
            def: &def,
            raiders: &raiders,
            rng: &mut rng,
            tick: 0,
            id: 0,
            orders: Vec::new(),
            outdoors: false,
        };
        think(&mut moth, &mut cx);
        assert!(
            cx.orders.iter().any(|o| matches!(o, Order::Call(0, _))),
            "{:?}",
            cx.orders
        );
        assert_eq!(moth.brain.doing, A::Bait);
        let show = moth.brain.peek.unwrap();
        assert!(
            m.clear(raiders[0].head, show, &[]),
            "it shows itself where the raider can see it"
        );
    }

    #[test]
    fn a_disarmed_wasp_rams_and_a_lame_strider_plants_and_fires() {
        let m = room();
        let (mut w, def) = drone("wasp", v3(0, int(2), ONE / 2));
        w.brain.target = Some(0);
        w.brain.seen_t = 0;
        w.brain.sees = true;
        w.pieces[5] = 0;
        let raiders = [seen(V3::ZERO, 0)];
        let mut rng = Rng::new(4);
        let mut cx = Ctx {
            map: &m,
            extra: &[],
            def: &def,
            raiders: &raiders,
            rng: &mut rng,
            tick: 0,
            id: 0,
            orders: Vec::new(),
            outdoors: false,
        };
        think(&mut w, &mut cx);
        assert_eq!(w.brain.doing, A::Ram);
        assert!(
            cx.orders.contains(&Order::Ram),
            "close enough to hit: {:?}",
            cx.orders
        );
        assert!(
            !cx.orders.iter().any(|o| matches!(o, Order::Bolt(_))),
            "no guns, no bolts"
        );

        let (mut s, sdef) = drone("strider", v3(int(12), 0, 0));
        s.brain.target = Some(0);
        s.brain.seen_t = 0;
        s.brain.sees = true;
        s.yaw = yaw_pitch_of(v3(-ONE, 0, 0)).0;
        for k in 0..4 {
            s.pieces[k] = 0;
        }
        let mut cx = Ctx {
            map: &m,
            extra: &[],
            def: &sdef,
            raiders: &raiders,
            rng: &mut rng,
            tick: 0,
            id: 0,
            orders: Vec::new(),
            outdoors: false,
        };
        think(&mut s, &mut cx);
        assert_eq!(s.brain.doing, A::Shoot, "it stays put and shoots");
        assert_eq!(s.brain.goal, s.pos);
    }

    #[test]
    fn a_moth_without_its_beacon_cannot_call() {
        let m = room();
        let (mut moth, def) = drone("moth", v3(0, int(5), -int(30)));
        moth.brain.target = Some(0);
        moth.brain.seen_t = 0;
        moth.pieces[6] = 0;
        let raiders = [seen(v3(0, 0, -int(3)), 0)];
        let mut rng = Rng::new(9);
        let mut cx = Ctx {
            map: &m,
            extra: &[],
            def: &def,
            raiders: &raiders,
            rng: &mut rng,
            tick: 0,
            id: 0,
            orders: Vec::new(),
            outdoors: false,
        };
        think(&mut moth, &mut cx);
        assert!(!cx.orders.iter().any(|o| matches!(o, Order::Call(..))));
    }

    #[test]
    fn a_tick_rushes_and_detonates_close() {
        let m = room();
        let (mut t, def) = drone("tick", v3(int(10), 0, 0));
        t.brain.target = Some(0);
        t.brain.seen_t = 0;
        let raiders = [seen(V3::ZERO, 0)];
        let mut rng = Rng::new(2);
        let mut cx = Ctx {
            map: &m,
            extra: &[],
            def: &def,
            raiders: &raiders,
            rng: &mut rng,
            tick: 0,
            id: 0,
            orders: Vec::new(),
            outdoors: false,
        };
        think(&mut t, &mut cx);
        assert_eq!(t.brain.doing, A::Rush);
        t.pos = v3(ONE, 0, 0);
        let mut boom = false;
        for tick in 0..30 {
            let mut cx = Ctx {
                map: &m,
                extra: &[],
                def: &def,
                raiders: &raiders,
                rng: &mut rng,
                tick,
                id: 0,
                orders: Vec::new(),
                outdoors: false,
            };
            think(&mut t, &mut cx);
            boom |= cx.orders.contains(&Order::Detonate);
        }
        assert!(boom);
    }
}
