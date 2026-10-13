# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
"""Faces and tessellations of examples/sample_rocket.step for the figures of the geometry chapter.

Run with the gmsh python of the CFD tools (called by geometry_data.py):

    python geometry_gmsh.py <step file> <out dir>

The file is read like the CAD input of the product (lengths in metres from the unit the file declares, OpenCASCADE
through gmsh). The fine tessellation is the one the geometry extraction cuts (longest edge extent / 400, 48 elements
per circle, surface algorithm 6, smallest edge 1/8 of the longest); its triangles aft of z = -1.28 m are kept with the
number of the face each belongs to. A coarser tessellation of the whole rocket (extent / 150, 36 elements per circle) is kept with the face
numbers as well, for a picture of the faces.
"""

import json
import sys
from pathlib import Path

import gmsh
import numpy as np

step, out = Path(sys.argv[1]), Path(sys.argv[2])
out.mkdir(parents=True, exist_ok=True)
gmsh.initialize()
gmsh.option.setNumber("General.Terminal", 0)
gmsh.option.setNumber("General.NumThreads", 1)
gmsh.option.setNumber("Mesh.MaxNumThreads1D", 1)
gmsh.option.setNumber("Mesh.MaxNumThreads2D", 1)
gmsh.option.setNumber("Mesh.MaxNumThreads3D", 1)
gmsh.option.setString("Geometry.OCCTargetUnit", "M")
gmsh.model.add("rocket")
gmsh.model.occ.importShapes(str(step), highestDimOnly=True)
gmsh.model.occ.synchronize()
vols = gmsh.model.getEntities(3)
faces = [t for _, t in gmsh.model.getBoundary(vols, oriented=False, combined=True)]
bb = gmsh.model.getBoundingBox(-1, -1)
extent = max(bb[3] - bb[0], bb[4] - bb[1], bb[5] - bb[2])


def tessellate(divisions, curvature):
    gmsh.model.mesh.clear()
    h = extent / divisions
    gmsh.option.setNumber("Mesh.MeshSizeMax", h)
    gmsh.option.setNumber("Mesh.MeshSizeMin", h / 8.0)
    gmsh.option.setNumber("Mesh.MeshSizeFromCurvature", curvature)
    gmsh.option.setNumber("Mesh.Algorithm", 6)
    gmsh.model.mesh.generate(2)
    tags, coord, _ = gmsh.model.mesh.getNodes()
    xyz = np.asarray(coord).reshape(-1, 3)
    idx = np.zeros(int(tags.max()) + 1, np.int64)
    idx[tags] = np.arange(len(tags))
    tri, fid = [], []
    for k, f in enumerate(faces):
        for et, en in zip(*gmsh.model.mesh.getElements(2, f)[::2]):
            if et == 2:
                t = idx[np.asarray(en, np.int64)].reshape(-1, 3)
                tri.append(t)
                fid.append(np.full(len(t), k, np.int16))
    return xyz, np.vstack(tri), np.concatenate(fid)


def save(path, xyz, tri, fid):
    used = np.unique(tri)
    remap = np.full(len(xyz), -1, np.int64)
    remap[used] = np.arange(len(used))
    np.savez_compressed(path, verts=xyz[used].astype(np.float32), tris=remap[tri].astype(np.int32), face=fid)


xyz, tri, fid = tessellate(400, 48)
n_fine = len(tri)
keep = xyz[tri][:, :, 2].min(axis=1) <= -1.28
save(out / "sample_cad_tail.npz", xyz, tri[keep], fid[keep])
xyz, tri, fid = tessellate(150, 36)
save(out / "sample_cad_coarse.npz", xyz, tri, fid)
info = []
for k, f in enumerate(faces):
    b = gmsh.model.getBoundingBox(2, f)
    info.append({"index": k, "tag": int(f), "type": gmsh.model.getType(2, f), "area_m2": float(gmsh.model.occ.getMass(2, f)),
                 "bbox_m": [round(float(c), 6) for c in b]})
json.dump({"solids": len(vols), "faces": len(faces), "triangles_fine": int(n_fine), "triangles_coarse": int(len(tri)),
           "extent_m": float(extent), "bbox_m": [round(float(c), 6) for c in bb], "face_list": info},
          open(out / "sample_cad_faces.json", "w"), indent=1)
print("faces", len(faces), "fine triangles", n_fine, "coarse triangles", len(tri), "tail kept", int(keep.sum()))
gmsh.finalize()
