//! The game as the page sees it: which screen is up, the stash and the
//! workshop, the client board, the lobby, a run (alone, or online with
//! rollback), the camera, the HUD and the screens as JSON, and the render
//! contract as plain numbers. The page draws what comes out of here and
//! decides nothing.

use crate::balance::*;
use crate::content::{Cat, Content, Goal, Rgb};
use crate::draw::{self, DroneColours, Particle};
use crate::fx::{self, deg, int, ONE};
use crate::geom::{facing, right_of, v3, yaw_pitch_of, Affine, V3};
use crate::gun::{self, Build, Gun};
use crate::mall;
use crate::mesh::{self, Mesh};
use crate::raider::{self, Input, Life, B_AIM, B_FIRE, B_FWD};
use crate::render::{self, Batch, Camera};
use crate::rng::Rng;
use crate::rollback::{self, Msg, Seat, Session};
use crate::run::{Event, Generator, Item, Ref, Run, Setup, Use};
use crate::world::Map;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// Each page action's bit. data/controls.json names the action for each
/// key; the movement and gun bits are the raider's own (raider.rs).
pub const ACTIONS: &[(&str, u32)] = &[
    ("forward", raider::B_FWD),
    ("back", raider::B_BACK),
    ("left", raider::B_LEFT),
    ("right", raider::B_RIGHT),
    ("fire", raider::B_FIRE),
    ("aim", raider::B_AIM),
    ("reload", raider::B_RELOAD),
    ("use", raider::B_USE),
    ("sprint", raider::B_SPRINT),
    ("jump", raider::B_JUMP),
    ("cancel", 1 << 11),
    ("turn_left", 1 << 12),
    ("turn_right", 1 << 13),
];
const RUN_BITS: u32 = (1 << 10) - 1;
const CANCEL: u32 = 1 << 11;
const TURN_LEFT: u32 = 1 << 12;
const TURN_RIGHT: u32 = 1 << 13;
/// Turning by keys: angle units a tick.
const KEY_TURN: i32 = deg(2);

/// Mouse: angle units turned per CSS pixel ×100.
const LOOK_PER_PX: i32 = deg(14) / 100;
const STARTER: &[(&str, i32)] = &[
    ("smg", 1),
    ("double", 1),
    ("pair", 1),
    ("dot", 1),
    ("fore", 1),
    ("ember", 1),
];
const RESULTS_AFTER: i32 = secs(2500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Title,
    Workshop,
    Board,
    Lobby,
    Play,
    Paused,
    Results,
}

impl Mode {
    fn name(self) -> &'static str {
        match self {
            Mode::Title => "title",
            Mode::Workshop => "workshop",
            Mode::Board => "board",
            Mode::Lobby => "lobby",
            Mode::Play => "play",
            Mode::Paused => "paused",
            Mode::Results => "results",
        }
    }
}

/// What a person keeps between runs, in their own browser.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Save {
    #[serde(default)]
    pub stash: BTreeMap<String, i32>,
    #[serde(default)]
    pub build: Vec<String>,
    #[serde(default)]
    pub runs: i32,
    #[serde(default)]
    pub extracted: i32,
    #[serde(default)]
    pub orders: i32,
    #[serde(default)]
    pub sens: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Results {
    pub success: bool,
    pub aborted: bool,
    pub lives: Vec<Life>,
    pub kills: Vec<i32>,
    pub found: Vec<usize>,
    pub pay: Vec<usize>,
    pub lost: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Host,
    Joiner,
}

/// The lobby and the wire, from this end.
#[derive(Debug, Clone)]
pub struct Online {
    pub role: Role,
    pub room: String,
    pub build: String,
    pub status: &'static str,
    pub outbox: Vec<(u32, String)>,
    /// host: peer id → seat
    pub peers: BTreeMap<u32, usize>,
    pub host: Option<u32>,
    pub seats: Vec<Seat>,
    pub you: usize,
    pub mission: usize,
    pub started: bool,
    pub sums_sent: u32,
    pub desync: Option<u32>,
}

impl Online {
    fn new(role: Role, room: &str, build: &str) -> Online {
        Online {
            role,
            room: room.to_string(),
            build: build.to_string(),
            status: if role == Role::Host {
                "online_hosting"
            } else {
                "online_joining"
            },
            outbox: Vec::new(),
            peers: BTreeMap::new(),
            host: None,
            seats: Vec::new(),
            you: 0,
            mission: 0,
            started: false,
            sums_sent: 0,
            desync: None,
        }
    }

    fn send(&mut self, peer: u32, m: &Msg) {
        self.outbox.push((peer, rollback::encode(m)));
    }

    fn broadcast(&mut self, m: &Msg, except: Option<u32>) {
        let text = rollback::encode(m);
        let peers: Vec<u32> = self
            .peers
            .keys()
            .copied()
            .filter(|p| Some(*p) != except)
            .collect();
        for p in peers {
            self.outbox.push((p, text.clone()));
        }
    }
}

pub struct Game {
    pub content: Content,
    pub map: Map,
    pub mode: Mode,
    pub save: Save,
    pub build: Build,
    pub pick: usize,
    pub setup: Option<Setup>,
    pub run: Option<Run>,
    pub session: Option<Session>,
    pub online: Option<Online>,
    pub me: usize,
    pub results: Option<Results>,
    over_t: i32,
    menu_open: bool,
    // the clock
    last_us: Option<i64>,
    acc: i64,
    frame: u32,
    pending: u32,
    prev_bits: u32,
    // the local look, ahead of the run
    yaw: i32,
    pitch: i32,
    kick: i32,
    // presentation
    particles: Vec<Particle>,
    beams: Vec<(V3, V3, i32, Rgb)>,
    sounds: Vec<&'static str>,
    hit_t: i32,
    hit_weak: bool,
    kill_t: i32,
    hurt_t: i32,
    toast: Option<(&'static str, i32)>,
    prng: Rng,
    eye: V3,
    fwd: V3,
    aspect: i32,
    static_values: Vec<i32>,
    static_draws: Vec<Vec<u32>>,
    dyn_values: Vec<i32>,
    dyn_draws: Vec<Vec<u32>>,
    scene_version: u32,
    preview: Option<Gun>,
    /// sounds already in a HUD the page has read
    sounds_heard: usize,
    /// goodbyes still to send after leaving
    parting: Vec<(u32, String)>,
}

fn pct(n: i32, d: i32) -> String {
    if d <= 0 {
        return "0%".into();
    }
    format!("{}%", (n.clamp(0, d) as i64 * 100 / d as i64))
}

fn pct_q16(q: i32) -> String {
    format!("{}.{}%", q / ONE, (q % ONE).abs() * 10 / ONE)
}

fn seconds(ticks: i32) -> String {
    let s = ticks.max(0) / TICKS_PER_SECOND;
    format!("{}:{:02}", s / 60, s % 60)
}

fn tenths(ticks: i32) -> String {
    let t = ticks.max(0) * 10 / TICKS_PER_SECOND;
    format!("{}.{}", t / 10, t % 10)
}

fn metres(q: i32) -> String {
    format!("{} m", q / ONE)
}

fn hex(c: Rgb) -> String {
    let ch = |v: i32| (v.clamp(0, ONE) * 255 / ONE) as u8;
    format!("#{:02X}{:02X}{:02X}", ch(c[0]), ch(c[1]), ch(c[2]))
}

impl Game {
    pub fn new(
        parts: &str,
        drones: &str,
        missions: &str,
        palette: &str,
        saved: &str,
    ) -> Result<Game, String> {
        let content = Content::parse(parts, drones, missions, palette)?;
        let map = mall::build(&content.palette);
        let mut b = Batch::new(content.palette.get("ink"));
        draw::map_scene(&map, &mut b);
        let (static_values, static_draws) = b.finish(render::STATIC_BUFFER);
        let build = gun::basic_build(&content);
        let mut g = Game {
            content,
            map,
            mode: Mode::Title,
            save: Save {
                stash: BTreeMap::new(),
                build: Vec::new(),
                runs: 0,
                extracted: 0,
                orders: 0,
                sens: 100,
            },
            build,
            pick: 0,
            setup: None,
            run: None,
            session: None,
            online: None,
            me: 0,
            results: None,
            over_t: 0,
            menu_open: false,
            last_us: None,
            acc: 0,
            frame: 0,
            pending: 0,
            prev_bits: 0,
            yaw: 0,
            pitch: 0,
            kick: 0,
            particles: Vec::new(),
            beams: Vec::new(),
            sounds: Vec::new(),
            hit_t: 0,
            hit_weak: false,
            kill_t: 0,
            hurt_t: 0,
            toast: None,
            prng: Rng::new(77),
            eye: v3(0, int(30), int(260)),
            fwd: v3(0, 0, -ONE),
            aspect: ONE * 16 / 9,
            static_values,
            static_draws,
            dyn_values: Vec::new(),
            dyn_draws: Vec::new(),
            scene_version: 1,
            preview: None,
            parting: Vec::new(),
            sounds_heard: 0,
        };
        g.load_saved(saved);
        Ok(g)
    }

    // ---- the record ---------------------------------------------------------------

    fn load_saved(&mut self, saved: &str) {
        match serde_json::from_str::<Save>(saved) {
            Ok(s) => self.save = s,
            Err(_) => {
                for (id, n) in STARTER {
                    self.save.stash.insert(id.to_string(), *n);
                }
            }
        }
        if self.save.sens <= 0 {
            self.save.sens = 100;
        }
        // the build, if every part still exists; else the basic one
        let mut b = gun::basic_build(&self.content);
        if self.save.build.len() == 6 {
            for (i, id) in self.save.build.iter().enumerate() {
                if let Some(k) = self.content.part(id) {
                    if self.content.parts[k].cat == Cat::ALL[i] {
                        b[i] = k;
                    }
                }
            }
        }
        self.build = b;
        self.save.build = self.build_ids(&b);
    }

    pub fn saved(&self) -> String {
        serde_json::to_string(&self.save).unwrap_or_default()
    }

    fn build_ids(&self, b: &Build) -> Vec<String> {
        b.iter().map(|&k| self.content.parts[k].id.clone()).collect()
    }

    fn build_from_ids(&self, ids: &[String]) -> Build {
        let mut b = gun::basic_build(&self.content);
        for (i, id) in ids.iter().enumerate().take(6) {
            if let Some(k) = self.content.part(id) {
                if self.content.parts[k].cat == Cat::ALL[i] {
                    b[i] = k;
                }
            }
        }
        b
    }

    fn owned(&self, k: usize) -> i32 {
        let p = &self.content.parts[k];
        if p.basic {
            return 99;
        }
        *self.save.stash.get(&p.id).unwrap_or(&0)
    }

    fn give(&mut self, k: usize, n: i32) {
        let p = &self.content.parts[k];
        if p.basic {
            return;
        }
        let e = self.save.stash.entry(p.id.clone()).or_insert(0);
        *e = (*e + n).max(0);
        if *e == 0 {
            self.save.stash.remove(&p.id);
        }
    }

    // ---- the screens ---------------------------------------------------------------

    pub fn workshop(&mut self) {
        if matches!(self.mode, Mode::Title | Mode::Board) {
            self.mode = Mode::Workshop;
            self.preview = Some(gun::compose(&self.content, &self.build));
        }
    }

    /// Puts a part from the stash on the gun; the part it replaces goes back.
    pub fn equip(&mut self, id: &str) -> bool {
        if self.mode != Mode::Workshop {
            return false;
        }
        let Some(k) = self.content.part(id) else {
            return false;
        };
        let slot = self.content.parts[k].cat.index();
        let old = self.build[slot];
        if old == k || self.owned(k) <= 0 {
            return false;
        }
        self.give(k, -1);
        self.give(old, 1);
        self.build[slot] = k;
        self.save.build = self.build_ids(&self.build.clone());
        self.preview = Some(gun::compose(&self.content, &self.build));
        self.sounds.push("equip");
        true
    }

    pub fn board(&mut self) {
        if matches!(self.mode, Mode::Title | Mode::Workshop) && self.online.is_none() {
            self.mode = Mode::Board;
        }
    }

    pub fn pick_mission(&mut self, i: usize) {
        if i < self.content.missions.len() {
            self.pick = i;
            if let Some(o) = &mut self.online {
                if o.role == Role::Host && !o.started {
                    o.mission = i;
                }
            }
            self.lobby_changed();
        }
    }

    pub fn back(&mut self) {
        match self.mode {
            Mode::Workshop | Mode::Board => self.mode = Mode::Title,
            Mode::Results => {
                self.results = None;
                self.run = None;
                self.session = None;
                self.setup = None;
                self.mode = if self.online.is_some() {
                    Mode::Lobby
                } else {
                    Mode::Title
                };
                if let Some(o) = &mut self.online {
                    o.started = false;
                    if o.role == Role::Joiner {
                        o.status = "online_waiting";
                    }
                }
                self.lobby_changed();
            }
            _ => {}
        }
    }

    /// Alone: the chosen order, now.
    pub fn start_solo(&mut self) {
        if self.mode != Mode::Board {
            return;
        }
        let seed = (self.frame as u64).wrapping_mul(0x9E37_79B9) ^ ((self.save.runs as u64 + 1) * 7919);
        self.begin_run(self.pick, seed, vec![self.build], 0, false);
    }

    fn begin_run(&mut self, mission: usize, seed: u64, builds: Vec<Build>, me: usize, online: bool) {
        let guns: Vec<Gun> = builds.iter().map(|b| gun::compose(&self.content, b)).collect();
        let setup = Setup {
            mission,
            builds: builds.clone(),
            guns,
            seed,
        };
        let run = {
            let r = Ref {
                content: &self.content,
                map: &self.map,
                setup: &setup,
            };
            Run::new(&r)
        };
        self.yaw = run.raiders[me].yaw;
        self.pitch = 0;
        self.me = me;
        self.session = if online {
            Some(Session::new(builds.len(), me))
        } else {
            None
        };
        self.run = Some(run);
        self.setup = Some(setup);
        self.mode = Mode::Play;
        self.results = None;
        self.over_t = 0;
        self.menu_open = false;
        self.particles.clear();
        self.beams.clear();
        self.save.runs += 1;
        self.toast = Some(("toast_doors", secs(5000)));
        self.sounds.push("van");
    }

    pub fn pause(&mut self) {
        if self.mode == Mode::Play {
            if self.online.is_some() {
                self.menu_open = true;
            } else {
                self.mode = Mode::Paused;
            }
        }
    }

    pub fn resume(&mut self) {
        if self.mode == Mode::Paused {
            self.mode = Mode::Play;
            self.last_us = None;
        }
        self.menu_open = false;
    }

    /// Give up the run: it counts as not coming back.
    pub fn quit_run(&mut self) {
        if !matches!(self.mode, Mode::Play | Mode::Paused) {
            return;
        }
        if let Some(run) = &mut self.run {
            let me = self.me;
            if run.raiders[me].life.active() {
                run.raiders[me].life = Life::Gone;
            }
        }
        if self.online.is_some() {
            self.leave_online();
        }
        self.finish(false);
    }

    /// A developer's shortcut for looking at the map: `x,z` in metres puts a
    /// solo raider there with the van doors open. Online runs ignore it, so
    /// no copy can disagree because of it.
    pub fn dev_place(&mut self, at: &str) {
        if self.session.is_some() || self.mode != Mode::Play {
            return;
        }
        let nums: Vec<i32> = at
            .split(',')
            .filter_map(|t| t.trim().parse::<i32>().ok())
            .collect();
        let (Some(run), [x, z]) = (&mut self.run, nums.as_slice()) else {
            return;
        };
        let p = v3(int(*x), 0, int(*z));
        let floor = self.map.floor_under(
            &crate::world::Aabb::standing(v3(p.x, ONE, p.z), ONE / 3, ONE),
            ONE,
            &run.extra,
        );
        run.raiders[self.me].pos = v3(p.x, floor, p.z);
        run.van.doors_open = true;
        run.extra = run.van.boxes();
    }

    pub fn set_sens(&mut self, pct: i32) {
        self.save.sens = pct.clamp(20, 300);
    }

    /// The mouse a step faster or slower.
    pub fn nudge_sens(&mut self, up: bool) {
        self.set_sens(self.save.sens + if up { 10 } else { -10 });
    }

    // ---- the end of a run -----------------------------------------------------------

    fn finish(&mut self, aborted: bool) {
        let (Some(run), Some(setup)) = (&self.run, &self.setup) else {
            return;
        };
        let me = self.me;
        let mine = run.raiders[me].life;
        let success = run.over == Some(true) && mine == Life::Extracted;
        let mut res = Results {
            success,
            aborted,
            lives: run.raiders.iter().map(|r| r.life).collect(),
            kills: run.raiders.iter().map(|r| r.kills).collect(),
            found: Vec::new(),
            pay: Vec::new(),
            lost: Vec::new(),
        };
        if aborted {
            // nobody's fault: nothing gained, nothing lost
        } else if mine == Life::Extracted {
            for it in &run.packs[me] {
                if let Item::Part(k) = it {
                    res.found.push(*k as usize);
                }
            }
            if success {
                let m = &self.content.missions[setup.mission];
                let mut rng = Rng::new(setup.seed ^ ((me as u64 + 1) * 0xabcdef));
                let pool: Vec<usize> = (0..self.content.parts.len())
                    .filter(|&k| self.content.parts[k].rarity >= 2)
                    .collect();
                for _ in 0..m.pay {
                    res.pay.push(pool[rng.range(0, pool.len() as i32 - 1) as usize]);
                }
            }
        } else {
            res.lost = Run::staked(&self.content, &self.build);
        }
        for &k in res.found.iter().chain(res.pay.iter()) {
            self.give(k, 1);
        }
        if !res.lost.is_empty() {
            let basic = gun::basic_build(&self.content);
            for &k in &res.lost {
                let slot = self.content.parts[k].cat.index();
                self.build[slot] = basic[slot];
            }
            self.save.build = self.build_ids(&self.build.clone());
        }
        if mine == Life::Extracted {
            self.save.extracted += 1;
        }
        if success {
            self.save.orders += 1;
        }
        self.sounds.push(if success { "win" } else { "lose" });
        self.results = Some(res);
        self.mode = Mode::Results;
        if let Some(o) = &mut self.online {
            o.started = false;
        }
    }

    // ---- online -------------------------------------------------------------------------

    pub fn host_online(&mut self, build: &str, seed: u64) -> String {
        let room = crate::online_code::room_code(seed);
        let mut o = Online::new(Role::Host, &room, build);
        o.seats = vec![Seat {
            kind: "host".into(),
            name: String::new(),
            build: self.build_ids(&self.build.clone()),
        }];
        o.mission = self.pick;
        self.online = Some(o);
        self.mode = Mode::Lobby;
        room
    }

    pub fn join_online(&mut self, build: &str, code: &str) -> bool {
        let room = if code.is_empty() {
            String::new()
        } else {
            match crate::online_code::tidy_code(code) {
                Some(r) => r,
                None => return false,
            }
        };
        self.online = Some(Online::new(Role::Joiner, &room, build));
        self.mode = Mode::Lobby;
        true
    }

    pub fn leave_online(&mut self) {
        if let Some(o) = &mut self.online {
            let m = Msg::Bye {
                why: "online_left".into(),
            };
            let peers: Vec<u32> = o.peers.keys().copied().chain(o.host).collect();
            for p in peers {
                o.send(p, &m);
            }
        }
        // the outbox is read once more by the page before the link closes
        let out = self.online.as_ref().map(|o| o.outbox.clone()).unwrap_or_default();
        self.online = None;
        self.parting = out;
        if matches!(self.mode, Mode::Lobby) {
            self.mode = Mode::Title;
        }
    }

    pub fn room(&self) -> String {
        self.online.as_ref().map(|o| o.room.clone()).unwrap_or_default()
    }

    pub fn net_peer(&mut self, peer: u32) {
        let gun = self.build_ids(&self.build.clone());
        let Some(o) = &mut self.online else { return };
        match o.role {
            Role::Joiner => {
                if o.host.is_none() {
                    o.host = Some(peer);
                    let m = Msg::Hello {
                        proto: rollback::PROTO,
                        build: o.build.clone(),
                        gun,
                    };
                    o.send(peer, &m);
                    // "waiting" once the host's lobby, with my seat in it, arrives
                    o.status = "online_joining";
                }
            }
            Role::Host => {}
        }
    }

    pub fn net_left(&mut self, peer: u32) {
        let Some(o) = &mut self.online else { return };
        match o.role {
            Role::Joiner => {
                if o.host == Some(peer) {
                    o.status = "online_host_left";
                    o.host = None;
                    if matches!(self.mode, Mode::Play | Mode::Paused) {
                        self.finish(true);
                    }
                }
            }
            Role::Host => {
                let Some(seat) = o.peers.remove(&peer) else { return };
                if o.started {
                    if let Some(s) = &mut self.session {
                        let from = s.upto[seat];
                        s.left(seat, from);
                        let m = Msg::Left { p: seat, from };
                        if let Some(o) = &mut self.online {
                            o.broadcast(&m, None);
                        }
                    }
                    self.toast = Some(("toast_left", secs(4000)));
                } else {
                    // a lobby seat frees up; later seats move down
                    o.seats.remove(seat);
                    for s in o.peers.values_mut() {
                        if *s > seat {
                            *s -= 1;
                        }
                    }
                    self.lobby_changed();
                }
            }
        }
    }

    pub fn net_error(&mut self, key: &str) {
        if let Some(o) = &mut self.online {
            o.status = match key {
                "online_no_relay" => "online_no_relay",
                "online_no_path" => "online_no_path",
                _ => "online_error",
            };
        }
    }

    pub fn net_outbox(&mut self) -> String {
        let mut out = std::mem::take(&mut self.parting);
        if let Some(o) = &mut self.online {
            out.extend(std::mem::take(&mut o.outbox));
        }
        serde_json::to_string(&out).unwrap_or_else(|_| "[]".into())
    }

    pub fn net_message(&mut self, peer: u32, text: &str) {
        let Some(m) = rollback::decode(text) else { return };
        let Some(role) = self.online.as_ref().map(|o| o.role) else {
            return;
        };
        match role {
            Role::Host => self.host_hears(peer, m),
            Role::Joiner => self.joiner_hears(m),
        }
    }

    fn host_hears(&mut self, peer: u32, m: Msg) {
        let Some(o) = &mut self.online else { return };
        match m {
            Msg::Hello { proto, build, gun } => {
                if proto != rollback::PROTO || build != o.build {
                    o.send(
                        peer,
                        &Msg::Bye {
                            why: "online_version".into(),
                        },
                    );
                    return;
                }
                if o.started || o.seats.len() >= 4 {
                    o.send(
                        peer,
                        &Msg::Bye {
                            why: "online_full".into(),
                        },
                    );
                    return;
                }
                if o.peers.contains_key(&peer) {
                    return;
                }
                o.peers.insert(peer, o.seats.len());
                o.seats.push(Seat {
                    kind: "remote".into(),
                    name: String::new(),
                    build: gun,
                });
                self.sounds.push("join");
                self.lobby_changed();
            }
            Msg::Gun { gun } => {
                if let Some(&s) = o.peers.get(&peer) {
                    if !o.started {
                        o.seats[s].build = gun;
                        self.lobby_changed();
                    }
                }
            }
            Msg::In { from, i, at, .. } => {
                let Some(&p) = o.peers.get(&peer) else { return };
                // pass it on to the other joiners, as the joiner's own
                let fwd = Msg::In {
                    p,
                    from,
                    i: i.clone(),
                    at,
                };
                o.broadcast(&fwd, Some(peer));
                if let Some(s) = &mut self.session {
                    s.remote(p, from, &i, at);
                }
            }
            Msg::Sum { tick, sum } => {
                let mine = self.session.as_ref().and_then(|s| s.sums.get(&tick).copied());
                if let Some(mine) = mine {
                    if mine != sum {
                        if let Some(o) = &mut self.online {
                            o.desync = Some(tick);
                            o.send(peer, &Msg::Desync { tick });
                        }
                    }
                }
            }
            Msg::Bye { .. } => self.net_left(peer),
            _ => {}
        }
    }

    fn joiner_hears(&mut self, m: Msg) {
        let Some(o) = &mut self.online else { return };
        match m {
            Msg::Lobby { seats, mission, you } => {
                o.seats = seats;
                o.mission = mission;
                o.you = you;
                o.status = "online_waiting";
                self.pick = mission;
            }
            Msg::Start {
                mission,
                seed,
                guns,
                you,
            } => {
                o.started = true;
                o.status = "online_playing";
                let builds: Vec<Build> = guns.iter().map(|g| self.build_from_ids(g)).collect();
                // play with my own gun, whatever the host thinks it is
                let mut builds = builds;
                if you < builds.len() {
                    builds[you] = self.build;
                }
                self.begin_run(mission, seed, builds, you, true);
            }
            Msg::In { p, from, i, at } => {
                if let Some(s) = &mut self.session {
                    s.remote(p, from, &i, at);
                }
            }
            Msg::Left { p, from } => {
                if let Some(s) = &mut self.session {
                    s.left(p, from);
                }
                self.toast = Some(("toast_left", secs(4000)));
            }
            Msg::Desync { tick } => o.desync = Some(tick),
            Msg::Bye { why } => {
                o.status = match why.as_str() {
                    "online_full" => "online_full",
                    "online_version" => "online_version",
                    _ => "online_host_left",
                };
                o.host = None;
                if matches!(self.mode, Mode::Play | Mode::Paused) {
                    self.finish(true);
                }
            }
            _ => {}
        }
    }

    fn lobby_changed(&mut self) {
        let Some(o) = &mut self.online else { return };
        if o.role != Role::Host {
            return;
        }
        o.seats[0].build = self.save.build.clone();
        let peers: Vec<(u32, usize)> = o.peers.iter().map(|(p, s)| (*p, *s)).collect();
        for (p, s) in peers {
            let m = Msg::Lobby {
                seats: o.seats.clone(),
                mission: o.mission,
                you: s,
            };
            o.send(p, &m);
        }
    }

    /// Host: start the run for everyone in the lobby.
    pub fn start_online(&mut self) {
        let Some(o) = &mut self.online else { return };
        if o.role != Role::Host || o.started || self.mode != Mode::Lobby {
            return;
        }
        o.started = true;
        o.status = "online_playing";
        let seed =
            (self.frame as u64).wrapping_mul(2654435761) ^ (o.room.len() as u64 + o.seats.len() as u64 * 977);
        let guns: Vec<Vec<String>> = o.seats.iter().map(|s| s.build.clone()).collect();
        let mission = o.mission;
        let peers: Vec<(u32, usize)> = o.peers.iter().map(|(p, s)| (*p, *s)).collect();
        for (p, s) in peers {
            let m = Msg::Start {
                mission,
                seed,
                guns: guns.clone(),
                you: s,
            };
            o.send(p, &m);
        }
        let mut builds: Vec<Build> = guns.iter().map(|g| self.build_from_ids(g)).collect();
        builds[0] = self.build;
        self.begin_run(mission, seed, builds, 0, true);
    }

    // ---- the clock ------------------------------------------------------------------------

    /// The page's clock (µs), the held action bits, the mouse's movement
    /// (CSS pixels ×100).
    pub fn advance(&mut self, now_us: i64, bits: u32, dx: i32, dy: i32) {
        // every sound is heard once: what the last frame reported goes, and a
        // sound a button made since then stays for this frame's HUD
        let heard = self.sounds_heard.min(self.sounds.len());
        self.sounds.drain(..heard);
        let dt = match self.last_us {
            None => TICK_US,
            Some(last) => (now_us - last).clamp(0, MAX_CATCH_UP_US),
        };
        self.last_us = Some(now_us);
        self.acc += dt;
        let edges = bits & !self.prev_bits;
        self.prev_bits = bits;
        self.pending |= edges;
        let mut due = 0u32;
        while self.acc >= TICK_US {
            self.acc -= TICK_US;
            due += 1;
        }
        if self.mode == Mode::Play && edges & CANCEL != 0 {
            if self.online.is_some() {
                self.menu_open = !self.menu_open;
            } else {
                self.mode = Mode::Paused;
            }
        }
        // the look
        if self.mode == Mode::Play && !self.menu_open {
            let zoom = self.aim_zoom(bits);
            let k = self.save.sens * zoom / 100;
            self.yaw = fx::wrap(self.yaw - dx * LOOK_PER_PX / 100 * k / 100);
            self.pitch = (self.pitch - dy * LOOK_PER_PX / 100 * k / 100).clamp(-MAX_PITCH, MAX_PITCH);
            let turn = (bits & TURN_LEFT != 0) as i32 - (bits & TURN_RIGHT != 0) as i32;
            self.yaw = fx::wrap(self.yaw + turn * KEY_TURN * due as i32 * zoom / 100);
        }
        for _ in 0..due {
            self.frame = self.frame.wrapping_add(1);
            self.present_tick();
        }
        if matches!(self.mode, Mode::Play) {
            self.play_frame(bits, due);
        }
        if self.mode == Mode::Workshop {
            self.workshop_camera();
        }
        self.render();
        self.sounds_heard = self.sounds.len();
    }

    fn aim_zoom(&self, bits: u32) -> i32 {
        let (Some(s), true) = (&self.setup, bits & B_AIM != 0) else {
            return 100;
        };
        s.guns[self.me].zoom_pct
    }

    fn play_frame(&mut self, bits: u32, due: u32) {
        let held = if self.menu_open { 0 } else { bits & RUN_BITS };
        let input = Input {
            b: held | (self.pending & RUN_BITS),
            y: self.yaw,
            p: self.pitch,
        };
        self.pending = 0;
        let (Some(run), Some(setup)) = (&mut self.run, &self.setup) else {
            return;
        };
        let r = Ref {
            content: &self.content,
            map: &self.map,
            setup,
        };
        let mut events = Vec::new();
        match &mut self.session {
            None => {
                for _ in 0..due {
                    run.step(&r, &[input]);
                    events.extend(run.events.iter().copied());
                }
            }
            Some(s) => {
                // keep pace: a copy far ahead of the others gives up a tick now and then
                let mut due = due;
                if s.ahead() > (rollback::DELAY as i32 + 3) && self.frame.is_multiple_of(4) && due > 0 {
                    due -= 1;
                }
                for _ in 0..due.max(1) {
                    s.local(input);
                }
                events = s.advance(run, &r, due);
                s.prune();
                let msg = s.take_unsent();
                let new_sums: Vec<(u32, u64)> = s.sums.iter().map(|(t, v)| (*t, *v)).collect();
                if let Some(o) = &mut self.online {
                    if let Some(m) = msg {
                        match o.role {
                            Role::Host => o.broadcast(&m, None),
                            Role::Joiner => {
                                if let Some(h) = o.host {
                                    o.send(h, &m);
                                }
                            }
                        }
                    }
                    if o.role == Role::Joiner {
                        for (t, sum) in new_sums {
                            if t >= o.sums_sent {
                                o.sums_sent = t + 1;
                                if let Some(h) = o.host {
                                    o.send(h, &Msg::Sum { tick: t, sum });
                                }
                            }
                        }
                    }
                }
            }
        }
        for e in events {
            self.event(e);
        }
        // the run is over once every input up to its end is known
        let over = self.run.as_ref().is_some_and(|run| run.over.is_some());
        let settled = match (&self.session, &self.run) {
            (Some(s), Some(run)) => s.confirmed() >= run.tick,
            _ => true,
        };
        if over && settled {
            self.over_t += due as i32;
            if self.over_t >= RESULTS_AFTER {
                self.finish(false);
            }
        }
        self.kick = self.kick * 80 / 100;
    }

    fn present_tick(&mut self) {
        for p in self.particles.iter_mut() {
            p.step();
        }
        self.particles.retain(|p| p.life > 0);
        for b in self.beams.iter_mut() {
            b.2 -= 1;
        }
        self.beams.retain(|b| b.2 > 0);
        self.hit_t = (self.hit_t - 1).max(0);
        self.kill_t = (self.kill_t - 1).max(0);
        self.hurt_t = (self.hurt_t - 1).max(0);
        if let Some((k, t)) = self.toast {
            self.toast = if t > 1 { Some((k, t - 1)) } else { None };
        }
    }

    fn c(&self, name: &str) -> Rgb {
        self.content.palette.get(name)
    }

    fn burst(
        &mut self,
        at: V3,
        n: i32,
        colours: &[&str],
        speed: i32,
        size: i32,
        glow: i32,
        life: i32,
        mesh: Mesh,
    ) {
        let cols: Vec<Rgb> = colours.iter().map(|c| self.c(c)).collect();
        for k in 0..n {
            let a = self.prng.range(0, fx::TURN - 1);
            let up = self.prng.range(-speed / 3, speed);
            let out = facing(a, 0).scale(speed * self.prng.range(30, 100) / 100);
            self.particles.push(Particle {
                pos: at,
                vel: out.add(v3(0, up, 0)),
                life: life * self.prng.range(60, 100) / 100,
                max: life,
                size: size * self.prng.range(70, 130) / 100,
                rgb: cols[k as usize % cols.len()],
                glow,
                fall: glow == 0,
                mesh,
            });
        }
    }

    fn event(&mut self, e: Event) {
        let me = self.me as u8;
        match e {
            Event::Fire { who, at, kind } => {
                if who == me {
                    let recoil = self.setup.as_ref().map(|s| s.guns[self.me].recoil).unwrap_or(0);
                    self.pitch = (self.pitch + recoil).clamp(-MAX_PITCH, MAX_PITCH);
                    self.yaw = fx::wrap(self.yaw + self.prng.range(-recoil / 3, recoil / 3));
                    self.kick = ONE;
                }
                self.sounds.push(match kind {
                    crate::content::ShotKind::Bullet => "shot",
                    crate::content::ShotKind::Rocket => "rocket",
                    crate::content::ShotKind::Grenade => "lob",
                });
                self.burst(
                    at,
                    3,
                    &["spark", "tracer"],
                    int(3),
                    ONE / 8,
                    ONE,
                    secs(120),
                    Mesh::Sphere,
                );
            }
            Event::DroneFire { at, heavy } => {
                self.sounds.push(if heavy { "orb" } else { "zap" });
                self.burst(
                    at,
                    2,
                    &["enemy_shot"],
                    int(2),
                    ONE / 8,
                    ONE,
                    secs(150),
                    Mesh::Sphere,
                );
            }
            Event::Laser { from, to } => {
                let c = self.c("laser");
                self.beams.push((from, to, secs(300), c));
                self.sounds.push("laser");
                self.burst(
                    to,
                    6,
                    &["laser", "spark"],
                    int(4),
                    ONE / 8,
                    ONE,
                    secs(300),
                    Mesh::Sphere,
                );
            }
            Event::Spark { at } => {
                self.burst(at, 3, &["spark"], int(3), ONE / 12, ONE, secs(200), Mesh::Sphere);
            }
            Event::Hit { at, weak, armour, by } => {
                if by == me {
                    self.hit_t = secs(200);
                    self.hit_weak = weak;
                    self.sounds.push(if weak {
                        "weak"
                    } else if armour {
                        "clank"
                    } else {
                        "hit"
                    });
                }
                let c = if weak {
                    "weak"
                } else if armour {
                    "metal"
                } else {
                    "spark"
                };
                self.burst(
                    at,
                    if weak { 7 } else { 4 },
                    &[c, "spark"],
                    int(4),
                    ONE / 10,
                    ONE,
                    secs(250),
                    Mesh::Sphere,
                );
            }
            Event::RotorOff { at } => {
                self.sounds.push("rotor");
                self.burst(
                    at,
                    8,
                    &["rotor", "metal_dark"],
                    int(5),
                    ONE / 6,
                    0,
                    secs(1200),
                    Mesh::Cube,
                );
            }
            Event::Boom { at, radius, effect } => {
                let c = match effect {
                    crate::content::Effect::Goo => "goo",
                    crate::content::Effect::Shock => "shock",
                    crate::content::Effect::Ember => "ember",
                    crate::content::Effect::None => "blast",
                };
                self.sounds.push("boom");
                self.burst(
                    at,
                    14,
                    &[c, "spark", "ember"],
                    radius * 2,
                    radius / 3,
                    ONE,
                    secs(500),
                    Mesh::Sphere,
                );
                self.burst(at, 8, &["smoke"], int(2), radius / 3, 0, secs(1400), Mesh::Blob);
            }
            Event::Wreck { at, .. } => {
                self.sounds.push("wreck");
                self.burst(
                    at,
                    12,
                    &["blast", "ember", "spark"],
                    int(5),
                    ONE / 2,
                    ONE,
                    secs(500),
                    Mesh::Sphere,
                );
                self.burst(
                    at,
                    10,
                    &["metal", "metal_dark", "rust"],
                    int(6),
                    ONE / 5,
                    0,
                    secs(1800),
                    Mesh::Cube,
                );
            }
            Event::Kill { by } => {
                if by == me {
                    self.kill_t = secs(600);
                    self.sounds.push("kill");
                }
            }
            Event::Hurt { who } => {
                if who == me {
                    self.hurt_t = secs(400);
                    self.sounds.push("hurt");
                }
            }
            Event::Downed { who } => {
                self.sounds.push("downed");
                if who == me {
                    self.toast = Some(("toast_downed", secs(4000)));
                }
            }
            Event::Revived { .. } => self.sounds.push("revive"),
            Event::Died { .. } => self.sounds.push("died"),
            Event::Picked { who, item } => {
                if who == me {
                    self.sounds.push(if item == Item::Mission {
                        "objective"
                    } else {
                        "pick"
                    });
                }
            }
            Event::Searched { at } => {
                self.sounds.push("search");
                self.burst(
                    at,
                    6,
                    &["loot", "cream"],
                    int(2),
                    ONE / 10,
                    ONE,
                    secs(400),
                    Mesh::Sphere,
                );
            }
            Event::Reload { who } => {
                if who == me {
                    self.sounds.push("reload");
                }
            }
            Event::Doors => {
                self.sounds.push("doors");
                self.toast = Some(("toast_out", secs(5000)));
            }
            Event::VanStart => {
                self.sounds.push("van");
                self.toast = Some(("toast_leaving", secs(4000)));
            }
            Event::Extracted => self.sounds.push("van"),
            Event::Spotted { .. } => self.sounds.push("spotted"),
            Event::Generator => {
                self.sounds.push("generator");
                self.toast = Some(("toast_generator", secs(5000)));
            }
            Event::Objective => {
                self.sounds.push("objective");
                self.toast = Some(("toast_objective", secs(4000)));
            }
            Event::Wave => {
                self.sounds.push("wave");
                self.toast = Some(("toast_wave", secs(3000)));
            }
            Event::Stomp { at } => {
                self.sounds.push("boom");
                self.burst(
                    at,
                    10,
                    &["dry", "smoke"],
                    int(5),
                    ONE / 3,
                    0,
                    secs(700),
                    Mesh::Blob,
                );
            }
        }
    }

    // ---- the camera and the scene --------------------------------------------------------------

    /// My raider as I see them: the run's, with my own look laid over it.
    fn my_raider(&self) -> Option<raider::Raider> {
        let run = self.run.as_ref()?;
        let mut r = run.raiders.get(self.me)?.clone();
        r.yaw = self.yaw;
        r.pitch = self.pitch;
        r.aiming = self.prev_bits & B_AIM != 0 && r.life == Life::Alive && !self.menu_open;
        Some(r)
    }

    /// Whose eyes the camera uses: mine, or a teammate's once I am out.
    fn watched(&self) -> Option<usize> {
        let run = self.run.as_ref()?;
        if run.raiders[self.me].life.active() {
            return Some(self.me);
        }
        run.raiders.iter().position(|r| r.life.active())
    }

    fn camera(&self) -> Camera {
        let fov = match (&self.setup, self.mode) {
            (Some(s), Mode::Play | Mode::Paused) if self.prev_bits & B_AIM != 0 => {
                FOV * s.guns[self.me].zoom_pct / 100
            }
            _ => FOV,
        };
        Camera {
            eye: self.eye,
            fwd: self.fwd,
            fov: fov + fx::mul(self.kick, deg(1)),
        }
    }

    fn workshop_camera(&mut self) {
        // the gun floats above the lot, turning; the camera looks at it from the side
        self.eye = v3(int(2), int(80) + ONE / 4, int(260));
        self.fwd = v3(-ONE, -ONE / 6, 0).norm();
    }

    fn update_camera(&mut self) {
        let Some(run) = &self.run else { return };
        let Some(w) = self.watched() else { return };
        let ra = if w == self.me {
            self.my_raider().unwrap_or_else(|| run.raiders[w].clone())
        } else {
            run.raiders[w].clone()
        };
        let (yaw, pitch) = if w == self.me {
            (self.yaw, self.pitch)
        } else {
            (ra.yaw, ra.pitch)
        };
        let (eye, fwd) = raider::camera(
            ra.pos,
            yaw,
            pitch,
            ra.aiming,
            ra.life == Life::Downed,
            &self.map,
            &run.extra,
        );
        self.eye = eye;
        self.fwd = fwd;
    }

    fn colours(&self, def: usize) -> DroneColours {
        let d = &self.content.drones[def];
        DroneColours {
            body: crate::content::hex(&d.colour),
            dark: crate::content::hex(&d.dark),
            armour: self.c("armour"),
            weak: self.c("weak"),
            rotor: self.c("rotor"),
            eye: self.c("eye"),
            beacon: self.c("beacon"),
            core: self.c("core"),
            goo: self.c("goo"),
            ember: self.c("ember"),
            shock: self.c("shock"),
            laser: self.c("laser"),
        }
    }

    fn render(&mut self) {
        let mut b = Batch::new(self.c("ink"));
        let tick = self.frame;
        if self.mode == Mode::Workshop {
            if let Some(g) = &self.preview {
                let spin = (self.frame as i32 * deg(1) / 2) & (fx::TURN - 1);
                let at = v3(-int(1), int(80), int(260) - ONE * 9 / 10);
                let frame = Affine::translate(at)
                    .then(&Affine::rot_y(spin))
                    .then(&Affine::scale(v3(ONE * 3, ONE * 3, ONE * 3)));
                draw::gun_model(&mut b, &frame, g);
            }
        }
        if matches!(self.mode, Mode::Play | Mode::Paused | Mode::Results) {
            self.update_camera();
            if let (Some(run), Some(setup)) = (&self.run, &self.setup) {
                let pal = &self.content.palette;
                draw::van(&mut b, &run.van, pal);
                let mine = self.my_raider();
                for (i, ra) in run.raiders.iter().enumerate() {
                    let ra = if i == self.me {
                        mine.as_ref().unwrap_or(ra)
                    } else {
                        ra
                    };
                    let look = draw::raider_look(pal, i);
                    draw::raider(&mut b, ra, &look, &setup.guns[i], tick);
                    draw::muzzle_flash(&mut b, ra, &setup.guns[i], self.c("spark"));
                }
                let near = self.eye;
                for d in &run.drones {
                    if d.pos.dist(near) > int(260) {
                        continue;
                    }
                    let c = self.colours(d.def);
                    draw::drone(&mut b, d, &c, tick);
                    // a hornet charging shows where it aims
                    if d.brain.charge > 0 && d.active() {
                        if let Some(t) = d.brain.target.and_then(|t| run.raiders.get(t as usize)) {
                            if tick % 4 < 3 {
                                draw::beam(
                                    &mut b,
                                    d.to_world(0, -8, -100),
                                    t.chest(),
                                    ONE / 40,
                                    c.laser,
                                    ONE / 2,
                                );
                            }
                        }
                    }
                }
                if let Some(h) = &run.hive {
                    draw::hive(&mut b, h, pal, tick);
                }
                for p in &run.pickups {
                    draw::pickup(&mut b, p, &self.content, tick, pal);
                }
                for s in &run.shots {
                    draw::shot(&mut b, s, pal);
                }
                if matches!(run.generator, Generator::Running(_) | Generator::Done) {
                    if let Some(g) = self.map.spots.iter().find(|s| s.key == "generator") {
                        draw::glow_ball(
                            &mut b,
                            v3(g.at.x, g.at.y + int(2), g.at.z),
                            int(3),
                            self.c("lamp"),
                            ONE / 2,
                        );
                    }
                }
                if let Some(g) = self.map.spots.iter().find(|s| s.key == "generator") {
                    if self.content.missions[setup.mission].goal == Goal::Power {
                        let at = g.at;
                        let base = Affine::translate(at);
                        draw::put(
                            &mut b,
                            Mesh::Cube,
                            &base,
                            v3(0, ONE / 2, 0),
                            v3(int(2), ONE, ONE * 3 / 2),
                            self.c("generator"),
                            0,
                            true,
                        );
                        draw::put(
                            &mut b,
                            Mesh::Cyl,
                            &base,
                            v3(0, ONE * 13 / 10, 0),
                            v3(ONE / 2, ONE / 2, ONE / 2),
                            self.c("metal_dark"),
                            0,
                            true,
                        );
                    }
                }
            }
            for (from, to, life, c) in &self.beams {
                draw::beam(&mut b, *from, *to, ONE / 15, *c, ONE.min(*life * ONE / 10));
            }
        }
        for p in &self.particles {
            p.draw(&mut b);
        }
        let (values, draws) = b.finish(render::DYNAMIC_BUFFER);
        self.dyn_values = values;
        self.dyn_draws = draws;
    }

    pub fn resize(&mut self, css_w: i32, css_h: i32, dpr_x100: i32) -> [i32; 2] {
        let w = (css_w.max(1) * dpr_x100 / 100).clamp(1, 4096);
        let h = (css_h.max(1) * dpr_x100 / 100).clamp(1, 4096);
        self.aspect = fx::ratio(w, h);
        [w, h]
    }

    pub fn view(&self) -> Vec<i32> {
        let cam = self.camera();
        let mut v = cam.values(self.aspect);
        let focus = self
            .watched()
            .and_then(|w| self.run.as_ref().map(|r| r.raiders[w].pos))
            .unwrap_or(v3(0, -int(50), 0));
        let cut = if matches!(self.mode, Mode::Play | Mode::Paused) {
            focus.y
        } else {
            int(500)
        };
        v.extend([focus.x, focus.y + ONE, focus.z, cut]);
        v
    }

    pub fn static_instances(&self) -> Vec<i32> {
        self.static_values.clone()
    }

    pub fn instances(&self) -> Vec<i32> {
        self.dyn_values.clone()
    }

    pub fn draws(&self) -> Vec<u32> {
        let mut out = Vec::new();
        for pass in 0..render::PASSES {
            if let Some(d) = self.static_draws.get(pass) {
                out.extend_from_slice(d);
            }
            if let Some(d) = self.dyn_draws.get(pass) {
                out.extend_from_slice(d);
            }
        }
        out
    }

    pub fn scene_version(&self) -> u32 {
        self.scene_version
    }

    pub fn scene_uniforms(&self) -> Vec<i32> {
        let pal = &self.content.palette;
        let l = v3(ONE * 4 / 10, ONE * 8 / 10, ONE * 3 / 10).norm();
        let mut u = vec![l.x, l.y, l.z];
        u.extend(pal.scene_rgb("sky"));
        u.extend(pal.scene_rgb("fog"));
        u.extend([int(pal.scene_num("fog_near")), int(pal.scene_num("fog_far"))]);
        u.extend(pal.scene_rgb("light"));
        u.extend(pal.scene_rgb("window_dark"));
        u.extend(pal.get("ink"));
        // the see-through cut: near the lens, wide at the lens, wide by the raider, short of them
        u.extend([ONE * 6 / 10, ONE / 2, ONE * 11 / 10, ONE * 9 / 10]);
        u.push(ONE * 55 / 100);
        u
    }

    pub fn clear_color(&self) -> Vec<i32> {
        self.content.palette.scene_rgb("sky").to_vec()
    }

    // ---- the HUD and the screens ------------------------------------------------------------------

    fn marker(&self, cam: &Camera, at: V3, kind: &str, from: V3) -> Value {
        let dist = metres(at.dist(from));
        match cam.screen(self.aspect, at) {
            Some((x, y)) if x > 0 && x < 100 * ONE && y > 0 && y < 100 * ONE => {
                json!({ "kind": kind, "x": pct_q16(x), "y": pct_q16(y), "dist": dist, "off": false })
            }
            _ => {
                // off screen: pinned to the edge, on the side it lies
                let rel = at.sub(cam.eye);
                let right = right_of(yaw_pitch_of(cam.fwd).0);
                let side = rel.dot(right);
                let x = if side >= 0 { 96 * ONE } else { 4 * ONE };
                json!({ "kind": kind, "x": pct_q16(x), "y": "50%", "dist": dist, "off": true })
            }
        }
    }

    pub fn hud_json(&self) -> String {
        let mut h = json!({
            "mode": self.mode.name(),
            "online": self.online.is_some(),
            "sounds": self.sounds,
        });
        let (Some(run), Some(setup)) = (&self.run, &self.setup) else {
            return h.to_string();
        };
        if !matches!(self.mode, Mode::Play | Mode::Paused | Mode::Results) {
            return h.to_string();
        }
        let r = Ref {
            content: &self.content,
            map: &self.map,
            setup,
        };
        let me = &run.raiders[self.me];
        let gun = &setup.guns[self.me];
        let m = &self.content.missions[setup.mission];
        let cam = self.camera();
        let from = me.pos;
        let mut markers = Vec::new();
        // the order's goal
        let goal_done = run.goal_done(&r);
        if !goal_done {
            match m.goal {
                Goal::Fetch => {
                    for p in run.pickups.iter().filter(|p| p.item == Item::Mission) {
                        markers.push(self.marker(
                            &cam,
                            v3(p.pos.x, p.pos.y + ONE, p.pos.z),
                            "objective",
                            from,
                        ));
                    }
                }
                Goal::Hive => {
                    if let Some(hv) = &run.hive {
                        markers.push(self.marker(&cam, hv.core(), "objective", from));
                    }
                }
                Goal::Power => {
                    if let Some(s) = self.map.spots.iter().find(|s| s.key == "generator") {
                        markers.push(self.marker(
                            &cam,
                            v3(s.at.x, s.at.y + int(2), s.at.z),
                            "objective",
                            from,
                        ));
                    }
                }
                Goal::Clear => {
                    if let Some(z) = self.map.zone(&m.zone) {
                        let c = v3((z.min.x + z.max.x) / 2, int(2), (z.min.z + z.max.z) / 2);
                        markers.push(self.marker(&cam, c, "objective", from));
                    }
                }
            }
        }
        let van_at = run.van.pos.add(v3(0, int(3), 0));
        markers.push(self.marker(&cam, van_at, if goal_done { "extract" } else { "van" }, from));
        for (i, o) in run.raiders.iter().enumerate() {
            if i != self.me && o.life.active() {
                markers.push(self.marker(
                    &cam,
                    o.head().add(v3(0, ONE / 2, 0)),
                    if o.life == Life::Downed { "downed" } else { "mate" },
                    from,
                ));
            }
        }
        // the scanner sight marks drones and their weak spots near by
        if gun.scanner && me.life == Life::Alive {
            for d in run
                .drones
                .iter()
                .filter(|d| d.active() && d.pos.dist(from) < int(40))
            {
                if let Some(w) = draw::weak_spot(d) {
                    markers.push(self.marker(&cam, w, "weak", from));
                }
            }
        }
        let prompt = run.use_target(&r, self.me).map(|u| {
            let progress = if me.use_on >= 0 {
                pct(me.use_t, u.needs())
            } else {
                "0%".into()
            };
            let extra = match u {
                Use::Pick(k) => match run.pickups.get(k as usize).map(|p| p.item) {
                    Some(Item::Part(pk)) => self.content.parts[pk as usize].id.clone(),
                    Some(Item::Mission) => format!("item_{}", m.item),
                    None => String::new(),
                },
                _ => String::new(),
            };
            json!({ "key": format!("use_{}", u.name()), "pct": progress, "part": extra })
        });
        let objective = match m.goal {
            Goal::Fetch => json!({ "key": format!("goal_{}", m.id), "have": run.found, "need": m.count }),
            Goal::Clear => {
                let left = run.drones.iter().filter(|d| d.nest && d.active()).count();
                json!({ "key": format!("goal_{}", m.id), "left": left })
            }
            Goal::Hive => {
                json!({ "key": format!("goal_{}", m.id), "hp": run.hive.as_ref().map(|hv| pct(hv.hp, HIVE_HP)).unwrap_or_default() })
            }
            Goal::Power => {
                let left = match run.generator {
                    Generator::Running(t) => seconds(t),
                    _ => String::new(),
                };
                let running = matches!(run.generator, Generator::Running(_));
                let key = if running {
                    "goal_power_running".to_string()
                } else {
                    format!("goal_{}", m.id)
                };
                json!({ "key": key, "running": running, "left": left })
            }
        };
        let zone = self
            .map
            .zone_at(me.pos)
            .map(|z| z.key.clone())
            .unwrap_or_default();
        let team: Vec<Value> = run
            .raiders
            .iter()
            .enumerate()
            .map(|(i, o)| {
                json!({
                    "colour": hex(self.c(&format!("jacket_{}", i % 4))),
                    "hp": pct(o.hp, RAIDER_HP),
                    "life": o.life.name(),
                    "you": i == self.me,
                    "seat": i + 1,
                })
            })
            .collect();
        let pack: Vec<String> = run.packs[self.me]
            .iter()
            .map(|it| match it {
                Item::Part(k) => self.content.parts[*k as usize].id.clone(),
                Item::Mission => format!("item_{}", m.item),
            })
            .collect();
        let spread = if me.aiming { gun.aim_spread } else { gun.hip_spread }
            * if me.moving && !me.aiming { 16 } else { 10 }
            / 10;
        let o = self.online.as_ref();
        h = json!({
            "mode": self.mode.name(),
            "online": self.online.is_some(),
            "menu": self.menu_open,
            "sounds": self.sounds,
            "hp": pct(me.hp, RAIDER_HP),
            "hp_n": me.hp,
            "life": me.life.name(),
            "watching": self.watched().filter(|w| *w != self.me).map(|w| w + 1),
            "bleed": pct(me.bleed, BLEED_OUT),
            "ammo": me.ammo,
            "mag": gun.mag,
            "reload": if me.reloading > 0 { Some(pct(gun.reload - me.reloading, gun.reload)) } else { None },
            "spin": if gun.gatling { Some(pct(me.spin, RELOAD_GATLING_SPIN)) } else { None },
            "aiming": me.aiming,
            "scope": me.aiming && gun.zoom_pct < 50,
            "spread": format!("{}px", 12 + spread * 400 / fx::TURN.max(1) * 3),
            "hit": self.hit_t > 0,
            "weak": self.hit_weak,
            "kill": self.kill_t > 0,
            "hurt": self.hurt_t > 0 || me.hurt_flash > 0,
            "prompt": prompt,
            "objective": objective,
            "done": goal_done,
            "mission": m.id,
            "zone": zone,
            "markers": markers,
            "team": team,
            "pack": pack,
            "pack_max": BACKPACK,
            "van_doors": run.van.doors_open,
            "leaving": run.van.leaving.map(tenths),
            "in_van": run.van.inside().contains(me.chest()),
            "time": seconds(run.tick as i32),
            "toast": self.toast.map(|t| t.0),
            "over": run.over,
            "net": o.map(|o| json!({
                "room": o.room,
                "status": o.status,
                "desync": o.desync.is_some(),
                "rollbacks": self.session.as_ref().map(|s| s.rollbacks).unwrap_or(0),
                "ahead": self.session.as_ref().map(|s| s.ahead()).unwrap_or(0),
                "tick": self.session.as_ref().map(|s| s.current).unwrap_or(0),
                "confirmed": self.session.as_ref().map(|s| s.confirmed()).unwrap_or(0),
                "upto": self.session.as_ref().map(|s| s.upto.clone()).unwrap_or_default(),
                "stalls": self.session.as_ref().map(|s| s.stalls).unwrap_or(0),
                "sums": self.session.as_ref().map(|s| s.sums.iter().rev().take(6).map(|(t, v)| format!("{t}:{v:x}")).collect::<Vec<_>>()).unwrap_or_default(),
            })),
            "drones": run.drones.iter().filter(|d| d.active()).count(),
        });
        h.to_string()
    }

    fn part_json(&self, k: usize) -> Value {
        let p = &self.content.parts[k];
        json!({
            "id": p.id,
            "cat": p.cat.name(),
            "basic": p.basic,
            "rarity": p.rarity,
            "owned": if p.basic { None } else { Some(self.owned(k)) },
        })
    }

    fn gun_stats(&self, g: &Gun) -> Value {
        let per_sec = |cd: i32| -> String {
            let x10 = TICKS_PER_SECOND * 10 / cd.max(1);
            format!("{}.{}", x10 / 10, x10 % 10)
        };
        json!({
            "volley": gun::volley_damage(g),
            "rate": per_sec(g.cooldown),
            "mag": g.mag,
            "reload": tenths(g.reload),
            "muzzles": g.muzzles.len(),
            "kind": match g.kind {
                crate::content::ShotKind::Bullet => "bullet",
                crate::content::ShotKind::Rocket => "rocket",
                crate::content::ShotKind::Grenade => "grenade",
            },
            "effect": match g.effect {
                crate::content::Effect::None => "none",
                crate::content::Effect::Goo => "goo",
                crate::content::Effect::Shock => "shock",
                crate::content::Effect::Ember => "ember",
            },
            "splash": g.splash > 0,
            "auto": g.auto || g.gatling,
            "zoom": g.zoom_pct < 80,
            "scanner": g.scanner,
            "speed": pct(g.move_pct, 100),
        })
    }

    pub fn screen_json(&self) -> String {
        let missions: Vec<Value> = self
            .content
            .missions
            .iter()
            .map(|m| json!({ "id": m.id, "entrance": m.entrance, "pay": m.pay, "drones": m.drones }))
            .collect();
        let stash: Vec<Value> = (0..self.content.parts.len()).map(|k| self.part_json(k)).collect();
        let build: Vec<String> = self.build_ids(&self.build);
        let gun = gun::compose(&self.content, &self.build);
        let lobby = self.online.as_ref().map(|o| {
            json!({
                "role": if o.role == Role::Host { "host" } else { "joiner" },
                "room": o.room,
                "status": o.status,
                "seats": o.seats.iter().enumerate().map(|(i, s)| json!({
                    "kind": s.kind,
                    "seat": i + 1,
                    "you": (o.role == Role::Host && i == 0) || (o.role == Role::Joiner && i == o.you && !o.seats.is_empty()),
                    "colour": hex(self.c(&format!("jacket_{}", i % 4))),
                    "gun": s.build,
                })).collect::<Vec<_>>(),
                "mission": o.mission,
                "can_start": o.role == Role::Host && !o.started,
            })
        });
        let results = self.results.as_ref().map(|r| {
            json!({
                "success": r.success,
                "aborted": r.aborted,
                "you": self.me,
                "lives": r.lives.iter().map(|l| l.name()).collect::<Vec<_>>(),
                "kills": r.kills,
                "colours": (0..r.lives.len()).map(|i| hex(self.c(&format!("jacket_{}", i % 4)))).collect::<Vec<_>>(),
                "found": r.found.iter().map(|&k| self.content.parts[k].id.clone()).collect::<Vec<_>>(),
                "pay": r.pay.iter().map(|&k| self.content.parts[k].id.clone()).collect::<Vec<_>>(),
                "lost": r.lost.iter().map(|&k| self.content.parts[k].id.clone()).collect::<Vec<_>>(),
                "mission": self.setup.as_ref().map(|s| self.content.missions[s.mission].id.clone()),
            })
        });
        json!({
            "mode": self.mode.name(),
            "missions": missions,
            "pick": self.pick,
            "stash": stash,
            "build": build,
            "gun": self.gun_stats(&gun),
            "record": { "runs": self.save.runs, "extracted": self.save.extracted, "orders": self.save.orders },
            "sens": self.save.sens,
            "lobby": lobby,
            "results": results,
        })
        .to_string()
    }
}

pub fn numbers() -> String {
    let vertex_bytes = mesh::VERTEX_VALUES * render::BYTES_PER_VALUE;
    let instance_bytes = render::INSTANCE_VALUES * render::BYTES_PER_VALUE;
    let vec4 = 4 * render::BYTES_PER_VALUE;
    let v3b = 3 * render::BYTES_PER_VALUE;
    json!({
        "frac_bits": fx::FRAC_BITS,
        "ticks_per_second": TICKS_PER_SECOND,
        "actions": ACTIONS.iter().map(|(n, b)| (n.to_string(), json!(b))).collect::<serde_json::Map<_, _>>(),
        "meshes": Mesh::ALL.iter().map(|m| json!({
            "id": *m as u32,
            "vertices": mesh::vertices(*m).len() / mesh::VERTEX_VALUES,
        })).collect::<Vec<_>>(),
        "terrain_mesh": Mesh::Terrain as u32,
        "vertex": {
            "stride": vertex_bytes,
            "values": mesh::VERTEX_VALUES,
            "attributes": [["aPos", 3, 0], ["aNormal", 3, v3b], ["aTint", 3, 2 * v3b]],
        },
        "instance": {
            "stride": instance_bytes,
            "attributes": [["iA", 4, 0], ["iB", 4, vec4], ["iC", 4, 2 * vec4], ["iD", 4, 3 * vec4]],
        },
        "draw_values": render::DRAW_VALUES,
        "buffers": { "static": render::STATIC_BUFFER, "dynamic": render::DYNAMIC_BUFFER },
        "passes": { "solid": render::PASS_SOLID, "facade": render::PASS_FACADE, "ink": render::PASS_INK, "glow": render::PASS_GLOW },
        "uniforms": [["uLightDir", 3], ["uSky", 3], ["uFog", 3], ["uFogRange", 2], ["uLight", 3], ["uWindowDark", 3], ["uInk", 3], ["uSee", 4], ["uSeeSolid", 1]],
        "view": { "matrix": [0, 16], "eye": [16, 19], "focus": [19, 23] },
        "sens": { "min": 20, "max": 300, "step": 10 },
    })
    .to_string()
}

pub fn mesh_vertices(id: u32) -> Vec<i32> {
    Mesh::ALL
        .iter()
        .find(|m| **m as u32 == id)
        .map(|m| mesh::vertices(*m))
        .unwrap_or_default()
}

pub fn noise(n: u32, seed: u32) -> Vec<i32> {
    let mut r = Rng::new(seed as u64);
    (0..n.min(1 << 20)).map(|_| r.range(-ONE, ONE)).collect()
}

#[allow(dead_code)]
fn unused() -> u32 {
    B_FIRE | B_FWD
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> Game {
        Game::new(
            include_str!("../../../data/parts.json"),
            include_str!("../../../data/drones.json"),
            include_str!("../../../data/missions.json"),
            include_str!("../../../data/palette.json"),
            "",
        )
        .unwrap()
    }

    fn frames(g: &mut Game, n: i32, bits: u32) {
        for _ in 0..n {
            let t = g.last_us.unwrap_or(0) + TICK_US;
            g.advance(t, bits, 0, 0);
        }
    }

    #[test]
    fn a_new_player_has_starter_parts_and_can_build_a_gun_in_the_workshop() {
        let mut g = game();
        assert_eq!(g.save.stash.get("smg"), Some(&1));
        g.workshop();
        assert!(g.equip("smg"));
        assert_eq!(g.save.stash.get("smg"), None, "the one smg is on the gun");
        assert!(g.equip("pistol"), "the basic frame swaps back");
        assert_eq!(g.save.stash.get("smg"), Some(&1));
        assert!(!g.equip("heavy"), "a part not owned cannot go on");
        let saved = g.saved();
        let mut h = game();
        h.load_saved(&saved);
        assert_eq!(h.save, g.save);
    }

    #[test]
    fn a_sound_is_reported_once_and_not_every_frame_after() {
        let mut g = game();
        g.board();
        g.start_solo();
        let mut vans = 0;
        for _ in 0..120 {
            frames(&mut g, 1, 0);
            let hud: Value = serde_json::from_str(&g.hud_json()).unwrap();
            vans += hud["sounds"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|s| *s == "van")
                .count();
        }
        assert_eq!(vans, 1, "the van starts once, then is quiet");
        // a button's sound, made between frames, reaches the next frame's HUD once
        g.mode = Mode::Workshop;
        g.equip("smg");
        frames(&mut g, 1, 0);
        let hud: Value = serde_json::from_str(&g.hud_json()).unwrap();
        assert!(hud["sounds"].as_array().unwrap().iter().any(|s| s == "equip"));
        frames(&mut g, 1, 0);
        let hud: Value = serde_json::from_str(&g.hud_json()).unwrap();
        assert!(!hud["sounds"].as_array().unwrap().iter().any(|s| s == "equip"));
    }

    #[test]
    fn a_solo_run_starts_from_the_board_and_draws_a_scene() {
        let mut g = game();
        g.board();
        g.pick_mission(1);
        g.start_solo();
        assert_eq!(g.mode, Mode::Play);
        frames(&mut g, 10, 0);
        assert!(!g.instances().is_empty());
        let hud: Value = serde_json::from_str(&g.hud_json()).unwrap();
        assert_eq!(hud["mode"], "play");
        assert_eq!(hud["prompt"]["key"], "use_doors");
        assert!(!g.draws().is_empty());
    }

    #[test]
    fn dying_loses_the_staked_parts_and_extracting_keeps_the_finds() {
        let mut g = game();
        g.workshop();
        g.equip("smg");
        g.back();
        g.board();
        g.start_solo();
        if let Some(run) = &mut g.run {
            run.raiders[0].life = Life::Dead;
            run.over = Some(false);
        }
        frames(&mut g, RESULTS_AFTER + 5, 0);
        assert_eq!(g.mode, Mode::Results);
        let r = g.results.clone().unwrap();
        assert_eq!(r.lost.len(), 1);
        assert_eq!(g.content.parts[g.build[0]].id, "pistol", "the frame is gone");
        assert_eq!(g.save.stash.get("smg"), None);
    }

    #[test]
    fn host_and_joiner_meet_in_the_lobby_and_start_the_same_run() {
        let mut host = game();
        let mut join = game();
        host.board();
        host.pick_mission(3);
        host.host_online("b1", 5);
        join.join_online("b1", &host.room());
        // the transport says each can see the other
        host.net_peer(1);
        join.net_peer(9);
        let pump = |from: &mut Game, to: &mut Game, peer: u32| {
            let out: Vec<(u32, String)> = serde_json::from_str(&from.net_outbox()).unwrap();
            for (_, text) in out {
                to.net_message(peer, &text);
            }
        };
        pump(&mut join, &mut host, 1);
        pump(&mut host, &mut join, 9);
        assert_eq!(host.online.as_ref().unwrap().seats.len(), 2);
        assert_eq!(join.online.as_ref().unwrap().mission, 3);
        host.start_online();
        pump(&mut host, &mut join, 9);
        assert_eq!(join.mode, Mode::Play);
        assert_eq!(join.me, 1);
        for _ in 0..120 {
            let t = host.last_us.unwrap_or(0) + TICK_US;
            host.advance(t, B_FWD, 0, 0);
            join.advance(t, 0, 0, 0);
            pump(&mut join, &mut host, 1);
            pump(&mut host, &mut join, 9);
        }
        let (a, b) = (host.session.as_ref().unwrap(), join.session.as_ref().unwrap());
        assert!(a.current > 60 && b.current > 60);
        let t = a.confirmed().min(b.confirmed());
        assert!(t > 30, "inputs flowed both ways: {t}");
    }
}
