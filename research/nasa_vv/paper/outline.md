# Outline of the research paper (shared by en/main.tex and ja/main.tex)

Both papers follow this outline section by section. Keys in `code` are the LaTeX labels; the two
papers use the same labels so that each table and figure maps one to one. The fact base is the
papers at commit 44cf91c (which carry the final review of
.superpowers/sdd/2026-10-17-nasa-cfd-vv/paper-final-review.md); no number changes, every
qualification stays, and every number keeps its `% source:` comment, also where it moves to an
appendix.

Reading rule for every results subsection and for the abstract: **what we found → the one key
piece of data → why it matters for the question.** Exceptions, extra checks and secondary numbers
go to Appendix A (or stay in the web report); the body carries the main line only. No paragraph of
the body is a list of numbers.

## Title

Can an automated CAD-to-coefficient CFD pipeline quantify its own numerical uncertainty? The NASA
WIND/NPARC procedure applied to IgnisYeet (the Japanese paper uses the equivalent title; the short
title stays "V&V of the IgnisYeet aerodynamic pipeline").

## Question (one sentence)

Can an automated CAD-to-coefficient CFD pipeline, here IgnisYeet over SU2, deliver aerodynamic
coefficients with a numerical uncertainty quantified by the NASA WIND/NPARC procedure, and where it
cannot, why not?

## Answer (two sentences)

Not yet, for this pipeline: in verification against exact solutions, SU2 with the product's
settings reproduces the inviscid surface pressure and shock angle within 0.2 % in the primary
setups, but the pipeline's own error is not quantified, on the sample rocket only one coefficient
at one condition (the Mach 2 normal-force slope, 10.6 % or 16.3 % by grid-size definition, the
latter flagged) receives a GCI that is neither refused nor unusable, the Basic Finner has no
reliable GCI with Euler (its GCIs are computed but its two triplets disagree) and none with RANS,
and the validation comparison cannot separate model error from numerical error. In the study's
judgement three obstacles stand behind this, ranked by how general and how permanent each is: the
product's mesher fixes the largest cell size so grids cannot be refined similarly (every family,
and it would remain if every solve converged); solver convergence is not robust enough for the gate
(RANS stalls or limit cycles, the layer-1 fine level; causes not established), which removes a GCI
wherever it occurs; and even a converged GCI is conditional, as the exact-solution layer shows. Both
the attribution and the ranking are judgement, not measurement: the study shows that the obstacles
exist, not that each one caused a particular failure.

## Sections

### Abstract
About 180–220 words (EN), the same content in JA. Order:
1. Significance and question: an automated CFD tool returns coefficients without a statement of
   trust; the question is whether the NASA procedure can quantify how far they can be trusted.
2. Approach in one sentence: the procedure applied in three layers (solver, whole chain,
   application).
3. The answer: the solver reproduces exact inviscid pressures (scoped as in review A3: inviscid
   surface pressure and shock angle within 0.2 % in the primary setups), but a reliable
   uncertainty for the automated chain as a whole was not obtained.
4. A few key numbers only: GCI bound held in 5 of 7 inviscid rows; no layer-1 GCI (finest level not
   converged); one sample-rocket band, the Mach 2 normal-force slope, 10.6 or 16.3 % (the latter
   flagged); no reliable Basic Finner GCI, so the free-flight comparison reports sizes, not
   verdicts.
5. The three obstacles, "in our judgement", with "for reasons not established" on convergence.
6. Significance in one sentence, worded as what the findings point to, not as a demonstration.
- Dropped from the abstract (kept in the body or an appendix): the laminar-plate counts, the
  0.8–3.6 % wall-Mach errors, the seven Euler–RANS conditions, the Basic Finner Mach numbers and
  the NPARC side finding.

### 1 Introduction `sec:intro`
Message: an automated pipeline returns coefficients with no statement of trust; the NASA procedure
is the standard way to attach one, but it presumes control of the grid family and of convergence;
we ask whether it can be done without that control and find that, for this pipeline, not yet.
- Background: automated CFD in rocket design tools, with IgnisYeet as the instance (called the
  product); coefficients feed flight simulation. No external citation for a trend; the paper does
  not claim one.
- The standard procedure: AIAA G-077-1998, NPARC tutorial, Roache GCI, Celik et al. 2008.
- **Related work** (one paragraph, every citation verified; see the reference list below):
  - established: the GCI and V&V procedures (Roache 1998; Celik et al. 2008; ASME V&V 20-2009;
    the review of code and solution verification by Roy 2005; the book of Oberkampf & Roy 2010);
    Eça & Hoekstra 2014 extend the estimate to grid-refinement data with scatter by least-squares
    fits, but still require solutions on systematically refined grids;
  - how small-rocket design tools get coefficients: semi-empirical methods such as the Barrowman
    method (Barrowman 1967), used in extended form by OpenRocket (Niskanen 2009), and Missile
    DATCOM (Blake 1998); IgnisYeet runs CFD automatically instead (stated as the instance, no
    trend claim);
  - the gap, stated as scope and not as a priority claim: these procedures are written for an
    analyst who builds the grid family and drives each solution; this paper applies them end to
    end to an automated, CAD-driven pipeline in which the user controls neither. The scope sentence
    stays: no survey of earlier grid-convergence studies on automatic or unstructured meshes; one
    application to one pipeline. No "first" and no "we are not aware of".
- The question, explicit.
- Approach: three layers, pointing to `fig:concept`.
- Findings preview: four bullets matching R1–R4.
- Terminology: verification, cross-comparison, validation (検証, 相互比較, 妥当性確認).

### 2 Method `sec:method`
Message: what is needed to read the results: the three layers and their cases, the GCI with its
acceptance gate, and the two grid-size definitions forced by non-similar grids.
- Software version (user request, both papers): the runs made for this study use IgnisYeet v0.4.0
  (source commit a8dff9c) with SU2 8.3.0, except one coarse-level rerun with another build, named
  in §3.3.1; the product studies called "existing" are re-evaluated from result files already
  committed with the product, without new CFD. `% source:` the headers of
  results/basic_finner_rans_levels.csv (binary built from a8dff9c, `--version` prints 0.4.0) and
  results/gci_existing.csv (no CFD run for that file). Data availability repeats the version.
- 2.1 Three layers and the six steps `sec:layers`. Message: layers 0 and 1 solve the same exact
  cone by two routes so that their difference would expose the error the chain adds; layer 2
  applies the six steps. Carriers: `fig:concept` and `tab:layers`.
- 2.2 The GCI and the gate `sec:gci`. Message: a number is reported only if every level passes
  the convergence gate, the differences are monotone and the order lies in 0.5–4. Carrier:
  equation `eq:celik` and the three rules.
- 2.3 Cases `sec:cases`. Message: the exact solutions are computed and checked; the layer-1 and
  layer-2 families are built through the product's own settings. Carrier: prose; grid details in
  Appendix A.

### Concept figure `fig:concept`
TikZ, drawn in the paper (vector), labels in each paper's language, `figure*` across both columns
at the top of the page after the Introduction starts. Three boxes left to right, joined by arrows
"same exact cone" (0 → 1) and "same product" (1 → 2). Each box has a heading, what is run, the
question (italic) and the outcome (bold):

| Layer | What is run | Question it answers | Outcome |
|---|---|---|---|
| 0 Solver | SU2 8.3.0 with the product's settings, against exact solutions | Does the solver reproduce exact solutions? Does the GCI bound the true error? | Inviscid surface pressure and shock angle within 0.2 % (primary setups); the GCI bound failed in 2 of 7 rows |
| 1 Whole chain | The product's chain, CAD to coefficients, on the layer-0 cone | What error does the chain add? | Not quantified: finest level not converged, no layer-1 GCI |
| 2 Application | The six steps on a sample rocket and on the Basic Finner; free-flight range data for the Basic Finner | Which coefficients get a band, and what can validation conclude? | Almost no usable band (one, wide); differences reported, no verdict |

No flat "verified" label (review A3); no "real rockets against flight data" (only the Basic Finner
has measurements).

### 3 Results `sec:results`

#### 3.1 R1. The solver reproduces exact inviscid pressures, but the GCI is not a guarantee `sec:r1`
- 3.1.1 Layer 0 against exact solutions `sec:l0results`. Finding: pressure and shock angle within
  0.2 % in the primary setups. Data: `tab:l0` (the individual errors are in the table, not
  repeated in prose). Why: the base for the later layers, for inviscid pressure, in these setups.
  Exceptions in one sentence (wall-Mach quantities, boundary junctions; details in Appendix A).
- 3.1.2 Does the GCI bound the true error? `sec:gcibound`. Finding: not always. Data: 5 of 7
  inviscid rows; cone wall Cp error 3.2 times its GCI; neither check detects both failures. Why:
  every later band is a GCI without an exact answer. Laminar plate in two or three sentences (2 of
  5, only at sign crossings; no clean verification); its residual effects to Appendix A.
- Side note in one or two sentences, labelled as such: the NPARC 10° cone table disagrees with
  itself, so the cone reference is computed (detail in Appendix A, `sec:cone10`).
- Closes ("What R1 establishes"): a GCI computed without an exact answer is a conditional
  estimate.

#### 3.2 R2. The pipeline's own error could not be quantified `sec:r2`
- 3.2.1 Layer 1: the finest level did not converge `sec:layer1`. Finding first; data
  `tab:layer1`, `fig:layer1cp`; why: the pipeline error cannot be formed. The tip-bin values, the
  rank counts and the list of other layer differences go to Appendix A (one sentence keeps that the
  layers differ in more than the chain).
- 3.2.2 Product grids cannot be refined similarly `sec:similar`. Carrier `tab:growth`.
- Closes with the A6 sentence: the study can give no number for the error the pipeline adds.

#### 3.3 R3. In application, almost no usable uncertainty band `sec:r3`
- 3.3.1 Spatial convergence `sec:grid`. Finding: one sample-rocket GCI (Mach 2 normal-force
  slope, 10.6 / 16.3 %, flagged on one definition). Data: `tab:sample`. Basic Finner Euler in two
  sentences (triplets disagree; ±0.17 % sensitivity against a +0.47 % change); per-triplet orders
  to Appendix A. RANS: the gate result, the A7 stall and limit-cycle definitions, the residual-
  measure sentence and the flipped verdicts stay in the body (cited by §4.2 and §4.4); the
  existing RANS sample-rocket study refused in one sentence.
- 3.3.2 Iterative error and consistency `sec:iter`, `sec:consistency`. Finding and key data only:
  no matched row above 1, only one comparable condition, a lower bound; symmetric meshes pass by
  construction, asymmetric ones fail (the A10 "24 rows (12 of C_N(0), 12 of the even part)" form
  stays); d'Alembert shrinks but does not demonstrably converge. Row counts, flat-plate rows and
  factors to Appendix A.
- 3.3.3 Code examination and temporal convergence `sec:step1`, `sec:temporal`. Two or three
  sentences: step 1 adds tests; step 5 does not apply to steady CFD, side note pointing to
  Appendix B.
- Closes with the A6 sentence: almost no usable band.

#### 3.4 R4. Validation and model uncertainty cannot be separated from numerical uncertainty `sec:r4`
- 3.4.1 Validation against free-flight data `sec:validation`. Carriers `fig:validation`,
  `tab:validation`.
- 3.4.2 Euler against RANS `sec:eulerrans`. Finding: none of seven conditions has a verdict; key
  datum: differences up to +54 %; the per-condition values go to Appendix A.
- Side note (labelled): Spalart–Allmaras against SST, one or two sentences in the body (−0.79 % in
  skin friction, not separable from SST's GCI of 1.14 %; cross-comparison, not validation); the
  full comparison, including the TMR cross-comparison, in Appendix A (`sec:model`).
- Closes: no comparison of this study can say which side is wrong.

### 4 Discussion `sec:discussion`
- 4.1 The answer `sec:answer`. Not yet, for this pipeline, apart from one wide band of poor
  reliability on the sample rocket.
- 4.2 Three obstacles, ranked `sec:causes`. Ranked by how general and how permanent each is,
  attribution and ranking stated as judgement.
- 4.3 Existence, not cause `sec:existence`. New. The study measured that each obstacle exists
  (`tab:growth`; the recorded stalls and limit cycles; the layer-0 bound failures). It did not
  show that any obstacle caused a particular failure: that non-similarity makes the Basic Finner
  triplets disagree or the sample-rocket slope fail to converge at Mach 0.3 and 0.8, and why the
  solves stall, were not tested.
- 4.4 What the findings imply, and the next experiment `sec:implications`. The three implications
  (global refinement control; convergence handling that separates stalls from limit cycles and
  uses the weakest residual; the band only when the gate passes, labelled conditional), none
  implemented or tested. Future work: the decisive experiment is to give the mesher a global
  refinement control that scales the whole field, build a similar family on the same cases, and
  test whether its GCI then passes the gate; for the second obstacle, to test the dependence on
  rank count and decomposition.
- 4.5 Limitations `sec:limits`. As at 44cf91c.
- 4.6 Significance `sec:significance`. New, closing paragraph: an automated CFD tool must return
  not only a number but a way to judge its reliability; the findings point to what that would
  take (worded as "points to", not "shows").
- The five findings about the product move to Appendix C, `sec:product`, with one pointer
  sentence in §4.

### 5 Conclusions `sec:conclusions`
Five steps, one paragraph:
1. Problem: an automated tool returns numbers whose reliability is unknown.
2. Aim: test whether the NASA procedure can quantify it.
3. Result: SU2 with the product's settings reproduces exact inviscid pressures (scoped as in A3);
   the whole chain's uncertainty was not quantified (one wide band on the sample rocket, none
   reliable on the Basic Finner).
4. Obstacles: meshing, convergence, the GCI's preconditions (judgement; existence shown, cause of
   each failure not).
5. Significance, with the implications stated as untested.

### Data availability
results/*.csv, figures/, the code, the full catalogue in the web report
research/nasa_vv/report/ja/report.md, and the software version (IgnisYeet v0.4.0, commit a8dff9c;
SU2 8.3.0).

### Appendix A. Details of the GCI procedure, the grids and the secondary results `sec:appA`
Acceptance tests of the GCI code; layer-0, layer-1 and layer-2 grid and setup details. Gains the
secondary results moved from the body, each with its `% source:` comment:
- layer 0: wall-Mach errors, wall entropy, junction values, mass conservation, plate residual
  effects;
- the NPARC 10° cone table (`sec:cone10`);
- layer 1: tip-bin values, rank counts, the other layer differences;
- layer 2: Basic Finner Euler orders per triplet; iterative-error row counts and the flat-plate
  rows; the consistency factors; the seven Euler–RANS differences;
- Spalart–Allmaras against SST and the TMR cross-comparison (`sec:model`).

### Appendix B. The flight integrator `sec:temporalapp`
Unchanged.

### Appendix C. Findings about the product `sec:product`
The five findings of the former §4.4, unchanged in wording.

## References added in revision 2 (each checked; sources in the EN report)

| Key | Reference |
|---|---|
| `asme2009` | ASME V&V 20-2009, Standard for Verification and Validation in Computational Fluid Dynamics and Heat Transfer, ASME, 2009 |
| `roy2005` | C. J. Roy, Review of code and solution verification procedures for computational simulation, J. Comput. Phys. 205(1), 131–156, 2005, doi:10.1016/j.jcp.2004.10.036 |
| `oberkampf2010` | W. L. Oberkampf and C. J. Roy, Verification and Validation in Scientific Computing, Cambridge University Press, 2010, doi:10.1017/CBO9780511760396 |
| `eca2014` | L. Eça and M. Hoekstra, A procedure for the estimation of the numerical uncertainty of CFD calculations based on grid refinement studies, J. Comput. Phys. 262, 104–130, 2014, doi:10.1016/j.jcp.2014.01.006 |
| `barrowman1967` | J. S. Barrowman, The Practical Calculation of the Aerodynamic Characteristics of Slender Finned Vehicles, MSc thesis, Catholic University of America, 1967 (NASA/TM-2001-209983) |
| `niskanen2009` | S. Niskanen, Development of an Open Source model rocket simulation software, MSc thesis, Helsinki University of Technology, 2009 |
| `blake1998` | W. B. Blake, Missile DATCOM: User's Manual – 1997 FORTRAN 90 Revision, AFRL-VA-WP-TR-1998-3009, 1998 |

## Tables and figures kept

| Key | Shows |
|---|---|
| `fig:concept` | New: the three layers, the question each answers, the outcome. |
| `tab:layers` | The three layers: what is run and the kind of comparison. |
| `tab:l0` | Layer 0 against exact answers: finest error, order, GCI, whether it bounds the error, window orders. |
| `tab:layer1` | Layer 1 levels: residual drop, solver verdict, Cp difference; layer-0 row for contrast. |
| `fig:layer1cp` | Layer 1 binned wall Cp of the three levels against Taylor–Maccoll; no band. |
| `tab:growth` | Growth per refinement level of every product grid family against 2.74. |
| `tab:sample` | Sample rocket Euler: order and GCI under both grid-size definitions. |
| `fig:validation` | Basic Finner CNα and CX0 against free-flight data; the only band is a ±0.17 % sensitivity. |
| `tab:validation` | Basic Finner measured coefficients, probable errors, CFD values and differences. |

## Moved from the body to an appendix in revision 2

| Moved | To |
|---|---|
| Wall-Mach, wall-entropy and junction detail; mass conservation | Appendix A, layer 0 |
| Laminar-plate residual effects | Appendix A, layer 0 |
| NPARC 10° cone table (former §3.1.3) | Appendix A, `sec:cone10`; one side-note sentence in R1 |
| Layer-1 tip bins, rank counts, other layer differences | Appendix A, layer 1 |
| Basic Finner Euler orders per triplet | Appendix A, layer 2 |
| Iterative-error row counts, flat-plate rows; consistency factors | Appendix A, layer 2 |
| The seven Euler–RANS differences | Appendix A, layer 2 |
| Spalart–Allmaras against SST and TMR (former §3.4.3) | Appendix A, `sec:model`; one side note in R4 |
| Findings about the product (former §4.4) | Appendix C, `sec:product` |

## Cut, and where it lives instead

| Cut from the paper | Lives in |
|---|---|
| `tab:cone10` (as a table; key numbers kept in Appendix A) | report.md §2.2; results/cone10_reference_check.csv, cone_shock_angle_sources.csv |
| `tab:detect` (merged into `tab:l0`) | results/layer0_gci.csv |
| `tab:plate` coarse/medium/extrapolated columns | report.md §2.6; results/layer0_plate.csv, layer0_gci.csv |
| Plate effects (i)–(v) in detail (domain extents, leading-edge spike values, Cf√Re_x stations) | report.md §2.6; results/layer0_plate.csv |
| `tab:turbref` and the correlation comparisons | report.md §2.7; results/gci_existing.csv |
| Mass-flux values per grid | report.md §4.3; results/massflux.csv |
| `tab:finner` (Basic Finner Euler orders and GCI per triplet, as a table) | report.md §4.4; results/gci_existing.csv |
| `tab:rans` (Basic Finner RANS residual drops per solve) | report.md §4.4; results/basic_finner_rans_levels.csv |
| Iterative-error detail (warm-start stops, per-row ratios) | report.md §4.2; results/iterative.csv |
| Consistency detail on the asymmetric meshes (per-row values) | report.md §4.3; results/consistency.csv |
| Cut surface output detail (counts of complete rows) | report.md §3.3; results/layer1_cone.csv |
| Flight-integrator detail beyond Appendix B | report.md §4.5; results/temporal.csv |
| SA–SST setup detail (numerics, y⁺, stop rules) | report.md §4.6.1; results/turbulence_models.csv |
