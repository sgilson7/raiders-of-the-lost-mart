//! The game's content, read from the data files the page hands over:
//! weapon parts, drone kinds, missions and the palette. Parsing is the only
//! thing done here; every rule that uses these values lives elsewhere.

use crate::fx::ONE;
use serde::Deserialize;

/// A colour, each channel in Q16 (`ONE` is full).
pub type Rgb = [i32; 3];

/// `#RRGGBB` to a colour; anything else is ink.
pub fn hex(s: &str) -> Rgb {
    let t = s.trim_start_matches('#');
    if t.len() != 6 {
        return [0; 3];
    }
    let ch = |i: usize| i32::from_str_radix(&t[i..i + 2], 16).unwrap_or(0) * ONE / 255;
    [ch(0), ch(2), ch(4)]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Cat {
    Frame,
    Barrel,
    Mount,
    Payload,
    Sight,
    Grip,
}

impl Cat {
    pub const ALL: [Cat; 6] = [
        Cat::Frame,
        Cat::Barrel,
        Cat::Mount,
        Cat::Payload,
        Cat::Sight,
        Cat::Grip,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        match self {
            Cat::Frame => "frame",
            Cat::Barrel => "barrel",
            Cat::Mount => "mount",
            Cat::Payload => "payload",
            Cat::Sight => "sight",
            Cat::Grip => "grip",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShotKind {
    #[default]
    Bullet,
    Rocket,
    Grenade,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Effect {
    #[default]
    None,
    Goo,
    Shock,
    Ember,
}

#[derive(Debug, Clone, Deserialize)]
struct RawPart {
    id: String,
    cat: Cat,
    #[serde(default)]
    basic: bool,
    #[serde(default)]
    auto: bool,
    #[serde(default)]
    cooldown: i32,
    #[serde(default)]
    mag: i32,
    #[serde(default)]
    reload: i32,
    #[serde(default)]
    dmg_pct: Option<i32>,
    #[serde(default)]
    recoil: i32,
    #[serde(default)]
    spread: i32,
    #[serde(default)]
    move_pct: Option<i32>,
    #[serde(default)]
    len: i32,
    #[serde(default)]
    height: i32,
    #[serde(default)]
    width: i32,
    colour: String,
    #[serde(default)]
    trim: Option<String>,
    #[serde(default)]
    kind: ShotKind,
    #[serde(default)]
    bores: i32,
    #[serde(default)]
    pellets: i32,
    #[serde(default)]
    damage: i32,
    #[serde(default)]
    speed: i32,
    #[serde(default)]
    min_cd: i32,
    #[serde(default)]
    pierce: i32,
    #[serde(default)]
    splash: i32,
    #[serde(default)]
    slots: Vec<(i32, i32)>,
    #[serde(default)]
    gatling: bool,
    #[serde(default)]
    effect: Effect,
    #[serde(default)]
    power: i32,
    #[serde(default)]
    zoom_pct: i32,
    #[serde(default)]
    aim_spread_pct: i32,
    #[serde(default)]
    hip_spread_pct: Option<i32>,
    #[serde(default)]
    scanner: bool,
    #[serde(default)]
    recoil_pct: Option<i32>,
    #[serde(default)]
    spread_pct: Option<i32>,
    #[serde(default)]
    rarity: i32,
    #[serde(default)]
    ammo: String,
}

/// One weapon part. Fields a category does not use are zero (or 100 for a
/// percentage), so stats can be combined without asking which part set them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub id: String,
    pub cat: Cat,
    pub basic: bool,
    pub auto: bool,
    pub cooldown: i32,
    pub mag: i32,
    pub reload: i32,
    pub dmg_pct: i32,
    pub recoil: i32,
    pub spread: i32,
    pub move_pct: i32,
    pub len: i32,
    pub height: i32,
    pub width: i32,
    pub colour: Rgb,
    pub trim: Rgb,
    pub kind: ShotKind,
    pub bores: i32,
    pub pellets: i32,
    pub damage: i32,
    pub speed: i32,
    pub min_cd: i32,
    pub pierce: i32,
    pub splash: i32,
    pub slots: Vec<(i32, i32)>,
    pub gatling: bool,
    pub effect: Effect,
    pub power: i32,
    pub zoom_pct: i32,
    pub aim_spread_pct: i32,
    pub hip_spread_pct: i32,
    pub scanner: bool,
    pub recoil_pct: i32,
    pub spread_pct: i32,
    pub rarity: i32,
    /// a barrel's ammunition, as an index into `Content::ammo`
    pub ammo: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DroneDef {
    pub id: String,
    pub walks: bool,
    pub hp: i32,
    pub armour_pct: i32,
    pub weak_pct: i32,
    /// tenths of a metre a second
    pub speed: i32,
    pub accel: i32,
    pub sight: i32,
    pub range: i32,
    pub damage: i32,
    pub shot_speed: i32,
    pub burst: i32,
    pub burst_gap: i32,
    pub reload: i32,
    #[serde(default)]
    pub charge: i32,
    #[serde(default)]
    pub fuse: i32,
    #[serde(default)]
    pub blast: i32,
    /// centimetres
    pub radius: i32,
    #[serde(default)]
    pub rotor_hp: i32,
    pub colour: String,
    pub dark: String,
    pub loot: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Goal {
    Fetch,
    Clear,
    Hive,
    Power,
    /// bring down the street's queen
    Queen,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MissionDef {
    pub id: String,
    pub entrance: String,
    pub goal: Goal,
    #[serde(default)]
    pub item: String,
    #[serde(default)]
    pub count: i32,
    #[serde(default)]
    pub spots: Vec<String>,
    #[serde(default)]
    pub zone: String,
    pub drones: i32,
    #[serde(default)]
    pub nest: i32,
    #[serde(default)]
    pub hold: i32,
    #[serde(default)]
    pub waves: i32,
    pub pay: i32,
}

/// Every colour the 3D scene uses, named in data/palette.json.
#[derive(Debug, Clone)]
pub struct Palette {
    pub names: Vec<(String, Rgb)>,
    /// the light and fog: colours by name, distances in metres
    pub scene: serde_json::Value,
}

impl Palette {
    pub fn scene_rgb(&self, key: &str) -> Rgb {
        hex(self.scene.get(key).and_then(|v| v.as_str()).unwrap_or(""))
    }

    pub fn scene_num(&self, key: &str) -> i32 {
        self.scene.get(key).and_then(|v| v.as_i64()).unwrap_or(0) as i32
    }

    pub fn get(&self, name: &str) -> Rgb {
        self.names
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, c)| *c)
            .unwrap_or([ONE, 0, ONE])
    }
}

/// The ten places a raider wears or carries something besides the gun.
pub const SLOTS: [&str; 10] = [
    "head",
    "shoulders",
    "chest",
    "coat",
    "hands",
    "legs",
    "knees",
    "feet",
    "belt",
    "melee",
];
pub const SLOT_BELT: usize = 8;
pub const SLOT_MELEE: usize = 9;

/// A piece of gear: what it is and what it does. Fields a piece does not
/// set are neutral (0 added, 100 percent, false).
#[derive(Debug, Clone, Deserialize)]
pub struct GearDef {
    pub id: String,
    pub slot: String,
    #[serde(default)]
    pub basic: bool,
    #[serde(default)]
    pub rarity: i32,
    #[serde(default)]
    pub colour: String,
    #[serde(default)]
    pub hp: i32,
    #[serde(default)]
    pub armour_pct: i32,
    #[serde(default)]
    pub regen_to: i32,
    #[serde(default = "hundred")]
    pub regen_pct: i32,
    #[serde(default = "hundred")]
    pub speed_pct: i32,
    #[serde(default = "hundred")]
    pub sprint_pct: i32,
    #[serde(default = "hundred")]
    pub jump_pct: i32,
    #[serde(default)]
    pub slots: i32,
    #[serde(default = "hundred")]
    pub notice_pct: i32,
    #[serde(default = "hundred")]
    pub noise_pct: i32,
    #[serde(default = "hundred")]
    pub reload_pct: i32,
    #[serde(default = "hundred")]
    pub search_pct: i32,
    #[serde(default = "hundred")]
    pub revive_pct: i32,
    #[serde(default)]
    pub revive_hp: i32,
    #[serde(default = "hundred")]
    pub recoil_pct: i32,
    #[serde(default = "hundred")]
    pub melee_pct: i32,
    #[serde(default = "hundred")]
    pub blast_pct: i32,
    #[serde(default)]
    pub crouch_full: bool,
    #[serde(default)]
    pub slide: bool,
    #[serde(default)]
    pub roll: bool,
    #[serde(default)]
    pub headset: bool,
    #[serde(default)]
    pub goo_immune: bool,
    #[serde(default)]
    pub belt: String,
    #[serde(default)]
    pub charges: i32,
    // melee
    #[serde(default)]
    pub damage: i32,
    #[serde(default)]
    pub cooldown: i32,
    #[serde(default)]
    pub reach: i32,
    #[serde(default)]
    pub arc: i32,
    #[serde(default)]
    pub rips: i32,
    #[serde(default)]
    pub knock: i32,
}

fn hundred() -> i32 {
    100
}

impl GearDef {
    pub fn slot_index(&self) -> usize {
        SLOTS.iter().position(|s| *s == self.slot).unwrap_or(0)
    }
}

/// A kind of ammunition.
#[derive(Debug, Clone, Deserialize)]
pub struct AmmoDef {
    pub id: String,
    /// the most a raider carries in
    pub carry: i32,
    /// rounds to a pack slot
    pub stack: i32,
    /// what a new stash starts with
    pub start: i32,
    /// topped up to this before every run, free
    pub free: i32,
    pub rarity: i32,
}

#[derive(Debug, Clone)]
pub struct Content {
    pub gear: Vec<GearDef>,
    pub ammo: Vec<AmmoDef>,
    pub parts: Vec<Part>,
    pub drones: Vec<DroneDef>,
    pub missions: Vec<MissionDef>,
    pub palette: Palette,
}

#[derive(Deserialize)]
struct PartsFile {
    parts: Vec<RawPart>,
}

#[derive(Deserialize)]
struct GearFile {
    gear: Vec<GearDef>,
    ammo: Vec<AmmoDef>,
}

#[derive(Deserialize)]
struct DronesFile {
    drones: Vec<DroneDef>,
}

#[derive(Deserialize)]
struct MissionsFile {
    missions: Vec<MissionDef>,
}

fn part_from(r: RawPart) -> Part {
    Part {
        id: r.id,
        cat: r.cat,
        basic: r.basic,
        auto: r.auto,
        cooldown: r.cooldown,
        mag: r.mag,
        reload: r.reload,
        dmg_pct: r.dmg_pct.unwrap_or(100),
        recoil: r.recoil,
        spread: r.spread,
        move_pct: r.move_pct.unwrap_or(100),
        len: r.len,
        height: r.height,
        width: r.width,
        colour: hex(&r.colour),
        trim: hex(r.trim.as_deref().unwrap_or(&r.colour)),
        kind: r.kind,
        bores: r.bores.max(1),
        pellets: r.pellets.max(1),
        damage: r.damage,
        speed: r.speed,
        min_cd: r.min_cd,
        pierce: r.pierce,
        splash: r.splash,
        slots: if r.slots.is_empty() { vec![(0, 0)] } else { r.slots },
        gatling: r.gatling,
        effect: r.effect,
        power: r.power,
        zoom_pct: if r.zoom_pct == 0 { 100 } else { r.zoom_pct },
        aim_spread_pct: if r.aim_spread_pct == 0 {
            100
        } else {
            r.aim_spread_pct
        },
        hip_spread_pct: r.hip_spread_pct.unwrap_or(100),
        scanner: r.scanner,
        recoil_pct: r.recoil_pct.unwrap_or(100),
        spread_pct: r.spread_pct.unwrap_or(100),
        rarity: r.rarity,
        ammo: 0,
    }
}

impl Content {
    pub fn parse(
        parts: &str,
        drones: &str,
        missions: &str,
        palette: &str,
        gear: &str,
    ) -> Result<Content, String> {
        let p: PartsFile = serde_json::from_str(parts).map_err(|e| format!("parts.json: {e}"))?;
        let g: GearFile = serde_json::from_str(gear).map_err(|e| format!("gear.json: {e}"))?;
        let d: DronesFile = serde_json::from_str(drones).map_err(|e| format!("drones.json: {e}"))?;
        let m: MissionsFile = serde_json::from_str(missions).map_err(|e| format!("missions.json: {e}"))?;
        let pal: serde_json::Value =
            serde_json::from_str(palette).map_err(|e| format!("palette.json: {e}"))?;
        let mut names = Vec::new();
        if let Some(o) = pal.get("colours").and_then(|v| v.as_object()) {
            for (k, v) in o {
                names.push((k.clone(), hex(v.as_str().unwrap_or(""))));
            }
        }
        let ammo_of = |id: &str| g.ammo.iter().position(|a| a.id == id).unwrap_or(0);
        let c = Content {
            parts: p
                .parts
                .into_iter()
                .map(|r| {
                    let a = ammo_of(&r.ammo);
                    Part {
                        ammo: a,
                        ..part_from(r)
                    }
                })
                .collect(),
            gear: g.gear,
            ammo: g.ammo,
            drones: d.drones,
            missions: m.missions,
            palette: Palette {
                names,
                scene: pal.get("scene").cloned().unwrap_or_default(),
            },
        };
        for cat in Cat::ALL {
            if !c.parts.iter().any(|p| p.cat == cat && p.basic) {
                return Err(format!("parts.json: no basic {}", cat.name()));
            }
        }
        for slot in SLOTS {
            if !c.gear.iter().any(|g| g.slot == slot && g.basic) {
                return Err(format!("gear.json: no basic {slot}"));
            }
        }
        Ok(c)
    }

    pub fn part(&self, id: &str) -> Option<usize> {
        self.parts.iter().position(|p| p.id == id)
    }

    pub fn drone(&self, id: &str) -> usize {
        self.drones.iter().position(|d| d.id == id).unwrap_or(0)
    }

    pub fn gear_id(&self, id: &str) -> Option<usize> {
        self.gear.iter().position(|g| g.id == id)
    }

    /// The basic piece for a gear slot.
    pub fn basic_gear(&self, slot: usize) -> usize {
        self.gear
            .iter()
            .position(|g| g.slot == SLOTS[slot] && g.basic)
            .unwrap_or(0)
    }

    pub fn ammo_id(&self, id: &str) -> Option<usize> {
        self.ammo.iter().position(|a| a.id == id)
    }

    /// The basic part of a category: never used up.
    pub fn basic(&self, cat: Cat) -> usize {
        self.parts
            .iter()
            .position(|p| p.cat == cat && p.basic)
            .unwrap_or(0)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub fn content() -> Content {
        Content::parse(
            include_str!("../../../data/parts.json"),
            include_str!("../../../data/drones.json"),
            include_str!("../../../data/missions.json"),
            include_str!("../../../data/palette.json"),
            include_str!("../../../data/gear.json"),
        )
        .unwrap()
    }

    #[test]
    fn the_data_files_parse_with_five_parts_in_each_of_six_categories() {
        let c = content();
        for cat in Cat::ALL {
            assert_eq!(
                c.parts.iter().filter(|p| p.cat == cat).count(),
                5,
                "{}",
                cat.name()
            );
        }
        // four flyers and two spiders, as the brief asks, and the queen besides
        let common: Vec<&DroneDef> = c.drones.iter().filter(|d| d.id != "queen").collect();
        assert_eq!(common.len(), 6);
        assert_eq!(common.iter().filter(|d| d.walks).count(), 2);
        assert!(c.drones.iter().any(|d| d.id == "queen" && d.walks));
        assert_eq!(c.missions.len(), 6);
        for slot in SLOTS {
            assert!(c.gear.iter().filter(|g| g.slot == slot).count() >= 3, "{slot}");
        }
        assert_eq!(c.ammo.len(), 5);
        let rocket = &c.parts[c.part("rocket").unwrap()];
        assert_eq!(c.ammo[rocket.ammo].id, "rockets");
        assert_eq!(hex("#FF0000"), [ONE, 0, 0]);
    }
}
