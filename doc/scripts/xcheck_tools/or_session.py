# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2024-2026 西濱大将 (NISHIHAMA Daisuke)
"""Start OpenRocket (the jar given by $OPENROCKET_JAR) headless in a JVM through JPype.

The Python package orhelper targets the pre-2022 Java package names, so the few calls it
would make are done here against OpenRocket 24.12 (info.openrocket.core.*).
"""

import os

import jpype
import jpype.imports


def start(jar=None):
    jar = jar or os.environ["OPENROCKET_JAR"]
    if jpype.isJVMStarted():
        return
    jpype.startJVM(
        jpype.getDefaultJVMPath(),
        "-Djava.awt.headless=true",
        "--enable-native-access=ALL-UNNAMED",
        f"-Djava.class.path={jar}",
    )
    from ch.qos.logback.classic import Level, Logger
    from com.google.inject import Guice
    from info.openrocket.core.plugin import PluginModule
    from info.openrocket.core.startup import Application
    from info.openrocket.swing.startup import GuiModule
    from org.slf4j import LoggerFactory

    LoggerFactory.getLogger(Logger.ROOT_LOGGER_NAME).setLevel(Level.ERROR)
    gm = GuiModule()
    Application.setInjector(Guice.createInjector(gm, PluginModule()))
    gm.startLoader()

    def private(obj, name):
        f = obj.getClass().getDeclaredField(name)
        f.setAccessible(True)
        return f.get(obj)

    private(gm, "presetLoader").blockUntilLoaded()
    private(gm, "motorLoader").blockUntilLoaded()
