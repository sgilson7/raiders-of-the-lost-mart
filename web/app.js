// The page draws what the core returns and decides nothing. It loads the data
// files, starts the core through the shim, and each display frame passes the
// clock, the held actions and the mouse movement in, then draws what comes
// out. The core says which screen is up; the page shows it.
import init, { Game, numbers } from "./pkg/rm_shim.js";
import { mesh } from "./pkg/rm_shim.js";
import { makeRenderer } from "./gl.js";
import { makeInput } from "./input.js";
import { makeHud } from "./hud.js";
import { makeScreens } from "./screens.js";
import { setup as setupSound, wake, toggleMute } from "./sound.js";
import { makeOnline } from "./online.js";

const BUILD = "__BUILD__";
const SAVE_KEY = "raiders-lost-mart.record";
const el = (id) => document.getElementById(id);
const file = (path) => fetch(`${path}?v=${BUILD}`).then((r) => r.text());

// Browser storage can be missing or refused; the game works without it.
const storage = {
  load() {
    try {
      return localStorage.getItem(SAVE_KEY) || "";
    } catch {
      return "";
    }
  },
  save(text) {
    try {
      localStorage.setItem(SAVE_KEY, text);
    } catch {
      // nothing to do: the record is simply not kept
    }
  },
};

function fillCopy(copy) {
  for (const node of document.querySelectorAll("[data-copy]")) {
    node.textContent = copy[node.dataset.copy];
  }
  document.title = copy.title;
  el("build").textContent = BUILD;
}

async function start() {
  const names = ["parts", "drones", "missions", "palette", "gear", "copy", "controls", "sounds", "display"];
  const texts = await Promise.all(
    names.map((n) => file(`data/${n}.json`)).concat([file("data/shaders/scene.vert"), file("data/shaders/scene.frag")]),
  );
  const [parts, drones, missions, palette, gear, copyText, controlsText, soundsText, displayText, vertex, fragment] = texts;
  const copy = JSON.parse(copyText);
  const display = JSON.parse(displayText);
  fillCopy(copy);
  setupSound(JSON.parse(soundsText));
  await init();
  const N = JSON.parse(numbers());

  let game;
  try {
    game = new Game(parts, drones, missions, palette, gear, storage.load());
  } catch (e) {
    el("message").textContent = String(e);
    document.body.dataset.ready = "true";
    return;
  }

  const canvas = el("view");
  const renderer = makeRenderer(canvas, N, { vertex, fragment, software: display.software }, (id) => mesh(id));
  if (!renderer) el("message").textContent = copy.no_webgl;
  const input = makeInput(canvas, JSON.parse(controlsText), N.actions);
  const hud = makeHud(copy, game);
  const save = () => storage.save(game.saved());
  const online = makeOnline(game, BUILD);
  const screens = makeScreens(game, copy, () => redraw());
  screens.help();

  let shownMode = null;
  // a button: wake the sound, act, keep the record, redraw
  const act = (fn) => () => {
    wake();
    fn();
    if (document.activeElement && document.activeElement.blur) document.activeElement.blur();
    save();
    redraw();
  };
  const on = (id, fn) => el(id).addEventListener("click", act(fn));

  on("t-solo", () => game.board());
  on("t-workshop", () => game.workshop());
  on("t-host", () => online.host("room"));
  on("t-host-code", () => {
    online.host("code");
    el("invite-box").open = true;
  });
  on("t-join", () => {
    el("join").hidden = false;
    el("join-error").textContent = "";
  });
  on("ws-back", () => game.back());
  on("ws-board", () => {
    game.back();
    game.board();
  });
  on("b-back", () => game.back());
  on("b-workshop", () => game.workshop());
  // ?at=x,z: a developer's shortcut into the map (the core ignores it online)
  const devAt = new URLSearchParams(location.search).get("at");
  on("b-go", () => {
    game.start_solo();
    if (devAt) game.dev_place(devAt);
    input.capture();
  });
  on("l-start", () => {
    game.start_online();
    input.capture();
  });
  on("l-leave", () => online.leave());
  on("l-copy", () => {
    const link = location.origin + location.pathname + "#room=" + game.room();
    if (navigator.clipboard) navigator.clipboard.writeText(link).catch(() => {});
  });
  on("resume", () => {
    game.resume();
    input.capture();
  });
  on("quit", () => {
    game.quit_run();
    online.close();
  });
  on("mute", () => toggleMute());
  on("r-next", () => game.back());
  on("sens-up", () => game.nudge_sens(true));
  on("sens-down", () => game.nudge_sens(false));
  for (const b of document.querySelectorAll(".open-help")) {
    b.addEventListener("click", () => {
      el("help").hidden = false;
    });
  }
  on("help-close", () => {
    el("help").hidden = true;
  });

  // joining
  on("join-back", () => {
    el("join").hidden = true;
    online.leave();
  });
  const joinRoom = () => {
    wake();
    if (!online.join(el("room-code").value)) {
      el("join-error").textContent = copy.online_bad_room;
      return;
    }
    el("join").hidden = true;
    redraw();
  };
  el("join-room").addEventListener("click", joinRoom);
  el("room-code").addEventListener("keydown", (e) => {
    if (e.code === "Enter") joinRoom();
  });
  el("make-reply").addEventListener("click", () => {
    wake();
    online
      .joinByCode(el("invite-in").value)
      .then((reply) => {
        el("reply-out").value = reply;
        el("reply-box").hidden = false;
        document.body.dataset.reply = "ready";
        redraw();
      })
      .catch(() => {
        el("join-error").textContent = copy.online_bad_code;
      });
  });
  // hosting by pasted codes
  el("new-invite").addEventListener("click", () => {
    el("invite-out").value = "";
    online.invitation().then((code) => {
      el("invite-out").value = code;
      document.body.dataset.invite = "ready";
    });
  });
  el("accept-reply").addEventListener("click", () => {
    online
      .accept(el("reply-in").value)
      .then(() => {
        el("reply-in").value = "";
      })
      .catch(() => {
        el("l-status").textContent = copy.online_bad_code;
      });
  });
  const hashRoom = location.hash.match(/^#room=(\w+)$/);
  if (hashRoom) {
    el("room-code").value = hashRoom[1];
    el("join").hidden = false;
  }

  // the pointer: captured while playing, freed for menus
  canvas.addEventListener("click", () => {
    wake();
    if (document.body.dataset.mode === "play") input.capture();
  });
  document.addEventListener("pointerlockchange", () => {
    // leaving the capture (Escape) while playing opens the menu, unless the
    // inventory sheet is what freed the pointer
    if (!input.locked() && document.body.dataset.mode === "play" && el("inventory").hidden) {
      game.pause();
      redraw();
    }
  });
  window.addEventListener("keydown", (e) => {
    if (e.code === "KeyM") toggleMute();
    if (e.code === "Escape" && !el("help").hidden) el("help").hidden = true;
    wake();
  });
  document.addEventListener("visibilitychange", () => {
    if (document.hidden) game.pause();
  });

  function resize() {
    const scale = renderer && renderer.software ? display.software_scale : 1;
    const [w, h] = game.resize(canvas.clientWidth, canvas.clientHeight, (window.devicePixelRatio || 1) * scale);
    if (renderer) renderer.resize(w, h);
  }
  window.addEventListener("resize", resize);
  resize();

  const overlays = { title: "title", workshop: "workshop", board: "board", lobby: "lobby", paused: "paused", results: "results" };
  let screenShown = "";
  function redraw() {
    shownMode = null;
    screenShown = "";
  }
  function showMode(h) {
    const mode = h.mode;
    const menu = Boolean(h.menu);
    const inv = Boolean(h.inventory);
    const key = mode + (menu ? "+menu" : "") + (inv ? "+inv" : "");
    if (key !== shownMode) {
      document.body.dataset.mode = mode;
      for (const id of Object.values(overlays)) el(id).hidden = true;
      if (overlays[mode]) el(overlays[mode]).hidden = false;
      if (menu) el("paused").hidden = false;
      el("p-title").textContent = menu ? copy.menu : copy.paused;
      el("hud").hidden = !["play", "paused"].includes(mode);
      if (mode !== "play" || menu || inv) {
        input.release();
        input.free();
      }
      if (mode === "results") save();
      if (mode === "lobby") el("join").hidden = true;
      shownMode = key;
    }
    // menus change with clicks and with messages from other browsers: redraw
    // when, and only when, what they show has changed
    if (mode !== "play") {
      const text = game.screen();
      if (text !== screenShown) {
        screenShown = text;
        screens.draw(JSON.parse(text));
      }
    }
  }

  let sceneVersion = null;
  function frame(now) {
    const { bits, dx, dy } = input.read();
    game.advance(now, bits, dx, dy);
    online.pump();
    const text = game.hud();
    // kept for the browser checks, which read it rather than the screen
    window.__hud = text;
    const h = JSON.parse(text);
    if (!h.online && online.active()) online.close();
    showMode(h);
    hud.draw(h);
    if (renderer) {
      if (game.scene_version() !== sceneVersion) {
        sceneVersion = game.scene_version();
        renderer.setScene(game.static_instances(), game.uniforms(), game.clear_color());
      }
      renderer.frame(game.view(), game.instances(), game.draws());
    }
    requestAnimationFrame(frame);
  }

  showMode(JSON.parse(game.hud()));
  document.body.dataset.ready = "true";
  requestAnimationFrame(frame);
}

start();
