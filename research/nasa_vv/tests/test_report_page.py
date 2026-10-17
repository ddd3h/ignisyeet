# SPDX-License-Identifier: GPL-3.0-or-later
"""The report page is generated from report.md and the committed figures.

doc/source/research/nasa-vv/index.rst and the figure copies under doc/source/_static/research/nasa-vv/
must be what report/ja/build_page.py produces now, so that a change to the report text or a figure
cannot leave the published page behind.
"""
import importlib.util
import pathlib
import re

import pytest

pytest.importorskip("markdown_it")

BUILD = pathlib.Path(__file__).resolve().parents[1] / "report" / "ja" / "build_page.py"


@pytest.fixture(scope="module")
def build_page():
    spec = importlib.util.spec_from_file_location("build_page", BUILD)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_the_committed_page_is_what_the_generator_produces(build_page):
    assert build_page.PAGE.read_text(encoding="utf-8") == build_page.render()


def test_the_generator_is_deterministic(build_page):
    assert build_page.render() == build_page.render()


def test_the_committed_figure_copies_equal_the_figures(build_page):
    figures = sorted(build_page.FIGURES_FROM.glob("*.svg"))
    assert figures
    for svg in figures:
        assert (build_page.FIGURES_TO / svg.name).read_bytes() == svg.read_bytes()


def test_every_image_of_the_page_points_at_a_copied_figure(build_page):
    text = build_page.PAGE.read_text(encoding="utf-8")
    sources = re.findall(r'<img src="([^"]+)"', text)
    assert sources
    page_dir = build_page.ROOT / "doc/source/research/nasa-vv"
    static = build_page.ROOT / "doc/source/_static"
    for src in sources:
        # the built page sits at <site>/research/nasa-vv/index.html, two levels below the site root
        assert src.startswith("../../_static/")
        assert (static / src[len("../../_static/"):]).is_file()
    assert page_dir.is_dir()


def test_the_page_is_orphan_and_wrapped_for_the_shared_top_bar(build_page):
    text = build_page.PAGE.read_text(encoding="utf-8")
    assert text.startswith(":orphan:\n")
    assert '<div class="iy iy-report">' in text
    assert not (build_page.ROOT / "doc/extra").exists()
