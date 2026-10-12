"""Raiders of the Lost Mart: the title art and the six mission postcards,
drawn as TikZ and compiled by tools/art.py (web/art/title.png,
web/art/missions/*.png). Never generated pixels.

Each picture is composed from sub-pictures defined once below and placed by
the picture's own function: the six drones (`wasp`, `moth`, `hornet`,
`warden`, `tick`, `strider`, via `drone`), a raider walking away or facing
out (`raider`) and the two homemade guns (`quad_shotgun`, `ak_rocket`), the
van with its doors open (`van`) and its load (`gear_bag`, `parts_crate`,
`clipboard`), lot props (`car_back`, `car_side`, `cart`, `light_pole`,
`cone`, `crack`, `weeds`, `blades`, `shrub`, `tree`), the mall seen from
the lot (`mall`, `glass_front`, `vine`), and interior pieces (`tee`,
`chain`, `poster`, `table`, `chair`, `stall`, `hard_drive`, `mannequin`,
`shelf_unit`, `pod`, `storefront`, `generator`, `work_light`, `cable`).
The look follows vagrancy/Sam_Comm_final.png and the title of Ribbit King
(NickHGame/tools/art.py, whose structure this follows): a warm yellow sky
with outlined puffy clouds, salmon, coral and tan walls, sage and teal
machines, flat fills, one plum ink with round joins, and lots of clutter.

    python3 tools/art.py title       # 25.6 x 14.4 cm at 150 dpi
    python3 tools/art.py missions    # 19.2 x 10.8 cm at 120 dpi each

Facts each picture must get right (the things an image generator gets
wrong), and where this file enforces each:

- no words, letters or digits anywhere: store signs, posters, band tees,
  menu boards, the mission order on the clipboard and the cinema's blade
  sign are coloured shapes and plain bars. Nothing here emits a \\node or
  text; tools/art.py's `check_no_text` refuses a .tex with \\node, \\text
  or a label before it compiles. The clock in `drives` has hands and no
  numerals. Shapes that could pass for a letter (an L-shaped boot was
  one) are not used.
- the calm areas stay free of clutter, because the page lays its words
  there: the title's lower-left corner (x < CALM_X = 9.0, y < CALM_Y = 5.0,
  at least a third each way, asserted in `title`) shows only plain sky and
  asphalt: everything drawn after the plain base passes through
  `calm_clip`, which clips it out of that corner. Each postcard's left third
  (x < MCALM = W/3) shows only plain wall and floor: every postcard draws
  its detail into one string clipped by `right_of(MCALM, ...)`. The only
  marks allowed across a calm floor are faint tile seams (`floor_tiles`,
  unoutlined light lines at low opacity).
- every flying drone has exactly four rotors (the warden's are four rotor
  pods on four arms): every rotor is drawn by `rotor`, which counts, and
  `wasp`, `moth`, `hornet` and `warden` each assert the count is ROTORS = 4.
- the walkers have exactly six legs: every leg is drawn by `spider_leg`,
  which counts, and `tick`, `strider` and `queen` assert LEGS (6 each; the
  brief fixes the strider's six and the tick and the queen follow it).
- the queen is as tall as Main Street's buildings: in `queen_card` her
  knees rise above the cornices behind her.
- drone colours and features match the 3D game: the wasp is yellow
  (`wasp`/`waspdark`) with a red eye; the moth is pale cream (`moth`) with
  wide flat solar-panel wings and a pink blinking light; the hornet is long
  and dark teal (`hornet`) with a long red lens; the warden is coral
  (`warden`) with a glowing orange core (`hot`/`hothi`); the tick is small,
  round and sage green (`tick`); the strider is big, coral/rust
  (`strider`) with a turret. Weak spots glow cyan (`glow`, #4FE6E0) on
  every drone. Each drone is defined once, in its own function, and every
  picture draws it through `drone`, so no scene can recolour one.
- the guns are the ones the game builds: `quad_shotgun` is a pistol frame
  with four barrels in a 2x2 square (its muzzle plate shows four bores in a
  square); `ak_rocket` is an AK with a rocket tube strapped on top. In the
  title the green raider carries the first and the teal raider the second.
- the title's raiders walk away toward the mall, so they show backpacks
  and the backs of helmets; only the one stepping down from the van faces
  out. They shrink with distance on the same ground plane as the lot
  (`gy`/`gs` in `title`: one camera height and horizon), so nobody floats.
- plum ink (#3B2340) outlines every filled shape (artlib's `inked` style);
  unoutlined translucent fills are light only (beams, glows, halos,
  shadows, glass glints).
- repeatable: every random choice comes from a seeded random.Random, one
  per picture.
"""
import math, random

from artlib import (P, f, pts, rect, rrect, circle, ellipse, ellipse_path, poly, blob, blob_path, hill,
                    line, curve, leaf, limb, place, clipped, glint, rot, sky, cloud, tuft)
import artlib

# --------------------------------------------------------------------------
# small helpers
# --------------------------------------------------------------------------

WALLS = ["wallA", "wallB", "wallC", "wallD", "wallE", "wallF"]
SIGNS = ["red", "teal", "gold", "magenta", "violet", "orange", "pink"]


def lw(w):
    return f",line width={f(w)}pt"


def fillo(path, colour, op):
    """Light, not a shape: an unoutlined translucent fill."""
    return f"\\fill[{colour}, opacity={f(op)}] {path};\n"


def polyp(points):
    return " -- ".join(P(x, y) for x, y in points) + " -- cycle"


def halo(x, y, r, colour="glow", n=4, op=0.13):
    """A glow: translucent rings, unoutlined (light, not a shape)."""
    out = ""
    for i in range(n, 0, -1):
        out += f"\\fill[{colour}, opacity={f(op)}] {P(x, y)} circle ({f(r * (1 + 0.55 * i))});\n"
    return out


def lerp(a, b, t):
    return (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)


def quad_pt(q, u, v):
    """Bilinear point in a quad given as (bottom-left, bottom-right, top-right, top-left)."""
    a = lerp(q[0], q[1], u)
    b = lerp(q[3], q[2], u)
    return lerp(a, b, v)


def calm_clip(W, H, cx, cy, body):
    """Clip body to everything outside the calm rectangle (0,0)-(cx,cy), so
    nothing in it can stray into the page's text area."""
    path = polyp([(cx, -1), (W + 1, -1), (W + 1, H + 1), (-1, H + 1), (-1, cy), (cx, cy)])
    return clipped(path, body)


# --------------------------------------------------------------------------
# the drones (each drawn about its own origin; place() puts it in a scene)
# Flying drones have exactly ROTORS rotors; every rotor goes through rotor().
# --------------------------------------------------------------------------

ROTORS = 4
LEGS = {"tick": 6, "strider": 6, "queen": 6}


class Count:
    """Counts the rotors or legs a sub-picture draws, for the asserts."""
    n = 0


def rotor(x, y, r=0.38, hub="metaldark", w=0.8):
    Count.n += 1
    s = rect(x - 0.06, y - 0.04, x + 0.06, y + 0.12, hub, lw(w))
    s += f"\\filldraw[draw=ink, line width={f(w)}pt, fill=metallight, fill opacity=0.55] {P(x, y + 0.14)} ellipse ({f(r)} and {f(r * 0.2)});\n"
    s += line((x - r * 0.85, y + 0.1), (x + r * 0.85, y + 0.18), lw=w + 0.3)
    s += circle(x, y + 0.14, 0.045, hub, lw(w * 0.8))
    return s


def wasp():
    """Yellow wasp drone, 3/4 front view: a striped yellow body, a red eye,
    exactly four small rotors on four arms, a cyan weak-spot dome on top."""
    Count.n = 0
    s = ""
    back = [(-0.72, 0.36), (0.72, 0.36)]
    front = [(-0.98, 0.0), (0.98, 0.0)]
    for x, y in back:
        s += limb([(0, 0.12), (x, y)], 0.07, 0.05, "waspdark")
        s += rotor(x, y, 0.32)
    s += poly([(-0.13, -0.36), (0.13, -0.36), (0, -0.66)], "waspdark", lw(0.9))
    body = ellipse_path(0, 0, 0.62, 0.44)
    s += f"\\fill[wasp] {body};\n"
    stripes = f"\\fill[waspdark] (-0.7,-0.16) rectangle (0.7,-0.04);\n\\fill[waspdark] (-0.7,-0.36) rectangle (0.7,-0.26);\n"
    s += clipped(body, stripes)
    s += f"\\draw[inked] {body};\n"
    s += halo(0, 0.42, 0.1, "glow", 2, 0.2)
    s += f"\\filldraw[inked,fill=glow,line width=0.8pt] (-0.17,0.38) arc[start angle=180,end angle=0,radius=0.17] -- cycle;\n"
    for x, y in front:
        s += limb([(0.45 * (1 if x > 0 else -1), 0.0), (x, y)], 0.08, 0.06, "waspdark")
        s += rotor(x, y, 0.36)
    s += circle(0, 0.05, 0.22, "armourdark", lw(0.9))
    s += circle(0, 0.05, 0.14, "red", lw(0.8))
    s += glint(-0.04, 0.1, 0.045)
    assert Count.n == ROTORS, "wasp must have exactly four rotors"
    return s


def moth():
    """Pale cream moth lure drone: wide flat solar-panel wings, a pink
    blinking light, four rotors (two on each wing), feathery antennae."""
    Count.n = 0
    s = ""
    for side in (-1, 1):
        q = [(0.3 * side, -0.16), (2.05 * side, -0.04), (1.95 * side, 0.46), (0.3 * side, 0.26)]
        s += poly(q, "moth", lw(1.0))
        for i in range(4):
            for j in range(2):
                u0, u1 = 0.06 + i * 0.235, 0.06 + i * 0.235 + 0.2
                v0, v1 = 0.14 + j * 0.38, 0.14 + j * 0.38 + 0.3
                cell = [quad_pt(q, u0, v0), quad_pt(q, u1, v0), quad_pt(q, u1, v1), quad_pt(q, u0, v1)]
                s += poly(cell, "mothdark", lw(0.5))
        for u in (0.55, 0.93):
            p = quad_pt(q, u, 0.55)
            s += rotor(p[0], p[1] + 0.05, 0.26, w=0.7)
    s += curve([(-0.08, 0.32), (-0.25, 0.62), (-0.48, 0.72)], lw=0.9)
    s += curve([(0.08, 0.32), (0.25, 0.62), (0.48, 0.72)], lw=0.9)
    s += ellipse(-0.5, 0.72, 0.09, 0.05, "mothdark", extra=lw(0.7))
    s += ellipse(0.5, 0.72, 0.09, 0.05, "mothdark", extra=lw(0.7))
    s += ellipse(0, 0.05, 0.36, 0.3, "moth")
    s += f"\\fill[mothdark] (-0.33,-0.02) rectangle (0.33,0.06);\n"
    s += ellipse(0, 0.05, 0.36, 0.3, "none")
    s += halo(0, -0.28, 0.12, "pink", 3, 0.2)
    s += circle(0, -0.28, 0.12, "pink", lw(0.9))
    s += glint(-0.035, -0.25, 0.04)
    assert Count.n == ROTORS, "moth must have exactly four rotors"
    return s


def hornet(beam=True):
    """Long dark-teal hornet sniper drone, side view facing right: a long
    fuselage, a long red lens out front, four rotors (two near, two far)."""
    Count.n = 0
    s = ""
    for x in (-0.55, 0.65):  # far rotors, behind
        s += line((x + 0.2, 0.2), (x + 0.2, 0.5), lw=1.6)
        s += rotor(x + 0.2, 0.5, 0.36, hub="hornetdark")
    if beam:
        s += fillo(polyp([(2.3, 0.02), (9.0, -0.5), (9.0, 0.3)]), "red", 0.12)
    s += poly([(-1.55, 0.05), (-1.95, 0.35), (-1.85, -0.05), (-1.6, -0.12)], "hornetdark", lw(0.9))
    s += blob([(-1.7, 0.0), (-1.0, 0.24), (0.3, 0.3), (1.1, 0.22), (1.25, -0.02), (1.0, -0.25), (0.0, -0.3), (-1.0, -0.16)], "hornet", tension=0.55)
    s += curve([(-1.0, -0.06), (0.0, -0.1), (1.0, -0.06)], lw=0.7)
    for x in (-0.6, -0.2, 0.2):
        s += line((x, 0.27), (x + 0.05, -0.27), lw=0.6, colour="hornetdark")
    s += halo(-0.25, 0.34, 0.08, "glow", 2, 0.25)
    s += ellipse(-0.25, 0.32, 0.17, 0.07, "glow", extra=lw(0.7))
    s += rrect(1.0, -0.13, 2.2, 0.13, "red", 0.08, lw(1.0))
    s += f"\\fill[redlight] (1.15,0.03) rectangle (2.05,0.08);\n"
    s += rect(1.95, -0.16, 2.08, 0.16, "hornetdark", lw(0.8))
    s += ellipse(2.25, 0, 0.07, 0.15, "red", extra=lw(0.8))
    s += glint(2.26, 0.06, 0.03)
    for x in (-0.55, 0.65):  # near rotors
        s += line((x, 0.22), (x, 0.42), lw=1.8)
        s += rotor(x, 0.42, 0.42, hub="hornetdark")
    assert Count.n == ROTORS, "hornet must have exactly four rotors"
    return s


def warden():
    """Big coral warden, front view: four rotor pods on four arms and a
    glowing orange core; cyan weak-spot vents on its shoulders."""
    Count.n = 0
    s = ""

    def pod(x, y, r):
        o = rrect(x - 0.22, y - 0.32, x + 0.22, y + 0.12, "wardendark", 0.12, lw(0.9))
        o += f"\\fill[ink, opacity=0.25] {P(x - 0.22, y - 0.1)} rectangle {P(x + 0.22, y - 0.04)};\n"
        o += rotor(x, y + 0.08, r, hub="wardendark")
        return o

    for side in (-1, 1):
        s += limb([(0.4 * side, 0.45), (1.0 * side, 0.85), (1.25 * side, 0.85)], 0.09, 0.07, "wardendark")
        s += pod(1.3 * side, 0.85, 0.45)
    for side in (-1, 1):  # grippers hanging under
        s += limb([(0.35 * side, -0.6), (0.5 * side, -1.0), (0.35 * side, -1.25)], 0.07, 0.04, "armourdark")
    s += blob([(-0.95, 0.1), (-0.7, 0.68), (0, 0.85), (0.7, 0.68), (0.95, 0.1), (0.75, -0.55), (0, -0.78), (-0.75, -0.55)], "warden", tension=0.5)
    s += curve([(-0.88, 0.25), (0, 0.42), (0.88, 0.25)], lw=0.8)
    s += curve([(-0.8, -0.4), (0, -0.55), (0.8, -0.4)], lw=0.8)
    for side in (-1, 1):
        s += circle(0.62 * side, 0.12, 0.05, "wardendark", lw(0.6))
        s += halo(0.42 * side, 0.6, 0.06, "glow", 2, 0.25)
        s += ellipse(0.42 * side, 0.6, 0.12, 0.06, "glow", extra=lw(0.6))
    s += halo(0, -0.08, 0.32, "hot", 4, 0.12)
    s += circle(0, -0.08, 0.36, "wardendark", lw(1.0))
    s += circle(0, -0.08, 0.27, "hot", lw(0.9))
    s += circle(0, -0.08, 0.14, "hothi", lw(0.6))
    s += glint(-0.07, -0.01, 0.05)
    for side in (-1, 1):
        s += limb([(0.55 * side, 0.2), (1.25 * side, 0.15), (1.6 * side, 0.3)], 0.1, 0.08, "wardendark")
        s += pod(1.68 * side, 0.32, 0.55)
    assert Count.n == ROTORS, "warden must have exactly four rotor pods"
    return s


def spider_leg(hip, knee, foot, w0, w1, fill):
    Count.n += 1
    return limb([hip, knee, foot], w0, w1, fill)


def tick():
    """Small round sage-green tick spider, front 3/4: a round body, a dark
    visor with two red eyes, a cyan weak spot on its back, six legs."""
    Count.n = 0
    s = ""
    for side in (-1, 1):  # far legs
        s += spider_leg((0.25 * side, 0.3), (0.75 * side, 0.75), (0.95 * side, 0.0), 0.06, 0.04, "tickdark")
    for side in (-1, 1):
        s += spider_leg((0.3 * side, 0.22), (0.95 * side, 0.55), (1.2 * side, -0.05), 0.07, 0.045, "tickdark")
    s += circle(0, 0.4, 0.5, "tick")
    s += curve([(-0.45, 0.55), (0, 0.72), (0.45, 0.55)], lw=0.7)
    s += halo(0, 0.82, 0.08, "glow", 2, 0.25)
    s += ellipse(0, 0.8, 0.16, 0.07, "glow", extra=lw(0.7))
    s += rrect(-0.3, 0.12, 0.3, 0.38, "tickdark", 0.12, lw(0.8))
    s += circle(-0.12, 0.25, 0.06, "red", lw(0.5))
    s += circle(0.12, 0.25, 0.06, "red", lw(0.5))
    for side in (-1, 1):  # near legs
        s += spider_leg((0.3 * side, 0.12), (0.8 * side, 0.3), (0.9 * side, -0.25), 0.075, 0.05, "tickdark")
    assert Count.n == LEGS["tick"], "tick must have six legs"
    return s


def strider(aim=20):
    """Big coral/rust strider spider, front 3/4: a wide armoured body, six
    long jointed legs, a turret on top, a cyan weak spot under its belly."""
    Count.n = 0
    s = ""
    for side in (-1, 1):  # far legs
        s += spider_leg((0.6 * side, 0.55), (1.5 * side, 1.55), (2.0 * side, -0.4), 0.11, 0.06, "striderdark")
    for side in (-1, 1):
        s += spider_leg((0.8 * side, 0.45), (2.0 * side, 1.25), (2.5 * side, -0.6), 0.13, 0.07, "striderdark")
    s += halo(0, 0.0, 0.18, "glow", 3, 0.18)
    s += ellipse(0, 0.0, 0.32, 0.13, "glow", extra=lw(0.8))
    s += blob([(-1.2, 0.5), (-0.8, 0.95), (0, 1.05), (0.8, 0.95), (1.2, 0.5), (0.8, 0.12), (0, 0.05), (-0.8, 0.12)], "strider", tension=0.5)
    s += curve([(-1.05, 0.55), (0, 0.65), (1.05, 0.55)], lw=0.8)
    for x in (-0.6, 0.0, 0.6):
        s += circle(x, 0.35, 0.05, "striderdark", lw(0.6))
    # turret
    tb = place(0, 1.35, 1.0, rrect(0.05, -0.1, 1.15, 0.1, "metaldark", 0.04, lw(0.9)) + rect(1.05, -0.13, 1.2, 0.13, "armourdark", lw(0.8)), rot=aim)
    s += tb
    s += f"\\filldraw[inked,fill=striderdark] (-0.45,0.95) -- (-0.4,1.35) arc[start angle=180,end angle=0,radius=0.4] -- (0.45,0.95) -- cycle;\n"
    s += rect(-0.25, 1.32, 0.25, 1.46, "armourdark", lw(0.7))
    s += circle(0.12, 1.39, 0.04, "red", lw(0.4))
    for side in (-1, 1):  # near legs
        s += spider_leg((0.9 * side, 0.3), (2.2 * side, 0.9), (2.45 * side, -1.05), 0.15, 0.08, "striderdark")
    assert Count.n == LEGS["strider"], "strider must have six legs"
    return s


def queen(stride=0.0):
    """The street's queen, side 3/4 facing left, feet on y = 0, about 6.6
    tall: a long plum hull on six legs whose knees rise above it, rocket pods
    on her shoulders, the eye cannon under her brow with three red eyes, a
    grey plate over the cyan core in her belly and a cyan vent on her back."""
    Count.n = 0
    s = ""
    hips = [(-1.3, 3.0), (0.0, 2.9), (1.3, 3.0)]
    # front legs reach forward, the middle pair out, the back legs back
    reach = [-1.0, 0.1, 1.0]
    for k, (hx, hy) in enumerate(hips):  # far legs, a little behind and darker
        sx = stride * (1 if k % 2 else -1)
        r_ = reach[k] + 0.35
        s += spider_leg((hx + 0.3, hy + 0.4), (hx + r_ * 1.5 + 0.3, 5.7), (hx + r_ * 2.7 + 0.3 + sx, 0.0), 0.15, 0.09, "queendark")
    # the hull
    hull = [(-2.6, 3.2), (-2.2, 4.25), (-0.6, 4.75), (1.4, 4.7), (2.8, 4.1), (3.0, 3.1), (1.8, 2.45), (-0.4, 2.35), (-2.1, 2.6)]
    s += blob(hull, "queen", tension=0.55)
    s += blob([(-1.6, 4.45), (-0.4, 4.85), (1.3, 4.8), (2.2, 4.4), (1.1, 4.2), (-0.6, 4.25)], "armour", tension=0.5)
    s += curve([(-2.2, 3.4), (0.3, 3.15), (2.7, 3.45)], lw=0.8)
    for x in (-1.4, -0.4, 0.6, 1.6):
        s += circle(x, 3.0, 0.06, "queendark", lw(0.5))
    # the vent on her back
    s += halo(2.05, 4.55, 0.22, "glow", 3, 0.16)
    s += ellipse(2.05, 4.5, 0.42, 0.13, "glow", extra=lw(0.8))
    for dx in (-0.2, 0.0, 0.2):
        s += line((2.05 + dx - 0.08, 4.45), (2.05 + dx + 0.08, 4.58), lw=0.6)
    # the belly plate, the core glowing at its edge
    s += halo(0.0, 2.2, 0.3, "glow", 3, 0.16)
    s += ellipse(0.0, 2.25, 0.75, 0.18, "glow", extra=lw(0.8))
    s += ellipse(0.0, 2.38, 1.1, 0.2, "armour", extra=lw(0.9))
    # the rocket pods on her shoulders
    for (px, py) in [(-1.25, 4.75), (0.1, 4.95)]:
        s += rrect(px - 0.55, py, px + 0.55, py + 0.7, "armour", 0.08, lw(0.9))
        for (tx, ty) in [(-0.3, 0.2), (0.0, 0.2), (0.3, 0.2), (-0.3, 0.48), (0.0, 0.48), (0.3, 0.48)]:
            s += circle(px + tx, py + ty, 0.09, "armourdark", lw(0.5))
    # the brow, the eye cannon and three red eyes
    s += rrect(-3.0, 2.9, -2.1, 3.75, "queendark", 0.15, lw(1.0))
    s += rect(-4.2, 3.0, -2.9, 3.28, "metaldark", lw(0.9))
    s += rect(-4.4, 2.96, -4.15, 3.32, "armourdark", lw(0.8))
    s += halo(-4.45, 3.14, 0.06, "red", 2, 0.25)
    for (ex, ey) in [(-2.75, 3.5), (-2.45, 3.58), (-2.6, 3.25)]:
        s += circle(ex, ey, 0.08, "red", lw(0.5))
    s += line((-2.6, 2.5), (-2.95, 2.95), lw=0.9)
    for k, (hx, hy) in enumerate(hips):  # near legs
        sx = stride * (-1 if k % 2 else 1)
        r_ = reach[k]
        knee = (hx + r_ * 1.7, 6.1)
        s += circle(hx, hy, 0.26, "queendark", lw(0.8))
        s += spider_leg((hx, hy), knee, (hx + r_ * 3.1 + sx, 0.0), 0.18, 0.11, "queen")
        s += circle(knee[0], knee[1], 0.24, "armour", lw(0.8))
    assert Count.n == LEGS["queen"], "the queen must have six legs"
    return s


DRONES = {"wasp": wasp, "moth": moth, "hornet": hornet, "warden": warden, "tick": tick, "strider": strider, "queen": queen}


def drone(kind, x, y, s, rot=0.0, flip=False):
    return place(x, y, s, DRONES[kind](), rot=rot, flip=flip)


# --------------------------------------------------------------------------
# the raiders and their guns
# --------------------------------------------------------------------------

def quad_shotgun():
    """The homemade gun: a pistol frame with four double-barrel barrels in
    a 2x2 square, side view pointing +x, the grip at the origin; the muzzle
    plate is turned toward the viewer so the square of four bores shows."""
    s = poly([(-0.1, 0.0), (0.12, 0.0), (0.2, 0.42), (-0.05, 0.42)], "wooddark", lw(0.9))
    s += rrect(-0.2, 0.36, 0.55, 0.62, "metaldark", 0.05, lw(0.9))
    s += curve([(0.08, 0.36), (0.12, 0.22), (0.26, 0.22), (0.3, 0.36)], lw=0.8)
    s += rrect(0.45, 0.4, 1.75, 0.58, "metal", 0.04, lw(0.9))
    s += rrect(0.45, 0.58, 1.75, 0.76, "metal", 0.04, lw(0.9))
    s += rect(0.7, 0.36, 0.82, 0.8, "wood", lw(0.7))
    s += rect(1.35, 0.36, 1.47, 0.8, "wood", lw(0.7))
    s += rrect(1.72, 0.3, 2.06, 0.86, "metaldark", 0.05, lw(0.9))
    for bx in (1.81, 1.97):
        for by in (0.44, 0.7):
            s += circle(bx, by, 0.055, "ink", lw(0.4))
    return s


def ak_rocket():
    """The AK with a rocket tube strapped on top, side view pointing +x,
    the grip at the origin."""
    s = poly([(-1.25, 0.25), (-0.4, 0.45), (-0.35, 0.68), (-1.2, 0.62)], "wood", lw(0.9))
    s += poly([(-0.05, 0.0), (0.15, 0.0), (0.2, 0.42), (0.0, 0.42)], "wooddark", lw(0.9))
    s += rrect(-0.45, 0.4, 0.8, 0.7, "metaldark", 0.05, lw(0.9))
    s += poly([(0.3, 0.42), (0.55, 0.42), (0.68, -0.15), (0.45, -0.25)], "metaldark", lw(0.9))
    s += rrect(0.75, 0.46, 1.35, 0.66, "wood", 0.06, lw(0.9))
    s += rect(1.35, 0.52, 2.15, 0.6, "metaldark", lw(0.8))
    s += rect(2.1, 0.49, 2.22, 0.63, "metaldark", lw(0.7))
    # the rocket tube on top, with its strap clamps and warhead
    s += rrect(-0.6, 0.78, 1.25, 1.08, "sagedark", 0.06, lw(0.9))
    s += poly([(1.25, 0.78), (1.55, 0.83), (1.82, 0.93), (1.55, 1.03), (1.25, 1.08)], "red", lw(0.9))
    s += rect(-0.62, 0.74, -0.5, 1.12, "metaldark", lw(0.7))
    for cx in (-0.1, 0.65):
        s += rect(cx, 0.68, cx + 0.1, 1.1, "wooddark", lw(0.6))
    return s


def raider(jacket, pants="asphaltdark", helmet="armour", pack="sagedark", skin="skin",
           gun=None, front=False, step=1, kit=0):
    """A raider, feet at the origin, about 3.4 tall. Back view (walking
    away, a backpack) or front view (face masked and goggled). gun is
    'quad' (held up at the shoulder) or 'ak' (resting on the shoulder)."""
    s = ""
    boot = "ink!70!wooddark"
    # gun behind the body for the back view
    if gun == "ak" and not front:
        s += place(0.45, 2.55, 0.85, ak_rocket(), rot=38)
    # legs: one planted, one stepping (heel up) when walking away
    lf = [(-0.2, 1.5), (-0.24, 0.8), (-0.26, 0.18)]
    rf = [(0.2, 1.5), (0.24, 0.85), (0.24, 0.42 if step else 0.18)]
    s += limb(lf, 0.19, 0.15, pants)
    s += limb(rf, 0.19, 0.15, pants)
    s += curve([(-0.38, 0.95), (-0.1, 0.9)], lw=0.6)  # knee pad seams
    s += rrect(-0.45, 1.0 - 0.05, -0.05, 1.22 - 0.05, "armourdark", 0.06, lw(0.7)) if kit % 2 == 0 else ""
    s += rrect(-0.44, -0.02, -0.08, 0.3, boot, 0.08, lw(0.9))
    if step and not front:
        s += rrect(0.07, 0.28, 0.41, 0.56, boot, 0.08, lw(0.9))
        s += f"\\fill[ink, opacity=0.35] (0.1,0.3) rectangle (0.38,0.36);\n"
    else:
        s += rrect(0.06, -0.02, 0.42, 0.3, boot, 0.08, lw(0.9))
    # torso
    s += rrect(-0.5, 1.35, 0.5, 2.85, jacket, 0.22)
    s += rect(-0.47, 1.42, 0.47, 1.58, "wooddark", lw(0.8))
    s += rect(-0.08, 1.4, 0.08, 1.6, "gold", lw(0.6))
    s += rrect(0.18, 1.2, 0.44, 1.5, "sagedark" if kit % 2 else "wood", 0.04, lw(0.7))  # pouch
    # arms
    if gun == "quad":
        s += limb([(0.45, 2.7), (0.8, 2.25), (0.72, 2.0)], 0.15, 0.12, jacket)
        s += circle(0.72, 2.0, 0.13, "wooddark", lw(0.8))
    else:
        s += limb([(0.45, 2.7), (0.62, 2.1), (0.6, 1.55)], 0.15, 0.12, jacket)
        s += circle(0.6, 1.5, 0.13, "wooddark", lw(0.8))
    if gun == "ak":
        s += limb([(-0.45, 2.7), (-0.25, 2.2), (0.15, 2.25)], 0.15, 0.12, jacket)
        s += circle(0.18, 2.27, 0.13, "wooddark", lw(0.8))
    else:
        s += limb([(-0.45, 2.7), (-0.65, 2.1), (-0.6, 1.55)], 0.15, 0.12, jacket)
        s += circle(-0.6, 1.5, 0.13, "wooddark", lw(0.8))
    if front:
        # vest with pouches, a zip, a radio
        s += rrect(-0.38, 1.65, 0.38, 2.55, "sagedark" if kit % 2 else "armourdark", 0.1, lw(0.9))
        for px in (-0.3, -0.05, 0.2):
            s += rect(px, 1.72, px + 0.18, 1.98, "wood" if kit % 2 else "sage", lw(0.7))
        s += rect(0.12, 2.15, 0.28, 2.45, "ink!70!metaldark", lw(0.7))
        s += line((0.2, 2.45), (0.24, 2.7), lw=0.8)
        # head: masked face, goggles, helmet
        s += rect(-0.1, 2.8, 0.1, 2.95, skin, lw(0.8))
        s += ellipse(0, 3.12, 0.25, 0.3, skin)
        s += f"\\filldraw[inked,fill=ink!60!asphaltdark] (-0.25,3.08) -- (0.25,3.08) -- (0.18,2.86) -- (0,2.82) -- (-0.18,2.86) -- cycle;\n"
        s += f"\\filldraw[inked,fill={helmet}] (-0.33,3.22) arc[start angle=180,end angle=0,radius=0.33] -- cycle;\n"
        s += rrect(-0.26, 3.13, 0.26, 3.28, "ink", 0.06, lw(0.6))
        s += circle(-0.12, 3.205, 0.07, "glow", lw(0.5))
        s += circle(0.12, 3.205, 0.07, "glow", lw(0.5))
    else:
        # backpack with a bedroll and straps
        s += line((-0.32, 2.85), (-0.3, 2.3), lw=2.0, colour="ink")
        s += line((0.32, 2.85), (0.3, 2.3), lw=2.0, colour="ink")
        s += rrect(-0.42, 1.7, 0.42, 2.68, pack, 0.12)
        s += rrect(-0.38, 2.38, 0.38, 2.62, f"{pack}!80!ink", 0.08, lw(0.8))
        s += rrect(-0.26, 1.8, 0.26, 2.12, f"{pack}!85!paper", 0.06, lw(0.8))
        s += line((-0.1, 2.38), (-0.1, 2.25), lw=1.0)
        s += line((0.1, 2.38), (0.1, 2.25), lw=1.0)
        if kit % 3 == 0:
            s += ellipse(0, 2.74, 0.45, 0.11, "wallE" if kit else "cream", extra=lw(0.8))
            s += line((-0.18, 2.63), (-0.18, 2.85), lw=0.7)
            s += line((0.18, 2.63), (0.18, 2.85), lw=0.7)
        if kit % 3 == 1:
            s += line((0.3, 2.6), (0.5, 3.7), lw=0.8)
            s += circle(0.5, 3.7, 0.04, "red", lw(0.4))
        # back of the head: neck, helmet with goggle strap
        s += ellipse(-0.24, 2.97, 0.06, 0.09, skin, extra=lw(0.6))
        s += ellipse(0.24, 2.97, 0.06, 0.09, skin, extra=lw(0.6))
        s += rrect(-0.24, 2.76, 0.24, 3.05, skin, 0.1, lw(0.8))
        s += f"\\filldraw[inked,fill={helmet}] (-0.37,3.0) arc[start angle=180,end angle=0,radius=0.37] -- cycle;\n"
        s += rrect(-0.39, 2.96, 0.39, 3.07, f"{helmet}!80!ink", 0.05, lw(0.7))
        s += curve([(-0.34, 3.14), (0, 3.18), (0.34, 3.14)], lw=1.6, colour="ink!80!asphaltdark")
        s += rrect(-0.08, 3.12, 0.08, 3.22, "metal", 0.03, lw(0.5))
    if gun == "quad":
        s += place(0.72, 1.95, 0.8, quad_shotgun(), rot=52)
    if gun == "ak" and front:
        s += place(0.0, 1.95, 0.85, ak_rocket(), rot=30)
    return s


# --------------------------------------------------------------------------
# the van (rear view, doors open) and its load
# --------------------------------------------------------------------------

def gear_bag(x, y, w, h, fill):
    s = blob([(x, y), (x + w, y), (x + w + 0.08, y + h * 0.6), (x + w * 0.8, y + h), (x + w * 0.2, y + h), (x - 0.08, y + h * 0.6)], fill, tension=0.5)
    s += curve([(x + w * 0.25, y + h * 0.95), (x + w * 0.5, y + h * 1.35), (x + w * 0.75, y + h * 0.95)], lw=1.4)
    s += line((x + 0.1, y + h * 0.55), (x + w - 0.1, y + h * 0.55), lw=0.7)
    s += rect(x + w * 0.4, y + h * 0.15, x + w * 0.6, y + h * 0.45, "metal", lw(0.6))
    return s


def parts_crate(x, y, w, h):
    """A crate of gun parts: barrels, a scope, a magazine and a grip
    sticking out of an open wooden crate."""
    s = ""
    s += rrect(x + 0.15, y + h - 0.1, x + 0.35, y + h + 0.9, "metaldark", 0.05, lw(0.8))
    s += rrect(x + 0.4, y + h - 0.1, x + 0.58, y + h + 0.75, "metal", 0.05, lw(0.8))
    s += place(x + w * 0.62, y + h + 0.15, 1.0, rrect(-0.1, 0, 0.1, 0.65, "metaldark", 0.04, lw(0.8)) + ellipse(0, 0.65, 0.16, 0.08, "glow", extra=lw(0.6)), rot=-25)
    s += poly([(x + w - 0.5, y + h - 0.1), (x + w - 0.3, y + h - 0.1), (x + w - 0.15, y + h + 0.45), (x + w - 0.35, y + h + 0.5)], "metaldark", lw(0.8))
    s += poly([(x + w * 0.35, y + h - 0.1), (x + w * 0.5, y + h - 0.1), (x + w * 0.55, y + h + 0.4), (x + w * 0.38, y + h + 0.42)], "wooddark", lw(0.8))
    s += rect(x, y, x + w, y + h, "wood")
    for k in (1, 2):
        s += line((x, y + h * k / 3), (x + w, y + h * k / 3), lw=0.7)
    s += line((x + 0.08, y + 0.08), (x + w - 0.08, y + h - 0.08), lw=0.7)
    s += rect(x + w * 0.4, y + h * 0.4, x + w * 0.6, y + h * 0.6, "gold", lw(0.6))
    return s


def clipboard(x, y, s_, rot_):
    """A clipboard with a mission order: paper, ink bars (not letters), a
    red stamp and a map scrap."""
    b = rrect(-0.42, -0.6, 0.42, 0.6, "wood", 0.06, lw(0.9))
    b += rect(-0.34, -0.52, 0.34, 0.46, "paper", lw(0.7))
    b += rrect(-0.15, 0.4, 0.15, 0.56, "metal", 0.04, lw(0.7))
    for k, wd in enumerate((0.5, 0.4, 0.45, 0.3)):
        b += f"\\fill[ink!60!paper] {P(-0.26, 0.28 - k * 0.12)} rectangle {P(-0.26 + wd, 0.31 - k * 0.12)};\n"
    b += rect(-0.26, -0.44, 0.06, -0.2, "sagelight", lw(0.5))
    b += line((-0.22, -0.4), (-0.05, -0.25), lw=0.6, colour="red")
    b += circle(0.18, -0.33, 0.1, "none", ",draw=red,line width=0.9pt")
    return place(x, y, s_, b, rot=rot_)


def van(x0, y0, w, h, cargo=""):
    """The back of the beat-up van, doors swung wide: the dark interior
    with a floor, side walls and a lit ceiling lamp, gear bags, a crate of
    gun parts, the clipboard on the wall; bumper, step, tail lights,
    wheels, dents and rust. cargo is drawn standing in the doorway."""
    x1, y1 = x0 + w, y0 + h
    s = ""
    # wheels under the body
    for wx in (x0 + 0.35, x1 - 1.35):
        s += rrect(wx, y0 - 0.95, wx + 1.0, y0 + 0.2, "ink!80!asphaltdark", 0.2)
    s += f"\\fill[ink, opacity=0.25] {P(x0 - 1.4, y0 - 1.0)} rectangle {P(x1 + 1.4, y0 - 0.8)};\n"
    # body
    s += rrect(x0, y0, x1, y1, "teal!55!sagelight", 0.35)
    s += f"\\fill[teal!55!sagelight!85!ink] {P(x0 + 0.1, y1 - 0.5)} rectangle {P(x1 - 0.1, y1 - 0.3)};\n"
    # opening and interior
    ox0, ox1, oy0, oy1 = x0 + 0.5, x1 - 0.5, y0 + 0.55, y1 - 0.45
    bx0, bx1, by0, by1 = ox0 + 1.0, ox1 - 1.0, oy0 + 0.9, oy1 - 0.75
    s += rect(ox0, oy0, ox1, oy1, "ink!75!wooddark")
    s += poly([(ox0, oy0), (ox1, oy0), (bx1, by0), (bx0, by0)], "wooddark!70!ink")  # floor
    for k in range(1, 6):
        t = k / 6
        s += line(lerp((ox0, oy0), (ox1, oy0), t), lerp((bx0, by0), (bx1, by0), t), lw=0.5, colour="ink")
    s += poly([(ox0, oy1), (bx0, by1), (bx1, by1), (ox1, oy1)], "ink!60!metaldark")  # ceiling
    s += poly([(ox0, oy0), (bx0, by0), (bx0, by1), (ox0, oy1)], "ink!55!sagedark")  # left wall
    s += poly([(ox1, oy0), (bx1, by0), (bx1, by1), (ox1, oy1)], "ink!55!sagedark")
    s += rect(bx0, by0, bx1, by1, "ink!65!sagedark")  # bulkhead
    s += rrect((bx0 + bx1) / 2 - 0.55, by1 - 0.95, (bx0 + bx1) / 2 + 0.55, by1 - 0.35, "window!50!skylow", 0.1, lw(0.9))
    # ceiling lamp and its light
    lx = (ox0 + ox1) / 2
    s += fillo(polyp([(lx - 0.3, oy1 - 0.4), (lx + 0.3, oy1 - 0.4), (lx + 2.0, oy0 + 0.1), (lx - 2.0, oy0 + 0.1)]), "window", 0.12)
    s += rrect(lx - 0.35, oy1 - 0.5, lx + 0.35, oy1 - 0.3, "window", 0.08, lw(0.8))
    # racks on the walls: a shelf of ammo tins on the left, clipboard right
    s += poly([(ox0 + 0.05, oy0 + 2.4), (bx0, by0 + 1.6), (bx0, by0 + 1.7), (ox0 + 0.05, oy0 + 2.55)], "metal", lw(0.7))
    for k in range(3):
        t0 = 0.12 + k * 0.28
        p = lerp((ox0 + 0.05, oy0 + 2.55), (bx0, by0 + 1.7), t0)
        q = lerp((ox0 + 0.05, oy0 + 2.55), (bx0, by0 + 1.7), t0 + 0.2)
        s += poly([p, q, (q[0], q[1] + 0.4 - 0.1 * k), (p[0], p[1] + 0.45 - 0.1 * k)], "sagedark" if k != 1 else "gold", lw(0.7))
    s += clipboard(ox1 - 0.55, oy0 + 2.7, 0.9, 6)
    # cargo on the floor
    s += gear_bag(ox0 + 0.25, oy0 + 0.15, 1.5, 0.85, "jacketd!80!wooddark")
    s += gear_bag(ox0 + 0.9, oy0 + 0.75, 1.1, 0.6, "sagedark")
    s += parts_crate(ox1 - 2.0, oy0 + 0.15, 1.55, 0.95)
    s += cargo
    # door frame, bumper, step, tail lights
    s += f"\\draw[inked, line width=1.6pt] {P(ox0, oy0)} rectangle {P(ox1, oy1)};\n"
    s += rrect(x0 - 0.2, y0 - 0.35, x1 + 0.2, y0 + 0.2, "metaldark", 0.1)
    s += rect((x0 + x1) / 2 - 1.0, y0 - 0.55, (x0 + x1) / 2 + 1.0, y0 - 0.3, "metal", lw(0.9))
    s += rect((x0 + x1) / 2 - 0.55, y0 - 0.25, (x0 + x1) / 2 + 0.55, y0 + 0.12, "cream", lw(0.8))
    for tx in (x0 + 0.08, x1 - 0.42):
        s += rrect(tx, y0 + 1.0, tx + 0.34, y0 + 2.0, "red", 0.08, lw(0.9))
        s += rect(tx, y0 + 1.7, tx + 0.34, y0 + 2.0, "gold", lw(0.7))
    # rust and dents
    s += blob([(x0 + 0.1, y0 + 0.3), (x0 + 0.45, y0 + 0.25), (x0 + 0.42, y0 + 0.5), (x0 + 0.15, y0 + 0.6)], "wallE", lw(0.6))
    s += blob([(x1 - 0.45, y1 - 0.3), (x1 - 0.15, y1 - 0.35), (x1 - 0.12, y1 - 0.12), (x1 - 0.4, y1 - 0.12)], "wallE", lw(0.6))
    # roof rack with a spare tyre and a jerrycan
    s += rect(x0 + 0.4, y1 - 0.05, x1 - 0.4, y1 + 0.12, "metaldark", lw(0.8))
    for px in (x0 + 0.8, x1 - 0.8):
        s += line((px, y1), (px, y1 + 0.3), lw=1.2)
    s += line((x0 + 0.5, y1 + 0.3), (x1 - 0.5, y1 + 0.3), lw=1.5)
    s += rrect(x0 + 1.0, y1 + 0.3, x0 + 2.6, y1 + 0.75, "ink!80!asphaltdark", 0.2)
    s += rrect(x1 - 2.0, y1 + 0.3, x1 - 1.3, y1 + 0.95, "red", 0.06, lw(0.8))
    # the doors, swung open, inner faces showing
    for side in (-1, 1):
        hx = x0 if side < 0 else x1
        ex = hx + 1.35 * side
        door = [(hx, y0 + 0.15), (ex, y0 - 0.25), (ex, y1 + 0.35), (hx, y1 - 0.15)]
        s += poly(door, "teal!55!sagelight!80!ink")
        inner = [quad_pt(door, 0.12, 0.05), quad_pt(door, 0.88, 0.05), quad_pt(door, 0.88, 0.95), quad_pt(door, 0.12, 0.95)]
        s += poly(inner, "ink!55!sagedark", lw(0.8))
        win = [quad_pt(door, 0.2, 0.6), quad_pt(door, 0.8, 0.6), quad_pt(door, 0.8, 0.9), quad_pt(door, 0.2, 0.9)]
        s += poly(win, "window!60!skylow", lw(0.8))
        s += f"\\fill[white, opacity=0.4] {polyp([quad_pt(door, 0.3, 0.62), quad_pt(door, 0.4, 0.62), quad_pt(door, 0.32, 0.88), quad_pt(door, 0.24, 0.88)])};\n"
        for v in (0.25, 0.75):
            hp = quad_pt(door, 0.02, v)
            s += rect(hp[0] - 0.08, hp[1] - 0.12, hp[0] + 0.08, hp[1] + 0.12, "metaldark", lw(0.7))
        s += line(quad_pt(door, 0.75, 0.4), quad_pt(door, 0.75, 0.5), lw=2.0)
    return s


# --------------------------------------------------------------------------
# lot and street props
# --------------------------------------------------------------------------

def car_back(colour, w=0.9):
    """A parked car from behind, in metres (1.8 wide), wheels on y=0."""
    e = lw(w)
    s = rrect(-0.85, -0.02, -0.55, 0.35, "ink!80!asphaltdark", 0.06, e)
    s += rrect(0.55, -0.02, 0.85, 0.35, "ink!80!asphaltdark", 0.06, e)
    s += poly([(-0.72, 0.92), (-0.58, 1.42), (0.58, 1.42), (0.72, 0.92)], colour, e)
    s += poly([(-0.6, 0.95), (-0.5, 1.33), (0.5, 1.33), (0.6, 0.95)], "windark!70!skylow", e)
    s += rrect(-0.9, 0.18, 0.9, 0.95, colour, 0.12, e)
    s += rect(-0.85, 0.22, 0.85, 0.36, "metaldark", e)
    s += rect(-0.82, 0.62, -0.55, 0.8, "red", e)
    s += rect(0.55, 0.62, 0.82, 0.8, "red", e)
    s += rect(-0.22, 0.42, 0.22, 0.58, "cream", e)
    return s


def car_side(colour, w=0.9, flat=False):
    """A sedan from the side, in metres (4.4 long), wheels on y=0."""
    e = lw(w)
    s = f"\\filldraw[inked,fill={colour},rounded corners=0.2cm{e}] (-2.2,0.25) rectangle (2.2,0.9);\n"
    s += f"\\filldraw[inked,fill={colour},rounded corners=0.15cm{e}] (-1.3,0.88) -- (-0.85,1.4) -- (0.75,1.4) -- (1.35,0.88) -- cycle;\n"
    s += f"\\filldraw[inked,fill=windark!70!skylow{e}] (-1.1,0.94) -- (-0.75,1.32) -- (-0.05,1.32) -- (-0.05,0.94) -- cycle;\n"
    s += f"\\filldraw[inked,fill=windark!70!skylow{e}] (0.05,0.94) -- (0.05,1.32) -- (0.68,1.32) -- (1.1,0.94) -- cycle;\n"
    s += rect(2.05, 0.55, 2.22, 0.72, "window", e)
    s += rect(-2.22, 0.5, -2.08, 0.7, "red", e)
    for wx in (-1.35, 1.35):
        if flat:
            s += f"\\filldraw[inked,fill=ink!80!asphaltdark{e}] {P(wx - 0.36, 0)} rectangle {P(wx + 0.36, 0.3)};\n"
        else:
            s += circle(wx, 0.3, 0.32, "ink!80!asphaltdark", e)
            s += circle(wx, 0.3, 0.13, "metal", e)
    return s


def cart(w=0.8, tipped=False):
    """A shopping cart from the side, in metres, wheels on y=0."""
    e = lw(w)
    s = ""
    basket = [(-0.45, 0.45), (0.35, 0.45), (0.5, 1.0), (-0.55, 1.0)]
    s += f"\\filldraw[draw=ink, line width={f(w)}pt, fill=metallight, fill opacity=0.35, line join=round] {polyp(basket)};\n"
    for k in range(1, 6):
        s += line(lerp(basket[0], basket[1], k / 6), lerp(basket[3], basket[2], k / 6), lw=w * 0.6)
    s += line(lerp(basket[0], basket[3], 0.5), lerp(basket[1], basket[2], 0.5), lw=w * 0.6)
    s += line((0.5, 1.0), (0.7, 1.12), lw=w * 1.3)
    s += line((0.66, 1.1), (0.68, 1.2), lw=w * 1.3)
    s += line((-0.4, 0.45), (-0.38, 0.12), lw=w)
    s += line((0.32, 0.45), (0.35, 0.12), lw=w)
    s += line((-0.42, 0.18), (0.4, 0.18), lw=w)
    s += line((0.6, 1.1), (0.68, 1.15), lw=w * 2.4, colour="red")
    for wx in (-0.38, 0.35):
        s += circle(wx, 0.07, 0.07, "ink", e)
    if tipped:
        return place(0, 0, 1, s, rot=-95)
    return s


def light_pole(x, y, h, w=1.0, lit=False):
    """A parking-lot light pole: a tall pole, a concrete footing, two
    shoebox lamps on a crossarm."""
    out = rect(x - 0.12 * h / 8, y, x + 0.12 * h / 8, y + 0.07 * h, "concrete", lw(w))
    pw = 0.05 * h / 8
    out += rect(x - pw, y + 0.07 * h, x + pw, y + h, "metaldark", lw(w))
    arm = 0.1 * h
    out += rect(x - arm, y + h - 0.01 * h, x + arm, y + h + 0.012 * h, "metaldark", lw(w))
    for sx in (-1, 1):
        lx = x + sx * arm
        out += rect(lx - 0.05 * h, y + h - 0.035 * h, lx + 0.05 * h, y + h + 0.01 * h, "metal", lw(w))
        if lit:
            out += rect(lx - 0.04 * h, y + h - 0.045 * h, lx + 0.04 * h, y + h - 0.035 * h, "window", lw(w * 0.8))
    return out


def crack(rng, x, y, n, step, ang, w=0.7):
    pts_ = [(x, y)]
    branches = ""
    for i in range(n):
        ang += rng.uniform(-35, 35)
        x += math.cos(math.radians(ang)) * step
        y += math.sin(math.radians(ang)) * step * 0.45
        pts_.append((x, y))
        if rng.random() < 0.3:
            a2 = ang + rng.choice([-60, 60])
            branches += line((x, y), (x + math.cos(math.radians(a2)) * step * 0.7, y + math.sin(math.radians(a2)) * step * 0.3), lw=w * 0.8)
    s = f"\\draw[ink, line width={f(w)}pt, line join=round, line cap=round] {' -- '.join(P(*p) for p in pts_)};\n"
    return s + branches, pts_


def blades(x, y, s, fill="greenB"):
    """A clump of grass blades, wide enough at small sizes that the green
    reads through the ink."""
    out = ""
    for a, l in ((-34, 0.5), (-14, 0.8), (6, 0.95), (24, 0.7), (42, 0.45)):
        tip = (x + math.sin(math.radians(a)) * l * s, y + math.cos(math.radians(a)) * l * s)
        out += leaf((x + a * 0.004 * s, y), tip, 0.13 * s, fill, bend=0.06 * s * (1 if a > 0 else -1), extra=lw(0.6))
    return out


def weeds(rng, x, y, s):
    """A clump of weeds pushing up through asphalt: blades and a flower."""
    out = blades(x, y, s, rng.choice(["greenB", "greenA", "greenC"]))
    if rng.random() < 0.4:
        out += line((x + 0.05 * s, y), (x + 0.1 * s, y + 0.9 * s), lw=0.7)
        out += circle(x + 0.1 * s, y + 0.92 * s, 0.07 * s, rng.choice(["gold", "pink", "cream"]), lw(0.5))
    return out


def shrub(x, y, s, rng, fill=None):
    fill = fill or rng.choice(["greenA", "greenB", "greenD", "sage"])
    out = ""
    for dx, dy, r in ((-0.55, 0.3, 0.42), (0.55, 0.32, 0.42), (0, 0.5, 0.55), (-0.25, 0.25, 0.4), (0.3, 0.22, 0.38)):
        out += circle(x + dx * s, y + dy * s, r * s, fill)
    out += f"\\fill[{fill}] {P(x - 0.8 * s, y - 0.02)} rectangle {P(x + 0.8 * s, y + 0.3 * s)};\n"
    out += line((x - 0.95 * s, y), (x + 0.95 * s, y), lw=artlib.CUR_INK)
    return out


def tree(x, y, s, rng, fill=None):
    fill = fill or rng.choice(["greenA", "greenB", "greenD"])
    out = poly([(x - 0.12 * s, y), (x + 0.12 * s, y), (x + 0.08 * s, y + 1.4 * s), (x - 0.08 * s, y + 1.4 * s)], "wooddark")
    pts_ = []
    for k in range(9):
        a = k / 9 * 2 * math.pi
        r = s * rng.uniform(0.85, 1.1)
        pts_.append((x + math.cos(a) * r * 1.0, y + 2.0 * s + math.sin(a) * r * 0.9))
    out += blob(pts_, fill, tension=0.8)
    out += curve([(x - 0.4 * s, y + 1.9 * s), (x - 0.1 * s, y + 2.2 * s), (x + 0.3 * s, y + 2.0 * s)], lw=0.7)
    return out


# --------------------------------------------------------------------------
# the mall from the lot
# --------------------------------------------------------------------------

def glass_front(x0, y0, x1, y1, cols, rows, rng, broken=0.15, w=0.7):
    """A tall glass entrance front: mullions, a few dark broken panes, a
    glint."""
    s = rect(x0, y0, x1, y1, "window!70!skylow", lw(w))
    dx, dy = (x1 - x0) / cols, (y1 - y0) / rows
    for i in range(cols):
        for j in range(rows):
            if rng.random() < broken:
                s += f"\\fill[windark] {P(x0 + i * dx, y0 + j * dy)} rectangle {P(x0 + (i + 1) * dx, y0 + (j + 1) * dy)};\n"
    for i in range(1, cols):
        s += line((x0 + i * dx, y0), (x0 + i * dx, y1), lw=w * 0.8)
    for j in range(1, rows):
        s += line((x0, y0 + j * dy), (x1, y0 + j * dy), lw=w * 0.8)
    s += f"\\fill[white, opacity=0.35] {polyp([(x0 + dx * 0.3, y0 + 0.1), (x0 + dx * 0.6, y0 + 0.1), (x0 + dx * 1.3, y1 - 0.1), (x0 + dx * 1.0, y1 - 0.1)])};\n"
    return s


def vine(x, y, length, rng, w=0.6):
    """Ivy hanging from a roof edge: a stem and leaves."""
    pts_ = [(x, y)]
    for k in range(1, 5):
        pts_.append((x + rng.uniform(-0.12, 0.12), y - length * k / 4))
    s = curve(pts_, lw=w)
    for k in range(1, 9):
        px, py = pts_[0][0] + rng.uniform(-0.08, 0.08), y - length * k / 9
        side = 1 if k % 2 else -1
        s += leaf((px, py), (px + side * 0.22, py - 0.08), 0.06, rng.choice(["greenA", "greenB", "greenD"]), extra=lw(w))
    return s


def mall(rng, base, sc):
    """The Streets at Southpoint from its parking lot: a long two-level
    stucco-and-brick body, two square entrance towers with tall glass
    fronts and canopies, a cinema block with a blade sign of coloured
    squares, two big anchor stores with logo shapes (never letters),
    roof clutter, ivy and shrubs. sc is cm per metre at the mall."""
    s = ""
    m = lambda metres: base + metres * sc

    def body(x0, x1, top, brick_to, fill="stone"):
        o = rect(x0, base, x1, m(top), fill)
        o += rect(x0, base, x1, m(brick_to), "wallE!85!stone", lw(0.8))
        o += rect(x0 - 0.06, m(top) - 0.12, x1 + 0.06, m(top) + 0.08, "concrete", lw(0.8))
        return o

    # main body (two levels), running the whole width behind everything
    s += body(7.6, 25.8, 11, 4.5)
    s += line((7.6, m(5.8)), (25.8, m(5.8)), lw=0.7)
    x = 8.1
    while x < 25.5:
        s += rect(x, base, x + 0.14, m(10.6), "stonedark", lw(0.6))
        if rng.random() < 0.5:
            s += rect(x + 0.35, m(6.6), x + 0.75, m(9.3), "windark", lw(0.6))
        x += rng.uniform(1.0, 1.4)
    # roof clutter on the main body
    for rx in (9.4, 13.6, 14.4, 18.2, 23.2):
        s += rect(rx, m(11.1), rx + 0.42, m(12.4), "metal", lw(0.7))
        s += line((rx + 0.1, m(12.4)), (rx + 0.32, m(12.4)), lw=0.6)
    for rx in (12.9, 17.5, 22.0):
        s += line((rx, m(11.1)), (rx, m(14.5)), lw=0.7)
        s += line((rx - 0.15, m(13.5)), (rx + 0.15, m(13.5)), lw=0.6)
    s += line((7.6, m(12.2)), (25.8, m(12.2)), lw=0.5, colour="metaldark")
    # left anchor store
    ax0, ax1 = 2.4, 7.8
    s += body(ax0, ax1, 16, 3.5, "wallD!35!stone")
    s += rect(ax0 + 0.4, m(12.5), ax1 - 0.4, m(13.0), "stonedark", lw(0.6))
    s += poly([(5.1, m(7.6)), (5.55, m(9.6)), (5.1, m(11.6)), (4.65, m(9.6))], "teal")
    s += poly([(5.1, m(8.5)), (5.3, m(9.6)), (5.1, m(10.7)), (4.9, m(9.6))], "gold", lw(0.7))
    s += rect(3.4, m(9.0), 4.4, m(9.4), "teal", lw(0.7))
    s += rect(5.8, m(9.0), 6.8, m(9.4), "teal", lw(0.7))
    s += rect(3.4, m(9.8), 4.2, m(10.2), "gold", lw(0.7))
    s += rect(6.0, m(9.8), 6.8, m(10.2), "gold", lw(0.7))
    s += rect(4.4, base, 5.8, m(3.4), "windark", lw(0.7))
    s += rect(4.2, m(3.4), 6.0, m(4.0), "teal", lw(0.8))
    for vx in (2.9, 7.1):
        s += vine(vx, m(15.5), rng.uniform(1.0, 1.8), rng)
    for rx in (3.2, 6.2):
        s += rect(rx, m(16.1), rx + 0.5, m(17.4), "metal", lw(0.7))
    # entrance tower A
    tx0, tx1 = 10.4, 12.7
    s += rect(tx0, base, tx1, m(21), "concrete")
    s += rect(tx0 - 0.08, m(20.5), tx1 + 0.08, m(21.6), "stonedark", lw(0.8))
    s += rect(tx0 - 0.15, m(21.6), tx1 + 0.15, m(22.2), "concrete", lw(0.8))
    s += glass_front(tx0 + 0.3, base, tx1 - 0.3, m(18.5), 4, 6, rng)
    s += rect(tx0 - 0.5, m(4.2), tx1 + 0.5, m(5.1), "tealdark", lw(0.9))
    for px in (tx0 - 0.35, tx1 + 0.35):
        s += line((px, base), (px, m(4.2)), lw=1.0)
    s += vine(tx0 + 0.05, m(20.4), 1.6, rng)
    s += vine(tx1 - 0.05, m(20.4), 0.9, rng)
    # cinema block with its blade sign (coloured squares, not letters)
    cx0, cx1 = 14.8, 17.2
    s += body(cx0, cx1, 15, 4.5, "wallA!55!stone")
    s += rect(cx0 + 0.2, m(5.5), cx1 - 0.2, m(7.2), "gold", lw(0.8))
    for k in range(6):
        s += circle(cx0 + 0.45 + k * 0.3, m(6.35), 0.06, "cream", lw(0.4))
    s += rect(cx1 - 0.55, m(8), cx1 - 0.05, m(18.5), "magenta!70!wallC", lw(0.9))
    for k in range(7):
        s += rect(cx1 - 0.45, m(8.6) + k * 0.31, cx1 - 0.15, m(8.6) + k * 0.31 + 0.22, rng.choice(["cream", "gold", "glowhi"]), lw(0.5))
    # entrance tower B
    tx0, tx1 = 19.4, 21.4
    s += rect(tx0, base, tx1, m(20), "concrete")
    s += rect(tx0 - 0.15, m(20), tx1 + 0.15, m(20.6), "concrete", lw(0.8))
    s += glass_front(tx0 + 0.3, base, tx1 - 0.3, m(17.5), 3, 6, rng, broken=0.25)
    s += rect(tx0 - 0.5, m(4.2), tx1 + 0.5, m(5.1), "tealdark", lw(0.9))
    s += vine(tx1 - 0.1, m(19.8), 1.3, rng)
    # right anchor
    s += body(22.6, 26.0, 15, 3.5, "wallB!30!stone")
    s += poly([(23.4, m(9.2)), (24.4, m(9.2)), (23.9, m(11.0))], "teal", lw(0.9))
    # shrubs along the base, overgrown
    for bx in [2.8, 3.9, 6.6, 8.6, 9.6, 13.3, 14.1, 18.2, 22.0, 25.2]:
        s += shrub(bx, base, rng.uniform(0.32, 0.5), rng)
    return s


def cone(w=0.8):
    """A traffic cone, in metres (0.7 tall), base on y=0."""
    s = rect(-0.22, 0, 0.22, 0.06, "orange", lw(w))
    s += poly([(-0.15, 0.06), (0.15, 0.06), (0.04, 0.7), (-0.04, 0.7)], "orange", lw(w))
    s += poly([(-0.1, 0.3), (0.1, 0.3), (0.08, 0.42), (-0.08, 0.42)], "cream", lw(w * 0.7))
    return s


def can(x, y, r, fill, rot_=0):
    b = rrect(-r, -r * 1.6, r, r * 1.6, fill, r * 0.4, lw(0.6))
    b += line((-r, r * 1.1), (r, r * 1.1), lw=0.5)
    return place(x, y, 1, b, rot=rot_)


def litter(rng, gy, gs):
    """Ground clutter in the open lot in front of the walkers: oil stains,
    a storm drain, a faded lane arrow, cans, a dropped cone, papers."""
    s = ""
    for (x, y, rx) in [(13.1, 1.4, 1.1), (15.4, 3.6, 0.7), (11.0, 4.6, 0.5), (17.0, 4.4, 0.6)]:
        s += fillo(ellipse_path(x, y, rx, rx * 0.22), "asphaltdark", 0.45)
    # a faded painted lane arrow, flattened by perspective
    ar = [(12.6, 2.5), (13.2, 2.5), (13.3, 3.3), (13.75, 3.3), (13.05, 3.9), (12.25, 3.3), (12.7, 3.3)]
    s += f"\\fill[roadline!70!asphalt, opacity=0.4] {polyp(ar)};\n"
    # lane dashes
    for k in range(5):
        za, zb = 7.5 + k * 3.2, 8.7 + k * 3.2
        s += f"\\fill[roadline!70!asphalt, opacity=0.55] {polyp([(9.8 + (za - 7) * 0.02, gy(za)), (10.2, gy(za)), (10.2 + 0.0, gy(zb)), (9.95, gy(zb))])};\n"
    # a storm drain
    s += poly([(14.6, 0.55), (16.2, 0.55), (16.0, 0.95), (14.75, 0.95)], "metaldark")
    for k in range(1, 6):
        s += line((14.6 + k * 0.27, 0.58), (14.65 + k * 0.24, 0.92), lw=0.7)
    s += place(12.0, 0.4, 1.2, cone(), rot=80)
    s += place(16.9, 2.75, 1.0, cone())
    s += can(11.3, 1.55, 0.09, "red", 70)
    s += can(13.9, 2.1, 0.08, "teal", 15)
    s += can(17.8, 3.6, 0.07, "gold", 95)
    for (x, y, r_) in [(12.9, 0.85, 12), (15.3, 2.35, -20), (10.6, 2.0, 30)]:
        s += place(x, y, 1, poly([(-0.25, -0.12), (0.22, -0.16), (0.27, 0.1), (-0.2, 0.14)], "paper", lw(0.6)), rot=r_)
    return s


def orbit(cx, cy, rx, ry, a0, a1):
    return f"\\draw[ink, line width=0.6pt, dash pattern=on 2pt off 3pt, opacity=0.6] {P(cx + rx * math.cos(math.radians(a0)), cy + ry * math.sin(math.radians(a0)))} arc[start angle={f(a0)}, end angle={f(a1)}, x radius={f(rx)}, y radius={f(ry)}];\n"


def title():
    """Key art: the raiders' van has pulled up in the Southpoint lot, rear
    doors open on a cluttered interior; one raider steps down and three
    walk away across the cracked, weedy lot toward the mall, where drones
    circle the roof and a strider stands on the anchor's parapet. The lower
    left (x < CALM_X, y < CALM_Y) stays plain sky and asphalt."""
    W, H = 25.6, 14.4
    CALM_X, CALM_Y = 9.0, 5.0
    rng = random.Random(4127)
    HZ, HCAM = 6.3, 1.6
    K = HZ * 4.0
    VX = 13.0

    def gy(Z):
        return HZ - K / Z

    def gs(Z):
        return K / (Z * HCAM)

    def gp(X, Z):
        return (VX + X * gs(Z), gy(Z))

    s = sky(W, H)
    for (cx, cy, cs) in [(2.6, 12.9, 1.05), (10.0, 13.5, 0.75), (17.6, 12.7, 0.95), (23.8, 13.4, 0.8), (6.5, 10.6, 0.55)]:
        s += cloud(cx, cy, cs)
    mb = gy(70)
    msc = gs(70)
    # trees beyond the mall
    s += hill([(-0.5, mb), (1.0, mb + 1.1), (2.5, mb + 1.5), (4.0, mb + 0.9), (6.0, mb + 0.6)], mb - 0.1, "greenD")
    s += hill([(17.0, mb), (19.0, mb + 2.9), (22.0, mb + 3.3), (26.0, mb + 2.6)], mb - 0.1, "greenE")
    s += hill([(-0.5, mb), (0.5, mb + 0.6), (2.0, mb + 0.8), (3.0, mb + 0.4)], mb - 0.1, "greenB")
    s += mall(rng, mb, msc)
    # drones circling over the roof
    s += orbit(13.5, 11.2, 3.6, 0.8, 0, 360)
    for a, sc in ((200, 0.36), (250, 0.4), (320, 0.42), (30, 0.34), (120, 0.3)):
        px, py = 13.5 + 3.6 * math.cos(math.radians(a)), 11.2 + 0.8 * math.sin(math.radians(a))
        s += drone("wasp", px, py, sc, rot=rng.uniform(-10, 10))
    s += drone("warden", 17.6, 12.6, 0.5, rot=-6)
    s += drone("moth", 8.6, 11.9, 0.42, rot=8)
    s += drone("hornet", 21.4, 11.6, 0.42, flip=True)
    s += drone("strider", 5.4, mb + 16.6 * msc + 1.05 * 0.5, 0.5)
    # the lot
    s += rect(-0.5, -0.5, W + 0.5, mb, "asphalt")
    s += line((-0.5, mb), (W + 0.5, mb), lw=artlib.CUR_INK)
    s += fillo(polyp([(-0.5, gy(30)), (W + 0.5, gy(30)), (W + 0.5, gy(47)), (-0.5, gy(47))]), "asphaltdark", 0.25)
    s += f"\\fill[asphalt!85!paper] {P(-0.5, gy(50))} rectangle {P(W + 0.5, mb)};\n"
    s += line((-0.5, gy(50)), (W + 0.5, gy(50)), lw=0.7)
    for k in range(46):  # weeds along the walk, the lot going wild
        wx = k * 0.56 + rng.uniform(-0.2, 0.2)
        s += blades(wx, gy(55) + rng.uniform(-0.05, 0.05), rng.uniform(0.3, 0.42), rng.choice(["greenB", "greenA", "greenC"]))
    base, s = s, ""  # from here on, everything is clipped out of the calm corner

    def stall_row(z0, z1, xa, xb, rng):
        o = ""
        zm = (z0 + z1) / 2
        X = xa
        while X <= xb:
            for za, zb in ((z0, zm), (zm, z1)):
                if rng.random() < 0.82:
                    a, b = gp(X, za), gp(X, zb)
                    o += f"\\draw[roadline!70!asphalt, opacity={f(rng.uniform(0.6, 1.0))}, line width={f(max(0.6, 2.4 * gs(za) * 0.7))}pt, line cap=round] {P(*a)} -- {P(*b)};\n"
            X += 2.7
        a, b = gp(xa, zm), gp(xb, zm)
        o += f"\\draw[roadline!70!asphalt, opacity=0.6, line width=0.9pt] {P(*a)} -- {P(*b)};\n"
        return o

    lot = ""
    # far row
    lot += stall_row(34, 46, -60, 60, rng)
    for X in (-27, -18.9, -10.8, 5.4, 13.5, 21.6, 35):
        if rng.random() < 0.8:
            cx_, cy_ = gp(X + 1.35, 34.5)
            lot += place(cx_, cy_, gs(34.5), car_back(rng.choice(["wallB", "sage", "cream", "teal", "wallF", "metallight"]), 0.6))
    for X in (-24, 2, 26):
        px, py = gp(X, 40)
        lot += light_pole(px, py, 10 * gs(40), 0.7)
    lot += place(*gp(-14, 28), gs(28), car_side("wallC", 0.8, flat=True), rot=-4)
    lot += place(*gp(-23, 26), gs(26), cart(0.7))
    s += lot
    # near row, whose left end is a kerbed island with a tree and a pole
    near = stall_row(14, 26, -2.4, 9, rng)
    il = [gp(-3.4, 14), gp(-2.6, 14), gp(-2.6, 26), gp(-3.4, 26)]
    near += poly([(il[0][0], il[0][1] - 0.18), (il[1][0], il[1][1] - 0.18), il[1], il[0]], "kerb", lw(0.8))
    near += poly(il, "greenC", lw(0.9))
    for zz in (15, 18, 22):
        near += weeds(rng, *gp(-3.0, zz), 0.45)
    near += tree(*gp(-3.0, 23), 0.75, rng, "greenB")
    px, py = gp(-3.0, 19.5)
    near += light_pole(px, py, 8.5 * gs(19.5), 1.0)
    for X, col in ((0.3, "wallA"), (3.0, "sagelight"), (8.4, "gold!60!stone")):
        cx_, cy_ = gp(X + 1.35, 14.5)
        near += place(cx_, cy_, gs(14.5), car_back(col, 0.9))
    near += vine(gp(1.65, 14.5)[0] - 0.4, gp(1.65, 14.5)[1] + 0.9, 0.6, rng)
    s += near
    # cracks and weeds through them
    cr = ""
    for (cx_, cy_, ang, n) in [(10.5, 1.0, 20, 9), (12.0, 3.0, 160, 6), (15.0, 0.3, 80, 5), (9.6, 4.9, 10, 8), (3.0, 5.4, -10, 7), (19.0, 5.3, 190, 5), (14.5, 4.4, 30, 5)]:
        c, cp = crack(rng, cx_, cy_, n, rng.uniform(0.35, 0.55), ang)
        cr += c
        for p in cp[1::3]:
            cr += weeds(rng, p[0], p[1], 0.5 * (1.6 - p[1] / HZ))
    for (wx, wy, ws) in [(16.8, 3.2, 0.5), (12.4, 0.6, 0.65), (9.8, 2.2, 0.5), (14.2, 2.0, 0.55), (21.5, 4.8, 0.3), (6.0, 5.6, 0.3), (1.2, 5.75, 0.25)]:
        cr += weeds(rng, wx, wy, ws)
    s += cr
    # a tipped cart
    s += fillo(ellipse_path(10.5, 3.15, 0.75, 0.08), "ink", 0.2)
    s += place(10.5, 3.15, gs(9), cart(1.0), rot=-8)
    s += litter(rng, gy, gs)

    # the raiders walking away (far to near), and the van
    def walker(Z, x, **kw):
        y = gy(Z)
        sc = 0.522 * gs(Z)
        o = fillo(ellipse_path(x, y + 0.02, 0.62 * sc, 0.1 * sc), "ink", 0.22)
        return o + place(x, y, sc, raider(**kw))

    s += walker(11.5, 11.9, jacket="jacketc", pants="wooddark", helmet="sagedark", pack="wallE", kit=2, step=0, skin="skinc")
    s += walker(8.6, 13.4, jacket="jacketb", pants="sagedark", helmet="armourlight", pack="wood", gun="ak", kit=1, skin="skinb")
    s += walker(6.4, 15.5, jacket="jacket", pants="asphaltdark", helmet="armour", pack="sagedark", gun="quad", kit=3)
    s += van(18.1, 1.6, 6.4, 5.6)
    s += place(21.2, 1.13, 1.45, raider("jacketd", pants="ink!60!jacketb", helmet="wallE", front=True, kit=1, step=0, skin="skin"))
    for (wx, ws) in ((17.6, 0.5), (24.9, 0.45)):
        s += weeds(rng, wx, 0.65, ws)
    assert CALM_X >= W / 3 and CALM_Y >= H / 3, "the calm corner covers the lower-left third"
    return base + calm_clip(W, H, CALM_X, CALM_Y, s), (W, H)



# --------------------------------------------------------------------------
# interiors: shared pieces for the mission postcards
# --------------------------------------------------------------------------

MW, MH = 19.2, 10.8
MCALM = MW / 3  # nothing but plain wall and floor left of this


def right_of(x0, body, H=MH, W=MW):
    """Clip body to x > x0: the postcards' left third stays calm."""
    return clipped(f"{P(x0, -1)} rectangle {P(W + 1, H + 1)}", body)


def floor_tiles(W, fy, vx, vy, colour, n=12, rows=6, op=0.35, w=0.6):
    """Faint tile seams on a floor (0..fy) converging on (vx, vy): light
    lines, not shapes, so a calm floor stays calm."""
    s = ""
    for i in range(-n, 2 * n + 1):
        x0 = i * W / n
        t = fy / vy
        s += f"\\draw[{colour}, opacity={f(op)}, line width={f(w)}pt] {P(x0, 0)} -- {P(x0 + (vx - x0) * t, fy)};\n"
    y = 0.0
    step = fy * 0.34
    for _ in range(rows):
        y += step
        step *= 0.66
        if y < fy:
            s += f"\\draw[{colour}, opacity={f(op)}, line width={f(w)}pt] {P(-1, y)} -- {P(W + 1, y)};\n"
    return s


def tee(fill, mark="gold", kind=0, hanger=True):
    """A t-shirt on a hanger, front view, about 1.5 wide, the hook at
    (0, 0.95). Its print is a shape (circle, star, bolt, triangle, eyes),
    never a word or a band name."""
    s = ""
    if hanger:
        s += curve([(0, 0.62), (0, 0.85), (0.1, 0.95), (0.18, 0.86)], lw=1.0, colour="metaldark")
        s += line((-0.45, 0.5), (0, 0.66), lw=1.1, colour="metaldark")
        s += line((0.45, 0.5), (0, 0.66), lw=1.1, colour="metaldark")
    body = [(-0.42, -0.72), (0.42, -0.72), (0.43, 0.22), (0.62, 0.1), (0.76, 0.36), (0.42, 0.6), (0.16, 0.62), (0, 0.52), (-0.16, 0.62), (-0.42, 0.6), (-0.76, 0.36), (-0.62, 0.1), (-0.43, 0.22)]
    s += poly(body, fill, lw(0.9))
    s += curve([(-0.16, 0.62), (0, 0.48), (0.16, 0.62)], lw=0.7)
    cy = -0.05
    if kind == 0:
        s += circle(0, cy, 0.24, mark, lw(0.7))
        s += circle(0, cy, 0.1, fill, lw(0.6))
    elif kind == 1:
        st = [(math.cos(math.radians(90 + k * 36)) * (0.27 if k % 2 == 0 else 0.11), cy + math.sin(math.radians(90 + k * 36)) * (0.27 if k % 2 == 0 else 0.11)) for k in range(10)]
        s += poly(st, mark, lw(0.6))
    elif kind == 2:
        s += poly([(0.05, cy + 0.3), (-0.17, cy - 0.02), (0.0, cy - 0.02), (-0.08, cy - 0.3), (0.18, cy + 0.06), (0.02, cy + 0.06)], mark, lw(0.6))
    elif kind == 3:
        s += poly([(-0.25, cy - 0.2), (0.25, cy - 0.2), (0, cy + 0.25)], mark, lw(0.6))
    else:
        s += ellipse(0, cy, 0.24, 0.2, mark, extra=lw(0.6))
        s += circle(-0.09, cy + 0.02, 0.06, fill, lw(0.4))
        s += circle(0.09, cy + 0.02, 0.06, fill, lw(0.4))
    return s


def chain(a, b, sag, n=12, colour="metallight"):
    """A hanging chain: oval links along a sagging curve."""
    s = ""
    for k in range(n + 1):
        t = k / n
        x = a[0] + (b[0] - a[0]) * t
        y = a[1] + (b[1] - a[1]) * t - sag * 4 * t * (1 - t)
        s += ellipse(x, y, 0.09, 0.05, colour, rot=30 if k % 2 else -30, extra=lw(0.6))
    return s


def poster(x0, y0, x1, y1, bg, rng, torn=True):
    """A poster as plain colour shapes: a field, a big disc or band, a
    stripe; a torn corner. Never words."""
    s = rect(x0, y0, x1, y1, bg, lw(0.9))
    w, h = x1 - x0, y1 - y0
    kind = rng.randint(0, 2)
    c1 = rng.choice(["gold", "pink", "glowhi", "cream", "redlight"])
    if kind == 0:
        s += circle(x0 + w / 2, y0 + h * 0.58, min(w, h) * 0.28, c1, lw(0.7))
    elif kind == 1:
        s += poly([(x0 + w * 0.15, y0 + h * 0.3), (x0 + w * 0.85, y0 + h * 0.3), (x0 + w / 2, y0 + h * 0.85)], c1, lw(0.7))
    else:
        s += poly([(x0 + w * 0.55, y0 + h * 0.9), (x0 + w * 0.25, y0 + h * 0.5), (x0 + w * 0.5, y0 + h * 0.5), (x0 + w * 0.4, y0 + h * 0.15), (x0 + w * 0.78, y0 + h * 0.6), (x0 + w * 0.55, y0 + h * 0.6)], c1, lw(0.7))
    s += f"\\fill[ink, opacity=0.3] {P(x0 + w * 0.12, y0 + h * 0.08)} rectangle {P(x1 - w * 0.12, y0 + h * 0.16)};\n"
    if torn:
        s += poly([(x1, y1), (x1 - w * 0.25, y1), (x1 - w * 0.05, y1 - h * 0.12), (x1, y1 - h * 0.2)], "ink!85!black", lw(0.7))
        s += poly([(x1 - w * 0.25, y1), (x1 - w * 0.05, y1 - h * 0.12), (x1 - w * 0.18, y1 - h * 0.2)], f"{bg}!70!paper", lw(0.7))
    return s


def shards(rng, x0, x1, y0, y1, n, fill="glowhi"):
    s = ""
    for _ in range(n):
        x, y = rng.uniform(x0, x1), rng.uniform(y0, y1)
        r = rng.uniform(0.06, 0.16)
        a = rng.uniform(0, 360)
        s += poly([rot((x + r, y), a, (x, y)), rot((x - r * 0.6, y + r * 0.5), a, (x, y)), rot((x - r * 0.4, y - r * 0.6), a, (x, y))], fill, lw(0.5))
    return s


def spotlight(x, y, x0, x1, yb, colour="window", op=0.14):
    """A cone of light from (x, y) down to the floor span x0..x1 at yb."""
    return fillo(polyp([(x - 0.15, y), (x + 0.15, y), (x1, yb), (x0, yb)]), colour, op) + fillo(polyp([(x - 0.08, y), (x + 0.08, y), ((x + x1) / 2 + 0.1, yb), ((x + x0) / 2 - 0.1, yb)]), colour, op)


# --------------------------------------------------------------------------
# missions/cudi
# --------------------------------------------------------------------------

def cudi():
    """Inside a dark alt-fashion store: black walls, a wall of face-out
    band tees, a rack of tees, hanging chains, posters as plain coloured
    shapes; the one rare vintage tee glows in a display case under a
    spotlight; a wasp drone hovers in the bright doorway. Left third: plain
    black wall and floor."""
    W, H = MW, MH
    rng = random.Random(2211)
    fy = 3.1
    s = rect(-0.5, -0.5, W + 0.5, H + 0.5, "ink!82!black")
    s += rect(-0.5, -0.5, W + 0.5, fy, "ink!72!asphaltdark")
    s += line((-0.5, fy), (W + 0.5, fy), lw=1.2, colour="black")
    s += floor_tiles(W, fy, 12.5, 6.5, "black", n=10, op=0.35)
    s += rect(-0.5, H - 0.7, W + 0.5, H + 0.5, "ink!90!black")
    d = ""
    # the doorway out to the lit mall concourse, gate half up
    dx0, dx1, dy1 = 15.9, 18.7, 8.0
    d += rect(dx0, fy, dx1, dy1, "skyhi")
    d += poly([(dx0, fy), (dx1, fy), (dx1, fy + 1.5), (dx0, fy + 1.5)], "stone", lw(0.8))
    d += rect(dx0, fy + 1.5, dx1, fy + 3.6, "wallB", lw(0.8))
    d += rect(dx0 + 0.4, fy + 1.5, dx0 + 1.3, fy + 3.0, "window", lw(0.7))
    d += rect(dx0 + 1.6, fy + 1.5, dx1 - 0.2, fy + 3.2, "teal!50!stone", lw(0.7))
    d += fillo(polyp([(dx0, fy), (dx1, fy), (dx1 - 1.0, 0.0), (dx0 - 2.2, 0.0)]), "skyhi", 0.18)
    d += rect(dx0 - 0.25, fy, dx0, dy1 + 0.3, "ink!70!metaldark")
    d += rect(dx1, fy, dx1 + 0.25, dy1 + 0.3, "ink!70!metaldark")
    d += rect(dx0 - 0.25, dy1, dx1 + 0.25, dy1 + 1.1, "metaldark")
    for k in range(7):
        d += line((dx0 - 0.25, dy1 + 0.12 + k * 0.14), (dx1 + 0.25, dy1 + 0.12 + k * 0.14), lw=0.6)
    d += rect(dx0 - 0.25, dy1 - 0.15, dx1 + 0.25, dy1 + 0.05, "metal", lw(0.8))
    d += line((dx0 + 0.6, dy1 - 0.15), (dx0 + 0.6, dy1 - 0.45), lw=1.4)
    d += halo(17.3, 5.6, 0.35, "red", 3, 0.08)
    d += drone("wasp", 17.3, 5.6, 0.95, rot=-6)
    # neon strip along the top of the wall
    d += f"\\fill[magenta, opacity=0.18] {P(6.8, H - 1.35)} rectangle {P(15.6, H - 0.75)};\n"
    d += rrect(6.8, H - 1.12, 15.6, H - 0.98, "magenta!40!white", 0.05, lw(0.8))
    # posters on the black wall
    for (x0, y0, w, h, bg) in [(7.0, 6.4, 1.3, 1.8, "violet"), (8.5, 7.2, 1.0, 1.4, "redlight"), (14.0, 6.1, 1.3, 1.8, "teal"), (14.3, 8.1, 1.0, 1.1, "gold")]:
        d += poster(x0, y0, x0 + w, y0 + h, bg, rng)
    # wall of face-out tees on a slatwall grid
    d += rect(9.8, 5.6, 13.6, 9.4, "ink!75!wooddark", lw(0.9))
    for k in range(1, 12):
        d += line((9.8, 5.6 + k * 0.32), (13.6, 5.6 + k * 0.32), lw=0.5, colour="black")
    for i, (tx, ty) in enumerate([(10.45, 8.4), (11.7, 8.4), (12.95, 8.4), (10.45, 6.55), (11.7, 6.55), (12.95, 6.55)]):
        d += place(tx, ty, 0.62, tee(rng.choice(["ink!90!black", "ink!70!black", "violet!40!black", "black"]), rng.choice(["glowhi", "pink", "gold", "cream", "redlight"]), i % 5))
    # wallet chains on hooks
    for k in range(4):
        hx = 7.1 + k * 0.45
        d += line((hx, 5.8), (hx, 5.55), lw=1.0, colour="metal")
        d += chain((hx, 5.55), (hx + 0.2, 4.4 - k * 0.15), 0.05, n=7)
    # chains swagged from the ceiling
    d += chain((6.8, H - 0.7), (10.5, H - 0.7), 0.7, n=24)
    d += chain((12.2, H - 0.7), (15.7, H - 0.7), 0.55, n=22)
    # a rack of tees (side-hung), in front of the wall
    d += line((7.0, 1.4), (7.0, 4.6), lw=2.0, colour="metal")
    d += line((10.3, 1.4), (10.3, 4.6), lw=2.0, colour="metal")
    d += rect(6.8, 1.25, 7.2, 1.4, "metaldark", lw(0.6))
    d += rect(10.1, 1.25, 10.5, 1.4, "metaldark", lw(0.6))
    d += line((6.9, 4.6), (10.4, 4.6), lw=2.4, colour="metallight")
    xs = 7.25
    while xs < 10.1:
        c = rng.choice(["ink!90!black", "ink!75!black", "violet!35!black", "wallC!35!black", "black"])
        d += curve([(xs, 4.6), (xs + 0.06, 4.75), (xs + 0.13, 4.65)], lw=0.8, colour="metal")
        d += poly([(xs - 0.12, 4.5), (xs + 0.12, 4.5), (xs + 0.16, 2.3 + rng.uniform(-0.1, 0.1)), (xs - 0.16, 2.3)], c, lw(0.8))
        xs += rng.uniform(0.22, 0.3)
    # the display case under its spotlight
    cx = 12.8
    d += spotlight(cx, H - 0.7, cx - 2.3, cx + 2.3, 1.1, "window", 0.1)
    d += poly([(cx - 0.35, H - 0.7), (cx + 0.35, H - 0.7), (cx + 0.25, H - 1.15), (cx - 0.25, H - 1.15)], "metaldark", lw(0.8))
    d += fillo(ellipse_path(cx, 1.15, 1.9, 0.25), "black", 0.5)
    d += rect(cx - 1.25, 1.15, cx + 1.25, 3.6, "ink!80!black")
    d += rect(cx - 1.35, 3.6, cx + 1.35, 3.85, "metal", lw(0.9))
    d += rect(cx - 0.9, 1.6, cx + 0.9, 3.2, "ink!70!wooddark", lw(0.7))
    d += halo(cx, 5.4, 1.2, "glow", 4, 0.07)
    d += f"\\filldraw[draw=ink, line width=1.1pt, fill=glow, fill opacity=0.28] {P(cx - 1.2, 3.85)} rectangle {P(cx + 1.2, 7.05)};\n"
    d += line((cx - 0.9, 6.55), (cx + 0.9, 6.55), lw=1.6, colour="metallight")
    d += place(cx, 5.4, 1.0, tee("wallC!70!cream", "cream", 2))
    d += f"\\fill[white, opacity=0.35] {polyp([(cx - 1.05, 4.0), (cx - 0.8, 4.0), (cx - 0.35, 6.9), (cx - 0.6, 6.9)])};\n"
    d += f"\\fill[white, opacity=0.25] {polyp([(cx + 0.6, 4.0), (cx + 0.72, 4.0), (cx + 1.05, 6.0), (cx + 0.93, 6.0)])};\n"
    d += rect(cx - 1.3, 7.05, cx + 1.3, 7.3, "metal", lw(0.9))
    for k in range(5):  # little glow LEDs along the lid
        d += circle(cx - 1.0 + k * 0.5, 7.17, 0.05, "glowhi", lw(0.4))
    # velvet rope on stanchions in front
    for sx in (cx - 2.0, cx + 2.0):
        d += rect(sx - 0.25, 0.55, sx + 0.25, 0.68, "gold", lw(0.7))
        d += rect(sx - 0.05, 0.68, sx + 0.05, 2.1, "gold", lw(0.7))
        d += circle(sx, 2.15, 0.1, "gold", lw(0.7))
    d += curve([(cx - 2.0, 2.0), (cx, 1.35), (cx + 2.0, 2.0)], lw=3.2, colour="ink")
    d += curve([(cx - 2.0, 2.0), (cx, 1.35), (cx + 2.0, 2.0)], lw=2.0, colour="red")
    # floor clutter: a fallen tee, hangers, glass bits, a boot
    d += place(10.7, 0.5, 0.6, tee("violet!40!black", "pink", 3, hanger=False), rot=78)
    d += place(15.6, 1.2, 0.55, tee("black", "gold", 1, hanger=False), rot=-100)
    for (hx, hy, r_) in [(9.8, 0.6, 15), (15.2, 2.4, -30), (17.0, 0.7, 70)]:
        d += place(hx, hy, 0.5, line((-0.45, 0), (0.45, 0), lw=1.0, colour="metal") + line((-0.45, 0), (0, 0.18), lw=1.0, colour="metal") + line((0.45, 0), (0, 0.18), lw=1.0, colour="metal"), rot=r_)
    d += shards(rng, 14.3, 17.8, 0.3, 2.6, 14, "glowhi")
    # a bust form in a studded collar, on its stand
    bx = 15.0
    d += rect(bx - 0.3, 1.0, bx + 0.3, 1.12, "metaldark", lw(0.7))
    d += rect(bx - 0.04, 1.12, bx + 0.04, 3.4, "metal", lw(0.7))
    d += blob([(bx - 0.55, 3.4), (bx + 0.55, 3.4), (bx + 0.62, 4.4), (bx + 0.35, 4.9), (bx - 0.35, 4.9), (bx - 0.62, 4.4)], "stone", tension=0.5)
    d += rect(bx - 0.12, 4.9, bx + 0.12, 5.15, "stone", lw(0.8))
    d += rrect(bx - 0.2, 5.1, bx + 0.2, 5.3, "ink!90!black", 0.06, lw(0.8))
    for k in range(5):
        d += circle(bx - 0.16 + k * 0.08, 5.2, 0.03, "metallight", lw(0.3))
    d += poly([(bx - 0.62, 4.4), (bx - 0.35, 4.9), (bx - 0.05, 4.9), (bx - 0.2, 3.8), (bx - 0.58, 3.6)], "violet!40!black", lw(0.8))
    # a crate of records in front of the rack
    d += place(8.3, 0.35, 1.0, "".join(rect(-0.75 + k * 0.17, 0.75, -0.75 + k * 0.17 + 0.5, 1.4 - (k % 3) * 0.08, c, lw(0.6)) for k, c in enumerate(["pink", "gold", "teal", "redlight", "cream", "violet", "glowhi"])) + rect(-0.85, 0, 0.85, 1.0, "wood") + line((-0.85, 0.5), (0.85, 0.5), lw=0.7) + rect(-0.25, 0.62, 0.25, 0.82, "ink!70!wooddark", lw(0.6)))
    # a stool knocked over by the door
    d += place(17.9, 0.9, 1.0, rect(-0.5, -0.1, 0.5, 0.12, "magenta!50!black", lw(0.8)) + line((-0.4, 0.12), (-0.6, 1.0), lw=1.6, colour="metal") + line((0.4, 0.12), (0.6, 1.0), lw=1.6, colour="metal") + line((-0.5, 0.6), (0.5, 0.6), lw=1.0, colour="metal"), rot=95)
    s += right_of(MCALM, d)
    return s, (W, H)



# --------------------------------------------------------------------------
# missions/foodcourt
# --------------------------------------------------------------------------

def table(top="cream", w=0.9):
    """A round food-court table, side view, about 1.6 wide; foot on y=0."""
    s = rect(-0.45, 0, 0.45, 0.1, "metaldark", lw(w))
    s += rect(-0.07, 0.1, 0.07, 1.15, "metaldark", lw(w))
    s += ellipse(0, 1.2, 0.85, 0.18, top, extra=lw(w))
    s += f"\\fill[ink, opacity=0.2] (-0.8,1.17) -- (0.8,1.17) -- (0.75,1.1) -- (-0.75,1.1) -- cycle;\n"
    return s


def chair(fill="teal", w=0.9):
    """A food-court chair, side view facing +x; feet on y=0."""
    s = line((-0.3, 0), (-0.25, 0.65), lw=1.4)
    s += line((0.3, 0), (0.25, 0.65), lw=1.4)
    s += rrect(-0.35, 0.62, 0.35, 0.76, fill, 0.05, lw(w))
    s += rrect(-0.38, 0.72, -0.24, 1.45, fill, 0.05, lw(w))
    return s


def tray_spill(x, y, s_, rng):
    o = place(x, y, s_, poly([(-0.6, 0), (0.6, 0), (0.5, 0.12), (-0.5, 0.12)], "wallC", lw(0.8)), rot=rng.uniform(-8, 8))
    o += place(x + 0.5 * s_, y + 0.12 * s_, s_, poly([(-0.14, 0), (0.14, 0), (0.18, 0.45), (-0.18, 0.45)], "cream", lw(0.7)) + rect(-0.2, 0.45, 0.2, 0.52, "red", lw(0.6)) + line((0.05, 0.52), (0.12, 0.75), lw=1.0, colour="red"), rot=rng.choice([0, 0, 80]))
    for _ in range(5):
        fx = x + rng.uniform(-0.7, 0.9) * s_
        o += place(fx, y + rng.uniform(-0.12, 0.05), s_, rect(-0.12, -0.025, 0.12, 0.025, "goldlight", lw(0.4)), rot=rng.uniform(0, 180))
    return o


def stall(rng, x0, x1, y0, frame, board, logo, shutter=False):
    """A fast-food stall: sign board with a logo shape, menu boards as
    blocks of colour, a dark kitchen with a fryer and a soda machine, a
    counter with a register and a stack of cups."""
    s = rect(x0, y0, x1, y0 + 4.9, frame)
    ox0, ox1, oy0, oy1 = x0 + 0.3, x1 - 0.3, y0 + 1.25, y0 + 3.5
    s += rect(ox0, oy0, ox1, oy1, "ink!65!wooddark")
    # kitchen: back shelf, a fryer, a soda machine
    s += rect(ox0 + 0.2, oy0 + 1.3, ox1 - 0.2, oy0 + 1.4, "metal", lw(0.6))
    for k in range(4):
        s += rect(ox0 + 0.3 + k * 0.4, oy0 + 1.4, ox0 + 0.6 + k * 0.4, oy0 + 1.7, rng.choice(["red", "gold", "cream", "teal"]), lw(0.5))
    s += rect(ox1 - 1.3, oy0, ox1 - 0.4, oy0 + 1.1, "metallight", lw(0.7))
    for k in range(3):
        s += rect(ox1 - 1.2 + k * 0.28, oy0 + 0.75, ox1 - 1.0 + k * 0.28, oy0 + 1.0, rng.choice(["red", "orange", "glowhi"]), lw(0.4))
    s += rect(ox0 + 0.2, oy0, ox0 + 1.1, oy0 + 0.8, "metal", lw(0.7))
    s += rect(ox0 + 0.3, oy0 + 0.8, ox0 + 1.0, oy0 + 0.9, "metaldark", lw(0.6))
    # menu boards: blocks of colour
    mw = (ox1 - ox0 - 0.4) / 3
    for k in range(3):
        mx = ox0 + 0.2 + k * mw
        s += rect(mx + 0.05, oy1 - 0.75, mx + mw - 0.05, oy1 - 0.08, "ink!80!black", lw(0.7))
        for j in range(3):
            s += f"\\fill[{rng.choice(['gold', 'redlight', 'cream', 'sagelight', 'pink'])}] {P(mx + 0.15, oy1 - 0.25 - j * 0.17)} rectangle {P(mx + 0.15 + mw * rng.uniform(0.3, 0.6), oy1 - 0.18 - j * 0.17)};\n"
    if shutter:
        sy = oy0 + 1.6
        s += rect(ox0, sy, ox1, oy1, "metal", lw(0.8))
        for k in range(1, 9):
            yy = sy + k * (oy1 - sy) / 9
            s += line((ox0, yy), (ox1, yy), lw=0.5)
        s += rect((ox0 + ox1) / 2 - 0.2, sy - 0.08, (ox0 + ox1) / 2 + 0.2, sy + 0.06, "metaldark", lw(0.6))
    # counter
    s += rect(x0 + 0.1, y0, x1 - 0.1, oy0, f"{frame}!80!ink", lw(0.9))
    s += rect(x0 + 0.1, y0 + 0.55, x1 - 0.1, y0 + 0.75, board, lw(0.7))
    s += rect(x0, oy0, x1, oy0 + 0.15, "metallight", lw(0.9))
    s += rect(ox0 + 0.4, oy0 + 0.15, ox0 + 0.95, oy0 + 0.55, "metaldark", lw(0.7))
    s += poly([(ox0 + 0.45, oy0 + 0.55), (ox0 + 0.9, oy0 + 0.55), (ox0 + 0.85, oy0 + 0.75), (ox0 + 0.5, oy0 + 0.75)], "ink!60!metaldark", lw(0.6))
    for k in range(3):
        s += poly([(ox1 - 1.6 + k * 0.03, oy0 + 0.15 + k * 0.18), (ox1 - 1.3 - k * 0.03, oy0 + 0.15 + k * 0.18), (ox1 - 1.28 - k * 0.03, oy0 + 0.33 + k * 0.18), (ox1 - 1.62 + k * 0.03, oy0 + 0.33 + k * 0.18)], "cream", lw(0.5))
    # sign board with its logo shape
    by0, by1 = oy1 + 0.25, y0 + 4.75
    s += rect(x0 + 0.1, by0, x1 - 0.1, by1, board)
    cx, cy = (x0 + x1) / 2, (by0 + by1) / 2
    if logo == "disc":
        # a burger: bun, leaf, patty, bun
        s += rrect(cx - 0.55, cy - 0.42, cx + 0.55, cy - 0.22, "woodlight", 0.08, lw(0.8))
        s += rrect(cx - 0.6, cy - 0.24, cx + 0.6, cy - 0.04, "wooddark", 0.08, lw(0.8))
        s += poly([(cx - 0.62, cy - 0.02), (cx - 0.4, cy + 0.06), (cx - 0.2, cy - 0.02), (cx, cy + 0.06), (cx + 0.2, cy - 0.02), (cx + 0.4, cy + 0.06), (cx + 0.62, cy - 0.02), (cx + 0.6, cy - 0.1), (cx - 0.6, cy - 0.1)], "greenA", lw(0.7))
        s += f"\\filldraw[inked,fill=woodlight] {P(cx - 0.58, cy + 0.04)} arc[start angle=180, end angle=0, x radius=0.58, y radius=0.4] -- cycle;\n"
        for (dx, dy) in ((-0.2, 0.2), (0.05, 0.3), (0.25, 0.16)):
            s += ellipse(cx + dx, cy + dy, 0.04, 0.025, "cream", extra=lw(0.4))
    elif logo == "leaf":
        s += leaf((cx - 0.5, cy - 0.3), (cx + 0.55, cy + 0.35), 0.35, "greenA")
        s += curve([(cx - 0.45, cy - 0.25), (cx + 0.45, cy + 0.28)], lw=0.8)
    else:
        s += poly([(cx - 0.55, cy + 0.35), (cx + 0.55, cy + 0.35), (cx, cy - 0.4)], "orange")
        s += ellipse(cx, cy + 0.35, 0.55, 0.12, "cream", extra=lw(0.8))
    for side in (x0 + 0.35, x1 - 0.35):
        s += rect(side - 0.12, cy - 0.12, side + 0.12, cy + 0.12, "cream", lw(0.6))
    return s


def foodcourt():
    """An abandoned food court: three fast-food stalls with sign boards of
    shapes, a skylight throwing beams of light, tables and chairs knocked
    over, spilled trays, an overgrown planter, and wasp and moth drones
    swarming between the tables. Left third: plain wall and floor."""
    W, H = MW, MH
    rng = random.Random(3307)
    fy = 4.2
    s = rect(-0.5, -0.5, W + 0.5, H + 0.5, "stone")
    s += rect(-0.5, 9.3, W + 0.5, H + 0.5, "concrete")
    s += line((-0.5, 9.3), (W + 0.5, 9.3), lw=1.0)
    s += rect(-0.5, -0.5, W + 0.5, fy, "paper!70!kerb")
    s += line((-0.5, fy), (W + 0.5, fy), lw=1.1)
    s += floor_tiles(W, fy, 12.5, 7.5, "stonedark", n=11, op=0.5)
    d = ""
    # the skylight in the ceiling
    sk = [(9.6, 9.35), (17.8, 9.35), (18.8, H + 0.1), (8.6, H + 0.1)]
    d += poly(sk, "skyhi")
    for k in range(1, 6):
        d += line(lerp(sk[0], sk[1], k / 6), lerp(sk[3], sk[2], k / 6), lw=0.9)
    d += line(lerp(sk[0], sk[3], 0.5), lerp(sk[1], sk[2], 0.5), lw=0.9)
    d += poly([(10.0, 9.9), (10.8, 9.9), (11.0, 10.4), (10.1, 10.4)], "greenB", lw(0.7))
    d += vine(15.5, 9.4, 1.4, rng)
    d += vine(11.5, 9.4, 0.9, rng)
    # three stalls along the back wall
    d += rect(6.4, fy, W + 0.5, fy + 0.2, "stonedark", lw(0.8))
    d += stall(rng, 6.7, 10.5, fy, "wallA", "red", "disc")
    d += stall(rng, 10.8, 14.7, fy, "sage", "greenD", "leaf", shutter=True)
    d += stall(rng, 15.0, 18.9, fy, "wallD", "orange", "cone")
    # light beams from the skylight
    for (a, b, c, e) in [(10.0, 11.4, 12.6, 9.6), (12.4, 13.6, 15.2, 12.4), (15.0, 16.2, 18.4, 15.6)]:
        d += fillo(polyp([(a, 9.35), (b, 9.35), (c, 0.0), (e, 0.0)]), "white", 0.16)
        d += fillo(ellipse_path((c + e) / 2, 1.4, (c - e) / 2 + 0.3, 0.35), "white", 0.12)
    # tables and chairs, back row then front row, some knocked over
    def set_(x, y, sc, knocked):
        o = fillo(ellipse_path(x, y, 0.9 * sc, 0.12 * sc), "ink", 0.18)
        if knocked == "table":
            o += place(x, y + 0.2 * sc, sc, table("cream"), rot=88)
        else:
            o += place(x, y, sc, table(rng.choice(["cream", "wallB", "sagelight"])))
        o += place(x - 1.0 * sc, y - 0.05 * sc, sc, chair(rng.choice(["teal", "wallC", "gold"])), rot=0 if knocked != "chair" else 95)
        o += place(x + 1.0 * sc, y - 0.08 * sc, sc, chair(rng.choice(["teal", "wallC", "gold"])), flip=True, rot=0 if knocked != "chairs" else -100)
        return o
    d += set_(8.3, 3.2, 0.7, "chair")
    d += set_(12.1, 3.35, 0.7, None)
    d += set_(15.9, 3.15, 0.7, "chairs")
    d += set_(9.4, 0.8, 1.05, "table")
    d += set_(13.9, 1.0, 1.05, "chairs")
    # a tipped bin spilling cups and wrappers
    bin_ = rrect(-0.45, 0, 0.45, 1.5, "tealdark", 0.12) + rect(-0.5, 1.4, 0.5, 1.65, "teal", lw(0.9)) + rect(-0.3, 1.45, 0.3, 1.55, "ink", lw(0.5))
    d += place(7.6, 0.75, 0.85, bin_, rot=-84)
    for (tx, ty, c) in [(8.9, 0.6, "cream"), (9.0, 0.25, "redlight"), (8.6, 0.15, "paper"), (9.3, 0.5, "gold")]:
        d += place(tx, ty, 1, rrect(-0.15, -0.09, 0.15, 0.09, c, 0.04, lw(0.6)), rot=rng.uniform(-40, 40))
    d += tray_spill(11.6, 0.35, 0.8, rng)
    d += tray_spill(16.8, 2.3, 0.6, rng)
    # an overgrown planter at the right edge
    d += rect(17.3, 0.0, W + 0.5, 1.6, "wallE")
    d += rect(17.2, 1.5, W + 0.5, 1.75, "stonedark", lw(0.8))
    for k in range(14):
        a = rng.uniform(-70, 70)
        b = (17.9 + rng.uniform(-0.3, 1.2), 1.7)
        d += leaf(b, (b[0] + math.sin(math.radians(a)) * rng.uniform(1.2, 2.4), b[1] + math.cos(math.radians(a)) * rng.uniform(1.2, 2.4)), 0.22, rng.choice(["greenA", "greenB", "greenD"]))
    # the swarm
    for (x, y, sc, r_) in [(9.1, 6.0, 0.5, 8), (11.2, 7.0, 0.42, -6), (12.8, 5.5, 0.55, 10), (14.6, 7.6, 0.4, -4), (16.6, 6.2, 0.5, 6), (10.4, 2.4, 0.62, -8), (15.2, 4.6, 0.66, 12), (12.4, 2.2, 0.45, 4), (17.4, 3.7, 0.55, -10), (7.8, 4.9, 0.4, 0)]:
        d += drone("wasp", x, y, sc, rot=r_)
    d += drone("moth", 13.4, 8.6, 0.45, rot=-5)
    d += drone("moth", 8.2, 7.7, 0.38, rot=7)
    s += right_of(MCALM, d)
    return s, (W, H)



# --------------------------------------------------------------------------
# missions/drives
# --------------------------------------------------------------------------

def crack_star(x, y, r, rng, colour="glowhi"):
    s = ""
    for k in range(7):
        a = k * 360 / 7 + rng.uniform(-15, 15)
        m = (x + math.cos(math.radians(a)) * r * 0.5, y + math.sin(math.radians(a)) * r * 0.5)
        e = (x + math.cos(math.radians(a + rng.uniform(-12, 12))) * r, y + math.sin(math.radians(a + rng.uniform(-12, 12))) * r)
        s += f"\\draw[{colour}, line width=0.6pt, line join=round] {P(x, y)} -- {P(*m)} -- {P(*e)};\n"
    return s


def hard_drive(x, y, w=1.0):
    """A hard drive lying on the desk, glowing cyan: it is the objective."""
    s = halo(x, y + 0.1, 0.45, "glow", 2, 0.16)
    s += poly([(x - 0.5, y), (x + 0.5, y), (x + 0.6, y + 0.25), (x - 0.4, y + 0.25)], "armourlight", lw(w))
    s += rect(x - 0.5, y - 0.35, x + 0.5, y, "armourdark", lw(w))
    s += rect(x - 0.4, y - 0.22, x + 0.4, y - 0.12, "glow", lw(0.6))
    s += poly([(x + 0.5, y - 0.35), (x + 0.6, y - 0.1), (x + 0.6, y + 0.25), (x + 0.5, y)], "armour", lw(w))
    s += circle(x + 0.2, y + 0.12, 0.06, "glowhi", lw(0.5))
    return s


def drives():
    """The mall security office: a wall of dead CCTV monitors (some cracked,
    one still glowing), a desk with three hard drives glowing cyan, a
    console, a mug and papers; a hornet sniper drone pokes its long red
    lens through a gap in the window blinds. Left third: plain wall and
    floor."""
    W, H = MW, MH
    rng = random.Random(5150)
    fy = 3.0
    wall = "sagelight!55!stone"
    s = rect(-0.5, -0.5, W + 0.5, H + 0.5, wall)
    s += rect(-0.5, 9.9, W + 0.5, H + 0.5, "concrete")
    s += line((-0.5, 9.9), (W + 0.5, 9.9), lw=1.0)
    s += rect(-0.5, -0.5, W + 0.5, fy, "metaldark!55!asphalt")
    s += rect(-0.5, fy, W + 0.5, fy + 0.25, "sagedark", lw(0.9))
    s += floor_tiles(W, fy, 12.0, 6.0, "ink", n=9, op=0.18)
    d = ""
    # ceiling tiles and a dangling dead strip light
    for k in range(9):
        d += line((6.4 + k * 1.6, 9.9), (6.4 + k * 1.6 + 0.3, H + 0.1), lw=0.6)
    d += line((11.0, 9.9), (11.2, 9.3), lw=0.8)
    d += line((13.4, 9.9), (13.4, 9.75), lw=0.8)
    d += poly([(11.1, 9.35), (13.5, 9.8), (13.5, 9.6), (11.1, 9.15)], "cream", lw(0.9))
    # the wall of monitors
    d += rect(6.8, 4.2, 14.6, 9.6, "ink!50!metaldark")
    glowing = (2, 1)
    cracked = {(0, 2), (1, 0), (3, 1), (2, 2), (0, 0)}
    mw, mh = 1.8, 1.65
    for i in range(4):
        for j in range(3):
            x0, y0 = 7.05 + i * (mw + 0.1), 4.45 + j * (mh + 0.12)
            d += rrect(x0, y0, x0 + mw, y0 + mh, "ink!70!metaldark", 0.08, lw(0.9))
            sx0, sy0, sx1, sy1 = x0 + 0.12, y0 + 0.12, x0 + mw - 0.12, y0 + mh - 0.22
            if (i, j) == glowing:
                d += halo((sx0 + sx1) / 2, (sy0 + sy1) / 2, 0.8, "glow", 3, 0.1)
                d += rect(sx0, sy0, sx1, sy1, "glow!70!tealdark", lw(0.7))
                # a corridor on the screen, all shapes
                cx_, cy_ = (sx0 + sx1) / 2, (sy0 + sy1) / 2 + 0.1
                for (a, b) in [((sx0, sy0), (cx_ - 0.2, cy_ - 0.15)), ((sx1, sy0), (cx_ + 0.2, cy_ - 0.15)), ((sx0, sy1), (cx_ - 0.2, cy_ + 0.2)), ((sx1, sy1), (cx_ + 0.2, cy_ + 0.2))]:
                    d += line(a, b, lw=0.6, colour="tealdark")
                d += rect(cx_ - 0.2, cy_ - 0.15, cx_ + 0.2, cy_ + 0.2, "glowhi", lw(0.5))
                d += circle(cx_ + 0.42, cy_ - 0.2, 0.1, "tealdark", lw(0.4))
                d += f"\\fill[red] {P(sx1 - 0.2, sy1 - 0.2)} circle (0.05);\n"
            else:
                d += rect(sx0, sy0, sx1, sy1, "ink!85!tealdark", lw(0.7))
                d += f"\\fill[white, opacity=0.1] {polyp([(sx0 + 0.15, sy0), (sx0 + 0.45, sy0), (sx0 + 0.75, sy1), (sx0 + 0.45, sy1)])};\n"
                if (i, j) in cracked:
                    d += crack_star(sx0 + rng.uniform(0.4, 1.1), sy0 + rng.uniform(0.3, 0.9), 0.75, rng, "metallight")
            d += circle(x0 + mw - 0.2, y0 + 0.1, 0.035, "red" if (i + j) % 3 == 0 else "metaldark", lw(0.3))
    # cables hanging under the monitors
    for k in range(5):
        x = 7.5 + k * 1.5
        d += curve([(x, 4.2), (x + 0.3, 3.5 - k % 2 * 0.2), (x + 0.8, 4.0)], lw=1.4, colour="ink")
    # a clock without numbers, a floor-plan of the mall in blocks
    d += circle(15.0, 9.3, 0.38, "cream")
    d += line((15.0, 9.3), (15.0, 9.55), lw=1.2)
    d += line((15.0, 9.3), (15.18, 9.2), lw=1.2)
    # the window with its blinds, the hornet peeking through
    wx0, wx1, wy0, wy1 = 15.6, 18.9, 4.0, 8.9
    d += rect(wx0 - 0.2, wy0 - 0.25, wx1 + 0.2, wy1 + 0.2, "cream")
    d += rect(wx0, wy0, wx1, wy1, "skyhi")
    d += rect(wx0, wy0, wx1, wy0 + 1.6, "sky", lw(0.6))
    d += place(16.75, 6.56, 0.6, hornet(beam=False), flip=True)
    gap0, gap1 = 6.38, 6.78
    y = wy1 - 0.05
    k = 0
    while y > wy0 + 0.1:
        if gap0 < y < gap1:
            y -= 0.26
            continue
        d += rrect(wx0 - 0.05, y - 0.16, wx1 + 0.05, y, "paper", 0.03, lw(0.6))
        y -= 0.26
        k += 1
    # the bent slats round the gap
    d += f"\\filldraw[inked,fill=paper,line width=0.6pt] {P(wx0 - 0.05, gap1)} -- {P(wx0 + 0.2, gap1)} .. controls {P(wx0 + 0.7, gap1 + 0.22)} and {P(wx0 + 1.5, gap1 + 0.22)} .. {P(wx0 + 2.0, gap1)} -- {P(wx1 + 0.05, gap1)} -- {P(wx1 + 0.05, gap1 + 0.16)} -- {P(wx0 - 0.05, gap1 + 0.16)} -- cycle;\n"
    d += f"\\filldraw[inked,fill=paper,line width=0.6pt] {P(wx0 - 0.05, gap0)} -- {P(wx0 + 0.2, gap0)} .. controls {P(wx0 + 0.7, gap0 - 0.2)} and {P(wx0 + 1.5, gap0 - 0.2)} .. {P(wx0 + 2.0, gap0)} -- {P(wx1 + 0.05, gap0)} -- {P(wx1 + 0.05, gap0 - 0.16)} -- {P(wx0 - 0.05, gap0 - 0.16)} -- cycle;\n"
    d += line((wx0 + 0.25, wy1), (wx0 + 0.25, wy0), lw=0.6, colour="metal")
    d += line((wx1 - 0.3, wy1), (wx1 - 0.3, 3.4), lw=0.6, colour="metal")
    d += rect(wx0 - 0.1, wy1 - 0.05, wx1 + 0.1, wy1 + 0.15, "metallight", lw(0.7))
    # the lens pokes into the room: drawn over the blinds, beam to the desk
    d += fillo(polyp([(15.15, 6.6), (10.2, 3.65), (10.9, 3.4)]), "red", 0.14)
    d += rrect(15.0, 6.5, 16.0, 6.68, "red", 0.06, lw(0.8))
    d += f"\\fill[redlight] {P(15.15, 6.6)} rectangle {P(15.85, 6.64)};\n"
    d += ellipse(15.0, 6.59, 0.05, 0.11, "red", extra=lw(0.7))
    d += glint(14.99, 6.63, 0.025)
    d += halo(15.0, 6.59, 0.08, "red", 2, 0.2)
    # a filing cabinet under the clock
    d += rect(14.7, fy - 0.3, 15.45, 5.3, "metal")
    for k in range(3):
        d += rect(14.78, fy - 0.2 + k * 0.85, 15.37, fy + 0.55 + k * 0.85, "metallight", lw(0.6))
        d += rect(14.95, fy + 0.3 + k * 0.85, 15.2, fy + 0.38 + k * 0.85, "metaldark", lw(0.5))
    d += poly([(14.75, 5.3), (15.4, 5.3), (15.35, 5.5), (14.8, 5.45)], "paper", lw(0.6))
    # the desk
    d += poly([(6.7, 3.2), (14.4, 3.2), (14.9, 3.75), (7.0, 3.75)], "wood")
    d += rect(6.7, 0.6, 14.4, 3.2, "wooddark")
    d += rect(7.0, 0.6, 9.0, 3.0, "wood", lw(0.8))
    for k in range(3):
        d += rect(7.1, 0.75 + k * 0.75, 8.9, 1.35 + k * 0.75, "woodlight", lw(0.6))
        d += rect(7.8, 1.0 + k * 0.75, 8.2, 1.1 + k * 0.75, "metaldark", lw(0.5))
    d += rect(12.4, 0.6, 14.3, 3.0, "wood", lw(0.8))
    d += rect(12.5, 0.75, 14.2, 2.9, "woodlight", lw(0.6))
    # the console on the desk: buttons and a stick
    d += poly([(12.85, 3.4), (14.4, 3.4), (14.2, 3.95), (13.0, 3.95)], "armour", lw(0.8))
    for k in range(3):
        d += circle(13.15 + k * 0.3, 3.62, 0.08, ["red", "gold", "sage"][k], lw(0.4))
    d += line((13.9, 3.9), (14.0, 4.3), lw=1.6)
    d += circle(14.0, 4.33, 0.08, "red", lw(0.5))
    # keyboard, mug, papers, the walkie
    d += poly([(7.6, 3.3), (9.3, 3.3), (9.15, 3.6), (7.75, 3.6)], "metallight", lw(0.7))
    for k in range(1, 6):
        d += line((7.65 + k * 0.27, 3.33), (7.75 + k * 0.24, 3.57), lw=0.4)
    d += rrect(9.45, 3.35, 9.75, 3.85, "red", 0.05, lw(0.7))
    d += curve([(9.75, 3.75), (9.9, 3.65), (9.75, 3.48)], lw=0.9)
    d += place(7.9, 3.85, 1, poly([(-0.35, -0.2), (0.3, -0.24), (0.35, 0.2), (-0.3, 0.22)], "paper", lw(0.6)), rot=-8)
    d += rrect(7.15, 3.3, 7.4, 3.95, "ink!70!metaldark", 0.05, lw(0.7))
    d += line((7.32, 3.95), (7.35, 4.35), lw=0.8)
    # the three hard drives, glowing
    for hx in (10.55, 11.6, 12.65):
        d += place(hx - 0.1, 3.5, 0.92, hard_drive(0, 0, 0.9))
    # the office chair, rolled away and turned
    cx_ = 16.0
    d += fillo(ellipse_path(cx_, 0.35, 1.1, 0.15), "ink", 0.2)
    for a in (-0.8, -0.3, 0.3, 0.8):
        d += line((cx_, 0.75), (cx_ + a, 0.3), lw=1.6)
        d += circle(cx_ + a, 0.25, 0.08, "ink", lw(0.4))
    d += rect(cx_ - 0.06, 0.75, cx_ + 0.06, 1.6, "metaldark", lw(0.7))
    d += rrect(cx_ - 0.75, 1.6, cx_ + 0.75, 1.95, "tealdark", 0.12)
    d += rrect(cx_ + 0.35, 1.8, cx_ + 0.7, 3.5, "tealdark", 0.15)
    # paper drifts on the floor
    for (px, py, r_) in [(10.0, 0.3, 10), (11.3, 0.5, -20), (17.6, 2.3, 35)]:
        d += place(px, py, 1, poly([(-0.3, -0.15), (0.28, -0.18), (0.3, 0.15), (-0.27, 0.17)], "paper", lw(0.6)), rot=r_)
    s += right_of(MCALM, d)
    return s, (W, H)



# --------------------------------------------------------------------------
# missions/hive
# --------------------------------------------------------------------------

def mannequin(fill="cream!70!stone", head=True, pose=0):
    """A shop mannequin standing, feet at the origin, about 3.4 tall:
    smooth, faceless, jointed at the shoulders."""
    s = rect(-0.35, 0, 0.35, 0.08, "metaldark", lw(0.8))
    s += line((0, 0.08), (0.05, 0.5), lw=1.4, colour="metal")
    s += limb([(-0.15, 1.6), (-0.18, 0.85), (-0.2, 0.25)], 0.13, 0.09, fill, cap=True)
    s += limb([(0.15, 1.6), (0.2, 0.85), (0.25, 0.25)], 0.13, 0.09, fill, cap=True)
    s += blob([(-0.38, 1.5), (0.38, 1.5), (0.42, 2.0), (0.5, 2.7), (0.2, 2.82), (-0.2, 2.82), (-0.5, 2.7), (-0.42, 2.0)], fill, tension=0.5)
    s += limb([(-0.45, 2.65), (-0.62, 2.0), (-0.55, 1.45 if pose == 0 else 2.6)], 0.1, 0.07, fill, cap=True)
    s += limb([(0.45, 2.65), (0.6, 2.05), (0.6, 1.5)], 0.1, 0.07, fill, cap=True)
    s += rect(-0.08, 2.8, 0.08, 2.95, fill, lw(0.7))
    if head:
        s += ellipse(0, 3.15, 0.2, 0.25, fill)
    return s


def shelf_unit(w, h, n, fill="metal", lwid=0.9):
    """A gondola shelf unit: two uprights and n shelves, origin bottom-left."""
    s = rect(0, 0, 0.12, h, fill, lw(lwid))
    s += rect(w - 0.12, 0, w, h, fill, lw(lwid))
    s += rect(0.12, 0.2, w - 0.12, h - 0.1, "ink!65!metaldark", lw(lwid * 0.7))
    for k in range(n):
        y = 0.2 + k * (h - 0.3) / max(1, n - 1) if n > 1 else 0.2
        s += rect(-0.05, y, w + 0.05, y + 0.12, "metallight", lw(lwid))
    return s


def pod(x, y, rot_=0, s_=1.0):
    """A charging pod: a glass capsule glowing cyan in a metal cradle."""
    b = halo(0, 0, 0.32, "glow", 3, 0.13)
    b += rrect(-0.22, -0.42, 0.22, 0.42, "glow", 0.2, lw(0.9))
    b += f"\\fill[glowhi] (-0.1,-0.25) rectangle (-0.04,0.28);\n"
    b += rect(-0.28, -0.5, 0.28, -0.36, "metaldark", lw(0.8))
    b += rect(-0.28, 0.36, 0.28, 0.5, "metaldark", lw(0.8))
    return place(x, y, s_, b, rot=rot_)


def hive():
    """Inside a department-store anchor: a huge drone hive and charging
    nest heaped from scavenged shelving, draped in cables and studded with
    glowing charging pods; two wardens and a crowd of wasps docked on it;
    mannequins knocked over on the floor, a rack tipped. Left third: plain
    wall and floor."""
    W, H = MW, MH
    rng = random.Random(6061)
    fy = 3.4
    wall = "wallF!55!ink"
    s = rect(-0.5, -0.5, W + 0.5, H + 0.5, wall)
    s += rect(-0.5, 9.6, W + 0.5, H + 0.5, "wallF!35!ink")
    s += line((-0.5, 9.6), (W + 0.5, 9.6), lw=1.0)
    s += rect(-0.5, -0.5, W + 0.5, fy, "stone!45!ink")
    s += rect(-0.5, fy, W + 0.5, fy + 0.2, "wallF!35!ink", lw(0.9))
    s += floor_tiles(W, fy, 12.6, 6.5, "ink", n=11, op=0.3)
    d = ""
    # ceiling grid with fallen tiles
    for k in range(10):
        d += line((6.4 + k * 1.5, 9.6), (6.4 + k * 1.5 + 0.35, H + 0.1), lw=0.6)
    d += poly([(16.3, 9.6), (17.6, 9.6), (17.4, 8.9)], "wallF!35!ink", lw(0.8))
    d += poly([(8.0, 9.6), (9.2, 9.6), (8.3, 9.05)], "wallF!35!ink", lw(0.8))
    # department signs hanging on chains: shapes only
    for (x, c, kind) in [(7.6, "teal", 0), (17.9, "gold", 1)]:
        d += line((x - 0.5, 9.6), (x - 0.5, 8.9), lw=0.7)
        d += line((x + 0.5, 9.6), (x + 0.5, 8.9), lw=0.7)
        d += rect(x - 0.75, 8.2, x + 0.75, 8.9, c)
        if kind == 0:
            d += circle(x, 8.55, 0.2, "cream", lw(0.7))
        else:
            d += poly([(x - 0.25, 8.35), (x + 0.25, 8.35), (x, 8.75)], "cream", lw(0.7))
    # a column, a wall of empty shelving behind the hive
    d += rect(17.8, fy, 18.6, 9.6, "wallF!45!ink", lw(1.0))
    d += rect(17.7, 9.3, 18.7, 9.6, "wallF!35!ink", lw(0.8))
    for k in range(3):
        d += place(7.0 + k * 1.6, fy + 0.1, 1, shelf_unit(1.5, 3.0, 4, "metaldark", 0.7))
    # the hive: a dark heaped mass, then scavenged shelf units on it
    hx = 12.6
    mass = [(8.4, fy - 1.2), (9.0, 5.5), (10.2, 7.8), (11.6, 9.1), (13.0, 9.4), (14.5, 8.6), (15.9, 7.0), (16.9, 4.6), (17.2, fy - 1.3), (12.6, fy - 1.9)]
    d += fillo(ellipse_path(hx, 1.6, 4.8, 0.7), "glow", 0.12)
    d += blob(mass, "ink!55!wallE", tension=0.55)
    units = [(9.2, 2.3, 1.6, 3.2, 4, -8), (10.6, 1.9, 1.8, 3.6, 5, 4), (12.6, 2.0, 1.8, 3.3, 4, -3), (14.5, 2.0, 1.7, 3.2, 4, 6), (9.9, 5.2, 1.5, 2.6, 3, 18), (11.6, 5.3, 1.7, 3.0, 4, -6), (13.4, 5.0, 1.6, 2.8, 3, 10), (15.0, 4.8, 1.3, 2.2, 3, -22), (11.9, 7.6, 1.4, 1.8, 3, 24), (12.9, 7.4, 1.2, 1.9, 3, -18)]
    for (x, y, w, h, n, r_) in units:
        d += place(x, y, 1, shelf_unit(w, h, n, rng.choice(["metal", "metaldark", "wallE"])), rot=r_)
    # cables draped across the nest
    for (a, m, b, c) in [((9.0, 6.0), (11.0, 5.0), (13.2, 6.4), "red"), ((10.3, 8.0), (12.4, 6.8), (14.8, 7.6), "gold"), ((9.2, 3.8), (12.5, 2.8), (16.4, 4.0), "teal"), ((11.2, 9.0), (12.0, 8.0), (13.4, 9.2), "ink"), ((14.0, 9.6), (14.2, 8.5), (15.6, 6.4), "red")]:
        d += curve([a, m, b], lw=3.4, colour="ink")
        d += curve([a, m, b], lw=2.0, colour=c)
    # cables up to the ceiling
    for x in (11.5, 13.6):
        d += curve([(x, 9.0), (x + 0.2, 9.4), (x + 0.1, H + 0.1)], lw=1.6)
    # charging pods studded through it
    for (x, y, r_) in [(9.6, 4.4, 10), (10.9, 6.6, -12), (12.2, 4.3, 0), (13.8, 6.2, 8), (15.6, 3.6, -6), (12.9, 8.5, 20), (11.2, 2.6, 0), (14.6, 4.6, 0), (16.2, 5.6, -20), (10.0, 7.4, 26)]:
        d += pod(x, y, r_, 0.85)
    # docked drones
    d += drone("warden", 12.6, 9.55, 0.7)
    d += drone("warden", 15.4, 7.3, 0.55, rot=-10)
    for (x, y, sc, r_) in [(10.0, 5.95, 0.42, 6), (13.1, 5.65, 0.4, -4), (11.6, 7.65, 0.36, 10), (14.9, 3.15, 0.4, 0), (9.6, 3.35, 0.38, -6), (16.6, 4.6, 0.36, 12)]:
        d += drone("wasp", x, y, sc, rot=r_)
    d += drone("wasp", 8.2, 6.9, 0.55, rot=-14)
    # glow pooling on the floor
    d += fillo(ellipse_path(hx, 0.9, 3.8, 0.45), "glow", 0.12)
    # mannequins: one flat on its back, one face down, one headless leaning
    d += place(10.3, 0.8, 0.82, mannequin(), rot=90)
    d += place(14.5, 0.55, 0.8, mannequin(pose=1), rot=-90)
    d += place(18.3, 1.25, 0.8, mannequin(head=False), rot=-12)
    d += ellipse(17.75, 0.42, 0.2, 0.25, "cream!70!stone", rot=20)
    # a tipped clothes rack with garments spilled
    d += place(12.6, 1.55, 1, line((-1.2, 0), (1.2, 0.15), lw=2.0, colour="metal") + line((-1.1, 0), (-1.0, -0.3), lw=1.6, colour="metal"), rot=4)
    for (x, c) in [(11.9, "teal"), (12.7, "wallC"), (13.5, "gold")]:
        d += place(x, 1.1, 0.55, tee(c, "cream", rng.randint(0, 3), hanger=False), rot=rng.uniform(70, 110))
    d += shards(rng, 15.2, 17.2, 1.2, 2.2, 8, "metallight")
    for (bx, by, c, r_) in [(9.0, 2.0, "wood", -8), (16.7, 2.0, "woodlight", 12)]:  # scrap boxes at the nest's foot
        d += place(bx, by, 1, rect(-0.45, 0, 0.45, 0.6, c) + line((-0.45, 0.45), (0.45, 0.45), lw=0.6) + rect(-0.1, 0.45, 0.1, 0.6, "cream", lw(0.5)), rot=r_)
    s += right_of(MCALM, d)
    return s, (W, H)



# --------------------------------------------------------------------------
# missions/power
# --------------------------------------------------------------------------

def storefront(rng, x0, x1, y0, y1, band, shut=0.0):
    """A mall shop front: a sign band with a shape, a glass front with
    mullions, a roll-down grille down by `shut` (0..1)."""
    s = rect(x0, y0, x1, y1, "stonedark")
    s += rect(x0 + 0.1, y1 - 0.55, x1 - 0.1, y1 - 0.1, band, lw(0.8))
    cx = (x0 + x1) / 2
    k = rng.randint(0, 2)
    if k == 0:
        s += circle(cx, y1 - 0.32, 0.15, "cream", lw(0.6))
    elif k == 1:
        s += poly([(cx - 0.2, y1 - 0.45), (cx + 0.2, y1 - 0.45), (cx, y1 - 0.18)], "cream", lw(0.6))
    else:
        s += rect(cx - 0.3, y1 - 0.4, cx + 0.3, y1 - 0.25, "cream", lw(0.6))
    gx0, gx1, gy0, gy1 = x0 + 0.15, x1 - 0.15, y0, y1 - 0.65
    s += rect(gx0, gy0, gx1, gy1, "windark", lw(0.7))
    s += f"\\fill[white, opacity=0.12] {polyp([(gx0 + 0.2, gy0), (gx0 + 0.6, gy0), (gx0 + 1.0, gy1), (gx0 + 0.6, gy1)])};\n"
    for k in range(1, 3):
        s += line((gx0 + (gx1 - gx0) * k / 3, gy0), (gx0 + (gx1 - gx0) * k / 3, gy1), lw=0.6)
    if shut > 0:
        sy = gy1 - (gy1 - gy0) * shut
        s += rect(gx0, sy, gx1, gy1, "metal", lw(0.7))
        y = sy + 0.14
        while y < gy1:
            s += line((gx0, y), (gx1, y), lw=0.4)
            y += 0.14
    return s


def generator(w=1.0):
    """A big portable generator on a wheeled skid, about 3.3 wide, base on
    y=0: a sage housing with louvres, a control panel with dials and
    sockets, a fuel tank, an exhaust stack with a smoke puff."""
    s = rect(-1.65, 0.25, 1.65, 0.5, "metaldark", lw(w))
    for wx in (-1.2, 1.2):
        s += circle(wx, 0.22, 0.25, "ink!80!asphaltdark", lw(w))
        s += circle(wx, 0.22, 0.1, "metal", lw(w * 0.7))
    s += rrect(-1.5, 0.5, 1.5, 2.3, "sage", 0.15, lw(w))
    s += rrect(-1.4, 2.3, 1.1, 2.6, "sagedark", 0.08, lw(w))
    for k in range(6):
        s += rect(-1.3 + k * 0.2, 0.75, -1.2 + k * 0.2, 1.9, "sagedark", lw(w * 0.6))
    s += rect(0.0, 0.8, 1.3, 2.05, "armourdark", lw(w))
    for k, c in enumerate(["glow", "gold", "red"]):
        s += circle(0.3 + k * 0.38, 1.7, 0.13, "cream", lw(w * 0.6))
        s += line((0.3 + k * 0.38, 1.7), (0.3 + k * 0.38 + 0.08, 1.78), lw=0.6, colour="red")
    for k in range(3):
        s += rrect(0.15 + k * 0.38, 1.0, 0.4 + k * 0.38, 1.3, "orange", 0.04, lw(w * 0.6))
    s += circle(1.1, 1.15, 0.06, "glow", lw(0.4))
    s += rect(1.1, 2.3, 1.3, 3.1, "metaldark", lw(w))
    s += rect(1.05, 3.05, 1.35, 3.15, "metaldark", lw(w))
    for (dx, dy, r) in [(0.25, 3.4, 0.2), (0.55, 3.75, 0.28), (0.95, 4.15, 0.34)]:
        s += circle(1.2 + dx - 0.2, dy, r, "concrete!70!metallight", lw(w * 0.8))
    s += rrect(-1.9, 0.5, -1.5, 1.4, "red", 0.06, lw(w))
    s += rect(-1.82, 1.4, -1.68, 1.55, "metaldark", lw(w * 0.6))
    s += f"\\fill[white, opacity=0.25] (-1.4,2.05) rectangle (1.4,2.15);\n"
    return s


def work_light(x, y, h, rot_=0):
    """A tripod work light, glowing, feet on y."""
    s = line((x, y + h * 0.35), (x - 0.4, y), lw=1.3)
    s += line((x, y + h * 0.35), (x + 0.4, y), lw=1.3)
    s += line((x, y + h * 0.35), (x + 0.05, y - 0.1), lw=1.3)
    s += line((x, y + h * 0.35), (x, y + h), lw=1.6)
    head = halo(0, 0.05, 0.3, "window", 3, 0.15) + rrect(-0.35, -0.25, 0.35, 0.3, "gold", 0.06) + rect(-0.28, -0.18, 0.28, 0.22, "window", lw(0.7))
    return s + place(x, y + h, 1, head, rot=rot_)


def cable(points, colour="ink!80!asphaltdark", w=2.2):
    return curve(points, lw=w + 1.2, colour="ink") + curve(points, lw=w, colour=colour)


def power():
    """The central atrium court, two levels: shops up and down, a glass
    railing along the upper walk, an escalator, a skylight; a big portable
    generator in the middle with cables to work lights and a string of
    bulbs; the fountain dried and cracked; tick spiders and a strider
    crawling along the upper railing. Left third: plain wall and floor."""
    W, H = MW, MH
    rng = random.Random(7781)
    fy = 2.6
    s = rect(-0.5, -0.5, W + 0.5, H + 0.5, "stone")
    s += rect(-0.5, -0.5, W + 0.5, fy, "paper!65!kerb")
    s += line((-0.5, fy), (W + 0.5, fy), lw=1.1)
    s += floor_tiles(W, fy, 12.0, 7.0, "stonedark", n=11, op=0.45)
    d = ""
    # skylight vault with trusses
    d += rect(6.4, 9.7, W + 0.5, H + 0.5, "skyhi")
    d += rect(6.4, 9.55, W + 0.5, 9.75, "concrete", lw(0.9))
    for k in range(8):
        x = 6.9 + k * 1.75
        d += f"\\draw[ink, line width=1.0pt] {P(x, 9.75)} arc[start angle=180, end angle=90, radius=1.05];\n"
        d += line((x, 9.75), (x, H + 0.1), lw=0.8)
    d += vine(9.4, 9.6, 1.0, rng)
    d += vine(15.8, 9.6, 1.4, rng)
    # upper shops
    xs = [6.6, 9.5, 12.3, 15.4, 18.3, 21.0]
    bands = ["wallC", "teal", "gold", "violet", "orange"]
    for k in range(5):
        d += storefront(rng, xs[k] + 0.1, xs[k + 1] - 0.1, 6.95, 9.5, bands[k], shut=rng.choice([0.0, 0.5, 1.0, 0.7]))
    # the upper slab and lower shops in its shade
    for k in range(5):
        d += storefront(rng, xs[k] + 0.4 - 0.3, xs[k + 1] - 0.4 + 0.2, fy, 5.7, ["teal", "redlight", "sage", "pink", "gold"][k], shut=rng.choice([0.3, 1.0, 0.6]))
    d += fillo(polyp([(6.4, 5.7), (W + 0.5, 5.7), (W + 0.5, 4.6), (6.4, 4.6)]), "ink", 0.15)
    d += rect(6.4, 5.7, W + 0.5, 6.95, "concrete")
    d += rect(6.4, 6.0, W + 0.5, 6.15, "window", lw(0.6))
    d += rect(6.45, fy, 6.9, 9.6, "stonedark")  # the column that closes the calm third
    # glass railing, handrail, and the spiders crawling on it
    for k in range(9):
        x = 6.8 + k * 1.4
        d += f"\\filldraw[draw=ink, line width=0.8pt, fill=glowhi, fill opacity=0.25] {P(x, 6.95)} rectangle {P(x + 1.3, 7.85)};\n"
    d += rrect(6.6, 7.85, W + 0.5, 8.0, "metallight", 0.04, lw(0.9))
    d += drone("tick", 8.3, 8.05, 0.45)
    d += drone("tick", 10.6, 7.25, 0.42, rot=-90)
    d += drone("tick", 12.9, 8.05, 0.4)
    d += drone("strider", 15.6, 8.55, 0.55)
    # string of bulbs along the railing, fed from the generator
    sb = [(6.9, 7.7), (9.0, 7.2), (11.0, 7.65), (13.2, 7.15), (15.0, 7.6), (17.2, 7.2), (19.4, 7.6)]
    d += curve(sb, lw=1.0)
    for k in range(1, 23):
        t = k / 23
        idx = min(len(sb) - 2, int(t * (len(sb) - 1)))
        u = t * (len(sb) - 1) - idx
        p = lerp(sb[idx], sb[idx + 1], u)
        d += halo(p[0], p[1] - 0.12, 0.05, "window", 2, 0.25)
        d += circle(p[0], p[1] - 0.12, 0.07, "window", lw(0.5))
    # escalator from the floor up to the upper walk
    e0, e1 = (14.7, 0.9), (19.8, 6.3)
    band = [(e0[0], e0[1]), (e0[0] + 1.6, e0[1]), (e1[0] + 1.6, e1[1]), (e1[0], e1[1])]
    d += poly([(e0[0] + 1.6, e0[1]), (e1[0] + 1.6, e1[1]), (e1[0] + 1.6, e1[1] - 1.0), (e0[0] + 2.4, e0[1] - 0.5)], "metaldark")
    d += poly(band, "metal")
    for k in range(1, 18):
        a, b = lerp(band[0], band[3], k / 18), lerp(band[1], band[2], k / 18)
        d += line(a, b, lw=0.6)
    d += f"\\filldraw[draw=ink, line width=0.9pt, fill=glowhi, fill opacity=0.3] {polyp([(e0[0] - 0.1, e0[1]), (e1[0] - 0.1, e1[1]), (e1[0] - 0.1, e1[1] + 0.9), (e0[0] - 0.1, e0[1] + 0.9)])};\n"
    d += cable([(e0[0] - 0.3, e0[1] + 0.85), (e0[0] - 0.1, e0[1] + 0.95), (e1[0] - 0.1, e1[1] + 0.95)], "ink!85!black", 2.0)
    d += rect(e0[0] - 0.4, e0[1] - 0.2, e0[0] + 1.8, e0[1] + 0.02, "metallight", lw(0.8))
    # light from the skylight
    for (a, b, c, e) in [(8.5, 9.6, 11.0, 8.6), (13.0, 14.3, 15.9, 13.1)]:
        d += fillo(polyp([(a, 9.6), (b, 9.6), (c, 0.0), (e, 0.0)]), "white", 0.14)
    # the generator and its cables
    gx, gy_ = 12.0, 1.85
    d += fillo(ellipse_path(gx, gy_ + 0.05, 2.2, 0.25), "ink", 0.2)
    d += cable([(gx - 1.5, gy_ + 1.0), (gx - 2.2, gy_ - 0.3), (gx - 3.6, gy_ + 0.6), (8.6, 2.55)])
    d += cable([(gx + 1.4, gy_ + 1.1), (gx + 2.0, gy_ - 0.2), (gx + 2.6, gy_ + 0.15), (15.3, 2.75)], "red!70!ink")
    d += cable([(gx - 0.8, gy_ + 2.2), (gx - 1.6, gy_ + 3.4), (gx - 2.2, 5.0), (9.0, 7.2)], "gold!70!ink", 1.8)
    d += work_light(8.6, 2.55, 2.4, rot_=-25)
    d += work_light(15.3, 2.75, 2.2, rot_=30)
    d += place(gx, gy_, 1.0, generator())
    d += place(gx + 2.3, gy_ - 0.65, 0.7, rrect(-0.35, 0, 0.35, 0.9, "red", 0.06) + rect(-0.15, 0.9, 0.15, 1.05, "metaldark", lw(0.6)) + line((-0.2, 0.6), (0.2, 0.6), lw=0.6))
    # the dry fountain in front
    fx, fy_ = 9.0, 0.75
    d += ellipse(fx, fy_ + 0.25, 2.2, 0.55, "stonedark")
    d += ellipse(fx, fy_ + 0.35, 1.9, 0.42, "concrete!80!wooddark")
    c1, _ = crack(rng, fx - 1.2, fy_ + 0.35, 6, 0.35, 10, 0.6)
    d += clipped(ellipse_path(fx, fy_ + 0.35, 1.9, 0.42), c1 + "".join(circle(fx + rng.uniform(-1.4, 1.4), fy_ + rng.uniform(0.1, 0.55), 0.05, "gold", lw(0.4)) for _ in range(9)))
    d += f"\\filldraw[inked,fill=stone] {P(fx - 2.2, fy_ + 0.25)} arc[start angle=180, end angle=360, x radius=2.2, y radius=0.55] -- {P(fx + 2.2, fy_ - 0.2)} arc[start angle=0, end angle=-180, x radius=2.2, y radius=0.55] -- cycle;\n"
    d += rect(fx - 0.25, fy_ + 0.35, fx + 0.25, fy_ + 1.5, "stone")
    d += ellipse(fx, fy_ + 1.55, 0.85, 0.2, "stonedark")
    d += ellipse(fx, fy_ + 1.6, 0.7, 0.13, "concrete!80!wooddark", extra=lw(0.7))
    d += rect(fx - 0.08, fy_ + 1.6, fx + 0.08, fy_ + 2.0, "stone", lw(0.8))
    d += circle(fx, fy_ + 2.05, 0.12, "stone", lw(0.8))
    d += curve([(fx - 0.6, fy_ + 1.45), (fx - 0.5, fy_ + 1.1)], lw=0.8, colour="wooddark")
    d += blades(fx + 1.6, fy_ + 0.1, 0.55, "greenB")
    d += blades(fx - 1.5, fy_ + 0.05, 0.5, "greenA")
    # a planter gone wild and a bench
    d += rect(17.6, 0.0, W + 0.5, 1.3, "wallE")
    for k in range(10):
        a = rng.uniform(-60, 60)
        b = (18.2 + rng.uniform(-0.4, 0.8), 1.3)
        d += leaf(b, (b[0] + math.sin(math.radians(a)) * 1.8, b[1] + math.cos(math.radians(a)) * 1.8), 0.22, rng.choice(["greenA", "greenB", "greenD"]))
    s += right_of(MCALM, d)
    return s, (W, H)


# --------------------------------------------------------------------------
# missions/queen
# --------------------------------------------------------------------------

def brick_front(rng, x0, x1, y1, fill, trim="stone", awning="teal"):
    """A Main Street building seen from the street: a brick face with rows of
    windows, a cornice and a striped awning over a dark shop front."""
    s = rect(x0, 1.2, x1, y1, fill)
    s += rect(x0 - 0.12, y1, x1 + 0.12, y1 + 0.3, trim)
    cols = max(2, int((x1 - x0) / 1.05))
    w = (x1 - x0) / cols
    y = 4.0
    while y + 0.9 < y1 - 0.2:
        for c in range(cols):
            wx = x0 + c * w + w * 0.22
            lit = rng.random() < 0.25
            s += rect(wx, y, wx + w * 0.56, y + 0.85, "window" if lit else "windark", lw(0.7))
            s += line((wx, y + 0.42), (wx + w * 0.56, y + 0.42), lw=0.5)
        y += 1.4
    s += rect(x0 + 0.2, 1.2, x1 - 0.2, 2.9, "ink!70!windark", lw(0.8))
    s += poly([(x0 + 0.05, 3.5), (x1 - 0.05, 3.5), (x1 + 0.2, 2.95), (x0 - 0.2, 2.95)], awning)
    k = 0
    x = x0
    while x < x1 - 0.2:
        if k % 2 == 0:
            s += poly([(x + 0.05, 3.5), (min(x + 0.45, x1 - 0.05), 3.5), (min(x + 0.5, x1 + 0.2), 2.95), (x - 0.0, 2.95)], "cream", lw(0.5))
        x += 0.45
        k += 1
    return s


def queen_card():
    """Main Street at dusk: brick fronts with awnings either side, street
    lamps, and the queen striding down the middle of the street, her knees
    above the cornices, rockets streaking from her pods and her wasps
    leaving her back; a raider in the right foreground, small against her.
    Left third: plain sky and street."""
    W, H = MW, MH
    rng = random.Random(9071)
    s = sky(W, H)
    s += rect(-0.5, -0.5, W + 0.5, 1.2, "asphalt")
    s += line((-0.5, 1.2), (W + 0.5, 1.2), lw=1.0)
    s += rect(-0.5, 1.2, W + 0.5, 1.45, "kerb", lw(0.8))
    d = ""
    for (cx, cy, cs) in [(9.6, 9.7, 0.7), (15.5, 10.1, 0.6)]:
        d += cloud(cx, cy, cs)
    # the fronts behind her, then her, then the near lamp and the raider
    for (x0, x1, y1, c, a) in [(6.7, 9.6, 7.4, "wallE", "teal"), (9.9, 12.8, 8.7, "wallA", "gold"), (13.1, 16.2, 6.9, "wallC", "teal"), (16.5, 19.6, 8.0, "wallE", "violet")]:
        d += brick_front(rng, x0, x1, y1, c, awning=a)
    for x in (8.4, 14.6):
        d += rect(x - 0.05, 1.45, x + 0.05, 5.0, "metaldark", lw(0.6))
        d += halo(x, 5.15, 0.14, "window", 3, 0.2)
        d += circle(x, 5.15, 0.2, "window", lw(0.7))
    # dust where her feet come down
    for (x, y) in [(9.6, 1.25), (12.6, 1.3), (15.7, 1.25)]:
        d += fillo(ellipse_path(x, y, 0.9, 0.22), "stonedark", 0.45)
    d += place(12.7, 1.3, 1.22, queen(stride=0.35))
    # rockets from her pods toward the raider, and her wasps
    for (a, b, c) in [((11.6, 8.5), (14.8, 9.3), (17.8, 4.6)), ((12.3, 8.7), (15.6, 9.7), (18.4, 5.4))]:
        d += curve([a, b, c], lw=3.2, colour="stone", extra=", opacity=0.55")
        d += curve([a, b, c], lw=1.2, colour="gold")
        d += circle(c[0], c[1], 0.13, "orange", lw(0.6))
        d += halo(c[0], c[1], 0.12, "orange", 3, 0.2)
    for (x, y, sc, r_) in [(14.6, 8.9, 0.32, -10), (15.8, 7.9, 0.28, 14), (10.4, 9.6, 0.3, 8)]:
        d += drone("wasp", x, y, sc, rot=r_)
    d += place(18.6, 0.3, 0.55, raider("jacketb", gun="ak"))
    s += right_of(MCALM, d)
    return s, (W, H)


PICTURES = {"title": title, "missions/cudi": cudi, "missions/foodcourt": foodcourt, "missions/drives": drives, "missions/hive": hive, "missions/power": power, "missions/queen": queen_card}
