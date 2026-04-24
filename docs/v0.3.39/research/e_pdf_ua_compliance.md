# PDF/UA compliance for DocumentBuilder — research

Audit target: `release/v0.3.39`. Scope supporting issue
[#393 Bundle F-0](../design/builder_gaps_plan.md) — the research prerequisite
for commits 23–26 (tagged structure, language tags, artifact marking, role
mapping). Research-only; no code changes.

## Summary

### Target: PDF/UA-1 only for v0.3.40

- **PDF/UA-1 (ISO 14289-1:2014)** is what every validator on the market
  actually checks today: veraPDF (main rule set), Adobe Acrobat Pro
  Accessibility Checker, PAC 2021 / PAC 2024 (axes4 / PDF/UA Foundation),
  Callas pdfGoHTML, Foxit PDF Accessibility.
- **PDF/UA-2 (ISO 14289-2:2024)** was published in Feb 2024 and requires
  PDF 2.0 as a base. Tooling is still catching up — veraPDF shipped a
  preliminary UA-2 profile in 2024 but it is not the default. Adobe
  Accessibility Checker does not yet verify UA-2. Enterprise
  requirements (US Section 508 refresh, EN 301 549) still map to UA-1.
- **Recommendation:** target **PDF/UA-1** for v0.3.40. Declare
  `pdfuaid:part = 1` in XMP. Defer UA-2 to a later minor release once
  PAC 2024 / veraPDF UA-2 profiles stabilise and once we have PDF 2.0
  output (we currently emit PDF 1.7 — `src/writer/pdf_writer.rs:54`).

### Compliance matrix (writer side — what DocumentBuilder emits)

| Requirement                                                  | Currently emitted? | Source                                            |
|--------------------------------------------------------------|--------------------|---------------------------------------------------|
| `/Type /Catalog` with `/MarkInfo <</Marked true>>`           | No                 | `src/writer/pdf_writer.rs:1496-1513`              |
| `/StructTreeRoot` in catalog                                 | No                 | ditto                                             |
| `/Lang` in catalog                                           | No                 | ditto                                             |
| `/ViewerPreferences <</DisplayDocTitle true>>`               | No                 | ditto                                             |
| `/Metadata` XMP stream with `pdfuaid:part`                   | No                 | `src/writer/xmp_metadata.rs` exists; not wired in |
| BDC/EMC marked-content wrapping per content item             | **Yes** (partial)  | `src/writer/content_stream.rs:1121-1154`          |
| MCID allocation                                              | Yes (counter)      | `src/writer/content_stream.rs:1100-1105`          |
| Structure element objects with `/K` → MCID back-refs         | No                 | (nothing writes them)                             |
| Structure element tree (Document → Sect → P / H1 …)          | No                 | `StructureElement` type exists; not serialised    |
| `/RoleMap` for custom types                                  | No                 | n/a                                               |
| `/Artifact BDC ... EMC` brackets for headers/footers         | No (write-side)    | read-side parses it at `extractors/text.rs:4622`  |
| Figure `/Alt` text                                           | Field exists       | `src/elements/mod.rs:159`                         |
| Per-run `/Lang`                                              | No                 | `TextContent` has no `lang` field                 |
| `/StructParents` on each page / `/StructParent` on annots    | No                 | n/a                                               |

### Scope for v0.3.40 Bundle F

- **F-1** — structure tree emission: serialise `StructureElement` into
  real indirect-object `/StructElem` nodes, emit `/StructTreeRoot`,
  wire MCID-to-structure back-refs via `/ParentTree`, set
  `/MarkInfo <</Marked true>>`.
- **F-2** — `/Lang` plumbing: document-level `/Lang` on the catalog
  plus a `lang: Option<String>` field on `TextContent` emitted as
  `/Lang` on the enclosing Span element.
- **F-3** — `/Artifact BDC/EMC` brackets for `TextContent.artifact_type
  == Some(_)`: write-side counterpart of the extractor that already
  reads them.
- **F-4** — `/RoleMap` for custom structure types; validate standard
  types at build time.
- **F-0** (this doc) — research + compliance-matrix table.

### Scope deferred to v0.3.41+

- Full **PDF/UA-2** conformance (requires PDF 2.0 output, `/StructParent
  Tree` constraints, stronger nesting rules, Namespace support).
- **Table** header/cell association — `/Headers`, `/Scope`, `/ID`
  attributes on `<TH>`/`<TD>` structure elements. Bundle E table work
  must land first; this becomes a follow-up.
- **Link** structure tags (`/Link` parent with `/OBJR` child pointing
  at the link annotation) — coupled to Bundle B navigation work.
- **Form field** accessibility (`/TU` tooltip, `/Form` structure
  elements) — coupled to existing AcroForm work in `src/form_fields/`.
- **Tab order** `/Tabs /S` on page dict — requires structure tree
  first; the enum value already exists at
  `src/writer/document_builder.rs:1121`.
- **Automatic heading-level policing** — validator-side, out of scope
  for the writer.
- **Colour-contrast hinting** — requires rendering pipeline.

## Prior art

### pikepdf (Python, QPDF wrapper)

pikepdf does **not** provide a high-level tagged-PDF authoring API. It
exposes `Pdf.root.StructTreeRoot` and lets the caller hand-craft the
`/StructElem` objects via its object-graph API. All MCID accounting,
parent-tree construction, and role-mapping are the author's problem.
Their docs explicitly recommend iText or Adobe's API for tagged-PDF
generation.

### pdfrw (Python, pure)

No tagged-PDF authoring support whatsoever. pdfrw is read-and-rewrite
oriented; structure trees survive a round-trip but new structure must
be built with raw dictionaries. Deprecated for new work; the community
moved to pikepdf and reportlab.

### ReportLab (Python, commercial core)

ReportLab's Platypus flow engine has a **partial** tagged-PDF mode
(`Canvas(beginMarkedContent=True)` plus `drawString` auto-tagged as
`<Span>`). Structure tree is produced automatically for Platypus
`Paragraph`, `Table`, `Image` flowables. PDF/UA validation with
veraPDF passes at the "no crashes" level but fails the Matterhorn
semantic rules (no `<Sect>` grouping, heading levels not checked).
Good *shape* to copy — fluent API that tags at the flowable level.

### iText (Java / .NET, AGPL + commercial)

Gold standard. `com.itextpdf.layout` produces PDF/UA-1 compliant output
by default when `PdfDocument` is created with a `PdfUAConformanceLevel`.
Every `Paragraph`, `Table`, `Image`, `Link` flowable in iText 8 carries
a `TagTreePointer` that points into the `PdfStructTreeRoot`; MCID
allocation is automatic; `/Artifact` marking happens when you call
`setMarkedAsArtifact(true)`. iText also exposes `setRole(PdfName)` and
`setLanguage(String)` on any element.

Relevant API surface to mirror:

- `IAccessibleElement.getAccessibilityProperties()` returns
  `AccessibilityProperties` with `setRole`, `setLanguage`, `setAlternateDescription`, `setActualText`, `setExpansion`, `setStructureElementIdString`, `addRef`.
- `PdfUAConformanceLevel.PDF_UA_1` is a constructor flag.
- `PdfDocument.setTagged()` turns structure-tree emission on.

### QuestPDF (C#, MIT)

Added tagged-PDF support in late 2023 (`AccessibleElement.Tag("H1")`
and `.AlternateText("...")`). Compliance is **not** validated — they
emit the BDC/EMC and structure tree but don't check Matterhorn. Good
**naming** to copy: `.Tag(StructureType.H1)`, `.AlternateText(...)`,
`.Language("en-US")`.

### PDFBox (Java, Apache)

Low-level: `PDStructureElement`, `PDStructureTreeRoot`,
`PDArtifactMarkedContent`. No auto-tagging; the caller builds the
tree. Similar level to pikepdf. Useful reference for **serialisation
patterns** of the structure tree and parent-tree numbering.

### printpdf / lopdf (Rust)

- `printpdf` has no tagged-PDF support.
- `lopdf` is object-level only; structure must be hand-authored.
- **typst-pdf** (inside Typst compiler) emits UA-1 output — the
  relevant code lives in `typst-pdf/src/tagging.rs` and demonstrates
  MCID counter + parent-tree algorithm in Rust. Worth reading as a
  pure-Rust reference implementation. Typst's model is tag-by-element
  at flowable level, same shape as our `StructureElement` already is.

## What we already have

Concrete code references in the current repo:

### Structure types and marked-content ops

- `src/elements/mod.rs:149-175` — `StructureElement { structure_type,
  bbox, children, reading_order, alt_text, language }`. Already carries
  `alt_text` and `language` fields. `children: Vec<ContentElement>` is
  recursive. **Default `Default` impl exists.**
- `src/elements/mod.rs:55-66` — `ContentElement::Structure(StructureElement)`
  variant so the whole tree is first-class in the unified element model.
- `src/writer/content_stream.rs:96-105` — `ContentStreamOp::BeginMarkedContentDict { tag, mcid }` and `::EndMarkedContent` variants; serialised as `/P <</MCID N>> BDC` … `EMC` at `src/writer/content_stream.rs:1280-1283`.
- `src/writer/content_stream.rs:281` — `mcid_counter: u32` on
  `ContentStreamBuilder`.
- `src/writer/content_stream.rs:1100-1105` — `next_mcid() -> u32`
  allocates the next MCID per content stream. Note: MCIDs are
  **per-page** in PDF — the current builder increments per
  content-stream, which matches if each page is one stream (today's
  reality).
- `src/writer/content_stream.rs:1121-1154` — `add_structure_element`
  already wraps children in BDC/EMC and recurses into nested
  `ContentElement::Structure`. **Critical finding:** this is a
  self-contained marked-content wrapper with MCID allocation but
  **nothing emits the matching `/StructElem` objects or
  `/StructTreeRoot`**. MCIDs are orphaned.

### Artifact plumbing (read-side only)

- `src/extractors/text.rs:1887-1911` — `ArtifactType { Pagination(PaginationSubtype), Layout, Page, Background }` enum with Header/Footer/Watermark/PageNumber/Other subtypes matches ISO 32000-1 §14.8.2.2 exactly.
- `src/elements/text.rs:28` — `TextContent.artifact_type: Option<ArtifactType>` field already carried through the writer data model.
- **Write-side gap:** `ContentStreamBuilder` never emits
  `/Artifact BDC … EMC` — greps for `"Artifact"` in `src/writer/`
  return zero matches outside dead imports.

### Catalog emission

- `src/writer/pdf_writer.rs:1496-1535` — Catalog object assembled from
  `Type`, `Pages`, `AcroForm?`, `Outlines?`, `PageLabels?`. No
  `MarkInfo`, `StructTreeRoot`, `Lang`, `Metadata`, or
  `ViewerPreferences`.
- `src/writer/xmp_metadata.rs` exists but is not referenced from
  `pdf_writer.rs::finish()`.

### Existing validator (read side)

- `src/compliance/pdf_ua.rs` is a 868-line **validator** (reads a PDF
  and reports UA violations). Useful as a self-test oracle — once the
  writer emits UA-1, we pipe DocumentBuilder output through the
  validator in a `tests/` integration check.
  - `pdf_ua.rs:500` — checks for `/StructTreeRoot`.
  - `pdf_ua.rs:477-499` — checks `/MarkInfo << /Marked true >>`.
  - `pdf_ua.rs:527-536` — checks `/Lang` in catalog.
  - `pdf_ua.rs:216-318` — `UaErrorCode` enum (48 codes) already
    covers the Matterhorn rule families.
- `src/compliance/validators.rs:300` — coarse-grained `has_structure_tree` check used in cross-validation.

### Structure tree reader

- `src/structure/types.rs` — `StructTreeRoot`, `StructElem`, `StructChild`
  types (read side). These are the **parsed** form, not the
  serialisation form.
- `src/structure/parser.rs` — 852 LoC read-side structure parser.
  Useful as a shape reference for what we need to emit.
- `src/structure/builder.rs` — 137-line builder, but it builds the
  **read** model (for round-trip), not a writer.

## What Bundle F must add

### F-1: structure tree emission (~300 LoC Rust + 80/binding)

Per-page and document-level changes to `PdfWriter::finish`:

1. Aggregate all `ContentElement::Structure` elements across pages
   into a single document-order tree rooted at a synthetic `/Document`
   structure element.
2. Allocate indirect-object IDs for each `/StructElem` node.
3. Emit each `/StructElem` as:
   ```
   << /Type /StructElem
      /S /P                      % structure type
      /P 7 0 R                   % parent structure element
      /Pg 12 0 R                 % page that contains the marked content
      /K [<<MCID-ref>> or <<StructElem-ref>>]
      /Alt (alt text)            % optional
      /Lang (en-US)              % optional
      /ActualText (...)          % optional
   >>
   ```
4. For the MCID back-reference, `/K` entries take two shapes:
   - An integer N when the child is a marked-content sequence on the
     parent page (most common).
   - A dictionary `<< /Type /MCR /Pg … /MCID N >>` when the marked
     content is on a different page or we want to be explicit.
5. Emit `/StructTreeRoot`:
   ```
   << /Type /StructTreeRoot
      /K <root-struct-elem-ref>
      /ParentTree <num-tree-ref>
      /ParentTreeNextKey N
      /RoleMap <<...>>           % optional, see F-4
   >>
   ```
6. Emit `/ParentTree` as a number tree mapping `StructParents`
   integer on each page → array of `/StructElem` references
   (parent of each MCID in that page). Each page dict gets
   `/StructParents N`.
7. Set catalog `/MarkInfo <</Marked true>>` and
   `/StructTreeRoot <ref>`.

**Required structure types (ISO 32000-1 §14.8.4, Standard Structure
Types):**

| Type        | Use                                              |
|-------------|--------------------------------------------------|
| `Document`  | Root grouping element. Mandatory.                |
| `Part`      | Major division.                                  |
| `Sect`      | Section / subsection.                            |
| `P`         | Paragraph. Default for `.paragraph()` /         |
|             |  `.text()`.                                      |
| `H1`–`H6`   | Headings. Mapped from `.heading(level, text)`.   |
| `L`         | List container.                                  |
| `LI`        | List item. `/Lbl` + `/LBody` children.           |
| `Lbl`       | List item label / bullet / numeral.              |
| `LBody`     | List item body text.                             |
| `Figure`    | Image / figure. Requires `/Alt`.                 |
| `Caption`   | Figure / table caption.                          |
| `Table`     | Table.                                           |
| `TR`        | Table row.                                       |
| `TH`        | Table header cell. `/Scope` required.            |
| `TD`        | Table data cell.                                 |
| `THead`/`TBody`/`TFoot` | Table row groups (optional).          |
| `Link`      | Hyperlink. Contains `/OBJR` pointing at link annotation. |
| `Span`      | Inline span (e.g. `/Lang`-tagged run).           |
| `Note`      | Footnote / endnote.                              |
| `Reference` | Citation.                                        |

These are all **standard types** — no `/RoleMap` entry required. For
anything else, see F-4.

**MCID contract** (PDF 1.7 §14.7.4 + §14.8.4):

- Every piece of visible page content that contributes to reading
  must live inside exactly one `/Tag <</MCID N>> BDC … EMC` sequence.
- MCID N must be **unique per page** (not per document).
- The MCID N must be referenced from exactly one `/StructElem` in the
  tree (via the `/ParentTree` number tree on that page's
  `/StructParents` entry).
- Content that is **not** part of the logical structure (decorative
  shapes, headers, footers, watermarks, page numbers) goes in
  `/Artifact BDC … EMC` sequences and is **not** referenced from the
  tree. See F-3.

### F-2: `/Lang` per run (~50 LoC Rust + 20/binding)

1. Add `DocumentBuilder::language(&str)` — emits `/Lang (xx-YY)` in
   the catalog. RFC 3066 / BCP 47 tag (e.g. `"en-US"`, `"de"`).
2. Add `TextContent.lang: Option<String>` field. Missing → inherits
   from ancestor structure element or catalog.
3. When a structure element's `/Lang` differs from its parent, emit
   `/Lang` on the `/StructElem` dictionary. For inline per-run
   `/Lang` (e.g. a French word inside an English paragraph), wrap
   the run in a `Span` structure element with `/Lang`.

**PDF/UA-1 rule 7.2 (Matterhorn 11-004):** the document must have a
natural language specified. Rule 11-005: every piece of content that
differs in language must be tagged.

### F-3: `/Artifact` BDC/EMC for headers/footers (~80 LoC Rust + 40/binding)

Write-side mirror of the existing read-side parser.

1. Add `ContentStreamOp::BeginArtifact { subtype: Option<String>, attached: Option<String>, artifact_type: Option<String> }` and `ContentStreamOp::EndArtifact` (aliases of EMC with a distinct marker for our own sanity).
2. Serialise as:
   ```
   /Artifact << /Type /Pagination /Subtype /Header >> BDC
   ...
   EMC
   ```
3. `FluentPageBuilder` / `PageTemplate` for headers+footers already
   identifies them structurally. Wire `PageTemplate::header(...)` and
   `PageTemplate::footer(...)` to emit `/Artifact` brackets
   automatically (see `src/writer/page_template.rs`).
4. Honour `TextContent.artifact_type` on user-added content when
   present (users can explicitly mark decorative content).
5. Artifact content must **not** get an MCID and must **not** appear
   in the structure tree. Matterhorn 1.03 rule 01-005 — artifact
   content inside structure tree is a failure.

### F-4: `/RoleMap` for custom types (~60 LoC Rust + 20/binding)

1. Add `DocumentBuilder::role_map(custom: &str, standard: &str)` —
   records a mapping. Standard type validated at call-site against the
   Standard Structure Type list.
2. Emit `/RoleMap << /Quote /Span /Aside /Sect >>` on the
   `/StructTreeRoot`.
3. Validate: every structure type appearing in the tree that isn't
   standard **must** appear as a `/RoleMap` key. Otherwise fail the
   build (compile-time error, not a runtime PDF).

## Matterhorn Protocol 1.03 mapping

Matterhorn Protocol 1.03 (2021) is the PDF/UA Foundation test suite.
50 % of checks are machine-checkable (veraPDF automates these); 50 %
require human review (reading order, alt-text quality).

| Rule       | Description                                              | F-N    | Status after v0.3.40         |
|------------|----------------------------------------------------------|--------|------------------------------|
| 01-003     | PDF not tagged (MarkInfo/Marked not true)                | F-1    | Pass                         |
| 01-004     | Logical structure not reliable                           | F-1    | Pass (semantic tagging)      |
| 01-005     | Content marked as artifact but in structure tree         | F-3    | Pass (builder enforces)      |
| 01-006     | Content in structure tree but should be artifact         | F-3    | User responsibility          |
| 01-007     | Suspected content inconsistencies                        | F-1    | Pass                         |
| 02-001     | Character encodings not reliable                         | FONT   | Already passes               |
| 04-001     | Role map invalid                                         | F-4    | Pass                         |
| 04-002     | Role map loops                                           | F-4    | Pass (validated)             |
| 04-003     | Standard structure types only (or role-mapped)           | F-1/F-4| Pass                         |
| 07-001     | `/Alt` missing on Figure                                 | F-1    | Pass (required field)        |
| 07-002     | Figure alt text meaningless                              | —      | User responsibility          |
| 09-001     | `/H` heading without `/H1`–`/H6`                         | F-1    | Pass (we use H1–H6 directly) |
| 09-004     | Heading levels skip                                      | —      | Validator warning only       |
| 10-001     | Table `<TH>` missing `/Scope`                            | DEFER  | v0.3.41                      |
| 10-002     | Table header–cell association                            | DEFER  | v0.3.41                      |
| 11-001     | Non-Latin text without Unicode mapping                   | FONT   | Already passes               |
| 11-004     | Catalog `/Lang` missing                                  | F-2    | Pass                         |
| 11-005     | Content with different language not tagged               | F-2    | Pass                         |
| 13-004     | Structure element nesting invalid                        | F-1    | Pass (builder enforces)      |
| 14-003     | `<Link>` structure element without `/OBJR`               | DEFER  | v0.3.41                      |
| 17-002     | Annotations not in tab order                             | DEFER  | v0.3.42 (coupled with D-4)   |
| 17-003     | `/Tabs /S` missing on page                               | F-1    | Pass (set when structure on) |
| 19-003     | `DisplayDocTitle` not true                               | F-1    | Pass (set in catalog)        |
| 19-004     | Document title missing                                   | meta   | Already plumbed              |
| 20-001     | XMP `pdfuaid:part` missing                               | F-1    | Pass (emit XMP)              |

Other Matterhorn rules (06 fonts, 15 lists, 18 multimedia, 21 colour,
22 contents, 23 security, 25 JavaScript, 26 forms, 30 OCProperties)
either already pass (fonts, security) or are out of scope for the
writer (user-content quality).

## Implementation plan

### F-1: structure tree emission

- **Files touched:**
  - new: `src/writer/struct_tree.rs` (~180 LoC) — builds
    `/StructTreeRoot`, `/StructElem` nodes, `/ParentTree` number tree.
  - `src/writer/pdf_writer.rs` — wire struct_tree output; add
    `/MarkInfo` and `/StructTreeRoot` to catalog; add `/StructParents`
    to each page.
  - `src/writer/document_builder.rs` — add
    `.tagged(true)` / `.tagged_pdf_ua1()` opt-in.
  - `src/writer/content_stream.rs` — make MCID allocation emit a
    back-ref record keyed by page index, not a bare counter.
- **Per-binding surface:**
  - Python: `DocumentBuilder.tagged_pdf_ua1()` kwarg or method.
  - WASM: same.
  - C FFI: `pdfox_builder_enable_pdf_ua1(builder)`.
  - C#: `.TaggedPdfUA1()`.
  - Go: `SetTaggedPdfUA1()`.
  - Node: same as WASM.
- **Estimated total:** ~300 Rust + 80 LoC per binding × 6 bindings
  = ~780 LoC.

### F-2: `/Lang` per run

- **Files touched:**
  - `src/writer/pdf_writer.rs` — add `PdfWriterConfig.language:
    Option<String>`, emit in catalog.
  - `src/writer/document_builder.rs` — `DocumentMetadata.language:
    Option<String>` + fluent `.language(...)`.
  - `src/elements/text.rs` — add `TextContent.lang: Option<String>`.
  - `src/writer/content_stream.rs` — when a text content's `lang`
    differs from ancestor, wrap in `Span` structure with `/Lang`.
- **Per-binding surface:** add `.language(str)` to DocumentBuilder
  and `.lang(str)` to text element constructors.
- **Estimated total:** ~50 Rust + 20 LoC per binding × 6 = ~170 LoC.

### F-3: `/Artifact` brackets

- **Files touched:**
  - `src/writer/content_stream.rs` — new `BeginArtifact` / `EndArtifact`
    ops.
  - `src/writer/page_template.rs` — header / footer emission wrapped
    in `/Artifact`.
  - `src/writer/document_builder.rs` — honour
    `TextContent.artifact_type` when emitting.
- **Per-binding surface:** add `.artifact(ArtifactType::Pagination(...))`
  and `.artifact_header()` / `.artifact_footer()` fluent helpers.
- **Estimated total:** ~80 Rust + 40 LoC per binding × 6 = ~320 LoC.

### F-4: `/RoleMap`

- **Files touched:**
  - `src/writer/struct_tree.rs` — accumulate role-map entries.
  - `src/writer/document_builder.rs` — `.role_map(custom, standard)`.
- **Per-binding surface:** `.roleMap(custom, standard)`.
- **Estimated total:** ~60 Rust + 20 LoC per binding × 6 = ~180 LoC.

### Validation test

- Add `tests/pdf_ua_roundtrip.rs` — build a tagged document with
  DocumentBuilder, run it through `PdfUaValidator::validate(doc,
  PdfUaLevel::Ua1)`, assert `is_compliant == true` and no errors of
  codes `UA-DOC-001`, `UA-DOC-002`, `UA-STRUCT-001..005`, `UA-FIG-001`.
- Corpus fixture: build a 3-page doc with headings H1/H2, paragraphs,
  one figure with alt text, one artifact header, one artifact
  footer.
- Manual: run veraPDF 1.24+ with PDF/UA-1 profile against the fixture.
  Expect zero errors.

### Dependency order inside Bundle F

```
F-0 (this doc) ─┐
                ├─→ F-1 (structure tree)  ─┐
                └─→ F-2 (/Lang)            ├─→ validation tests
F-3 (artifacts) ──────────────────────────┤
F-4 (role map) ──────────────────────────┘
```

F-1 must land first. F-2/F-3/F-4 can land in parallel once F-1 is in.

### Total Bundle F size

| F-N | Rust LoC | Per-binding LoC | 6-binding total | Tests |
|-----|----------|-----------------|-----------------|-------|
| F-1 | 300      | 80              | 780             | 120   |
| F-2 | 50       | 20              | 170             | 40    |
| F-3 | 80       | 40              | 320             | 60    |
| F-4 | 60       | 20              | 180             | 30    |
| **Total** | **490** | **—**      | **~1,450**      | **250** |

Sits comfortably in a ~2,000-LoC release bundle — comparable to the
#393 table bundle.

## Open questions

### 1. Decorative images — `/Alt` mandatory or omit?

**PDF/UA-1 §7.3**: every `<Figure>` structure element must have an
`/Alt` entry. There is no "decorative image" mode on `<Figure>` —
instead, purely-decorative images should be marked as `/Artifact` and
*not* appear in the structure tree. This means:

- `FluentPageBuilder::image_from_file(...)` either requires an
  `.alt_text(...)` call or must go into an `/Artifact` bracket.
- **Proposal:** builder requires alt text when tagged mode is on;
  explicit `.decorative()` method marks image as artifact.

Matterhorn rule 13-004 bites if we don't enforce this.

### 2. `/Tabs /S` — required or coupled to Bundle D-4?

**PDF/UA-1 §7.5** requires each page with annotations to have
`/Tabs /S` (structure order) on the page dict. This **requires** the
structure tree. Once F-1 lands, setting `/Tabs /S` on every page is a
one-line addition; the hard part (ordering by structure tree) already
comes for free from reading-order. Bundle D-4 ("tab order") becomes
trivial after F-1. **Proposal:** fold D-4 into F-1.

### 3. Hyperlinks — how does `link_url` integrate?

Current `.link_url(url)` at `document_builder.rs:643` adds a link
annotation to the page. For PDF/UA-1:

1. The link must be inside a `<Link>` structure element.
2. The `<Link>` element's `/K` contains an `/OBJR` dictionary
   (object reference marker) pointing at the link annotation.
3. The link annotation gets a `/StructParent` back-reference to the
   `<Link>` element in `/ParentTree`.
4. The annotation itself gets `/Contents (alt description of link)`.

**Proposal:** when tagged mode is on, `link_url` automatically wraps
the current text run in a `<Link>` structure element and emits the
OBJR dictionary. Shape:

```rust
pub fn link_url(self, url: &str) -> Self  // existing signature kept
// Internally: wraps current element in <Link>, emits OBJR, sets
// /Contents on the annotation, allocates /StructParent slot.
```

Zero API change for callers, full UA-1 conformance under the hood.

### 4. Streaming tables — can we tag `streaming_table` output?

`streaming_table` at `document_builder.rs:404` emits rows
incrementally. Structure tree emission **requires** deferring the
final catalog + structure tree until all pages are written. This is
already the case (`PdfWriter::finish` assembles everything at the
end), but we must buffer row-level structure elements in memory. For
the 30k-row MigraDoc comparison from #393, this is ~30k
`<TR>` elements + 5×30k `<TD>` elements = ~180k `/StructElem`
objects. Each is ~80 bytes → 14 MB of structure tree. Acceptable but
worth documenting.

### 5. UA-2 in parallel?

**Option A (recommended):** UA-1 only in v0.3.40, UA-2 deferred.
**Option B:** emit both — set `pdfuaid:part = 1` unless caller
opts into UA-2, in which case we additionally ensure PDF 2.0 output,
stricter nesting, and Namespace support. Cost of B: +200 LoC, +1
release of stabilisation work. Go with A.

### 6. Nested lists — `/LI` children rule

Matterhorn 15-004: nested list must be a `/L` child of an `/LI`, not
of another `/L`. Our `FluentPageBuilder` has no list primitive today,
so this is **not** a v0.3.40 concern but worth flagging for Bundle E
(Rich-text).

### 7. ActualText on ligatures / custom glyphs

Matterhorn 02-001 requires text in the content stream to be
Unicode-mappable. Our font subsetter emits `/ToUnicode` CMaps, so
ligatures pass. If a user draws glyph-id-based text (we don't
currently expose this), we'd need `/ActualText` on the enclosing
marked-content sequence. Out of scope for v0.3.40.

---

## Recommended UA-1 scope summary for v0.3.40 (10-line)

1. Target **PDF/UA-1 (ISO 14289-1:2014)** only. UA-2 deferred until
   v0.3.42+.
2. **F-1 — structure tree:** serialise `StructureElement` tree into
   `/StructTreeRoot` + `/StructElem` objects with `/ParentTree` number
   tree; set catalog `/MarkInfo /Marked true`, XMP `pdfuaid:part 1`.
3. **F-2 — `/Lang`:** document-level on catalog, per-run on `Span`
   structure elements when content language differs.
4. **F-3 — `/Artifact` brackets:** write-side BDC/EMC with
   `/Type /Pagination /Subtype /Header|/Footer` for headers, footers,
   page numbers, watermarks; honour `TextContent.artifact_type`.
5. **F-4 — `/RoleMap`:** custom-to-standard mapping with build-time
   validation against Matterhorn 04-003.
6. Opt-in via `.tagged_pdf_ua1()` on DocumentBuilder; zero effect on
   existing callers when not enabled.
7. Deferred: table header/cell association, link `<Link>`+`/OBJR`,
   form-field accessibility, per-page tab order — all require
   coordination with other bundles.
8. Budget: ~490 Rust LoC + ~1,450 LoC across 6 bindings + ~250 LoC
   tests. Round-trip validation against existing
   `PdfUaValidator::validate(_, PdfUaLevel::Ua1)` at
   `src/compliance/pdf_ua.rs:459`; external sanity check with
   veraPDF 1.24+ PDF/UA-1 profile.
9. Prior-art baseline: iText 8 (gold standard), QuestPDF (naming),
   typst-pdf (pure-Rust algorithm reference).
10. Risk: **medium**. The hardest part — MCID ↔ structure back-refs —
    already has half its plumbing in
    `src/writer/content_stream.rs:1100-1154`; the missing piece is
    the tree-emission side, which follows the same pattern as
    outlines and page labels.
