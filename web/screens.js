// The menus: the workshop, the client board, the lobby, the results and
// the help sheet, drawn from the core's screen() JSON. Clicking a part or an
// order calls the core; the core says what changed.
import { fill, itemIcon } from "./hud.js";

const el = (id) => document.getElementById(id);
const make = (tag, cls, text) => {
  const n = document.createElement(tag);
  if (cls) n.className = cls;
  if (text !== undefined) n.textContent = text;
  return n;
};

export function makeScreens(game, copy, changed) {
  function partTile(p, s, onPick) {
    const b = make("button", "part r" + p.rarity);
    b.type = "button";
    const on = s.build.includes(p.id);
    if (on) b.classList.add("on");
    const usable = p.basic || p.owned > 0 || on;
    if (!usable) b.classList.add("none");
    const img = make("img");
    img.src = "art/parts/" + p.id + ".png";
    img.alt = "";
    b.append(img, make("span", "p-name", copy["part_" + p.id]));
    const count = on ? copy.ws_on : p.basic ? copy.ws_basic : fill(copy.ws_owned, { n: p.owned });
    b.append(make("span", "p-count", count));
    b.title = copy["part_" + p.id + "_about"];
    b.addEventListener("click", () => onPick(p.id));
    return b;
  }

  function workshop(s) {
    const cats = ["frame", "barrel", "mount", "payload", "sight", "grip"];
    el("ws-build").replaceChildren(
      ...s.build.map((id, i) => {
        const d = make("div", "slot");
        const img = make("img");
        img.src = "art/parts/" + id + ".png";
        img.alt = "";
        d.append(make("div", "label", copy["ws_slot_" + cats[i]]), img, make("div", "p-name", copy["part_" + id]));
        return d;
      }),
    );
    const g = s.gun;
    const stat = (k, v) => {
      const d = make("div", "stat");
      d.append(make("span", "label", copy["stat_" + k]), make("strong", "", String(v)));
      return d;
    };
    const tags = [copy["kind_" + g.kind], g.auto ? copy.tag_auto : copy.tag_semi];
    if (g.splash) tags.push(copy.tag_splash);
    if (g.zoom) tags.push(copy.tag_zoom);
    if (g.scanner) tags.push(copy.tag_scanner);
    if (copy["effect_" + g.effect]) tags.push(copy["effect_" + g.effect]);
    el("ws-stats").replaceChildren(
      stat("volley", g.volley),
      stat("rate", g.rate),
      stat("mag", g.mag),
      stat("reload", g.reload),
      stat("muzzles", g.muzzles),
      stat("speed", g.speed),
      make("div", "tags", tags.join(" · ")),
    );
    // the gear: a row per slot, the piece worn first
    el("ws-gear").replaceChildren(
      ...s.slots.map((slot) => {
        const box = make("div", "cat");
        box.append(make("div", "label", copy["slot_" + slot]));
        const row = make("div", "parts");
        for (const g of s.gear.filter((q) => q.slot === slot)) {
          const b = make("button", "part r" + g.rarity);
          b.type = "button";
          if (g.on) b.classList.add("on");
          if (!g.basic && !g.owned && !g.on) b.classList.add("none");
          const img = make("img");
          img.src = "art/gear/" + g.id + ".png";
          img.alt = "";
          b.append(img, make("span", "p-name", copy["gear_" + g.id]));
          b.append(make("span", "p-count", g.on ? copy.ws_worn : g.basic ? copy.ws_basic : fill(copy.ws_owned, { n: g.owned })));
          b.title = copy["gear_" + g.id + "_about"];
          b.addEventListener("click", () => {
            game.equip(g.id);
            changed();
          });
          row.append(b);
        }
        box.append(row);
        return box;
      }),
    );
    const k = s.kit;
    const kstat = (key, v) => {
      const d = make("div", "stat");
      d.append(make("span", "label", copy["kit_" + key]), make("strong", "", String(v)));
      return d;
    };
    el("ws-kit").replaceChildren(
      kstat("hp", k.hp),
      kstat("armour", k.armour + "%"),
      kstat("speed", k.speed + "%"),
      kstat("slots", k.slots),
      kstat("notice", k.notice + "%"),
      kstat("melee", k.melee),
    );
    el("ws-ammo").replaceChildren(
      ...s.ammo.map((a) => {
        const d = make("div", "ammo-tile" + (a.id === s.gun_ammo ? " on" : ""));
        const img = make("img");
        img.src = "art/gear/ammo_" + a.id + ".png";
        img.alt = "";
        d.title = copy["ammo_" + a.id + "_about"];
        d.append(img, make("span", "p-name", copy["ammo_" + a.id]), make("strong", "", String(a.n)));
        return d;
      }),
    );
    el("ws-cats").replaceChildren(
      ...cats.map((cat) => {
        const box = make("div", "cat");
        box.append(make("div", "label", copy["ws_slot_" + cat]));
        const row = make("div", "parts");
        for (const p of s.stash.filter((q) => q.cat === cat)) {
          row.append(
            partTile(p, s, (id) => {
              game.equip(id);
              changed();
            }),
          );
        }
        box.append(row);
        return box;
      }),
    );
  }

  function missionCard(m, i, pick, onPick) {
    const b = make("button", "mission" + (i === pick ? " on" : ""));
    b.type = "button";
    const img = make("img");
    img.src = "art/missions/" + m.id + ".png";
    img.alt = "";
    b.append(img, make("span", "m-name", copy["mission_" + m.id + "_name"]), make("span", "m-client", copy["mission_" + m.id + "_client"]));
    b.addEventListener("click", () => onPick(i));
    return b;
  }

  function detail(prefix, m) {
    el(prefix + "-art").src = "art/missions/" + m.id + ".png";
    el(prefix + "-client").textContent = copy["mission_" + m.id + "_client"];
    el(prefix + "-name").textContent = copy["mission_" + m.id + "_name"];
    el(prefix + "-brief").textContent = copy["mission_" + m.id + "_brief"];
  }

  function board(s) {
    const pickIt = (i) => {
      game.pick_mission(i);
      changed();
    };
    el("b-list").replaceChildren(...s.missions.map((m, i) => missionCard(m, i, s.pick, pickIt)));
    const m = s.missions[s.pick];
    detail("b", m);
    el("b-entrance").textContent = copy["entrance_" + m.entrance];
    el("b-pay").textContent = fill(copy.pay, { n: m.pay });
  }

  function lobby(s) {
    const l = s.lobby;
    if (!l) return;
    el("l-room").textContent = l.room || "—";
    el("l-status").textContent = copy[l.status] || "";
    el("l-seats").replaceChildren(
      ...l.seats.map((seat) => {
        const d = make("div", "seat" + (seat.you ? " you" : ""));
        const dot = make("span", "dot");
        dot.style.background = seat.colour;
        d.append(dot, make("strong", "", copy["lobby_seat_" + seat.kind] + " " + seat.seat + (seat.you ? " (" + copy.lobby_you + ")" : "")));
        const parts = make("div", "parts-row");
        for (const id of seat.gun) {
          const img = make("img");
          img.src = "art/parts/" + id + ".png";
          img.alt = copy["part_" + id];
          img.title = img.alt;
          parts.append(img);
        }
        d.append(parts);
        const worn = make("div", "parts-row");
        for (const id of seat.gear || []) {
          const img = make("img");
          img.src = "art/gear/" + id + ".png";
          img.alt = copy["gear_" + id];
          img.title = img.alt;
          worn.append(img);
        }
        d.append(worn);
        return d;
      }),
    );
    detail("l", s.missions[l.mission]);
    el("l-picker").hidden = !l.can_start;
    if (l.can_start) {
      el("l-picker").replaceChildren(
        ...s.missions.map((m, i) =>
          missionCard(m, i, l.mission, (k) => {
            game.pick_mission(k);
            changed();
          }),
        ),
      );
    }
    el("l-start").hidden = !l.can_start;
    el("invite-box").hidden = l.role !== "host";
  }

  function results(s) {
    const r = s.results;
    if (!r) return;
    const mine = r.lives[r.you];
    const key = r.aborted ? "aborted" : r.success ? "success" : mine === "extracted" ? "back" : "lost";
    const titles = { aborted: copy.r_aborted, success: copy.r_success, back: copy.r_back, lost: copy.r_lost_title };
    el("r-kicker").textContent = copy["mission_" + r.mission + "_name"] || "";
    el("r-title").textContent = titles[key];
    el("r-text").textContent = copy["r_text_" + key];
    el("results").dataset.outcome = key;
    el("r-team").replaceChildren(
      ...r.lives.map((life, i) => {
        const d = make("div", "seat" + (i === r.you ? " you" : ""));
        const dot = make("span", "dot");
        dot.style.background = r.colours[i];
        d.append(dot, make("strong", "", copy["life_" + life]), make("span", "fine", fill(copy.kills, { n: r.kills[i] })));
        return d;
      }),
    );
    const row = (id, list) => {
      el(id).replaceChildren(...(list.length ? list.map((it) => itemIcon(it, copy)) : [make("span", "fine", copy.r_none)]));
    };
    row("r-found", r.found);
    row("r-pay", r.pay);
    row("r-lost", r.lost);
  }

  function help() {
    el("help-rows").replaceChildren(
      ...copy.help_rows.map(([k, v]) => {
        const d = make("div", "help-row");
        d.append(make("kbd", "", k), make("span", "", v));
        return d;
      }),
      make("p", "fine", copy.help_text),
      make("div", "label", copy.help_drones),
    );
    el("help-drones").replaceChildren(
      ...["wasp", "moth", "hornet", "warden", "tick", "strider"].map((id) => {
        const d = make("div", "drone");
        const img = make("img");
        img.src = "art/drones/" + id + ".png";
        img.alt = "";
        d.append(img, make("strong", "", copy["drone_" + id]), make("span", "fine", copy["drone_" + id + "_about"]));
        return d;
      }),
    );
  }

  return {
    help,
    draw(s) {
      if (s.mode === "workshop") workshop(s);
      if (s.mode === "board") board(s);
      if (s.mode === "lobby") lobby(s);
      if (s.mode === "results") results(s);
      if (s.mode === "title") el("t-record").textContent = fill(copy.record, s.record);
      el("sens-n").textContent = s.sens + "%";
    },
  };
}
