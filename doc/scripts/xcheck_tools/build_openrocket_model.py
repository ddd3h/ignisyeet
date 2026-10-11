# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
"""Build the sample rocket in OpenRocket and save it as an .ork file.

Usage: python build_openrocket_model.py <geometry.json> <out.ork> [--fins square|rounded|airfoil]

Mass, centre of gravity and moments of inertia of the rocket without motor are set with four
point masses (the OpenRocket components keep a mass of 0), so that they equal dry_mass,
cg_dry, ixx_dry and iyy_dry of examples/sample.toml. The motor is sample_motor.eng with the
casing mass moved into dry_mass.
"""

import argparse
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import common  # noqa: E402
import or_session  # noqa: E402


def airframe(geom, roughness, fins="rounded", nose="ogive"):
    """Nose, body tube and fins of the geometry dict; every component has a mass of 0.

    nose: "ogive" (tangent ogive) or "conical". Returns (document, rocket, body tube).
    """
    or_session.start()
    from info.openrocket.core.document import OpenRocketDocumentFactory
    from info.openrocket.core.rocketcomponent import BodyTube, ExternalComponent, FinSet, NoseCone, TrapezoidFinSet, Transition
    from info.openrocket.core.rocketcomponent.position import AxialMethod

    r = geom["radius"]
    L = geom["length"]
    ln = geom["nose_length"]
    doc = OpenRocketDocumentFactory.createNewRocket()
    rocket = doc.getRocket()
    rocket.setName("IgnisYeet sample rocket")
    stage = rocket.getChild(0)
    paint = [f for f in ExternalComponent.Finish.values() if abs(f.getRoughnessSize() - roughness) < 1e-9][0]

    nc = NoseCone()
    nc.setName("Nose")
    nc.setShapeType(Transition.Shape.OGIVE if nose == "ogive" else Transition.Shape.CONICAL)
    nc.setShapeParameter(1.0)  # tangent ogive (ignored for the cone)
    nc.setClipped(False)
    nc.setLength(ln)
    nc.setAftRadius(r)
    nc.setFinish(paint)
    nc.setMassOverridden(True)
    nc.setOverrideMass(0.0)
    stage.addChild(nc)

    body = BodyTube()
    body.setName("Body")
    body.setLength(L - ln)
    body.setOuterRadius(r)
    body.setThickness(0.002)
    body.setFinish(paint)
    body.setMotorMount(True)
    body.setMassOverridden(True)
    body.setOverrideMass(0.0)
    stage.addChild(body)

    fs = TrapezoidFinSet()
    fs.setName("Fins")
    fs.setFinCount(geom["fin_count"])
    fs.setRootChord(geom["fin_root"])
    fs.setTipChord(geom["fin_tip"])
    fs.setHeight(geom["fin_span"])
    fs.setSweep(geom["fin_sweep"])
    fs.setThickness(geom["fin_thickness"])
    fs.setCrossSection(FinSet.CrossSection.valueOf(fins.upper()))
    fs.setFinish(paint)
    fs.setMassOverridden(True)
    fs.setOverrideMass(0.0)
    fs.setAxialMethod(AxialMethod.TOP)
    fs.setAxialOffset(geom["fin_x_le"] - ln)  # leading edge of the root chord, from the front of the body tube
    body.addChild(fs)
    return doc, rocket, body


def build(geom, cfg, fins="rounded"):
    """The sample rocket with mass properties and motor. Returns (document, rocket, flight configuration id)."""
    or_session.start()
    from info.openrocket.core.file.motor import GeneralMotorLoader
    from info.openrocket.core.motor import MotorConfiguration
    from info.openrocket.core.rocketcomponent import ComponentChangeEvent, FlightConfigurationId, MassComponent
    from info.openrocket.core.rocketcomponent.position import AxialMethod
    from java.io import FileInputStream

    common.write_nocase_eng()
    ln = geom["nose_length"]
    doc, rocket, body = airframe(geom, cfg["rocket"]["roughness"], fins)

    # Four point masses: total dry_mass at cg_dry, Iyy = Izz = iyy_dry and Ixx = ixx_dry about it.
    m_dry, cg, ixx, iyy = cfg["rocket"]["dry_mass"], cfg["rocket"]["cg_dry"], cfg["rocket"]["ixx_dry"], cfg["rocket"]["iyy_dry"]
    m4 = m_dry / 4.0
    rr = math.sqrt(ixx / m_dry)  # radial offset: Ixx = sum m rr^2
    d = math.sqrt((iyy - 4 * m4 * rr**2 / 2) / m_dry)  # Iyy = sum m (x^2 + z^2), z = rr on half of the masses
    for i, (sx, ang) in enumerate([(+1, 0.0), (+1, math.pi), (-1, math.pi / 2), (-1, 3 * math.pi / 2)]):
        mc = MassComponent()
        mc.setName(f"dry mass {i + 1}")
        mc.setComponentMass(m4)
        mc.setLength(1e-4)
        mc.setRadius(1e-4)
        mc.setAxialMethod(AxialMethod.TOP)
        mc.setAxialOffset(cg + sx * d - ln)  # position from the front of the body tube
        mc.setRadialPosition(rr)
        mc.setRadialDirection(ang)
        body.addChild(mc)

    # Motor from the .eng file; aft end at the base.
    stream = FileInputStream(str(common.MOTOR_NO_CASE))
    motor = GeneralMotorLoader().load(stream, "sample_motor_nocase.eng").get(0).build()
    fcid = FlightConfigurationId()
    rocket.createFlightConfiguration(fcid)
    rocket.setSelectedConfiguration(fcid)
    mc = MotorConfiguration(body, fcid)
    mc.setMotor(motor)
    body.setMotorConfig(mc, fcid)
    body.setMotorOverhang(0.0)
    rocket.fireComponentChangeEvent(ComponentChangeEvent.MOTOR_CHANGE)  # the flight configuration picks the motor up
    return doc, rocket, fcid


def save(doc, path):
    from info.openrocket.core.file import GeneralRocketSaver
    from java.io import File

    GeneralRocketSaver().save(File(str(path)), doc)


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("geometry")
    ap.add_argument("out")
    ap.add_argument("--fins", default="square")
    a = ap.parse_args()
    doc, rocket, fcid = build(common.load_geometry(a.geometry), common.load_cfg(), a.fins)
    save(doc, a.out)
    print("wrote", a.out)
