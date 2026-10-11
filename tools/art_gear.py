"""Raiders of the Lost Mart's forty gear icons (web/art/gear/*.png): the
raider's body gear, melee weapons and ammo, drawn as TikZ and compiled,
never generated as pixels. tools/art.py builds them
(`python3 tools/art.py gear`); the helpers and palette are in
tools/artlib.py, and the tile, `solid` shading, bolts, tape and welds are
art_parts.py's, imported, so a gear icon and a part icon finish alike.

Each icon is a 6 cm square: a rounded tile in its group's colour with one
item on it, drawn worn and home-made (stitches, patches, tape, rivets,
scuffs).

Facts each picture must get right (the things an image generator gets
wrong), and where this file enforces each:

- no words, letters or digits in any picture; the page sets every word.
  Nothing here emits a \\node or any text: every mark is a path, and
  art.py's `check_no_text` refuses a .tex containing \\node, \\text or a
  label before it is compiled. The crosses on `gear/mesh` and
  `gear/medgloves` are two crossed bars (`cross`), the stripe on
  `gear/quickgloves` is a filled zigzag polygon (`zigzag`), and the sound
  on `gear/decoys` is arcs only (`sound`): none is a glyph.
- the eleven groups read apart: every icon sits on a rounded tile in its
  group's colour (`GROUP`: head wallB, shoulders wallF, chest wallD, coat
  sage, hands greenA, legs wallE, knees wallA, feet teal, belt gold, melee
  red, ammo metal). `tile` is the only place a tile is drawn and
  `PICTURES` is built from `GROUP`, so an item cannot sit on the wrong
  tile; the build asserts forty distinct items, the per-group counts
  (3, 4, 3, 4, 3, 3, 3, 4, 5, 3, 5), and a drawing for every item.
- the basic items (cap, sling, tee, hoodie, workgloves, jeans, patches,
  sneakers, belt, bat) are plain, everyday versions: `check_basic`
  asserts that their source uses no accent colour (cyan, glow, magenta,
  hot) and no emblem (`cross`, `zigzag` and `sound` each leave an
  `% emblem` comment, which a basic item's source must not contain).
- the items that need a mark carry it: `check_marks` asserts mesh and
  medgloves draw a cross, quickgloves a zigzag, decoys sound arcs, the
  headset and paint use cyan, and the mine a red light.
- pairs come in pairs: gloves, knee patches, knee pads, knee braces,
  shoulder pads and every shoe are drawn by one function each placed
  twice (`pair`).
- plum ink (#3B2340) outlines every filled shape at 1.8 pt, as on the
  parts (`artlib.INK["gear"]` is set here): every shape goes through
  `solid` or the artlib helpers, which use the `inked` style. Glints,
  shading, smoke glow and spray mist are light, not shapes: unoutlined.
- light from the top left: `solid` darkens each shape's lower-right edge
  (one shade, the fill mixed with ink) and glints sit at upper left.
- repeatable: the only randomness (camo blots, mesh, paint mist) comes
  from random.Random seeded by the picture's name.
"""
import math, random, re

import artlib
from artlib import P, f, circle, ellipse, poly, line, curve, tube, place, clipped, glint, rot, blob_path, limb_pts
from art_parts import R, C, E, L, solid, box, disc, streak, bolt, tape, weld, shotshell, TILE_PATH, W, H

artlib.INK["gear"] = 1.8

GROUP = {
    "head": ("wallB", ["cap", "helmet", "headset"]),
    "shoulders": ("wallF", ["sling", "pads", "straps", "brace"]),
    "chest": ("wallD", ["tee", "vest", "mesh"]),
    "coat": ("sage", ["hoodie", "poncho", "trench", "hazmat"]),
    "hands": ("greenA", ["workgloves", "quickgloves", "medgloves"]),
    "legs": ("wallE", ["jeans", "cargo", "joggers"]),
    "knees": ("wallA", ["patches", "kneepads", "springs"]),
    "feet": ("teal", ["sneakers", "runners", "sockshoes", "boots"]),
    "belt": ("gold", ["belt", "mines", "decoys", "paint", "smoke"]),
    "melee": ("red", ["bat", "sledge", "machete"]),
    "ammo": ("metal", ["ammo_light", "ammo_heavy", "ammo_shells", "ammo_rockets", "ammo_grenades"]),
}
TILE = {item: colour for colour, items in GROUP.values() for item in items}
BASIC = ["cap", "sling", "tee", "hoodie", "workgloves", "jeans", "patches", "sneakers", "belt", "bat"]
ACCENTS = ("cyan", "glow", "glowhi", "magenta", "hot", "hothi")

DENIM = "tealdark!60!violet"
BLACKPAD = "ink!78!metaldark"
STEEL = "metal"
BLUED = "metaldark!85!ink"
BRASS = "gold!88!wood"


# ---------------------------------------------------------------- helpers

def tile(name, body):
    """The group's tile, the item clipped to it, and the tile's ink edge
    drawn last so nothing spills past the rounded corners."""
    s = f"\\fill[{TILE[name]}] {TILE_PATH};\n"
    s += clipped(TILE_PATH, body)
    s += f"\\draw[inked] {TILE_PATH};\n"
    return s, (W, H)


def mirror(right, cx=3.0):
    """A symmetric outline from its right half, listed top-centre to
    bottom-centre."""
    return right + [(2 * cx - x, y) for x, y in reversed(right)]


def bez(p0, p1, p2, n=6):
    return [((1 - t) ** 2 * p0[0] + 2 * (1 - t) * t * p1[0] + t * t * p2[0],
             (1 - t) ** 2 * p0[1] + 2 * (1 - t) * t * p1[1] + t * t * p2[1]) for t in (k / n for k in range(n + 1))]


def stitch(path, colour="ink", lw=0.9):
    """A row of stitches along any path (a seam, a hem, a patch)."""
    return f"\\draw[{colour}, line width={f(lw)}pt, line cap=round, dash pattern=on 2.2pt off 1.8pt] {path};\n"


def seam(points, colour="ink", lw=0.9):
    return stitch(" -- ".join(P(*p) for p in points), colour, lw)


def smooth(points, tension=0.6):
    return f"plot[smooth, tension={tension}] coordinates {{{artlib.pts(points)}}}"


def blobby(points, fill, d=0.13, tension=0.7):
    return solid(blob_path(points, tension), fill, d)


def gloss(points, lw=2.0, op=0.5):
    """A glint along a curved surface: an unoutlined white stroke."""
    return curve(points, lw=lw, colour="white", extra=f", opacity={f(op)}")


def cross(cx, cy, size, fill, bg="white", r=0.18):
    """A cross patch: a stitched square and two crossed bars (no glyph)."""
    s = "% emblem: cross of two bars\n"
    s += box(cx - size / 2, cy - size / 2, cx + size / 2, cy + size / 2, bg, r=r, d=0.08)
    s += stitch(f"[rounded corners={f(r * 0.7)}cm] {R(cx - size / 2 + 0.1, cy - size / 2 + 0.1, cx + size / 2 - 0.1, cy + size / 2 - 0.1)}", f"{bg}!55!ink", 0.8)
    a, b = size * 0.34, size * 0.11
    s += solid(L([(cx - b, cy + a), (cx + b, cy + a), (cx + b, cy + b), (cx + a, cy + b), (cx + a, cy - b), (cx + b, cy - b),
                  (cx + b, cy - a), (cx - b, cy - a), (cx - b, cy - b), (cx - a, cy - b), (cx - a, cy + b), (cx - b, cy + b)]), fill, d=0.05)
    return s


def zigzag(x0, x1, y, amp, n, h, fill):
    """A lightning stripe: a filled zigzag band, not a glyph."""
    top = [(x0 + (x1 - x0) * k / n, y + (amp if k % 2 else -amp)) for k in range(n + 1)]
    bottom = [(x + 0.1, yy - h) for x, yy in reversed(top)]
    return "% emblem: zigzag stripe\n" + poly(top + bottom, fill)


def sound(cx, cy, a0, a1, radii, lw=2.0):
    """Sound waves: arcs only."""
    s = "% emblem: sound arcs\n"
    for r in radii:
        sx, sy = cx + r * math.cos(math.radians(a0)), cy + r * math.sin(math.radians(a0))
        s += f"\\draw[ink, line width={f(lw)}pt, line cap=round] {P(sx, sy)} arc[start angle={f(a0)}, end angle={f(a1)}, radius={f(r)}];\n"
    return s


def rivet(x, y, r=0.07, fill="metallight"):
    return circle(x, y, r, fill, ",line width=1.0pt")


def buckle(x, y, w=0.55, h=0.42, fill="metallight", a=0):
    """A square strap buckle with its bar."""
    pts = [rot(p, a) for p in [(-w / 2, -h / 2), (w / 2, -h / 2), (w / 2, h / 2), (-w / 2, h / 2)]]
    s = f"\\draw[ink, line width=4.6pt, line join=round] {L([(x + px, y + py) for px, py in pts])};\n"
    s += f"\\draw[{fill}, line width=2.2pt, line join=round] {L([(x + px, y + py) for px, py in pts])};\n"
    p, q = rot((0, -h / 2), a), rot((0, h / 2), a)
    s += line((x + p[0], y + p[1]), (x + q[0], y + q[1]), lw=1.6)
    return s


def pair(left, right):
    return left + right


# ---------------------------------------------------------------- head

def cap():
    s = ""
    crown = "(0.85,2.45) .. controls (0.7,4.3) and (1.9,5.05) .. (3.0,5.05) .. controls (4.2,5.05) and (4.75,4.0) .. (4.6,2.75) .. controls (3.4,2.25) and (2.0,2.2) .. (0.85,2.45) -- cycle"
    # the brim's underside, then the crown, then the brim's top
    s += solid("(3.2,2.55) .. controls (4.2,2.4) and (5.2,2.0) .. (5.6,1.65) .. controls (5.0,1.2) and (3.8,1.5) .. (3.0,2.1) -- cycle", "greenD!70!ink", d=0.05)
    s += solid(crown, "greenD", d=0.2)
    s += clipped(crown, curve([(3.05, 5.1), (3.3, 3.6), (3.25, 2.3)], lw=1.1) + curve([(1.9, 4.85), (1.95, 3.6), (2.05, 2.3)], lw=1.1)
                 + curve([(4.2, 4.55), (4.45, 3.6), (4.6, 2.6)], lw=1.1))
    s += seam([(0.95, 2.75), (2.0, 2.58), (3.2, 2.6), (4.5, 2.95)], "greenD!40!ink")
    s += solid("(3.0,2.75) .. controls (4.2,2.75) and (5.25,2.3) .. (5.7,1.85) .. controls (5.55,1.55) and (5.2,1.5) .. (4.9,1.6) .. controls (4.3,1.95) and (3.6,2.25) .. (2.85,2.35) -- cycle", "greenD", d=0.12)
    s += stitch("(3.2,2.58) .. controls (4.2,2.55) and (5.0,2.15) .. (5.4,1.8)", "greenD!40!ink")
    s += circle(3.05, 5.08, 0.17, "greenD")
    for x, y in ((2.2, 4.15), (3.9, 4.1)):
        s += circle(x, y, 0.07, "greenD!50!ink", ",line width=0.9pt")
    s += gloss([(1.25, 3.4), (1.45, 4.2), (2.05, 4.7)], lw=2.6, op=0.45)
    # worn: a frayed brim edge and a darned rip
    for x, y in ((5.05, 1.6), (5.25, 1.58), (4.85, 1.66)):
        s += line((x, y), (x + 0.04, y - 0.16), lw=1.0)
    s += seam([(1.3, 3.1), (1.6, 3.0)], "white")
    return s


def helmet():
    s = ""
    shell = "(0.75,2.2) .. controls (0.55,4.4) and (1.7,5.45) .. (3.0,5.45) .. controls (4.3,5.45) and (5.45,4.4) .. (5.25,2.2) .. controls (5.25,1.5) and (4.7,1.2) .. (4.3,1.2) -- (4.3,2.0) -- (1.7,2.0) -- (1.7,1.2) .. controls (1.3,1.2) and (0.75,1.5) .. (0.75,2.2) -- cycle"
    s += solid(shell, "armourdark", d=0.2)
    # the face opening, dark inside, padded rim
    face = "(1.45,3.75) -- (4.55,3.75) -- (4.45,1.35) .. controls (3.6,1.0) and (2.4,1.0) .. (1.55,1.35) -- cycle"
    s += f"\\filldraw[inked, fill=ink!85!armourdark] {face};\n"
    s += f"\\fill[ink!60!armourdark] (2.3,3.0) .. controls (2.3,2.2) and (3.7,2.2) .. (3.7,3.0) -- (3.7,3.75) -- (2.3,3.75) -- cycle;\n"
    s += gloss([(1.15, 3.6), (1.45, 4.5), (2.3, 5.05)], lw=3.0, op=0.4)
    s += gloss([(3.6, 5.2), (3.9, 5.15)], lw=2.0, op=0.6)
    # a vent ridge along the top, and tape where it cracked
    s += box(2.6, 4.75, 3.4, 5.3, "armour", r=0.12, d=0.08)
    s += tape(4.3, 4.55, 0.85, 0.28, a=-38, fill="stone")
    # the visor: clear, tinted, with a thick black rim at the brow
    visor = "(1.05,3.95) -- (4.95,3.95) .. controls (5.05,2.8) and (4.9,1.6) .. (4.6,0.95) .. controls (3.6,0.65) and (2.4,0.65) .. (1.4,0.95) .. controls (1.1,1.6) and (0.95,2.8) .. (1.05,3.95) -- cycle"
    s += f"\\fill[glowhi, opacity=0.35] {visor};\n"
    s += f"\\fill[white, opacity=0.55] (1.35,3.75) -- (1.85,3.75) -- (1.5,1.4) -- (1.3,1.6) -- cycle;\n"
    s += f"\\fill[white, opacity=0.35] (2.05,3.75) -- (2.25,3.75) -- (1.9,1.2) -- (1.75,1.25) -- cycle;\n"
    s += f"\\draw[inked] {visor};\n"
    s += box(0.95, 3.75, 5.05, 4.15, BLACKPAD, r=0.12, d=0.06)
    for x in (1.0, 5.0):
        s += disc(x, 3.0, 0.3, "armour")
        s += bolt(x, 3.0, r=0.12)
    return s


def headset():
    s = ""
    # the band, padded on its underside
    s += tube([[(1.2, 3.1), (1.4, 4.6), (3.0, 5.35), (4.6, 4.6), (4.8, 3.1)]], 0.34, "armourdark", tension=0.55)
    s += tube([[(1.95, 4.75), (3.0, 5.0), (4.05, 4.75)]], 0.3, BLACKPAD, tension=0.5)
    for x in (2.35, 2.75, 3.25, 3.65):
        y = 4.9 + 0.1 * math.cos((x - 3.0) * 1.3)
        s += line((x, y - 0.06), (x, y + 0.06), lw=1.0, colour="metal")
    # the mic boom from the left cup to the mouth
    s += tube([[(1.1, 2.0), (1.35, 1.0), (2.3, 0.75), (2.95, 0.9)]], 0.16, BLUED, tension=0.5)
    s += blobby([(2.85, 0.92), (3.1, 1.2), (3.55, 1.15), (3.65, 0.85), (3.3, 0.6), (2.95, 0.65)], BLACKPAD, d=0.08)
    for x in (3.1, 3.3, 3.45):
        s += f"\\fill[metal] {C(x, 0.92, 0.04)};\n"
    # the ear cups
    for cx, flip in ((1.1, False), (4.9, True)):
        sx = -1 if flip else 1
        s += box(cx - 0.75, 1.75, cx + 0.75, 4.0, "armour", r=0.55, d=0.2)
        s += box(cx + sx * 0.45 - 0.35, 2.0, cx + sx * 0.45 + 0.35, 3.75, BLACKPAD, r=0.3, d=0.08)
        for k in range(3):
            y = 2.45 + k * 0.38
            s += line((cx - sx * 0.45, y), (cx - sx * 0.05, y), lw=1.4, colour="armourdark")
        s += f"\\fill[white, opacity=0.45, rounded corners=0.1cm] {R(cx - 0.6, 3.6, cx + 0.2, 3.75)};\n"
    s += bolt(1.15, 3.3)
    s += tape(1.1, 1.95, 0.5, 0.75, a=10, fill="red")
    # the small cyan light, glowing
    for r, op in ((0.6, 0.18), (0.42, 0.25)):
        s += f"\\fill[glow, opacity={f(op)}] {C(4.95, 2.35, r)};\n"
    s += circle(4.95, 2.35, 0.2, "cyan", ",line width=1.2pt")
    s += glint(4.9, 2.42, 0.07)
    s += circle(4.85, 3.35, 0.16, "metallight", ",line width=1.2pt")
    return s


# ---------------------------------------------------------------- shoulders

def sling():
    s = ""
    pts = []
    cx, cy = 2.75, 3.3
    for k in range(80):
        t = 1.68 * 2 * math.pi * k / 79
        r = 0.55 + 0.72 * t / (2 * math.pi)
        pts.append((cx + r * math.cos(t + 1.3), cy + r * math.sin(t + 1.3) * 0.92))
    tail = [pts[-1], (4.6, 1.5), (4.85, 0.95)]
    path = pts + tail[1:]
    s += tube([path], 0.5, "sagedark", tension=0.5)
    s += f"\\draw[sagelight, line width=0.9pt, dash pattern=on 2.2pt off 1.8pt] {smooth(path, 0.5)};\n"
    s += gloss([pts[56], pts[60], pts[64]], lw=1.6, op=0.55)
    # the slide on the coil, and the swivel clip at the free end
    s += buckle(*pts[40], w=0.62, h=0.36, a=math.degrees(math.atan2(pts[41][1] - pts[39][1], pts[41][0] - pts[39][0])))
    s += f"\\draw[ink, line width=5pt] {E(4.95, 0.65, 0.32, 0.26)};\n"
    s += f"\\draw[metallight, line width=2.4pt] {E(4.95, 0.65, 0.32, 0.26)};\n"
    s += box(4.55, 0.85, 5.0, 1.15, "sagedark!70!ink", r=0.06, d=0.05)
    return s


def pad_shell(shell, lame):
    """One hard shoulder pad about its own origin: a domed cap, a second
    plate below it, and two straps with buckles."""
    s = ""
    for x in (-0.55, 0.55):
        s += box(x - 0.17, -2.0, x + 0.17, -0.6, "wooddark", r=0.04, d=0.05)
        s += buckle(x, -1.55, w=0.46, h=0.3)
    s += solid("(-1.1,-0.6) .. controls (-1.15,-0.2) and (-1.0,0.0) .. (-0.95,0.05) -- (0.95,0.05) .. controls (1.0,0.0) and (1.15,-0.2) .. (1.1,-0.6) .. controls (0.4,-0.85) and (-0.4,-0.85) .. (-1.1,-0.6) -- cycle", lame, d=0.12)
    dome = "(-1.2,-0.1) .. controls (-1.3,1.35) and (1.3,1.35) .. (1.2,-0.1) .. controls (0.4,-0.38) and (-0.4,-0.38) .. (-1.2,-0.1) -- cycle"
    s += solid(dome, shell, d=0.18)
    s += curve([(-0.75, 0.5), (-0.4, 0.8), (0.05, 0.9)], lw=2.2, colour="white", extra=", opacity=0.5")
    for x in (-0.85, 0.85):
        s += rivet(x, -0.1)
    s += rivet(0, -0.55)
    return s


def pads():
    s = ""
    left = pad_shell("armour", "armourlight")
    right = pad_shell("armour", "armourlight") + tape(0.35, 0.35, 0.95, 0.3, a=-28, fill="gold")
    s += pair(place(1.6, 3.25, 1.22, left, rot=8), place(4.4, 3.25, 1.22, right, rot=-8, flip=True))
    return s


def straps():
    s = ""
    web = "greenE"
    # the two shoulder straps, curving over the top
    for x0, x1 in ((1.15, 1.85), (4.15, 4.85)):
        mx = (x0 + x1) / 2
        s += solid(L([(x0 - 0.15, 6.0), (x1 - 0.15, 6.0), (x1 + 0.05 * (1 if mx > 3 else -1) * -1, 0.0), (x0 + 0.05 * (1 if mx > 3 else -1) * -1, 0.0)]), web, d=0.14)
        s += seam([(x0 - 0.02, 5.9), (x0 + 0.08, 0.1)], "greenE!40!ink")
        s += seam([(x1 - 0.22, 5.9), (x1 - 0.12, 0.1)], "greenE!40!ink")
    # the sternum strap and its side-release buckle
    s += box(1.6, 3.15, 4.4, 3.55, "greenD", r=0.06, d=0.08)
    s += solid("(2.3,3.0) -- (2.95,3.0) -- (2.95,3.7) -- (2.3,3.7) .. controls (2.15,3.5) and (2.15,3.2) .. (2.3,3.0) -- cycle", BLACKPAD, d=0.08)
    s += solid("(3.0,3.05) -- (3.65,3.05) .. controls (3.85,3.2) and (3.85,3.5) .. (3.65,3.65) -- (3.0,3.65) -- cycle", BLACKPAD, d=0.08)
    s += f"\\fill[metal] {R(2.5, 3.25, 2.8, 3.45)};\n"
    # ladder-lock buckles low down, tails dangling
    for x in (1.45, 4.45):
        s += buckle(x, 1.45, w=0.85, h=0.38, fill="metallight")
        s += box(x - 0.18, 0.25, x + 0.18, 1.3, "greenE!80!ink", r=0.05, d=0.05)
    # pouches: a flapped mag pouch on the left, a radio pouch on the right
    s += box(0.75, 3.85, 2.25, 5.1, "sagedark", r=0.12, d=0.14)
    s += solid("(0.7,5.15) -- (2.3,5.15) -- (2.3,4.55) .. controls (1.9,4.35) and (1.1,4.35) .. (0.7,4.55) -- cycle", "sagedark!80!ink", d=0.08)
    s += circle(1.5, 4.45, 0.1, BLACKPAD, ",line width=1.0pt")
    s += box(3.85, 1.95, 5.15, 2.95, "sagedark", r=0.12, d=0.14)
    s += tube([[(4.85, 2.9), (4.95, 3.6)]], 0.14, BLACKPAD)
    s += box(4.05, 2.15, 4.65, 2.75, BLACKPAD, r=0.08, d=0.05)
    s += circle(4.92, 3.65, 0.08, "red", ",line width=1.0pt")
    for y in (2.2, 2.55):
        s += line((3.95, y), (5.05, y), lw=1.0, colour="sagedark!50!ink")
    s += rivet(1.5, 5.6)
    s += rivet(4.5, 5.6)
    return s


def brace():
    s = ""
    cx, cy = 3.0, 2.15
    # the strap under the arm, its buckle at the bottom
    s += tube([[(1.05, 2.3), (1.25, 1.0), (3.0, 0.55), (4.75, 1.0), (4.95, 2.3)]], 0.34, "wooddark", tension=0.55)
    s += buckle(3.0, 0.57, w=0.6, h=0.5)
    # the padded shoulder cap: a thick arch, quilted
    outer, inner = 2.15, 1.05
    arch = f"{P(cx + outer, cy)} arc[start angle=0, end angle=180, radius={f(outer)}] -- {P(cx - inner, cy)} arc[start angle=180, end angle=0, radius={f(inner)}] -- cycle"
    s += solid(f"[rounded corners=0.2cm] {arch}", "red!65!wood", d=0.2)
    for a in (35, 70, 110, 145):
        p, q = rot((cx + inner + 0.12, cy), a, (cx, cy)), rot((cx + outer - 0.12, cy), a, (cx, cy))
        s += line(p, q, lw=1.1, colour="red!40!ink", extra=", dash pattern=on 2pt off 2pt")
    s += f"\\draw[white, opacity=0.4, line width=2.2pt, line cap=round] {P(cx - 1.85, cy + 0.45)} arc[start angle=166, end angle=125, radius=1.9];\n"
    # the steel plate bolted on top of the cap
    plate = f"{P(cx + 1.55, cy + 0.85)} arc[start angle=29, end angle=151, radius=1.78] -- {P(cx - 1.12, cy + 0.62)} arc[start angle=151, end angle=29, radius=1.28] -- cycle"
    s += solid(f"[rounded corners=0.12cm] {plate}", "metal", d=0.12)
    for a in (45, 90, 135):
        s += bolt(*rot((cx + 1.53, cy), a, (cx, cy)), r=0.09)
    s += weld([rot((cx + 1.3 + 0.0, cy), a, (cx, cy)) for a in (60, 66, 72)])
    return s


# ---------------------------------------------------------------- chest

def tee_outline(neck=0.62, body=1.4, sleeve_out=2.45, sleeve_len=1.0, hem=0.75):
    """The right half of a short-sleeved shirt, top-centre to hem-centre."""
    right = bez((3.0, 4.6), (3.0 + neck * 0.7, 4.65), (3.0 + neck, 5.15), n=4)
    right += [(3.0 + neck + 0.85, 4.9), (3.0 + sleeve_out, 4.0), (3.0 + sleeve_out - 0.6, 4.0 - sleeve_len * 0.75),
              (3.0 + body, 3.7), (3.0 + body + 0.05, hem), (3.0, hem)]
    return mirror(right)


def tee():
    s = ""
    fill = "jacketb"
    out = tee_outline()
    s += f"\\filldraw[inked, fill={fill}!55!ink] (2.4,5.15) .. controls (2.7,4.95) and (3.3,4.95) .. (3.6,5.15) -- (3.4,4.7) -- (2.6,4.7) -- cycle;\n"
    s += solid(f"[rounded corners=0.12cm] {L(out)}", fill, d=0.2)
    s += curve(bez((2.38, 5.15), (2.55, 4.62), (3.0, 4.6)) + bez((3.0, 4.6), (3.45, 4.62), (3.62, 5.15))[1:], lw=4.0, colour=f"{fill}!75!ink")
    s += curve(bez((2.38, 5.15), (2.55, 4.62), (3.0, 4.6)) + bez((3.0, 4.6), (3.45, 4.62), (3.62, 5.15))[1:], lw=1.0, colour="ink")
    s += seam([(1.65, 1.0), (4.4, 1.0)], f"{fill}!40!ink")
    s += seam([(0.85, 3.7), (1.35, 3.35)], f"{fill}!40!ink")
    s += seam([(5.15, 3.7), (4.65, 3.35)], f"{fill}!40!ink")
    # creases and a glint
    s += curve([(2.0, 2.6), (2.5, 2.2), (2.6, 1.6)], lw=1.0, colour=f"{fill}!60!ink")
    s += curve([(3.9, 3.4), (4.05, 2.8)], lw=1.0, colour=f"{fill}!60!ink")
    s += gloss([(1.85, 4.45), (1.75, 3.4)], lw=2.6, op=0.4)
    return s


def vest():
    s = ""
    fill = "greenE"
    right = bez((3.0, 3.9), (3.4, 4.6), (3.65, 5.2), n=4) + [(4.35, 5.15)] + bez((4.35, 5.15), (4.3, 3.85), (4.8, 3.55), n=4)[1:] + [(4.85, 0.85), (3.0, 0.75)]
    out = mirror(right)
    s += f"\\filldraw[inked, fill={fill}!45!ink] (2.35,5.2) .. controls (2.7,4.85) and (3.3,4.85) .. (3.65,5.2) -- (3.3,4.2) -- (2.7,4.2) -- cycle;\n"
    s += solid(f"[rounded corners=0.14cm] {L(out)}", fill, d=0.2)
    # padded collar and shoulder straps
    for sx in (-1, 1):
        s += solid(L([(3.0 + sx * 0.65, 5.25), (3.0 + sx * 1.35, 5.2), (3.0 + sx * 1.32, 4.7), (3.0 + sx * 0.75, 4.75)]), "greenD", d=0.06)
    # the front plate pocket with its flap
    s += box(1.85, 1.6, 4.15, 3.9, "greenD", r=0.14, d=0.16)
    s += solid("(1.8,3.95) -- (4.2,3.95) -- (4.2,3.35) .. controls (3.5,3.1) and (2.5,3.1) .. (1.8,3.35) -- cycle", "greenD!80!ink", d=0.08)
    s += box(2.7, 3.2, 3.3, 3.55, "stone", r=0.05, d=0.04)
    # MOLLE webbing rows
    for y in (2.15, 2.75):
        s += f"\\fill[greenE!70!ink] {R(1.85, y - 0.1, 4.15, y + 0.1)};\n"
        for x in (2.2, 2.75, 3.3, 3.85):
            s += line((x, y - 0.12), (x, y + 0.12), lw=1.0)
    # side straps and buckles
    for sx in (-1, 1):
        x = 3.0 + sx * 1.6
        s += box(x - 0.25, 1.3, x + 0.25, 2.6, "greenE!75!ink", r=0.05, d=0.05)
        s += rivet(x, 2.35)
        s += rivet(x, 1.55)
    s += seam([(1.3, 1.05), (4.7, 1.05)], "greenE!40!ink")
    s += tape(4.3, 3.2, 0.7, 0.3, a=20, fill="stone")
    s += gloss([(1.45, 3.4), (1.5, 2.2)], lw=2.4, op=0.35)
    return s


def mesh():
    s = ""
    fill = "violet"
    rng = random.Random("gear/mesh")
    out = tee_outline(neck=0.55, body=1.2, sleeve_out=2.15, sleeve_len=0.75)
    s += f"\\filldraw[inked, fill={fill}!45!ink] (2.45,5.15) .. controls (2.7,4.95) and (3.3,4.95) .. (3.55,5.15) -- (3.4,4.7) -- (2.6,4.7) -- cycle;\n"
    s += solid(f"[rounded corners=0.12cm] {L(out)}", fill, d=0.2)
    # the mesh: a grid of small holes, and darker side panels
    holes = ""
    for i in range(30):
        for j in range(28):
            x, y = 0.6 + i * 0.17 + (0.085 if j % 2 else 0), 0.7 + j * 0.17
            holes += f"\\fill[{fill}!65!ink] {C(x, y, 0.035)};\n"
    s += clipped(f"[rounded corners=0.12cm] {L(out)}", holes + f"\\fill[{fill}!70!ink] {L([(1.75, 3.6), (2.15, 3.4), (2.0, 0.6), (1.6, 0.6)])};\n"
                 + f"\\fill[{fill}!70!ink] {L([(4.25, 3.6), (3.85, 3.4), (4.0, 0.6), (4.4, 0.6)])};\n")
    s += f"\\draw[inked] [rounded corners=0.12cm] {L(out)};\n"
    s += curve(bez((2.43, 5.15), (2.6, 4.62), (3.0, 4.6)) + bez((3.0, 4.6), (3.4, 4.62), (3.57, 5.15))[1:], lw=4.0, colour="white!80!violet")
    s += curve(bez((2.43, 5.15), (2.6, 4.62), (3.0, 4.6)) + bez((3.0, 4.6), (3.4, 4.62), (3.57, 5.15))[1:], lw=1.0, colour="ink")
    s += cross(3.0, 3.25, 1.3, "greenB")
    s += seam([(1.85, 1.0), (4.2, 1.0)], f"{fill}!40!ink")
    s += gloss([(2.0, 4.4), (1.95, 3.8)], lw=2.4, op=0.45)
    _ = rng
    return s


# ---------------------------------------------------------------- coat

def sleeves(fill, hw0=0.5, hw1=0.42, end=(5.25, 1.25), cuff="", top=(4.2, 4.55)):
    """Two long sleeves hanging behind the body, each with a cuff."""
    s = ""
    cuff = cuff or f"{fill}!80!ink"
    for sx in (1, -1):
        a = (3.0 + sx * (top[0] - 3.0), top[1])
        b = (3.0 + sx * (end[0] - 3.0), end[1])
        m = ((a[0] + b[0]) / 2 + sx * 0.25, (a[1] + b[1]) / 2)
        s += solid(blob_path(limb_pts([a, m, b], hw0, hw1), 0.5), fill, d=0.16)
        dx, dy = b[0] - m[0], b[1] - m[1]
        Ln = math.hypot(dx, dy)
        ux, uy = dx / Ln, dy / Ln
        nx, ny = -uy, ux
        c0 = (b[0] - ux * 0.05, b[1] - uy * 0.05)
        c1 = (b[0] + ux * 0.45, b[1] + uy * 0.45)
        w = hw1 * 0.95
        s += solid(L([(c0[0] + nx * w, c0[1] + ny * w), (c1[0] + nx * w, c1[1] + ny * w), (c1[0] - nx * w, c1[1] - ny * w), (c0[0] - nx * w, c0[1] - ny * w)]), cuff, d=0.06, opts="rounded corners=0.06cm")
    return s


def hoodie():
    s = ""
    fill = "wallC"
    # the hood, down behind the neck
    s += blobby([(1.75, 4.9), (2.0, 5.6), (3.0, 5.85), (4.0, 5.6), (4.25, 4.9), (3.0, 4.6)], fill, d=0.14)
    s += f"\\filldraw[inked, fill={fill}!45!ink] (2.25,5.0) .. controls (2.4,5.45) and (3.6,5.45) .. (3.75,5.0) .. controls (3.4,4.75) and (2.6,4.75) .. (2.25,5.0) -- cycle;\n"
    s += sleeves(fill, top=(4.25, 4.4), end=(5.2, 1.0))
    right = [(3.0, 4.7), (3.75, 5.0), (4.55, 4.75), (4.65, 3.6), (4.6, 1.0), (3.0, 1.0)]
    s += solid(f"[rounded corners=0.18cm] {L(mirror(right))}", fill, d=0.2)
    s += box(1.35, 0.55, 4.65, 1.1, f"{fill}!80!ink", r=0.1, d=0.06)
    for k in range(12):
        x = 1.55 + k * 0.27
        s += line((x, 0.65), (x, 1.0), lw=0.9, colour=f"{fill}!50!ink")
    # the kangaroo pocket
    s += solid("(1.9,1.15) -- (4.1,1.15) -- (3.75,2.55) -- (2.25,2.55) -- cycle", fill, d=0.1)
    s += f"\\draw[{fill}!50!ink, line width=1.2pt] (1.9,1.25) -- (2.25,2.45) (4.1,1.25) -- (3.75,2.45);\n"
    s += seam([(2.3, 2.42), (3.7, 2.42)], f"{fill}!45!ink")
    # drawstrings
    for x, x1 in ((2.65, 2.55), (3.35, 3.45)):
        s += curve([(x, 4.75), (x - 0.05 * (1 if x < 3 else -1), 4.1), (x1, 3.5)], lw=1.8, colour="ink")
        s += curve([(x, 4.75), (x - 0.05 * (1 if x < 3 else -1), 4.1), (x1, 3.5)], lw=0.8, colour="cream")
        s += box(x1 - 0.07, 3.15, x1 + 0.07, 3.5, "metallight", r=0.03, d=0.02)
    s += gloss([(1.75, 4.3), (1.7, 3.0)], lw=2.6, op=0.4)
    return s


def hood_up(fill, cx=3.0, cy=4.55, rx=1.25, ry=1.2, opening="ink!80!armourdark"):
    s = solid(E(cx, cy, rx, ry), fill, d=0.16)
    s += f"\\filldraw[inked, fill={opening}] {E(cx, cy - 0.12, rx * 0.6, ry * 0.62)};\n"
    s += gloss([(cx - rx * 0.8, cy + 0.2), (cx - rx * 0.55, cy + ry * 0.7)], lw=2.4, op=0.45)
    return s


def poncho():
    s = ""
    rng = random.Random("gear/poncho")
    base = "wood!80!sagedark"
    cape = "(2.4,4.3) -- (3.6,4.3) .. controls (4.4,3.6) and (5.2,2.7) .. (5.6,1.7) .. controls (4.6,1.15) and (3.8,0.55) .. (3.0,0.45) .. controls (2.2,0.55) and (1.4,1.15) .. (0.4,1.7) .. controls (0.8,2.7) and (1.6,3.6) .. (2.4,4.3) -- cycle"
    camo = ""
    for k in range(26):
        x, y = rng.uniform(0.3, 5.7), rng.uniform(0.3, 4.6)
        r = rng.uniform(0.25, 0.5)
        col = ("greenE", "wooddark", "sagedark")[k % 3]
        pts = [(x + r * rng.uniform(0.7, 1.3) * math.cos(a), y + r * rng.uniform(0.6, 1.1) * math.sin(a)) for a in (i * math.pi / 3 for i in range(6))]
        camo += f"\\fill[{col}] {blob_path(pts, 0.8)};\n"
    s += solid(cape, base, d=0.2)
    s += clipped(cape, camo + f"\\fill[ink, opacity=0.18] (5.6,1.7) .. controls (4.6,1.15) and (3.8,0.55) .. (3.0,0.45) -- (3.0,0.9) .. controls (3.8,1.0) and (4.6,1.5) .. (5.45,2.0) -- cycle;\n")
    s += f"\\draw[inked] {cape};\n"
    s += stitch("(0.75,1.85) .. controls (1.6,1.4) and (2.3,0.85) .. (3.0,0.78) .. controls (3.7,0.85) and (4.4,1.4) .. (5.25,1.85)", "ink!70!wood")
    s += curve([(3.0, 3.6), (2.95, 2.4), (3.0, 1.2)], lw=1.0, colour="ink!60!wood")
    # the hood, with camo too, and a drawcord
    hood = E(3.0, 4.55, 1.2, 1.15)
    s += solid(hood, base, d=0.16)
    rng2 = random.Random("gear/poncho/hood")
    hc = ""
    for k in range(7):
        x, y = rng2.uniform(1.9, 4.1), rng2.uniform(3.5, 5.6)
        col = ("greenE", "wooddark", "sagedark")[k % 3]
        pts = [(x + 0.32 * math.cos(a) * rng2.uniform(0.7, 1.2), y + 0.25 * math.sin(a)) for a in (i * math.pi / 3 for i in range(6))]
        hc += f"\\fill[{col}] {blob_path(pts, 0.8)};\n"
    s += clipped(hood, hc)
    s += f"\\draw[inked] {hood};\n"
    s += f"\\filldraw[inked, fill=ink!82!wooddark] {E(3.0, 4.4, 0.7, 0.68)};\n"
    s += curve([(2.75, 3.75), (2.65, 3.3), (2.7, 2.95)], lw=1.4)
    s += curve([(3.25, 3.75), (3.35, 3.3), (3.3, 2.95)], lw=1.4)
    s += circle(2.7, 2.9, 0.07, "metallight", ",line width=0.9pt")
    s += circle(3.3, 2.9, 0.07, "metallight", ",line width=0.9pt")
    s += gloss([(2.0, 4.8), (2.3, 5.4)], lw=2.2, op=0.4)
    return s


def flap_pocket(x0, y0, x1, y1, fill, snap="metallight"):
    s = box(x0, y0, x1, y1, fill, r=0.08, d=0.08)
    s += solid(L([(x0 - 0.03, y1 + 0.03), (x1 + 0.03, y1 + 0.03), (x1 + 0.03, y1 - 0.28), ((x0 + x1) / 2, y1 - 0.4), (x0 - 0.03, y1 - 0.28)]), f"{fill}!85!ink", d=0.05, opts="rounded corners=0.05cm")
    s += circle((x0 + x1) / 2, y1 - 0.25, 0.06, snap, ",line width=0.9pt")
    return s


def trench():
    s = ""
    fill = "woodlight"
    s += sleeves(fill, top=(4.25, 4.6), end=(5.25, 1.55), hw0=0.48, hw1=0.42)
    right = [(3.0, 4.7), (3.7, 5.3), (4.55, 4.95), (4.6, 3.3), (4.95, 0.35), (3.0, 0.35)]
    body = f"[rounded corners=0.14cm] {L(mirror(right))}"
    s += solid(body, fill, d=0.2)
    s += line((3.0, 2.8), (3.1, 0.35), lw=1.4)
    # wide lapels and the collar turned up
    s += solid(L([(3.0, 3.6), (2.3, 5.35), (1.85, 4.9), (2.1, 4.2)]), f"{fill}!85!ink", d=0.08, opts="rounded corners=0.06cm")
    s += solid(L([(3.0, 3.6), (3.7, 5.35), (4.15, 4.9), (3.9, 4.2)]), f"{fill}!85!ink", d=0.08, opts="rounded corners=0.06cm")
    s += f"\\filldraw[inked, fill={fill}!40!ink] (2.55,5.2) -- (3.45,5.2) -- (3.0,3.95) -- cycle;\n"
    # epaulettes
    for sx in (-1, 1):
        s += solid(L([(3.0 + sx * 0.85, 5.05), (3.0 + sx * 1.55, 4.9), (3.0 + sx * 1.55, 4.72), (3.0 + sx * 0.85, 4.85)]), f"{fill}!85!ink", d=0.04)
    # double-breasted buttons
    for y in (3.75, 3.25):
        for x in (2.55, 3.45):
            s += circle(x, y, 0.1, "wooddark", ",line width=1.0pt")
    # the belt and its buckle
    s += solid(L([(1.35, 2.55), (4.65, 2.55), (4.67, 2.95), (1.33, 2.95)]), f"{fill}!80!ink", d=0.06)
    s += buckle(2.4, 2.75, w=0.5, h=0.5, fill="gold")
    s += box(1.85, 1.95, 2.15, 2.55, f"{fill}!80!ink", r=0.05, d=0.03)
    # pockets, lots of them
    s += flap_pocket(1.5, 3.35, 2.2, 4.0, fill)
    s += flap_pocket(3.8, 3.35, 4.5, 4.0, fill)
    s += flap_pocket(1.35, 1.25, 2.4, 2.25, fill)
    s += flap_pocket(3.6, 1.25, 4.65, 2.25, fill)
    s += flap_pocket(1.4, 0.55, 2.1, 1.0, fill)
    s += flap_pocket(3.9, 0.55, 4.6, 1.0, fill)
    s += seam([(0.95, 0.5), (2.95, 0.5)], f"{fill}!50!ink")
    s += seam([(3.15, 0.5), (5.0, 0.5)], f"{fill}!50!ink")
    s += gloss([(1.55, 4.65), (1.5, 4.25)], lw=2.4, op=0.45)
    return s


def hazmat():
    s = ""
    fill = "gold"
    s += sleeves(fill, top=(4.35, 3.8), end=(5.3, 0.85), hw0=0.6, hw1=0.5, cuff=BLACKPAD)
    right = [(3.0, 4.2), (4.15, 4.25), (4.7, 3.8), (4.75, 0.3), (3.0, 0.3)]
    s += solid(f"[rounded corners=0.2cm] {L(mirror(right))}", fill, d=0.2)
    # taped seams, a reflective band, the zip flap with snaps
    for sx in (-1, 1):
        s += seam([(3.0 + sx * 1.6, 3.9), (3.0 + sx * 1.62, 0.4)], "gold!55!ink")
    s += solid(R(1.27, 1.6, 4.73, 2.05), "metallight", d=0.06)
    s += f"\\fill[white, opacity=0.6] {R(1.3, 1.9, 4.7, 1.98)};\n"
    s += solid(R(2.8, 0.3, 3.3, 4.2), "gold!80!wood", d=0.06)
    for y in (0.75, 2.55, 3.3):
        s += circle(3.05, y, 0.09, BLACKPAD, ",line width=1.0pt")
    s += flap_pocket(3.65, 2.5, 4.4, 3.25, "gold")
    s += tape(1.9, 3.0, 0.75, 0.3, a=-20, fill="stone")
    s += gloss([(1.55, 3.9), (1.45, 2.4)], lw=3.0, op=0.45)
    # the hood up, its face opening dark
    s += hood_up(fill, cy=4.55, rx=1.35, ry=1.2)
    s += f"\\draw[gold!55!ink, line width=0.9pt, dash pattern=on 2.2pt off 1.8pt] {E(3.0, 4.43, 0.98, 0.95)};\n"
    return s


# ---------------------------------------------------------------- hands

def glove(fill, cuff, fingers=(1.0, 1.22, 1.15, 0.88), open_tips=False, cuff_len=0.75, cuff_w=0.9, seam_col=None):
    """One glove about its palm centre, back of the hand facing us,
    fingers up, thumb to the left."""
    s = ""
    seam_col = seam_col or f"{fill}!45!ink"
    # thumb, fingers, then the back of the hand over their roots
    s += solid(blob_path(limb_pts([(-0.55, -0.35), (-1.0, 0.0), (-1.25, 0.5)], 0.25, 0.21, cap=True), 0.5), fill, d=0.1)
    if open_tips:
        s += f"\\filldraw[inked, fill=ink!85!wooddark, rotate around={{-60:(-1.27,0.5)}}] {E(-1.27, 0.5, 0.22, 0.12)};\n"
    for k, ln in enumerate(fingers):
        x = -0.57 + k * 0.38
        top = 0.45 + ln
        s += box(x - 0.18, 0.3, x + 0.18, top, fill, r=0.17, d=0.08)
        if open_tips:
            s += f"\\filldraw[inked, fill=ink!85!wooddark] {E(x, top - 0.05, 0.16, 0.07)};\n"
        else:
            s += line((x - 0.1, top - 0.45), (x + 0.1, top - 0.45), lw=0.9, colour=seam_col)
    s += box(-0.82, -0.95, 0.82, 0.75, fill, r=0.32, d=0.16)
    s += box(-cuff_w, -0.95 - cuff_len, cuff_w, -0.8, cuff, r=0.1, d=0.08)
    return s


def workgloves():
    def one(tilt):
        s = glove("woodlight", "sagelight", cuff_w=0.92)
        s += seam([(-0.6, 0.55), (0.6, 0.55)], "wood!50!ink")
        s += seam([(-0.75, -1.55), (0.75, -1.55)], "sagedark")
        s += f"\\fill[wood!80!woodlight] {E(0.2, -0.2, 0.32, 0.22)};\n"
        s += gloss([(-0.55, -0.1), (-0.5, 0.45)], lw=2.0, op=0.45)
        return s
    return pair(place(1.95, 3.1, 1.1, one(0), rot=12), place(4.05, 2.95, 1.1, one(0), rot=-12, flip=True))


def quickgloves():
    def one():
        s = glove("armourdark", BLACKPAD, fingers=(0.62, 0.7, 0.66, 0.55), open_tips=True, cuff_len=0.45, cuff_w=0.85)
        s += box(-0.75, 0.25, 0.75, 0.62, "armour", r=0.15, d=0.06)
        for x in (-0.38, 0.0, 0.38):
            s += line((x, 0.31), (x, 0.56), lw=1.0, colour="armourdark")
        s += clipped(f"[rounded corners=0.32cm] {R(-0.82, -0.95, 0.82, 0.75)}", zigzag(-0.95, 0.95, -0.25, 0.17, 6, 0.24, "gold"))
        s += box(-0.85, -1.15, 0.85, -0.85, "red", r=0.06, d=0.04)
        s += gloss([(-0.6, -0.65), (-0.62, -0.5)], lw=2.0, op=0.5)
        return s
    return pair(place(1.9, 2.95, 1.1, one(), rot=12), place(4.1, 2.8, 1.1, one(), rot=-12, flip=True))


def medgloves():
    def one(mark):
        s = glove("white", "white!90!metallight", fingers=(1.05, 1.25, 1.18, 0.9), cuff_len=1.1, cuff_w=0.8, seam_col="metallight")
        s += f"\\draw[ink, line width=1.0pt] {P(-0.8, -1.65)} -- {P(0.8, -1.65)};\n"
        s += gloss([(-0.55, -0.4), (-0.55, 0.45)], lw=2.0, op=0.8)
        if mark:
            s += cross(0.05, -0.15, 0.95, "red", bg="white")
        return s
    return pair(place(1.95, 3.25, 1.05, one(False), rot=12), place(4.05, 3.0, 1.05, one(True), rot=-12, flip=True))


# ---------------------------------------------------------------- legs

def pants_outline(waist=(1.65, 5.15), hip=1.35, hem_out=1.25, hem_in=2.7, crotch=3.0, hem=0.55, taper=0.0):
    right = [(3.0, waist[1]), (6.0 - waist[0], waist[1]), (6.0 - hip, 4.0), (6.0 - hem_out - taper, hem),
             (6.0 - hem_in + taper * 0.3, hem), (3.0, crotch)]
    return mirror(right)[:-1]


def pants_body(fill, out, waistband=None):
    s = solid(f"[rounded corners=0.1cm] {L(out)}", fill, d=0.2)
    s += box(1.6, 4.65, 4.4, 5.2, waistband or f"{fill}!85!ink", r=0.06, d=0.06)
    return s


def jeans():
    s = ""
    out = pants_outline()
    s += pants_body(DENIM, out)
    # belt loops, button, fly, pockets, a worn knee, orange stitching
    for x in (1.9, 2.55, 3.45, 4.1):
        s += box(x - 0.08, 4.55, x + 0.08, 5.28, DENIM, r=0.03, d=0.03)
    s += circle(3.0, 4.92, 0.11, "metallight", ",line width=1.0pt")
    s += seam([(3.3, 4.6), (3.3, 3.6), (3.0, 3.3)], "orange")
    s += line((3.0, 4.6), (3.0, 3.1), lw=1.1)
    for sx in (-1, 1):
        s += f"\\draw[ink, line width=1.1pt] {P(3.0 + sx * 0.95, 4.65)} .. controls {P(3.0 + sx * 0.95, 4.15)} and {P(3.0 + sx * 1.3, 4.0)} .. {P(3.0 + sx * 1.62, 3.95)};\n"
        s += seam([(3.0 + sx * 1.0, 0.75), (3.0 + sx * 1.7, 0.75)], "orange")
        s += rivet(3.0 + sx * 1.55, 4.45, r=0.06, fill="gold")
    s += f"\\fill[{DENIM}!55!white] {E(1.95, 2.3, 0.38, 0.5)};\n"
    s += f"\\fill[{DENIM}!55!white] {E(4.05, 2.15, 0.36, 0.45)};\n"
    for y in (2.15, 2.3, 2.45):
        s += line((1.8, y), (2.1, y), lw=0.8, colour="white")
    s += gloss([(1.55, 3.9), (1.45, 1.2)], lw=2.4, op=0.35)
    return s


def cargo():
    s = ""
    fill = "sagedark"
    out = pants_outline(hem_out=1.3, hem_in=2.65)
    s += pants_body(fill, out)
    for x in (1.9, 2.55, 3.45, 4.1):
        s += box(x - 0.08, 4.55, x + 0.08, 5.28, fill, r=0.03, d=0.03)
    s += line((3.0, 4.6), (3.0, 3.1), lw=1.1)
    s += box(2.92, 4.75, 3.08, 5.1, "metallight", r=0.03, d=0.02)
    # big bellows pockets on both thighs
    for sx in (-1, 1):
        x0 = 3.0 + sx * 1.75
        x1 = 3.0 + sx * 0.6
        lo, hi = min(x0, x1), max(x0, x1)
        s += box(lo, 1.75, hi, 3.35, f"{fill}!92!wood", r=0.12, d=0.14)
        s += line((lo + 0.15, 1.9), (lo + 0.15, 3.2), lw=1.0, colour=f"{fill}!50!ink")
        s += solid(L([(lo - 0.05, 3.5), (hi + 0.05, 3.5), (hi + 0.05, 3.05), (lo - 0.05, 3.05)]), f"{fill}!80!ink", d=0.05, opts="rounded corners=0.06cm")
        s += circle((lo + hi) / 2, 3.15, 0.08, "metallight", ",line width=1.0pt")
        s += seam([(lo + 0.1, 1.9), (hi - 0.1, 1.9)], f"{fill}!45!ink")
        s += box(3.0 + sx * 1.65 - 0.25, 0.55, 3.0 + sx * 1.65 + 0.25, 0.85, f"{fill}!80!ink", r=0.04, d=0.03)
    s += tape(4.2, 1.15, 0.55, 0.28, a=15, fill="stone")
    s += gloss([(1.5, 4.0), (1.45, 3.6)], lw=2.4, op=0.4)
    return s


def joggers():
    s = ""
    fill = "armourdark"
    out = pants_outline(hem_out=1.75, hem_in=2.6, taper=0.0)
    s += pants_body(fill, out, waistband="armour")
    for k in range(9):
        x = 1.75 + k * 0.31
        s += line((x, 4.72), (x, 5.12), lw=0.9, colour="armourdark")
    # drawstring
    s += curve([(2.85, 4.75), (2.75, 4.2), (2.6, 3.8)], lw=2.0)
    s += curve([(3.15, 4.75), (3.25, 4.2), (3.4, 3.8)], lw=2.0)
    s += curve([(2.85, 4.75), (2.75, 4.2), (2.6, 3.8)], lw=0.9, colour="cream")
    s += curve([(3.15, 4.75), (3.25, 4.2), (3.4, 3.8)], lw=0.9, colour="cream")
    # two white side stripes down each leg
    for sx in (-1, 1):
        for off in (0.0, 0.22):
            a = (3.0 + sx * (1.45 - off), 4.6)
            b = (3.0 + sx * (1.75 - off * 0.8), 1.0)
            s += line(a, b, lw=5.2)
            s += line(a, b, lw=3.0, colour="white")
        # ribbed cuffs, gathered
        s += box(3.0 + sx * 1.72 - 0.47, 0.45, 3.0 + sx * 1.72 + 0.47, 0.95, "armour", r=0.12, d=0.06)
        for k in range(4):
            x = 3.0 + sx * 1.72 - 0.3 + k * 0.2
            s += line((x, 0.55), (x, 0.85), lw=0.9, colour="armourdark")
    s += gloss([(1.85, 4.3), (1.95, 2.0)], lw=2.4, op=0.35)
    return s


# ---------------------------------------------------------------- knees

def patch(fill, a):
    s = ""
    pth = f"[rounded corners=0.35cm] {R(-0.95, -1.2, 0.95, 1.2)}"
    s += solid(pth, fill, d=0.16)
    s += stitch(f"[rounded corners=0.25cm] {R(-0.8, -1.05, 0.8, 1.05)}", "gold", lw=1.1)
    for k in range(5):
        y = -0.8 + k * 0.4
        s += line((0.95, y), (1.13, y - 0.04), lw=1.0, colour="white!60!violet")
    s += f"\\fill[white, opacity=0.25] {E(-0.15, 0.25, 0.45, 0.6)};\n"
    s += gloss([(-0.65, 0.4), (-0.6, 0.85)], lw=2.0, op=0.5)
    return place(0, 0, 1, s, rot=a)


def patches():
    s = pair(place(1.85, 3.3, 1.08, patch(DENIM, 8)), place(4.15, 2.8, 1.08, patch(DENIM + "!85!white", -10)))
    s += f"\\draw[ink, line width=4.4pt, line cap=round] (3.85,4.5) -- (4.75,4.95);\n"
    s += f"\\draw[metallight, line width=2.2pt, line cap=round] (3.85,4.5) -- (4.75,4.95);\n"
    s += circle(4.85, 5.0, 0.13, "metallight", ",line width=1.2pt")
    return s


def kneepad():
    s = ""
    # straps out to the sides, top and bottom
    for y in (0.8, -0.8):
        s += box(-1.3, y - 0.2, 1.3, y + 0.2, BLACKPAD, r=0.06, d=0.05)
    s += blobby([(0, 1.6), (0.95, 1.1), (1.05, -0.2), (0.75, -1.4), (0, -1.75), (-0.75, -1.4), (-1.05, -0.2), (-0.95, 1.1)], BLACKPAD, d=0.12)
    shell = [(0, 1.3), (0.75, 0.9), (0.82, -0.1), (0.55, -1.15), (0, -1.4), (-0.55, -1.15), (-0.82, -0.1), (-0.75, 0.9)]
    s += blobby(shell, "armour", d=0.18)
    s += curve([(-0.5, 0.35), (0, 0.55), (0.5, 0.35)], lw=1.2, colour="armourdark")
    s += curve([(-0.5, -0.35), (0, -0.15), (0.5, -0.35)], lw=1.2, colour="armourdark")
    s += gloss([(-0.55, 0.15), (-0.45, 0.8)], lw=2.4, op=0.55)
    s += rivet(0, 1.0)
    s += rivet(0, -1.1)
    s += buckle(1.25, 0.8, w=0.3, h=0.5)
    return s


def kneepads():
    left = kneepad()
    right = kneepad() + tape(0.15, 0.1, 0.9, 0.3, a=30, fill="red")
    return pair(place(1.75, 3.1, 1.2, left, rot=6), place(4.25, 2.9, 1.2, right, rot=-6, flip=True))


def spring_brace():
    s = ""
    # padded cuffs above and below the knee
    for y0, y1 in ((1.15, 1.9), (-1.9, -1.15)):
        s += box(-1.05, y0, 1.05, y1, "tealdark", r=0.2, d=0.1)
        s += seam([(-0.85, (y0 + y1) / 2), (0.85, (y0 + y1) / 2)], "teal!50!white")
    # the hinged side bar
    s += box(-0.85, 0.15, -0.55, 1.65, STEEL, r=0.08, d=0.06)
    s += box(-0.85, -1.65, -0.55, -0.15, STEEL, r=0.08, d=0.06)
    s += disc(-0.7, 0, 0.32, "metallight")
    s += f"\\fill[ink] {C(-0.7, 0, 0.08)};\n"
    # the coil spring on the other side, between two mounts
    s += box(0.25, 0.95, 0.95, 1.25, BLUED, r=0.06, d=0.05)
    s += box(0.25, -1.25, 0.95, -0.95, BLUED, r=0.06, d=0.05)
    zz = [(0.6, 0.95)] + [(0.6 + (0.32 if k % 2 else -0.32), 0.85 - k * 0.2) for k in range(10)] + [(0.6, -0.95)]
    s += tube([zz], 0.12, "gold", tension=0.35)
    for x in (-0.7,):
        for y in (1.35, -1.35):
            s += rivet(x, y)
    return s


def springs():
    return pair(place(1.6, 3.1, 1.08, spring_brace(), rot=4), place(4.4, 2.9, 1.08, spring_brace(), rot=-4, flip=True))


# ---------------------------------------------------------------- feet

def shoe(style):
    """One shoe about its own origin, side view, toe to the right, the
    sole's foot on y = -0.3."""
    s = ""
    if style == "sneaker":
        upper, sole, trim = "redlight", "cream", "cream"
        up = [(-1.5, 0.1), (-1.55, 0.95), (-1.3, 1.3), (-0.75, 1.15), (-0.35, 1.5), (-0.05, 1.42), (0.35, 0.95), (1.05, 0.68), (1.55, 0.42), (1.65, 0.1)]
        s += solid(f"[rounded corners=0.2cm] {L(up)}", upper, d=0.14)
        s += solid("(0.9,0.1) -- (1.65,0.1) -- (1.55,0.42) .. controls (1.3,0.62) and (1.05,0.7) .. (0.9,0.7) -- cycle", trim, d=0.06)
        s += solid("(-1.55,0.1) -- (-0.95,0.1) -- (-0.95,1.1) -- (-1.35,1.25) -- (-1.55,0.95) -- cycle", f"{upper}!80!ink", d=0.05)
        s += solid(f"[rounded corners=0.18cm] {R(-1.65, -0.3, 1.8, 0.18)}", sole, d=0.08)
        s += seam([(-1.45, -0.05), (1.6, -0.05)], "cream!60!ink")
        for k in range(4):
            x = -0.15 + k * 0.28
            y = 1.25 - k * 0.2
            s += line((x - 0.12, y - 0.1), (x + 0.16, y + 0.06), lw=1.4, colour="white")
            s += f"\\fill[ink] {C(x - 0.12, y - 0.1, 0.04)};\n"
        s += gloss([(-1.3, 0.55), (-1.25, 1.0)], lw=1.8, op=0.5)
    elif style == "runner":
        upper, sole = "gold", "white"
        up = [(-1.5, 0.15), (-1.5, 0.75), (-1.25, 0.98), (-0.75, 0.85), (-0.35, 1.08), (0.05, 1.0), (0.6, 0.62), (1.45, 0.38), (1.75, 0.12)]
        s += solid(f"[rounded corners=0.22cm] {L(up)}", upper, d=0.12)
        s += curve([(-1.2, 0.35), (-0.3, 0.6), (0.7, 0.35)], lw=3.4, colour="ink")
        s += curve([(-1.2, 0.35), (-0.3, 0.6), (0.7, 0.35)], lw=1.6, colour="orange")
        s += solid("(-1.65,-0.3) -- (1.8,-0.3) .. controls (1.95,-0.15) and (1.9,0.1) .. (1.75,0.15) -- (-1.5,0.3) .. controls (-1.7,0.2) and (-1.75,-0.1) .. (-1.65,-0.3) -- cycle", sole, d=0.06)
        for x in (-1.2, -0.7, -0.2, 0.3, 0.8, 1.3):
            s += line((x, -0.3), (x + 0.15, -0.12), lw=1.0, colour="metal")
        for k in range(3):
            x = -0.1 + k * 0.25
            s += line((x - 0.1, 0.92 - k * 0.12), (x + 0.12, 1.0 - k * 0.12), lw=1.2, colour="white")
        s += gloss([(-1.3, 0.5), (-1.2, 0.8)], lw=1.8, op=0.6)
    elif style == "sock":
        upper, sole = "pink", "cream"
        up = [(-1.45, 0.15), (-1.5, 1.85), (-0.55, 1.85), (-0.35, 1.0), (0.4, 0.8), (1.4, 0.55), (1.7, 0.15)]
        s += solid(f"[rounded corners=0.35cm] {L(up)}", upper, d=0.14)
        s += clipped(f"[rounded corners=0.35cm] {L(up)}", "".join(line((x, 0.3), (x + 0.1, 1.95), lw=0.9, colour="pink!60!ink") for x in (-1.3, -1.1, -0.9, -0.7)))
        s += box(-1.6, 1.45, -0.4, 1.95, "pink!70!white", r=0.15, d=0.06)
        for x in (-1.4, -1.2, -1.0, -0.8, -0.6):
            s += line((x, 1.55), (x, 1.85), lw=0.9, colour="pink!55!ink")
        s += blobby([(-1.7, -0.05), (-1.5, 0.3), (0.0, 0.32), (1.6, 0.3), (1.9, 0.0), (1.6, -0.35), (0.0, -0.38), (-1.5, -0.35)], sole, d=0.1, tension=0.6)
        for x in (-0.9, -0.1, 0.7):
            s += line((x, -0.25), (x, 0.2), lw=1.0, colour="stonedark")
        s += gloss([(0.4, 0.62), (1.0, 0.48)], lw=1.8, op=0.5)
    else:  # boot
        upper, sole = "wood", BLACKPAD
        up = [(-1.4, 0.2), (-1.45, 2.3), (-0.25, 2.35), (-0.1, 1.2), (0.85, 0.9), (1.5, 0.65), (1.65, 0.2)]
        s += solid(f"[rounded corners=0.18cm] {L(up)}", upper, d=0.16)
        s += box(-1.55, 2.05, -0.15, 2.45, "wooddark", r=0.1, d=0.05)
        # lace hooks up the front, crossed laces
        for k in range(4):
            y = 0.95 + k * 0.33
            x = -0.15 - k * 0.02
            s += line((x - 0.35, y), (x + 0.05, y + 0.18), lw=1.4, colour="cream")
            s += rivet(x - 0.35, y, r=0.06, fill="metallight")
        # the steel toe, scuffed through
        s += solid("(0.65,0.2) -- (1.65,0.2) -- (1.5,0.65) .. controls (1.2,0.85) and (0.85,0.95) .. (0.65,0.95) -- cycle", "metallight", d=0.08)
        s += rivet(0.9, 0.5, r=0.06)
        s += line((1.05, 0.75), (1.35, 0.6), lw=1.0, colour="metal")
        s += solid(f"[rounded corners=0.1cm] {R(-1.6, -0.35, 1.8, 0.22)}", sole, d=0.06)
        for x in (-1.35, -0.85, -0.35, 0.15, 0.65, 1.15):
            s += f"\\fill[ink] {R(x, -0.38, x + 0.25, -0.24)};\n"
        s += box(-1.4, 0.4, -0.35, 0.75, "wooddark", r=0.08, d=0.05)
        s += gloss([(-1.2, 0.9), (-1.15, 1.9)], lw=2.0, op=0.4)
    return s


def shoe_pair(style, s=1.15, back=(3.3, 3.45), front=(2.7, 1.55)):
    return pair(place(back[0], back[1], s, shoe(style)), place(front[0], front[1], s, shoe(style)))


def sneakers():
    return shoe_pair("sneaker")


def runners():
    s = ""
    for (x0, x1, y) in ((0.35, 1.15, 4.3), (0.25, 1.0, 3.75), (0.35, 0.95, 2.35), (0.2, 1.05, 1.8)):
        s += line((x0, y), (x1, y), lw=2.4)
    s += shoe_pair("runner", back=(3.45, 3.6), front=(2.95, 1.75))
    return s


def sockshoes():
    return shoe_pair("sock", s=1.05, back=(3.35, 3.4), front=(2.65, 1.3))


def boots():
    return shoe_pair("boot", s=1.0, back=(3.55, 3.15), front=(2.55, 0.85))


# ---------------------------------------------------------------- belt

def belt():
    s = ""
    web = "wooddark"
    cx, cy = 3.0, 3.0
    ring = f"{E(cx, cy, 2.55, 1.45)} {E(cx, cy, 2.05, 1.0)}"
    s += "\\begin{scope}[even odd rule]\n" + solid(ring, web, d=0.16) + "\\end{scope}\n"
    s += stitch(E(cx, cy, 2.45, 1.36), "woodlight")
    s += stitch(E(cx, cy, 2.15, 1.09), "woodlight")
    # empty keepers and D-rings round the band
    for a in (200, 240, 300, 340):
        x, y = cx + 2.3 * math.cos(math.radians(a)), cy + 1.22 * math.sin(math.radians(a))
        s += box(x - 0.14, y - 0.3, x + 0.14, y + 0.3, "wood", r=0.04, d=0.04)
    for a in (160, 20):
        x, y = cx + 2.3 * math.cos(math.radians(a)), cy + 1.22 * math.sin(math.radians(a))
        s += f"\\draw[ink, line width=4.4pt] {P(x - 0.2, y + 0.2)} arc[start angle=180, end angle=0, radius=0.2];\n"
        s += f"\\draw[metallight, line width=2.2pt] {P(x - 0.2, y + 0.2)} arc[start angle=180, end angle=0, radius=0.2];\n"
    # the buckle at the front
    s += box(2.35, 1.2, 3.65, 2.1, "metallight", r=0.12, d=0.1)
    s += box(2.6, 1.42, 3.4, 1.88, "wood", r=0.06, d=0.0)
    s += line((3.0, 1.42), (3.0, 1.88), lw=2.0)
    s += gloss([(2.45, 2.0), (2.75, 2.0)], lw=1.6, op=0.7)
    s += f"\\draw[white, opacity=0.4, line width=2.2pt, line cap=round] {P(cx - 2.35, cy + 0.3)} arc[start angle=165, end angle=120, radius=2.3];\n"
    return s


def mines():
    s = ""
    # the belt across, the pouch on it
    s += box(-0.2, 1.35, 6.2, 2.05, "wooddark", r=0.0, d=0.12)
    s += seam([(0.1, 1.48), (5.9, 1.48)], "woodlight")
    s += seam([(0.1, 1.92), (5.9, 1.92)], "woodlight")
    # the mine: a squat drum seen from above the rim
    s += f"\\fill[red, opacity=0.25] {C(3.0, 4.45, 0.95)};\n"
    s += solid(f"{P(1.55, 3.55)} -- {P(1.55, 2.95)} arc[start angle=180, end angle=360, x radius=1.45, y radius=0.5] -- {P(4.45, 3.55)} -- cycle", "greenE", d=0.1)
    s += solid(E(3.0, 3.55, 1.45, 0.52), "greenD", d=0.08)
    s += f"\\draw[greenE!50!ink, line width=1.0pt] {E(3.0, 3.55, 1.05, 0.36)};\n"
    for a in (20, 160, 250, 300):
        s += rivet(3.0 + 1.25 * math.cos(math.radians(a)), 3.55 + 0.44 * math.sin(math.radians(a)), r=0.06)
    s += solid("(2.65,3.6) -- (2.65,3.85) .. controls (2.65,4.35) and (3.35,4.35) .. (3.35,3.85) -- (3.35,3.6) -- cycle", "red", d=0.08)
    s += f"\\draw[ink, line width=1.2pt] {E(3.0, 3.6, 0.35, 0.12)};\n"
    s += glint(2.85, 4.0, 0.08)
    for k, r in enumerate((0.6, 0.8)):
        s += f"\\draw[red, line width=1.4pt, line cap=round, opacity=0.8] {P(3.0 + r * math.cos(math.radians(35)), 3.8 + r * math.sin(math.radians(35)))} arc[start angle=35, end angle=70, radius={f(r)}];\n"
        s += f"\\draw[red, line width=1.4pt, line cap=round, opacity=0.8] {P(3.0 + r * math.cos(math.radians(110)), 3.8 + r * math.sin(math.radians(110)))} arc[start angle=110, end angle=145, radius={f(r)}];\n"
    # the pouch's front, over the drum's foot, with its flap folded back
    s += solid("(1.35,0.6) -- (4.65,0.6) -- (4.75,3.0) .. controls (3.9,2.75) and (2.1,2.75) .. (1.25,3.0) -- cycle", "sagedark", d=0.16, opts="rounded corners=0.12cm")
    s += seam([(1.5, 2.7), (2.4, 2.55), (3.6, 2.55), (4.5, 2.7)], "sagedark!45!ink")
    s += box(2.55, 1.35, 3.45, 2.1, "sagedark!80!ink", r=0.06, d=0.05)
    s += circle(3.0, 1.75, 0.1, BLACKPAD, ",line width=1.0pt")
    s += tape(1.8, 1.4, 0.4, 0.95, a=-6, fill="stone")
    s += gloss([(1.55, 2.6), (1.5, 1.2)], lw=2.0, op=0.4)
    return s


def canister(fill, h=1.6, r=0.5):
    s = ""
    s += solid(f"{P(-r, h)} -- {P(-r, 0)} arc[start angle=180, end angle=360, x radius={f(r)}, y radius={f(r * 0.35)}] -- {P(r, h)} -- cycle", fill, d=0.12)
    s += solid(f"{P(-r, 0.35)} -- {P(-r, 0.15)} arc[start angle=180, end angle=360, x radius={f(r)}, y radius={f(r * 0.35)}] -- {P(r, 0.35)} arc[start angle=0, end angle=-180, x radius={f(r)}, y radius={f(r * 0.35)}] -- cycle", f"{fill}!75!ink", d=0.0)
    s += solid(E(0, h, r, r * 0.35), "metallight", d=0.05)
    for (x, y) in ((-0.15, h), (0.0, h + 0.06), (0.15, h), (0.0, h - 0.07)):
        s += f"\\fill[ink] {C(x, y, 0.035)};\n"
    s += f"\\fill[white, opacity=0.45, rounded corners=0.05cm] {R(-r + 0.1, 0.5, -r + 0.22, h - 0.25)};\n"
    return s


def decoys():
    s = ""
    s += place(1.6, 1.0, 1.0, canister("red", h=2.0, r=0.55))
    s += place(3.05, 0.75, 1.0, canister("teal", h=1.55, r=0.5))
    s += place(4.4, 1.15, 1.0, canister("violet", h=1.85, r=0.5) + tape(0, 1.0, 0.75, 0.3, a=8, fill="stone"))
    s += sound(1.6, 3.0, 50, 130, (0.55, 0.85, 1.15))
    s += sound(3.05, 2.3, 40, 140, (0.5, 0.8))
    s += sound(4.4, 3.0, 55, 125, (0.5, 0.8, 1.1))
    return s


def paint():
    s = ""
    rng = random.Random("gear/paint")
    # the painted stripe behind the can, with drips
    stripe = "(0.1,4.6) .. controls (1.5,5.05) and (3.0,4.2) .. (5.9,4.85) -- (5.9,3.85) .. controls (4.5,3.45) and (3.6,3.75) .. (3.3,3.7) -- (3.25,3.15) .. controls (3.25,3.0) and (3.05,3.0) .. (3.05,3.15) -- (3.0,3.7) .. controls (2.0,3.8) and (1.5,3.5) .. (1.35,3.55) -- (1.3,2.95) .. controls (1.3,2.75) and (1.1,2.75) .. (1.1,2.95) -- (1.05,3.6) .. controls (0.8,3.7) and (0.4,3.6) .. (0.1,3.6) -- cycle"
    s += solid(stripe, "cyan", d=0.12)
    s += gloss([(0.4, 4.45), (1.4, 4.65), (2.4, 4.4)], lw=2.0, op=0.5)
    # the can, tilted, nozzle up and to the right
    can = ""
    can += box(-0.75, -1.9, 0.75, 1.15, "cyan", r=0.2, d=0.18)
    can += box(-0.75, -0.75, 0.75, 0.15, "white", r=0.0, d=0.06)
    can += f"\\fill[cyan] {R(-0.73, -0.45, 0.73, -0.15)};\n"
    can += line((-0.73, -0.45), (0.73, -0.45), lw=1.0)
    can += line((-0.73, -0.15), (0.73, -0.15), lw=1.0)
    can += solid("(-0.62,1.1) .. controls (-0.55,1.6) and (0.55,1.6) .. (0.62,1.1) -- cycle", "metallight", d=0.06)
    can += box(-0.18, 1.45, 0.22, 1.85, "cyan", r=0.08, d=0.05)
    can += box(0.15, 1.6, 0.35, 1.74, BLACKPAD, r=0.03, d=0.0)
    can += box(-0.75, -1.9, 0.75, -1.6, "metallight", r=0.08, d=0.04)
    can += f"\\fill[white, opacity=0.55, rounded corners=0.06cm] {R(-0.6, -1.5, -0.42, 0.95)};\n"
    can += f"\\fill[cyan, opacity=0.7] {C(0.3, -1.1, 0.1)} {C(0.45, -1.35, 0.06)};\n"
    s += place(2.8, 2.15, 1.0, can, rot=-15)
    # mist from the nozzle
    nx, ny = rot((0.35, 1.67), -15)
    nx, ny = nx + 2.8, ny + 2.15
    for k in range(18):
        a = math.radians(rng.uniform(-20, 25))
        d = rng.uniform(0.35, 1.9)
        s += f"\\fill[cyan, opacity={f(rng.uniform(0.5, 0.9))}] {C(nx + d * math.cos(a), ny + d * math.sin(a), rng.uniform(0.04, 0.1))};\n"
    return s


def pellet(fill):
    s = disc(0, 0, 0.55, fill)
    s += clipped(C(0, 0, 0.55), f"\\filldraw[inked, fill={fill}!70!ink] {R(-0.6, -0.12, 0.6, 0.12)};\n")
    s += tube([[(0.0, 0.55), (0.0, 0.8)]], 0.1, "metallight")
    s += f"\\draw[ink, line width=3.6pt] {C(0.0, 0.97, 0.16)};\n\\draw[metallight, line width=1.6pt] {C(0.0, 0.97, 0.16)};\n"
    s += glint(-0.22, 0.25, 0.1)
    return s


def smoke():
    s = ""
    puffs = [(3.0, 4.3, 0.95), (4.15, 4.6, 0.8), (2.0, 4.85, 0.7), (4.6, 3.6, 0.65), (3.6, 3.45, 0.7), (2.15, 3.7, 0.62), (3.15, 5.25, 0.6)]
    for x, y, r in puffs:
        s += circle(x, y, r, "metallight")
    s += "".join(f"\\fill[metallight] {C(x, y, r - 0.04)};\n" for x, y, r in puffs)
    for x, y, r in puffs[:4]:
        s += f"\\fill[white, opacity=0.55] {C(x - r * 0.3, y + r * 0.3, r * 0.35)};\n"
    s += f"\\fill[metal, opacity=0.6] (2.4,3.2) .. controls (3.0,2.9) and (3.8,3.0) .. (4.3,3.1) .. controls (3.8,3.5) and (3.0,3.4) .. (2.4,3.2) -- cycle;\n"
    s += tube([[(1.95, 1.95), (2.1, 2.5), (2.4, 2.9)]], 0.3, "metallight", tension=0.5)
    s += place(1.75, 1.55, 1.0, pellet("greenD"), rot=10)
    s += place(3.25, 1.2, 1.0, pellet("greenD"), rot=-15)
    s += place(4.55, 1.6, 0.9, pellet("orange!70!wood"), rot=25)
    return s


# ---------------------------------------------------------------- melee

def bat():
    b = ""
    body = "(-2.55,-0.16) -- (-0.9,-0.16) .. controls (0.0,-0.18) and (0.4,-0.44) .. (1.0,-0.44) -- (2.3,-0.44) arc[start angle=-90, end angle=90, x radius=0.26, y radius=0.44] -- (1.0,0.44) .. controls (0.4,0.44) and (0.0,0.18) .. (-0.9,0.16) -- (-2.55,0.16) -- cycle"
    b += solid(body, "metallight", d=0.14)
    b += f"\\fill[white, opacity=0.6, rounded corners=0.05cm] (-0.6,0.12) .. controls (0.1,0.12) and (0.5,0.32) .. (1.0,0.32) -- (2.3,0.32) -- (2.3,0.22) -- (1.0,0.22) .. controls (0.5,0.22) and (0.1,0.04) .. (-0.6,0.04) -- cycle;\n"
    # grip tape, wound
    grip = R(-2.6, -0.2, -1.05, 0.2)
    wraps = f"\\fill[ink!80!metaldark] {grip};\n" + "".join(line((x, -0.25), (x + 0.2, 0.25), lw=1.0, colour="metal") for x in [-2.55 + k * 0.22 for k in range(8)])
    b += clipped(grip, wraps)
    b += f"\\draw[inked] {grip};\n"
    b += ellipse(-2.65, 0, 0.13, 0.33, "metallight")
    b += box(-1.25, -0.24, -1.0, 0.24, "red", r=0.04, d=0.03)
    # dents
    for x in (1.35, 1.85):
        b += f"\\draw[metal, line width=1.0pt] {P(x, -0.2)} arc[start angle=-120, end angle=-60, radius=0.25];\n"
    return place(3.0, 3.0, 1.22, b, rot=45)


def sledge():
    b = ""
    rng = random.Random("gear/sledge")
    handle = f"[rounded corners=0.18cm] {R(-2.75, -0.2, 1.6, 0.2)}"
    b += solid(handle, "wood", d=0.1)
    b += clipped(handle, "".join(curve([(x, rng.uniform(-0.08, 0.08)), (x + 0.6, rng.uniform(-0.08, 0.08))], lw=0.9, colour="wooddark") for x in (-2.3, -1.2, -0.2)))
    b += tape(-2.0, 0, 1.1, 0.48, fill="red")
    # the head, across the end of the handle
    b += solid(f"{R(1.2, -1.35, 2.25, 1.35)}", "metal", d=0.18, opts="rounded corners=0.12cm")
    b += box(1.05, -1.42, 2.4, -1.05, "metal!70!ink", r=0.1, d=0.06)
    b += box(1.05, 1.05, 2.4, 1.42, "metal!70!ink", r=0.1, d=0.06)
    b += f"\\fill[white, opacity=0.5, rounded corners=0.05cm] {R(1.3, -0.95, 1.45, 0.95)};\n"
    b += line((1.95, -0.6), (2.1, -0.45), lw=1.0, colour="metaldark")
    b += box(1.0, -0.26, 1.25, 0.26, "metaldark", r=0.04, d=0.03)
    b += weld([(1.7, -0.3), (1.7, -0.1), (1.7, 0.1)])
    return place(2.75, 2.8, 1.18, b, rot=45)


def machete():
    b = ""
    blade = "(-0.6,0.3) -- (2.3,0.45) .. controls (2.8,0.48) and (3.05,0.3) .. (3.0,0.02) .. controls (2.75,-0.62) and (1.85,-0.78) .. (1.0,-0.62) -- (-0.6,-0.35) -- cycle"
    b += solid(blade, "metallight", d=0.14)
    b += clipped(blade, "\\fill[white!80!metallight] (-0.6,-0.18) -- (1.0,-0.42) .. controls (1.85,-0.56) and (2.65,-0.42) .. (2.85,-0.05) -- (3.1,-0.1) -- (3.1,-1.0) -- (-0.6,-1.0) -- cycle;\n")
    b += f"\\draw[inked] {blade};\n"
    b += line((-0.4, 0.15), (2.2, 0.28), lw=1.0, colour="metal")
    for (x, y, r) in ((0.3, 0.05, 0.09), (0.55, -0.1, 0.06), (1.9, 0.2, 0.07)):
        b += f"\\fill[wood!80!orange] {C(x, y, r)};\n"
    # guard, and the cord-wrapped handle
    b += box(-0.75, -0.55, -0.5, 0.5, BLUED, r=0.06, d=0.05)
    hd = f"[rounded corners=0.15cm] {R(-2.75, -0.3, -0.72, 0.3)}"
    b += solid(hd, "wooddark", d=0.08)
    wraps = "".join(tube([[(x, -0.35), (x + 0.22, 0.35)]], 0.1, "red!70!wood") for x in [-2.55 + k * 0.28 for k in range(7)])
    b += clipped(hd, wraps)
    b += f"\\draw[inked] {hd};\n"
    b += rivet(-2.45, 0, r=0.08)
    b += gloss([(-0.3, 0.2), (1.8, 0.33)], lw=1.6, op=0.6)
    return place(3.05, 3.05, 1.14, b, rot=38)


# ---------------------------------------------------------------- ammo

def round_up(x, y, s=1.0, case=BRASS, tip="orange!55!wood", hw=0.2, h=0.55, nose=0.4):
    """A pistol round standing up: case, then a rounded bullet."""
    out = solid(f"{P(x - hw * s, y)} -- {P(x - hw * s, y + h * s)} .. controls {P(x - hw * s, y + (h + nose) * s)} and {P(x + hw * s, y + (h + nose) * s)} .. {P(x + hw * s, y + h * s)} -- {P(x + hw * s, y)} -- cycle", tip, d=0.05)
    out += solid(R(x - hw * s, y - 0.3 * s, x + hw * s, y + h * s), case, d=0.05)
    out += f"\\fill[white, opacity=0.55] {R(x - hw * s * 0.6, y - 0.2 * s, x - hw * s * 0.25, y + h * s * 0.85)};\n"
    return out


def ammo_light():
    s = ""
    cb = "woodlight"
    # the lid flap open behind
    s += solid(L([(1.85, 3.25), (4.85, 3.25), (4.3, 4.85), (1.3, 4.85)]), f"{cb}!92!ink", d=0.12)
    s += f"\\filldraw[inked, fill={cb}!35!ink] {L([(1.0, 2.75), (4.3, 2.75), (5.05, 3.3), (1.75, 3.3)])};\n"
    # two rows of rounds, back row first
    for row, (y0, dx) in enumerate(((2.85, 0.37), (2.45, 0.0))):
        for k in range(6):
            s += round_up(1.55 + dx + k * 0.52, y0, s=1.0)
    # the box: front and side
    s += solid(L([(1.0, 0.9), (4.3, 0.9), (4.3, 2.75), (1.0, 2.75)]), cb, d=0.16)
    s += solid(L([(4.3, 0.9), (5.05, 1.45), (5.05, 3.3), (4.3, 2.75)]), f"{cb}!80!ink", d=0.06)
    s += tape(2.65, 1.85, 2.2, 0.36, fill="cream")
    s += box(1.25, 1.1, 1.95, 1.45, "red", r=0.05, d=0.03)
    s += line((1.0, 2.75), (4.3, 2.75), lw=1.6)
    # loose rounds in front
    s += place(4.4, 0.65, 0.75, f"\\begin{{scope}}[rotate=-90]\n{round_up(0, 0)}\\end{{scope}}\n")
    s += place(0.9, 0.55, 0.75, f"\\begin{{scope}}[rotate=75]\n{round_up(0, 0)}\\end{{scope}}\n")
    return s


def rifle_round():
    """A long rifle round standing up, its rim at y = -2.0."""
    s = ""
    s += solid(R(-0.3, -2.05, 0.3, -1.85), "gold!80!ink", d=0.05)
    s += solid(L([(-0.27, -1.85), (0.27, -1.85), (0.27, 0.35), (0.14, 0.62), (0.14, 0.95), (-0.14, 0.95), (-0.14, 0.62), (-0.27, 0.35)]), BRASS, d=0.08)
    s += solid("(-0.13,0.95) .. controls (-0.13,1.5) and (-0.04,1.75) .. (0,1.85) .. controls (0.04,1.75) and (0.13,1.5) .. (0.13,0.95) -- cycle", "orange!55!wood", d=0.04)
    s += f"\\fill[white, opacity=0.55, rounded corners=0.04cm] {R(-0.19, -1.7, -0.1, 0.25)};\n"
    return s


def ammo_heavy():
    g = ""
    for k in range(5):
        g += place(-1.24 + k * 0.62, 0.0, 1.0, rifle_round())
    # the stripper clip across the rims
    g += box(-1.65, -2.2, 1.65, -1.75, "metal", r=0.06, d=0.08)
    g += line((-1.55, -1.85), (1.55, -1.85), lw=1.0, colour="metaldark")
    g += f"\\fill[white, opacity=0.5] {R(-1.55, -2.05, 1.55, -1.98)};\n"
    g += box(-1.75, -2.25, -1.55, -1.7, "metaldark", r=0.03, d=0.02)
    g += box(1.55, -2.25, 1.75, -1.7, "metaldark", r=0.03, d=0.02)
    return place(3.05, 3.0, 1.15, g, rot=-18)


def ammo_shells():
    s = ""
    for k, x in enumerate((1.45, 2.5, 3.55, 4.6)):
        s += place(x, 3.55 + (0.12 if k % 2 else 0), 0.68, shotshell(), rot=90)
    s += place(2.9, 1.25, 0.72, shotshell(), rot=-8)
    s += f"\\fill[ink, opacity=0.15] {E(3.0, 2.15, 2.3, 0.12)};\n"
    return s


def rocket(body="greenD", nose="red"):
    """A small rocket standing up, its tail at y = 0."""
    s = ""
    for sx in (-1, 1):
        s += solid(L([(sx * 0.3, 0.2), (sx * 0.7, -0.1), (sx * 0.7, 0.35), (sx * 0.3, 0.9)]), "metal", d=0.05)
    s += box(-0.38, 0.0, 0.38, 2.6, body, r=0.05, d=0.12)
    s += solid("(-0.38,2.55) .. controls (-0.38,3.2) and (-0.12,3.55) .. (0,3.7) .. controls (0.12,3.55) and (0.38,3.2) .. (0.38,2.55) -- cycle", nose, d=0.1)
    s += solid(R(-0.4, 2.2, 0.4, 2.45), "gold", d=0.04)
    s += f"\\fill[white, opacity=0.5, rounded corners=0.04cm] {R(-0.28, 0.3, -0.17, 2.1)};\n"
    s += glint(-0.15, 3.0, 0.07)
    return s


def ammo_rockets():
    s = ""
    rng = random.Random("gear/ammo_rockets")
    # the open crate: inside and back first, then rockets, then the front
    s += f"\\filldraw[inked, fill=wooddark!55!ink] {L([(0.75, 2.95), (4.6, 2.95), (5.3, 3.5), (1.45, 3.5)])};\n"
    s += place(2.35, 1.3, 1.0, rocket(), rot=8)
    s += place(3.75, 1.4, 1.0, rocket("greenE", "orange"), rot=-6)
    front = L([(0.75, 0.6), (4.6, 0.6), (4.6, 2.95), (0.75, 2.95)])
    s += solid(front, "wood", d=0.16)
    s += line((0.75, 1.38), (4.6, 1.38), lw=1.1)
    s += line((0.75, 2.17), (4.6, 2.17), lw=1.1)
    s += clipped(front, "".join(curve([(x, y), (x + 0.6, y + rng.uniform(-0.05, 0.05)), (x + 1.2, y)], lw=0.9, colour="wooddark") for x, y in ((1.0, 1.0), (2.6, 1.75), (1.4, 2.55), (3.1, 0.95))))
    s += solid(L([(4.6, 0.6), (5.3, 1.15), (5.3, 3.5), (4.6, 2.95)]), "wood!80!ink", d=0.06)
    for x in (0.95, 4.4):
        for y in (0.8, 2.75):
            s += rivet(x, y, r=0.06)
    s += solid("(1.9,0.85) -- (3.4,0.85) -- (3.4,1.2) -- (1.9,1.2) -- cycle", "gold", d=0.03)
    s += clipped(R(1.9, 0.85, 3.4, 1.2), "".join(f"\\fill[ink] {L([(x, 0.85), (x + 0.12, 0.85), (x + 0.32, 1.2), (x + 0.2, 1.2)])};\n" for x in [1.8 + k * 0.28 for k in range(7)]))
    s += f"\\draw[inked] {R(1.9, 0.85, 3.4, 1.2)};\n"
    return s


def grenade():
    """A round grenade-launcher grenade standing up, its base at y = 0."""
    s = ""
    s += solid(R(-0.5, 0.0, 0.5, 1.15), BRASS, d=0.1)
    s += solid(R(-0.55, -0.05, 0.55, 0.15), "gold!80!ink", d=0.04)
    s += solid("(-0.5,1.1) -- (-0.5,1.5) .. controls (-0.5,2.3) and (0.5,2.3) .. (0.5,1.5) -- (0.5,1.1) -- cycle", "greenB", d=0.12)
    s += solid(R(-0.5, 1.3, 0.5, 1.45), "gold", d=0.03)
    s += glint(-0.22, 1.85, 0.1)
    s += f"\\fill[white, opacity=0.5] {R(-0.38, 0.2, -0.26, 1.0)};\n"
    return s


def ammo_grenades():
    s = ""
    band = "(-0.2,1.7) .. controls (2.0,1.35) and (4.0,1.35) .. (6.2,1.7) -- (6.2,2.55) .. controls (4.0,2.2) and (2.0,2.2) .. (-0.2,2.55) -- cycle"
    s += solid(band, "sagedark", d=0.14)
    s += stitch("(-0.2,1.85) .. controls (2.0,1.5) and (4.0,1.5) .. (6.2,1.85)", "sagelight")
    s += stitch("(-0.2,2.4) .. controls (2.0,2.05) and (4.0,2.05) .. (6.2,2.4)", "sagelight")
    for x, a in ((1.45, 10), (3.0, 0), (4.55, -10)):
        y = 1.55 + 0.0005 * (x - 3.0) ** 2 * 100
        s += place(x, y + 0.05, 1.08, grenade(), rot=a)
        # the loop of webbing round each grenade's case
        s += place(x, y + 0.05, 1.08, box(-0.62, 0.15, 0.62, 0.85, "sagedark!88!wood", r=0.1, d=0.08) + seam([(-0.5, 0.25), (0.5, 0.25)], "sagelight"), rot=a)
    s += rivet(0.55, 2.1)
    s += rivet(5.45, 2.1)
    s += tape(5.3, 1.95, 0.4, 0.95, fill="stone")
    return s


# ---------------------------------------------------------------- registry

DRAW = {"cap": cap, "helmet": helmet, "headset": headset,
        "sling": sling, "pads": pads, "straps": straps, "brace": brace,
        "tee": tee, "vest": vest, "mesh": mesh,
        "hoodie": hoodie, "poncho": poncho, "trench": trench, "hazmat": hazmat,
        "workgloves": workgloves, "quickgloves": quickgloves, "medgloves": medgloves,
        "jeans": jeans, "cargo": cargo, "joggers": joggers,
        "patches": patches, "kneepads": kneepads, "springs": springs,
        "sneakers": sneakers, "runners": runners, "sockshoes": sockshoes, "boots": boots,
        "belt": belt, "mines": mines, "decoys": decoys, "paint": paint, "smoke": smoke,
        "bat": bat, "sledge": sledge, "machete": machete,
        "ammo_light": ammo_light, "ammo_heavy": ammo_heavy, "ammo_shells": ammo_shells,
        "ammo_rockets": ammo_rockets, "ammo_grenades": ammo_grenades}

_names = [p for _, items in GROUP.values() for p in items]
assert len(_names) == 40 and len(set(_names)) == 40, "forty distinct items"
assert [len(items) for _, items in GROUP.values()] == [3, 4, 3, 4, 3, 3, 3, 4, 5, 3, 5], "items per group"
assert set(DRAW) == set(_names), "every item has a drawing and every drawing an item"
assert set(BASIC) <= set(_names) and len(BASIC) == 10, "one basic item per body group, and the bat"


def _uses(colour, tex):
    return re.search(rf"(?<![A-Za-z]){colour}(?![A-Za-z])", tex) is not None


def check_basic():
    for name in BASIC:
        tex = DRAW[name]()
        assert "% emblem" not in tex, f"{name}: a basic item carries no emblem"
        for c in ACCENTS:
            assert not _uses(c, tex), f"{name}: a basic item uses no accent colour ({c})"


def check_marks():
    tex = {n: DRAW[n]() for n in ("mesh", "medgloves", "quickgloves", "decoys", "headset", "paint", "mines")}
    assert "% emblem: cross" in tex["mesh"] and "% emblem: cross" in tex["medgloves"]
    assert "% emblem: zigzag" in tex["quickgloves"]
    assert "% emblem: sound" in tex["decoys"]
    assert _uses("cyan", tex["headset"]) and _uses("cyan", tex["paint"])
    assert _uses("red", tex["mines"])


check_basic()
check_marks()


def _make(name):
    return lambda: tile(name, DRAW[name]())


PICTURES = {f"gear/{n}": _make(n) for n in _names}
