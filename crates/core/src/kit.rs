//! What a raider wears and carries besides the gun: ten slots of gear
//! (head to feet, a belt gadget, a melee weapon), folded into one `Kit` of
//! numbers the run reads. A slot's basic piece is neutral and never lost.

use crate::content::{Content, SLOTS, SLOT_BELT, SLOT_MELEE};

/// One piece per slot, as indices into `Content::gear`, in `SLOTS` order.
pub type Wear = [usize; 10];

pub fn basic_wear(c: &Content) -> Wear {
    let mut w = [0; 10];
    for (i, slot) in w.iter_mut().enumerate() {
        *slot = c.basic_gear(i);
    }
    w
}

pub fn valid_wear(c: &Content, w: &Wear) -> bool {
    w.iter()
        .enumerate()
        .all(|(i, &g)| c.gear.get(g).is_some_and(|d| d.slot == SLOTS[i]))
}

/// A belt gadget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gadget {
    /// a proximity mine left where you stood
    Mine,
    /// a thrown noise-maker: drones go to check it
    Decoy,
    /// a painted strip: raiders run faster on it, drones are slowed
    Paint,
    /// a thrown smoke pellet: drones cannot see through the cloud
    Smoke,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Melee {
    pub damage: i32,
    pub cooldown: i32,
    /// cm
    pub reach: i32,
    /// degrees, the whole sweep
    pub arc: i32,
    /// percent of the damage a part it hits takes
    pub rips: i32,
    pub knock: i32,
}

/// Everything the gear adds up to. Percentages are of normal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kit {
    pub max_hp: i32,
    /// percent of damage taken away
    pub armour_pct: i32,
    pub regen_to: i32,
    pub regen_pct: i32,
    pub speed_pct: i32,
    pub sprint_pct: i32,
    pub jump_pct: i32,
    pub slots: i32,
    pub notice_pct: i32,
    pub noise_pct: i32,
    pub reload_pct: i32,
    pub search_pct: i32,
    pub revive_pct: i32,
    pub revive_hp: i32,
    pub recoil_pct: i32,
    pub melee_pct: i32,
    pub blast_pct: i32,
    pub crouch_full: bool,
    pub slide: bool,
    pub roll: bool,
    pub headset: bool,
    pub goo_immune: bool,
    pub gadget: Option<Gadget>,
    pub charges: i32,
    pub melee: Melee,
}

/// What the gear changes about moving, which raider.rs reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Moves {
    pub speed_pct: i32,
    pub sprint_pct: i32,
    pub jump_pct: i32,
    pub crouch_full: bool,
    pub slide: bool,
    pub roll: bool,
}

pub const NEUTRAL: Moves = Moves {
    speed_pct: 100,
    sprint_pct: 100,
    jump_pct: 100,
    crouch_full: false,
    slide: false,
    roll: false,
};

impl Kit {
    pub fn moves(&self) -> Moves {
        Moves {
            speed_pct: self.speed_pct,
            sprint_pct: self.sprint_pct,
            jump_pct: self.jump_pct,
            crouch_full: self.crouch_full,
            slide: self.slide,
            roll: self.roll,
        }
    }
}

/// Folds the worn pieces into a kit: added things add, percentages multiply.
pub fn kit(c: &Content, w: &Wear) -> Kit {
    let mut k = Kit {
        max_hp: crate::balance::RAIDER_HP,
        armour_pct: 0,
        regen_to: crate::balance::REGEN_TO,
        regen_pct: 100,
        speed_pct: 100,
        sprint_pct: 100,
        jump_pct: 100,
        slots: crate::balance::BACKPACK as i32,
        notice_pct: 100,
        noise_pct: 100,
        reload_pct: 100,
        search_pct: 100,
        revive_pct: 100,
        revive_hp: crate::balance::REVIVE_HP,
        recoil_pct: 100,
        melee_pct: 100,
        blast_pct: 100,
        crouch_full: false,
        slide: false,
        roll: false,
        headset: false,
        goo_immune: false,
        gadget: None,
        charges: 0,
        melee: Melee {
            damage: 20,
            cooldown: 34,
            reach: 220,
            arc: 70,
            rips: 150,
            knock: 4,
        },
    };
    let pct = |a: i32, b: i32| a * b / 100;
    for (i, &g) in w.iter().enumerate() {
        let Some(d) = c.gear.get(g) else { continue };
        k.max_hp += d.hp;
        // armour stacks with diminishing returns: each piece takes its share of what is left
        k.armour_pct = 100 - (100 - k.armour_pct) * (100 - d.armour_pct) / 100;
        k.regen_to += d.regen_to;
        k.regen_pct = pct(k.regen_pct, d.regen_pct);
        k.speed_pct = pct(k.speed_pct, d.speed_pct);
        k.sprint_pct = pct(k.sprint_pct, d.sprint_pct);
        k.jump_pct = pct(k.jump_pct, d.jump_pct);
        k.slots += d.slots;
        k.notice_pct = pct(k.notice_pct, d.notice_pct);
        k.noise_pct = pct(k.noise_pct, d.noise_pct);
        k.reload_pct = pct(k.reload_pct, d.reload_pct);
        k.search_pct = pct(k.search_pct, d.search_pct);
        k.revive_pct = pct(k.revive_pct, d.revive_pct);
        k.revive_hp = k.revive_hp.max(d.revive_hp);
        k.recoil_pct = pct(k.recoil_pct, d.recoil_pct);
        k.melee_pct = pct(k.melee_pct, d.melee_pct);
        k.blast_pct = pct(k.blast_pct, d.blast_pct);
        k.crouch_full |= d.crouch_full;
        k.slide |= d.slide;
        k.roll |= d.roll;
        k.headset |= d.headset;
        k.goo_immune |= d.goo_immune;
        if i == SLOT_BELT {
            k.gadget = match d.belt.as_str() {
                "mine" => Some(Gadget::Mine),
                "decoy" => Some(Gadget::Decoy),
                "paint" => Some(Gadget::Paint),
                "smoke" => Some(Gadget::Smoke),
                _ => None,
            };
            k.charges = d.charges;
        }
        if i == SLOT_MELEE {
            k.melee = Melee {
                damage: d.damage,
                cooldown: d.cooldown,
                reach: d.reach,
                arc: d.arc,
                rips: d.rips,
                knock: d.knock,
            };
        }
    }
    k.regen_to = k.regen_to.min(k.max_hp);
    k.melee.damage = pct(k.melee.damage, k.melee_pct);
    k
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::content;

    fn wear(c: &Content, ids: &[&str]) -> Wear {
        let mut w = basic_wear(c);
        for id in ids {
            let g = c.gear_id(id).unwrap();
            w[c.gear[g].slot_index()] = g;
        }
        assert!(valid_wear(c, &w));
        w
    }

    #[test]
    fn basic_gear_is_neutral_and_a_bat_is_always_carried() {
        let c = content();
        let k = kit(&c, &basic_wear(&c));
        assert_eq!(k.max_hp, crate::balance::RAIDER_HP);
        assert_eq!((k.speed_pct, k.notice_pct, k.armour_pct), (100, 100, 0));
        assert_eq!(k.gadget, None);
        assert!(k.melee.damage > 0, "the bat");
    }

    #[test]
    fn pieces_add_up_and_armour_diminishes() {
        let c = content();
        let k = kit(
            &c,
            &wear(
                &c,
                &[
                    "vest", "helmet", "poncho", "runners", "mines", "sledge", "cargo", "straps",
                ],
            ),
        );
        assert_eq!(k.max_hp, crate::balance::RAIDER_HP + 35 + 10);
        assert!(k.armour_pct > 12 && k.armour_pct < 22, "{}", k.armour_pct);
        assert!(k.notice_pct < 60);
        assert!(k.roll);
        assert_eq!((k.gadget, k.charges), (Some(Gadget::Mine), 3));
        assert_eq!(k.slots, crate::balance::BACKPACK as i32 + 10);
        assert!(k.melee.damage > 50, "a sledge");
    }
}
