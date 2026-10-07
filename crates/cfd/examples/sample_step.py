# SPDX-License-Identifier: GPL-3.0-or-later
# Writes the STEP fixture examples/sample_rocket.step (run by the sample_step example with the gmsh python).
#
# The rocket is rebuilt like the CAD surface of mesh_gmsh.py (body of revolution of the extracted profile fused with
# biconvex lens fins, one fin at 90 deg + roll from +y towards +z in the CFD frame), plus details that the STL does
# not have: a launch lug (tube 6.35 mm outer / 4.8 mm inner diameter, 50 mm long, at 45 % of the length) and two
# rail buttons (stem 8 mm diameter, flange 11 mm, standing 6 mm proud of the body) a quarter turn from the lug. Every detail
# is sunk 0.5 mm into the body, so the boolean union has proper intersection curves. The solid is moved from the CFD
# frame back to the STL coordinates and written in millimetres.
import json
import math
import sys

import gmsh

p = json.load(open(sys.argv[1]))
gmsh.initialize()
gmsh.option.setNumber("General.Terminal", 0)
gmsh.model.add("sample")
occ = gmsh.model.occ
prof = p["profile"]
fins = p["fins"]
L = p["length"]
roll = math.radians(p["fin_roll_deg"])
nf = fins["count"]


def radius(x):
    for (xa, ra), (xb, rb) in zip(prof[:-1], prof[1:]):
        if xa <= x <= xb:
            return ra + (rb - ra) * (x - xa) / max(xb - xa, 1e-12)
    return prof[-1][1]


seam = math.radians(90.0 + 180.0 / nf) + roll
cs, sn = math.cos(seam), math.sin(seam)
pts = [occ.addPoint(x, r * cs, r * sn) for x, r in prof]
axis1 = occ.addPoint(prof[-1][0], 0, 0)
segs = [occ.addLine(pts[i], pts[i + 1]) for i in range(len(pts) - 1)] + [occ.addLine(pts[-1], axis1), occ.addLine(axis1, pts[0])]
face = occ.addPlaneSurface([occ.addWire(segs)])
rev = occ.revolve([(2, face)], 0, 0, 0, 1, 0, 0, 2 * math.pi)
body = [(3, t) for d, t in rev if d == 3]


def lens(x_le, x_te, r_pos, phi, thick, n=8):
    u = (math.cos(phi), math.sin(phi))
    w = (-math.sin(phi), math.cos(phi))

    def pt(xi, side):
        x = x_le + xi * (x_te - x_le)
        h = 0.5 * thick * 4.0 * xi * (1.0 - xi) * side
        return occ.addPoint(x, r_pos * u[0] + h * w[0], r_pos * u[1] + h * w[1])

    le, te = pt(0.0, 1), pt(1.0, 1)
    up = [le] + [pt(i / n, 1) for i in range(1, n)] + [te]
    lo = [le] + [pt(i / n, -1) for i in range(1, n)] + [te]
    return occ.addWire([occ.addSpline(up), occ.addSpline(lo)])


tools = []
f = fins
for k in range(nf):
    phi = math.radians(90.0) + roll + 2 * math.pi * k / nf
    w1 = lens(f["x_le"], f["x_te"], 0.5 * f["inner_radius"], phi, f["thickness"])
    w2 = lens(f["x_le"] + f["sweep"], f["x_le"] + f["sweep"] + f["tip_chord"], f["body_radius"] + f["span"], phi, f["thickness"])
    fin = occ.addThruSections([w1, w2], makeSolid=True, makeRuled=True)
    tools.append(fin[0] if not isinstance(fin[0], int) else (3, fin[0]))

# Launch lug between two fins, rail buttons between the next two (a quarter turn on): the shape is mirror symmetric
# about neither y = 0 nor z = 0. `"details": false` leaves them out (a symmetric shape, for tests).
sink = 0.0005
phi_l = math.radians(180.0 + 180.0 / nf) + roll
ro, ri, lug_len = 0.5 * 6.35e-3, 0.5 * 4.8e-3, 0.05
x_l = 0.45 * L
rc = radius(x_l + 0.5 * lug_len) + ro - sink
cy, cz = rc * math.cos(phi_l), rc * math.sin(phi_l)
outer = occ.addCylinder(x_l, cy, cz, lug_len, 0, 0, ro)
inner = occ.addCylinder(x_l - 1e-3, cy, cz, lug_len + 2e-3, 0, 0, ri)
lug, _ = occ.cut([(3, outer)], [(3, inner)])
extra = list(lug)
phi_b = phi_l + 0.5 * math.pi
d = (0.0, math.cos(phi_b), math.sin(phi_b))
for xb in (0.35 * L, 0.7 * L):
    r0 = radius(xb) - sink
    stem = occ.addCylinder(xb, r0 * d[1], r0 * d[2], 0, 6.5e-3 * d[1], 6.5e-3 * d[2], 4e-3)
    r1 = radius(xb) + 4e-3
    flange = occ.addCylinder(xb, r1 * d[1], r1 * d[2], 0, 2e-3 * d[1], 2e-3 * d[2], 5.5e-3)
    extra += [(3, stem), (3, flange)]
if p.get("details", True):
    tools += extra
else:
    occ.remove(extra, recursive=True)
out, _ = occ.fuse(body, tools)
occ.synchronize()
vols = gmsh.model.getEntities(3)
if len(vols) != 1:
    raise SystemExit("expected one solid, got %d" % len(vols))
# Back to the STL coordinates: v = M^T (p - t).
a = p["affine"]
M = [a[0:3], a[4:7], a[8:11]]
t = [a[3], a[7], a[11]]
Mt = [[M[j][i] for j in range(3)] for i in range(3)]
tt = [-sum(Mt[i][j] * t[j] for j in range(3)) for i in range(3)]
occ.affineTransform(vols, [Mt[0][0], Mt[0][1], Mt[0][2], tt[0], Mt[1][0], Mt[1][1], Mt[1][2], tt[1], Mt[2][0], Mt[2][1], Mt[2][2], tt[2]])
occ.synchronize()
# Coordinates in metres in the model; written as millimetres (SI_UNIT(.MILLI.,.METRE.)).
gmsh.option.setString("Geometry.OCCTargetUnit", "M")
gmsh.write(p["out"])
print("faces %d, volume %.6e m^3, bbox %s" % (len(gmsh.model.getEntities(2)), occ.getMass(3, vols[0][1]), gmsh.model.getBoundingBox(3, vols[0][1])))
gmsh.finalize()
