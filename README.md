# Raiders of the Lost Mart

A four-player co-op extraction shooter that runs in the browser, in 3D. You and up to three others are hired raiders. Each run starts in the back of a van parked far out in the lot of an abandoned Southpoint mall in Durham, now full of drones. A client's order sends you in: fetch a vintage Kid Cudi tee from the Hot Topic, clear a nest out of the food court, pull the camera drives, break a hive, or get the lights on. Whatever you carry back to the van is yours, and you build your guns from it.

**Play:** https://sgilson7.github.io/raiders-of-the-lost-mart/

## The game

- **Guns from parts.** Six slots, five parts each: a frame, a barrel, a mount that sets out how many barrels and where, a payload every shot carries, a sight and a grip. Shots leave from the end of each barrel on the model, so a pistol frame with a square mount of double barrels fires eight loads of buckshot from eight muzzles, and an AK frame with a rocket tube full of goo fires rockets that slow drones and eat their armour without hurting them.
- **Extraction.** A part on your gun comes off your shelf. Come back in the van and you keep it and everything in your pack; get left behind and it is gone. Fill the order and the client pays in rare parts.
- **Six drones.** Wasps circle and strafe. Moths spot you, call the others, and fly out where you can see them to draw you out of cover toward waiting wasps. Hornets hide, peek out somewhere new and charge a laser. Wardens keep their armour toward you. Tick spiders run at you and blow up; striders keep their distance and stomp. Each has armour, a glowing weak spot and, if it flies, rotors you can shoot off; hits shove and spin them like real drones.
- **One map.** The concourse with its balconies and bridges, the centre court and its escalators, the food court, three department stores, a side hall out to Main Street and the cinema, and the lots all round. Each order parks the van in front of a different entrance.
- **Online.** Up to four people, each in their own browser, meet by room code or pasted invitation. Every browser runs the whole run and rolls back when another raider's input arrives late, so nobody waits on the host.

## Playing it

W A S D move, the mouse looks (click the view to capture it), left button fires, right button aims, R reloads, E (held) opens doors, searches, picks up, revives and drives the van off, Shift sprints, Space jumps, Esc pauses. The arrow keys turn without a mouse, and Enter fires.

## How it is built

A [Builder](https://github.com/sgilson7/builder-setting) project, from Ribbit King: every rule is in a Rust core (`crates/core`) in fixed-point integer maths, compiled to WebAssembly through a shim that decides nothing (`crates/shim`), and a static page (`web/`) draws what the core returns. The menu art, the drone portraits and the part icons are drawn in TikZ (`tools/art.py`).

    ./scripts/setup.sh     # once: Playwright and three browser engines
    ./scripts/check.sh     # format, lint, tests, build, WebAssembly audit, browsers, four-browser online run
    python3 scripts/serve.py   # then open http://127.0.0.1:8000

`BRIEF.md` has the design as it was given, `PLAN.md` the decisions taken from it, `SECOND-ORDER.md` what came up while building, and `HANDOFF.md` what was checked.

A fan-made game from a friend's design. MIT licence; see `LICENSES.md`.
