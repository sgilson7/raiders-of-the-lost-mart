// The heads-up display: the core's hud() as text, bars and markers. Every
// number arrives already worked out (percentages as strings, distances in
// words); this file places them.
import { play } from "./sound.js";

const el = (id) => document.getElementById(id);

// "{n} left" with the core's values put in
export function fill(text, values) {
  let out = text || "";
  for (const key of Object.keys(values || {})) out = out.split("{" + key + "}").join(String(values[key]));
  return out;
}

// An item (the core's item JSON) as a little picture, with its count.
export function itemIcon(it, copy) {
  const wrap = document.createElement("span");
  wrap.className = "item r" + it.rarity + " " + it.kind;
  const img = document.createElement("img");
  img.src = "art/" + it.icon + ".png";
  img.alt = copy[it.name] || "";
  wrap.title = img.alt;
  wrap.append(img);
  if (it.n !== undefined) {
    const n = document.createElement("span");
    n.className = "count";
    n.textContent = it.n;
    wrap.append(n);
  }
  return wrap;
}

export function makeHud(copy, game) {
  let shownInv = "";
  let chosen = null;
  // the inventory sheet: the pack, the gun's parts and what is worn; hover
  // or click an item to read it, and drop what is in the pack
  function detail(it, slot) {
    chosen = slot;
    el("inv-d-icon").src = "art/" + it.icon + ".png";
    el("inv-d-name").textContent = (copy[it.name] || "") + (it.n !== undefined ? " ×" + it.n : "");
    el("inv-d-about").textContent = copy[it.about] || "";
    el("inv-drop").hidden = slot === null;
  }
  function inventory(h) {
    const key = JSON.stringify([h.pack, h.wear, h.gun_parts]);
    if (key === shownInv) return;
    shownInv = key;
    const grid = (id, list, droppable) =>
      el(id).replaceChildren(
        ...list.map((it, k) => {
          const b = document.createElement("button");
          b.type = "button";
          b.className = "inv-item";
          b.append(itemIcon(it, copy));
          const show = () => detail(it, droppable ? k : null);
          b.addEventListener("mouseenter", show);
          b.addEventListener("focus", show);
          b.addEventListener("click", show);
          return b;
        }),
      );
    grid("inv-pack", h.pack, true);
    grid("inv-gun", h.gun_parts, false);
    grid("inv-wear", h.wear, false);
    if (h.pack.length) detail(h.pack[0], 0);
    else el("inv-drop").hidden = true;
  }
  el("inv-drop").addEventListener("click", () => {
    if (chosen !== null) game.drop_item(chosen);
    chosen = null;
    el("inv-drop").hidden = true;
  });
  el("inv-close").addEventListener("click", () => game.close_inventory());

  // A drone's health bar: its name, how much health it has left, a pip for
  // each breakable part (dark once torn off), and what losing them did.
  function healthBar(d, m) {
    if (m.low) d.classList.add("low");
    if (m.aimed) d.classList.add("aimed");
    const name = document.createElement("div");
    name.className = "bar-name";
    name.textContent = copy[m.name] || "";
    const bar = document.createElement("div");
    bar.className = "bar thin";
    const fillBar = document.createElement("span");
    fillBar.style.width = m.hp;
    bar.append(fillBar);
    const pips = document.createElement("div");
    pips.className = "pips";
    for (const [kind, on] of m.parts) {
      const pip = document.createElement("span");
      pip.className = "pip " + kind + (on ? "" : " gone");
      pip.title = copy["piece_" + kind] || kind;
      pips.append(pip);
    }
    d.append(name, bar, pips);
    if (m.state.length) {
      const state = document.createElement("div");
      state.className = "bar-state";
      state.textContent = m.state.map((k) => copy[k]).join(" · ");
      d.append(state);
    }
    return d;
  }

  const markers = el("markers");
  let shownPack = "";
  let shownTeam = "";

  function setBar(id, value) {
    el(id).style.width = value;
  }

  return {
    itemIcon: (it) => itemIcon(it, copy),
    draw(h) {
      for (const s of h.sounds || []) play(s);
      if (h.mode !== "play" && h.mode !== "paused") return;
      // the order
      el("o-name").textContent = copy["mission_" + h.mission + "_name"] || "";
      el("o-goal").textContent = fill(copy[h.objective.key], h.objective);
      el("o-zone").textContent = copy["zone_" + h.zone] || "";
      el("order").classList.toggle("done", Boolean(h.done));
      // the queen's bar, while she is near
      el("boss").hidden = !h.boss;
      if (h.boss) {
        el("boss-name").textContent = copy[h.boss.name] || "";
        setBar("boss-bar", h.boss.hp);
        el("boss-pips").replaceChildren(
          ...h.boss.parts.map(([kind, on]) => {
            const pip = document.createElement("span");
            pip.className = "pip " + kind + (on ? "" : " gone");
            pip.title = copy["piece_" + kind] || kind;
            return pip;
          }),
        );
      }
      // health and the gun
      setBar("hp-bar", h.hp);
      el("hp-n").textContent = h.hp_n;
      el("ammo-n").textContent = h.ammo + " / " + h.mag;
      const busy = h.reload || h.spin;
      el("reload-wrap").hidden = !busy;
      if (busy) setBar("reload-bar", h.reload || h.spin);
      el("ammo-sub").textContent = h.reload ? copy.hud_reloading : h.ammo === 0 ? (h.reserve ? copy.hud_empty : copy.hud_dry) : h.spin ? copy.hud_spinning : "";
      el("reserve-n").textContent = h.reserve;
      el("ammo-icon").src = "art/gear/" + h.ammo_kind + ".png";
      el("ammo-icon").title = copy[h.ammo_kind] || "";
      // stance, melee, the belt gadget, the roll
      el("stance").textContent = h.crouch ? copy.hud_crouched : copy.hud_standing;
      el("k-stance").classList.toggle("on", Boolean(h.crouch));
      el("melee-name").textContent = copy[h.melee_name] || "";
      setBar("melee-bar", h.melee);
      el("k-gadget").hidden = !h.gadget;
      if (h.gadget) {
        el("gadget-icon").src = "art/" + h.gadget.icon + ".png";
        el("gadget-name").textContent = copy[h.gadget.name] || "";
        el("gadget-n").textContent = h.gadget.charges;
      }
      el("k-roll").hidden = !h.roll;
      if (h.roll) setBar("roll-bar", h.roll);
      el("inventory").hidden = !h.inventory;
      if (h.inventory) inventory(h);
      else shownInv = "";
      // the crosshair, the hit marker, the scope and the hurt flash
      document.body.style.setProperty("--spread", h.spread);
      el("hitmark").hidden = !h.hit;
      el("hitmark").classList.toggle("weak", Boolean(h.weak));
      el("hitmark").classList.toggle("kill", Boolean(h.kill));
      el("scope").hidden = !h.scope;
      el("crosshair").hidden = Boolean(h.scope) || h.life !== "alive";
      el("hurt").hidden = !h.hurt;
      // what E would do
      el("prompt").hidden = !h.prompt;
      if (h.prompt) {
        const item = h.prompt.item ? copy[h.prompt.item.name] + (h.prompt.item.n !== undefined ? " ×" + h.prompt.item.n : "") : "";
        el("prompt-text").textContent = copy[h.prompt.key] + (item ? ": " + item : "");
        setBar("prompt-bar", h.prompt.pct);
      }
      el("toast").hidden = !h.toast;
      if (h.toast) el("toast").textContent = copy[h.toast];
      el("down").hidden = h.life !== "downed";
      if (h.life === "downed") setBar("bleed-bar", h.bleed);
      el("watching").hidden = !h.watching;
      if (h.watching) el("watching").textContent = fill(copy.hud_watching, { n: h.watching });
      el("leaving").hidden = !h.leaving;
      if (h.leaving) el("leave-n").textContent = h.leaving;
      el("clock").textContent = h.time;
      // the pack
      const packKey = JSON.stringify(h.pack);
      if (packKey !== shownPack) {
        shownPack = packKey;
        el("pack-items").replaceChildren(...h.pack.map((it) => itemIcon(it, copy)));
      }
      el("pack-n").textContent = h.pack.length + " / " + h.pack_max;
      // the team
      const teamKey = JSON.stringify(h.team);
      if (teamKey !== shownTeam) {
        shownTeam = teamKey;
        el("team").replaceChildren(
          ...h.team.map((t) => {
            const row = document.createElement("div");
            row.className = "mate " + t.life + (t.you ? " you" : "");
            const dot = document.createElement("span");
            dot.className = "dot";
            dot.style.background = t.colour;
            const bar = document.createElement("span");
            bar.className = "bar thin";
            const fillBar = document.createElement("span");
            fillBar.style.width = t.hp;
            bar.append(fillBar);
            const word = document.createElement("span");
            word.className = "word";
            word.textContent = copy["life_" + t.life];
            row.append(dot, bar, word);
            return row;
          }),
        );
      }
      // markers over the world
      const nodes = h.markers.map((m) => {
        const d = document.createElement("div");
        d.className = "marker " + m.kind + (m.off ? " off" : "");
        d.style.left = m.x;
        d.style.top = m.y;
        if (m.kind === "health") return healthBar(d, m);
        if (m.kind === "aware") {
          // a drone's awareness of me: a meter that fills, "?" then "!"
          if (m.alert) d.classList.add("alert");
          else if (m.suspect) d.classList.add("suspect");
          const fillMeter = document.createElement("span");
          fillMeter.className = "fill";
          fillMeter.style.height = m.level;
          const mark = document.createElement("span");
          mark.className = "mark";
          mark.textContent = m.alert ? "!" : "?";
          d.append(fillMeter, mark);
          return d;
        }
        if (m.kind === "pop") {
          d.classList.add(m.cls);
          d.textContent = m.text.startsWith("off_") ? copy[m.text] : m.text;
          d.style.opacity = m.fade;
          return d;
        }
        const label = copy["marker_" + m.kind];
        if (m.kind !== "weak") d.textContent = (label ? label + " · " : "") + m.dist;
        return d;
      });
      markers.replaceChildren(...nodes);
      // online
      el("netline").hidden = !h.net;
      if (h.net) {
        el("netline").textContent = h.net.room + " · " + fill(copy.hud_rollbacks, { n: h.net.rollbacks }) + (h.net.desync ? " · " + copy.hud_desync : "");
      }
    },
  };
}
