# Outline of the research paper (shared by en/main.tex and ja/main.tex)

Both papers follow this outline section by section. Keys in `code` are the LaTeX labels; the two
papers use the same labels so that each table and figure maps one to one. The fact base is the
paper at commit e377960; no number changes, and every qualification of that paper stays.

## Title

Can an automated CAD-to-coefficient CFD pipeline quantify its own numerical uncertainty? The NASA
WIND/NPARC procedure applied to IgnisYeet (the Japanese paper uses the equivalent title; the short
title stays "V&V of the IgnisYeet aerodynamic pipeline").

## Question (one sentence)

Can an automated CAD-to-coefficient CFD pipeline, here IgnisYeet over SU2, deliver aerodynamic
coefficients with a numerical uncertainty quantified by the NASA WIND/NPARC procedure, and where it
cannot, why not?

## Answer (two sentences)

Not yet, for this pipeline: the solver with the product's settings is verified against exact
solutions, but the pipeline's own error is not quantified, on the sample rocket only one coefficient
at one condition (the Mach 2 normal-force slope, 10.6 % or 16.3 % by grid-size definition, the
latter flagged) receives a GCI that is neither refused nor unusable, the Basic Finner has no
reliable GCI with Euler (its GCIs are computed but its two triplets disagree) and none with RANS,
and the validation comparison cannot separate model error from numerical error. The causes, in the order of how much
of the study each blocks, are that the product's mesher fixes the largest cell size so grids cannot
be refined similarly, that solver convergence is not robust enough for the gate (RANS stalls or
limit cycles, the layer-1 fine level; causes not established), and that even a converged GCI is conditional, as the
exact-solution layer shows.

## Sections

### Abstract
Message: question, approach in three layers, answer, the three causes. Same numbers as e377960.

### 1 Introduction `sec:intro`
Message: an automated pipeline returns coefficients with no statement of trust; the NASA procedure
is the standard way to attach one, but applying it when the user does not control the mesh is not
routine; we ask whether it can be done and find that, for this pipeline, not yet.
- Background: automated CFD in rocket design tools (IgnisYeet as the instance, no external
  citation for the trend); coefficients feed flight simulation.
- The standard procedure: AIAA G-077-1998, NPARC tutorial, Roache GCI, Celik et al. 2008.
- Gap: the procedure assumes control of the grid family and of convergence.
- The question, explicit.
- Approach: three layers (solver, pipeline, application).
- Findings preview: four bullets matching R1–R4.
- Terminology: verification, cross-comparison, validation (検証, 相互比較, 妥当性確認).
- Carrier: none (prose).

### 2 Method `sec:method`
Message: what is needed to read the results: the three layers and their cases, the GCI with its
acceptance gate, and the two grid-size definitions forced by non-similar grids.
- 2.1 Three layers and the six steps `sec:layers`. Message: layers 0 and 1 solve the same exact
  cone by two routes so that their difference would isolate the pipeline; layer 2 applies the six
  steps. Carrier: `tab:layers` (single column: layer, what is run, kind of comparison).
- 2.2 The GCI and the gate `sec:gci`. Message: a number is reported only if every level passes
  the convergence gate, the differences are monotone and the order lies in 0.5–4. Carrier:
  equation `eq:celik` and the three rules.
- 2.3 Cases `sec:cases`. Message: the exact solutions are computed and checked; the layer-1 and
  layer-2 families are built through the product's own settings. Carrier: prose; grid details in
  Appendix A.

### 3 Results `sec:results`

#### 3.1 R1. The solver and settings are verified, but the GCI is not a guarantee `sec:r1`
Message: SU2 with the product's settings reproduces exact inviscid solutions to within 0.2 % on
surface pressure and shock angle, but where the exact answer is known the GCI bounded the true
error in only 5 of 7 inviscid and 2 of 5 laminar-plate rows, and the available checks do not
detect the failures.
- 3.1.1 Layer 0 against exact solutions `sec:l0results`. Carrier: `tab:l0` (merged: inviscid and
  laminar-plate rows, finest error, order, GCI, bounds, window orders where computed). Wall Mach
  errors, wall entropy and tip junction condensed to one paragraph; mass conservation one sentence.
- 3.1.2 Does the GCI bound the true error? `sec:gcibound`. Carrier: `tab:l0` and the window-order
  column (former `tab:detect`, merged). Plate bounds only at sign crossings; plate residual effects
  (i)–(v) condensed.
- 3.1.3 Side finding: the cone10 reference table `sec:cone10`. Carrier: prose (former
  `tab:cone10` cut to numbers in text).
- Closes: a GCI computed without an exact answer is a conditional estimate.

#### 3.2 R2. The pipeline's own error could not be quantified `sec:r2`
Message: through the product's chain the cone's finest level did not converge, so there is no
layer-1 GCI and no pipeline error; independently, the product's grids cannot be refined similarly,
which limits every product GCI.
- 3.2.1 Layer 1: the finest level did not converge `sec:layer1`. Carrier: `tab:layer1` and
  `fig:layer1cp`.
- 3.2.2 Product grids cannot be refined similarly `sec:similar`. Carrier: `tab:growth` (cell or
  node growth per level of every product family against 2.74; numbers from e377960).
- Closes: the non-convergence (cause not tested) and the non-similarity (structural) are separate
  limits.

#### 3.3 R3. In application, almost no usable uncertainty band `sec:r3`
Message: on the sample rocket the six steps leave one GCI that is neither refused nor unusable,
the Mach 2 normal-force slope, which is wide and of poor reliability; the Basic Finner Euler study
computes GCIs but its two triplets disagree, so it establishes no reliable one; there is no RANS GCI.
Scope matters: "the only GCI" is said of the sample rocket, never of layer 2 as a whole.
- 3.3.1 Spatial convergence `sec:grid`. Carrier: `tab:sample`. Basic Finner Euler (former
  `tab:finner`) condensed to text: triplets disagree, ±0.17 % sensitivity, +0.47 % change. Basic
  Finner RANS (former `tab:rans`) condensed to text: 2° solves not converged on every level.
  Existing RANS sample family refused; its published GCI does not meet the precondition (not a
  claim that it is wrong).
- 3.3.2 Iterative error and consistency `sec:iter`, `sec:consistency`. Carrier: prose. Only one
  condition can be compared; its iterative error is a lower bound. C_N(0)=0 passes by mesh
  symmetry; asymmetric meshes fail; |C_A| at Mach 0.3 shrinks but its extrapolation crosses zero.
- 3.3.3 Code examination and temporal convergence `sec:step1`, `sec:temporal`. Message: step 1
  adds tests; step 5 does not apply to steady CFD, and the flight integrator is treated in
  Appendix B. Carrier: prose.
- Closes: almost no usable band.

#### 3.4 R4. Validation and model uncertainty cannot be separated from numerical uncertainty `sec:r4`
Message: the Euler CNα lies 2.9–10.5 % below free-flight data, and neither this difference nor the
Euler–RANS or SA–SST differences can be attributed to the model, because the matching numerical
bands are missing or flagged.
- 3.4.1 Validation against free-flight data `sec:validation`. Carrier: `fig:validation` and
  `tab:validation`.
- 3.4.2 Euler against RANS `sec:eulerrans`. Carrier: prose (seven conditions, no verdict).
- 3.4.3 Spalart–Allmaras against SST `sec:model`. Carrier: prose (flat plate, SU2 directly; SA
  against TMR as a cross-comparison, including the existing turbulent-plate data).
- Closes: no comparison of this study can say which side is wrong.

### 4 Discussion `sec:discussion`
- 4.1 The answer `sec:answer`. Message: not yet, for this pipeline, apart from one wide band of poor
  reliability on the sample rocket.
- 4.2 Root causes, ranked `sec:causes`. Message: (1) the mesher fixes the largest cell size;
  (2) convergence robustness (RANS stalls and limit cycles, layer-1 fine level, causes untested);
  (3) a GCI without an exact answer is conditional (layer-0 evidence).
- 4.3 What the findings imply `sec:implications`. Message, worded as implications and not as tested
  fixes: a global refinement control that scales the whole field; convergence handling that
  separates stalls from limit cycles and applies one residual measure; reporting the band only when
  the gate passes, with its flags.
- 4.4 Findings about the product `sec:product`. One paragraph: published RANS GCI precondition,
  cut surface output, mesher abort with small fins, Taylor–Maccoll bracket, zero-length bar.
- 4.5 Limitations `sec:limits`. The e377960 limits paragraph plus the single measured data set,
  the SA-only product and the self-built plate grids.

### 5 Conclusions `sec:conclusions`
Five sentences that answer the question.

### Data availability
results/*.csv, figures/, the code, and the full catalogue in the web report
research/nasa_vv/report/ja/report.md.

### Appendix A. Details of the GCI procedure and the grids `sec:appA`
Acceptance tests of the GCI code (tutorial example, Celik Table 1 including the inconsistent
cell); layer-0, layer-1 and layer-2 grid and setup details that the results do not need.

### Appendix B. The flight integrator `sec:temporalapp`
Step refinement of the six-degree-of-freedom integrator: no order on the sample rocket; the rail
hold as the main cause on the evidence of controls; a second contribution not excluded; the floor.

## Tables and figures kept

| Key | Shows |
|---|---|
| `tab:layers` | The three layers: what is run and the kind of comparison. |
| `tab:l0` | Layer 0 against exact answers: finest error, order, GCI, whether it bounds the error, window orders (merges the former `tab:l0`, `tab:detect` and the bounds part of `tab:plate`). |
| `tab:layer1` | Layer 1 levels: residual drop, solver verdict, Cp difference; layer-0 row for contrast. |
| `fig:layer1cp` | Layer 1 binned wall Cp of the three levels against Taylor–Maccoll; no band. |
| `tab:growth` | Growth per refinement level of every product grid family against 2.74. |
| `tab:sample` | Sample rocket Euler: order and GCI under both grid-size definitions. |
| `fig:validation` | Basic Finner CNα and CX0 against free-flight data; the only band is a ±0.17 % sensitivity. |
| `tab:validation` | Basic Finner measured coefficients, probable errors, CFD values and differences. |

## Cut, and where it lives instead

| Cut from the paper | Lives in |
|---|---|
| `tab:cone10` (as a table; key numbers kept in text) | report.md §2.2; results/cone10_reference_check.csv, cone_shock_angle_sources.csv |
| `tab:detect` (merged into `tab:l0`) | results/layer0_gci.csv |
| `tab:plate` coarse/medium/extrapolated columns | report.md §2.6; results/layer0_plate.csv, layer0_gci.csv |
| Plate effects (i)–(v) in detail (domain extents, leading-edge spike values, Cf√Re_x stations) | report.md §2.6; results/layer0_plate.csv |
| `tab:turbref` and the correlation comparisons | report.md §2.7; results/gci_existing.csv |
| Wall-entropy values per grid | report.md §2.4; results/layer0.csv |
| Mass-flux values per grid | report.md §4.3; results/massflux.csv |
| `tab:finner` (Basic Finner Euler orders and GCI per triplet) | report.md §4.4; results/gci_existing.csv |
| `tab:rans` (Basic Finner RANS residual drops per solve) | report.md §4.4; results/basic_finner_rans_levels.csv |
| Iterative-error detail (warm-start stops, per-row ratios) | report.md §4.2; results/iterative.csv |
| Consistency detail on the asymmetric meshes (per-row values) | report.md §4.3; results/consistency.csv |
| Cut surface output detail (counts of complete rows) | report.md §3.3; results/layer1_cone.csv |
| Flight-integrator detail beyond Appendix B | report.md §4.5; results/temporal.csv |
| Separate "Wall quantities", "Reference data" discussion paragraphs | folded into R1 and the limitations |
