//! How long a tick of a busy run takes, and a run's snapshot (rollback
//! clones one each tick). Prints; asserts only a loose ceiling.
//!
//!     cargo test -p rm_core --release --test perf -- --nocapture

use rm_core::content::Content;
use rm_core::fx::{int, ONE};
use rm_core::geom::v3;
use rm_core::gun::{basic_build, compose};
use rm_core::raider::{Input, B_FIRE, B_FWD, B_LEFT};
use rm_core::run::{Ref, Run, Setup};
use std::time::Instant;

#[test]
fn a_busy_tick_is_cheap_enough_to_roll_back() {
    let c = Content::parse(
        include_str!("../../../data/parts.json"),
        include_str!("../../../data/drones.json"),
        include_str!("../../../data/missions.json"),
        include_str!("../../../data/palette.json"),
    )
    .unwrap();
    let t0 = Instant::now();
    let map = rm_core::mall::build(&c.palette);
    println!(
        "map: {} solids, {} nodes, built in {:?}",
        map.solids.len(),
        map.nodes.len(),
        t0.elapsed()
    );
    let b = basic_build(&c);
    let s = Setup {
        mission: 4,
        builds: vec![b; 4],
        guns: vec![compose(&c, &b); 4],
        seed: 1,
    };
    let r = Ref {
        content: &c,
        map: &map,
        setup: &s,
    };
    let mut run = Run::new(&r);
    // drop the raiders into the concourse among the drones
    for (i, ra) in run.raiders.iter_mut().enumerate() {
        ra.pos = v3(int(i as i32 * 2 - 3), 0, int(30));
    }
    let n = 1800;
    let t = Instant::now();
    let mut clone_time = std::time::Duration::ZERO;
    for k in 0..n {
        let inputs: Vec<Input> = (0..4)
            .map(|i| Input {
                b: if (k / 60 + i) % 3 == 0 {
                    B_FIRE
                } else {
                    B_FWD | B_LEFT | B_FIRE
                },
                y: k * 300 + i * 9000,
                p: -ONE / 30,
            })
            .collect();
        let tc = Instant::now();
        let _snap = run.clone();
        clone_time += tc.elapsed();
        run.step(&r, &inputs);
    }
    let per = t.elapsed() / n as u32;
    println!(
        "{} drones, {} active; per tick {:?} (clone {:?})",
        run.drones.len(),
        run.drones.iter().filter(|d| d.active()).count(),
        per,
        clone_time / n as u32
    );
    assert!(per.as_micros() < 4000, "{per:?}");
}

#[test]
fn drones_find_and_fight_a_raider_standing_in_the_concourse() {
    let c = Content::parse(
        include_str!("../../../data/parts.json"),
        include_str!("../../../data/drones.json"),
        include_str!("../../../data/missions.json"),
        include_str!("../../../data/palette.json"),
    )
    .unwrap();
    let map = rm_core::mall::build(&c.palette);
    let b = basic_build(&c);
    let s = Setup {
        mission: 0,
        builds: vec![b; 1],
        guns: vec![compose(&c, &b); 1],
        seed: 3,
    };
    let r = Ref {
        content: &c,
        map: &map,
        setup: &s,
    };
    let mut run = Run::new(&r);
    run.raiders[0].pos = v3(0, 0, int(40));
    let mut shots = 0;
    let mut doing = std::collections::BTreeMap::new();
    for _ in 0..1800 {
        run.step(&r, &[Input::default()]);
        for e in &run.events {
            if matches!(
                e,
                rm_core::run::Event::DroneFire { .. } | rm_core::run::Event::Laser { .. }
            ) {
                shots += 1;
            }
        }
        for d in &run.drones {
            if d.brain.target.is_some() {
                *doing
                    .entry(format!("{:?}/{:?}", d.kind, d.brain.doing))
                    .or_insert(0) += 1;
            }
        }
    }
    println!(
        "drone shots {shots}, raider hp {} life {:?}",
        run.raiders[0].hp, run.raiders[0].life
    );
    println!("{doing:?}");
    assert!(shots > 5);
}
