#!/usr/bin/env python3
"""Four browsers play one run online, introduced by pasted codes.

After Vagrancy's testing/online.py. `?ice=none` drops the STUN servers, so the
browsers connect on host candidates alone and no request leaves the origin;
the room-code path (Trystero over public relays) and STUN are checked by hand
(HANDOFF.md). The host lets in three joiners one at a
time: an invitation from the host, a reply from the joiner. The host picks
an order and starts; everyone opens the van doors and walks out, some
turning and firing, so each copy predicts the others and rolls back.

It fails on: a joiner never reaching the lobby, a page error or console
error, a request off the origin, any browser showing the copies disagree,
or the four copies' hashes of the same confirmed ticks not matching.

    python3 tests/browser/online.py [chromium firefox ...]   (default: chromium)
"""
import functools
import http.server
import json
import pathlib
import sys
import threading

from playwright.sync_api import sync_playwright

ROOT = pathlib.Path(__file__).resolve().parents[2]
DIST = ROOT / "dist"
COPY = json.loads((ROOT / "data" / "copy.json").read_text())
failures = []


def check(ok, what):
    print(("  PASS " if ok else "  FAIL ") + what)
    if not ok:
        failures.append(what)


class Quiet(http.server.SimpleHTTPRequestHandler):
    extensions_map = {**http.server.SimpleHTTPRequestHandler.extensions_map, ".wasm": "application/wasm", ".js": "text/javascript"}

    def log_message(self, *a):
        pass


def serve():
    srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(Quiet, directory=str(DIST)))
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    return srv


def run(p, engine, origin):
    print(f"\n[{engine}] online, four browsers")
    # Browsers hide a machine's own addresses behind mDNS names, which a CI
    # runner may not resolve; the four browsers here share one machine, so
    # plain local addresses are what lets them meet without STUN.
    options = {
        "chromium": {"args": ["--disable-features=WebRtcHideLocalIpsWithMdns"]},
        "firefox": {"firefox_user_prefs": {"media.peerconnection.ice.obfuscate_host_addresses": False}},
    }.get(engine, {})
    browser = getattr(p, engine).launch(**options)
    problems, offsite = [], []
    pages = []
    for who in ["host", "joiner 1", "joiner 2", "joiner 3"]:
        ctx = browser.new_context()
        page = ctx.new_page()
        page.on("console", lambda m, w=who: problems.append(f"{w}: {m.text}") if m.type == "error" else None)
        page.on("pageerror", lambda e, w=who: problems.append(f"{w}: {e}"))
        page.on("request", lambda r: None if r.url.startswith((origin, "blob:", "data:")) else offsite.append(r.url))
        page.goto(origin + "/index.html?ice=none")
        page.wait_for_selector("body[data-ready='true']", state="attached", timeout=20000)
        pages.append(page)
    host = pages[0]
    host.click("#t-host-code")
    host.wait_for_selector("body[data-mode='lobby']", state="attached", timeout=10000)

    for k, joiner in enumerate(pages[1:], start=1):
        host.evaluate("delete document.body.dataset.invite")
        host.click("#new-invite")
        host.wait_for_selector("body[data-invite='ready']", state="attached", timeout=30000)
        invite = host.input_value("#invite-out")
        joiner.click("#t-join")
        joiner.click("#join details summary")
        joiner.fill("#invite-in", invite)
        joiner.click("#make-reply")
        joiner.wait_for_selector("body[data-reply='ready']", state="attached", timeout=15000)
        reply = joiner.input_value("#reply-out")
        host.fill("#reply-in", reply)
        host.click("#accept-reply")
        try:
            joiner.wait_for_selector("body[data-mode='lobby']", state="attached", timeout=15000)
            joiner.locator("#l-status", has_text=COPY["online_waiting"]).wait_for(timeout=15000)
            joiner.wait_for_selector("#l-seats .seat.you", state="attached", timeout=15000)
            ok = True
        except Exception:
            ok = False
        check(ok, f"joiner {k} reaches the lobby")
        joiner.evaluate("document.getElementById('join').hidden = true")
        mine = joiner.query_selector_all("#l-seats .seat.you")
        check(len(mine) == 1, f"joiner {k} sees one seat as its own ({len(mine)})")
    host.wait_for_timeout(500)
    seats = host.query_selector_all("#l-seats .seat")
    check(len(seats) == 4, f"the host's lobby has four raiders ({len(seats)})")

    # the host picks the power order; every lobby shows it
    host.click("#l-picker .mission:nth-child(5)")
    host.wait_for_timeout(800)
    names = [pg.inner_text("#l-name") for pg in pages]
    check(len(set(names)) == 1 and "Lights" in names[0], f"every lobby shows the order the host picked {names}")

    host.click("#l-start")
    for page in pages:
        page.wait_for_selector("body[data-mode='play']", state="attached", timeout=15000)
    check(True, "every browser is in the van")

    # everyone holds E (the doors), then walks out, turning; host and one
    # joiner fire, so drones and shots are in every copy
    for page in pages:
        page.keyboard.down("KeyE")
    pages[0].wait_for_timeout(1500)
    for i, page in enumerate(pages):
        page.keyboard.up("KeyE")
        page.keyboard.down("KeyW")
        if i % 2 == 0:
            page.keyboard.down("KeyD")
    for _ in range(12):
        for i, page in enumerate(pages):
            if i < 2:
                page.mouse.move(200 + 30 * i, 200)
        pages[0].wait_for_timeout(1000)
    for page in pages:
        page.keyboard.up("KeyW")
        page.keyboard.up("KeyD")
    pages[0].wait_for_timeout(6000)
    for page in pages:
        n = json.loads(page.evaluate("() => window.__hud")).get("net")
        print("   ", {k: n[k] for k in ("tick", "confirmed", "upto", "stalls", "rollbacks", "ahead")})
    lines = [page.inner_text("#netline") for page in pages]
    print("   ", lines)
    check(all(COPY["hud_desync"] not in t for t in lines), "no browser reports the copies disagree")
    sums = [page.evaluate("() => JSON.parse(window.__hud || 'null')") for page in pages]
    ticks = [set(s.get("net", {}).get("sums", [])) if s else set() for s in sums]
    common = set.intersection(*ticks) if all(ticks) else set()
    tick_of = lambda pair: pair.split(":")[0]
    shared_ticks = set.intersection(*[{tick_of(p) for p in t} for t in ticks]) if all(ticks) else set()
    check(bool(shared_ticks), f"the four copies hashed some of the same ticks {[sorted(t) for t in ticks]}")
    agree = all({p for p in t if tick_of(p) in shared_ticks} == {p for p in ticks[0] if tick_of(p) in shared_ticks} for t in ticks)
    check(agree and bool(common), "and every hash of a shared tick is the same in all four")
    doors = [page.inner_text("#prompt-text") if page.is_visible("#prompt") else "" for page in pages]
    check(all("doors" not in d.lower() for d in doors), f"the van doors are open in every copy {doors}")
    check(not problems, f"no page or console error {problems[:3]}")
    check(not offsite, f"no request off the origin {offsite[:3]}")
    browser.close()


def main():
    if not (DIST / "index.html").exists():
        sys.exit("dist/ is missing: run scripts/build.sh first")
    engines = sys.argv[1:] or ["chromium"]
    srv = serve()
    origin = "http://127.0.0.1:%d" % srv.server_address[1]
    with sync_playwright() as p:
        for engine in engines:
            run(p, engine, origin)
    srv.shutdown()
    if failures:
        print(f"\nONLINE CHECK FAILED: {len(failures)} problem(s)")
        for f in failures:
            print("  - " + f)
        sys.exit(1)
    print(f"\nONLINE CHECK PASSED in {', '.join(engines)}")


if __name__ == "__main__":
    main()
