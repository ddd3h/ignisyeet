# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2024-2026 西濱大将 (NISHIHAMA Daisuke)
# Writes basic_finner.step and basic_finner.stl: the Army-Navy Basic Finner reference projectile of Dupuis & Hathaway,
# DREV-TM-9703 (1997), Fig. 2 and Section 3.1, with d = 30 mm (the tested model).
#
#   python make_basic_finner.py [outdir] [--sharp]
#
# Run with the gmsh python of the CFD tools, e.g.
#   ~/.local/share/micromamba/envs/ignisyeet-cfd/bin/python make_basic_finner.py .
#
# Geometry (calibres, 1 cal = d = 30 mm):
#   * overall length 10.0; 20 deg cone (10 deg half angle) of length 2.84 on a cylinder of diameter 1.0;
#   * nose tip: a flat meplat of radius 0.004 cal at x = 0 (the text: "the meplat of the nose was also at a radius of
#     0.004 cal"; the figure marks it "R 0.004"), so the cone half angle is atan(0.496 / 2.84) = 9.91 deg;
#   * four rectangular fins, chord 1.0 (trailing edge flush with the base), span 1.0 from the body surface
#     (3.0 cal tip to tip), in the "+" arrangement (fins in the planes y = 0 and z = 0), no cant;
#   * fin section: a single wedge ("conical in shape"), sharp leading edge of radius 0.004 cal, thickness 0.08 cal at
#     the base of the fin (the blunt trailing edge); flat fin tip.
# --sharp leaves the meplat and the leading-edge radius out (pointed nose, knife-edge fin leading edge).
#
# Coordinates: metres in the model, x from the nose tip towards the base (the nose points to -x); the STEP declares
# millimetres (as examples/sample_rocket.step), the STL is written in millimetres (rocket.stl_scale = 0.001).
import math
import os
import sys

import gmsh

args = [a for a in sys.argv[1:] if not a.startswith("--")]
sharp = "--sharp" in sys.argv
outdir = args[0] if args else os.path.dirname(os.path.abspath(__file__))

D = 0.030  # calibre [m]
R = 0.5 * D
L = 10.0 * D
L_NOSE = 2.84 * D
RM = 0.0 if sharp else 0.004 * D  # meplat (flat nose tip) radius
FIN_C = 1.0 * D
FIN_S = 1.0 * D
FIN_T = 0.08 * D  # thickness at the base of the fin (trailing edge)
RLE = 0.0 if sharp else 0.004 * D  # fin leading-edge radius

gmsh.initialize()
gmsh.option.setNumber("General.Terminal", 0)
gmsh.option.setNumber("General.NumThreads", 1)
gmsh.model.add("basic_finner")
occ = gmsh.model.occ

# --- body of revolution: a flat meplat of radius RM at the tip (x = 0), the cone from (0, RM) to the shoulder at
# (L_NOSE, R), the cylinder to the flat base at x = L.
th = math.atan((R - RM) / L_NOSE)
p_tip = occ.addPoint(0.0, 0.0, 0.0)
p_m = occ.addPoint(0.0, RM, 0.0) if RM > 0 else p_tip
p_sh = occ.addPoint(L_NOSE, R, 0.0)
p_bc = occ.addPoint(L, R, 0.0)
p_ba = occ.addPoint(L, 0.0, 0.0)
curves = ([occ.addLine(p_tip, p_m)] if RM > 0 else []) + [occ.addLine(p_m, p_sh), occ.addLine(p_sh, p_bc), occ.addLine(p_bc, p_ba), occ.addLine(p_ba, p_tip)]
face = occ.addPlaneSurface([occ.addCurveLoop(curves)])
# Seam of the surfaces of revolution half way between two fins (not under a fin root).
occ.rotate([(2, face)], 0, 0, 0, 1, 0, 0, 0.25 * math.pi)
rev = occ.revolve([(2, face)], 0, 0, 0, 1, 0, 0, 2 * math.pi)
body = [(3, t) for d, t in rev if d == 3]

# --- one fin in the plane z = 0 (+y side): wedge section in the x-z plane, extruded along +y from inside the body
# (r = 0.5 R, so the boolean union has proper intersection curves) to the tip at r = R + FIN_S.
x_le, x_te = L - FIN_C, L
h = 0.5 * FIN_T
beta = math.atan(h / FIN_C)  # wedge half angle, measured from the sharp apex at x_le
y0 = 0.5 * R
if RLE > 0:
    # circle of radius RLE tangent to both faces; its front is at x_le.
    xcle = x_le + RLE
    # apex of the faces (virtual) such that the circle is tangent: distance apex->centre = RLE / sin(beta)
    # with the faces through the trailing-edge corners (x_te, +-h): fixed point iteration on the face angle bt.
    bt = beta
    for _ in range(50):
        xap = xcle - RLE / math.sin(bt)
        bt = math.atan(h / (x_te - xap))
    xtan = xcle - RLE * math.sin(bt)
    ztan = RLE * math.cos(bt)
    a0 = occ.addPoint(x_le, y0, 0.0)
    cc = occ.addPoint(xcle, y0, 0.0)
    up_t = occ.addPoint(xtan, y0, ztan)
    lo_t = occ.addPoint(xtan, y0, -ztan)
    te_u = occ.addPoint(x_te, y0, h)
    te_l = occ.addPoint(x_te, y0, -h)
    c1 = occ.addCircleArc(lo_t, cc, a0)
    c2 = occ.addCircleArc(a0, cc, up_t)
    sec = [c1, c2, occ.addLine(up_t, te_u), occ.addLine(te_u, te_l), occ.addLine(te_l, lo_t)]
else:
    a0 = occ.addPoint(x_le, y0, 0.0)
    te_u = occ.addPoint(x_te, y0, h)
    te_l = occ.addPoint(x_te, y0, -h)
    sec = [occ.addLine(a0, te_u), occ.addLine(te_u, te_l), occ.addLine(te_l, a0)]
sface = occ.addPlaneSurface([occ.addCurveLoop(sec)])
ext = occ.extrude([(2, sface)], 0.0, R + FIN_S - y0, 0.0)
fin0 = [(3, t) for d, t in ext if d == 3]
fins = list(fin0)
for k in range(1, 4):
    c = occ.copy(fin0)
    occ.rotate(c, 0, 0, 0, 1, 0, 0, k * 0.5 * math.pi)
    fins += c
out, _ = occ.fuse(body, fins)
occ.synchronize()
# The revolve leaves its planar source face as a free surface: keep only the solid and its boundary.
keep = set(t for _, t in gmsh.model.getBoundary(gmsh.model.getEntities(3), oriented=False))
free = [(2, t) for _, t in gmsh.model.getEntities(2) if t not in keep]
if free:
    occ.remove(free, recursive=True)
    occ.synchronize()
vols = gmsh.model.getEntities(3)
if len(vols) != 1:
    raise SystemExit("expected one solid, got %d" % len(vols))
vol = occ.getMass(3, vols[0][1])
bb = gmsh.model.getBoundingBox(3, vols[0][1])

gmsh.option.setString("Geometry.OCCTargetUnit", "M")
step = os.path.join(outdir, "basic_finner.step")
gmsh.write(step)

# --- STL tessellation (millimetres): curvature-adapted surface mesh, edges between 0.4 and 2.5 mm.
gmsh.option.setNumber("Mesh.ScalingFactor", 1000.0)  # nodes written in millimetres
gmsh.option.setNumber("Mesh.MeshSizeMax", 2.5e-3)
gmsh.option.setNumber("Mesh.MeshSizeMin", 0.4e-3)
gmsh.option.setNumber("Mesh.MeshSizeFromCurvature", 48)
gmsh.option.setNumber("Mesh.Algorithm", 6)
gmsh.option.setNumber("Mesh.Binary", 1)
gmsh.model.mesh.generate(2)
stl = os.path.join(outdir, "basic_finner.stl")
gmsh.write(stl)
ntri = sum(len(t) for t in gmsh.model.mesh.getElementsByType(2)[0:1])
print("half angle %.4f deg, meplat radius %.3f mm, faces %d, volume %.6e m^3, bbox %s, triangles %d"
      % (math.degrees(th), RM * 1e3, len(gmsh.model.getEntities(2)), vol, [round(v, 6) for v in bb], ntri))
gmsh.finalize()
