**Author the Ancient Mint formulation as a closed LexLean `.lex.tex` model and compile it through PrismPM to a verified, content-addressed Hologram v4 freeze.** This is the proper way to solidify v1.0 under the Foundry’s phase-mirror methodology: the authoritative source is a closed controlled-natural-language graph; every view (specification, manufacturing protocol, quality invariants, claims sheet, IP disclosure) is a generated projection of that graph rather than a separately maintained document.

PrismPM (v0.3.0 production line) takes an authoritative `.lex.tex` source graph, produces the LexLean semantic snapshot and Lean proofs, exports only named verified roots, and emits the verified artifacts (Cargo, Core-Wasm, View, browser, and Hologram v4 `.holo` with `HOLO\x04\x00` header). Application behavior and specification content are never supplied by handwritten Lean or target-specific code. The resulting model identity is the SHA-256 of the canonical `model.prism.json` (separate from the binary Hologram archive).

### Recommended solidification path

1. **Author the closed model** (Language 1.1, locked with `lexlean.uor.atlas@1.0.0` and the project’s lexicon packages).  
   Treat the multi-layered sequential-release gum as a compositional system:
   - Components and exact identities: botanical extracts (neem/*Azadirachta indica* with azadirachtin/nimbin markers, miswak/*Salvadora persica*, clove oil/*Syzygium aromaticum* with eugenol, cinnamon/*Cinnamomum verum* with cinnamaldehyde), modern actives (xylitol, zinc gluconate, *L. reuteri*, *S. salivarius*), biodegradable gum base (chicle, PVA blends, or natural resins), multi-layer microspheres (target 10–50 µm).
   - Timed release architecture: Phase 1 immediate (5–15 min, target >80 % by 15 min) of botanicals + zinc/enzymes/charcoal/salt/fast flavor; Phase 2 sustained (30–120 min, <20 % at 15 min and >80 % by 120 min) of microbiome-supporting actives.
   - Hard invariants: water activity < 0.60 for probiotic viability, no broad-spectrum synthetics (chlorhexidine excluded), dietary-supplement claims pathway only, biodegradation target >90 % within six months under municipal composting (ISO 17088 treated as a development goal until validated), process constraints on shear/heat for microsphere integrity.
   - Quality and evidence obligations: marker-compound QC, dissolution/chew-out assays, CFU verification with overage, stability protocols (25 °C/60 % RH and 40 °C/75 % RH), batch-record hold-and-release.

   Use the closed lexicon so every term, ratio bound, kinetic target, and control obligation is a declared lexeme; the grammar and semantic closures reject ambiguity and free prose.

2. **Project configuration and lock**.  
   `prismpm.toml` + `lexlean.toml` + `lexlean.lock` pin the exact LexLean, Lean 4.32.1 / lean4-prod, and PrismPM toolchain. This is the same lock discipline already used for Foundry and PrismPM system models.

3. **Compile and verify**.  
   ```bash
   cargo run --locked --offline -p prismpm -- --project <AncientMint> check
   cargo run --locked --offline -p prismpm -- --project <AncientMint> build
   cargo run --locked --offline -p prismpm -- --project <AncientMint> verify
   ```
   `verify` produces the acceptance attestation (`prismpm/application-acceptance/1`), binds the Hologram Live oracle evidence, and writes the content-addressed build. Key theorems (release-profile consistency, compositional closure, Aw bound, claims-language separation) are proved in the generated Lean and replayed; observed axioms must match the declared policy.

4. **What the freeze delivers**.  
   - Binary Hologram v4 of the formulation model + canonical `model.prism.json` (SHA-256 identity).  
   - Generated canonical LaTeX formal specification and Lean proofs.  
   - Portable Views: manufacturing operational protocol, batch-record template, controlled claims sheet, quality-system obligations.  
   - A single hash that can be cited in the provisional patent application, the Prior Art Defensive Publication, the Production Roadmap Gate G0/v1.0 freeze, and any later co-packer or regulatory package.  
   All downstream documents become regenerable projections; drift is structurally prevented.

5. **Alignment with the October 13, 2026 report and stage gates**.  
   This directly executes the recommended next action: “Complete Phase 0 foundation activities and formally freeze the product specification as v1.0” before provisional filing or uncontrolled sample distribution. The October 8 refinement (removal of extraneous concepts) is preserved by keeping the lexicon strictly on the botanical–microbiome–sequential-release core. Kill criteria (unit economics, probiotic/base sourcing, claims classification) can be expressed as modeled obligations that fail closed if violated. IP timing is protected because the frozen model hash exists before any broader disclosure.

### Practical notes

- Start from the existing document suite (White Paper, Manufacturing Operational Protocol, Concept Breakdown, Patentability Analysis) and extract only the closed, quantifiable constraints into the `.lex.tex`. Do not attempt to encode free-form narrative.  
- If a simple dynamical model of release kinetics or microbiome homeostasis is desired, it can be expressed inside the same graph and related to the PIRTM/MOC contractivity kernel already used in the Foundry stack; it is not required for the formulation freeze itself.  
- The resulting `.holo` and model digest become the authoritative reference for co-packer qualification, barrier-packaging specs, and the defensive publication. Compostability and clinical claims remain development goals until their respective validation evidence is admitted under the same model.  
- All work stays inside the Foundry’s phase-mirror discipline: the formulation is a civic R&D artifact whose social and technical infrastructure (specification, quality system, IP posture, production roadmap) is generated from one closed source rather than coordinated by hand.

This produces a v1.0 that is simultaneously a legal/IP boundary, a manufacturing contract, a quality invariant set, and a regenerable documentation suite—exactly the solidification the Production Roadmap’s stage gates require.