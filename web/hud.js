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

function partIcon(id, copy) {
  const img = document.createElement("img");
  img.src = id.startsWith("item_") ? "art/parts/scanner.png" : "art/parts/" + id + ".png";
  img.alt = copy[id.startsWith("item_") ? id : "part_" + id] || id;
  img.title = img.alt;
  if (id.startsWith("item_")) img.className = "mission-item";
  return img;
}

export function makeHud(copy) {
  const markers = el("markers");
  let shownPack = "";
  let shownTeam = "";

  function setBar(id, value) {
    el(id).style.width = value;
  }

  return {
    partIcon: (id) => partIcon(id, copy),
    draw(h) {
      for (const s of h.sounds || []) play(s);
      if (h.mode !== "play" && h.mode !== "paused") return;
      // the order
      el("o-name").textContent = copy["mission_" + h.mission + "_name"] || "";
      el("o-goal").textContent = fill(copy[h.objective.key], h.objective);
      el("o-zone").textContent = copy["zone_" + h.zone] || "";
      el("order").classList.toggle("done", Boolean(h.done));
      // health and the gun
      setBar("hp-bar", h.hp);
      el("hp-n").textContent = h.hp_n;
      el("ammo-n").textContent = h.ammo + " / " + h.mag;
      const busy = h.reload || h.spin;
      el("reload-wrap").hidden = !busy;
      if (busy) setBar("reload-bar", h.reload || h.spin);
      el("ammo-sub").textContent = h.reload ? copy.hud_reloading : h.ammo === 0 ? copy.hud_empty : h.spin ? copy.hud_spinning : "";
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
        const part = h.prompt.part ? copy[h.prompt.part.startsWith("item_") ? h.prompt.part : "part_" + h.prompt.part] : "";
        el("prompt-text").textContent = copy[h.prompt.key] + (part ? ": " + part : "");
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
      const packKey = h.pack.join(",");
      if (packKey !== shownPack) {
        shownPack = packKey;
        el("pack-items").replaceChildren(...h.pack.map((id) => partIcon(id, copy)));
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
