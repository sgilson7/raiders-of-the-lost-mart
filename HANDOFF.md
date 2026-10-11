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

## Waiting on Sam

1. **The website listing.** The record, project page, tile and route are written in `~/Documents/website` (uncommitted): `content/projects.yaml` (a `raiders-of-the-lost-mart` record before `iron-vector`), `content/projects/raiders-of-the-lost-mart.md`, `static/img/tiles/raiders-of-the-lost-mart.webp`, and `/raiders-of-the-lost-mart/` in `OFF_BUILD_ROUTES` in `crates/cli/src/main.rs`. Another session (`website-1e`) had uncommitted work in progress in the same tree, which makes `make packet` fail on five project links. On a clean copy of HEAD plus only these four files, `make check` and `make packet` pass. I did not deploy over that session's unfinished work; I asked it to fold these files in or say when the tree is clear. See the status line at the end of this file.
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
