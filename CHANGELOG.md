# Changelog

All notable changes to PDFOxide are documented here.

## [0.3.4] - 2026-02-12

### ⚠️ Breaking Changes
- **`parse_header()` function signature** - Now includes offset tracking
  - **Before**: `parse_header(reader) -> Result<(u8, u8)>`
  - **After**: `parse_header(reader, lenient) -> Result<(u8, u8, u64)>`
  - **Migration**: Replace `let (major, minor) = parse_header(&mut reader)?;` with `let (major, minor, _offset) = parse_header(&mut reader, true)?;`
  - Note: This is a public API function; consider using `doc.version()` for typical use cases instead

### Fixed - PDF Parsing Robustness (Issue #41)
- **Header offset support** - PDFs with binary prefixes or BOM headers now open successfully
  - Parse header function now searches first 1024 bytes for `%PDF-` marker (PDF spec compliant)
  - Supports UTF-8 BOM, email headers, and other leading binary data
  - `parse_header()` returns byte offset where header was found
  - Lenient mode (default) handles real-world malformed PDFs; strict mode for compliance testing
  - Fixes parsing errors like "expected '%PDF-', found '1b965'"

### Added - Character-Level Text Extraction (Issue #39)
- **`extract_chars()` API** - Low-level character-level extraction for layout analysis
  - Returns `Vec<TextChar>` with per-character positioning, font, and styling data
  - Includes transformation matrix, rotation angle, advance width
  - Sorted in reading order (top-to-bottom, left-to-right)
  - Overlapping characters (rendered multiple times) deduplicated
  - 30-50% faster than span extraction for character-only use cases
  - Exposed in both Rust and Python APIs
  - **Python binding**: `doc.extract_chars(page_index)` returns list of `TextChar` objects

### Added - XObject Path Extraction (Issue #40)
- **Form XObject support in path extraction** - Now extracts vectors from embedded XObjects
  - `extract_paths()` recursively processes Form XObjects via `Do` operator
  - Image XObjects properly skipped (only Form XObjects extracted)
  - Coordinate transformations via `/Matrix` properly applied
  - Graphics state properly isolated (save/restore)
  - Duplicate XObject detection prevents infinite loops
  - Nested XObjects (XObject containing XObject) supported

### Changed
- **Dependencies**: Upgraded nom parser library from 7.1 to 8.0
  - Updated all parser combinators to use `.parse()` method
  - No user-facing API changes
  - All parser functionality maintained
  - Performance stable (no regressions detected)
- `parse_header()` signature updated: now returns `(major, minor, offset)` tuple
- All parse_header test cases updated to use new signature

## [0.3.1] - 2026-01-14

### Added - Form Field Coverage (95% across Read/Create/Modify)

#### Hierarchical Field Creation
- **Parent/Child Field Structures** - Create complex form hierarchies like `address.street`, `address.city`
  - `add_parent_field()` - Create container fields without widgets
  - `add_child_field()` - Add child fields to existing parents
  - `add_form_field_hierarchical()` - Auto-create parent hierarchy from dotted names
  - `ParentFieldConfig` for configuring container fields
  - Property inheritance between parent and child fields (FT, V, DV, Ff, DA, Q)

#### Field Property Modification
- **Edit All Field Properties** - Beyond just values
  - `set_form_field_readonly()` / `set_form_field_required()` - Flag manipulation
  - `set_form_field_rect()` - Reposition/resize fields
  - `set_form_field_tooltip()` - Set hover text (TU)
  - `set_form_field_max_length()` - Text field length limits
  - `set_form_field_alignment()` - Text alignment (left/center/right)
  - `set_form_field_default_value()` - Default values (DV)
  - `BorderStyle` and `AppearanceCharacteristics` support
- **Critical Bug Fix** - Modified existing fields now persist on save (was only saving new fields)

#### FDF/XFDF Export
- **Forms Data Format Export** - ISO 32000-1:2008 Section 12.7.7
  - `FdfWriter` - Binary FDF export for form data exchange
  - `XfdfWriter` - XML XFDF export for web integration
  - `export_form_data_fdf()` / `export_form_data_xfdf()` on FormExtractor, DocumentEditor, Pdf
  - Hierarchical field representation in exports

### Added - Text Extraction Enhancements
- **TextChar Transformation** - Per-character positioning metadata (#27)
  - `origin` - Font baseline coordinates (x, y)
  - `rotation_degrees` - Character rotation angle
  - `matrix` - Full transformation matrix
  - Essential for pdfium-render migration

### Added - Image Metadata
- **DPI Calculation** - Resolution metadata for images
  - `horizontal_dpi` / `vertical_dpi` fields on `ImageContent`
  - `resolution()` - Get (h_dpi, v_dpi) tuple
  - `is_high_resolution()` / `is_low_resolution()` / `is_medium_resolution()` helpers
  - `calculate_dpi()` - Compute from pixel dimensions and bbox

### Added - Bounded Text Extraction
- **Spatial Filtering** - Extract text from rectangular regions
  - `RectFilterMode::Intersects` - Any overlap (default)
  - `RectFilterMode::FullyContained` - Completely within bounds
  - `RectFilterMode::MinOverlap(f32)` - Minimum overlap fraction
  - `TextSpanSpatial` trait - `intersects_rect()`, `contained_in_rect()`, `overlap_with_rect()`
  - `TextSpanFiltering` trait - `filter_by_rect()`, `extract_text_in_rect()`

### Added - Multimedia Annotations
- **MovieAnnotation** - Embedded video content
- **SoundAnnotation** - Audio content with playback controls
- **ScreenAnnotation** - Media renditions (video/audio players)
- **RichMediaAnnotation** - Flash/video rich media content

### Added - 3D Annotations
- **ThreeDAnnotation** - 3D model embedding
  - U3D and PRC format support
  - `ThreeDView` - Camera angles and lighting
  - `ThreeDAnimation` - Playback controls

### Added - Path Extraction
- **PathExtractor** - Vector graphics extraction
  - Lines, curves, rectangles, complex paths
  - Path transformation and bounding box calculation

### Added - XFA Form Support
- **XfaExtractor** - Extract XFA form data
- **XfaParser** - Parse XFA XML templates
- **XfaConverter** - Convert XFA forms to AcroForm

### Changed - Python Bindings
- **True Python 3.8-3.14 Support** - Fixed via `abi3-py38` (was only working on 3.11)
- **Modern Tooling** - uv, pdm, ruff integration
- **Code Quality** - All Python code formatted with ruff

### 🏆 Community Contributors

🥇 **@monchin** - Massive thanks for revolutionizing our Python ecosystem! Your PR #29 fixed a critical compatibility issue where PDFOxide only worked on Python 3.11 despite claiming 3.8+ support. By switching to `abi3-py38`, you enabled true cross-version compatibility (Python 3.8-3.14). The introduction of modern tooling (uv, pdm, ruff) brings PDFOxide's Python development to 2026 standards. This work directly enables thousands more Python developers to use PDFOxide. 💪🐍

🥈 **@bikallem** - Thanks for the thoughtful feature request (#27) comparing PDFOxide to pdfium-render. Your detailed analysis of missing origin coordinates and rotation angles led directly to our TextChar transformation feature. This makes PDFOxide a viable migration path for pdfium-render users. 🎯

## [0.3.0] - 2026-01-10

### Added - Unified `Pdf` API
- **One API for Extract, Create, and Edit** - The new `Pdf` class unifies all PDF operations
  - `Pdf::open("input.pdf")` - Open existing PDF for reading and editing
  - `Pdf::from_markdown(content)` - Create new PDF from Markdown
  - `Pdf::from_html(content)` - Create new PDF from HTML
  - `Pdf::from_text(content)` - Create new PDF from plain text
  - `Pdf::from_image(path)` - Create PDF from image file
  - DOM-like page navigation with `pdf.page(0)` for querying and modifying content
  - Seamless save with `pdf.save("output.pdf")` or `pdf.save_encrypted()`
- **Fluent Builder Pattern** - `PdfBuilder` for advanced configuration
  ```rust
  PdfBuilder::new()
      .title("My Document")
      .author("Author Name")
      .page_size(PageSize::A4)
      .from_markdown("# Content")?
  ```

### Added - PDF Creation
- **PDF Creation API** - Fluent `DocumentBuilder` for programmatic PDF generation
  - `Pdf::create()` / `DocumentBuilder::new()` entry points
  - Page sizing (Letter, A4, custom dimensions)
  - Text rendering with Base14 fonts and styling
  - Image embedding (JPEG/PNG) with positioning
- **Table Rendering** - `TableRenderer` for styled tables
  - Headers, borders, cell spans, alternating row colors
  - Column width control (fixed, percentage, auto)
  - Cell alignment and padding
- **Graphics API** - Advanced visual effects
  - Colors (RGB, CMYK, grayscale)
  - Linear and radial gradients
  - Tiling patterns with presets
  - Blend modes and transparency (ExtGState)
- **Page Templates** - Reusable page elements
  - Headers and footers with placeholders
  - Page numbering formats
  - Watermarks (text-based)
- **Barcode Generation** (requires `barcodes` feature)
  - QR codes with configurable size and error correction
  - Code128, EAN-13, UPC-A, Code39, ITF barcodes
  - Customizable colors and dimensions

### Added - PDF Editing
- **Editor API** - DOM-like editing with round-trip preservation
  - `DocumentEditor` for modifying existing PDFs
  - Content addition without breaking existing structure
  - Resource management for fonts and images
- **Annotation Support** - Full read/write for all types
  - Text markup: highlights, underlines, strikeouts, squiggly
  - Notes: sticky notes, comments, popups
  - Shapes: rectangles, circles, lines, polygons, polylines
  - Drawing: ink/freehand annotations
  - Stamps: standard and custom stamps
  - Special: file attachments, redactions, carets
- **Form Fields** - Interactive form creation
  - Text fields (single/multiline, password, comb)
  - Checkboxes with custom appearance
  - Radio button groups
  - Dropdown and list boxes
  - Push buttons with actions
  - Form flattening (convert fields to static content)
- **Link Annotations** - Navigation support
  - External URLs
  - Internal page navigation
  - Styled link appearance
- **Outline Builder** - Bookmark/TOC creation
  - Hierarchical structure
  - Page destinations
  - Styling (bold, italic, colors)
- **PDF Layers** - Optional Content Groups (OCG)
  - Create and manage content layers
  - Layer visibility controls

### Added - PDF Compliance & Validation
- **PDF/A Validation** - ISO 19005 compliance checking
  - PDF/A-1a, PDF/A-1b (ISO 19005-1)
  - PDF/A-2a, PDF/A-2b, PDF/A-2u (ISO 19005-2)
  - PDF/A-3a, PDF/A-3b (ISO 19005-3)
- **PDF/A Conversion** - Convert documents to archival format
  - Automatic font embedding
  - XMP metadata injection
  - ICC color profile conversion
- **PDF/X Validation** - ISO 15930 print production compliance
  - PDF/X-1a:2001, PDF/X-1a:2003
  - PDF/X-3:2002, PDF/X-3:2003
  - PDF/X-4, PDF/X-4p
  - PDF/X-5g, PDF/X-5n, PDF/X-5pg
  - PDF/X-6, PDF/X-6n, PDF/X-6p
  - 40+ specific error codes for violations
- **PDF/UA Validation** - ISO 14289 accessibility compliance
  - Tagged PDF structure validation
  - Language specification checks
  - Alt text requirements
  - Heading hierarchy validation
  - Table header validation
  - Form field accessibility
  - Reading order verification

### Added - Security & Encryption
- **Encryption on Write** - Password-protect PDFs when saving
  - AES-256 (V=5, R=6) - Modern 256-bit encryption (default)
  - AES-128 (V=4, R=4) - Modern 128-bit encryption
  - RC4-128 (V=2, R=3) - Legacy 128-bit encryption
  - RC4-40 (V=1, R=2) - Legacy 40-bit encryption
  - `Pdf::save_encrypted()` for simple password protection
  - `Pdf::save_with_encryption()` for full configuration
- **Permission Controls** - Granular access restrictions
  - Print, copy, modify, annotate permissions
  - Form fill and accessibility extraction controls
- **Digital Signatures** (foundation, requires `signatures` feature)
  - ByteRange calculation for signature placeholders
  - PKCS#7/CMS signature structure support
  - X.509 certificate parsing
  - Signature verification framework

### Added - Document Features
- **Page Labels** - Custom page numbering
  - Roman numerals, letters, decimal formats
  - Prefix support (e.g., "A-1", "B-2")
  - `PageLabelsBuilder` for creation
  - Extract existing labels from documents
- **XMP Metadata** - Extensible metadata support
  - Dublin Core properties (title, creator, description)
  - PDF properties (producer, keywords)
  - Custom namespace support
  - Full read/write capability
- **Embedded Files** - File attachments
  - Attach files to PDF documents
  - MIME type and description support
  - Relationship specification (Source, Data, etc.)
- **Linearization** - Web-optimized PDFs
  - Fast web view support
  - Streaming delivery optimization

### Added - Search & Analysis
- **Text Search** - Pattern-based document search
  - Regex pattern support
  - Case-sensitive/insensitive options
  - Position tracking with page/coordinates
  - Whole word matching
- **Page Rendering** (requires `rendering` feature)
  - Render pages to PNG/JPEG images
  - Configurable DPI and scale
  - Pure Rust via tiny-skia (no external dependencies)
- **Debug Visualization** (requires `rendering` feature)
  - Visualize text bounding boxes
  - Element highlighting for debugging
  - Export annotated page images

### Added - Document Conversion
- **Office to PDF** (requires `office` feature)
  - **DOCX**: Word documents with paragraphs, headings, lists, formatting
  - **XLSX**: Excel spreadsheets via calamine (sheets, cells, tables)
  - **PPTX**: PowerPoint presentations (slides, titles, text boxes)
  - `OfficeConverter` with auto-detection
  - `OfficeConfig` for page size, margins, fonts
  - Python bindings: `OfficeConverter.from_docx()`, `from_xlsx()`, `from_pptx()`

### Added - Python Bindings
- `Pdf` class for PDF creation
- `Color`, `BlendMode`, `ExtGState` for graphics
- `LinearGradient`, `RadialGradient` for gradients
- `LineCap`, `LineJoin`, `PatternPresets` for styling
- `save_encrypted()` method with permission flags
- `OfficeConverter` class for Office document conversion

### Changed
- Description updated to "The Complete PDF Toolkit: extract, create, and edit PDFs"
- Python module docstring updated for v0.3.0 features
- Branding updated with Extract/Create/Edit pillars

### Fixed
- **Outline action handling** - correctly dereference actions indirectly referenced by outline items

### 🏆 Community Contributors

🥇 **@jvantuyl** - Thanks for the thorough PR #16 fixing outline action dereferencing! Your investigation uncovered that some PDFs embed actions directly while others use indirect references - a subtle PDF spec detail that was breaking bookmark navigation. Your fix included comprehensive tests ensuring this won't regress. 🔍✨

🙏 **@mert-kurttutan** - Thanks for the honest feedback in issue #15 about README clutter. Your perspective as a new user helped us realize we were overwhelming people with information. The resulting documentation cleanup makes PDFOxide more approachable. 📚

## [0.2.6] - 2026-01-09

### Added
- **TagSuspect/MarkInfo support** (ISO 32000-1 Section 14.7.1)
  - Parse MarkInfo dictionary from document catalog (`marked`, `suspects`, `user_properties`)
  - `PdfDocument::mark_info()` method to retrieve MarkInfo
  - Automatic fallback to geometric ordering when structure tree is marked as suspect
- **Word Break /WB structure element** (Section 14.8.4.4)
  - Support for explicit word boundaries in CJK text
  - `StructType::WB` variant and `is_word_break()` helper
  - Word break markers emitted during structure tree traversal
- **Predefined CMap support for CJK fonts** (Section 9.7.5.2)
  - Adobe-GB1 (Simplified Chinese) - ~500 common character mappings
  - Adobe-Japan1 (Japanese) - Hiragana, Katakana, Kanji mappings
  - Adobe-CNS1 (Traditional Chinese) - Bopomofo and CJK mappings
  - Adobe-Korea1 (Korean) - Hangul and Hanja mappings
  - Fallback identity mapping for common Unicode ranges
- **Abbreviation expansion /E support** (Section 14.9.5)
  - Parse `/E` entry from marked content properties
  - `expansion` field on `StructElem` for structure-level abbreviations
- **Object reference resolution utility**
  - `PdfDocument::resolve_references()` for recursive reference handling in complex PDF structures
- **Type 0 /W array parsing** for CIDFont glyph widths
  - Proper spacing for CJK text using CIDFont width specifications
- **ActualText verification tests** - comprehensive test coverage for PDF Spec Section 14.9.4

### Fixed
- **Soft hyphen handling** (U+00AD) - now correctly treated as valid continuation hyphen for word reconstruction

### Changed
- **Enhanced artifact filtering** with subtype support
  - `ArtifactType::Pagination` with subtypes: Header, Footer, Watermark, PageNumber
  - `ArtifactType::Layout` and `ArtifactType::Background` classification
- `OrderedContent.mcid` changed to `Option<u32>` to support word break markers

## [0.2.5] - 2026-01-09

### Added
- **Image embedding**: Both HTML and Markdown now support embedded base64 images when `embed_images=true` (default)
  - HTML: `<img src="data:image/png;base64,...">`
  - Markdown: `![alt](data:image/png;base64,...)` (works in Obsidian, Typora, VS Code, Jupyter)
- **Image file export**: Set `embed_images=false` + `image_output_dir` to save images as files with relative path references
- New `embed_images` option in `ConversionOptions` to control embedding behavior
- `PdfImage::to_base64_data_uri()` method for converting images to data URIs
- `PdfImage::to_png_bytes()` method for in-memory PNG encoding
- Python bindings: new `embed_images` parameter for `to_html`, `to_markdown`, and `*_all` methods

## [0.2.4] - 2026-01-09

### Fixed
- CTM (Current Transformation Matrix) now correctly applied to text positions per PDF Spec ISO 32000-1:2008 Section 9.4.4 (#11)

### Added
- Structure tree: `/Alt` (alternate description) parsing for accessibility text on formulas and figures
- Structure tree: `/Pg` (page reference) resolution - correctly maps structure elements to page numbers
- `FormulaRenderer` module for extracting formula regions as base64 images from rendered pages
- `ConversionOptions`: new fields `render_formulas`, `page_images`, `page_dimensions` for formula image embedding
- Regression tests for CTM transformation

### 🏆 Community Contributors

🐛➡️✅ **@mert-kurttutan** - Thanks for the detailed bug report (#11) with reproducible sample PDF! Your report exposed a fundamental CTM transformation bug affecting text positioning across the entire library. This fix was critical for production use. 🎉

## [0.2.3] - 2026-01-07

### Fixed
- BT/ET matrix reset per PDF spec Section 9.4.1 (PR #10 by @drahnr)
- Geometric spacing detection in markdown converter (#5)
- Verbose extractor logs changed from info to trace (#7)
- docs.rs build failure (excluded tesseract-rs)

### Added
- `apply_intelligent_text_processing()` method for ligature expansion, hyphenation reconstruction, and OCR cleanup (#6)

### Changed
- Removed unused tesseract-rs dependency

### 🏆 Community Contributors

🥇 **@drahnr** - Huge thanks for PR #10 fixing the BT/ET matrix reset issue! This was a subtle PDF spec compliance bug (Section 9.4.1) where text matrices weren't being reset between text blocks, causing positions to accumulate and become unusable. Your fix restored correct text positioning for all PDFs. 💪📐

🔬 **@JanIvarMoldekleiv** - Thanks for the detailed bug report (#5) about missing spaces and lost table structure! Your analysis even identified the root cause in the code - the markdown converter wasn't using geometric spacing analysis. This level of investigation made the fix straightforward. 🕵️‍♂️

🎯 **@Borderliner** - Thanks for two important catches! Issue #6 revealed that `apply_intelligent_text_processing()` was documented but not actually available (oops! 😅), and #7 caught our overly verbose INFO-level logging flooding terminals. Both fixed immediately! 🔧

## [0.2.2] - 2025-12-15

### Changed
- Optimized crate keywords for better discoverability

## [0.2.1] - 2025-12-15

### Fixed
- Encrypted stream decoding improvements (#3)
- CI/CD pipeline fixes

### 🏆 Community Contributors

🥇 **@threebeanbags** - Huge thanks for PRs #2 and #3 fixing encrypted PDF support! 🔐 Your first PR identified that decryption needed to happen before decompression - a critical ordering issue. Your follow-up PR #3 went deeper, fixing encryption handler initialization timing and adding Form XObject encryption support. These fixes made PDFOxide actually work with password-protected PDFs in production. 💪🎉

## [0.1.4] - 2025-12-12

### Fixed
- Encrypted stream decoding (#2)
- Documentation and doctest fixes

## [0.1.3] - 2025-12-12

### Fixed
- Encrypted stream decoding refinements

## [0.1.2] - 2025-11-27

### Added
- Python 3.13 support
- GitHub sponsor configuration

## [0.1.1] - 2025-11-26

### Added
- Cross-platform binary builds (Linux, macOS, Windows)

## [0.1.0] - 2025-11-06

### Added
- Initial release
- PDF text extraction with spec-compliant Unicode mapping
- Intelligent reading order detection
- Python bindings via PyO3
- Support for encrypted PDFs
- Form field extraction
- Image extraction

### 🌟 Early Adopters

💖 **@magnus-trent** - Thanks for issue #1, our first community feedback! Your message that PDFOxide "unlocked an entire pipeline" you'd been working on for a month validated that we were solving real problems. Early encouragement like this keeps open source projects going. 🚀
