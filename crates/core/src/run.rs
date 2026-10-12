//! One run: four raiders (or fewer), a client's order, the mall and its
//! drones, from the back of the van to the van leaving. Every rule of a run
//! is here and nothing here reads a clock or a device: `step` takes the
//! raiders' inputs for one tick and moves everything one tick on, so the
//! same inputs give the same run on every machine. That is what lets online
//! play roll back and run ticks again (rollback.rs), and it is why a `Run` is a
//! plain value that can be cloned and compared.

use crate::ai::{self, Ctx, Order, Seen};
use crate::balance::*;
use crate::content::{Cat, Content, Effect, Goal, ShotKind};
use crate::drone::{self, Drone, Kind, PieceKind, State, Zone};
use crate::fx::{self, deg, int, ONE};
use crate::geom::{facing, right_of, v3, yaw_pitch_of, V3};
use crate::gun::{Build, Gun};
use crate::kit::{Gadget, Kit, Wear};
use crate::raider::{
    self, gun_frame, Input, Life, Raider, B_FIRE, B_GADGET, B_GONE, B_MELEE, B_RELOAD, B_USE,
};
use crate::rng::Rng;
use crate::world::{Aabb, Map, SpotKind};

/// What a raider carries out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Part(u16),
    /// the mission's thing: the shirt, a drive
    Mission,
    /// a piece of gear, by index into `Content::gear`
    Gear(u16),
    /// rounds of one kind of ammunition (index into `Content::ammo`)
    Ammo(u8, u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pickup {
    pub pos: V3,
    pub item: Item,
    pub vy: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShotFrom {
    Raider(u8),
    Drone(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shot {
    pub from: ShotFrom,
    pub kind: ShotKind,
    pub pos: V3,
    pub vel: V3,
    pub life: i32,
    pub damage: i32,
    pub splash: i32,
    pub effect: Effect,
    pub power: i32,
    pub pierce: i32,
    /// a warden's slow orb, drawn bigger
    pub heavy: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hive {
    pub pos: V3,
    pub hp: i32,
    pub pods: [i32; 6],
    pub spawn_t: i32,
}

impl Hive {
    pub fn pod_at(&self, i: usize) -> V3 {
        let a = deg(60) * i as i32;
        let r = ONE * 19 / 10;
        v3(
            self.pos.x + fx::mul(fx::sin(a), r),
            self.pos.y + int(2) + (i as i32 % 2) * ONE,
            self.pos.z + fx::mul(fx::cos(a), r),
        )
    }

    pub fn core(&self) -> V3 {
        v3(self.pos.x, self.pos.y + int(3), self.pos.z)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Van {
    /// the middle of the back doors, on the ground
    pub pos: V3,
    /// the way the back doors face: a multiple of a quarter turn
    pub yaw: i32,
    pub doors_open: bool,
    pub door_t: i32,
    pub leaving: Option<i32>,
}

pub const VAN_FLOOR: i32 = ONE * 6 / 10;
const VAN_HALF_W: i32 = ONE * 12 / 10;
const VAN_CARGO: i32 = int(4);
const VAN_CAB: i32 = ONE * 17 / 10;
const VAN_ROOF: i32 = ONE * 27 / 10;

impl Van {
    /// The van's boxes in the world: the floor, the walls, the roof, the
    /// cab, the bumper step, and the doors when shut.
    pub fn boxes(&self) -> Vec<Aabb> {
        let f = facing(self.yaw, 0);
        let r = right_of(self.yaw);
        let at =
            |back: i32, side: i32, y: i32| self.pos.sub(f.scale(back)).add(r.scale(side)).add(v3(0, y, 0));
        let bx = |a: V3, b: V3| Aabb::new(a, b);
        let t = ONE * 15 / 100;
        let mut out = vec![
            bx(
                at(0, -VAN_HALF_W, 0),
                at(VAN_CARGO + VAN_CAB, VAN_HALF_W, VAN_FLOOR),
            ),
            bx(
                at(0, -VAN_HALF_W, VAN_FLOOR),
                at(VAN_CARGO, -VAN_HALF_W + t, VAN_ROOF),
            ),
            bx(
                at(0, VAN_HALF_W - t, VAN_FLOOR),
                at(VAN_CARGO, VAN_HALF_W, VAN_ROOF),
            ),
            bx(
                at(0, -VAN_HALF_W, VAN_ROOF),
                at(VAN_CARGO, VAN_HALF_W, VAN_ROOF + ONE / 5),
            ),
            bx(
                at(VAN_CARGO, -VAN_HALF_W, VAN_FLOOR),
                at(VAN_CARGO + VAN_CAB, VAN_HALF_W, VAN_ROOF - ONE / 4),
            ),
            bx(
                at(-ONE * 4 / 10, -VAN_HALF_W + ONE / 3, 0),
                at(0, VAN_HALF_W - ONE / 3, ONE * 3 / 10),
            ),
        ];
        if !self.doors_open {
            out.push(bx(at(0, -VAN_HALF_W, VAN_FLOOR), at(t, VAN_HALF_W, VAN_ROOF)));
        }
        out
    }

    /// The cargo space: who is "in the van".
    pub fn inside(&self) -> Aabb {
        let f = facing(self.yaw, 0);
        let r = right_of(self.yaw);
        let at =
            |back: i32, side: i32, y: i32| self.pos.sub(f.scale(back)).add(r.scale(side)).add(v3(0, y, 0));
        Aabb::new(at(-ONE / 2, -VAN_HALF_W, 0), at(VAN_CARGO, VAN_HALF_W, VAN_ROOF))
    }

    /// Where raider `i` starts: sitting along the benches, facing the doors.
    pub fn seat(&self, i: usize) -> V3 {
        let f = facing(self.yaw, 0);
        let r = right_of(self.yaw);
        let back = int(1) + (i as i32 / 2) * ONE * 14 / 10;
        let side = if i.is_multiple_of(2) {
            -ONE * 6 / 10
        } else {
            ONE * 6 / 10
        };
        self.pos
            .sub(f.scale(back))
            .add(r.scale(side))
            .add(v3(0, VAN_FLOOR, 0))
    }
}

/// Things that happened on a tick, for sound and sparks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Fire {
        who: u8,
        at: V3,
        kind: ShotKind,
    },
    DroneFire {
        at: V3,
        heavy: bool,
    },
    Laser {
        from: V3,
        to: V3,
    },
    Spark {
        at: V3,
    },
    /// a shot landed on a drone (or the hive, drone `u16::MAX`), doing `dealt`
    Hit {
        at: V3,
        weak: bool,
        armour: bool,
        by: u8,
        dealt: i32,
        drone: u16,
    },
    /// a part was torn off a drone and falls away
    PieceOff {
        at: V3,
        kind: PieceKind,
        drone: u16,
        by: u8,
    },
    Boom {
        at: V3,
        radius: i32,
        effect: Effect,
    },
    Wreck {
        at: V3,
        kind: Kind,
    },
    Kill {
        by: u8,
    },
    Hurt {
        who: u8,
    },
    Downed {
        who: u8,
    },
    Revived {
        who: u8,
    },
    Died {
        who: u8,
    },
    Picked {
        who: u8,
        item: Item,
    },
    Searched {
        at: V3,
    },
    Reload {
        who: u8,
    },
    Doors,
    VanStart,
    Extracted,
    Spotted {
        at: V3,
    },
    Generator,
    Objective,
    Wave,
    Stomp {
        at: V3,
    },
    /// one of the queen's feet came down
    Tread {
        at: V3,
    },
    /// the queen's wasps left her back
    Brood {
        at: V3,
    },
    /// a melee swing landed on something (or swished through air)
    Melee {
        who: u8,
        at: V3,
        hit: bool,
    },
    /// a gadget went out, or went off
    Gadget {
        kind: Gadget,
        at: V3,
        by: u8,
    },
    /// a drone began to wonder about someone
    Noticed {
        at: V3,
    },
    /// a raider's gun is dry and the pack has nothing for it
    NoAmmo {
        who: u8,
    },
}

/// What a raider's E key would do now, for the HUD prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Use {
    Revive(u8),
    Pick(u16),
    Search(u16),
    Generator,
    Doors,
    Leave,
}

impl Use {
    fn code(self) -> i32 {
        match self {
            Use::Revive(i) => 1000 + i as i32,
            Use::Pick(i) => 2000 + i as i32,
            Use::Search(i) => 3000 + i as i32,
            Use::Generator => 4000,
            Use::Doors => 5000,
            Use::Leave => 6000,
        }
    }

    pub fn needs(self) -> i32 {
        match self {
            Use::Revive(_) => REVIVE,
            Use::Pick(_) => secs(150),
            Use::Search(_) => SEARCH,
            Use::Generator => GENERATOR_START,
            Use::Doors => secs(200),
            Use::Leave => secs(500),
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Use::Revive(_) => "revive",
            Use::Pick(_) => "pick",
            Use::Search(_) => "search",
            Use::Generator => "generator",
            Use::Doors => "doors",
            Use::Leave => "leave",
        }
    }
}

/// What does not change during a run: the order, who is playing and with
/// what guns, and the seed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setup {
    pub mission: usize,
    pub builds: Vec<Build>,
    pub guns: Vec<Gun>,
    pub seed: u64,
    /// what each raider wears, and what it adds up to
    pub wears: Vec<Wear>,
    pub kits: Vec<Kit>,
    /// rounds of their gun's ammunition each raider brings
    pub ammo_in: Vec<i32>,
}

impl Setup {
    /// A setup with every raider in basic gear and the basic gun, carrying
    /// the free light rounds: for tests and probes.
    pub fn basic(c: &Content, mission: usize, players: usize, seed: u64) -> Setup {
        let b = crate::gun::basic_build(c);
        let w = crate::kit::basic_wear(c);
        Setup {
            mission,
            builds: vec![b; players],
            guns: vec![crate::gun::compose(c, &b); players],
            seed,
            wears: vec![w; players],
            kits: vec![crate::kit::kit(c, &w); players],
            ammo_in: vec![c.ammo[0].free.max(60); players],
        }
    }
}

/// Something a raider or a drone heard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Noise {
    pub at: V3,
    pub radius: i32,
    /// who made it (drones learn where they are), or nobody (a decoy)
    pub who: Option<u8>,
}

/// A belt gadget out in the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placed {
    pub kind: Gadget,
    pub pos: V3,
    pub vel: V3,
    pub yaw: i32,
    /// ticks left
    pub life: i32,
    /// thrown things fly until they land
    pub landed: bool,
    pub owner: u8,
    /// a mine arms after a moment
    pub armed: i32,
}

/// A melee swing: the blow lands this many ticks in, and it is over at the second.
pub const MELEE_HIT: i32 = secs(140);
pub const MELEE_RECOVER: i32 = secs(380);

/// Awareness, in thousandths: full is alerted, past suspect it comes to look.
pub const AWARE_FULL: i32 = 1000;
pub const AWARE_SUSPECT: i32 = 350;
/// A raider it has fought, seen again with this much left, it knows at once.
pub const AWARE_KNOWN: i32 = 500;
/// Gained a look in plain view, close, walking; lost a look out of sight.
pub const AWARE_GAIN: i32 = 140;
pub const AWARE_FADE: i32 = 22;
pub const SUSPECT_TIME: i32 = secs(7000);

/// Does a smoke cloud centred at `c` stand between two points?
pub fn smoke_between(c: V3, a: V3, b: V3) -> bool {
    let d = b.sub(a);
    let len = d.len().max(1);
    let dir = d.norm();
    let t = c.sub(a).dot(dir).clamp(0, len);
    a.add(dir.scale(t)).dist(c) < SMOKE_RADIUS * 9 / 10
}

/// Rounds as pack items, in stacks.
/// The average health of a drone kind, the measure of starting rounds
/// (the queen is no measure).
pub fn average_hp(c: &Content) -> i32 {
    let d: Vec<i32> = c
        .drones
        .iter()
        .filter(|d| d.id != "queen")
        .map(|d| d.hp)
        .collect();
    d.iter().sum::<i32>() / (d.len() as i32).max(1)
}

pub fn ammo_stacks(c: &Content, ammo: usize, n: i32) -> Vec<Item> {
    let stack = c.ammo[ammo].stack.max(1);
    let mut out = Vec::new();
    let mut left = n;
    while left > 0 {
        let put = left.min(stack);
        out.push(Item::Ammo(ammo as u8, put as u16));
        left -= put;
    }
    out
}

pub const MINE_RADIUS: i32 = ONE * 28 / 10;
pub const MINE_BLAST: i32 = int(4);
pub const MINE_DAMAGE: i32 = 80;
pub const DECOY_NOISE: i32 = int(45);
pub const SMOKE_RADIUS: i32 = int(5);
pub const PAINT_LONG: i32 = int(9);
pub const PAINT_WIDE: i32 = ONE * 24 / 10;
pub const PAINT_BOOST: i32 = 140;
pub const PAINT_SLOW: i32 = 50;

pub struct Ref<'a> {
    pub content: &'a Content,
    pub map: &'a Map,
    pub setup: &'a Setup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Generator {
    Off,
    Running(i32),
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub tick: u32,
    pub rng: Rng,
    pub raiders: Vec<Raider>,
    pub packs: Vec<Vec<Item>>,
    pub drones: Vec<Drone>,
    pub shots: Vec<Shot>,
    pub pickups: Vec<Pickup>,
    /// one per map spot: searched already (loot), or taken (a mission spot)
    pub searched: Vec<bool>,
    pub van: Van,
    pub hive: Option<Hive>,
    pub generator: Generator,
    pub wave_t: i32,
    pub waves_left: i32,
    pub found: i32,
    pub over: Option<bool>,
    pub events: Vec<Event>,
    /// the van's boxes, rebuilt when the doors change
    pub extra: Vec<Aabb>,
    pub noise: Vec<Noise>,
    pub gadgets: Vec<Placed>,
    /// each raider's kit, for the damage their gear takes off
    pub kits: Vec<Kit>,
}

fn snap_quarter(yaw: i32) -> i32 {
    ((yaw + deg(45)) & (fx::TURN - 1)) / deg(90) * deg(90)
}

impl Run {
    pub fn new(r: &Ref) -> Run {
        let (c, map, s) = (r.content, r.map, r.setup);
        let m = &c.missions[s.mission];
        let ent = map.entrance(&m.entrance).clone();
        let yaw = snap_quarter(ent.van_yaw);
        let van = Van {
            pos: ent.van,
            yaw,
            doors_open: false,
            door_t: 0,
            leaving: None,
        };
        let mut rng = Rng::new(s.seed);
        let players = s.guns.len();
        let raiders: Vec<Raider> = (0..players)
            .map(|i| Raider::new(van.seat(i), yaw, s.guns[i].mag))
            .collect();
        let mut run = Run {
            tick: 0,
            rng: Rng::new(s.seed ^ 0x5eed),
            raiders,
            packs: vec![Vec::new(); players],
            drones: Vec::new(),
            shots: Vec::new(),
            pickups: Vec::new(),
            searched: vec![false; map.spots.len()],
            van,
            hive: None,
            generator: Generator::Off,
            wave_t: 0,
            waves_left: m.waves,
            found: 0,
            over: None,
            events: Vec::new(),
            extra: Vec::new(),
            noise: Vec::new(),
            gadgets: Vec::new(),
            kits: s.kits.clone(),
        };
        run.extra = run.van.boxes();
        for i in 0..players {
            let k = &s.kits[i];
            let ra = &mut run.raiders[i];
            ra.max_hp = k.max_hp;
            ra.hp = k.max_hp;
            ra.charges = k.charges;
            let ammo = s.guns[i].ammo;
            run.packs[i] = ammo_stacks(c, ammo, s.ammo_in[i]);
        }
        run.populate(r, &mut rng, &ent.door);
        // mission things lie at their spots
        if m.goal == Goal::Fetch {
            for key in &m.spots {
                if let Some(spot) = map.spots.iter().find(|sp| sp.key == *key) {
                    run.pickups.push(Pickup {
                        pos: spot.at,
                        item: Item::Mission,
                        vy: 0,
                    });
                }
            }
        }
        // for the queen, the client left rounds behind the van for every gun
        if m.goal == Goal::Queen {
            let back = facing(run.van.yaw, 0);
            let side = right_of(run.van.yaw);
            for i in 0..players {
                let a = s.guns[i].ammo;
                let n = crate::gun::starting_rounds(&s.guns[i], average_hp(c)) * QUEEN_SUPPLY_PCT / 100;
                for (k, item) in ammo_stacks(c, a, n).into_iter().enumerate() {
                    let at = run
                        .van
                        .pos
                        .add(back.scale(int(3) + int(1) * k as i32))
                        .add(side.scale((i as i32 * 2 - 3) * ONE));
                    run.pickups.push(Pickup { pos: at, item, vy: 0 });
                }
            }
        }
        if m.goal == Goal::Hive {
            if let Some(spot) = map.spots.iter().find(|sp| sp.key == "hive") {
                run.hive = Some(Hive {
                    pos: spot.at,
                    hp: HIVE_HP,
                    pods: [HIVE_POD_HP; 6],
                    spawn_t: HIVE_SPAWN_EVERY / 2,
                });
            }
        }
        run
    }

    fn spawn(&mut self, c: &Content, map: &Map, kind: &str, at: V3, rng: &mut Rng, nest: bool) -> usize {
        let def = c.drone(kind);
        let k = Kind::from_id(kind);
        let outdoors = !map.zone_at(at).is_some_and(|z| z.indoors);
        let hover = if k.walks() {
            0
        } else if outdoors {
            OUTDOOR_HOVER
        } else {
            HOVER_LOW + int(1)
        };
        let pos = v3(at.x, at.y + hover, at.z);
        let mut d = Drone::new(k, def, &c.drones[def], pos, rng.range(0, fx::TURN - 1), at);
        d.nest = nest;
        // a patrol loop: three more waypoints within 30 m
        let mut route = vec![at];
        for _ in 0..12 {
            if route.len() >= 4 {
                break;
            }
            let n = rng.range(0, map.nodes.len() as i32 - 1) as usize;
            let p = map.nodes[n];
            if p.dist(at) < int(30) && p.dist(at) > int(6) && (p.y - at.y).abs() < int(2) {
                route.push(p);
            }
        }
        if route.len() < 3 {
            for k in 0..3 {
                let a = deg(120) * k;
                let p = at.add(facing(a, 0).scale(int(6)));
                if map.clear(v3(at.x, at.y + int(2), at.z), v3(p.x, p.y + int(2), p.z), &[]) {
                    route.push(p);
                }
            }
        }
        d.brain.patrol = route;
        self.drones.push(d);
        self.drones.len() - 1
    }

    fn populate(&mut self, r: &Ref, rng: &mut Rng, door: &V3) {
        let (c, map, s) = (r.content, r.map, r.setup);
        let m = &c.missions[s.mission];
        let players = s.guns.len() as i32;
        let indoor: Vec<usize> = (0..map.nodes.len())
            .filter(|&i| {
                let p = map.nodes[i];
                map.zone_at(p).is_some_and(|z| z.indoors) && p.dist(*door) > int(40)
            })
            .collect();
        let outdoor: Vec<usize> = (0..map.nodes.len())
            .filter(|&i| {
                let p = map.nodes[i];
                !map.zone_at(p).is_some_and(|z| z.indoors)
                    && p.dist(self.van.pos) > int(80)
                    && p.len_xz() < int(200)
            })
            .collect();
        // a few out in the lot
        for k in 0..(3 + players) {
            if outdoor.is_empty() {
                break;
            }
            let n = outdoor[rng.range(0, outdoor.len() as i32 - 1) as usize];
            let kind = if k % 3 == 2 { "moth" } else { "wasp" };
            self.spawn(c, map, kind, map.nodes[n], rng, false);
        }
        // the heavy ones on the main ways through, so a run meets them: the
        // concourse and the centre court, on wide beats
        let main: Vec<usize> = indoor
            .iter()
            .copied()
            .filter(|&i| {
                map.zone_at(map.nodes[i])
                    .is_some_and(|z| z.key == "concourse" || z.key == "centercourt")
            })
            .collect();
        let wardens = 1 + players / 3;
        let striders = 2 + players / 3;
        for k in 0..(wardens + striders) {
            if main.is_empty() {
                break;
            }
            let n = main[rng.range(0, main.len() as i32 - 1) as usize];
            let kind = if k < wardens { "warden" } else { "strider" };
            let j = self.spawn(c, map, kind, map.nodes[n], rng, false);
            self.widen_beat(j, map, rng);
        }
        // most inside
        let count = (m.drones * (2 + players) / 6 - wardens - striders).max(0);
        let weights = [
            ("wasp", 38),
            ("moth", 10),
            ("hornet", 17),
            ("warden", 4),
            ("tick", 21),
            ("strider", 10),
        ];
        for _ in 0..count {
            if indoor.is_empty() {
                break;
            }
            let n = indoor[rng.range(0, indoor.len() as i32 - 1) as usize];
            let mut roll = rng.range(0, 99);
            let mut kind = "wasp";
            for (k, w) in weights {
                if roll < w {
                    kind = k;
                    break;
                }
                roll -= w;
            }
            self.spawn(c, map, kind, map.nodes[n], rng, false);
        }
        // the queen, on her beat up Main Street, stronger for every raider
        if m.goal == Goal::Queen {
            if let Some(spot) = map.spots.iter().find(|sp| sp.key == "queen") {
                let j = self.spawn(c, map, "queen", spot.at, rng, true);
                let beat: Vec<V3> = map
                    .spots
                    .iter()
                    .filter(|sp| sp.key == "queen_beat")
                    .map(|sp| sp.at)
                    .collect();
                let q = &mut self.drones[j];
                q.yaw = 0;
                q.max_hp = q.max_hp * (100 + QUEEN_HP_PER_RAIDER_PCT * (players - 1)) / 100;
                q.hp = q.max_hp;
                if !beat.is_empty() {
                    q.brain.patrol = beat;
                }
            }
        }
        // the food court's nest
        if m.goal == Goal::Clear {
            let zone = map.zone(&m.zone).cloned();
            if let Some(z) = zone {
                let inside: Vec<usize> = (0..map.nodes.len())
                    .filter(|&i| {
                        let p = map.nodes[i];
                        p.x >= z.min.x && p.x <= z.max.x && p.z >= z.min.z && p.z <= z.max.z
                    })
                    .collect();
                let nest = m.nest * (2 + players) / 6;
                for k in 0..nest.max(4) {
                    if inside.is_empty() {
                        break;
                    }
                    let n = inside[rng.range(0, inside.len() as i32 - 1) as usize];
                    let kind = ["wasp", "wasp", "tick", "hornet", "strider", "moth"][k as usize % 6];
                    self.spawn(c, map, kind, map.nodes[n], rng, true);
                }
            }
        }
    }

    /// A heavy drone's beat: four waypoints anywhere within 60 m on its floor.
    fn widen_beat(&mut self, j: usize, map: &Map, rng: &mut Rng) {
        let at = self.drones[j].brain.home;
        let mut route = vec![at];
        for _ in 0..40 {
            if route.len() >= 5 {
                break;
            }
            let p = map.nodes[rng.range(0, map.nodes.len() as i32 - 1) as usize];
            let indoors = map.zone_at(p).is_some_and(|z| z.indoors);
            if indoors && p.dist(at) < int(60) && p.dist(at) > int(15) && (p.y - at.y).abs() < int(2) {
                route.push(p);
            }
        }
        if route.len() >= 3 {
            self.drones[j].brain.patrol = route;
        }
    }

    pub fn players(&self) -> usize {
        self.raiders.len()
    }

    // ---- the tick -----------------------------------------------------------------

    pub fn step(&mut self, r: &Ref, inputs: &[Input]) {
        self.events.clear();
        self.noise.clear();
        if self.over.is_some() {
            self.tick += 1;
            return;
        }
        let map = r.map;
        for i in 0..self.raiders.len() {
            let inp = inputs.get(i).copied().unwrap_or_default();
            self.raider_tick(r, i, &inp);
        }
        self.gadgets_tick(r);
        self.drones_tick(r);
        self.shots_tick(r);
        self.pickups_tick(map);
        self.mission_tick(r);
        self.van_tick(r);
        self.tick += 1;
    }

    // ---- raiders ------------------------------------------------------------------

    fn raider_tick(&mut self, r: &Ref, i: usize, inp: &Input) {
        let gun = &r.setup.guns[i];
        let kit = r.setup.kits[i];
        if inp.b & B_GONE != 0 {
            let me = &mut self.raiders[i];
            if me.life.active() {
                me.life = Life::Gone;
                let pos = me.pos;
                let pack = std::mem::take(&mut self.packs[i]);
                self.drop_items(pos, pack);
            }
            return;
        }
        if !self.raiders[i].life.active() {
            return;
        }
        // painted ground speeds a raider up
        let boost = if self.on_paint(self.raiders[i].pos) {
            PAINT_BOOST
        } else {
            100
        };
        let me = &mut self.raiders[i];
        me.hurt_flash = (me.hurt_flash - 1).max(0);
        me.firing = (me.firing - 1).max(0);
        me.step_move_kit(inp, r.map, &self.extra, gun.move_pct * boost / 100, &kit.moves());
        if me.pos.y < -int(5) {
            me.pos.y = 0;
        }
        // footsteps, which drones hear: loud running, nothing crouched
        let step_noise = if me.landed {
            int(9)
        } else if !me.moving || !me.grounded || me.crouch {
            0
        } else if me.sprinting {
            int(13)
        } else {
            int(6)
        } * kit.noise_pct
            / 100;
        if step_noise > 0 && (self.tick + i as u32).is_multiple_of(6) {
            self.noise.push(Noise {
                at: me.pos,
                radius: step_noise,
                who: Some(i as u8),
            });
        }
        if inp.d > 0 {
            self.drop_from_pack(i, inp.d as usize - 1);
        }
        let me = &mut self.raiders[i];
        let pressed = |b: u32| inp.b & b != 0 && me.prev & b == 0;
        let held = |b: u32| inp.b & b != 0;
        if me.life == Life::Downed {
            me.bleed -= 1;
            if me.bleed <= 0 {
                me.life = Life::Dead;
                let pos = me.pos;
                self.events.push(Event::Died { who: i as u8 });
                let pack = std::mem::take(&mut self.packs[i]);
                self.drop_items(pos, pack);
            }
            self.raiders[i].prev = inp.b;
            return;
        }
        // health creeps back after a while unhurt, up to the kit's threshold
        me.since_hurt += 1;
        if me.since_hurt > REGEN_AFTER && me.hp < kit.regen_to {
            me.regen_t += kit.regen_pct;
            if me.regen_t >= REGEN_EVERY * 100 {
                me.regen_t = 0;
                me.hp += 1;
            }
        }
        // the melee swing: a wind-up, the blow, a recovery
        me.melee_cd = (me.melee_cd - 1).max(0);
        if me.melee_t > 0 {
            me.melee_t += 1;
            if me.melee_t > MELEE_RECOVER {
                me.melee_t = 0;
            }
        }
        let swing = pressed(B_MELEE) && me.melee_cd == 0 && me.melee_t == 0;
        if swing {
            me.melee_t = 1;
            me.melee_cd = kit.melee.cooldown;
            me.reloading = 0;
        }
        let blow = me.melee_t == MELEE_HIT;
        let gadget = pressed(B_GADGET) && me.charges > 0 && kit.gadget.is_some();
        // the gun
        me.cooldown = (me.cooldown - 1).max(0);
        let mut reload_done = false;
        if me.reloading > 0 {
            me.reloading -= 1;
            reload_done = me.reloading == 0;
        }
        let wants_reload = pressed(B_RELOAD) && me.ammo < gun.mag;
        let trigger = if gun.auto || gun.gatling {
            held(B_FIRE)
        } else {
            pressed(B_FIRE)
        };
        if gun.gatling {
            me.spin = if held(B_FIRE) {
                (me.spin + 1).min(RELOAD_GATLING_SPIN)
            } else {
                (me.spin - 2).max(0)
            };
        }
        let dry_click = trigger && me.ammo == 0 && me.prev & B_FIRE == 0;
        let swinging = me.melee_t > 0;
        let spun = !gun.gatling || me.spin >= RELOAD_GATLING_SPIN;
        let can_fire = trigger && me.reloading == 0 && me.cooldown == 0 && me.ammo > 0 && spun && !swinging;
        let use_held = held(B_USE);
        self.raiders[i].prev = inp.b;
        if reload_done {
            // the magazine fills from the rounds in the pack
            let want = (gun.mag - self.raiders[i].ammo) * gun.per_volley;
            let got = self.take_rounds(i, gun.ammo, want);
            let volleys = got / gun.per_volley.max(1);
            let spare = got - volleys * gun.per_volley.max(1);
            if spare > 0 {
                self.give_rounds(i, gun.ammo, spare);
            }
            self.raiders[i].ammo += volleys;
        }
        let can_reload =
            self.raiders[i].reloading == 0 && !swinging && self.rounds(i, gun.ammo) >= gun.per_volley;
        if can_reload && (wants_reload || (trigger && self.raiders[i].ammo == 0)) {
            self.raiders[i].reloading = gun.reload * kit.reload_pct / 100;
            self.events.push(Event::Reload { who: i as u8 });
        } else if dry_click && !can_reload && self.raiders[i].reloading == 0 {
            self.events.push(Event::NoAmmo { who: i as u8 });
        }
        if can_fire && !self.in_van(i) {
            self.fire(r, i);
        }
        if blow {
            self.melee(r, i);
        }
        if gadget && !self.in_van(i) {
            self.use_gadget(r, i);
        }
        self.use_tick(r, i, use_held);
    }

    /// Rounds of one ammunition in a raider's pack.
    pub fn rounds(&self, i: usize, ammo: usize) -> i32 {
        self.packs[i]
            .iter()
            .map(|it| match it {
                Item::Ammo(a, n) if *a as usize == ammo => *n as i32,
                _ => 0,
            })
            .sum()
    }

    fn take_rounds(&mut self, i: usize, ammo: usize, want: i32) -> i32 {
        let mut got = 0;
        for it in self.packs[i].iter_mut() {
            if let Item::Ammo(a, n) = it {
                if *a as usize == ammo && got < want {
                    let take = (*n as i32).min(want - got);
                    *n -= take as u16;
                    got += take;
                }
            }
        }
        self.packs[i].retain(|it| !matches!(it, Item::Ammo(_, 0)));
        got
    }

    fn give_rounds(&mut self, i: usize, ammo: usize, n: i32) {
        self.packs[i].push(Item::Ammo(ammo as u8, n as u16));
    }

    /// Slots a pack has free (the order's things do not take one).
    fn pack_room(&self, r: &Ref, i: usize) -> i32 {
        let used = self.packs[i].iter().filter(|it| **it != Item::Mission).count() as i32;
        r.setup.kits[i].slots - used
    }

    fn drop_from_pack(&mut self, i: usize, k: usize) {
        if k >= self.packs[i].len() {
            return;
        }
        let item = self.packs[i].remove(k);
        if item == Item::Mission {
            self.found -= 1;
        }
        let me = &self.raiders[i];
        let at = me.pos.add(facing(me.yaw, 0).scale(ONE * 8 / 10));
        self.pickups.push(Pickup {
            pos: v3(at.x, me.pos.y + ONE / 2, at.z),
            item,
            vy: int(2),
        });
    }

    /// Is a point on a painted strip?
    pub fn on_paint(&self, p: V3) -> bool {
        self.gadgets.iter().any(|g| {
            if g.kind != Gadget::Paint {
                return false;
            }
            let rel = p.sub(g.pos);
            let along = rel.dot(facing(g.yaw, 0));
            let across = rel.dot(right_of(g.yaw));
            along.abs() <= PAINT_LONG / 2 && across.abs() <= PAINT_WIDE / 2 && (p.y - g.pos.y).abs() < int(3)
        })
    }

    /// Does smoke stand between two points?
    pub fn smoked(&self, a: V3, b: V3) -> bool {
        self.gadgets.iter().any(|g| {
            if g.kind != Gadget::Smoke || !g.landed {
                return false;
            }
            let c = v3(g.pos.x, g.pos.y + int(2), g.pos.z);
            let d = b.sub(a);
            let len = d.len().max(1);
            let dir = d.norm();
            let t = c.sub(a).dot(dir).clamp(0, len);
            a.add(dir.scale(t)).dist(c) < SMOKE_RADIUS * 9 / 10
        })
    }

    /// The blow of a melee swing: the nearest thing in reach along the look,
    /// or anything in the swing's arc. A part it lands on takes extra.
    fn melee(&mut self, r: &Ref, i: usize) {
        let kit = r.setup.kits[i];
        let me = &self.raiders[i];
        let from = me.chest();
        let dir = facing(me.yaw, me.pitch);
        let reach = kit.melee.reach * ONE / 100;
        self.noise.push(Noise {
            at: me.pos,
            radius: int(8),
            who: Some(i as u8),
        });
        let mut best: Option<(i32, usize, Zone)> = None;
        for (k, d) in self.drones.iter().enumerate() {
            if !d.active() || d.centre().dist(from) > reach + int(3) {
                continue;
            }
            if let Some((t, z)) = d.ray(from, dir, reach) {
                if best.is_none_or(|b| t < b.0) {
                    best = Some((t, k, z));
                }
            }
        }
        if best.is_none() {
            // a sweep: anything in the arc in front, at body height
            let half = deg(kit.melee.arc) / 2;
            for (k, d) in self.drones.iter().enumerate() {
                if !d.active() {
                    continue;
                }
                let to = d.centre().sub(from);
                let dist = to.len();
                if dist > reach + ONE / 2 {
                    continue;
                }
                let off = fx::wrap(yaw_pitch_of(to).0 - me.yaw).abs();
                if off <= half && best.is_none_or(|b| dist < b.0) {
                    best = Some((dist, k, Zone::Body));
                }
            }
        }
        let Some((t, k, z)) = best else {
            self.events.push(Event::Melee {
                who: i as u8,
                at: from.add(dir.scale(reach)),
                hit: false,
            });
            return;
        };
        let at = from.add(dir.scale(t.min(reach)));
        let dmg = match z {
            Zone::Piece(_) => kit.melee.damage * kit.melee.rips / 100,
            _ => kit.melee.damage,
        };
        let push = dir.scale(ONE * kit.melee.knock / 4);
        self.strike_drone(r.content, k, z, dmg, push.norm(), at, i as u8, Effect::None, 0);
        let d = &mut self.drones[k];
        d.vel = d.vel.add(push);
        d.stun = d.stun.max(secs(150));
        self.events.push(Event::Melee {
            who: i as u8,
            at,
            hit: true,
        });
    }

    fn use_gadget(&mut self, r: &Ref, i: usize) {
        let Some(kind) = r.setup.kits[i].gadget else {
            return;
        };
        let me = &mut self.raiders[i];
        me.charges -= 1;
        let fwd = facing(me.yaw, 0);
        let (pos, vel, life, landed) = match kind {
            Gadget::Mine => (me.pos, V3::ZERO, secs(180_000), true),
            Gadget::Paint => (
                me.pos.add(fwd.scale(PAINT_LONG / 2 + ONE / 2)),
                V3::ZERO,
                secs(40_000),
                true,
            ),
            Gadget::Decoy | Gadget::Smoke => {
                let throw = facing(me.yaw, me.pitch + deg(18)).scale(int(15));
                (me.chest(), throw, secs(14_000), false)
            }
        };
        let yaw = me.yaw;
        self.gadgets.push(Placed {
            kind,
            pos: v3(pos.x, if landed { me.pos.y + ONE / 50 } else { pos.y }, pos.z),
            vel,
            yaw,
            life,
            landed,
            owner: i as u8,
            armed: secs(1000),
        });
        self.events.push(Event::Gadget {
            kind,
            at: pos,
            by: i as u8,
        });
    }

    fn gadgets_tick(&mut self, r: &Ref) {
        let mut boom: Vec<(V3, u8)> = Vec::new();
        for g in self.gadgets.iter_mut() {
            g.life -= 1;
            g.armed = (g.armed - 1).max(0);
            if !g.landed {
                g.vel.y -= SHOT_GRAVITY / TICKS_PER_SECOND;
                let step = g.vel.scale(ONE / TICKS_PER_SECOND);
                let len = step.len();
                match r.map.ray(g.pos, step.norm(), len, &self.extra, true) {
                    Some(h) => {
                        g.pos = h.at.add(h.normal.scale(ONE / 20));
                        if h.normal.y > ONE / 2 {
                            g.landed = true;
                            g.vel = V3::ZERO;
                        } else {
                            let into = g.vel.dot(h.normal);
                            g.vel = g.vel.sub(h.normal.scale(into * 2)).scale(ONE / 2);
                        }
                    }
                    None => g.pos = g.pos.add(step),
                }
                continue;
            }
            match g.kind {
                Gadget::Decoy => {
                    // a beep every second: drones come to see
                    if g.life % TICKS_PER_SECOND == 0 {
                        self.noise.push(Noise {
                            at: g.pos,
                            radius: DECOY_NOISE,
                            who: None,
                        });
                        self.events.push(Event::Gadget {
                            kind: Gadget::Decoy,
                            at: g.pos,
                            by: g.owner,
                        });
                    }
                    if g.life > secs(7000) {
                        g.life = secs(7000);
                    }
                }
                Gadget::Smoke => {
                    if g.life > secs(12_000) {
                        g.life = secs(12_000);
                    }
                }
                Gadget::Mine => {
                    if g.armed == 0
                        && self.drones.iter().any(|d| {
                            d.active()
                                && d.centre().sub(g.pos).len_xz() < MINE_RADIUS
                                && d.centre().y - g.pos.y < int(4)
                        })
                    {
                        boom.push((g.pos, g.owner));
                        g.life = 0;
                    }
                }
                Gadget::Paint => {}
            }
        }
        self.gadgets.retain(|g| g.life > 0);
        for (at, by) in boom {
            self.explode(
                r,
                v3(at.x, at.y + ONE / 2, at.z),
                MINE_BLAST,
                MINE_DAMAGE,
                Effect::None,
                0,
                Some(by),
            );
        }
    }

    fn in_van(&self, i: usize) -> bool {
        !self.van.doors_open && self.van.inside().contains(self.raiders[i].chest())
    }

    /// Where a raider's crosshair lands: the camera's ray, through drones too.
    pub fn aim_point(&self, r: &Ref, i: usize) -> V3 {
        let me = &self.raiders[i];
        let (eye, fwd) = raider::camera_at(
            me.pos,
            me.yaw,
            me.pitch,
            me.aiming,
            me.eye_up(),
            r.map,
            &self.extra,
        );
        let mut t = r
            .map
            .ray(eye, fwd, AIM_RANGE, &self.extra, true)
            .map(|h| h.t)
            .unwrap_or(AIM_RANGE);
        for d in &self.drones {
            if d.active() {
                if let Some((dt, _)) = d.ray(eye, fwd, t) {
                    t = t.min(dt);
                }
            }
        }
        if let Some(h) = &self.hive {
            if h.hp > 0 {
                if let Some(ht) = drone::sphere(eye, fwd, h.core(), int(2), t) {
                    t = t.min(ht);
                }
            }
        }
        // never closer than just past the raider, so the muzzles converge ahead
        eye.add(fwd.scale(t.max(int(3))))
    }

    fn fire(&mut self, r: &Ref, i: usize) {
        let gun = &r.setup.guns[i];
        let aim = self.aim_point(r, i);
        let me = &self.raiders[i];
        let frame = gun_frame(me);
        let fwd = facing(me.yaw, me.pitch);
        let mut spread = if me.aiming { gun.aim_spread } else { gun.hip_spread };
        if me.moving && !me.aiming {
            spread = spread * 16 / 10;
        }
        let muzzles: Vec<V3> = if gun.gatling {
            vec![gun.muzzles[me.next_muzzle % gun.muzzles.len()]]
        } else {
            gun.muzzles.clone()
        };
        let mut first = None;
        for local in muzzles {
            let at = frame.apply(local);
            first.get_or_insert(at);
            let mut dir = aim.sub(at).norm();
            if dir.dot(fwd) < ONE / 5 {
                dir = fwd;
            }
            let (yaw, pitch) = yaw_pitch_of(dir);
            for _ in 0..gun.pellets {
                let jy = self.rng.range(-spread, spread);
                let jp = self.rng.range(-spread, spread);
                let d = facing(yaw + jy, pitch + jp);
                self.shots.push(Shot {
                    from: ShotFrom::Raider(i as u8),
                    kind: gun.kind,
                    pos: at,
                    vel: d.scale(gun.speed),
                    life: match gun.kind {
                        ShotKind::Bullet => SHOT_LIFE,
                        ShotKind::Rocket => ROCKET_LIFE,
                        ShotKind::Grenade => GRENADE_FUSE,
                    },
                    damage: gun.damage,
                    splash: gun.splash,
                    effect: gun.effect,
                    power: gun.power,
                    pierce: gun.pierce,
                    heavy: false,
                });
            }
        }
        let me = &mut self.raiders[i];
        me.ammo -= 1;
        me.cooldown = gun.cooldown;
        me.next_muzzle = me.next_muzzle.wrapping_add(1);
        me.firing = 6;
        let at = first.unwrap_or(me.chest());
        self.events.push(Event::Fire {
            who: i as u8,
            at,
            kind: gun.kind,
        });
        self.noise.push(Noise {
            at: me.pos,
            radius: NOISE,
            who: Some(i as u8),
        });
    }

    /// What E would do for raider `i` now.
    pub fn use_target(&self, r: &Ref, i: usize) -> Option<Use> {
        let me = &self.raiders[i];
        if me.life != Life::Alive {
            return None;
        }
        let p = me.pos;
        let near = |q: V3| q.sub(p).len_xz() < REACH && (q.y - p.y).abs() < int(2);
        for (j, o) in self.raiders.iter().enumerate() {
            if j != i && o.life == Life::Downed && near(o.pos) {
                return Some(Use::Revive(j as u8));
            }
        }
        let mut best: Option<(i32, Use)> = None;
        for (k, pk) in self.pickups.iter().enumerate() {
            if near(pk.pos) {
                let d = pk.pos.sub(p).len_xz();
                if best.is_none_or(|b| d < b.0) {
                    best = Some((d, Use::Pick(k as u16)));
                }
            }
        }
        if let Some(b) = best {
            return Some(b.1);
        }
        for (k, s) in r.map.spots.iter().enumerate() {
            if s.kind == SpotKind::Loot && !self.searched[k] && near(s.at) {
                return Some(Use::Search(k as u16));
            }
        }
        let m = &r.content.missions[r.setup.mission];
        if m.goal == Goal::Power && self.generator == Generator::Off {
            if let Some(s) = r.map.spots.iter().find(|s| s.key == "generator") {
                if near(s.at) {
                    return Some(Use::Generator);
                }
            }
        }
        if self.van.inside().contains(me.chest()) {
            if !self.van.doors_open {
                return Some(Use::Doors);
            }
            if self.van.leaving.is_none() && self.tick > secs(5000) as u32 {
                return Some(Use::Leave);
            }
        }
        None
    }

    /// How long raider `i` holds E for this, with what their gloves do.
    pub fn needs_for(&self, r: &Ref, i: usize, u: Use) -> i32 {
        Self::needs_with(r, i, u)
    }

    fn needs_with(r: &Ref, i: usize, u: Use) -> i32 {
        let k = &r.setup.kits[i];
        match u {
            Use::Search(_) => u.needs() * k.search_pct / 100,
            Use::Revive(_) => u.needs() * k.revive_pct / 100,
            _ => u.needs(),
        }
    }

    fn use_tick(&mut self, r: &Ref, i: usize, held: bool) {
        let target = if held { self.use_target(r, i) } else { None };
        let me = &mut self.raiders[i];
        let Some(u) = target else {
            me.use_t = 0;
            me.use_on = -1;
            return;
        };
        if me.use_on != u.code() {
            me.use_on = u.code();
            me.use_t = 0;
        }
        me.use_t += 1;
        let needs = Self::needs_with(r, i, u);
        if me.use_t < needs {
            return;
        }
        me.use_t = 0;
        me.use_on = -1;
        match u {
            Use::Revive(j) => {
                let o = &mut self.raiders[j as usize];
                o.life = Life::Alive;
                o.hp = r.setup.kits[i].revive_hp.min(o.max_hp);
                o.since_hurt = 0;
                self.events.push(Event::Revived { who: j });
            }
            Use::Pick(k) => {
                let pk = self.pickups[k as usize];
                if let Item::Ammo(a, n) = pk.item {
                    // rounds go on to the stacks there are, then into a new one
                    let stack = r.content.ammo[a as usize].stack;
                    let mut left = n as i32;
                    for it in self.packs[i].iter_mut() {
                        if let Item::Ammo(b, m) = it {
                            if *b == a && (*m as i32) < stack {
                                let put = (stack - *m as i32).min(left);
                                *m += put as u16;
                                left -= put;
                            }
                        }
                    }
                    while left > 0 && self.pack_room(r, i) > 0 {
                        let put = left.min(stack);
                        self.packs[i].push(Item::Ammo(a, put as u16));
                        left -= put;
                    }
                    if left > 0 {
                        self.pickups[k as usize].item = Item::Ammo(a, left as u16);
                    } else {
                        self.pickups.remove(k as usize);
                    }
                    if left < n as i32 {
                        self.events.push(Event::Picked {
                            who: i as u8,
                            item: pk.item,
                        });
                    }
                    return;
                }
                let room = self.pack_room(r, i) > 0;
                if pk.item == Item::Mission || room {
                    self.pickups.remove(k as usize);
                    self.packs[i].push(pk.item);
                    if pk.item == Item::Mission {
                        self.found += 1;
                        self.events.push(Event::Objective);
                    }
                    self.events.push(Event::Picked {
                        who: i as u8,
                        item: pk.item,
                    });
                }
            }
            Use::Search(k) => {
                self.searched[k as usize] = true;
                let at = r.map.spots[k as usize].at;
                let n = self.rng.range(0, 2) + i32::from(self.rng.chance(30));
                for _ in 0..n {
                    let item = self.roll_loot(r.content, 0);
                    self.pickups.push(Pickup {
                        pos: v3(at.x, at.y + ONE / 2, at.z),
                        item,
                        vy: int(3),
                    });
                }
                self.events.push(Event::Searched { at });
            }
            Use::Generator => {
                let m = &r.content.missions[r.setup.mission];
                self.generator = Generator::Running(m.hold);
                self.wave_t = secs(3000);
                self.events.push(Event::Generator);
            }
            Use::Doors => {
                self.open_doors();
            }
            Use::Leave => {
                self.van.leaving = Some(VAN_LEAVE);
                self.events.push(Event::VanStart);
            }
        }
    }

    fn open_doors(&mut self) {
        if !self.van.doors_open {
            self.van.doors_open = true;
            self.extra = self.van.boxes();
            self.events.push(Event::Doors);
        }
    }

    /// A random part to find, rarer ones less often; `luck` shifts the odds.
    /// What a container gives: rounds most often, then a part, now and
    /// then a piece of gear.
    fn roll_loot(&mut self, c: &Content, luck: i32) -> Item {
        let roll = self.rng.range(0, 99);
        if roll < 45 {
            return self.roll_ammo(c);
        }
        if roll < 85 {
            return self.roll_part(c, luck);
        }
        let weight = |g: &crate::content::GearDef| match g.rarity {
            1 => 6,
            2 => 3 + luck,
            3 => 1 + luck,
            _ => 0,
        };
        let total: i32 = c.gear.iter().map(weight).sum();
        let mut roll = self.rng.range(0, total.max(1) - 1);
        for (k, g) in c.gear.iter().enumerate() {
            let w = weight(g);
            if roll < w {
                return Item::Gear(k as u16);
            }
            roll -= w;
        }
        self.roll_part(c, luck)
    }

    /// Rounds: plenty of the common kinds, a few rockets.
    fn roll_ammo(&mut self, c: &Content) -> Item {
        let weight = |rarity: i32| match rarity {
            1 => 10,
            2 => 5,
            _ => 2,
        };
        let total: i32 = c.ammo.iter().map(|a| weight(a.rarity)).sum();
        let mut roll = self.rng.range(0, total - 1);
        for (k, a) in c.ammo.iter().enumerate() {
            let w = weight(a.rarity);
            if roll < w {
                let n = a.stack / 2 + self.rng.range(0, a.stack / 2);
                return Item::Ammo(k as u8, n.max(1) as u16);
            }
            roll -= w;
        }
        Item::Ammo(0, 10)
    }

    fn roll_part(&mut self, c: &Content, luck: i32) -> Item {
        let weight = |rarity: i32| -> i32 {
            match rarity {
                1 => 10,
                2 => 5 + luck,
                3 => 2 + luck,
                _ => 0,
            }
        };
        let total: i32 = c.parts.iter().map(|p| weight(p.rarity)).sum();
        let mut roll = self.rng.range(0, total.max(1) - 1);
        for (k, p) in c.parts.iter().enumerate() {
            let w = weight(p.rarity);
            if roll < w {
                return Item::Part(k as u16);
            }
            roll -= w;
        }
        Item::Part(0)
    }

    fn drop_items(&mut self, at: V3, items: Vec<Item>) {
        for (k, item) in items.into_iter().enumerate() {
            let a = deg(47) * k as i32;
            let p = at.add(facing(a, 0).scale(ONE / 2));
            self.pickups.push(Pickup {
                pos: v3(p.x, at.y + ONE / 2, p.z),
                item,
                vy: int(2),
            });
        }
    }

    fn pickups_tick(&mut self, map: &Map) {
        for pk in self.pickups.iter_mut() {
            if pk.vy != 0 || pk.pos.y > 0 {
                pk.vy -= GRAVITY / TICKS_PER_SECOND;
                let next = pk.pos.y + pk.vy / TICKS_PER_SECOND;
                let floor = map.floor_under(
                    &Aabb::standing(pk.pos, ONE / 8, ONE / 8),
                    pk.pos.y + ONE / 8,
                    &self.extra,
                );
                if next <= floor {
                    pk.pos.y = floor;
                    pk.vy = 0;
                } else {
                    pk.pos.y = next;
                }
            }
        }
    }

    // ---- drones ------------------------------------------------------------------------

    fn seen(&self) -> Vec<Seen> {
        self.raiders
            .iter()
            .enumerate()
            .map(|(i, r)| Seen {
                pos: r.pos,
                chest: r.chest(),
                head: r.head(),
                yaw: r.yaw,
                pitch: r.pitch,
                vel: r.vel,
                notice: {
                    let stance = if r.firing > 0 || r.melee_t > 0 {
                        300
                    } else if r.roll_t > 0 || r.slide_t > 0 {
                        120
                    } else if r.crouch {
                        if r.moving {
                            40
                        } else {
                            25
                        }
                    } else if r.sprinting {
                        160
                    } else if r.moving {
                        100
                    } else {
                        60
                    };
                    stance * self.kits.get(i).map(|k| k.notice_pct).unwrap_or(100) / 100
                },
                active: r.life.active() && (self.van.doors_open || !self.van.inside().contains(r.chest())),
            })
            .collect()
    }

    fn drones_tick(&mut self, r: &Ref) {
        let (c, map) = (r.content, r.map);
        let seen = self.seen();
        let n = self.drones.len();
        let tick = self.tick;
        let mut orders: Vec<(usize, Order)> = Vec::new();
        let noise = self.noise.clone();
        let smoke: Vec<V3> = self
            .gadgets
            .iter()
            .filter(|g| g.kind == Gadget::Smoke && g.landed)
            .map(|g| v3(g.pos.x, g.pos.y + int(2), g.pos.z))
            .collect();
        let painted: Vec<usize> = (0..n)
            .filter(|&k| {
                let d = &self.drones[k];
                d.active() && (d.kind.walks() || d.pos.y < int(3)) && self.on_paint(d.pos)
            })
            .collect();
        for k in 0..n {
            let def = &c.drones[self.drones[k].def];
            let d = &mut self.drones[k];
            if !d.active() {
                if d.state != State::Wreck {
                    let before = d.state;
                    d.step_body(def, V3::ZERO, d.yaw, map, &self.extra);
                    if d.state == State::Wreck && before != State::Wreck {
                        orders.push((k, Order::Detonate));
                    }
                }
                continue;
            }
            // too far from every raider: drift along the patrol and nothing else
            let near = seen.iter().any(|s| s.active && s.pos.dist(d.pos) < WAKE_RANGE);
            // hearing: what a drone hears raises its awareness of whoever
            // made it, and it comes to look at the spot
            for nz in &noise {
                let dist = nz.at.dist(d.pos);
                if dist >= nz.radius {
                    continue;
                }
                let close = ONE - fx::div(dist, nz.radius);
                if let Some(j) = nz.who {
                    let j = j as usize;
                    if j < d.brain.aware.len() {
                        let gain = if nz.radius >= int(30) {
                            500 + fx::mul(close, 600)
                        } else {
                            40 + fx::mul(close, 90)
                        };
                        d.brain.aware[j] = (d.brain.aware[j] + gain).min(AWARE_FULL);
                    }
                }
                if d.brain.target.is_none() {
                    d.brain.suspect = Some(nz.at);
                    d.brain.suspect_t = SUSPECT_TIME;
                }
            }
            // sight: awareness fills while a raider is in view, faster close,
            // in front of it, and the more they stand out
            if near && (tick + k as u32).is_multiple_of(LOOK_EVERY) {
                let eye = d.eye();
                let face = facing(d.yaw, 0);
                let range = int(def.sight);
                let kind_pct = match d.kind {
                    Kind::Moth => 140,
                    Kind::Hornet => 120,
                    Kind::Warden => 90,
                    Kind::Tick => 80,
                    _ => 100,
                };
                for (j, s) in seen.iter().enumerate().take(d.brain.aware.len()) {
                    let dist = s.chest.dist(eye);
                    let visible = s.active
                        && dist < range
                        && (map.clear(eye, s.chest, &self.extra) || map.clear(eye, s.head, &self.extra))
                        && !smoke.iter().any(|c| smoke_between(*c, eye, s.head));
                    if !visible {
                        d.brain.aware[j] = (d.brain.aware[j] - AWARE_FADE).max(0);
                        if d.brain.target == Some(j as u8) {
                            d.brain.sees = false;
                        }
                        continue;
                    }
                    let flat = v3(s.chest.x - eye.x, 0, s.chest.z - eye.z).norm();
                    let cos = flat.dot(face);
                    let mut cone = if cos > ONE * 64 / 100 {
                        100
                    } else if cos > -ONE * 34 / 100 {
                        30
                    } else {
                        8
                    };
                    if dist < int(3) {
                        cone = cone.max(70);
                    }
                    let dist_pct = 25 + 75 * ((range - dist) / (range / 100).max(1)) / 100;
                    let gain = AWARE_GAIN * cone / 100 * dist_pct / 100 * s.notice / 100 * kind_pct / 100;
                    let before = d.brain.aware[j];
                    // one it has fought already, it knows again at once
                    let known = d.brain.target == Some(j as u8) && before >= AWARE_KNOWN;
                    d.brain.aware[j] = if known {
                        AWARE_FULL
                    } else {
                        (before + gain.max(1)).min(AWARE_FULL)
                    };
                    if d.brain.aware[j] >= AWARE_FULL {
                        if d.brain.target != Some(j as u8)
                            && (d.brain.target.is_none() || d.brain.seen_t > secs(3000))
                        {
                            self.events.push(Event::Spotted { at: d.pos });
                        }
                        if d.brain.target.is_none()
                            || d.brain.target == Some(j as u8)
                            || d.brain.seen_t > secs(3000)
                        {
                            d.brain.target = Some(j as u8);
                            d.brain.seen_at = seen[j].pos;
                            d.brain.seen_t = 0;
                            d.brain.sees = true;
                            d.brain.ambush = None;
                            d.brain.suspect = None;
                        }
                    } else if d.brain.aware[j] >= AWARE_SUSPECT && d.brain.target.is_none() {
                        if before < AWARE_SUSPECT {
                            self.events.push(Event::Noticed { at: d.pos });
                        }
                        d.brain.suspect = Some(seen[j].pos);
                        d.brain.suspect_t = SUSPECT_TIME;
                    }
                }
            }
            let outdoors = !map.zone_at(d.pos).is_some_and(|z| z.indoors);
            if near && (tick + k as u32).is_multiple_of(ai::THINK) {
                let mut cx = Ctx {
                    map,
                    extra: &self.extra,
                    def,
                    raiders: &seen,
                    rng: &mut self.rng,
                    tick,
                    id: k,
                    orders: Vec::new(),
                    outdoors,
                };
                ai::think(d, &mut cx);
                for o in cx.orders {
                    orders.push((k, o));
                }
            } else if !near && (tick + k as u32).is_multiple_of(ai::THINK * 4) {
                let mut cx = Ctx {
                    map,
                    extra: &self.extra,
                    def,
                    raiders: &seen,
                    rng: &mut self.rng,
                    tick,
                    id: k,
                    orders: Vec::new(),
                    outdoors,
                };
                d.brain.target = None;
                ai::think(d, &mut cx);
            }
            // searching and patrolling it goes carefully; painted ground slows it
            let careful = matches!(d.brain.doing, crate::ai::A::Patrol | crate::ai::A::Search);
            let mut top = def.speed * ONE / 10 * d.speed_pct() / 100;
            if careful {
                top = top * 55 / 100;
            }
            if painted.contains(&k) {
                top = top * PAINT_SLOW / 100;
            }
            // stuck against something for a while: forget the path and shake loose
            if (tick + k as u32).is_multiple_of(30) {
                let moved = d.pos.dist(d.brain.last_pos);
                d.brain.last_pos = d.pos;
                let trying = d.brain.goal.dist(d.pos) > int(2);
                d.brain.stuck_t = if trying && moved < ONE / 3 {
                    d.brain.stuck_t + 30
                } else {
                    0
                };
                if d.brain.stuck_t >= 90 {
                    d.brain.stuck_t = 0;
                    d.brain.path.clear();
                    d.brain.replan = 0;
                    let side =
                        crate::geom::right_of(d.yaw).scale(if (tick / 30 + k as u32).is_multiple_of(2) {
                            int(3)
                        } else {
                            -int(3)
                        });
                    d.vel = d
                        .vel
                        .add(side)
                        .add(v3(0, if d.kind.walks() { 0 } else { -int(1) }, 0));
                }
            }
            let want = ai::steer(d, map, &self.extra, top, outdoors);
            let face = d.brain.face;
            let stepped = d.gait;
            d.step_body(def, want, face, map, &self.extra);
            if d.kind == Kind::Queen {
                d.pos = crate::mall::queen_clamp(d.pos);
                // a foot comes down twice a stride
                if (stepped ^ d.gait) & fx::HALF != 0 {
                    self.events.push(Event::Tread { at: d.pos });
                }
            }
            // burning
            if d.burn > 0 && d.burn % 10 == 0 {
                let by = d.burn_by;
                let at = d.pos;
                let s = d.strike(def, Zone::Body, 2, V3::ZERO, at, by);
                if s.killed {
                    self.kill(k, by);
                }
            }
        }
        // keep flyers from stacking on one another
        for a in 0..n {
            if !self.drones[a].active() || self.drones[a].kind.walks() || !(tick + a as u32).is_multiple_of(3)
            {
                continue;
            }
            for b in 0..n {
                if a == b || !self.drones[b].active() {
                    continue;
                }
                let off = self.drones[a].pos.sub(self.drones[b].pos);
                let dist = off.len();
                if dist < int(2) && dist > 0 {
                    let push = off.norm().scale(ONE / 2);
                    self.drones[a].vel = self.drones[a].vel.add(push);
                }
            }
        }
        for (k, o) in orders {
            self.order(r, k, o);
        }
    }

    fn order(&mut self, r: &Ref, k: usize, o: Order) {
        let c = r.content;
        let def = &c.drones[self.drones[k].def];
        match o {
            Order::Bolt(to) => {
                let d = &self.drones[k];
                let from = match d.kind {
                    Kind::Strider => d.to_world(0, 250, -70),
                    Kind::Warden => d.to_world(0, -30, -90),
                    Kind::Queen => d.to_world(0, 820, -910),
                    _ => d.to_world(0, -15, -40),
                };
                let dir = to.sub(from).norm();
                let (yaw, pitch) = yaw_pitch_of(dir);
                let jy = self.rng.range(-DRONE_SHOT_SPREAD, DRONE_SHOT_SPREAD);
                let jp = self.rng.range(-DRONE_SHOT_SPREAD, DRONE_SHOT_SPREAD);
                let heavy = matches!(d.kind, Kind::Warden | Kind::Queen);
                self.shots.push(Shot {
                    from: ShotFrom::Drone(k as u16),
                    kind: ShotKind::Bullet,
                    pos: from,
                    vel: facing(yaw + jy, pitch + jp).scale(def.shot_speed * ONE),
                    life: SHOT_LIFE,
                    damage: def.damage,
                    splash: if heavy { int(2) } else { 0 },
                    effect: Effect::None,
                    power: 0,
                    pierce: 0,
                    heavy,
                });
                self.events.push(Event::DroneFire { at: from, heavy });
            }
            Order::Laser(to) => {
                let from = self.drones[k].to_world(0, -8, -100);
                let dir = to.sub(from).norm();
                let len = to.dist(from) + int(2);
                let wall = r
                    .map
                    .ray(from, dir, len, &self.extra, true)
                    .map(|h| h.t)
                    .unwrap_or(len);
                let mut end = from.add(dir.scale(wall));
                let mut hit: Option<(i32, usize)> = None;
                for (j, ra) in self.raiders.iter().enumerate() {
                    if !ra.life.active() {
                        continue;
                    }
                    if let Some(t) = drone::sphere(from, dir, ra.chest(), ONE / 2, wall) {
                        if hit.is_none_or(|h| t < h.0) {
                            hit = Some((t, j));
                        }
                    }
                }
                if let Some((t, j)) = hit {
                    end = from.add(dir.scale(t));
                    self.hurt_raider(j, def.damage);
                }
                self.events.push(Event::Laser { from, to: end });
            }
            Order::Call(t, ambush) => {
                let at = self.drones[k].pos;
                let seen_at = self.drones[k].brain.seen_at;
                for (j, d) in self.drones.iter_mut().enumerate() {
                    if j == k || !d.active() || d.pos.dist(at) > int(45) {
                        continue;
                    }
                    if d.brain.target.is_none() || d.brain.target == Some(t) {
                        if let Some(a) = d.brain.aware.get_mut(t as usize) {
                            *a = AWARE_FULL;
                        }
                        d.brain.target = Some(t);
                        d.brain.seen_at = seen_at;
                        d.brain.seen_t = d.brain.seen_t.min(secs(2000));
                        if d.kind == Kind::Wasp && !d.brain.sees {
                            d.brain.ambush = Some(ambush);
                        }
                    }
                }
            }
            Order::Detonate => {
                let d = &mut self.drones[k];
                let at = d.centre();
                let kind = d.kind;
                let from_tick = kind == Kind::Tick && d.active();
                d.state = State::Wreck;
                if from_tick {
                    self.explode(r, at, int(def.blast) / 10, def.damage, Effect::None, 0, None);
                } else {
                    self.events.push(Event::Wreck { at, kind });
                    self.drop_loot(r, k);
                }
            }
            Order::Ram => {
                let d = &self.drones[k];
                let at = d.centre();
                let Some(t) = d.brain.target.map(|t| t as usize) else {
                    return;
                };
                if t < self.raiders.len() && self.raiders[t].life.active() {
                    let away = self.raiders[t].pos.sub(at).norm();
                    self.raiders[t].vel = self.raiders[t].vel.add(v3(away.x, ONE / 3, away.z).scale(int(5)));
                    self.hurt_raider(t, def.damage * 4 + 6);
                }
                // it bounces back off, shaken
                let d = &mut self.drones[k];
                d.vel = d.vel.neg().scale(ONE / 2).add(v3(0, int(2), 0));
                d.spin += deg(90);
                d.lean_v.0 -= deg(10);
                self.events.push(Event::Stomp { at });
            }
            Order::Stomp => {
                let at = self.drones[k].pos;
                let (range, damage) = if self.drones[k].kind == Kind::Queen {
                    (QUEEN_STOMP_RANGE, QUEEN_STOMP_DAMAGE)
                } else {
                    (STOMP_RANGE, STOMP_DAMAGE)
                };
                for j in 0..self.raiders.len() {
                    let ra = &mut self.raiders[j];
                    if ra.life.active() && ra.pos.dist(at) < range {
                        let away = ra.pos.sub(at).norm();
                        ra.vel = ra.vel.add(v3(away.x, ONE / 2, away.z).scale(int(6)));
                        self.hurt_raider_by(j, damage, true);
                    }
                }
                self.events.push(Event::Stomp { at });
            }
            Order::Barrage(to) => {
                // a salvo from each pod she still has, spread about the point
                for &pod in drone::QUEEN_PODS.iter() {
                    let d = &self.drones[k];
                    if d.pieces[pod] <= 0 {
                        continue;
                    }
                    let x = if pod == drone::QUEEN_PODS[0] { -330 } else { 330 };
                    let from = d.to_world(x, 1120, -280);
                    for n in 0..QUEEN_ROCKETS {
                        let aim = to.add(v3(0, ONE / 2, 0)).sub(from).norm();
                        let (yaw, pitch) = yaw_pitch_of(aim);
                        let jy = self.rng.range(-QUEEN_ROCKET_SPREAD, QUEEN_ROCKET_SPREAD);
                        let jp = self.rng.range(-QUEEN_ROCKET_SPREAD, QUEEN_ROCKET_SPREAD) / 2;
                        // one after another, not all from the same point
                        let start = from.add(facing(yaw, pitch).scale(ONE * n));
                        self.shots.push(Shot {
                            from: ShotFrom::Drone(k as u16),
                            kind: ShotKind::Rocket,
                            pos: start,
                            vel: facing(yaw + jy, pitch + jp).scale(QUEEN_ROCKET_SPEED),
                            life: ROCKET_LIFE,
                            damage: def.damage * 2,
                            splash: QUEEN_ROCKET_SPLASH,
                            effect: Effect::None,
                            power: 0,
                            pierce: 0,
                            heavy: true,
                        });
                    }
                    self.events.push(Event::DroneFire {
                        at: from,
                        heavy: true,
                    });
                }
            }
            Order::Brood => {
                let q = &self.drones[k];
                let (hatch, target, seen_at, ground) = (
                    q.to_world(0, 1150, 160),
                    q.brain.target,
                    q.brain.seen_at,
                    v3(q.pos.x, 0, q.pos.z),
                );
                let hers = self
                    .drones
                    .iter()
                    .filter(|d| d.active() && d.kind == Kind::Wasp && d.pos.dist(hatch) < int(60))
                    .count();
                let n = (QUEEN_BROOD_MOST.saturating_sub(hers) as i32).min(QUEEN_BROOD);
                let mut rng = Rng::new(self.rng.next());
                for w in 0..n {
                    let j = self.spawn(r.content, r.map, "wasp", ground, &mut rng, false);
                    let d = &mut self.drones[j];
                    d.pos = hatch.add(v3((w - 1) * int(2), 0, 0));
                    d.vel = v3((w - 1) * int(3), int(4), 0);
                    d.brain.target = target;
                    d.brain.seen_at = seen_at;
                    d.brain.seen_t = 0;
                    if let Some(t) = target {
                        if let Some(a) = d.brain.aware.get_mut(t as usize) {
                            *a = AWARE_FULL;
                        }
                    }
                }
                if n > 0 {
                    self.events.push(Event::Brood { at: hatch });
                }
            }
        }
    }

    fn drop_loot(&mut self, r: &Ref, k: usize) {
        let d = &self.drones[k];
        let at = d.pos;
        let luck = r.content.drones[d.def].loot;
        if d.kind == Kind::Queen {
            // her hoard: rare parts, gear and rounds round where she fell
            for n in 0..QUEEN_HOARD {
                let item = match n % 3 {
                    0 => self.roll_part(r.content, luck * 2),
                    1 => self.roll_loot(r.content, luck * 2),
                    _ => self.roll_ammo(r.content),
                };
                let off = facing(n * deg(360) / QUEEN_HOARD, 0).scale(int(3));
                self.pickups.push(Pickup {
                    pos: v3(at.x + off.x, int(2), at.z + off.z),
                    item,
                    vy: int(3),
                });
            }
            return;
        }
        if self.rng.chance(DRONE_DROP_PCT + luck * 10) {
            let item = if self.rng.chance(40) {
                self.roll_ammo(r.content)
            } else {
                self.roll_part(r.content, luck * 2)
            };
            let floor = r
                .map
                .floor_under(&Aabb::standing(at, ONE / 4, ONE / 4), at.y, &self.extra);
            self.pickups.push(Pickup {
                pos: v3(at.x, floor.max(at.y - int(3)), at.z),
                item,
                vy: int(2),
            });
        }
    }

    fn kill(&mut self, k: usize, by: u8) {
        if let Some(ra) = self.raiders.get_mut(by as usize) {
            ra.kills += 1;
        }
        self.events.push(Event::Kill { by });
        let _ = k;
    }

    fn hurt_raider(&mut self, j: usize, dmg: i32) {
        self.hurt_raider_by(j, dmg, false);
    }

    /// Damage to a raider, less what their gear takes: armour from
    /// everything, blast protection from explosions. A roll dodges shots.
    fn hurt_raider_by(&mut self, j: usize, dmg: i32, blast: bool) {
        let k = self.kits.get(j).copied();
        let ra = &mut self.raiders[j];
        if !ra.life.active() {
            return;
        }
        if ra.roll_t > 0 && !blast {
            return;
        }
        let mut dmg = dmg;
        if let Some(k) = k {
            dmg = dmg * (100 - k.armour_pct) / 100;
            if blast {
                dmg = dmg * k.blast_pct / 100;
            }
        }
        if ra.hurt(dmg.max(1)) {
            self.events.push(Event::Downed { who: j as u8 });
        } else {
            self.events.push(Event::Hurt { who: j as u8 });
        }
    }

    // ---- shots ---------------------------------------------------------------------------

    fn shots_tick(&mut self, r: &Ref) {
        let (c, map) = (r.content, r.map);
        let mut k = 0;
        while k < self.shots.len() {
            let mut s = self.shots[k];
            s.life -= 1;
            if s.kind == ShotKind::Grenade {
                s.vel.y -= SHOT_GRAVITY / TICKS_PER_SECOND;
            }
            let step = s.vel.scale(ONE / TICKS_PER_SECOND);
            let len = step.len();
            let dir = step.norm();
            let mut end = len;
            let mut done = false;
            let mut explode_at: Option<V3> = None;
            // the walls and floors
            let wall = map.ray(s.pos, dir, len, &self.extra, true);
            if let Some(h) = wall {
                end = h.t;
            }
            match s.from {
                ShotFrom::Raider(who) => {
                    // drones along the segment, nearest first
                    let mut hits: Vec<(i32, usize, Zone)> = Vec::new();
                    for (j, d) in self.drones.iter().enumerate() {
                        if !d.active() || d.pos.dist(s.pos) > len + int(4) {
                            continue;
                        }
                        if let Some((t, z)) = d.ray(s.pos, dir, end) {
                            hits.push((t, j, z));
                        }
                    }
                    hits.sort();
                    for (t, j, z) in hits {
                        let at = s.pos.add(dir.scale(t));
                        if s.kind != ShotKind::Bullet {
                            explode_at = Some(at);
                            end = t;
                            // the direct hit, then the blast
                            self.strike_drone(c, j, z, s.damage, dir, at, who, s.effect, s.power);
                            done = true;
                            break;
                        }
                        self.strike_drone(c, j, z, s.damage, dir, at, who, s.effect, s.power);
                        if s.splash > 0 {
                            explode_at = Some(at);
                        }
                        if s.pierce > 0 && explode_at.is_none() {
                            s.pierce -= 1;
                            continue;
                        }
                        end = t;
                        done = true;
                        break;
                    }
                    // the hive
                    if !done {
                        if let Some(at) = self.hit_hive(s.pos, dir, end, s.damage, who) {
                            done = true;
                            if s.splash > 0 || s.kind != ShotKind::Bullet {
                                explode_at = Some(at);
                            }
                        }
                    }
                }
                ShotFrom::Drone(_) => {
                    for j in 0..self.raiders.len() {
                        let ra = &self.raiders[j];
                        if !ra.life.active() || ra.pos.dist(s.pos) > len + int(3) {
                            continue;
                        }
                        let hit = drone::sphere(s.pos, dir, ra.chest(), ONE * 45 / 100, end)
                            .or_else(|| drone::sphere(s.pos, dir, ra.head(), ONE / 4, end));
                        if let Some(t) = hit {
                            end = t;
                            done = true;
                            let at = s.pos.add(dir.scale(t));
                            if s.splash > 0 {
                                explode_at = Some(at);
                            } else {
                                self.hurt_raider(j, s.damage);
                            }
                            break;
                        }
                    }
                }
            }
            let at = s.pos.add(dir.scale(end));
            if !done {
                if let Some(h) = wall {
                    if s.kind == ShotKind::Grenade && s.life > 0 {
                        // bounce: lose the speed into the surface and half the rest
                        let n = h.normal;
                        let into = s.vel.dot(n);
                        s.vel = s.vel.sub(n.scale(into * 2)).scale(ONE / 2);
                        s.pos = h.at.add(n.scale(ONE / 20));
                        self.shots[k] = s;
                        k += 1;
                        continue;
                    }
                    done = true;
                    if s.splash > 0 || s.kind != ShotKind::Bullet {
                        explode_at = Some(at);
                    } else {
                        self.events.push(Event::Spark { at });
                    }
                }
            }
            if !done && s.life <= 0 {
                done = true;
                if s.kind != ShotKind::Bullet || s.splash > 0 {
                    explode_at = Some(at);
                }
            }
            if let Some(e) = explode_at {
                let by = match s.from {
                    ShotFrom::Raider(w) => Some(w),
                    ShotFrom::Drone(_) => None,
                };
                let radius = if s.splash > 0 { s.splash } else { int(2) };
                self.explode(r, e, radius, s.damage, s.effect, s.power, by);
            }
            if done {
                self.shots.swap_remove(k);
            } else {
                s.pos = s.pos.add(step);
                self.shots[k] = s;
                k += 1;
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn strike_drone(
        &mut self,
        c: &Content,
        j: usize,
        z: Zone,
        dmg: i32,
        dir: V3,
        at: V3,
        who: u8,
        e: Effect,
        power: i32,
    ) {
        let def = &c.drones[self.drones[j].def];
        let s = self.drones[j].strike(def, z, dmg, dir, at, who);
        self.drones[j].apply_effect(e, power, who);
        self.alert(j, who);
        let plated = matches!(z, Zone::Piece(i) if matches!(drone::pieces(self.drones[j].kind)[i].kind, PieceKind::Plate | PieceKind::Wing));
        self.events.push(Event::Hit {
            at,
            weak: z == Zone::Weak,
            armour: z == Zone::Armour || plated,
            by: who,
            dealt: s.dealt,
            drone: j as u16,
        });
        if let Some((_, kind)) = s.lost {
            self.events.push(Event::PieceOff {
                at,
                kind,
                drone: j as u16,
                by: who,
            });
        }
        if s.killed {
            self.kill(j, who);
        }
        // a shock jumps to the nearest other drone
        if e == Effect::Shock {
            let from = self.drones[j].pos;
            let mut best: Option<(i32, usize)> = None;
            for (o, d) in self.drones.iter().enumerate() {
                if o != j && d.active() {
                    let dist = d.pos.dist(from);
                    if dist < int(7) && best.is_none_or(|b| dist < b.0) {
                        best = Some((dist, o));
                    }
                }
            }
            if let Some((_, o)) = best {
                let def = &c.drones[self.drones[o].def];
                let to = self.drones[o].pos;
                let s = self.drones[o].strike(def, Zone::Body, dmg / 2, dir, to, who);
                self.drones[o].apply_effect(Effect::Shock, power / 2, who);
                self.events.push(Event::Laser { from, to });
                if s.killed {
                    self.kill(o, who);
                }
            }
        }
    }

    /// A drone that is hit knows who did it.
    fn alert(&mut self, j: usize, who: u8) {
        let pos = self.raiders.get(who as usize).map(|r| r.pos);
        let b = &mut self.drones[j].brain;
        if let Some(a) = b.aware.get_mut(who as usize) {
            *a = AWARE_FULL;
        }
        if b.target.is_none() {
            b.target = Some(who);
            b.seen_t = 0;
            if let Some(p) = pos {
                b.seen_at = p;
            }
        }
    }

    fn hit_hive(&mut self, o: V3, d: V3, max_t: i32, dmg: i32, who: u8) -> Option<V3> {
        let h = self.hive.as_mut()?;
        if h.hp <= 0 {
            return None;
        }
        let mut best: Option<(i32, Option<usize>)> = None;
        for i in 0..6 {
            if h.pods[i] > 0 {
                if let Some(t) = drone::sphere(o, d, h.pod_at(i), ONE * 6 / 10, max_t) {
                    if best.is_none_or(|b| t < b.0) {
                        best = Some((t, Some(i)));
                    }
                }
            }
        }
        if let Some(t) = drone::sphere(o, d, h.core(), ONE * 17 / 10, max_t) {
            if best.is_none_or(|b| t < b.0) {
                best = Some((t, None));
            }
        }
        let (t, pod) = best?;
        let at = o.add(d.scale(t));
        match pod {
            Some(i) => {
                h.pods[i] -= dmg;
                self.events.push(Event::Hit {
                    at,
                    weak: true,
                    armour: false,
                    by: who,
                    dealt: dmg,
                    drone: u16::MAX,
                });
            }
            None => {
                let open = h.pods.iter().all(|p| *p <= 0);
                let dealt = if open { dmg } else { dmg / 10 };
                h.hp -= dealt;
                self.events.push(Event::Hit {
                    at,
                    weak: open,
                    armour: !open,
                    by: who,
                    dealt,
                    drone: u16::MAX,
                });
            }
        }
        if h.hp <= 0 {
            h.hp = 0;
            self.events.push(Event::Boom {
                at: h.core(),
                radius: int(6),
                effect: Effect::None,
            });
            self.events.push(Event::Objective);
        }
        Some(at)
    }

    #[allow(clippy::too_many_arguments)]
    fn explode(&mut self, r: &Ref, at: V3, radius: i32, dmg: i32, e: Effect, power: i32, by: Option<u8>) {
        let c = r.content;
        self.events.push(Event::Boom {
            at,
            radius,
            effect: e,
        });
        match by {
            Some(who) => {
                for j in 0..self.drones.len() {
                    let d = &self.drones[j];
                    if !d.active() {
                        continue;
                    }
                    let dist = d.centre().dist(at);
                    let reach = radius + d.radius(&c.drones[d.def]);
                    if dist > reach {
                        continue;
                    }
                    if !r.map.clear(at, d.centre(), &self.extra) {
                        continue;
                    }
                    let fall = ONE - fx::div(dist, reach.max(1)) / 2;
                    let dir = d.centre().sub(at).norm();
                    let centre = d.centre();
                    let def = &c.drones[d.def];
                    // a blast meets the queen's hull, not her insides
                    let zone = if d.kind == Kind::Queen {
                        Zone::Armour
                    } else {
                        Zone::Body
                    };
                    let s = self.drones[j].strike(def, zone, fx::mul(dmg, fall), dir, centre, who);
                    self.drones[j].apply_effect(e, power, who);
                    self.events.push(Event::Hit {
                        at: centre,
                        weak: false,
                        armour: false,
                        by: who,
                        dealt: s.dealt,
                        drone: j as u16,
                    });
                    // a blast tears at every part near it, not only the one it hit
                    for (part_at, kind) in self.drones[j].blast(at, radius, dmg) {
                        self.events.push(Event::PieceOff {
                            at: part_at,
                            kind,
                            drone: j as u16,
                            by: who,
                        });
                    }
                    if s.killed {
                        self.kill(j, who);
                    }
                }
                if let Some(h) = &mut self.hive {
                    if h.hp > 0 && h.core().dist(at) < radius + int(2) {
                        let open = h.pods.iter().all(|p| *p <= 0);
                        h.hp -= if open { dmg } else { dmg / 10 };
                        for i in 0..6 {
                            if h.pod_at(i).dist(at) < radius + ONE {
                                h.pods[i] -= dmg;
                            }
                        }
                        if h.hp <= 0 {
                            h.hp = 0;
                            self.events.push(Event::Objective);
                        }
                    }
                }
            }
            None => {
                for j in 0..self.raiders.len() {
                    let ra = &self.raiders[j];
                    if !ra.life.active() {
                        continue;
                    }
                    let dist = ra.chest().dist(at);
                    if dist < radius && r.map.clear(at, ra.chest(), &self.extra) {
                        let fall = ONE - fx::div(dist, radius.max(1)) / 2;
                        self.hurt_raider_by(j, fx::mul(dmg, fall), true);
                    }
                }
            }
        }
    }

    // ---- the order and the van ---------------------------------------------------------------

    pub fn goal_done(&self, r: &Ref) -> bool {
        let m = &r.content.missions[r.setup.mission];
        match m.goal {
            Goal::Fetch => self.carried(None) >= m.count,
            Goal::Clear => !self.drones.iter().any(|d| d.nest && d.active()),
            Goal::Hive => self.hive.as_ref().is_some_and(|h| h.hp <= 0),
            Goal::Power => self.generator == Generator::Done,
            Goal::Queen => !self.drones.iter().any(|d| d.kind == Kind::Queen && d.active()),
        }
    }

    /// How many mission things raiders hold (only those in the van, if asked).
    pub fn carried(&self, in_van: Option<bool>) -> i32 {
        let mut n = 0;
        for (i, p) in self.packs.iter().enumerate() {
            let ra = &self.raiders[i];
            let counts = match in_van {
                None => ra.life.active() || ra.life == Life::Extracted,
                Some(_) => ra.life == Life::Extracted,
            };
            if counts {
                n += p.iter().filter(|it| **it == Item::Mission).count() as i32;
            }
        }
        n
    }

    fn mission_tick(&mut self, r: &Ref) {
        let (c, map) = (r.content, r.map);
        let m = &c.missions[r.setup.mission];
        // the hive sends out wasps while it lives
        if let Some(h) = &mut self.hive {
            if h.hp > 0 {
                h.spawn_t -= 1;
                let near = self
                    .raiders
                    .iter()
                    .any(|ra| ra.life.active() && ra.pos.dist(h.pos) < int(60));
                let out = self.drones.iter().filter(|d| d.nest && d.active()).count();
                if h.spawn_t <= 0 && near && out < HIVE_SPAWN_MAX {
                    h.spawn_t = HIVE_SPAWN_EVERY;
                    let at = h.core();
                    let mut rng = Rng::new(self.rng.next());
                    let k = self.spawn(c, map, "wasp", v3(at.x, 0, at.z), &mut rng, true);
                    self.drones[k].pos = at;
                    self.drones[k].brain.seen_t = 0;
                    let near_raider = self.nearest_raider(at);
                    self.drones[k].brain.target = near_raider;
                    if let Some(t) = near_raider {
                        self.drones[k].brain.seen_at = self.raiders[t as usize].pos;
                    }
                }
            }
        }
        // the generator: hold the court while it spins up, waves coming
        if let Generator::Running(t) = self.generator {
            let t = t - 1;
            self.generator = if t <= 0 {
                Generator::Done
            } else {
                Generator::Running(t)
            };
            if t <= 0 {
                self.events.push(Event::Objective);
            }
            self.wave_t -= 1;
            if self.wave_t <= 0 && self.waves_left > 0 {
                self.waves_left -= 1;
                self.wave_t = WAVE_EVERY;
                self.wave(r);
            }
        }
        let _ = m;
    }

    fn nearest_raider(&self, at: V3) -> Option<u8> {
        let mut best: Option<(i32, u8)> = None;
        for (j, ra) in self.raiders.iter().enumerate() {
            if ra.life.active() {
                let d = ra.pos.dist(at);
                if best.is_none_or(|b| d < b.0) {
                    best = Some((d, j as u8));
                }
            }
        }
        best.map(|b| b.1)
    }

    fn wave(&mut self, r: &Ref) {
        let (c, map) = (r.content, r.map);
        let Some(g) = map.spots.iter().find(|s| s.key == "generator").map(|s| s.at) else {
            return;
        };
        let ring: Vec<usize> = (0..map.nodes.len())
            .filter(|&i| {
                let d = map.nodes[i].dist(g);
                d > int(35) && d < int(70) && map.zone_at(map.nodes[i]).is_some_and(|z| z.indoors)
            })
            .collect();
        if ring.is_empty() {
            return;
        }
        let n = 3 + self.raiders.iter().filter(|ra| ra.life.active()).count() as i32;
        let mut rng = Rng::new(self.rng.next());
        for k in 0..n {
            let at = map.nodes[ring[rng.range(0, ring.len() as i32 - 1) as usize]];
            let kind = ["wasp", "tick", "wasp", "strider", "hornet", "tick"][k as usize % 6];
            let d = self.spawn(c, map, kind, at, &mut rng, false);
            let t = self.nearest_raider(g);
            self.drones[d].brain.target = t;
            self.drones[d].brain.seen_at = g;
            self.drones[d].brain.seen_t = 0;
        }
        self.events.push(Event::Wave);
    }

    fn van_tick(&mut self, r: &Ref) {
        self.van.door_t += 1;
        if !self.van.doors_open && self.van.door_t >= VAN_DOORS_SELF_OPEN {
            self.open_doors();
        }
        if let Some(t) = self.van.leaving {
            let t = t - 1;
            self.van.leaving = Some(t);
            if t <= 0 {
                let inside = self.van.inside();
                for ra in self.raiders.iter_mut() {
                    if ra.life.active() {
                        ra.life = if inside.contains(ra.chest()) {
                            Life::Extracted
                        } else {
                            Life::Left
                        };
                    }
                }
                self.events.push(Event::Extracted);
                let m = &r.content.missions[r.setup.mission];
                let ok = self.goal_done(r) && (m.goal != Goal::Fetch || self.carried(Some(true)) >= m.count);
                self.over = Some(ok && self.raiders.iter().any(|ra| ra.life == Life::Extracted));
            }
        }
        // everyone down or gone: the run is lost
        if self.over.is_none() && !self.raiders.iter().any(|ra| ra.life == Life::Alive) {
            for ra in self.raiders.iter_mut() {
                if ra.life == Life::Downed {
                    ra.life = Life::Dead;
                }
            }
            self.over = Some(false);
        }
    }

    /// A checksum of the run, for online copies to compare.
    pub fn checksum(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut mix = |v: i64| {
            h ^= v as u64;
            h = h.wrapping_mul(0x0100_0000_01b3);
        };
        mix(self.tick as i64);
        for ra in &self.raiders {
            mix(ra.pos.x as i64);
            mix(ra.pos.y as i64);
            mix(ra.pos.z as i64);
            mix(ra.hp as i64);
            mix(ra.ammo as i64);
            mix(ra.life as i64);
        }
        for d in &self.drones {
            mix(d.pos.x as i64);
            mix(d.pos.y as i64);
            mix(d.pos.z as i64);
            mix(d.hp as i64);
        }
        mix(self.shots.len() as i64);
        mix(self.pickups.len() as i64);
        let mut r = self.rng.clone();
        mix(r.next() as i64);
        h
    }

    /// Which part slots a raider loses if they do not come back: every part
    /// that is not basic.
    pub fn staked(c: &Content, b: &Build) -> Vec<usize> {
        Cat::ALL
            .iter()
            .map(|cat| b[cat.index()])
            .filter(|&p| !c.parts[p].basic)
            .collect()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::content::tests::content;
    use crate::gun::{basic_build, compose};
    use crate::mall::tests::map;
    use crate::raider::{B_FWD, B_RIGHT};

    pub fn setup(c: &Content, mission: &str, players: usize) -> Setup {
        Setup::basic(
            c,
            c.missions.iter().position(|m| m.id == mission).unwrap(),
            players,
            42,
        )
    }

    #[test]
    fn a_run_starts_in_the_van_with_the_doors_shut_and_drones_in_the_mall() {
        let c = content();
        let m = map();
        let s = setup(&c, "cudi", 4);
        let r = Ref {
            content: &c,
            map: &m,
            setup: &s,
        };
        let run = Run::new(&r);
        assert_eq!(run.raiders.len(), 4);
        for ra in &run.raiders {
            assert!(
                run.van.inside().contains(ra.chest()),
                "everyone starts in the van"
            );
            assert!(!m.blocked(&ra.body(), &run.extra), "and not stuck in it");
        }
        let indoors = run
            .drones
            .iter()
            .filter(|d| m.zone_at(d.pos).is_some_and(|z| z.indoors))
            .count();
        assert!(
            indoors > run.drones.len() / 2,
            "{indoors} of {}",
            run.drones.len()
        );
        assert!(run.drones.iter().any(|d| d.kind.walks()));
        assert_eq!(
            run.pickups.iter().filter(|p| p.item == Item::Mission).count(),
            1,
            "the shirt"
        );
    }

    #[test]
    fn opening_the_doors_and_walking_out_works() {
        let c = content();
        let m = map();
        let s = setup(&c, "drives", 1);
        let r = Ref {
            content: &c,
            map: &m,
            setup: &s,
        };
        let mut run = Run::new(&r);
        let yaw = run.van.yaw;
        let walk = |run: &mut Run, b: u32, n: i32| {
            for _ in 0..n {
                run.step(
                    &r,
                    &[Input {
                        b,
                        y: yaw,
                        p: 0,
                        d: 0,
                    }],
                );
            }
        };
        walk(&mut run, B_FWD, 60);
        assert!(
            run.van.inside().contains(run.raiders[0].chest()),
            "the shut doors keep them in"
        );
        walk(&mut run, B_USE, 20);
        assert!(run.van.doors_open);
        walk(&mut run, B_FWD, 180);
        assert!(
            !run.van.inside().contains(run.raiders[0].chest()),
            "out of the van"
        );
        assert!(
            run.raiders[0].pos.y < ONE / 10,
            "down on the asphalt: {}",
            run.raiders[0].pos.y
        );
    }

    #[test]
    fn the_same_inputs_give_the_same_run() {
        let c = content();
        let m = map();
        let s = setup(&c, "power", 2);
        let r = Ref {
            content: &c,
            map: &m,
            setup: &s,
        };
        let mut a = Run::new(&r);
        let mut b = Run::new(&r);
        for t in 0..400 {
            let i = [
                Input {
                    b: if t % 50 < 25 {
                        B_FWD | B_USE
                    } else {
                        B_RIGHT | B_FIRE
                    },
                    y: t * 40,
                    p: 0,
                    d: 0,
                },
                Input::default(),
            ];
            a.step(&r, &i);
            b.step(&r, &i);
        }
        assert_eq!(a.checksum(), b.checksum());
        assert_eq!(a, b);
    }

    #[test]
    fn a_volley_from_four_double_barrels_kills_a_wasp_at_close_range() {
        let c = content();
        let m = map();
        let mut s = setup(&c, "cudi", 1);
        let mut b = basic_build(&c);
        b[0] = c.part("pistol").unwrap();
        b[1] = c.part("double").unwrap();
        b[2] = c.part("square").unwrap();
        s.builds[0] = b;
        s.guns[0] = compose(&c, &b);
        let r = Ref {
            content: &c,
            map: &m,
            setup: &s,
        };
        let mut run = Run::new(&r);
        run.drones.clear();
        run.open_doors();
        // stand in the concourse, a wasp four metres ahead at chest height
        run.raiders[0].pos = v3(0, 0, int(40));
        let wasp = c.drone("wasp");
        run.drones.push(Drone::new(
            Kind::Wasp,
            wasp,
            &c.drones[wasp],
            v3(int(1), ONE * 17 / 10, int(36)),
            0,
            V3::ZERO,
        ));
        let (yaw, pitch) = yaw_pitch_of(run.drones[0].pos.sub(v3(int(1), ONE * 17 / 10, int(40))));
        run.step(
            &r,
            &[Input {
                b: 0,
                y: yaw,
                p: pitch,
                d: 0,
            }],
        );
        let fired_before = run.raiders[0].ammo;
        for _ in 0..20 {
            run.step(
                &r,
                &[Input {
                    b: B_FIRE,
                    y: yaw,
                    p: pitch,
                    d: 0,
                }],
            );
            run.raiders[0].prev = 0;
        }
        assert!(run.raiders[0].ammo < fired_before);
        assert!(!run.drones[0].active(), "hp left {}", run.drones[0].hp);
    }

    #[test]
    fn a_downed_raider_is_revived_by_a_teammate_holding_e() {
        let c = content();
        let m = map();
        let s = setup(&c, "cudi", 2);
        let r = Ref {
            content: &c,
            map: &m,
            setup: &s,
        };
        let mut run = Run::new(&r);
        run.drones.clear();
        run.open_doors();
        run.raiders[0].pos = v3(0, 0, int(40));
        run.raiders[1].pos = v3(ONE, 0, int(40));
        run.hurt_raider(1, 200);
        assert_eq!(run.raiders[1].life, Life::Downed);
        for _ in 0..(REVIVE + 2) {
            run.step(
                &r,
                &[
                    Input {
                        b: B_USE,
                        y: 0,
                        p: 0,
                        d: 0,
                    },
                    Input::default(),
                ],
            );
        }
        assert_eq!(run.raiders[1].life, Life::Alive);
    }

    #[test]
    fn leaving_in_the_van_extracts_those_inside_and_leaves_the_rest() {
        let c = content();
        let m = map();
        let s = setup(&c, "cudi", 2);
        let r = Ref {
            content: &c,
            map: &m,
            setup: &s,
        };
        let mut run = Run::new(&r);
        run.drones.clear();
        run.open_doors();
        run.raiders[1].pos = v3(0, 0, int(40));
        run.tick = secs(6000) as u32;
        for _ in 0..(secs(500) + VAN_LEAVE + 4) {
            run.step(
                &r,
                &[
                    Input {
                        b: B_USE,
                        y: run.van.yaw,
                        p: 0,
                        d: 0,
                    },
                    Input::default(),
                ],
            );
        }
        assert_eq!(run.raiders[0].life, Life::Extracted);
        assert_eq!(run.raiders[1].life, Life::Left);
        assert_eq!(run.over, Some(false), "no shirt: the order is not done");
    }

    /// A run with one raider at (x, z) in the concourse, no drones, doors open.
    fn open_run<'a>(r: &Ref<'a>, at: V3) -> Run {
        let mut run = Run::new(r);
        run.drones.clear();
        run.open_doors();
        run.raiders[0].pos = at;
        run
    }

    fn place(run: &mut Run, c: &Content, kind: &str, at: V3, yaw: i32) -> usize {
        let def = c.drone(kind);
        let mut d = Drone::new(Kind::from_id(kind), def, &c.drones[def], at, yaw, at);
        d.brain.patrol = vec![at];
        run.drones.push(d);
        run.drones.len() - 1
    }

    fn input(b: u32) -> Input {
        Input { b, y: 0, p: 0, d: 0 }
    }

    #[test]
    fn reloading_takes_rounds_from_the_pack_and_an_empty_pack_clicks() {
        let c = content();
        let m = map();
        let mut s = setup(&c, "cudi", 1);
        s.ammo_in = vec![14];
        let r = Ref {
            content: &c,
            map: &m,
            setup: &s,
        };
        let mut run = open_run(&r, v3(0, 0, int(40)));
        assert_eq!(run.rounds(0, 0), 14);
        run.raiders[0].ammo = 0;
        run.step(&r, &[input(B_RELOAD)]);
        for _ in 0..200 {
            run.step(&r, &[input(0)]);
        }
        assert_eq!(run.raiders[0].ammo, 12, "a full magazine");
        assert_eq!(run.rounds(0, 0), 2, "twelve rounds came out of the pack");
        run.raiders[0].ammo = 0;
        run.packs[0].clear();
        run.step(&r, &[input(B_FIRE)]);
        assert!(run.events.iter().any(|e| matches!(e, Event::NoAmmo { .. })));
        assert_eq!(run.raiders[0].reloading, 0, "nothing to load");
    }

    /// Ticks until a wasp facing a raider 15 m off is fully aware of them.
    fn ticks_to_notice(bits: u32, notice: &str) -> u32 {
        let c = content();
        let m = map();
        let mut s = setup(&c, "cudi", 1);
        if !notice.is_empty() {
            let mut w = crate::kit::basic_wear(&c);
            let g = c.gear_id(notice).unwrap();
            w[c.gear[g].slot_index()] = g;
            s.wears[0] = w;
            s.kits[0] = crate::kit::kit(&c, &w);
        }
        let r = Ref {
            content: &c,
            map: &m,
            setup: &s,
        };
        let mut run = open_run(&r, v3(0, 0, int(40)));
        // the wasp hovers 15 m up the concourse, facing the raider (+z)
        place(&mut run, &c, "wasp", v3(0, int(3), int(25)), fx::HALF);
        for t in 0..1200u32 {
            // held where they are, keys down (so moving when walking); the
            // wasp held facing them
            run.step(&r, &[input(bits)]);
            run.raiders[0].pos = v3(0, run.raiders[0].pos.y, int(40));
            run.drones[0].yaw = fx::HALF;
            run.drones[0].pos = v3(0, int(3), int(25));
            if run.drones[0].brain.aware[0] >= AWARE_FULL {
                return t;
            }
        }
        1200
    }

    #[test]
    fn standing_still_or_crouching_takes_a_drone_longer_to_notice() {
        let walking = ticks_to_notice(B_FWD, "");
        let still = ticks_to_notice(0, "");
        let crouched = ticks_to_notice(raider::B_CROUCH, "");
        let poncho = ticks_to_notice(raider::B_CROUCH, "poncho");
        assert!(walking < still, "{walking} {still}");
        assert!(still < crouched, "{still} {crouched}");
        assert!(crouched < poncho, "{crouched} {poncho}");
        assert!(walking > 10, "not at once: {walking}");
    }

    #[test]
    fn running_behind_a_drone_is_heard_and_creeping_is_not() {
        let c = content();
        let m = map();
        let s = setup(&c, "cudi", 1);
        let r = Ref {
            content: &c,
            map: &m,
            setup: &s,
        };
        for (bits, heard) in [
            (B_FWD | raider::B_SPRINT, true),
            (B_FWD | raider::B_CROUCH, false),
        ] {
            let mut run = open_run(&r, v3(0, 0, int(40)));
            // facing away (−z), the raider 8 m behind it
            place(&mut run, &c, "wasp", v3(0, int(3), int(32)), 0);
            for _ in 0..120 {
                run.step(
                    &r,
                    &[Input {
                        b: bits,
                        y: fx::HALF,
                        p: 0,
                        d: 0,
                    }],
                );
                run.raiders[0].pos = v3(0, 0, int(40));
            }
            let d = &run.drones[0].brain;
            assert_eq!(
                d.aware[0] > AWARE_SUSPECT || d.target.is_some(),
                heard,
                "{bits}: {}",
                d.aware[0]
            );
        }
    }

    #[test]
    fn a_decoy_draws_a_drone_to_look_and_smoke_hides_a_raider() {
        let c = content();
        let m = map();
        let mut s = setup(&c, "cudi", 1);
        for g in ["decoys", "smoke"] {
            let mut w = crate::kit::basic_wear(&c);
            w[crate::content::SLOT_BELT] = c.gear_id(g).unwrap();
            s.wears[0] = w;
            s.kits[0] = crate::kit::kit(&c, &w);
            let r = Ref {
                content: &c,
                map: &m,
                setup: &s,
            };
            let mut run = open_run(&r, v3(0, 0, int(40)));
            // facing away from the raider, toward where the gadget lands
            let k = place(&mut run, &c, "wasp", v3(0, int(3), int(10)), 0);
            run.raiders[0].pitch = 0;
            run.step(&r, &[input(B_GADGET)]);
            assert_eq!(run.gadgets.len(), 1);
            // the decoy draws it to look (it may then turn and see the raider)
            let mut drawn = false;
            for _ in 0..240 {
                run.step(&r, &[input(0)]);
                if let Some(gd) = run.gadgets.first() {
                    drawn |= gd.landed
                        && run.drones[k]
                            .brain
                            .suspect
                            .is_some_and(|p| p.dist(gd.pos) < int(2));
                }
            }
            let g0 = run.gadgets.first().map(|g| (g.kind, g.landed, g.pos));
            if g == "decoys" {
                assert!(drawn, "the wasp went to look at the decoy");
            } else {
                let (_, _, at) = g0.unwrap();
                let eye = v3(0, int(3), int(10));
                let behind = at.add(v3(0, int(1), int(4)));
                assert!(smoke_between(v3(at.x, at.y + int(2), at.z), eye, behind));
            }
        }
    }

    #[test]
    fn a_bat_knocks_a_tick_about_and_tears_a_leg_off() {
        let c = content();
        let m = map();
        let s = setup(&c, "cudi", 1);
        let r = Ref {
            content: &c,
            map: &m,
            setup: &s,
        };
        let mut run = open_run(&r, v3(0, 0, int(40)));
        let k = place(&mut run, &c, "tick", v3(0, 0, int(39)), 0);
        let hp = run.drones[k].hp;
        let legs = run.drones[k].alive_count(crate::drone::PieceKind::Leg);
        for t in 0..400 {
            let b = if t % 60 == 0 { B_MELEE } else { 0 };
            run.step(
                &r,
                &[Input {
                    b,
                    y: 0,
                    p: -deg(40),
                    d: 0,
                }],
            );
            run.raiders[0].pos = v3(0, 0, int(40));
            run.raiders[0].hp = 100;
        }
        let d = &run.drones[k];
        assert!(
            !d.active() || d.hp < hp || d.alive_count(crate::drone::PieceKind::Leg) < legs,
            "the bat landed"
        );
    }

    #[test]
    fn a_mine_goes_off_under_a_drone() {
        let c = content();
        let m = map();
        let mut s = setup(&c, "cudi", 1);
        let mut w = crate::kit::basic_wear(&c);
        w[crate::content::SLOT_BELT] = c.gear_id("mines").unwrap();
        s.wears[0] = w;
        s.kits[0] = crate::kit::kit(&c, &w);
        let r = Ref {
            content: &c,
            map: &m,
            setup: &s,
        };
        let mut run = open_run(&r, v3(0, 0, int(40)));
        run.step(&r, &[input(B_GADGET)]);
        assert_eq!(run.raiders[0].charges, 2);
        run.raiders[0].pos = v3(int(10), 0, int(60));
        for _ in 0..90 {
            run.step(&r, &[input(0)]);
        }
        let k = place(&mut run, &c, "tick", v3(ONE, 0, int(40)), 0);
        run.drones[k].brain.patrol = vec![v3(ONE, 0, int(40))];
        for _ in 0..30 {
            run.step(&r, &[input(0)]);
        }
        assert!(run.gadgets.is_empty(), "the mine went off");
        assert!(run.drones[k].hp < c.drones[run.drones[k].def].hp);
    }

    #[test]
    fn dropping_a_pack_item_leaves_it_on_the_floor() {
        let c = content();
        let m = map();
        let s = setup(&c, "cudi", 1);
        let r = Ref {
            content: &c,
            map: &m,
            setup: &s,
        };
        let mut run = open_run(&r, v3(0, 0, int(40)));
        let before = run.packs[0].len();
        let picks = run.pickups.len();
        run.step(
            &r,
            &[Input {
                b: 0,
                y: 0,
                p: 0,
                d: 1,
            }],
        );
        assert_eq!(run.packs[0].len(), before - 1);
        assert_eq!(run.pickups.len(), picks + 1);
    }

    #[test]
    fn gear_takes_damage_off_and_adds_health() {
        let c = content();
        let m = map();
        let mut s = setup(&c, "cudi", 1);
        let mut w = crate::kit::basic_wear(&c);
        w[2] = c.gear_id("vest").unwrap();
        s.wears[0] = w;
        s.kits[0] = crate::kit::kit(&c, &w);
        let r = Ref {
            content: &c,
            map: &m,
            setup: &s,
        };
        let mut run = open_run(&r, v3(0, 0, int(40)));
        assert_eq!(run.raiders[0].hp, RAIDER_HP + 35);
        run.hurt_raider(0, 20);
        assert_eq!(run.raiders[0].hp, RAIDER_HP + 35 - 18, "the vest takes a tenth");
    }

    #[test]
    fn a_warden_flies_under_the_balconies_without_its_rotors_in_them() {
        let c = content();
        let m = map();
        let s = setup(&c, "cudi", 1);
        let r = Ref {
            content: &c,
            map: &m,
            setup: &s,
        };
        let mut run = open_run(&r, v3(int(60), 0, int(200)));
        let k = place(&mut run, &c, "warden", v3(int(6), int(3), int(40)), 0);
        run.drones[k].brain.patrol = vec![
            v3(int(6), 0, int(30)),
            v3(int(6), 0, int(100)),
            v3(-int(6), 0, int(70)),
        ];
        let def = c.drones[run.drones[k].def].clone();
        for _ in 0..1800 {
            run.step(&r, &[input(0)]);
            let d = &run.drones[k];
            // its whole span, rotors and all, never sinks into a solid
            let span = Aabb::standing(
                v3(d.pos.x, d.pos.y - ONE / 4, d.pos.z),
                ONE * 165 / 100,
                ONE * 9 / 10,
            );
            assert!(
                !m.blocked(&span, &[]) || d.pos.y < int(1),
                "wedged at {:?}",
                d.pos
            );
            let _ = &def;
        }
        let travelled = run.drones[k].pos.dist(v3(int(6), int(3), int(40)));
        assert!(travelled > int(10), "it got about: {travelled}");
    }

    #[test]
    fn every_order_fields_wardens_and_striders_on_the_main_ways() {
        let c = content();
        let m = map();
        for mission in 0..c.missions.len() {
            for seed in [1u64, 2, 3, 40] {
                let s = Setup::basic(&c, mission, 1, seed);
                let r = Ref {
                    content: &c,
                    map: &m,
                    setup: &s,
                };
                let run = Run::new(&r);
                let on_main = |k: Kind| {
                    run.drones
                        .iter()
                        .filter(|d| d.kind == k)
                        .filter(|d| {
                            m.zone_at(d.brain.home)
                                .is_some_and(|z| z.key == "concourse" || z.key == "centercourt")
                        })
                        .count()
                };
                assert!(
                    on_main(Kind::Warden) >= 1,
                    "mission {mission} seed {seed}: no warden"
                );
                assert!(
                    on_main(Kind::Strider) >= 2,
                    "mission {mission} seed {seed}: striders"
                );
            }
        }
    }

    fn queen_run(players: usize) -> (Content, Map, Setup) {
        let c = content();
        let m = map();
        let s = setup(&c, "queen", players);
        (c, m, s)
    }

    fn the_queen(run: &Run) -> usize {
        run.drones.iter().position(|d| d.kind == Kind::Queen).unwrap()
    }

    #[test]
    fn the_queen_walks_main_street_stronger_for_more_raiders_with_rounds_behind_the_van() {
        let (c, m, s) = queen_run(1);
        let r = Ref {
            content: &c,
            map: &m,
            setup: &s,
        };
        let run = Run::new(&r);
        assert_eq!(run.drones.iter().filter(|d| d.kind == Kind::Queen).count(), 1);
        let q = &run.drones[the_queen(&run)];
        assert_eq!(m.zone_at(q.pos).map(|z| z.key.as_str()), Some("mainstreet"));
        let solo = q.max_hp;
        let s4 = setup(&c, "queen", 4);
        let r4 = Ref {
            content: &c,
            map: &m,
            setup: &s4,
        };
        let run4 = Run::new(&r4);
        assert!(run4.drones[the_queen(&run4)].max_hp > solo * 2);
        // the client's rounds lie behind the van, for the raider's own gun
        let near_van: Vec<&Pickup> = run
            .pickups
            .iter()
            .filter(|p| p.pos.dist(run.van.pos) < int(8))
            .collect();
        assert!(!near_van.is_empty());
        assert!(near_van
            .iter()
            .all(|p| matches!(p.item, Item::Ammo(a, _) if a as usize == s.guns[0].ammo)));
        assert!(!run.goal_done(&r));
    }

    #[test]
    fn the_queen_keeps_to_the_street_while_she_chases() {
        let (c, m, s) = queen_run(1);
        let r = Ref {
            content: &c,
            map: &m,
            setup: &s,
        };
        let mut run = Run::new(&r);
        run.open_doors();
        let k = the_queen(&run);
        let start = run.drones[k].pos;
        let mut furthest = 0;
        // a raider dodging about the shops on either side of the street
        for t in 0..60 * 60 {
            let side = if (t / 600) % 2 == 0 { int(50) } else { int(92) };
            run.raiders[0].pos = v3(side, 0, int(110) + (t % 600) * ONE / 10);
            run.raiders[0].hp = 100;
            // she has seen them go in, and comes after them
            let q = &mut run.drones[k];
            q.brain.target = Some(0);
            q.brain.aware[0] = AWARE_FULL;
            q.brain.seen_at = run.raiders[0].pos;
            q.brain.seen_t = 0;
            run.step(&r, &[Input::default()]);
            let p = run.drones[k].pos;
            assert_eq!(crate::mall::queen_clamp(p), p, "she left the street at {p:?}");
            furthest = furthest.max(p.dist(start));
        }
        assert!(furthest > int(5), "she walked: {}", furthest / ONE);
    }

    #[test]
    fn the_queen_stamps_fires_rocket_salvos_and_launches_wasps() {
        let (c, m, s) = queen_run(1);
        let r = Ref {
            content: &c,
            map: &m,
            setup: &s,
        };
        let mut run = Run::new(&r);
        run.open_doors();
        // her alone
        run.drones.retain(|d| d.kind == Kind::Queen);
        let k = the_queen(&run);
        let q = run.drones[k].pos;
        let wasps = |run: &Run| {
            run.drones
                .iter()
                .filter(|d| d.kind == Kind::Wasp && d.active())
                .count()
        };
        let (mut rockets, mut stamped) = (false, false);
        for t in 0..60 * 25 {
            // in the open, in front of her, then under her
            let at = if t < 60 * 15 { v3(q.x, 0, q.z - int(30)) } else { q };
            run.raiders[0].pos = at;
            run.raiders[0].hp = 100;
            run.raiders[0].life = Life::Alive;
            run.drones[k].pos = q;
            run.step(&r, &[Input::default()]);
            rockets |= run
                .shots
                .iter()
                .any(|s| s.kind == ShotKind::Rocket && matches!(s.from, ShotFrom::Drone(_)));
            stamped |= run.events.iter().any(|e| matches!(e, Event::Stomp { .. })) && t >= 60 * 15;
        }
        assert!(rockets, "a rocket salvo");
        assert!(wasps(&run) >= 1, "wasps from her back");
        assert!(stamped, "a stamp on whoever is under her");
    }

    #[test]
    fn her_knees_pods_cannon_and_belly_plate_come_off() {
        let (c, m, s) = queen_run(1);
        let r = Ref {
            content: &c,
            map: &m,
            setup: &s,
        };
        let mut run = Run::new(&r);
        let k = the_queen(&run);
        let def = c.drones[run.drones[k].def].clone();
        let q = &mut run.drones[k];
        q.yaw = 0;
        // from under her, the plate covers the core
        let below = v3(q.pos.x + int(2), ONE, q.pos.z);
        let up = q.to_world(0, 610, 0).sub(below).norm();
        assert_eq!(q.ray(below, up, int(20)).map(|h| h.1), Some(Zone::Piece(8)));
        for _ in 0..40 {
            q.strike(&def, Zone::Piece(8), 60, up, below, 0);
        }
        assert_eq!(
            q.ray(below, up, int(20)).map(|h| h.1),
            Some(Zone::Weak),
            "the core is bare"
        );
        // the vent on her back, from behind and above (a roof)
        let roof = q.to_world(0, 1400, 2000);
        let to_vent = q.to_world(0, 1090, 420).sub(roof).norm();
        assert_eq!(q.ray(roof, to_vent, int(30)).map(|h| h.1), Some(Zone::Weak));
        // four knees gone: she cannot walk
        for leg in [0, 1, 3, 4] {
            for _ in 0..10 {
                q.strike(&def, Zone::Piece(leg), 80, up, below, 0);
            }
        }
        assert!(q.lame() && q.active());
        for pod in crate::drone::QUEEN_PODS {
            for _ in 0..10 {
                q.strike(&def, Zone::Piece(pod), 80, up, below, 0);
            }
            assert!(q.pieces[pod] <= 0, "pod {pod} off");
        }
    }

    #[test]
    fn bringing_down_the_queen_fills_the_order_and_drops_her_hoard() {
        let (c, m, s) = queen_run(1);
        let r = Ref {
            content: &c,
            map: &m,
            setup: &s,
        };
        let mut run = Run::new(&r);
        let k = the_queen(&run);
        let def = c.drones[run.drones[k].def].clone();
        let before = run.pickups.len();
        let at = run.drones[k].centre();
        while run.drones[k].active() {
            run.drones[k].strike(&def, Zone::Weak, 100, v3(0, ONE, 0), at, 0);
        }
        for _ in 0..60 * 15 {
            run.step(&r, &[Input::default()]);
        }
        assert!(run.goal_done(&r));
        assert!(run.pickups.len() >= before + QUEEN_HOARD as usize);
    }
}
