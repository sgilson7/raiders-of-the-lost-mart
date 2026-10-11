//! Online play: rollback netcode over Ribbit King's transport.
//!
//! Ribbit King ran a host's tick stream: the host decided every tick and
//! joiners played it back, which suits golf, where one frog moves at a
//! time. A shooter has four people moving at once, and waiting a round trip
//! for the host would put that delay on every step and every shot. So here
//! every browser runs the whole run itself, GGPO-style:
//!
//! * each raider's own input is used at once on their own machine (after a
//!   small fixed delay of `DELAY` ticks, which hides most of the trip) and
//!   sent to everybody;
//! * for the other raiders, the newest input heard is assumed to continue;
//! * when a real input arrives that differs from what was assumed for an
//!   earlier tick, the run is put back to the snapshot before that tick and
//!   the ticks since are run again with the true inputs ("rollback");
//! * a machine never runs more than `MAX_AHEAD` ticks past the last tick it
//!   has every input for; if it gets there it waits a frame.
//!
//! The run is deterministic integer maths (run.rs), so the copies agree;
//! every `SUM_EVERY` ticks each copy hashes a confirmed tick and the host
//! compares. The transport is still the star Ribbit King used (every joiner
//! talks to the host, which passes each joiner's inputs on to the others),
//! so a joiner's input reaches another joiner in two hops.

use crate::raider::{Input, B_GONE};
use crate::run::{Event, Ref, Run};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

pub const PROTO: u32 = 1;
/// Ticks between a local input and the tick it is used on.
pub const DELAY: u32 = 2;
/// The furthest a copy runs ahead of its confirmed ticks.
pub const MAX_AHEAD: u32 = 14;
pub const SUM_EVERY: u32 = 120;

/// A seat in the lobby.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Seat {
    /// host, remote, or open
    pub kind: String,
    pub name: String,
    /// the seat's gun, as part ids in category order
    pub build: Vec<String>,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(tag = "t")]
pub enum Msg {
    /// Joiner to host, first: who I am and what I carry.
    Hello {
        proto: u32,
        build: String,
        gun: Vec<String>,
    },
    /// Host to joiner: the lobby, the order chosen, and my seat.
    Lobby {
        seats: Vec<Seat>,
        mission: usize,
        you: usize,
    },
    /// Joiner to host: my gun changed.
    Gun {
        gun: Vec<String>,
    },
    /// Host to everyone: the run begins.
    Start {
        mission: usize,
        seed: u64,
        guns: Vec<Vec<String>>,
        you: usize,
    },
    /// Inputs of player `p` from tick `from` on; `at` is the sender's
    /// current tick, for keeping pace.
    In {
        p: usize,
        from: u32,
        i: Vec<Input>,
        at: u32,
    },
    /// Host to everyone: player `p` left; their inputs from tick `from` are gone.
    Left {
        p: usize,
        from: u32,
    },
    Sum {
        tick: u32,
        sum: u64,
    },
    Desync {
        tick: u32,
    },
    Bye {
        why: String,
    },
}

pub fn encode(m: &Msg) -> String {
    serde_json::to_string(m).unwrap_or_default()
}

pub fn decode(text: &str) -> Option<Msg> {
    serde_json::from_str(text).ok()
}

/// The rollback session for one run, on one machine.
#[derive(Debug, Clone)]
pub struct Session {
    pub players: usize,
    pub me: usize,
    /// the next tick to run
    pub current: u32,
    /// inputs heard (or made) per player, by tick
    known: Vec<BTreeMap<u32, Input>>,
    /// per player: every tick below this is known
    pub upto: Vec<u32>,
    pub gone_from: Vec<Option<u32>>,
    /// the inputs each run tick used, while it may yet be wrong
    used: VecDeque<(u32, Vec<Input>)>,
    /// the run before each tick that may need running again
    snaps: VecDeque<(u32, Run)>,
    pub rollback_to: Option<u32>,
    /// the furthest tick ever run: events of ticks run again are not repeated
    pub shown: u32,
    /// local inputs not yet sent, from this tick
    pub unsent: Vec<Input>,
    pub unsent_from: u32,
    /// what each other machine last said its current tick was
    pub their_at: Vec<u32>,
    pub rollbacks: u32,
    pub rolled_ticks: u32,
    pub stalls: u32,
    pub sums: BTreeMap<u32, u64>,
}

impl Session {
    pub fn new(players: usize, me: usize) -> Session {
        let mut known = vec![BTreeMap::new(); players];
        // the first ticks have no input from anybody: the same on every copy
        for k in known.iter_mut() {
            for t in 0..DELAY {
                k.insert(t, Input::default());
            }
        }
        Session {
            players,
            me,
            current: 0,
            known,
            upto: vec![DELAY; players],
            gone_from: vec![None; players],
            used: VecDeque::new(),
            snaps: VecDeque::new(),
            rollback_to: None,
            shown: 0,
            unsent: Vec::new(),
            unsent_from: DELAY,
            their_at: vec![0; players],
            rollbacks: 0,
            rolled_ticks: 0,
            stalls: 0,
            sums: BTreeMap::new(),
        }
    }

    /// Every tick below this has every player's true input.
    pub fn confirmed(&self) -> u32 {
        (0..self.players)
            .map(|p| match self.gone_from[p] {
                Some(g) if g <= self.upto[p] => u32::MAX,
                _ => self.upto[p],
            })
            .min()
            .unwrap_or(0)
    }

    /// The input player `p` gives on tick `t`, as well as it is known.
    pub fn input_for(&self, p: usize, t: u32) -> Input {
        if self.gone_from[p].is_some_and(|g| t >= g) {
            return Input {
                b: B_GONE,
                ..Input::default()
            };
        }
        if let Some(i) = self.known[p].get(&t) {
            return *i;
        }
        // predict: the newest input before it carries on
        self.known[p]
            .range(..t)
            .next_back()
            .map(|(_, i)| *i)
            .unwrap_or_default()
    }

    fn record(&mut self, p: usize, t: u32, i: Input) {
        if self.known[p].contains_key(&t) {
            return;
        }
        self.known[p].insert(t, i);
        while self.known[p].contains_key(&self.upto[p]) {
            self.upto[p] += 1;
        }
        // was a different input used for this tick?
        if t < self.current {
            if let Some((_, used)) = self.used.iter().find(|(ut, _)| *ut == t) {
                if used[p] != i {
                    self.rollback_to = Some(self.rollback_to.map_or(t, |r| r.min(t)));
                }
            }
        }
    }

    /// This machine's input for every tick up to `DELAY` ahead of the
    /// current one that does not have one yet. A frame that runs two ticks
    /// leaves no gap: a gap would hold the confirmed tick back for good.
    pub fn local(&mut self, i: Input) {
        let to = self.current + DELAY;
        let mut t = self.upto[self.me];
        while t <= to {
            if !self.known[self.me].contains_key(&t) {
                self.record(self.me, t, i);
                if self.unsent.is_empty() {
                    self.unsent_from = t;
                }
                self.unsent.push(i);
            }
            t += 1;
        }
    }

    /// The inputs to send since the last call.
    pub fn take_unsent(&mut self) -> Option<Msg> {
        if self.unsent.is_empty() {
            return None;
        }
        Some(Msg::In {
            p: self.me,
            from: self.unsent_from,
            i: std::mem::take(&mut self.unsent),
            at: self.current,
        })
    }

    /// Inputs heard from player `p`.
    pub fn remote(&mut self, p: usize, from: u32, inputs: &[Input], at: u32) {
        if p >= self.players || p == self.me {
            return;
        }
        self.their_at[p] = self.their_at[p].max(at);
        for (k, i) in inputs.iter().enumerate() {
            self.record(p, from + k as u32, *i);
        }
    }

    pub fn left(&mut self, p: usize, from: u32) {
        if p >= self.players || self.gone_from[p].is_some() {
            return;
        }
        self.gone_from[p] = Some(from);
        if from < self.current {
            self.rollback_to = Some(self.rollback_to.map_or(from, |r| r.min(from)));
        }
    }

    fn inputs(&self, t: u32) -> Vec<Input> {
        (0..self.players).map(|p| self.input_for(p, t)).collect()
    }

    /// Runs up to `due` new ticks (after any rollback) and returns the
    /// events of ticks shown for the first time.
    pub fn advance(&mut self, run: &mut Run, r: &Ref, due: u32) -> Vec<Event> {
        let mut events = Vec::new();
        if let Some(to) = self.rollback_to.take() {
            if let Some(k) = self.snaps.iter().position(|(t, _)| *t == to) {
                *run = self.snaps[k].1.clone();
                self.snaps.truncate(k);
                self.used.retain(|(t, _)| *t < to);
                self.rollbacks += 1;
                let target = self.current;
                self.current = to;
                while self.current < target {
                    self.rolled_ticks += 1;
                    self.tick_once(run, r, &mut events);
                }
            }
        }
        for _ in 0..due {
            if self.current >= self.confirmed().saturating_add(MAX_AHEAD) {
                self.stalls += 1;
                break;
            }
            self.tick_once(run, r, &mut events);
        }
        // what is confirmed can no longer be rolled back past
        let c = self.confirmed();
        while self.snaps.front().is_some_and(|(t, _)| *t < c) {
            self.snaps.pop_front();
        }
        while self.used.front().is_some_and(|(t, _)| *t < c) {
            self.used.pop_front();
        }
        events
    }

    fn tick_once(&mut self, run: &mut Run, r: &Ref, events: &mut Vec<Event>) {
        let t = self.current;
        let inputs = self.inputs(t);
        if t >= self.confirmed() {
            self.snaps.push_back((t, run.clone()));
            self.used.push_back((t, inputs.clone()));
        }
        run.step(r, &inputs);
        if t >= self.shown {
            events.extend(run.events.iter().copied());
            self.shown = t + 1;
        }
        self.current = t + 1;
        // a confirmed tick's state is final: hash it now and then
        if t.is_multiple_of(SUM_EVERY) && t < self.confirmed() && !self.sums.contains_key(&t) {
            self.sums.insert(t, run.checksum());
            while self.sums.len() > 32 {
                let first = *self.sums.keys().next().unwrap_or(&0);
                self.sums.remove(&first);
            }
        }
    }

    /// How many ticks this copy is ahead of the slowest other copy, for the
    /// game to slow down by now and then so nobody is always predicting.
    pub fn ahead(&self) -> i32 {
        let mut slowest = u32::MAX;
        for p in 0..self.players {
            if p != self.me && self.gone_from[p].is_none() {
                slowest = slowest.min(self.their_at[p]);
            }
        }
        if slowest == u32::MAX {
            return 0;
        }
        self.current as i32 - slowest as i32
    }

    /// Drops what will never be needed again.
    pub fn prune(&mut self) {
        let c = self.confirmed();
        let keep = c.saturating_sub(4);
        for k in self.known.iter_mut() {
            while let Some((&t, _)) = k.iter().next() {
                // keep the newest known input for predictions
                if t >= keep || k.len() <= 1 {
                    break;
                }
                k.remove(&t);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::tests::content;
    use crate::mall::tests::map;
    use crate::raider::{B_FIRE, B_FWD, B_LEFT, B_USE};
    use crate::run::tests::setup;

    fn script(p: usize, t: u32) -> Input {
        let b = match (t / 37 + p as u32) % 4 {
            0 => B_FWD | B_USE,
            1 => B_FWD | B_LEFT,
            2 => B_FIRE,
            _ => B_FWD,
        };
        Input {
            b,
            y: (t as i32 * 97 * (p as i32 + 1)) & 0xffff,
            p: 0,
        }
    }

    /// Two machines, each sending to the other with a lag of `lag` frames;
    /// both must end on the same run as a copy fed every input on time.
    #[test]
    fn two_copies_with_lag_roll_back_into_agreement_with_the_true_run() {
        let c = content();
        let m = map();
        let s = setup(&c, "power", 2);
        let r = Ref {
            content: &c,
            map: &m,
            setup: &s,
        };
        let mut truth = Run::new(&r);
        let mut runs = [Run::new(&r), Run::new(&r)];
        let mut sess = [Session::new(2, 0), Session::new(2, 1)];
        let lag = 5usize;
        let mut wires: [VecDeque<(usize, Msg)>; 2] = [VecDeque::new(), VecDeque::new()];
        let frames = 600u32;
        for f in 0..frames {
            for k in 0..2 {
                let t = sess[k].current + DELAY;
                sess[k].local(script(k, t));
                if let Some(msg) = sess[k].take_unsent() {
                    wires[1 - k].push_back((f as usize + lag, msg));
                }
                while wires[k].front().is_some_and(|(at, _)| *at <= f as usize) {
                    let (_, msg) = wires[k].pop_front().unwrap();
                    if let Msg::In { p, from, i, at } = msg {
                        sess[k].remote(p, from, &i, at);
                    }
                }
                sess[k].advance(&mut runs[k], &r, 1);
            }
        }
        // let the last messages land
        for f in frames..frames + 40 {
            for k in 0..2 {
                let t = sess[k].current + DELAY;
                sess[k].local(script(k, t));
                if let Some(msg) = sess[k].take_unsent() {
                    wires[1 - k].push_back((f as usize + lag, msg));
                }
                while wires[k].front().is_some_and(|(at, _)| *at <= f as usize) {
                    let (_, msg) = wires[k].pop_front().unwrap();
                    if let Msg::In { p, from, i, at } = msg {
                        sess[k].remote(p, from, &i, at);
                    }
                }
            }
        }
        // bring both copies to the same tick, every input known
        let end = sess[0].confirmed().min(sess[1].confirmed());
        for k in 0..2 {
            let due = end.saturating_sub(sess[k].current);
            sess[k].advance(&mut runs[k], &r, due);
        }
        assert!(sess[0].rollbacks > 0, "lag made it predict and roll back");
        // the true run, every input on time (first DELAY ticks have none)
        for t in 0..sess[0].current {
            let i: Vec<Input> = (0..2)
                .map(|p| if t < DELAY { Input::default() } else { script(p, t) })
                .collect();
            truth.step(&r, &i);
        }
        assert_eq!(sess[0].current, sess[1].current);
        assert_eq!(
            runs[0].checksum(),
            truth.checksum(),
            "copy 0 agrees with the truth"
        );
        assert_eq!(
            runs[1].checksum(),
            truth.checksum(),
            "copy 1 agrees with the truth"
        );
    }

    #[test]
    fn frames_that_run_several_ticks_leave_no_gap_in_the_inputs() {
        let c = content();
        let m = map();
        let s = setup(&c, "cudi", 2);
        let r = Ref {
            content: &c,
            map: &m,
            setup: &s,
        };
        let mut runs = [Run::new(&r), Run::new(&r)];
        let mut sess = [Session::new(2, 0), Session::new(2, 1)];
        for f in 0..120u32 {
            let due = [1 + f % 3, 1 + (f + 1) % 2];
            for k in 0..2 {
                sess[k].local(script(k, sess[k].current));
            }
            let msgs: Vec<Option<Msg>> = (0..2).map(|k| sess[k].take_unsent()).collect();
            for (k, m) in msgs.into_iter().enumerate() {
                if let Some(Msg::In { p, from, i, at }) = m {
                    sess[1 - k].remote(p, from, &i, at);
                }
            }
            for k in 0..2 {
                sess[k].advance(&mut runs[k], &r, due[k]);
            }
        }
        assert!(sess[0].confirmed() > 150, "{}", sess[0].confirmed());
        assert!(sess[1].confirmed() > 150, "{}", sess[1].confirmed());
    }

    #[test]
    fn a_copy_waits_rather_than_run_too_far_ahead() {
        let c = content();
        let m = map();
        let s = setup(&c, "cudi", 2);
        let r = Ref {
            content: &c,
            map: &m,
            setup: &s,
        };
        let mut run = Run::new(&r);
        let mut s0 = Session::new(2, 0);
        for _ in 0..40 {
            s0.local(Input::default());
            s0.advance(&mut run, &r, 1);
        }
        assert_eq!(s0.current, DELAY + MAX_AHEAD, "nothing heard from player 1");
        assert!(s0.stalls > 0);
        // player 1 leaves: everything confirms and it runs on
        s0.left(1, DELAY);
        s0.advance(&mut run, &r, 5);
        assert!(s0.current > DELAY + MAX_AHEAD);
    }

    #[test]
    fn every_message_round_trips() {
        let msgs = vec![
            Msg::Hello {
                proto: PROTO,
                build: "b".into(),
                gun: vec!["pistol".into()],
            },
            Msg::In {
                p: 2,
                from: 9,
                i: vec![Input { b: 5, y: -3, p: 7 }, Input::default()],
                at: 11,
            },
            Msg::Left { p: 1, from: 30 },
            Msg::Start {
                mission: 2,
                seed: 7,
                guns: vec![vec!["ak".into()]],
                you: 1,
            },
        ];
        for m in msgs {
            assert_eq!(decode(&encode(&m)), Some(m));
        }
        assert_eq!(
            encode(&Msg::In {
                p: 0,
                from: 0,
                i: vec![Input::default()],
                at: 0
            }),
            r#"{"t":"In","p":0,"from":0,"i":[{"b":0,"y":0,"p":0}],"at":0}"#
        );
    }
}
