# IgnisYeet research

| Study | Status | Report |
|---|---|---|
| NASA WIND V&V of the aerodynamic methods | in progress | `nasa_vv/report/ja/` |

## nasa_vv

Applies the NASA Glenn WIND/NPARC verification and validation tutorial
(https://www.grc.nasa.gov/www/wind/valid/tutorial/tutorial.html, following
AIAA G-077-1998) to the IgnisYeet aerodynamic methods.

Design: `../docs/superpowers/specs/2026-10-17-nasa-cfd-vv-design.md`
Plan: `../docs/superpowers/plans/2026-10-17-nasa-cfd-vv.md`

Run the tests with:

    cd research && uv run --project nasa_vv --group dev python -m pytest nasa_vv/tests/ -v
