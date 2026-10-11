//! Feel numbers in one place. Distances in Q16 metres, times in ticks.

use crate::fx::{deg, int, ratio, ONE};

pub const TICKS_PER_SECOND: i32 = 60;
pub const TICK_US: i64 = 1_000_000 / TICKS_PER_SECOND as i64;
/// After a stall (a hidden tab), the clock runs at most this far behind.
pub const MAX_CATCH_UP_US: i64 = 250_000;

pub const fn secs(ms: i32) -> i32 {
    ms * TICKS_PER_SECOND / 1000
}

// ---- raiders ---------------------------------------------------------------
pub const RAIDER_HALF: i32 = ONE * 35 / 100;
pub const RAIDER_H: i32 = ONE * 18 / 10;
pub const WALK: i32 = ratio(42, 10);
pub const SPRINT: i32 = ratio(68, 10);
pub const AIM_WALK: i32 = ratio(26, 10);
pub const CRAWL: i32 = ratio(8, 10);
pub const ACCEL: i32 = ratio(40, 1);
pub const JUMP: i32 = ratio(52, 10);
pub const GRAVITY: i32 = int(18);
pub const STEP_UP: i32 = ONE * 45 / 100;
pub const RAIDER_HP: i32 = 100;
/// a raider's health regrows this long after the last hurt, a point every `REGEN_EVERY`
pub const REGEN_AFTER: i32 = secs(6000);
pub const REGEN_EVERY: i32 = secs(250);
pub const REGEN_TO: i32 = 60;
pub const BLEED_OUT: i32 = secs(30_000);
pub const REVIVE: i32 = secs(3000);
pub const REVIVE_HP: i32 = 35;
pub const REACH: i32 = ONE * 22 / 10;
pub const SEARCH: i32 = secs(800);
pub const BACKPACK: usize = 12;
pub const MAX_PITCH: i32 = deg(70);

// ---- the camera --------------------------------------------------------------
pub const CAM_UP: i32 = ONE * 165 / 100;
pub const CAM_RIGHT: i32 = ONE * 55 / 100;
pub const CAM_BACK: i32 = ONE * 30 / 10;
pub const CAM_BACK_AIM: i32 = ONE * 13 / 10;
pub const FOV: i32 = deg(70);
pub const AIM_RANGE: i32 = int(220);

// ---- shots -----------------------------------------------------------------------
pub const SHOT_LIFE: i32 = secs(2500);
pub const GRENADE_FUSE: i32 = secs(1600);
pub const ROCKET_LIFE: i32 = secs(5000);
pub const SHOT_GRAVITY: i32 = int(14);
/// a shot's noise wakes drones this far off
pub const NOISE: i32 = int(40);
pub const RELOAD_GATLING_SPIN: i32 = secs(400);

// ---- drones --------------------------------------------------------------------------
pub const THINK_EVERY: u32 = 4;
pub const LOOK_EVERY: u32 = 8;
pub const HOVER_LOW: i32 = ONE * 25 / 10;
pub const HOVER_HIGH: i32 = int(5);
pub const OUTDOOR_HOVER: i32 = int(9);
/// a drone forgets a raider it has not seen for this long
pub const FORGET: i32 = secs(9000);
pub const ORBIT_RADIUS: i32 = int(11);
pub const DRONE_SHOT_SPREAD: i32 = deg(3);
pub const WAKE_RANGE: i32 = int(110);
pub const FALL_TIME: i32 = secs(2500);
pub const GOO_SLOW_PCT: i32 = 45;
pub const STOMP_RANGE: i32 = int(4);
pub const STOMP_DAMAGE: i32 = 22;
pub const HIVE_HP: i32 = 900;
pub const HIVE_POD_HP: i32 = 120;
pub const HIVE_SPAWN_EVERY: i32 = secs(14_000);
pub const HIVE_SPAWN_MAX: usize = 5;

// ---- the van and the run ----------------------------------------------------------------
pub const VAN_LEAVE: i32 = secs(10_000);
pub const VAN_DOORS_SELF_OPEN: i32 = secs(12_000);
pub const GENERATOR_START: i32 = secs(4000);
pub const WAVE_EVERY: i32 = secs(15_000);
pub const DRONE_DROP_PCT: i32 = 45;
