# Handoff

11 October 2026. Raiders of the Lost Mart, the MVP.

## What exists

| What | Where |
| --- | --- |
| The game, live | https://sgilson7.github.io/raiders-of-the-lost-mart/ |
| Repository (public, MIT) | https://github.com/sgilson7/raiders-of-the-lost-mart |
| Deploys | `.github/workflows/deploy.yml`: a push to `main` runs `./scripts/check.sh` (Chromium) and deploys `dist/` to Pages only if it passes |

Everything the brief asked for is in: one to four raiders, alone or online; rollback netcode; a third-person shooter; the van start far out in the lot and the van as the way out; one map of the Streets at Southpoint with its lots, Main Street and five entrances; four flying drones and two spider walkers with weak spots, armour, rotors, drone-like hit reactions and behaviour trees (patrol, hover, circle, peek, bait out of cover); guns built from six kinds of part, five of each, firing from their own muzzles; extraction to a stash; five client orders; TikZ art for every 2D picture.

## Checks run, and results

| Command | Result |
| --- | --- |
| `cargo test --workspace --release` | 61 core tests, 12 boundary tests, 2 perf probes: all pass |
| `./scripts/check.sh` (local, three engines) | format, clippy, tests, build, WebAssembly audit: pass. Browser check: PASSED in Chromium, Firefox and WebKit. Four-browser online run: hashes of the same confirmed ticks identical in all four copies in every engine (Firefox and WebKit to tick 1080), doors open in every copy; a lobby timing race in the test failed "joiner sees its seat" in Firefox and WebKit; fixed and re-run in WebKit: ONLINE CHECK PASSED |
| CI on `main` (Chromium) | check: success, deploy: success |
| `BUILDER_ORIGIN=https://sgilson7.github.io/raiders-of-the-lost-mart tests/browser/check.py` | PASSED in Chromium, Firefox and WebKit against the live site |
| `cargo test -p rm_core --release --test perf -- --nocapture` | a busy four-raider tick with 29 drones: about 35 µs natively; a run snapshot (rollback clones one a tick): under 1 µs |

## Since the first release (playtest rounds)

- Sound: the van's engine replayed every frame (the "droning"); legs walked at six strides a second. Fixed.
- Drones: breakable parts (rotors, plates, guns, legs, wings, beacon) that change behaviour when lost; health bars, part pips and damage numbers.
- Online: rooms named by protocol, not build, so a cached page still finds the room; a mismatched build is told to reload. Live room-code test through the public relays: two browsers met in about 4 s and ran in step.
- Flicker: z-fighting walls removed; a test lists any visible coplanar faces (171 before, none now).
- Crouch, gradual stealth with "?" and "!" meters, five ammo kinds, melee, ten gear slots with thirty-five pieces and four belt gadgets, the Tab inventory with drops; wardens fly under the balconies; inputs tapped between ticks are kept.
- Checks after the last round: 83 core tests, 12 boundary tests; browser check passed in Chromium, Firefox and WebKit locally and against the live site; the four-browser online run passed in all three engines.

- **Ammunition for gear, and a fair start (row 18).** Every run starts with rounds enough for three or four drones of average health, worked out from the gun on the bench (`gun::starting_rounds`, `START_*` in `balance.rs`). The workshop's trade counter turns a spare piece of gear into a stack of the picked ammunition per step of its rarity. One drop no longer drops several things on a frame that runs several ticks (row 19). `./scripts/check.sh` passed in Chromium, Firefox and WebKit, online included.
- **Crates to search (row 20).** Every place to search is a drawn crate with a yellow lid and, close up, a soft pulsing glow until it is searched (then it stands open and grey). `./scripts/check.sh` passed in Chromium, Firefox and WebKit, online included.
- **Heavies you meet, and the queen (rows 21-22).** Wardens and striders now always patrol the concourse and the centre court. A sixth order, The Queen, sends the van to the north end of Main Street to bring down a building-sized walking mech. `./scripts/check.sh` passed in Chromium, Firefox and WebKit, online included.

## Waiting on Sam

1. **The website listing is live.** `website-1e`, the session working in `~/Documents/website`, folded the record, project page, tile and route into its commit `5f38125` and deployed it (build d1e468dc; make check, packet, gate and gate-live all passed). https://sgilson7.github.io/projects/raiders-of-the-lost-mart/ answers 200, and /projects/ links to the game. The tile has no hover clip; an animated `raiders-of-the-lost-mart-clip.webp` in `static/img/tiles/` would add one.
2. **The name** ("Raiders of the Lost Mart" is a working title) and **whether to credit the friend by name**.
3. **Real names.** Southpoint, Hot Topic and Kid Cudi appear in words only, because the design names them. No logo is drawn.
4. **Playtest.** Balance is first-pass (drone damage was lowered once after a test raider died in 2.5 s inside the food-court nest). Feel numbers are in `crates/core/src/balance.rs` and `data/*.json`.

## Known gaps

- The STUN/room-code path was not exercised by the automated test (it uses pasted codes with `?ice=none`, as Ribbit King's does); the transport is unchanged from Ribbit King, whose room-code path was checked by hand between machines.
- No mid-run joining and no host migration: if the host leaves, the run ends for everyone with nothing lost.
- A joiner always plays its own gun; the lobby shows everyone's, and there is no way to change a gun inside the lobby, so the copies cannot disagree about it yet (SECOND-ORDER row 8).
- Solo, going down ends the run (there is nobody to revive you).
- `?at=x,z` is a developer's shortcut that places a solo raider on the map; online runs ignore it.
- Mutation testing (`cargo mutants`) was not run.

## Where things are

`BRIEF.md` (the request and the design, verbatim), `PLAN.md` (each decision and the words it answers), `SECOND-ORDER.md` (what came up), `crates/core/src/` (`run.rs` the rules of a run, `rollback.rs` the netcode, `ai.rs` the behaviour trees, `drone.rs` the drone bodies, `gun.rs` the gun builder, `mall.rs` the map, `game.rs` the screens and the HUD), `tools/art*.py` (the TikZ).
