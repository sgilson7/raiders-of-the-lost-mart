"""The seven drone portraits (web/art/drones/*.png), drawn as TikZ and built by
tools/art.py (`python3 tools/art.py drones`). Each is a 10 x 10 cm picture
on a transparent background, the machine filling about 80% of the frame in
a three-quarter view, after the frog portraits in NickHGame/tools/art.py and
the robot cat in vagrancy/Sam_Comm_final.png: flat warm fills, one shade band
and a glint or two, every shape outlined in plum ink, and mechanical clutter
(rivets, panel seams, hoses, vents).

The 3D game builds the same drones from primitives, so each portrait must
match it. Facts each picture must get right, and where this file enforces
each:

- the counts: wasp 4 rotors, moth 4 rotors (hidden under the wings, their
  rims showing), hornet 2 rotors, warden 4 ducted rotor pods, tick 6 legs,
  strider 6 legs, queen 6 legs. Every rotor is drawn by `rotor` or `duct` and every leg by
  `leg`, which tally what they draw in TALLY; `build` resets the tally,
  draws the picture and asserts the tally equals COUNTS[name], so the build
  stops if a picture gains or loses a rotor or a leg.
- the colours: each drone's body uses its own palette pair from
  tools/artlib.py COLOURS (wasp/waspdark, moth/mothdark, hornet/hornetdark,
  warden/wardendark, tick/tickdark, strider/striderdark); armour plates are
  always the dull greys armour/armourdark/armourlight, drawn by `plate`.
- every weak spot glows cyan: glow with a glowhi centre and a soft
  translucent halo, drawn by `weak` (and `halo`), the only place a weak spot
  is drawn; `build` asserts each picture drew exactly one weak spot
  (TALLY["weak"] == 1). The weak spots: the wasp's battery pack at the rear,
  the moth's core between the wings, the hornet's lens housing ring, the
  warden's ring round its core opening (the core itself glows orange,
  hot/hothi), the tick's underside between its legs, the strider's abdomen
  sac at the rear underside; the queen has two, the core glowing at the
  rim of her belly plate and the vent on her back (TALLY["weak"] == 2).
- the features named in the brief, each drawn once by its picture's
  function: the wasp's plum stripes, front armour plate, one big red camera
  eye, X frame of short arms and twin chin barrels; the moth's two flat
  rectangular solar wings with a grid of cells, pink beacon on top and
  dangling antennae; the hornet's long slim body, long boom to the rear
  rotor, sniper barrel with a big red lens at its tip and thin armour fins;
  the warden's riveted plates, thick arms, belly gaps showing the core, lamp
  array and cables; the tick's dome, red charge light on top; the strider's
  grey plates, pointed feet, twin-barrelled turret and three red eyes.
  `build` asserts the red-eye tally (TALLY["eyes"]) is 1 for the wasp and 3
  for the strider.
- transparent background: no picture draws a background rectangle; the only
  ground is a soft translucent shadow ellipse (`shadow`).
- no words, letters or digits, and repeatable: every mark is a path (no
  \\node), checked by artlib.check_no_text in tools/art.py; the only
  randomness (scuffs) comes from a seeded random.Random.
"""
import math, random
from collections import Counter

import artlib
from artlib import P, f, rect, rrect, circle, ellipse, ellipse_path, poly, blob, blob_path, line, curve, clipped, glint, rot

W, H = 10.0, 10.0
COUNTS = {
    "wasp": {"rotors": 4, "weak": 1, "eyes": 1},
    "moth": {"rotors": 4, "weak": 1},
    "hornet": {"rotors": 2, "weak": 1},
    "warden": {"rotors": 4, "weak": 1},
    "tick": {"legs": 6, "weak": 1},
    "strider": {"legs": 6, "weak": 1, "eyes": 3},
    "queen": {"legs": 6, "weak": 2, "eyes": 3},
}
TALLY = Counter()
# each picture is drawn about the frame's centre then scaled into frame by
# (zoom, dy); tube and leg widths are given in pt, so they scale by ZOOM too
FRAME = {"wasp": (0.94, -0.35), "moth": (0.95, 0.0), "hornet": (1.0, -0.35), "warden": (0.95, 0.35), "tick": (1.0, 0.0), "strider": (1.0, 0.0), "queen": (1.0, 0.0)}
ZOOM = [1.0]


def ink():
    return artlib.CUR_INK


# --- small parts ---------------------------------------------------------

def shadow(x, y, rx, ry=None):
    ry = ry if ry is not None else rx * 0.18
    return (f"\\fill[ink, opacity=0.10] {P(x, y)} ellipse ({f(rx * 1.12)} and {f(ry * 1.25)});\n"
            f"\\fill[ink, opacity=0.14] {P(x, y)} ellipse ({f(rx)} and {f(ry)});\n")


def shaded(path, base, dark, dx=0.14, dy=0.2, lw=None):
    """A flat fill with one shade band along the lower right edge: the shape
    in the dark shade, then the same shape nudged up and left in the base
    colour, clipped to the shape, then the ink outline."""
    s = f"\\fill[{dark}] {path};\n"
    s += clipped(path, f"\\begin{{scope}}[shift={{{P(-dx, dy)}}}]\\fill[{base}] {path};\\end{{scope}}\n")
    w = f",line width={f(lw)}pt" if lw else ""
    s += f"\\draw[inked{w}] {path};\n"
    return s


def poly_path(points):
    return " -- ".join(P(x, y) for x, y in points) + " -- cycle"


def rivet(x, y, r=0.065):
    return (f"\\fill[ink] {P(x, y)} circle ({f(r)});\n"
            f"\\fill[white, opacity=0.55] {P(x - r * 0.35, y + r * 0.35)} circle ({f(r * 0.4)});\n")


def seam(points, lw=0.7, opacity=0.8):
    return curve(points, lw=lw, extra=f", opacity={f(opacity)}", tension=0.5)


def plate(points, rivets=(), tension=None):
    """A dull grey armour plate with a shade band, a highlight edge and its
    rivets."""
    path = blob_path(points, tension) if tension else poly_path(points)
    s = shaded(path, "armour", "armourdark", 0.1, 0.13)
    a, b = points[0], points[1]
    s += f"\\draw[armourlight, line width={f(ink() * 1.2)}pt, line cap=round] {P(a[0] + (b[0] - a[0]) * 0.12, a[1] + (b[1] - a[1]) * 0.12 - 0.09)} -- {P(a[0] + (b[0] - a[0]) * 0.75, a[1] + (b[1] - a[1]) * 0.75 - 0.09)};\n"
    for x, y in rivets:
        s += rivet(x, y)
    return s


def halo(x, y, r, colour="glow", rx=None, ry=None):
    rx = rx or r
    ry = ry or r
    s = ""
    for k, op in ((2.0, 0.10), (1.6, 0.14), (1.28, 0.2)):
        s += f"\\fill[{colour}, opacity={f(op)}] {P(x, y)} ellipse ({f(rx * k)} and {f(ry * k)});\n"
    return s


def weak(x, y, rx, ry=None, rot_=0):
    """The cyan weak spot: a halo, a glowing disc, a pale centre, a glint."""
    TALLY["weak"] += 1
    ry = ry or rx
    s = halo(x, y, max(rx, ry), rx=rx, ry=ry)
    s += ellipse(x, y, rx, ry, "glow", rot=rot_)
    s += f"\\fill[glowhi] {ellipse_path(x - rx * 0.12, y + ry * 0.12, rx * 0.55, ry * 0.55, rot_)};\n"
    s += glint(x - rx * 0.4, y + ry * 0.4, min(rx, ry) * 0.18)
    return s


def red_eye(x, y, r, lens="red", count=True):
    """A red camera lens in a dark bezel, with a pupil ring and two glints."""
    if count:
        TALLY["eyes"] += 1
    s = circle(x, y, r * 1.22, "armourdark")
    s += f"\\fill[red, opacity=0.25] {P(x, y)} circle ({f(r * 1.6)});\n" if r > 0.4 else ""
    s += circle(x, y, r, lens)
    s += f"\\fill[redlight] {P(x - r * 0.15, y + r * 0.15)} circle ({f(r * 0.62)});\n"
    s += f"\\fill[red!60!ink] {P(x + r * 0.05, y - r * 0.05)} circle ({f(r * 0.36)});\n"
    s += f"\\draw[ink, line width={f(ink() * 0.7)}pt] {P(x, y)} circle ({f(r * 0.36)});\n"
    s += glint(x - r * 0.42, y + r * 0.4, r * 0.2)
    s += glint(x + r * 0.35, y - r * 0.38, r * 0.09)
    s += f"\\draw[inked] {P(x, y)} circle ({f(r)});\n"
    return s


def rotor(cx, cy, r, sq=0.3, spin=20, pod=True):
    """One open rotor seen from a little above: a motor pod, a translucent
    blur disc and two blades. Tallied."""
    TALLY["rotors"] += 1
    s = ""
    if pod:
        s += rrect(cx - 0.2, cy - 0.5, cx + 0.2, cy + 0.02, "armourdark", r=0.08)
        s += f"\\fill[white, opacity=0.25] {P(cx - 0.13, cy - 0.45)} rectangle {P(cx - 0.05, cy - 0.04)};\n"
    s += f"\\filldraw[ink, line width={f(ink() * 0.6)}pt, fill=white, fill opacity=0.32, draw opacity=0.7] {P(cx, cy)} ellipse ({f(r)} and {f(r * sq)});\n"
    s += f"\\fill[white, opacity=0.35] {P(cx, cy)} ellipse ({f(r * 0.72)} and {f(r * sq * 0.72)});\n"
    for a in (spin, spin + 180):
        tip = (cx + math.cos(math.radians(a)) * r * 0.96, cy + math.sin(math.radians(a)) * r * sq * 0.96)
        s += artlib.leaf((cx, cy), tip, 0.09, "armourdark", bend=0.03)
    s += ellipse(cx, cy + 0.03, 0.17, 0.1, "metal")
    return s


def duct(cx, cy, r, base, dark, sq=0.42, spin=30):
    """A ducted fan ring: a thick coloured ring with a fan spinning inside,
    seen from a little above. Tallied as a rotor."""
    TALLY["rotors"] += 1
    s = ""
    # the outer wall: a band below the rim
    wall = 0.38
    s += f"\\filldraw[inked, fill={dark}] {P(cx - r, cy)} -- {P(cx - r, cy - wall)} arc[start angle=180, end angle=360, x radius={f(r)}, y radius={f(r * sq)}] -- {P(cx + r, cy)} arc[start angle=0, end angle=-180, x radius={f(r)}, y radius={f(r * sq)}] -- cycle;\n"
    for k in (-0.55, 0.0, 0.55):
        x = cx + k * r
        y0 = cy - r * sq * math.sqrt(max(0.0, 1 - k * k))
        s += rivet(x, y0 - wall * 0.5, 0.055)
    # the rim
    s += ellipse(cx, cy, r, r * sq, base)
    inner = ellipse_path(cx, cy, r * 0.76, r * sq * 0.76)
    s += f"\\fill[ink!80!{dark}] {inner};\n"
    blades = ""
    for k in range(5):
        a = spin + k * 72
        tip = (cx + math.cos(math.radians(a)) * r * 0.72, cy + math.sin(math.radians(a)) * r * sq * 0.72)
        blades += artlib.leaf((cx, cy), tip, 0.1, "armour", bend=0.08, extra=",line width=0.6pt")
    blades += f"\\fill[white, opacity=0.18] {P(cx, cy)} ellipse ({f(r * 0.7)} and {f(r * sq * 0.7)});\n"
    s += clipped(inner, blades)
    s += circle(cx, cy, 0.14, "metal")
    s += f"\\draw[inked] {inner};\n"
    s += f"\\draw[white, opacity=0.45, line width={f(ink() * 1.3)}pt, line cap=round] ([shift={{(150:{f(r * 0.88)} and {f(r * sq * 0.88)})}}]{f(cx)},{f(cy)}) arc[start angle=150, end angle=95, x radius={f(r * 0.88)}, y radius={f(r * sq * 0.88)}];\n"
    return s


def pipe(points, w, fill, tension=0.55):
    return artlib.tube([points], w * ZOOM[0], fill, tension)


def leg(points, w, fill, joint="armourdark", foot=None):
    """A jointed leg through hip, knee(s) and foot: straight tube segments,
    a joint disc at every bend, and an optional pointed foot. Tallied."""
    TALLY["legs"] += 1
    s = ""
    wpt = w * artlib.PT_PER_CM * ZOOM[0]
    path = " -- ".join(P(x, y) for x, y in points)
    s += f"\\draw[ink, line width={f(wpt + 2 * ink())}pt, line cap=round, line join=round] {path};\n"
    s += f"\\draw[{fill}, line width={f(wpt)}pt, line cap=round, line join=round] {path};\n"
    s += f"\\draw[white, opacity=0.3, line width={f(wpt * 0.25)}pt, line cap=round] {P(points[0][0] - w * 0.15, points[0][1] + w * 0.15)} -- {P(points[1][0] - w * 0.15, points[1][1] + w * 0.15)};\n"
    for x, y in points[1:-1]:
        s += circle(x, y, w * 0.85, joint)
        s += f"\\fill[ink] {P(x, y)} circle ({f(w * 0.25)});\n"
    if foot:
        (x0, y0), (x1, y1) = points[-2], points[-1]
        L = math.hypot(x1 - x0, y1 - y0)
        ux, uy = (x1 - x0) / L, (y1 - y0) / L
        nx, ny = -uy, ux
        b = (x1 - ux * foot * 0.2, y1 - uy * foot * 0.2)
        tip = (x1 + ux * foot, y1 + uy * foot)
        s += poly([(b[0] + nx * w * 0.9, b[1] + ny * w * 0.9), tip, (b[0] - nx * w * 0.9, b[1] - ny * w * 0.9)], "armourdark")
    return s


def vent(x, y, w, h, n=3, fill="armourdark"):
    s = rrect(x, y, x + w, y + h, fill, r=0.06)
    for k in range(n):
        yy = y + h * (k + 1) / (n + 1)
        s += line((x + 0.08, yy), (x + w - 0.08, yy), lw=0.8)
    return s


def scuffs(rng, clip, box, n, colour):
    x0, y0, x1, y1 = box
    s = ""
    for _ in range(n):
        x, y = rng.uniform(x0, x1), rng.uniform(y0, y1)
        s += f"\\fill[{colour}, opacity=0.5] {P(x, y)} ellipse ({f(rng.uniform(0.05, 0.12))} and {f(rng.uniform(0.03, 0.07))});\n"
    return clipped(clip, s)


# --- the six drones ------------------------------------------------------

def wasp():
    """Small fast flier: a rounded gold body with plum stripes, a grey
    armour plate on the front, one big red camera eye, four small rotors on
    an X frame, twin chin barrels, a glowing battery pack at the rear."""
    rng = random.Random(11)
    s = shadow(5.1, 1.55, 3.0)
    # the X frame: far arms and rotors first
    far = [((4.0, 6.7), (3.15, 8.75)), ((6.6, 6.6), (7.55, 8.8))]
    near = [((3.55, 5.7), (1.35, 7.25)), ((7.0, 5.55), (8.85, 7.3))]
    for a, b in far:
        s += pipe([a, b], 0.26, "waspdark")
        s += rotor(b[0], b[1], 1.0, spin=25)
    # the battery pack at the rear (weak spot), behind the body's tail
    s += rrect(7.45, 3.85, 9.0, 6.0, "armourdark", r=0.25)
    s += weak(8.3, 4.92, 0.6, 0.85)
    for k in range(3):
        y = 4.42 + k * 0.4
        s += f"\\draw[glowhi, opacity=0.85, line width=1.2pt, line cap=round] {P(8.05, y)} -- {P(8.6, y)};\n"
    s += pipe([(7.7, 5.85), (8.25, 6.35), (8.8, 6.0)], 0.11, "armour")
    # the body
    body = [(2.3, 5.0), (2.9, 6.5), (4.6, 7.2), (6.5, 7.0), (7.85, 6.0), (8.05, 4.5), (7.15, 3.2), (5.0, 2.7), (3.1, 3.2)]
    bp = blob_path(body)
    s += f"\\fill[waspdark] {bp};\n"
    inner = f"\\begin{{scope}}[shift={{(-0.2,0.28)}}]\\fill[wasp] {bp};\\end{{scope}}\n"
    for x in (5.55, 6.6, 7.5):
        inner += f"\\fill[ink] {P(x - 0.2, 2.3)} .. controls {P(x + 0.05, 4.8)} .. {P(x - 0.25, 7.6)} -- {P(x + 0.12, 7.6)} .. controls {P(x + 0.42, 4.8)} .. {P(x + 0.17, 2.3)} -- cycle;\n"
    inner += scuffs(rng, bp, (4.9, 3.2, 7.6, 6.6), 9, "waspdark")
    inner += f"\\fill[white, opacity=0.4] {P(6.0, 6.55)} ellipse (0.95 and 0.2);\n"
    s += clipped(bp, inner)
    s += f"\\draw[inked] {bp};\n"
    # the front armour plate wrapping the nose
    s += plate([(2.42, 5.45), (3.15, 6.65), (4.85, 7.05), (5.15, 5.1), (4.75, 3.05), (3.2, 3.1), (2.35, 4.3)],
               rivets=[(3.3, 6.35), (4.55, 6.65), (4.75, 3.45), (3.4, 3.4)], tension=0.35)
    s += seam([(4.45, 6.95), (4.8, 5.0), (4.4, 3.15)])
    # near arms
    for a, b in near:
        s += pipe([a, b], 0.3, "waspdark")
        s += circle(a[0], a[1], 0.24, "armourdark")
        s += rivet(a[0], a[1])
    # the twin chin barrels
    for dy in (0.0, 0.32):
        s += rrect(1.75, 2.7 + dy, 3.6, 2.92 + dy, "metal", r=0.08)
        s += circle(1.8, 2.81 + dy, 0.11, "metaldark")
    s += rrect(3.2, 2.5, 4.3, 3.4, "armourdark", r=0.12)
    s += rivet(3.75, 2.95)
    # the one big red camera eye
    s += red_eye(3.45, 4.95, 0.74)
    for a, b in near:
        s += rotor(b[0], b[1], 1.12, spin=-15)
    return s, (W, H)


def moth():
    """The lure: a pale cream body, two wide flat solar-panel wings with a
    grid of cells, four rotors hidden under the wings, a pink beacon on top,
    dangling sensor antennae, and a cyan core between the wings."""
    s = shadow(5.0, 1.15, 3.4)
    # wings as quads: back-outer, back-inner, front-inner, front-outer
    L = [(0.25, 6.65), (3.9, 6.25), (3.95, 4.85), (0.45, 4.65)]
    R = [(9.75, 6.75), (6.1, 6.25), (6.05, 4.85), (9.55, 4.75)]
    # rotors under the wings: rims peeking out below the front edges
    for x, y, r in ((1.55, 4.3, 1.0), (3.05, 4.35, 0.85), (6.95, 4.35, 0.85), (8.45, 4.38, 1.0)):
        s += rotor(x, y, r, sq=0.34, spin=10, pod=False)
        s += pipe([(x, y + 0.05), (x, y + 0.35)], 0.16, "armourdark")
    # antennae dangling from the belly
    for (x0, y0), (x1, y1) in (((4.55, 3.2), (3.7, 1.55)), ((5.45, 3.2), (6.35, 1.4)), ((5.0, 3.0), (4.95, 1.85))):
        s += curve([(x0, y0), ((x0 + x1) / 2 + (0.25 if x1 < x0 else -0.25 if x1 > x0 else 0.12), (y0 + y1) / 2), (x1, y1)], lw=ink() * 1.4)
        s += circle(x1, y1, 0.16, "mothdark")
        s += f"\\fill[pink] {P(x1, y1)} circle (0.07);\n"
    # the body
    body = [(5.0, 2.9), (6.15, 3.4), (6.4, 4.8), (6.1, 6.2), (5.0, 6.75), (3.9, 6.2), (3.6, 4.8), (3.85, 3.4)]
    bp = blob_path(body)
    s += shaded(bp, "moth", "mothdark", 0.2, 0.22)
    for y in (3.75, 4.35):
        s += seam([(4.0, y + 0.1), (5.0, y - 0.12), (6.05, y + 0.1)])
    s += vent(4.55, 3.35, 0.9, 0.32, 2, "mothdark")
    # wings
    for q, side in ((L, -1), (R, 1)):
        # the wing's thickness, a strip under the front edge
        s += poly([q[3], q[2], (q[2][0], q[2][1] - 0.22), (q[3][0], q[3][1] - 0.22)], "mothdark")
        s += poly(q, "moth")
        # the grid of cells, bilinear over the quad
        def at(u, v):
            top = (q[0][0] + (q[1][0] - q[0][0]) * u, q[0][1] + (q[1][1] - q[0][1]) * u)
            bot = (q[3][0] + (q[2][0] - q[3][0]) * u, q[3][1] + (q[2][1] - q[3][1]) * u)
            return (top[0] + (bot[0] - top[0]) * v, top[1] + (bot[1] - top[1]) * v)
        nu, nv, m = 4, 2, 0.05
        for i in range(nu):
            for j in range(nv):
                u0, u1 = (i + m * 1.0) / nu * 0.9 + 0.03, (i + 1 - m) / nu * 0.9 + 0.03
                v0, v1 = (j + m * 1.6) / nv * 0.86 + 0.07, (j + 1 - m * 1.6) / nv * 0.86 + 0.07
                cell = [at(u0, v0), at(u1, v0), at(u1, v1), at(u0, v1)]
                s += poly(cell, "tealdark!85!ink", ",line width=0.8pt")
                a, b = at(u0 + 0.02, v0 + 0.14), at(u1 - 0.02, v0 + 0.14)
                c = at((u0 + u1) / 2, v0 + 0.05)
                d = at((u0 + u1) / 2, v1 - 0.05)
                s += f"\\draw[teal, line width=0.7pt] {P(*a)} -- {P(*b)};\n"
                s += f"\\draw[teal, line width=0.7pt] {P(*c)} -- {P(*d)};\n"
                g0, g1 = at(u0 + 0.03, v0 + 0.05), at(u0 + 0.12, v0 + 0.05)
                s += f"\\draw[white, opacity=0.5, line width=1.4pt, line cap=round] {P(*g0)} -- {P(*g1)};\n"
        for u in (0.02, 0.98):
            s += rivet(*at(u, 0.05))
            s += rivet(*at(u, 0.95))
        # the wing root clamp
        rx = q[1][0]
        s += rrect(rx - 0.35 if side < 0 else rx - 0.15, 5.0, rx + 0.15 if side < 0 else rx + 0.35, 6.15, "armour", r=0.1)
    # the core between the wings
    s += circle(5.0, 5.35, 0.78, "armourdark")
    s += weak(5.0, 5.35, 0.6)
    for a in range(0, 360, 60):
        x, y = 5.0 + math.cos(math.radians(a)) * 0.68, 5.35 + math.sin(math.radians(a)) * 0.68
        s += rivet(x, y, 0.05)
    # the beacon on top
    s += rrect(4.75, 6.5, 5.25, 7.15, "armour", r=0.08)
    s += halo(5.0, 7.55, 0.45, "pink")
    for a in (20, 60, 120, 160):
        x0, y0 = 5.0 + math.cos(math.radians(a)) * 0.7, 7.55 + math.sin(math.radians(a)) * 0.7
        x1, y1 = 5.0 + math.cos(math.radians(a)) * 1.05, 7.55 + math.sin(math.radians(a)) * 1.05
        s += line((x0, y0), (x1, y1), lw=ink() * 1.3, colour="pink")
    s += f"\\filldraw[inked, fill=pink] (4.55,7.15) -- (4.55,7.45) arc[start angle=180, end angle=0, radius=0.45] -- (5.45,7.15) -- cycle;\n"
    s += f"\\fill[white, opacity=0.6] (4.82,7.6) ellipse (0.12 and 0.2);\n"
    s += rrect(4.4, 6.95, 5.6, 7.2, "armourdark", r=0.06)
    return s, (W, H)


def hornet():
    """The sniper: a long slim dark-teal body, one rotor at the front and
    one at the back on a long boom, a long barrel ending in a big red lens
    ringed by the cyan lens housing, thin armour fins."""
    rng = random.Random(23)
    s = shadow(5.2, 2.2, 3.1)
    # the rear rotor on its mast at the end of the boom; the boom
    s += pipe([(6.0, 5.55), (8.5, 6.6)], 0.32, "hornetdark")
    s += poly([(8.0, 6.55), (8.95, 6.75), (9.25, 5.45), (8.75, 5.5)], "armour")  # tail fin
    s += line((8.55, 6.55), (9.0, 5.65), lw=0.7, extra=",opacity=0.7")
    s += pipe([(8.5, 6.6), (8.5, 7.35)], 0.2, "armourdark")
    s += rotor(8.5, 7.5, 1.35, sq=0.3, spin=15)
    # the barrel, under the body, out to the lens
    bar = [(4.2, 4.4), (1.75, 3.45)]
    s += pipe(bar, 0.26, "metal")
    for t in (0.3, 0.55):
        x, y = 4.2 + (1.75 - 4.2) * t, 4.4 + (3.45 - 4.4) * t
        s += ellipse(x, y, 0.1, 0.22, "armour", rot=-20)
    # the body: a long slim capsule
    body = artlib.limb_pts([(2.55, 4.7), (4.0, 5.15), (5.5, 5.5), (6.6, 5.8)], 0.62, 0.42, cap=True)
    bp = blob_path(body + [(2.1, 4.55)], 0.5)
    s += f"\\fill[hornetdark] {bp};\n"
    inner = f"\\begin{{scope}}[shift={{(-0.1,0.2)}}]\\fill[hornet] {bp};\\end{{scope}}\n"
    inner += scuffs(rng, bp, (2.5, 4.4, 6.5, 6.2), 10, "hornetdark")
    inner += f"\\fill[white, opacity=0.3] {P(4.3, 5.6)} ellipse (1.1 and 0.12);\n"
    s += clipped(bp, inner)
    s += f"\\draw[inked] {bp};\n"
    for x in (3.7, 5.3):
        y = 4.7 + (x - 2.55) * 0.27
        s += seam([(x - 0.05, y + 0.85), (x + 0.1, y + 0.3), (x + 0.05, y - 0.25)])
    s += vent(4.3, 5.0, 0.7, 0.3, 2, "hornetdark")
    for x, y in ((2.75, 5.15), (6.25, 6.15), (6.3, 5.4)):
        s += rivet(x, y)
    # thin armour fins: a dorsal fin and a ventral fin
    s += poly([(4.6, 5.85), (5.7, 6.2), (6.05, 7.0), (5.55, 6.95)], "armour")
    s += poly([(5.3, 5.0), (6.2, 5.3), (6.6, 4.25), (6.2, 4.3)], "armour")
    s += line((5.15, 6.0), (5.8, 6.85), lw=0.7, extra=",opacity=0.7")
    # the front rotor on a mast over the nose
    s += pipe([(3.4, 5.4), (3.3, 6.45)], 0.22, "armourdark")
    s += rotor(3.3, 6.6, 1.35, sq=0.3, spin=-20)
    # the lens at the tip: cyan housing ring (the weak spot), red lens
    lx, ly = 1.55, 3.35
    s += ellipse(3.75, 4.2, 0.22, 0.34, "armourdark", rot=-20)
    s += rivet(3.75, 4.2)
    s += weak(lx, ly, 0.7)
    s += circle(lx, ly, 0.6, "glow")
    s += f"\\draw[glowhi, line width=1.6pt] {P(lx, ly)} circle (0.53);\n"
    s += red_eye(lx, ly, 0.42, count=False)
    return s, (W, H)


def warden():
    """Heavy hover drone: a coral riveted plate shell, four ducted rotor
    pods on thick arms, a glowing orange core seen through the belly armour
    with a cyan ring round its opening, a lamp array, cables."""
    rng = random.Random(41)
    s = shadow(5.0, 1.35, 3.8)
    # far arms and pods
    for (a, b) in (((3.6, 6.4), (2.35, 8.1)), ((6.4, 6.4), (7.65, 8.1))):
        s += pipe([a, b], 0.55, "wardendark")
        s += duct(b[0], b[1], 1.12, "warden", "wardendark")
    # the shell: dome over a skirt
    shell = [(1.9, 4.6), (2.25, 6.3), (3.4, 7.3), (5.0, 7.6), (6.6, 7.3), (7.75, 6.3), (8.1, 4.6), (7.2, 3.3), (5.0, 2.85), (2.8, 3.3)]
    sp = blob_path(shell, 0.45)
    s += shaded(sp, "warden", "wardendark", 0.18, 0.25)
    s += clipped(sp, scuffs(rng, sp, (2.2, 3.4, 7.8, 7.2), 14, "wardendark"))
    # plate seams and rivets on the dome
    s += seam([(2.0, 5.25), (3.5, 5.6), (5.0, 5.7), (6.5, 5.6), (8.0, 5.25)])
    s += seam([(3.6, 7.25), (3.75, 6.4), (3.55, 5.6)])
    s += seam([(6.4, 7.25), (6.25, 6.4), (6.45, 5.6)])
    for x, y in ((2.6, 5.15), (3.9, 5.45), (6.1, 5.45), (7.4, 5.15), (3.95, 7.05), (6.05, 7.05), (2.75, 6.35), (7.25, 6.35)):
        s += rivet(x, y, 0.075)
    s += f"\\fill[white, opacity=0.35] (4.3,7.15) ellipse (0.9 and 0.15);\n"
    # the visor and its lamp array
    s += rrect(3.6, 5.95, 6.4, 6.85, "ink!85!wardendark", r=0.35)
    for x in (4.15, 4.72, 5.28, 5.85):
        s += f"\\fill[goldlight, opacity=0.22] {P(x, 6.4)} circle (0.36);\n"
        s += circle(x, 6.4, 0.2, "goldlight")
        s += glint(x - 0.06, 6.46, 0.06)
    s += poly([(3.3, 7.05), (5.0, 6.72), (6.7, 7.05), (6.55, 6.8), (5.0, 6.52), (3.45, 6.8)], "armour")
    # the belly: grey armour slats with gaps showing the orange core
    belly = [(2.45, 4.65), (7.55, 4.65), (6.9, 3.4), (5.0, 2.95), (3.1, 3.4)]
    bp = blob_path(belly, 0.3)
    glowing = f"\\fill[hot] {bp};\n"
    glowing += f"\\fill[hothi, opacity=0.7] (5.0,3.85) ellipse (2.0 and 0.6);\n"
    s += glowing + f"\\draw[inked] {bp};\n"
    slats = ""
    for (x0, x1) in ((2.2, 3.35), (6.65, 7.8)):
        slats += poly([(x0, 2.8), (x1, 2.8), (x1, 4.75), (x0, 4.75)], "armour")
    for y0 in (3.95, ):
        slats += poly([(2.2, y0), (7.8, y0), (7.8, y0 + 0.45), (2.2, y0 + 0.45)], "armour")
    s += clipped(bp, slats)
    s += f"\\draw[inked] {bp};\n"
    for x, y in ((2.85, 4.18), (7.15, 4.18), (2.95, 3.55), (7.05, 3.55)):
        s += rivet(x, y)
    # the core opening: orange core, cyan weak-spot ring round it
    cx, cy = 5.0, 3.45
    s += weak(cx, cy, 1.0, 0.62)
    s += ellipse(cx, cy, 0.78, 0.46, "hot")
    s += f"\\fill[hothi] {P(cx - 0.05, cy + 0.03)} ellipse (0.45 and 0.25);\n"
    s += f"\\fill[white, opacity=0.7] {P(cx - 0.15, cy + 0.08)} ellipse (0.16 and 0.08);\n"
    s += f"\\draw[inked] {P(cx, cy)} ellipse (0.78 and 0.46);\n"
    # cables looping from the shell to the near arms
    s += pipe([(2.5, 3.6), (1.8, 3.3), (1.55, 3.85)], 0.14, "armourdark")
    s += pipe([(7.5, 3.6), (8.2, 3.3), (8.45, 3.85)], 0.14, "armourdark")
    # near arms and pods
    for (a, b) in (((2.6, 4.9), (1.45, 4.55)), ((7.4, 4.9), (8.55, 4.55))):
        s += pipe([a, b], 0.62, "wardendark")
        s += circle(a[0], a[1], 0.42, "armour")
        s += rivet(a[0], a[1], 0.08)
    for b in ((1.45, 4.55), (8.55, 4.55)):
        s += duct(b[0], b[1], 1.3, "warden", "wardendark", spin=10)
    # hose over the top
    s += pipe([(4.2, 7.55), (5.0, 8.05), (5.8, 7.55)], 0.16, "armourdark")
    return s, (W, H)


def tick():
    """Small round walking kamikaze: a sage dome on six thin jointed legs,
    a blinking red charge light on top, a cyan weak spot glowing underneath
    between the legs."""
    rng = random.Random(5)
    s = shadow(5.0, 1.85, 4.3, 0.75)
    # the glow underneath, on the ground and on the belly, first
    s += f"\\fill[glow, opacity=0.16] (5.0,1.9) ellipse (2.3 and 0.5);\n"
    # six legs spaced round the dome on the ground, splayed so no near knee
    # hides a far one; legs whose hips are on the far half go behind the dome
    far, near = [], []
    for a in (10, 65, 115, 170, 235, 305):
        c, sn = math.cos(math.radians(a)), math.sin(math.radians(a))
        p = [(5 + 2.45 * c, 3.85 + 0.6 * sn), (5 + 3.55 * c, 6.25 + 0.75 * sn), (5 + 4.35 * c, 1.95 + 1.05 * sn)]
        (far if sn > 0.01 else near).append(p)
    for p in far:
        s += leg(p, 0.16, "tickdark", foot=0.25)
    # the underside: weak spot between the legs
    s += ellipse(5.0, 3.7, 2.6, 0.75, "armourdark")
    s += weak(5.0, 3.35, 0.9, 0.42)
    # the dome
    dome = f"(2.1,3.95) .. controls (2.1,6.6) and (7.9,6.6) .. (7.9,3.95) -- cycle"
    s += shaded(dome, "tick", "tickdark", 0.2, 0.25)
    s += clipped(dome, scuffs(rng, dome, (2.4, 4.0, 7.6, 5.8), 10, "tickdark")
                 + f"\\fill[white, opacity=0.35] (3.9,5.55) ellipse (0.8 and 0.22);\n")
    s += seam([(3.0, 4.0), (3.4, 5.4), (4.4, 5.9)])
    s += seam([(7.0, 4.0), (6.6, 5.4), (5.6, 5.9)])
    # the skirt band
    s += rrect(1.85, 3.55, 8.15, 4.15, "armour", r=0.25)
    for x in (2.4, 3.6, 5.0, 6.4, 7.6):
        s += rivet(x, 3.85)
    s += vent(4.3, 4.45, 1.4, 0.45, 3, "tickdark")
    # the charge light on top
    s += rrect(4.4, 5.85, 5.6, 6.15, "armourdark", r=0.06)
    s += halo(5.0, 6.5, 0.5, "red")
    s += f"\\filldraw[inked, fill=red] (4.55,6.1) -- (4.55,6.45) arc[start angle=180, end angle=0, radius=0.45] -- (5.45,6.1) -- cycle;\n"
    s += f"\\fill[redlight] (4.82,6.58) ellipse (0.15 and 0.2);\n"
    s += glint(4.8, 6.66, 0.08)
    for a in (30, 90, 150):
        x0, y0 = 5.0 + math.cos(math.radians(a)) * 0.8, 6.45 + math.sin(math.radians(a)) * 0.8
        x1, y1 = 5.0 + math.cos(math.radians(a)) * 1.2, 6.45 + math.sin(math.radians(a)) * 1.2
        s += line((x0, y0), (x1, y1), lw=ink() * 1.3, colour="red")
    # near legs
    for p in near:
        s += circle(p[0][0], p[0][1], 0.24, "armourdark")
        s += leg(p, 0.18, "tick", foot=0.28)
    return s, (W, H)


def strider():
    """Big walking spider: a rust armoured body with grey plates on six long
    jointed legs with pointed feet, a twin-barrelled turret on top, three
    red eyes at the front, a cyan abdomen sac at the rear underside."""
    rng = random.Random(77)
    s = shadow(5.0, 1.15, 4.6, 0.55)
    far = [[(3.4, 5.6), (2.6, 8.4), (1.6, 2.1)], [(5.0, 5.75), (5.6, 8.8), (6.1, 2.35)], [(6.3, 5.6), (8.3, 8.35), (9.15, 2.15)]]
    near = [[(3.5, 4.35), (1.0, 5.0), (0.4, 0.9)], [(5.3, 4.05), (4.85, 5.0), (3.85, 0.6)], [(7.0, 4.15), (9.25, 6.2), (9.65, 0.85)]]
    for p in far:
        s += leg(p, 0.2, "striderdark", foot=0.42)
    # the abdomen sac (weak spot), under the rear
    s += weak(7.6, 3.85, 0.9, 0.72)
    s += f"\\draw[glowhi, line width=1pt, line cap=round] (7.1,3.4) .. controls (7.35,3.95) .. (7.95,4.2);\n"
    # the abdomen
    ab = [(5.4, 4.6), (6.4, 6.0), (8.0, 6.3), (9.0, 5.4), (8.75, 4.3), (7.6, 4.0), (6.3, 4.0)]
    abp = blob_path(ab, 0.55)
    s += shaded(abp, "strider", "striderdark", 0.15, 0.22)
    s += clipped(abp, scuffs(rng, abp, (5.8, 4.2, 8.8, 6.1), 9, "striderdark")
                 + "\\fill[white, opacity=0.3] (7.4,6.05) ellipse (0.7 and 0.12);\n")
    s += plate([(6.5, 5.85), (8.1, 6.1), (8.65, 5.3), (7.25, 5.05)], rivets=[(6.95, 5.65), (8.1, 5.7)])
    s += plate([(7.0, 4.7), (8.5, 4.95), (8.55, 4.45), (7.3, 4.25)], rivets=[(7.55, 4.55)])
    s += seam([(6.0, 4.25), (6.35, 5.0), (6.9, 4.4)])
    # the thorax
    th = [(1.8, 5.45), (2.4, 6.45), (4.2, 6.65), (5.75, 6.0), (5.85, 4.7), (4.3, 4.0), (2.45, 4.45)]
    thp = blob_path(th, 0.5)
    s += shaded(thp, "strider", "striderdark", 0.16, 0.22)
    s += clipped(thp, scuffs(rng, thp, (2.2, 4.2, 5.6, 6.4), 8, "striderdark")
                 + "\\fill[white, opacity=0.3] (4.6,6.4) ellipse (0.6 and 0.12);\n")
    s += plate([(3.6, 4.35), (5.55, 4.6), (5.6, 5.4), (3.75, 5.2)], rivets=[(4.0, 4.75), (5.25, 4.95)])
    s += seam([(3.25, 6.5), (3.05, 5.4), (3.35, 4.3)])
    s += vent(4.15, 5.55, 0.6, 0.35, 2, "striderdark")
    # the turret
    s += ellipse(3.85, 6.55, 0.95, 0.3, "armourdark")
    for dy in (0.0, 0.3):
        s += rrect(1.1, 6.85 + dy, 3.5, 7.06 + dy, "metal", r=0.08)
        s += rrect(0.95, 6.8 + dy, 1.3, 7.11 + dy, "metaldark", r=0.06)
    s += f"\\filldraw[inked, fill=armour] (3.0,6.55) -- (3.1,7.15) .. controls (3.35,7.75) and (4.35,7.75) .. (4.7,7.15) -- (4.75,6.55) -- cycle;\n"
    s += f"\\fill[armourlight] (3.3,7.15) .. controls (3.5,7.5) and (3.95,7.55) .. (4.15,7.45) -- (4.05,7.35) .. controls (3.75,7.4) and (3.5,7.35) .. (3.4,7.1) -- cycle;\n"
    s += seam([(3.05, 6.85), (4.75, 6.85)])
    for x in (3.35, 4.4):
        s += rivet(x, 6.7)
    # cable from the turret down into the abdomen
    s += pipe([(4.7, 6.9), (5.5, 6.7), (6.3, 6.0)], 0.12, "armourdark")
    # three red eyes at the front
    s += red_eye(2.2, 5.65, 0.3)
    s += red_eye(2.85, 5.85, 0.24)
    s += red_eye(2.6, 5.15, 0.22)
    # near legs
    for p in near:
        s += circle(p[0][0], p[0][1], 0.3, "armourdark")
        s += rivet(p[0][0], p[0][1], 0.08)
        s += leg(p, 0.23, "strider", foot=0.48)
    return s, (W, H)


def queen():
    """The street's queen, side 3/4 facing left: a long plum hull with a
    ridge of grey plate, six legs whose knees rise above the hull, two
    rocket pods of tubes on her shoulders, the eye cannon under a dark brow
    with three red eyes, a grey belly plate with the cyan core glowing at
    its rim, and the cyan heat vent on her back (two weak spots)."""
    rng = random.Random(4242)
    s = shadow(5.0, 0.75, 4.7, 0.5)
    hips = [(3.7, 4.3), (5.0, 4.15), (6.3, 4.3)]
    reach = [-1.0, 0.05, 1.0]
    for k, (hx, hy) in enumerate(hips):
        r_ = reach[k] + 0.3
        s += leg([(hx + 0.25, hy + 0.35), (hx + r_ * 1.25 + 0.2, 7.9), (hx + r_ * 2.35 + 0.2, 0.95)], 0.15, "queendark", foot=0.3)
    # the vent on her back (weak), behind the hull's ridge
    s += weak(7.0, 6.15, 0.55, 0.2)
    # the hull
    hull = [(2.3, 4.6), (2.75, 5.7), (4.4, 6.25), (6.5, 6.2), (7.9, 5.6), (8.05, 4.6), (6.9, 3.85), (4.6, 3.75), (2.95, 3.95)]
    hp = blob_path(hull, 0.55)
    s += shaded(hp, "queen", "queendark", 0.15, 0.22)
    s += clipped(hp, scuffs(rng, hp, (2.8, 4.0, 7.8, 6.0), 10, "queendark")
                 + "\\fill[white, opacity=0.28] (5.1,6.05) ellipse (1.1 and 0.12);\n")
    s += plate([(3.4, 5.95), (5.0, 6.35), (6.6, 6.3), (7.3, 5.9), (5.4, 5.75)], rivets=[(4.3, 6.05), (5.6, 6.1), (6.7, 6.0)])
    s += seam([(2.9, 4.75), (5.2, 4.5), (7.85, 4.85)])
    for x in (3.6, 4.6, 5.6, 6.6):
        s += rivet(x, 4.25)
    # the belly plate, and the core glowing at its rim (weak)
    s += weak(5.1, 3.62, 0.85, 0.2)
    s += plate([(3.9, 3.85), (6.3, 3.85), (6.0, 3.6), (4.2, 3.6)], rivets=[(4.6, 3.73), (5.6, 3.73)])
    # the rocket pods on her shoulders
    for px in (3.9, 5.2):
        s += rrect(px - 0.55, 6.1, px + 0.55, 6.95, "armour", r=0.08)
        for (tx, ty) in [(-0.3, 0.25), (0.0, 0.25), (0.3, 0.25), (-0.3, 0.6), (0.0, 0.6), (0.3, 0.6)]:
            s += circle(px + tx, 6.1 + ty, 0.1, "armourdark")
            s += f"\\fill[ink] {P(px + tx, 6.1 + ty)} circle (0.045);\n"
    # the brow, the eye cannon and three red eyes
    s += rrect(1.75, 4.25, 2.85, 5.35, "queendark", r=0.18)
    s += rrect(0.35, 4.45, 1.9, 4.8, "metaldark", r=0.06)
    s += rrect(0.15, 4.38, 0.45, 4.87, "armourdark", r=0.05)
    s += red_eye(2.1, 5.0, 0.16)
    s += red_eye(2.5, 5.08, 0.14)
    s += red_eye(2.35, 4.62, 0.13)
    for k, (hx, hy) in enumerate(hips):
        r_ = reach[k]
        s += circle(hx, hy, 0.3, "queendark")
        s += rivet(hx, hy, 0.08)
        s += leg([(hx, hy), (hx + r_ * 1.45, 8.35), (hx + r_ * 2.75, 0.7)], 0.18, "queen", joint="armour", foot=0.36)
    return s, (W, H)


def build(name, make):
    def run():
        TALLY.clear()
        z, dy = FRAME[name]
        ZOOM[0] = z
        body, size = make()
        body = f"\\begin{{scope}}[shift={{{P(5 * (1 - z), 5 * (1 - z) + dy)}}}, scale={f(z)}]\n{body}\\end{{scope}}\n"
        want = COUNTS[name]
        got = {k: TALLY[k] for k in set(want) | {"rotors", "legs", "weak", "eyes"} if TALLY[k] or k in want}
        if got != want:
            raise SystemExit(f"drones/{name}: drew {got}, the spec says {want}")
        return body, size
    return run


PICTURES = {f"drones/{n}": build(n, fn) for n, fn in
            (("wasp", wasp), ("moth", moth), ("hornet", hornet), ("warden", warden), ("tick", tick), ("strider", strider), ("queen", queen))}
