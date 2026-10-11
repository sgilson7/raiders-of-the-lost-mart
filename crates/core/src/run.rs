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
use crate::drone::{self, Drone, Kind, State, Zone};
use crate::fx::{self, deg, int, ONE};
use crate::geom::{facing, right_of, v3, yaw_pitch_of, V3};
use crate::gun::{Build, Gun};
use crate::raider::{self, gun_frame, Input, Life, Raider, B_FIRE, B_GONE, B_RELOAD, B_USE};
use crate::rng::Rng;
use crate::world::{Aabb, Map, SpotKind};

/// What a raider carries out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    Part(u16),
    /// the mission's thing: the shirt, a drive
    Mission,
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
    Hit {
        at: V3,
        weak: bool,
        armour: bool,
        by: u8,
    },
    RotorOff {
        at: V3,
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
}

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
    pub noise: Vec<(V3, u8)>,
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
        };
        run.extra = run.van.boxes();
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
        // most inside
        let count = m.drones * (2 + players) / 6;
        let weights = [
            ("wasp", 34),
            ("moth", 9),
            ("hornet", 15),
            ("warden", 8),
            ("tick", 19),
            ("strider", 15),
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
        let me = &mut self.raiders[i];
        if inp.b & B_GONE != 0 {
            if me.life.active() {
                me.life = Life::Gone;
                let pos = me.pos;
                let pack = std::mem::take(&mut self.packs[i]);
                self.drop_items(pos, pack);
            }
            return;
        }
        if !me.life.active() {
            return;
        }
        me.hurt_flash = (me.hurt_flash - 1).max(0);
        me.firing = (me.firing - 1).max(0);
        me.step_move(inp, r.map, &self.extra, gun.move_pct);
        if me.pos.y < -int(5) {
            me.pos.y = 0;
        }
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
        // health creeps back after a while unhurt
        me.since_hurt += 1;
        if me.since_hurt > REGEN_AFTER && me.hp < REGEN_TO {
            me.regen_t += 1;
            if me.regen_t >= REGEN_EVERY {
                me.regen_t = 0;
                me.hp += 1;
            }
        }
        // the gun
        me.cooldown = (me.cooldown - 1).max(0);
        if me.reloading > 0 {
            me.reloading -= 1;
            if me.reloading == 0 {
                me.ammo = gun.mag;
            }
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
        if me.reloading == 0 && (wants_reload || (trigger && me.ammo == 0)) {
            me.reloading = gun.reload;
            self.events.push(Event::Reload { who: i as u8 });
        }
        let spun = !gun.gatling || me.spin >= RELOAD_GATLING_SPIN;
        let can_fire =
            trigger && me.reloading == 0 && me.cooldown == 0 && me.ammo > 0 && spun && !self.in_van(i);
        let use_held = held(B_USE);
        self.raiders[i].prev = inp.b;
        if can_fire {
            self.fire(r, i);
        }
        self.use_tick(r, i, use_held);
    }

    fn in_van(&self, i: usize) -> bool {
        !self.van.doors_open && self.van.inside().contains(self.raiders[i].chest())
    }

    /// Where a raider's crosshair lands: the camera's ray, through drones too.
    pub fn aim_point(&self, r: &Ref, i: usize) -> V3 {
        let me = &self.raiders[i];
        let (eye, fwd) = raider::camera(me.pos, me.yaw, me.pitch, me.aiming, false, r.map, &self.extra);
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
        self.noise.push((me.pos, i as u8));
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
        if me.use_t < u.needs() {
            return;
        }
        me.use_t = 0;
        me.use_on = -1;
        match u {
            Use::Revive(j) => {
                let o = &mut self.raiders[j as usize];
                o.life = Life::Alive;
                o.hp = REVIVE_HP;
                o.since_hurt = 0;
                self.events.push(Event::Revived { who: j });
            }
            Use::Pick(k) => {
                let pk = self.pickups[k as usize];
                let room = self.packs[i]
                    .iter()
                    .filter(|it| matches!(it, Item::Part(_)))
                    .count()
                    < BACKPACK;
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
                    let item = self.roll_part(r.content, 0);
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
            .map(|r| Seen {
                pos: r.pos,
                chest: r.chest(),
                head: r.head(),
                yaw: r.yaw,
                pitch: r.pitch,
                vel: r.vel,
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
            // hearing: a shot wakes drones that have no target
            for &(at, who) in &noise {
                if d.brain.target.is_none() && at.dist(d.pos) < NOISE {
                    d.brain.target = Some(who);
                    d.brain.seen_at = at;
                    d.brain.seen_t = FORGET / 2;
                }
            }
            // sight
            if near && (tick + k as u32).is_multiple_of(LOOK_EVERY) {
                let eye = d.eye();
                let mut best: Option<(i32, usize)> = None;
                for (j, s) in seen.iter().enumerate() {
                    if !s.active {
                        continue;
                    }
                    let dist = s.chest.dist(eye);
                    let close = dist < int(4);
                    if (dist < int(def.sight) || close)
                        && (close
                            || map.clear(eye, s.chest, &self.extra)
                            || map.clear(eye, s.head, &self.extra))
                        && best.is_none_or(|b| dist < b.0)
                    {
                        best = Some((dist, j));
                    }
                }
                match best {
                    Some((_, j)) => {
                        if d.brain.target.is_none() || d.brain.seen_t > secs(3000) {
                            self.events.push(Event::Spotted { at: d.pos });
                        }
                        d.brain.target = Some(j as u8);
                        d.brain.seen_at = seen[j].pos;
                        d.brain.seen_t = 0;
                        d.brain.sees = true;
                        d.brain.ambush = None;
                    }
                    None => d.brain.sees = false,
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
            let top = def.speed * ONE / 10 * d.speed_pct() / 100;
            let want = ai::steer(d, map, &self.extra, top, outdoors);
            let face = d.brain.face;
            d.step_body(def, want, face, map, &self.extra);
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
                    _ => d.to_world(0, -15, -40),
                };
                let dir = to.sub(from).norm();
                let (yaw, pitch) = yaw_pitch_of(dir);
                let jy = self.rng.range(-DRONE_SHOT_SPREAD, DRONE_SHOT_SPREAD);
                let jp = self.rng.range(-DRONE_SHOT_SPREAD, DRONE_SHOT_SPREAD);
                let heavy = d.kind == Kind::Warden;
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
            Order::Stomp => {
                let at = self.drones[k].pos;
                for j in 0..self.raiders.len() {
                    let ra = &mut self.raiders[j];
                    if ra.life.active() && ra.pos.dist(at) < STOMP_RANGE {
                        let away = ra.pos.sub(at).norm();
                        ra.vel = ra.vel.add(v3(away.x, ONE / 2, away.z).scale(int(6)));
                        self.hurt_raider(j, STOMP_DAMAGE);
                    }
                }
                self.events.push(Event::Stomp { at });
            }
        }
    }

    fn drop_loot(&mut self, r: &Ref, k: usize) {
        let d = &self.drones[k];
        let at = d.pos;
        let luck = r.content.drones[d.def].loot;
        if self.rng.chance(DRONE_DROP_PCT + luck * 10) {
            let item = self.roll_part(r.content, luck * 2);
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
        let ra = &mut self.raiders[j];
        if !ra.life.active() {
            return;
        }
        if ra.hurt(dmg) {
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
        self.events.push(Event::Hit {
            at,
            weak: z == Zone::Weak,
            armour: z == Zone::Armour,
            by: who,
        });
        if s.rotor_lost {
            self.events.push(Event::RotorOff { at });
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
                });
            }
            None => {
                let open = h.pods.iter().all(|p| *p <= 0);
                h.hp -= if open { dmg } else { dmg / 10 };
                self.events.push(Event::Hit {
                    at,
                    weak: open,
                    armour: !open,
                    by: who,
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
                    let s = self.drones[j].strike(def, Zone::Body, fx::mul(dmg, fall), dir, centre, who);
                    self.drones[j].apply_effect(e, power, who);
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
                        self.hurt_raider(j, fx::mul(dmg, fall));
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
        let b = basic_build(c);
        Setup {
            mission: c.missions.iter().position(|m| m.id == mission).unwrap(),
            builds: vec![b; players],
            guns: vec![compose(c, &b); players],
            seed: 42,
        }
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
                run.step(&r, &[Input { b, y: yaw, p: 0 }]);
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
            run.step(&r, &[Input { b: B_USE, y: 0, p: 0 }, Input::default()]);
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
                    },
                    Input::default(),
                ],
            );
        }
        assert_eq!(run.raiders[0].life, Life::Extracted);
        assert_eq!(run.raiders[1].life, Life::Left);
        assert_eq!(run.over, Some(false), "no shirt: the order is not done");
    }
}
