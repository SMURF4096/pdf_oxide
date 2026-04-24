# DocumentBuilder — v0.3.39 expansion plan

**Input:** 26 gap items identified in the post-#393 audit, grouped
into 4 tiers (see conversation `2026-04-23` and earlier `docs/v0.3.39/
research/d_builder_gap_analysis.md`).

**Goal per user request:** plan shipping all 4 tiers in v0.3.39. This
doc presents the full plan **plus** an honest sub-release split so the
decision of "ship one huge release" vs "ship three themed releases"
can be made with numbers in view.

**Status:** PLAN ONLY — no implementation work in this commit.

---

## Executive summary

- **26 items** across 4 tiers, grouped into **7 bundles** (A–G).
- **Honest effort estimate for the full set: 15–20 developer-weeks**,
  assuming solo work at the velocity observed during #393 (22 commits,
  ~2 days calendar with parallel agents).
- Shipping everything as v0.3.39 is possible but creates a release ~7×
  larger than v0.3.39 today (tables alone). Review burden, user-facing
  breakage surface, and CI-flake risk scale with size.
- **Recommended split** below divides the work into v0.3.39 (already
  done) + v0.3.40 + v0.3.41 + v0.3.42. Each release is coherent, each
  has a clear theme, each is shippable in 1–2 weeks of focused work.
- This document describes both options. Decision belongs to the
  maintainer.

---

## Bundle map — 7 themed bundles covering all 26 items

Items grouped by what makes sense to ship together (users either use
all of a bundle or none).

### Bundle A — "Images + transforms" (**4 commits estimated**)

| # | Item                                | Backing impl? | LOC est |
|---|-------------------------------------|---------------|---------|
| 1 | `page.image(source, rect)` + variants | `ImageData::{from_file, from_bytes, from_jpeg, from_png}` exists | ~150 Rust + ~50/binding |
| 7 | `rotate()`, `scale()`, `translate()`, `clip()` | Partial: `TextContent.matrix` field exists | ~200 Rust + ~50/binding |

**Dependencies:** none (independent of all other bundles).
**Risk:** transforms need careful spec work — PDF's `cm` operator vs
per-element matrix needs a design choice (see Open Questions below).

### Bundle B — "Navigation + document structure" (**3 commits**)

| # | Item                                     | Backing impl?           | LOC est |
|---|------------------------------------------|-------------------------|---------|
| 2 | `doc.bookmark()` / outline fluent chain  | `OutlineBuilder` works  | ~100 Rust + ~30/binding |
| 6 | `doc.with_page_labels(ranges)`           | `PageLabelsBuilder` works | ~40 Rust + ~20/binding |
|   | ToC generator helper (auto from bookmarks) | Needs design             | ~120 Rust + ~40/binding |

**Dependencies:** ToC generator depends on #2.
**Risk:** low — existing impls are mature.

### Bundle C — "Shape primitives + dash patterns" (**2 commits**)

| # | Item                                       | Backing impl?        | LOC est |
|---|--------------------------------------------|----------------------|---------|
| 5 | `circle`, `ellipse`, `polygon`, `arc`, `bezier_curve` | `PathOperation::CurveTo` exists | ~80 Rust + ~30/binding |
| 13| Dash patterns on `LineStyle`               | Content-stream writer has no `SetDashPattern` op — add | ~60 core + 30 line-style + ~20/binding |

**Dependencies:** none.
**Risk:** low. Dash-pattern already has a #400 entry to inherit design from.

### Bundle D — "Forms complete" (**6 commits**)

| # | Item                                      | Backing impl?                  | LOC est |
|---|-------------------------------------------|--------------------------------|---------|
| 3 | `page.list_box(name, rect, options, ...)` | `ListBoxWidget` fully implemented | ~30 Rust + ~40/binding |
| 4 | `page.signature_field(name, rect, ...)`   | Partial — leverage `src/signatures/signer.rs` placeholder | ~120 Rust + ~50/binding |
| 17| Fluent `.required()`, `.read_only()`, `.tooltip(s)` on all 6 field methods | Backing structs have the fields | ~60 Rust + ~30/binding |
| 20| `doc.set_tab_order([...])` / `page.tab_order([...])` | `/Tabs` dict in PageData needed | ~70 Rust + ~30/binding |
| 22| Barcode form-field auto-generator         | `BarcodeGenerator` exists; needs "form-bound" wrapper | ~100 Rust + ~40/binding |
| 18| Field validation — regex mask, numeric range | Need Action/JS layer | ~180 Rust + ~50/binding |

**Dependencies:** #17 is prerequisite for #18's "required" marker UX.
**Risk:** medium — field validation needs JavaScript-action wiring
(infrastructure exists for link actions, not yet for field actions).

### Bundle E — "Rich text + layout primitives" (**5 commits**)

| # | Item                                | Backing impl?      | LOC est |
|---|-------------------------------------|--------------------|---------|
| 8 | Rich text inline (`bold(...)`, `italic(...)`, `color(...)` within `.text()`)  | Needs TextRun accumulator | ~200 Rust + ~60/binding |
| 9 | Bullet + numbered lists             | None — greenfield    | ~150 Rust + ~40/binding |
| 10| Code blocks (mono font + bg fill)   | Compose from existing| ~60 Rust + ~20/binding  |
| 11| Multi-column flow on `DocumentBuilder` | Exists in `html_css/layout`; needs port | ~250 Rust + ~60/binding |
| 12| Footnotes / endnotes                | None — needs cross-page bookkeeping | ~200 Rust + ~50/binding |

**Dependencies:** #8 is prerequisite for #9 (list item text runs).
**Risk:** **high**. Rich text + multi-column flow require a block-level
layout model that pdf_oxide currently only has in `html_css/layout`.
Porting that into `DocumentBuilder` is the biggest scope in the plan.

### Bundle F — "Accessibility / PDF/UA" (**4 commits + research**)

| # | Item                                      | Prior work?       | LOC est |
|---|-------------------------------------------|-------------------|---------|
|   | RESEARCH DOC: PDF/UA compliance mapping   | None              | docs only |
| 23| Tagged PDF / logical structure tree       | Partial — structure elements exist in `pdf_writer` | ~300 Rust + ~80/binding |
| 24| Language tags per content run             | Needs `/Lang` plumbing | ~50 Rust + ~20/binding |
| 25| Artifact marking (headers/footers as artifacts) | Partial — `artifact_type` field on TextContent exists | ~80 Rust + ~40/binding |
| 26| Role mapping for non-standard structure   | Needs `/RoleMap` dict | ~60 Rust + ~20/binding |

**Dependencies:** all depend on the research doc landing first.
**Risk:** **high**. PDF/UA is a compliance standard with sub-profiles
(PDF/UA-1, PDF/UA-2). A credible implementation needs its own `#393`-
scale research pass before any code is written.

### Bundle G — "Advanced forms" (**2 commits — recommend DEFER**)

| # | Item                              | Backing impl? | LOC est |
|---|-----------------------------------|---------------|---------|
| 19| Calculated fields / JavaScript actions | None — needs JS-action layer | ~400 Rust + ~100/binding |
| 21| XFA write-side                    | Read-side only in `src/xfa/` | ~600 Rust + ~120/binding |

**Recommendation:** file as issues for a later release (v0.3.42 or
later). XFA is deprecated in PDF 2.0; JavaScript calculations are
niche and security-sensitive.

---

## Dependencies between bundles

```
A (images + transforms) ──────────────┐
                                      │
B (navigation) ───────────────────────┤
                                      ├─→ shippable independently
C (shapes + dashes) ──────────────────┤
                                      │
D (forms) ────────────────────────────┘

E (rich text + layout) ──── depends on A for image-in-text + transforms
                            for rotated text blocks

F (accessibility) ────────── SHOULD come after E, because tagging only
                             makes sense once the content model is
                             stable (lists, multi-column, etc.)

G (advanced forms) ──────── standalone, LOW priority
```

A / B / C / D are all independent — any subset is a valid release.
E depends on A. F depends on E for meaningful structure tagging.

---

## Cross-cutting work per bundle

Every bundle ships across all 6 bindings per the v0.3.38 memory rule
("all 7 bindings every time"). Per-bundle cost:

| Bundle | Rust core | FFI | Python | WASM | C# | Go | Node | Total commits |
|--------|----------:|----:|-------:|-----:|---:|---:|-----:|--------------:|
| A      |         2 |   1 |      1 |    1 |  1 |  1 |    1 |             8 |
| B      |         2 |   1 |      1 |    1 |  1 |  1 |    1 |             8 |
| C      |         1 |   1 |      1 |    1 |  1 |  1 |    1 |             7 |
| D      |         3 |   2 |      1 |    1 |  1 |  1 |    1 |            10 |
| E      |         4 |   2 |      1 |    1 |  1 |  1 |    1 |            11 |
| F      |         3 |   1 |      1 |    1 |  1 |  1 |    1 |             9 |
| G      |         2 |   1 |      1 |    1 |  1 |  1 |    1 |             8 |

Plus:
- CHANGELOG updates: 1 commit per release.
- README examples per bundle: 1 commit per bundle.
- Benchmark work where applicable (esp. B's ToC, E's multi-column).

### Velocity note

During #393 (tables), we landed 22 commits in ~2 days of calendar with
5 parallel binding agents for step 6. That velocity assumes:

- The Rust core is a clear single-person task.
- Per-binding work is parallelisable via agents.
- The researcher + planner + Rust-core author is the same person (me).

At that velocity each bundle above is roughly 1–3 days calendar. Seven
bundles × 2 days avg = **~2 weeks calendar for the full plan**.

Reality will be 50 % slower — research docs (esp. Bundle F), review
cycles, fix-ups, test flakes. So realistic calendar is **3–4 weeks for
all 4 tiers**, not 15–20 weeks. (The 15–20-week number at the top of
this doc was a conservative "nobody parallelises" estimate.)

---

## Option 1 — ship everything as v0.3.39

**Stated user goal.** All 4 tiers land in v0.3.39.

### What the branch looks like

Current: 22 commits for tables.
After: ~22 + ~60 = **~82 commits**.

### Timeline

~3–4 weeks calendar with parallel agents + focused work. No shipping
of v0.3.39 in the meantime.

### Risks

1. **One huge PR to review.** Tables alone is already 9,096 lines; +60
   more commits likely pushes the diff to 30–40 kLoC. PR review
   becomes impractical without splitting.
2. **CI flake amplification.** The v0.3.38 post-merge CI showed 3
   infra flakes in 110 jobs (#399). Scaling to 60 more commits on the
   same branch multiplies the chance of rebase-level conflicts with
   dependabot / main-branch security patches during the build-up.
3. **Single shot of tables gets delayed.** The #393 motivation was a
   concrete user pain (MigraDoc 30k rows). Holding tables hostage to
   accessibility research is the opposite of the release pattern
   v0.3.38 set ("ship the verified slice, file the rest").
4. **PDF/UA research (Bundle F) is a real blocker.** We don't yet have
   enough prior art in-repo to write the PDF/UA research doc without
   at least 2 days of focused reading. That serialises the release.

### Mitigation

- Split the WIP branch into per-bundle stack of branches (`release/v0.3.39-bundle-A`, etc.) and squash-merge bundle-by-bundle. Same release, segmented review.
- Freeze main except for dependabot during the build-up.
- Commit the accessibility research doc EARLY so code authoring can start before it's fully approved.

---

## Option 2 — themed release split (recommended)

### v0.3.39 ships as-is (tables — already done)

Already on `release/v0.3.39`. Ready to push + PR.

### v0.3.40 — "DocumentBuilder completeness"

Bundles A + B + C + D-minimal (items 3, 4, 17 — list_box, signature
widget, field metadata). ~25 commits. 1–2 weeks.

Why these together: all independent, all Tier 1 or low-effort Tier 3,
all deliver "the builder can actually do what you expect from a
builder now" UX wins. Includes images (the single biggest Tier 1
gap), bookmarks, shapes, list_box, signature widget.

Tracked in #400 today; split into #401 (images), #402 (navigation),
#403 (shapes), #404 (forms-minimal) if the split reads better.

### v0.3.41 — "Rich text + layout primitives"

Bundle E (items 8–12). ~11 commits. 1–2 weeks.

Why its own release: touches the biggest single scope item (multi-
column flow + rich text model). Needs prototype work before commit to
an API shape. Users get it as a coherent "DocumentBuilder can now be
used like Typst or QuestPDF for rich content" story.

### v0.3.42 — "PDF/UA-ready DocumentBuilder"

Bundle F (items 23–26) + the research doc. ~9 commits + docs.
2–3 weeks (research-heavy).

Why its own release: accessibility compliance is a standalone value
prop. Government / enterprise customers buy on the strength of
"PDF/UA-1 conformance shipped"; mixing it in with generic feature
growth dilutes the messaging.

### Later — "Advanced forms" (Bundle G)

Items 19 (calc fields + JS actions) and 21 (XFA write-side). File as
issues; pick up only if real customer demand. Note: JavaScript
actions in PDF have security implications that merit their own
research pass.

### Timeline — option 2

| Release | Weeks from now | Scope                             |
|---------|---------------:|-----------------------------------|
| v0.3.39 | 0 (already done) | Tables                          |
| v0.3.40 | 1–2             | Images + navigation + shapes + forms-minimal |
| v0.3.41 | 3–4             | Rich text + layout primitives     |
| v0.3.42 | 5–7             | PDF/UA                            |
| later   | —               | Advanced forms                    |

Total: ~6–8 weeks to close every Tier 1–4 item; users get working
value every 1–2 weeks instead of waiting 3–4 weeks for a single
megarelease.

---

## Open questions — answer before implementation starts

1. **Transforms API shape (Bundle A).** Do we ship `.rotate(deg).text(...)`
   where the transform is STATEFUL on the builder, or `.rotated_text(deg, text, ...)`
   variants that transform per-element? The former is closer to PDF's
   `q/Q` graphics-state stack; the latter is easier to reason about.
   Needs a one-paragraph RFC before Bundle A commits.

2. **Field validation format (Bundle D / Tier 3).** Do we ship a
   declarative validation object (`Validation::Regex(r"^\d{5}$")`) or
   a JS-string escape hatch (`.validate_js("function() { ... }")`)?
   Declarative is safer; JS is more flexible. Probably ship both.

3. **Rich text accumulator (Bundle E).** Do we introduce a new
   `TextRun` type that `.text()` / `.bold()` / `.italic()` append to,
   flushed at `.paragraph()` / `.done()`? That's the natural shape
   but has lifetime + fluent-chain consequences — study how Typst's
   `TextNode` or iText's `Paragraph.Add(Chunk)` models it.

4. **Accessibility (Bundle F).** Do we target PDF/UA-1 or PDF/UA-2?
   UA-2 is 2024 and stricter; most tooling still validates against
   UA-1. Probably ship UA-1 first, upgrade in a minor release.

5. **Binding parity ceiling for Bundle G.** If we ever ship XFA write-
   side or JS actions, do they go to all 6 bindings (expensive) or
   just Rust + Python + C# (the three most likely to see enterprise
   XFA use)? v0.3.38 discipline says all 6; pragma says fewer.

---

## Recommendation (maintainer's-seat)

Take **Option 2**. Specifically:

1. Push `release/v0.3.39` today as a PR against main; merge when CI is
   green. Users get tables.
2. Open a new tracking issue (`v0.3.40 — DocumentBuilder completeness`)
   that links to Bundle A + B + C + D-minimal, with this doc as the
   scope anchor.
3. Work Bundle C first (shortest, unblocks dashed table rules), then
   A (biggest user ask — images), then B, then D-minimal. Each bundle
   a separate branch + PR.
4. After v0.3.40 lands, open v0.3.41 + v0.3.42 tracking issues and
   start the rich-text / accessibility research docs respectively.
5. Leave Bundle G open-ended — file issues #408 #409 and let demand
   pull them forward.

---

## If we DO go with Option 1

Concrete ordering within v0.3.39 if we chose to cram everything in:

```
week 1  — bundles C + A   (shapes + images, easy wins to build momentum)
week 2  — bundles B + D-minimal (nav + list_box/signature_field/metadata)
week 3  — bundle E        (rich text + layout — biggest single chunk)
week 3  — bundle F research doc (parallel, no code)
week 4  — bundle F implementation (accessibility)
week 5  — FINAL: CHANGELOG, README, retest, prepare PR
```

Total: ~5 weeks calendar, ~80 commits on the branch.

This is doable but the release PR will be ~40 kLoC with ~80 commits
— approach the ceiling of what a single reviewer can absorb in a
reasonable timeframe. Recommend the split per Option 2.

---

## Open-ended items after all 4 tiers close

Even after Tiers 1–4 are all done, gaps still exist that we saw during
the audit but don't rise to Tier 1:

- Table-of-contents auto-generation from heading hierarchy (called out
  in Bundle B but limited to hand-declared bookmarks).
- Embedded files / attachments fluent surface (backing infrastructure
  in `src/writer/embedded_files.rs` already exists).
- Named destinations fluent surface.
- Optional Content Groups (layers — `src/writer/layers/`).
- 3D / movie annotations (we have backing — `src/writer/movie.rs`,
  `src/writer/threed.rs`).

These accumulate as "builder-surface drift" — the v0.3.40 completeness
work should include an audit pass to keep the drift under control.

---

## Next step

Decide: Option 1 or Option 2. The plan above handles either. If you
pick Option 1, I start on Bundle C first (matches the "momentum"
ordering above); if you pick Option 2, I file the v0.3.40 tracking
issue and we push v0.3.39 as-is.
