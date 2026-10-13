# ADR-AM-002: Closed CNL Solidification — Authoring Ancient Mint as an Authoritative LexLean `.lex.tex` Graph

> **Status**: Accepted  
> **Date**: 2026-10-12  
> **Authors**: Phase Mirror Formal Methods Team & Runtime Architecture  
> **Project**: Ancient Mint (Non-Compute Oral Care Formulation)  
> **Directory**: `Phase Mirror/projects/Ancient Mint`  
> **Precursor Baseline**: `ancient-mint-v1.0-sealed` (Commit `61eec2e88`, Tree `3c01ead5…`, FreeTSA Serial `0x0925C214`)  
> **Spec References**: SPEC.md §33, §34.0, ADR-011 (Mirror Dissonance), ADR-013 (L0 Contractivity Invariant), ADR-015 (On-Tree Ground Truth), ADR-AM-001 (Biochemical Stratification)

---

## 1. Context & Motivation

Under [`ADR-AM-001`](file:///media/citizen/b361d448-7c51-413a-aa23-9515cb626930/home/citizen/Multiplicity/Phase%20Mirror/projects/Ancient%20Mint/ADR-AM-001-Non-Compute-Biochemical-Stratification.md), Ancient Mint was stratified into a discrete Lean 4 transition model, admissible PIRTM kernel program, and empirical assay evidence. This baseline was sealed at commit `61eec2e88` and timestamped under RFC 3161 (`ancient_mint_tree.tsr`).

As defined in [`AM Formulation as a Closed LexLean.md`](file:///media/citizen/b361d448-7c51-413a-aa23-9515cb626930/home/citizen/Multiplicity/Phase%20Mirror/projects/Ancient%20Mint/AM%20Formulation%20as%20a%20Closed%20LexLean.md), maintaining separately handwritten Lean proofs, PIRTM kernel scripts, and documentation files creates long-term divergence debt. Under the Foundry's phase-mirror discipline:
1. Handwritten Lean and target-specific code are non-authoritative precursors.
2. The sole authoritative source must be a closed Controlled Natural Language (CNL) `.lex.tex` model locked under `lexlean.uor.atlas@1.0.0`.
3. Every downstream artifact—Lean proofs, manufacturing protocols, quality invariants, claims sheets, and IP disclosures—must be a generated projection of that single closed graph.
4. PrismPM v0.3.0 compiles this closed graph to emit the canonical `model.prism.json` and a content-addressed Hologram v4 `.holo` freeze archive.

---

## 2. Phase Mirror Dissonance Analysis

### Mirror
- The Ancient Mint formulation is a multi-layered sequential release chewing gum uniting botanical actives and modern microbiome science.
- Precursor artifacts (`AncientMint.lean`, `AncientMint.pirtm`, `ancient_mint_hplc_assay_v1.json`) established legal prior art under 35 U.S.C. §§ 102/112 and FRE 902(13)/(14).
- Solidification requires compiling the formulation into a Hologram v4 freeze.

### Dissonance
1. **Precursor Independence vs. Closed Monism**: Maintaining distinct handwritten files (`.lean`, `.pirtm`, `.json`) allows undetected semantic drift between the physical manufacturing protocol and mathematical proofs.
2. **Admissibility of Evidence**: External laboratory assay data cannot stand as free-floating JSON; it must be admitted as content-addressed evidence bound by declared model invariants.
3. **PIRTM Kernel Decoupling Invariant**: While PIRTM models the L0 contractive kernel ($|G|_1 < 1$), dropping the PIRTM layer must leave every SPEC §33 check bit-identical.

### Phase
1. Author `AncientMint.lex.tex` as a closed, zero-prose CNL graph under `lexlean.uor.atlas@1.0.0`.
2. Model all components, layer boundaries, release windows, and hard invariants as formal declarations and theorems.
3. Retain `ancient-mint-v1.0-sealed` (`61eec2e88`) as the immutable pre-CNL priority baseline, while the new Hologram v4 digest becomes the canonical Gate G1 freeze identity.

---

## 3. Authoritative Architecture & Model Boundaries

### 3.1 Closed CNL Component Declarations (`AncientMint.lex.tex`)
- **Continuous Phase (Biodegradable Base)**: Chicle, polyvinyl alcohol (PVOH) blends, natural resins; ISO 17088 municipal biodegradation (>90% within 180 days) modeled as an unverified obligation until empirical assay admission.
- **Botanical Actives (Layer 1 Payload)**:
  - Neem extract (*Azadirachta indica*) with azadirachtin/nimbin markers.
  - Miswak extract (*Salvadora persica*).
  - Clove oil (*Syzygium aromaticum*) with eugenol marker ($\le 0.10\%$ finished gum w/w).
  - Cinnamon bark (*Cinnamomum verum*) with cinnamaldehyde marker.
- **Modern Actives (Layer 2 / Core Payload)**:
  - Non-acidogenic polyol: Xylitol (250 mg).
  - Malodor chelator: Zinc gluconate (15 mg).
  - Oral Probiotic Consortium: *Lactobacillus reuteri* + *Streptococcus salivarius* ($1\text{--}10 \times 10^9\,\mathrm{CFU}$/unit).
  - Enamel remineralizing phase: nano-Hydroxyapatite (nHA).
- **Concentric Microsphere Morphology**:
  - Architecture: Core-shell-shell-shell.
  - Particle Size: $d_{50} \in [10, 50]\,\mu\mathrm{m}$, target $[25, 35]\,\mu\mathrm{m}$.
  - Polydispersity index: $\mathrm{PDI} < 0.30$.

### 3.2 Programmed Kinetic Invariants (Modeled Theorems)
- **Phase 1 Release (0–15 min)**: Cumulative botanical release $>80\%$ at 15 min ($40\text{--}60\%$ at 5 min).
- **Barrier Layer (15–30 min)**: Mechanical and salivary erosion; Phase 2 leakage $<10\%$ at 15 min.
- **Phase 2 Release (30–120 min)**: Cumulative microbiome/remineralizing release $<20\%$ at 15 min and $>80\%$ at 120 min.
- **Contractivity Invariant**: Bioavailability non-increasing ($\Delta B \le 0$).
- **Monotonic Stage Progress**: Stage rank non-decreasing ($L_1 \le L_2 \le L_3 \le \text{Terminal}$).

### 3.3 Strict Formulation Invariants
- **Water Activity**: $A_w < 0.60$ (probiotic viability threshold; violations fail closed).
- **Regulatory Pathway**: Dietary supplement / food claims only; disease treatment claims prohibited.
- **Synthetics Exclusion**: Synthetic antiseptics (chlorhexidine, triclosan) strictly excluded.

---

## 4. Solidification Pipeline & Toolchain Execution

1. **Toolchain Locks**: `prismpm.toml` and `lexlean.toml` pin LexLean, Lean 4, and PrismPM v0.3.0.
2. **Compilation Gates**:
   ```bash
   cargo run --locked --offline -p prismpm -- --project AncientMint check
   cargo run --locked --offline -p prismpm -- --project AncientMint build
   cargo run --locked --offline -p prismpm -- --project AncientMint verify
   ```
3. **Artifact Output**:
   - Hologram v4 archive: `AncientMint.holo` (prefixed `HOLO\x04\x00`).
   - Canonical Model Identity: SHA-256 of RFC-8785 canonical `model.prism.json`.
   - Generated Projections: Specification LaTeX, Lean 4 proofs, manufacturing SOP, quality control sheets.

---

## 5. Work Breakdown Structure & Verification Gates

| Step | Action | Output / Metric | Status |
|---|---|---|---|
| **5.1** | Stage, commit, and mirror `AM Formulation as a Closed LexLean.md` | SHA-256 `3b998198…` verified across mirrors | ✅ Complete |
| **5.2** | Record and accept ADR-AM-002 | Mirrored in `docs/` and `artifacts/` | ✅ Complete |
| **5.3** | Author `AncientMint.lex.tex` closed model | Closed CNL graph adhering to `lexlean.uor.atlas@1.0.0` | ⏳ In Progress |
| **5.4** | Configure project locks (`prismpm.toml`) | Locked toolchain versions | ⏳ Pending |
| **5.5** | Execute PrismPM verification | Emit `.holo` and `model.prism.json` digest | ⏳ Pending |
| **5.6** | Reconcile precursor ledger | Precursor hash `61eec2e88` supplemented by Hologram v4 digest | ⏳ Pending |
