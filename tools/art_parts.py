"""Raiders of the Lost Mart's thirty weapon-part icons (web/art/parts/*.png),
drawn as TikZ and compiled, never generated as pixels. tools/art.py builds
them (`python3 tools/art.py parts`); the helpers and palette are in
tools/artlib.py. The target is Ribbit King's icons (NickHGame/web/art/icons):
bold plum ink, flat fills, one shade, a glint.

Each icon is a 6 cm square: a rounded tile in its category's colour with
one part on it, drawn salvaged and home-made (bolts, tape, welds,
mismatched paint).

Facts each picture must get right (the things an image generator gets
wrong), and where this file enforces each:

- no words, letters or digits in any picture; the page sets every word.
  Nothing here emits a \\node or any text: every mark is a path, and
  art.py's `check_no_text` refuses a .tex that contains \\node, \\text or a
  label before it is compiled. The lightning on `parts/shock` is a filled
  zigzag polygon (`ZAP`), not a glyph.
- the six categories read apart: every icon sits on a rounded tile whose
  fill is its category's colour, and the part is clipped to the tile so
  nothing spills onto the transparent corners (`CATEGORY` and `TILE`: frames wallD, barrels
  wallC, mounts sage, payloads teal, sights wallB, grips greenA). `icon`
  is the only place a tile is drawn, and `PICTURES` is built from
  `CATEGORY`, so a part cannot sit on the wrong tile; the build asserts
  every part name appears exactly once and the six categories hold
  5 parts each (30 in all).
- side view, muzzle to the right: frames, barrels, payloads, sights and
  grips are drawn with the firing end at larger x (each function's
  coordinates; barrels end in a muzzle face ellipse at the right).
- mounts are drawn from the front, looking down the bores, and show the
  barrel arrangement exactly: `MOUNTS` lists the socket centres and
  `mount` draws one socket per entry; `check_mounts` asserts single has 1,
  pair 2 side by side (same y), stack 2 one above the other (same x),
  square 4 in a 2x2 (two xs by two ys), gatling 6 equally spaced on one
  ring around the axle; that no two sockets overlap; and that every
  socket lies inside its plate. Bolts are drawn far smaller than sockets
  (BOLT_R < SOCKET_R / 4, asserted) so they are never counted as sockets.
- payload colours: slug brass (gold), blast a red shell, goo a glass shell
  of goo/goodark, shock a cell with a cyan bolt, ember an orange shell;
  the scanner's screen shows a drone outline in `glow`.
- plum ink (#3B2340) outlines every filled shape: every shape goes through
  `solid` or the artlib helpers, which use the `inked` style. Glints,
  shading, beams and glows are light, not shapes: unoutlined.
- light from the top left: `solid` darkens each shape's lower-right edge
  (one shade, the fill mixed with ink) and glints sit at upper left.
- repeatable: the only randomness (`wood_grain`) comes from a
  random.Random seeded by the picture's name.
"""
import math, random

from artlib import P, f, circle, ellipse, poly, line, curve, tube, place, clipped, glint, rot
import artlib

W, H = 6.0, 6.0

CATEGORY = {
    "frames": ("wallD", ["pistol", "smg", "ak", "pump", "heavy"]),
    "barrels": ("wallC", ["short", "rifle", "double", "rocket", "lobber"]),
    "mounts": ("sage", ["single", "pair", "stack", "square", "gatling"]),
    "payloads": ("teal", ["slug", "blast", "goo", "shock", "ember"]),
    "sights": ("wallB", ["iron", "dot", "scope", "laser", "scanner"]),
    "grips": ("greenA", ["wood", "fore", "stock", "skeleton", "brace"]),
}
TILE = {part: colour for colour, parts in CATEGORY.values() for part in parts}

BORE = "ink!88!metaldark"
STEEL = "metal"
BLUED = "metaldark!85!ink"


# ---------------------------------------------------------------- helpers

def R(x0, y0, x1, y1):
    return f"{P(x0, y0)} rectangle {P(x1, y1)}"


def C(x, y, r):
    return f"{P(x, y)} circle ({f(r)})"


def E(x, y, rx, ry):
    return f"{P(x, y)} ellipse ({f(rx)} and {f(ry)})"


def L(points):
    return " -- ".join(P(*p) for p in points) + " -- cycle"


def solid(path, fill, d=0.13, opts="", shade=None):
    """A shape with one shade: its lower-right edge in the fill mixed with
    ink (light from the top left), outlined in ink."""
    shade = shade or f"{fill}!70!ink"
    o = f",{opts}" if opts else ""
    s = f"\\begin{{scope}}\\clip[{opts}] {path};\n" if opts else f"\\begin{{scope}}\\clip {path};\n"
    s += f"\\fill[{shade}{o}] {path};\n"
    s += f"\\fill[{fill}{o}, shift={{({f(-d * 0.35)},{f(d)})}}] {path};\n"
    s += "\\end{scope}\n"
    s += f"\\draw[inked{o}] {path};\n"
    return s


def box(x0, y0, x1, y1, fill, r=0.12, d=0.13):
    return solid(R(x0, y0, x1, y1), fill, d, f"rounded corners={f(r)}cm" if r else "")


def disc(x, y, r, fill, d=None):
    return solid(C(x, y, r), fill, d if d is not None else r * 0.22)


def streak(x0, y0, x1, y1, op=0.55):
    """A glint along a surface: an unoutlined translucent white bar."""
    return f"\\fill[white, opacity={f(op)}, rounded corners=0.05cm] {R(x0, y0, x1, y1)};\n"


BOLT_R = 0.1


def bolt(x, y, r=BOLT_R, fill="metallight"):
    s = circle(x, y, r, fill, ",line width=1.1pt")
    s += line((x - r * 0.55, y - r * 0.55), (x + r * 0.55, y + r * 0.55), lw=0.9)
    return s


def tape(cx, cy, w, h, a=0, fill="stone"):
    """A strip of tape with torn (zigzag) ends, turned a degrees."""
    n = 4
    pts = [(-w / 2, -h / 2), (w / 2, -h / 2)]
    for k in range(1, n):
        pts.append((w / 2 + (0.07 if k % 2 else 0), -h / 2 + h * k / n))
    pts += [(w / 2, h / 2), (-w / 2, h / 2)]
    for k in range(n - 1, 0, -1):
        pts.append((-w / 2 - (0.07 if k % 2 else 0), -h / 2 + h * k / n))
    pts = [rot(p, a) for p in pts]
    pts = [(cx + x, cy + y) for x, y in pts]
    s = poly(pts, fill)
    s += f"\\fill[white, opacity=0.35] {L([(cx + x, cy + y) for x, y in [rot(p, a) for p in [(-w / 2 + 0.05, h / 2 - 0.1), (w / 2 - 0.05, h / 2 - 0.1), (w / 2 - 0.05, h / 2 - 0.04), (-w / 2 + 0.05, h / 2 - 0.04)]]])};\n"
    return s


def weld(points):
    """A weld bead: a row of small overlapping scallops."""
    s = ""
    for x, y in points:
        s += circle(x, y, 0.07, "metallight", ",line width=0.9pt")
    return s


def guard(x0, y0, x1, y1, w=0.13):
    """A trigger guard: a loop under the receiver, and the trigger in it."""
    mid = (x0 + x1) / 2
    s = tube([[(x0, y0), (x0 + 0.1, y0 - 0.45), (mid, y0 - 0.62), (x1 - 0.05, y0 - 0.35), (x1, y1)]], w, BLUED)
    s += leaf_((mid - 0.05, y0 + 0.02), (mid - 0.18, y0 - 0.4), 0.09, "metaldark")
    return s


def leaf_(b, t, w, fill):
    return artlib.leaf(b, t, w, fill)


def wood_grain(x0, y0, x1, y1, rng, n=3, colour="wooddark"):
    s = ""
    for k in range(n):
        y = y0 + (y1 - y0) * (k + 0.6) / (n + 0.2)
        xa = x0 + rng.uniform(0.1, 0.5)
        xb = x1 - rng.uniform(0.1, 0.6)
        wob = rng.uniform(-0.08, 0.08)
        s += curve([(xa, y), ((xa + xb) / 2, y + wob), (xb, y - wob * 0.5)], lw=1.0, colour=colour)
    return s


def flame(x, y, h, w, a=0, outer="orange", inner="gold"):
    """A small flame standing on (x, y), turned a degrees."""
    def shape(h, w, dy):
        pts = [(-w, 0.3 * h), (-w * 0.95, -0.05 * h), (w * 0.95, -0.05 * h), (w, 0.3 * h),
               (w * 0.55, 0.62 * h), (0.15 * w, h), (-0.35 * w, 0.6 * h)]
        pts = [rot((px, py + dy), a) for px, py in pts]
        return [(x + px, y + py) for px, py in pts]
    s = artlib.blob(shape(h, w, 0), outer, tension=0.55)
    s += f"\\fill[{inner}] {artlib.blob_path(shape(h * 0.55, w * 0.5, -0.02 * h), 0.55)};\n"
    return s


def rail(x0, x1, y, h=0.38):
    """A picatinny rail clamp: a bar with slots."""
    s = box(x0, y - h, x1, y, BLUED, r=0.06, d=0.08)
    k = x0 + 0.3
    while k < x1 - 0.2:
        s += line((k, y - 0.05), (k, y - h + 0.08), lw=1.4, colour="metal")
        k += 0.36
    return s


def muzzle(x, y, ry, rim=STEEL, rx=None):
    """The open end of a barrel, facing right: a rim and a dark bore."""
    rx = rx if rx is not None else ry * 0.38
    s = ellipse(x, y, rx, ry, rim)
    s += f"\\fill[{BORE}] {E(x + rx * 0.12, y, rx * 0.62, ry * 0.62)};\n"
    return s


TILE_PATH = f"[rounded corners=0.75cm] {R(0.12, 0.12, 5.88, 5.88)}"


def icon(name, body):
    """The category's tile, the part clipped to it, and the tile's ink
    edge drawn last so nothing spills past the rounded corners."""
    s = f"\\fill[{TILE[name]}] {TILE_PATH};\n"
    s += clipped(TILE_PATH, body)
    s += f"\\draw[inked] {TILE_PATH};\n"
    return s, (W, H)


# ---------------------------------------------------------------- frames

def pistol():
    s = ""
    # grip stub, raked back, taped
    s += solid(L([(1.2, 3.3), (2.45, 3.3), (2.1, 1.35), (0.95, 1.4)]), "wood", opts="rounded corners=0.18cm")
    for k in range(4):
        y = 1.75 + k * 0.33
        s += line((1.15 + (y - 1.4) * 0.12, y), (1.95 + (y - 1.4) * 0.17, y), lw=1.0, colour="wooddark")
    s += tape(1.6, 2.35, 1.25, 0.3, a=-4)
    s += guard(2.45, 3.0, 3.45, 3.0)
    # frame (repainted green) and slide (bare steel)
    s += box(1.25, 2.9, 4.3, 3.5, "greenE", r=0.1)
    s += box(0.85, 3.4, 4.65, 4.4, STEEL, r=0.14)
    for k in range(5):
        x = 1.15 + k * 0.17
        s += line((x, 3.55), (x, 4.2), lw=1.0, colour="metaldark")
    s += poly([(0.85, 4.05), (0.5, 4.35), (0.6, 4.55), (1.05, 4.4)], BLUED)
    s += box(1.3, 4.4, 1.7, 4.62, BLUED, r=0.04, d=0.05)
    s += box(4.15, 4.4, 4.4, 4.6, BLUED, r=0.04, d=0.05)
    s += box(2.3, 3.6, 3.4, 4.05, BORE, r=0.08, d=0.0)
    s += streak(0.95, 4.18, 4.4, 4.27)
    # the barrel socket, bolted on
    s += box(4.62, 3.5, 5.3, 4.3, BLUED, r=0.08)
    s += bolt(4.95, 3.9)
    s += bolt(3.75, 3.82)
    s += bolt(2.0, 3.2)
    return s


def smg():
    s = ""
    # stick magazine
    s += solid(L([(2.6, 3.05), (3.45, 3.05), (3.6, 0.8), (2.8, 0.8)]), BLUED, opts="rounded corners=0.08cm")
    for k in range(3):
        y = 1.3 + k * 0.5
        s += line((2.85, y), (3.4, y), lw=1.0, colour="metal")
    s += tape(3.17, 1.85, 1.15, 0.36, a=4, fill="red")
    # grip
    s += solid(L([(0.95, 3.05), (1.8, 3.05), (1.5, 1.4), (0.7, 1.45)]), "armour", opts="rounded corners=0.15cm")
    s += guard(1.8, 3.0, 2.55, 3.0)
    # receiver: steel rear, a front shroud painted sage
    s += box(0.6, 3.0, 3.6, 4.5, STEEL, r=0.12)
    s += box(3.45, 3.15, 5.05, 4.35, "sage", r=0.1)
    for k in range(3):
        x = 3.75 + k * 0.42
        s += box(x, 3.5, x + 0.22, 4.0, BORE, r=0.08, d=0)
    s += weld([(3.45 + 0.0, 3.25 + k * 0.14) for k in range(7)])
    # cocking knob and slot
    s += box(1.0, 4.05, 2.7, 4.22, BORE, r=0.06, d=0)
    s += circle(1.35, 4.62, 0.17, "metallight")
    s += line((1.35, 4.45), (1.35, 4.22), lw=2.2)
    s += streak(0.75, 4.28, 3.4, 4.37)
    s += bolt(0.9, 3.35)
    s += bolt(3.2, 3.35)
    s += box(5.02, 3.4, 5.45, 4.1, BLUED, r=0.08)
    s += bolt(5.23, 3.75, r=0.08)
    return s


def ak():
    s = ""
    rng = random.Random("parts/ak")
    # wooden stock stub
    s += solid(L([(0.35, 2.25), (1.6, 3.15), (1.6, 4.0), (0.35, 3.6)]), "wood", opts="rounded corners=0.08cm")
    s += wood_grain(0.5, 2.9, 1.5, 3.6, rng, n=2)
    s += box(0.25, 2.2, 0.5, 3.65, BLUED, r=0.05, d=0.05)
    # wooden grip
    s += solid(L([(1.8, 3.15), (2.45, 3.15), (2.15, 1.65), (1.5, 1.7)]), "wood", opts="rounded corners=0.14cm")
    s += guard(2.45, 3.05, 2.95, 3.05)
    # banana magazine, curving forward, in orange bakelite
    mag = "(2.85,3.15) .. controls (2.8,2.0) and (3.15,1.25) .. (3.65,0.7) -- (4.4,1.0) .. controls (3.85,1.6) and (3.6,2.3) .. (3.6,3.15) -- cycle"
    s += solid(mag, "orange!80!wooddark")
    for t in (0.35, 0.6):
        s += f"\\draw[orange!55!ink, line width=1.0pt, line cap=round] {P(2.95 + t * 0.6, 3.0 - t * 1.9)} -- {P(3.6 + t * 0.35, 3.0 - t * 1.75)};\n"
    # receiver and ribbed dust cover
    s += box(1.5, 3.1, 4.25, 4.05, STEEL, r=0.08)
    s += box(1.6, 4.0, 3.95, 4.35, "metal!80!ink", r=0.12, d=0.06)
    for k in range(4):
        s += line((2.0 + k * 0.5, 4.08), (2.0 + k * 0.5, 4.28), lw=1.0, colour="metaldark")
    s += line((2.7, 3.75), (3.35, 3.55), lw=2.4)
    for x in (1.75, 2.25, 3.9, 4.05):
        s += f"\\fill[ink] {C(x, 3.3, 0.045)};\n"
    s += streak(1.6, 3.85, 4.15, 3.94)
    # wood handguard and gas tube
    s += box(4.25, 4.0, 5.35, 4.3, STEEL, r=0.1, d=0.06)
    s += solid(R(4.25, 3.15, 5.25, 3.95), "woodlight", opts="rounded corners=0.18cm")
    s += line((4.5, 3.4), (5.0, 3.4), lw=1.0, colour="wooddark")
    s += line((4.5, 3.65), (5.0, 3.65), lw=1.0, colour="wooddark")
    s += box(5.22, 3.3, 5.55, 3.85, BLUED, r=0.06, d=0.06)
    return s


def pump():
    s = ""
    rng = random.Random("parts/pump")
    # grip
    s += solid("(1.45,3.2) -- (2.15,3.2) .. controls (1.75,2.4) and (1.6,1.9) .. (1.55,1.4) -- (0.75,1.5) .. controls (0.85,2.2) and (1.1,2.8) .. (1.45,3.2) -- cycle", "wood")
    s += wood_grain(0.95, 1.7, 1.75, 2.6, rng, n=2)
    s += guard(2.1, 3.1, 2.85, 3.1)
    # receiver, blued, with a red shell in the ejection port
    s += box(1.3, 3.05, 3.55, 4.45, BLUED, r=0.12, d=0.15)
    s += box(1.75, 3.65, 3.05, 4.15, BORE, r=0.08, d=0)
    s += box(2.0, 3.72, 2.75, 4.08, "red", r=0.05, d=0.06)
    s += box(2.75, 3.72, 2.95, 4.08, "gold", r=0.03, d=0.04)
    s += streak(1.45, 4.22, 3.4, 4.31, op=0.45)
    s += bolt(1.55, 3.3)
    s += bolt(3.3, 3.3)
    # barrel socket above, tube magazine below, wooden pump on the tube
    s += box(3.5, 3.65, 4.15, 4.35, STEEL, r=0.06)
    s += line((3.85, 3.7), (3.85, 4.3), lw=1.1, colour="metaldark")
    s += box(3.5, 2.75, 5.5, 3.25, STEEL, r=0.22, d=0.08)
    s += solid(R(4.05, 2.45, 5.0, 3.55), "wood", opts="rounded corners=0.25cm")
    for k in range(4):
        x = 4.25 + k * 0.19
        s += line((x, 2.6), (x, 3.4), lw=1.1, colour="wooddark")
    s += box(5.2, 2.65, 5.55, 3.35, BLUED, r=0.1, d=0.06)
    return s


def heavy():
    s = ""
    # carry handle
    s += tube([[(1.85, 4.3), (2.05, 5.05), (3.6, 5.05), (3.8, 4.3)]], 0.24, BLUED, tension=0.4)
    # drum box under the receiver, olive, with a latch
    s += box(2.15, 0.75, 4.1, 2.95, "greenD", r=0.16, d=0.16)
    s += line((2.25, 2.45), (4.0, 2.45), lw=1.2)
    s += box(2.9, 2.3, 3.35, 2.6, "metallight", r=0.04, d=0.05)
    s += f"\\fill[gold] {R(2.4, 1.2, 3.85, 1.42)};\n"
    s += f"\\draw[inked] {R(2.4, 1.2, 3.85, 1.42)};\n"
    # grip
    s += solid(L([(1.0, 2.9), (1.7, 2.9), (1.45, 1.4), (0.75, 1.45)]), "armourdark", opts="rounded corners=0.15cm")
    s += guard(1.7, 2.85, 2.15, 2.85)
    # chunky receiver, buffer plate and a welded patch
    s += box(0.5, 2.95, 0.95, 4.25, BLUED, r=0.08, d=0.06)
    s += box(0.9, 2.8, 4.75, 4.4, "armour", r=0.16, d=0.16)
    s += solid(R(1.25, 3.25, 2.4, 4.0), "wallE")
    s += weld([(1.25 + k * 0.16, 3.25) for k in range(8)])
    s += bolt(1.45, 3.8)
    s += bolt(2.2, 3.8)
    for k in range(3):
        x = 3.0 + k * 0.45
        s += box(x, 3.25, x + 0.22, 4.0, BORE, r=0.08, d=0)
    s += streak(1.05, 4.14, 4.6, 4.24)
    s += box(4.72, 3.1, 5.45, 4.1, BLUED, r=0.08)
    s += bolt(5.08, 3.35, r=0.08)
    s += bolt(5.08, 3.85, r=0.08)
    return s


# ---------------------------------------------------------------- barrels

def short():
    s = ""
    s += box(1.1, 2.15, 2.4, 3.85, BLUED, r=0.12, d=0.15)
    s += box(1.45, 1.65, 2.0, 2.2, BLUED, r=0.06, d=0.08)
    s += circle(1.73, 1.88, 0.11, "metallight", ",line width=1.0pt")
    s += box(2.35, 2.45, 4.05, 3.55, STEEL, r=0.0, d=0.2)
    s += streak(2.4, 3.3, 3.95, 3.42)
    s += tape(3.15, 3.0, 0.5, 1.25, fill="stone")
    s += box(3.95, 2.3, 4.55, 3.7, "metal!70!ink", r=0.1, d=0.1)
    s += muzzle(4.6, 3.0, 0.5)
    s += bolt(1.75, 3.4)
    s += bolt(1.75, 2.6)
    return s


def rifle():
    s = ""
    s += box(0.35, 2.4, 1.35, 3.6, BLUED, r=0.12, d=0.15)
    s += solid(L([(1.3, 2.62), (4.3, 2.72), (4.3, 3.28), (1.3, 3.38)]), STEEL, d=0.15)
    s += line((1.6, 3.12), (3.9, 3.12), lw=1.3, colour="metaldark")
    s += line((1.6, 2.9), (3.9, 2.9), lw=1.3, colour="metaldark")
    s += streak(1.45, 3.22, 4.1, 3.3)
    s += box(2.4, 3.3, 2.75, 3.6, BLUED, r=0.04, d=0.05)
    s += box(4.2, 2.5, 5.35, 3.5, "metal!65!ink", r=0.1, d=0.1)
    for k in range(3):
        x = 4.38 + k * 0.3
        s += box(x, 2.7, x + 0.14, 3.3, BORE, r=0.06, d=0)
    s += muzzle(5.4, 3.0, 0.32)
    s += bolt(0.85, 3.0)
    return s


def double():
    s = ""
    # breech block with its hinge
    s += box(0.5, 1.85, 1.55, 4.25, BLUED, r=0.14, d=0.16)
    s += circle(1.0, 2.15, 0.18, "metallight")
    s += f"\\fill[ink] {C(1.0, 2.15, 0.06)};\n"
    s += box(0.8, 1.45, 1.25, 1.9, BLUED, r=0.06, d=0.06)
    # far barrel (higher, seen from slightly above), then the near one
    for y0, y1, shade in ((3.05, 3.95, STEEL), (2.05, 2.95, "metal!92!white")):
        s += box(1.5, y0, 4.7, y1, shade, r=0.0, d=0.16)
        s += streak(1.6, y1 - 0.24, 4.55, y1 - 0.14)
        s += muzzle(4.75, (y0 + y1) / 2, 0.48, rim="metal!75!ink", rx=0.24)
    s += box(1.5, 3.93, 4.6, 4.08, BLUED, r=0.04, d=0)
    s += circle(4.45, 4.15, 0.09, "gold", ",line width=1.0pt")
    s += tape(2.6, 3.0, 0.5, 2.05, fill="stone")
    return s


def rocket():
    s = ""
    # rear venturi, the tube, hazard stripes near the mouth
    s += poly([(0.35, 1.6), (1.0, 2.05), (1.0, 3.95), (0.35, 4.4)], BLUED)
    s += ellipse(0.35, 3.0, 0.16, 1.4, BORE)
    s += box(0.95, 1.95, 4.45, 4.05, "greenD", r=0.0, d=0.22)
    stripes = ""
    for k in range(-3, 8):
        x = 3.1 + k * 0.32
        stripes += f"\\fill[ink] {L([(x, 1.9), (x + 0.16, 1.9), (x + 0.16 + 0.6, 4.1), (x + 0.6, 4.1)])};\n"
    s += f"\\fill[gold] {R(3.2, 1.97, 4.1, 4.03)};\n"
    s += clipped(R(3.2, 1.97, 4.1, 4.03), stripes)
    s += f"\\draw[inked] {R(3.2, 1.97, 4.1, 4.03)};\n"
    s += streak(1.05, 3.7, 3.15, 3.85)
    s += box(1.65, 4.0, 2.0, 4.55, BLUED, r=0.04, d=0.06)
    s += box(1.45, 1.6, 2.15, 1.98, BLUED, r=0.05, d=0.06)
    s += bolt(1.3, 3.0)
    s += tape(2.55, 3.0, 0.45, 2.3, fill="stone")
    # the mouth and the rocket nose peeking out
    s += ellipse(4.45, 3.0, 0.32, 1.08, "metal!70!ink")
    s += f"\\fill[{BORE}] {E(4.48, 3.0, 0.22, 0.86)};\n"
    nose = "(4.4,3.68) .. controls (5.05,3.68) and (5.5,3.3) .. (5.62,3.0) .. controls (5.5,2.7) and (5.05,2.32) .. (4.4,2.32) -- cycle"
    s += solid(nose, "red", d=0.14)
    s += f"\\fill[white, opacity=0.55] (4.6,3.45) .. controls (4.95,3.45) and (5.2,3.3) .. (5.3,3.2) -- (5.2,3.18) .. controls (5.05,3.3) and (4.8,3.35) .. (4.6,3.35) -- cycle;\n"
    s += f"\\draw[ink, line width=1.2pt] (4.75,3.66) .. controls (4.68,3.2) and (4.68,2.8) .. (4.75,2.34);\n"
    return s


def lobber():
    s = ""
    # motion arcs of the lobbed grenade
    for k, r in enumerate((0.9, 1.25)):
        s += f"\\draw[ink, line width=1.3pt, line cap=round] {P(3.65 + r * math.cos(math.radians(150)) * 0.5, 4.25 + k * 0.2)} arc[start angle=150, end angle=95, radius={f(r)}];\n"
    s += box(0.5, 2.05, 1.2, 3.95, BLUED, r=0.12, d=0.12)
    s += circle(0.85, 2.3, 0.14, "metallight", ",line width=1.0pt")
    s += box(1.1, 1.8, 3.85, 4.2, STEEL, r=0.0, d=0.24)
    s += solid(R(2.25, 1.8, 2.8, 4.2), "gold")
    s += streak(1.2, 3.85, 3.75, 3.98)
    s += bolt(1.6, 2.2)
    s += bolt(3.45, 2.2)
    s += muzzle(3.88, 3.0, 1.2, rim="metal!70!ink", rx=0.38)
    # the grenade, leaving the mouth
    gx, gy = 4.75, 4.0
    s += disc(gx, gy, 0.72, "greenB")
    s += clipped(C(gx, gy, 0.72), f"\\filldraw[inked, fill=gold] {R(gx - 0.2, gy - 0.8, gx + 0.08, gy + 0.8)};\n")
    s += ellipse(gx + 0.72, gy, 0.12, 0.28, "metallight")
    s += glint(gx - 0.32, gy + 0.32, 0.13)
    return s


# ---------------------------------------------------------------- mounts

SOCKET_R = 0.6
MOUNTS = {
    # plate (x0, y0, x1, y1) or ("disc", cx, cy, r); socket centres
    "single": ((1.65, 1.8, 4.35, 4.8), [(3.0, 3.3)]),
    "pair": ((0.7, 1.95, 5.3, 4.65), [(1.95, 3.3), (4.05, 3.3)]),
    "stack": ((1.8, 1.25, 4.2, 5.45), [(3.0, 4.35), (3.0, 2.35)]),
    "square": ((1.0, 1.25, 5.0, 5.25), [(2.05, 4.25), (3.95, 4.25), (2.05, 2.25), (3.95, 2.25)]),
    "gatling": (("disc", 3.0, 3.1, 2.45), [(3.0 + 1.48 * math.cos(math.radians(90 + 60 * k)),
                                            3.1 + 1.48 * math.sin(math.radians(90 + 60 * k))) for k in range(6)]),
}
EXPECTED = {"single": 1, "pair": 2, "stack": 2, "square": 4, "gatling": 6}


def check_mounts():
    assert BOLT_R < SOCKET_R / 4
    for name, n in EXPECTED.items():
        plate, socks = MOUNTS[name]
        assert len(socks) == n, f"{name}: {len(socks)} sockets, want {n}"
        r = SOCKET_R * (0.85 if name == "gatling" else 1)
        for i, a in enumerate(socks):
            for b in socks[i + 1:]:
                assert math.dist(a, b) > 2 * r + 0.15, f"{name}: sockets overlap"
            if plate[0] == "disc":
                assert math.dist(a, plate[1:3]) + r < plate[3], f"{name}: socket off the plate"
            else:
                assert plate[0] < a[0] - r and a[0] + r < plate[2] and plate[1] < a[1] - r and a[1] + r < plate[3], f"{name}: socket off the plate"
        xs = {round(x, 3) for x, _ in socks}
        ys = {round(y, 3) for _, y in socks}
        if name == "pair":
            assert len(ys) == 1 and len(xs) == 2, "pair: side by side"
        if name == "stack":
            assert len(xs) == 1 and len(ys) == 2, "stack: one above the other"
        if name == "square":
            assert len(xs) == 2 and len(ys) == 2, "square: 2x2"
        if name == "gatling":
            hub = plate[1:3]
            d = [math.dist(p, hub) for p in socks]
            assert max(d) - min(d) < 1e-6, "gatling: one ring"
            angs = sorted(math.degrees(math.atan2(y - hub[1], x - hub[0])) % 360 for x, y in socks)
            gaps = [(angs[(i + 1) % 6] - angs[i]) % 360 for i in range(6)]
            assert all(abs(g - 60) < 1e-6 for g in gaps), "gatling: equally spaced"


def socket(x, y, r):
    s = disc(x, y, r, STEEL, d=r * 0.2)
    s += circle(x, y, r * 0.6, BORE)
    s += f"\\fill[metaldark] {E(x + r * 0.12, y - r * 0.14, r * 0.35, r * 0.3)};\n"
    s += f"\\fill[{BORE}] {C(x + r * 0.05, y - r * 0.05, r * 0.36)};\n"
    s += f"\\draw[white, opacity=0.6, line width=1.6pt, line cap=round] {P(x - r * 0.78 * math.cos(math.radians(30)), y + r * 0.78 * math.sin(math.radians(30)))} arc[start angle=150, end angle=105, radius={f(r * 0.78)}];\n"
    return s


def mount(name):
    plate, socks = MOUNTS[name]
    s = ""
    if plate[0] == "disc":
        _, cx, cy, pr = plate
        s += box(2.55, 0.35, 3.45, 1.2, BLUED, r=0.08, d=0.08)
        s += disc(cx, cy, pr, "stone", d=0.2)
        s += f"\\draw[stonedark, line width=1.4pt] {C(cx, cy, pr - 0.22)};\n"
        r = SOCKET_R * 0.85
        for k in range(6):
            a = 60 * k
            s += bolt(*rot((cx + pr - 0.4, cy), a, (cx, cy)), r=0.08)
        for k in range(6):
            a = 90 + 60 * k
            s += line(rot((cx + 0.5, cy), a, (cx, cy)), rot((cx + 1.48 - r, cy), a, (cx, cy)), lw=3.2, colour="ink")
            s += line(rot((cx + 0.5, cy), a, (cx, cy)), rot((cx + 1.48 - r, cy), a, (cx, cy)), lw=1.4, colour="metallight")
        for x, y in socks:
            s += socket(x, y, r)
        s += disc(cx, cy, 0.48, BLUED)
        s += poly([rot((cx + 0.26, cy), 30 + 60 * k, (cx, cy)) for k in range(6)], "metallight")
        s += f"\\fill[ink] {C(cx, cy, 0.08)};\n"
        return s
    x0, y0, x1, y1 = plate
    mx = (x0 + x1) / 2
    s += box(mx - 0.45, 0.4, mx + 0.45, y0 + 0.2, BLUED, r=0.08, d=0.08)
    s += circle(mx, 0.75, 0.13, "metallight", ",line width=1.0pt")
    s += box(x0, y0, x1, y1, "stone", r=0.35, d=0.2)
    s += f"\\draw[stonedark, line width=1.4pt, rounded corners=0.22cm] {R(x0 + 0.2, y0 + 0.2, x1 - 0.2, y1 - 0.2)};\n"
    for bx, by in ((x0 + 0.32, y0 + 0.32), (x1 - 0.32, y0 + 0.32), (x0 + 0.32, y1 - 0.32), (x1 - 0.32, y1 - 0.32)):
        s += bolt(bx, by, r=0.09)
    for x, y in socks:
        s += socket(x, y, SOCKET_R)
    if name == "pair":
        s += weld([(3.0, 2.55 + k * 0.15) for k in range(11)])
    if name == "stack":
        s += tape(3.0, 3.35, 2.75, 0.32, a=-6, fill="cream")
    return s


# ---------------------------------------------------------------- payloads

def cartridge(case="gold", tip="orange!55!wood", body_len=2.6, hw=0.55):
    """A round about its own origin pointing along +x: rim, case, neck,
    and a pointed bullet."""
    x0 = -1.9
    s = ""
    s += solid(R(x0 - 0.05, -hw - 0.04, x0 + 0.22, hw + 0.04), f"{case}!80!ink", d=0.08)
    s += solid(L([(x0 + 0.22, -hw), (x0 + body_len, -hw), (x0 + body_len + 0.3, -hw * 0.75),
                  (x0 + body_len + 0.3, hw * 0.75), (x0 + body_len, hw), (x0 + 0.22, hw)]), case, d=0.14)
    bx = x0 + body_len + 0.3
    s += solid(f"{P(bx, -hw * 0.75)} .. controls {P(bx + 0.9, -hw * 0.75)} and {P(bx + 1.2, -0.15)} .. {P(bx + 1.35, 0)} .. controls {P(bx + 1.2, 0.15)} and {P(bx + 0.9, hw * 0.75)} .. {P(bx, hw * 0.75)} -- cycle", tip, d=0.12)
    s += f"\\fill[white, opacity=0.55, rounded corners=0.05cm] {R(x0 + 0.35, hw * 0.35, x0 + body_len - 0.1, hw * 0.65)};\n"
    return s


def slug():
    s = ""
    for k, (dx, dy, l) in enumerate(((-0.1, 0.55, 0.8), (0.25, 0.0, 1.05), (-0.1, -0.55, 0.7))):
        a = rot((-2.3 + dx, dy), 30)
        b = rot((-2.3 + dx - l, dy), 30)
        s += line((3.0 + a[0], 3.0 + a[1]), (3.0 + b[0], 3.0 + b[1]), lw=2.4)
    s += place(3.05, 3.0, 1.0, cartridge(), rot=30)
    return s


def shotshell(hull="red"):
    s = ""
    s += solid(R(-2.05, -0.72, -1.85, 0.72), "gold!80!ink", d=0.08)
    s += solid(R(-1.85, -0.65, -1.05, 0.65), "gold", d=0.12)
    s += solid(R(-1.05, -0.62, 1.1, 0.62), hull, d=0.16)
    for k in range(3):
        y = -0.4 + k * 0.4
        s += line((0.85, y), (1.1, y * 0.6), lw=1.0)
    s += f"\\fill[white, opacity=0.5, rounded corners=0.05cm] {R(-0.9, 0.25, 0.8, 0.42)};\n"
    s += line((-1.3, -0.45), (-1.3, 0.45), lw=1.0, colour="gold!60!ink")
    return s


def burst(cx, cy, ro, ri, n, a0=0, fills=("orange", "gold", "hothi")):
    s = ""
    for k, fill in enumerate(fills):
        sc = 1 - k * 0.3
        pts = []
        for i in range(2 * n):
            r = (ro if i % 2 == 0 else ri) * sc
            a = math.radians(a0 + 180 * i / n)
            pts.append((cx + r * math.cos(a), cy + r * math.sin(a)))
        s += poly(pts, fill, ",line width=1.4pt" if k else "")
    return s


def blast():
    s = burst(4.2, 4.1, 1.55, 0.85, 9, a0=10)
    s += place(2.3, 2.3, 0.95, shotshell(), rot=38)
    return s


def goo():
    s = ""
    # glass shell with brass ends, half full of goo; it drips
    gx0, gx1, gy0, gy1 = 1.35, 4.35, 2.75, 4.2
    s += curve([(3.1, 2.8), (3.15, 2.2), (3.08, 1.75)], lw=7.5, colour="ink")
    s += curve([(3.1, 2.8), (3.15, 2.2), (3.08, 1.75)], lw=4.4, colour="goo")
    s += artlib.blob([(3.08, 1.05), (3.38, 1.4), (3.08, 1.85), (2.78, 1.4)], "goo", tension=0.6)
    s += ellipse(2.95, 0.72, 1.2, 0.22, "goo")
    s += f"\\fill[goodark] {E(3.1, 0.68, 0.7, 0.09)};\n"
    s += artlib.blob([(1.95, 2.3), (2.12, 2.5), (1.95, 2.8), (1.78, 2.5)], "goo", tension=0.6)
    glass = R(gx0, gy0, gx1, gy1)
    s += f"\\fill[white!75!glowhi] {glass};\n"
    inside = f"\\fill[goo] (1.2,2.6) -- (1.2,3.45) .. controls (2.0,3.75) and (2.6,3.2) .. (3.3,3.5) .. controls (3.8,3.7) and (4.2,3.45) .. (4.5,3.4) -- (4.5,2.6) -- cycle;\n"
    inside += f"\\fill[goodark] (1.2,2.6) -- (4.5,2.6) -- (4.5,3.0) .. controls (3.2,2.95) and (2.2,3.05) .. (1.2,2.95) -- cycle;\n"
    for (bx, by, br) in ((2.0, 3.15, 0.12), (2.9, 3.0, 0.08), (3.7, 3.2, 0.1)):
        inside += f"\\draw[goodark, line width=1.1pt] {C(bx, by, br)};\n"
    s += clipped(glass, inside)
    s += f"\\fill[white, opacity=0.7, rounded corners=0.05cm] {R(1.55, 3.82, 4.1, 3.98)};\n"
    s += f"\\draw[inked] {glass};\n"
    s += box(0.75, 2.6, 1.4, 4.35, "gold", r=0.08, d=0.12)
    s += box(0.55, 2.5, 0.8, 4.45, "gold!80!ink", r=0.05, d=0.06)
    s += solid(f"(4.3,2.75) -- (4.3,4.2) .. controls (5.0,4.2) and (5.45,3.7) .. (5.55,3.475) .. controls (5.45,3.25) and (5.0,2.75) .. (4.3,2.75) -- cycle", "gold", d=0.12)
    s += glint(4.6, 3.95, 0.09)
    return s


ZAP = [(0.3, 2.7), (1.0, 2.7), (0.55, 1.6), (1.0, 1.6), (0.0, -0.05), (0.25, 1.25), (-0.2, 1.25)]


def shock():
    s = ""
    # the cell: steel can, cyan bands, a terminal
    cell = ""
    cell += solid(R(-1.7, -0.7, 1.0, 0.7), "metal", d=0.16, opts="rounded corners=0.12cm")
    cell += solid(R(-1.25, -0.7, -0.85, 0.7), "cyan", d=0.1)
    cell += solid(R(0.2, -0.7, 0.6, 0.7), "cyan", d=0.1)
    cell += solid(R(-1.95, -0.5, -1.65, 0.5), BLUED, d=0.06, opts="rounded corners=0.06cm")
    cell += solid(R(0.98, -0.32, 1.35, 0.32), "metallight", d=0.06, opts="rounded corners=0.06cm")
    cell += f"\\fill[white, opacity=0.5, rounded corners=0.05cm] {R(-1.55, 0.32, 0.85, 0.48)};\n"
    s += place(2.3, 2.2, 1.0, cell, rot=35)
    # the bolt: a filled zigzag, with a glow
    tip = (2.3 + 1.45 * math.cos(math.radians(35)), 2.2 + 1.45 * math.sin(math.radians(35)))
    pts = [rot(p, -32) for p in ZAP]
    pts = [(tip[0] + 0.05 + x * 1.0, tip[1] + 0.05 + y * 1.0) for x, y in pts]
    cx = sum(p[0] for p in pts) / len(pts)
    cy = sum(p[1] for p in pts) / len(pts)
    for r, op in ((1.1, 0.12), (0.85, 0.15), (0.6, 0.2)):
        s += f"\\fill[glow, opacity={f(op)}] {C(cx, cy, r)};\n"
    s += poly(pts, "cyan")
    inner = [(cx + (x - cx) * 0.55, cy + (y - cy) * 0.55) for x, y in pts]
    s += f"\\fill[glowhi] {L(inner)};\n"
    # small sparks
    for (a, b) in (((4.45, 2.6), (4.95, 2.35)), ((4.55, 3.0), (5.15, 3.05)), ((2.65, 4.2), (2.4, 4.75))):
        s += line(a, b, lw=2.2, colour="cyan")
    return s


def ember():
    s = ""
    # flames rise from the shell's upper side and trail off its tail
    for lx, h, w in ((-1.7, 1.25, 0.4), (-0.7, 1.05, 0.34), (0.3, 0.85, 0.28)):
        x, y = rot((lx * 0.95, 0.45 * 0.95), 35)
        s += flame(3.0 + x, 2.8 + y, h, w, a=12)
    s += flame(1.05, 1.35, 0.8, 0.3, a=40)
    s += place(3.0, 2.8, 0.95, cartridge(case="orange", tip="red", hw=0.6), rot=35)
    for x, y in ((4.3, 2.05), (4.85, 2.5), (1.0, 3.75), (2.3, 5.25), (3.6, 5.1)):
        s += circle(x, y, 0.09, "hothi", ",line width=1.0pt")
    return s


# ---------------------------------------------------------------- sights

def iron():
    s = rail(0.6, 5.4, 1.85)
    # rear: a notch block, seen from behind so the notch shows
    s += solid("(0.95,1.85) -- (0.95,3.75) -- (1.55,3.75) -- (1.6,3.1) .. controls (1.75,2.85) and (2.05,2.85) .. (2.2,3.1) -- (2.25,3.75) -- (2.85,3.75) -- (2.85,1.85) -- cycle", BLUED, d=0.15)
    s += streak(1.05, 3.5, 1.45, 3.62)
    s += streak(2.35, 3.5, 2.75, 3.62)
    s += bolt(1.9, 2.25)
    # front: a post inside a protective hood
    s += box(3.4, 1.85, 5.2, 2.5, BLUED, r=0.1, d=0.08)
    s += solid("(3.55,2.4) -- (3.55,4.1) .. controls (3.55,4.6) and (5.05,4.6) .. (5.05,4.1) -- (5.05,2.4) -- (4.75,2.4) -- (4.75,4.0) .. controls (4.75,4.25) and (3.85,4.25) .. (3.85,4.0) -- (3.85,2.4) -- cycle", "metal", d=0.1)
    s += box(4.17, 2.45, 4.43, 3.9, "gold", r=0.06, d=0.06)
    s += circle(4.3, 3.9, 0.15, "redlight", ",line width=1.2pt")
    return s


def dot():
    s = rail(0.7, 5.3, 1.75)
    # a reflex box seen from behind and to the left: the rear window shows
    # the glass with its red dot; the housing runs off to the right
    s += solid(L([(2.0, 1.8), (4.9, 2.2), (4.9, 4.45), (2.0, 4.65)]), "greenE", d=0.12)
    s += solid(R(0.95, 1.75, 3.15, 4.75), "metal", opts="rounded corners=0.3cm", d=0.16)
    s += solid(R(1.25, 2.1, 2.85, 4.35), "cyan!22!white", opts="rounded corners=0.18cm", d=0.0)
    s += f"\\fill[white, opacity=0.7] {L([(1.45, 3.4), (1.95, 4.15), (2.15, 4.15), (1.6, 3.35)])};\n"
    s += f"\\fill[red, opacity=0.3] {C(2.05, 3.2, 0.42)};\n"
    s += circle(2.05, 3.2, 0.2, "red", ",line width=1.2pt")
    s += glint(2.0, 3.25, 0.06)
    s += box(3.45, 4.35, 4.15, 4.75, BLUED, r=0.06, d=0.06)
    s += bolt(4.45, 2.75)
    s += tape(4.0, 3.35, 0.45, 1.4, fill="stone")
    return s


def scope():
    s = rail(0.7, 5.3, 1.6)
    for x in (1.6, 3.7):
        s += box(x, 1.55, x + 0.55, 3.0, BLUED, r=0.08, d=0.08)
        s += bolt(x + 0.27, 2.0, r=0.08)
    # eyepiece bell, main tube, objective bell
    s += solid(L([(0.35, 2.45), (1.35, 2.75), (1.35, 3.65), (0.35, 3.95)]), "metaldark", d=0.12, opts="rounded corners=0.06cm")
    s += box(1.3, 2.75, 4.2, 3.65, "metal!85!ink", r=0.0, d=0.14)
    s += solid(L([(4.15, 2.75), (4.75, 2.2), (5.5, 2.2), (5.5, 4.2), (4.75, 4.2), (4.15, 3.65)]), "metaldark", d=0.14, opts="rounded corners=0.06cm")
    s += streak(1.4, 3.42, 4.1, 3.53)
    s += box(2.55, 3.6, 3.15, 4.15, BLUED, r=0.06, d=0.06)
    s += box(2.45, 4.1, 3.25, 4.4, "metal", r=0.06, d=0.06)
    s += tape(1.9, 3.2, 0.45, 1.1, fill="red")
    # the lenses, glinting
    s += ellipse(5.5, 3.2, 0.24, 0.98, "teal")
    s += f"\\fill[white, opacity=0.85] {E(5.48, 3.55, 0.07, 0.25)};\n"
    s += glint(5.55, 2.85, 0.07)
    s += ellipse(0.35, 3.2, 0.14, 0.73, "teal")
    s += glint(0.33, 3.45, 0.06)
    return s


def laser():
    s = ""
    # beam stub, glowing, to the tile's edge
    s += f"\\fill[red, opacity=0.25, rounded corners=0.2cm] {R(4.3, 2.4, 5.88, 3.1)};\n"
    s += f"\\fill[redlight] {R(4.3, 2.63, 5.88, 2.87)};\n"
    s += f"\\fill[white, opacity=0.8] {R(4.3, 2.72, 5.88, 2.78)};\n"
    s += rail(1.0, 3.6, 4.2)
    s += box(0.75, 1.75, 3.75, 3.85, "armourdark", r=0.18, d=0.18)
    s += solid(R(0.9, 2.3, 1.6, 3.3), "gold", opts="rounded corners=0.1cm", d=0.08)
    s += box(3.7, 2.25, 4.35, 3.25, "metal", r=0.08, d=0.1)
    s += ellipse(4.35, 2.75, 0.14, 0.45, "red")
    s += glint(4.33, 2.88, 0.06)
    s += box(1.95, 3.85, 2.7, 4.05, "red", r=0.06, d=0.04)
    s += streak(1.0, 3.55, 3.5, 3.66, op=0.4)
    s += bolt(2.3, 2.15)
    s += bolt(3.3, 2.15)
    s += tape(2.75, 2.8, 0.4, 2.2, fill="stone")
    return s


def scanner():
    s = ""
    # antenna, then the box
    s += line((4.55, 4.5), (4.9, 5.45), lw=5.0)
    s += line((4.55, 4.5), (4.9, 5.45), lw=2.4, colour="metallight")
    s += circle(4.92, 5.45, 0.17, "red", ",line width=1.2pt")
    s += rail(1.6, 4.4, 1.25, h=0.3)
    s += box(0.7, 1.3, 5.3, 4.65, "sagedark", r=0.3, d=0.2)
    s += f"\\draw[ink, line width=1.0pt, rounded corners=0.2cm] {R(0.9, 1.5, 5.1, 4.45)};\n"
    scr = R(1.1, 1.75, 4.0, 4.2)
    s += f"\\filldraw[inked, fill=tealdark!55!ink, rounded corners=0.15cm] {scr};\n"
    lines = "".join(f"\\draw[glow, opacity=0.18, line width=0.8pt] {P(1.1, 1.9 + k * 0.22)} -- {P(4.0, 1.9 + k * 0.22)};\n" for k in range(11))
    s += clipped(scr, lines)
    # the drone on screen: an outline in glow (body, arms, rotors, legs)
    g = "\\draw[glow, line width=1.6pt, line cap=round, line join=round]"
    cx, cy = 2.55, 2.95
    s += f"{g} [rounded corners=0.12cm] {R(cx - 0.42, cy - 0.22, cx + 0.42, cy + 0.25)};\n"
    s += f"{g} {P(cx - 0.42, cy + 0.12)} -- {P(cx - 0.95, cy + 0.5)} {P(cx + 0.42, cy + 0.12)} -- {P(cx + 0.95, cy + 0.5)};\n"
    s += f"{g} {E(cx - 0.95, cy + 0.6, 0.42, 0.1)} {E(cx + 0.95, cy + 0.6, 0.42, 0.1)};\n"
    s += f"{g} {P(cx - 0.25, cy - 0.22)} -- {P(cx - 0.4, cy - 0.55)} {P(cx + 0.25, cy - 0.22)} -- {P(cx + 0.4, cy - 0.55)};\n"
    s += f"\\fill[glow] {C(cx, cy + 0.02, 0.09)};\n"
    for k in range(4):
        a = 45 + 90 * k
        p, q = rot((cx + 1.1, cy + 0.15), a, (cx, cy + 0.15)), rot((cx + 1.35, cy + 0.15), a, (cx, cy + 0.15))
        s += f"\\draw[glow, line width=1.2pt, line cap=round, opacity=0.8] {P(*p)} -- {P(*q)};\n"
    s += f"\\fill[white, opacity=0.25] {L([(1.25, 3.4), (1.9, 4.1), (2.2, 4.1), (1.35, 3.1)])};\n"
    # dial and buttons
    s += disc(4.6, 3.6, 0.36, "metallight")
    s += line((4.6, 3.6), (4.78, 3.82), lw=1.6)
    s += circle(4.4, 2.6, 0.15, "red", ",line width=1.2pt")
    s += circle(4.85, 2.6, 0.15, "gold", ",line width=1.2pt")
    s += box(4.3, 1.8, 4.95, 2.15, "metaldark", r=0.06, d=0.04)
    s += bolt(0.95, 4.25, r=0.08)
    s += bolt(0.95, 1.7, r=0.08)
    return s


# ---------------------------------------------------------------- grips

def wood():
    s = ""
    s += box(1.6, 4.15, 4.6, 4.75, STEEL, r=0.1, d=0.1)
    s += bolt(2.0, 4.45)
    s += bolt(4.2, 4.45)
    g = "(2.15,4.2) -- (4.25,4.2) .. controls (4.0,3.2) and (3.55,1.9) .. (3.45,1.05) -- (1.6,0.95) .. controls (1.7,2.0) and (1.85,3.2) .. (2.15,4.2) -- cycle"
    s += solid(g, "wood", d=0.18)
    hatch = ""
    for k in range(-6, 10):
        hatch += f"\\draw[wooddark, line width=0.9pt] {P(1.4 + k * 0.28, 1.2)} -- {P(1.4 + k * 0.28 + 1.5, 3.8)};\n"
        hatch += f"\\draw[wooddark, line width=0.9pt] {P(1.4 + k * 0.28, 3.8)} -- {P(1.4 + k * 0.28 + 1.5, 1.2)};\n"
    panel = "(2.35,3.6) -- (3.5,3.6) .. controls (3.35,2.9) and (3.1,2.1) .. (3.0,1.55) -- (2.05,1.5) .. controls (2.1,2.3) and (2.2,3.0) .. (2.35,3.6) -- cycle"
    s += f"\\fill[woodlight] {panel};\n"
    s += clipped(panel, hatch)
    s += f"\\draw[inked] {panel};\n"
    s += curve([(2.2, 3.95), (2.0, 2.6), (1.85, 1.3)], lw=2.0, colour="white", extra=", opacity=0.5")
    s += box(1.45, 0.7, 3.6, 1.1, BLUED, r=0.1, d=0.08)
    s += circle(2.72, 2.55, 0.17, "metallight", ",line width=1.2pt")
    s += line((2.6, 2.55), (2.84, 2.55), lw=1.0)
    return s


def fore():
    s = ""
    s += box(1.0, 4.35, 5.0, 5.0, BLUED, r=0.12, d=0.1)
    for k in range(6):
        x = 1.4 + k * 0.6
        s += line((x, 4.5), (x, 4.85), lw=1.3, colour="metal")
    s += disc(5.1, 4.68, 0.25, "metallight")
    body = "(2.35,4.35) -- (3.65,4.35) .. controls (3.55,4.0) and (3.95,3.8) .. (3.75,3.5) .. controls (3.55,3.25) and (3.95,3.05) .. (3.75,2.75) .. controls (3.55,2.5) and (3.95,2.3) .. (3.75,2.0) -- (3.7,1.1) -- (2.3,1.1) -- (2.35,4.35) -- cycle"
    s += solid(body, "armourdark", d=0.2)
    s += curve([(2.55, 4.2), (2.5, 2.8), (2.52, 1.4)], lw=2.4, colour="white", extra=", opacity=0.35")
    s += tape(3.0, 2.25, 1.65, 0.45, a=-5, fill="red")
    s += box(2.1, 0.7, 3.95, 1.15, "metal", r=0.12, d=0.1)
    s += bolt(3.0, 3.75)
    return s


def stock():
    rng = random.Random("parts/stock")
    s = ""
    st = "(5.3,3.95) -- (1.0,4.3) .. controls (0.85,4.3) and (0.8,4.2) .. (0.8,4.05) -- (0.75,1.55) -- (5.3,3.05) -- cycle"
    s += solid(st, "wood", d=0.2)
    s += clipped(st, wood_grain(1.0, 2.0, 5.0, 4.0, rng, n=4))
    s += curve([(1.2, 4.08), (3.0, 3.95), (5.1, 3.82)], lw=2.0, colour="white", extra=", opacity=0.45")
    s += solid(L([(0.35, 1.35), (0.85, 1.5), (0.9, 4.3), (0.4, 4.4)]), "ink!70!metaldark", d=0.08, opts="rounded corners=0.1cm")
    for k in range(5):
        y = 1.8 + k * 0.5
        s += line((0.45, y), (0.8, y), lw=1.0, colour="metal")
    s += box(5.05, 2.9, 5.6, 4.15, STEEL, r=0.08, d=0.08)
    s += bolt(5.32, 3.53)
    s += circle(2.2, 1.85, 0.2, "metallight")
    s += circle(2.2, 1.85, 0.09, "greenA")
    s += tape(3.4, 3.15, 0.5, 1.45, a=-8, fill="stone")
    return s


def skeleton():
    s = ""
    tubes = [[(5.1, 3.95), (2.5, 4.05), (1.1, 3.85)],
             [(5.1, 3.05), (3.0, 2.55), (1.15, 1.75)],
             [(3.75, 3.98), (3.1, 3.3), (2.65, 2.45)]]
    s += tube(tubes, 0.3, "metallight", tension=0.5)
    s += f"\\draw[white, opacity=0.6, line width=1.4pt, line cap=round] plot[smooth, tension=0.5] coordinates {{{P(4.9, 4.03)} {P(2.5, 4.13)} {P(1.6, 4.0)}}};\n"
    s += weld([(3.75 + 0.0, 3.82), (3.62, 3.75)])
    s += weld([(2.62, 2.6), (2.75, 2.56)])
    s += solid(L([(0.45, 1.25), (1.2, 1.4), (1.25, 4.3), (0.5, 4.45)]), "ink!70!metaldark", d=0.08, opts="rounded corners=0.14cm")
    for k in range(5):
        y = 1.7 + k * 0.55
        s += line((0.6, y), (1.08, y), lw=1.0, colour="metal")
    s += box(4.85, 2.65, 5.55, 4.35, BLUED, r=0.08, d=0.1)
    s += bolt(5.2, 3.0)
    s += bolt(5.2, 4.0)
    s += tape(2.0, 3.98, 0.42, 0.62, a=-5, fill="red")
    return s


def brace():
    s = ""
    body = "(5.3,4.0) -- (1.35,4.2) -- (1.35,1.3) -- (5.3,2.75) -- cycle"
    s += solid(body, "armour", d=0.2, opts="rounded corners=0.12cm")
    s += f"\\draw[armourdark, line width=1.4pt] (2.2,2.15) -- (4.6,3.0);\n"
    # the padded cheek rest, quilted
    pad = R(1.6, 3.95, 4.4, 4.95)
    s += solid(pad, "red!65!wood", d=0.16, opts="rounded corners=0.4cm")
    for x in (2.3, 3.0, 3.7):
        s += line((x, 4.08), (x, 4.82), lw=1.1, colour="red!40!ink", extra=", dash pattern=on 2pt off 2pt")
    s += f"\\fill[white, opacity=0.35, rounded corners=0.1cm] {R(1.85, 4.62, 4.1, 4.75)};\n"
    # thick recoil pad
    s += solid(L([(0.35, 0.85), (1.45, 1.05), (1.45, 4.35), (0.35, 4.6)]), "ink!72!metaldark", d=0.1, opts="rounded corners=0.25cm")
    for k in range(6):
        y = 1.35 + k * 0.52
        s += line((0.55, y), (1.25, y), lw=1.2, colour="metal")
    s += box(5.05, 2.55, 5.6, 4.15, BLUED, r=0.08, d=0.1)
    s += bolt(1.9, 3.6)
    s += bolt(4.6, 3.6)
    s += bolt(1.9, 1.85)
    s += weld([(3.0 + k * 0.15, 3.98) for k in range(6)])
    return s


# ---------------------------------------------------------------- registry

DRAW = {"pistol": pistol, "smg": smg, "ak": ak, "pump": pump, "heavy": heavy,
        "short": short, "rifle": rifle, "double": double, "rocket": rocket, "lobber": lobber,
        "single": lambda: mount("single"), "pair": lambda: mount("pair"), "stack": lambda: mount("stack"),
        "square": lambda: mount("square"), "gatling": lambda: mount("gatling"),
        "slug": slug, "blast": blast, "goo": goo, "shock": shock, "ember": ember,
        "iron": iron, "dot": dot, "scope": scope, "laser": laser, "scanner": scanner,
        "wood": wood, "fore": fore, "stock": stock, "skeleton": skeleton, "brace": brace}

_names = [p for _, parts in CATEGORY.values() for p in parts]
assert len(_names) == 30 and len(set(_names)) == 30, "thirty distinct parts"
assert all(len(parts) == 5 for _, parts in CATEGORY.values()), "five parts per category"
assert set(DRAW) == set(_names), "every part has a drawing and every drawing a part"
check_mounts()


def _make(name):
    return lambda: icon(name, DRAW[name]())


PICTURES = {f"parts/{n}": _make(n) for n in _names}
