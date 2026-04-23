//go:build !cgo

// Package pdfoxide — purego backend (CGO_ENABLED=0).
//
// This file provides a cgo-free implementation of the Go bindings that
// dynamically loads `libpdf_oxide.{so,dylib,dll}` at init via
// github.com/ebitengine/purego. It unlocks `CGO_ENABLED=0` cross-compiles
// and pure-Go toolchain builds at the cost of a small per-call dispatch
// overhead compared to the cgo backend (pdf_oxide.go).
//
// Scope (v0.3.38): read-side PdfDocument API + a minimal PdfCreator
// for tests. Editor, DocumentBuilder, barcodes, signatures, TSA,
// rendering, forms, OCR, and other surfaces remain cgo-only for now
// — constructing them under this build tag will yield a compile error
// since the owning types/functions are tagged `//go:build cgo`.
//
// Runtime lookup:
//
//	PDF_OXIDE_LIB_PATH — if set, purego.Dlopen is called with that path.
//	Otherwise we search (in order):
//	  $XDG_CACHE_HOME/pdf_oxide/v<ver>/lib/<GOOS_GOARCH>/libpdf_oxide.<ext>
//	  <os.UserCacheDir>/pdf_oxide/v<ver>/lib/<GOOS_GOARCH>/libpdf_oxide.<ext>
//	  (Linux only) system loader — `libpdf_oxide.so` via dlopen RTLD lookup
//
// If none resolve, every public entry point returns an error wrapping
// ErrPuregoLibraryNotFound.

package pdfoxide

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"sync"
	"unsafe"

	"github.com/ebitengine/purego"
)

// ErrPuregoLibraryNotFound is returned when the shared lib can't be located
// at build time. Setting PDF_OXIDE_LIB_PATH or running `go run
// github.com/yfedoseev/pdf_oxide/go/cmd/install@latest -shared` fixes it.
var ErrPuregoLibraryNotFound = errors.New("pdf_oxide: could not locate libpdf_oxide shared library (set PDF_OXIDE_LIB_PATH or run the installer with -shared)")

// ─── Library loading ─────────────────────────────────────────────────────────

var (
	libOnce sync.Once
	libErr  error
	libHdl  uintptr
)

// loadLib ensures libpdf_oxide is dlopened exactly once. Subsequent calls
// are no-ops. Errors are sticky so we don't retry a failed load.
func loadLib() error {
	libOnce.Do(func() {
		path, err := locateLib()
		if err != nil {
			libErr = err
			return
		}
		handle, err := purego.Dlopen(path, purego.RTLD_NOW|purego.RTLD_GLOBAL)
		if err != nil {
			libErr = fmt.Errorf("pdf_oxide: dlopen %s: %w", path, err)
			return
		}
		libHdl = handle
		registerFFI(handle)
	})
	return libErr
}

// libExtension returns the per-platform shared-object extension.
func libExtension() string {
	switch runtime.GOOS {
	case "darwin":
		return ".dylib"
	case "windows":
		return ".dll"
	default:
		return ".so"
	}
}

// libBasename is the platform-appropriate basename the release workflow ships.
func libBasename() string {
	if runtime.GOOS == "windows" {
		return "pdf_oxide.dll"
	}
	return "libpdf_oxide" + libExtension()
}

// locateLib returns an absolute path to libpdf_oxide — PDF_OXIDE_LIB_PATH
// first, then the installer's cache layout, then falls back to the system
// loader (bare basename; Dlopen will use LD_LIBRARY_PATH / rpath).
func locateLib() (string, error) {
	if env := os.Getenv("PDF_OXIDE_LIB_PATH"); env != "" {
		return env, nil
	}
	// Match the installer's layout: <user-cache>/pdf_oxide/v<VER>/lib/<GOOS_GOARCH>/<libname>
	// We don't know the version here, so we scan the directory for any vX.Y.Z.
	if cacheDir, err := os.UserCacheDir(); err == nil {
		root := filepath.Join(cacheDir, "pdf_oxide")
		if entries, err := os.ReadDir(root); err == nil {
			sub := runtime.GOOS + "_" + runtime.GOARCH
			for _, e := range entries {
				if !e.IsDir() {
					continue
				}
				candidate := filepath.Join(root, e.Name(), "lib", sub, libBasename())
				if _, err := os.Stat(candidate); err == nil {
					return candidate, nil
				}
			}
		}
	}
	// Last-resort: system loader lookup. Will fail cleanly if the user has
	// neither set PDF_OXIDE_LIB_PATH nor installed it via -shared.
	return libBasename(), nil
}

// ─── FFI function table ──────────────────────────────────────────────────────
//
// Purego requires Go-native types in function signatures; cgo types like
// C.int are replaced by int32, C-strings by `string` (automatically marshalled
// to NUL-terminated cstring by purego), and opaque handles by `uintptr`.
//
// Strings returned *from* C come back as `uintptr` (we don't want purego to
// allocate+copy without giving us a chance to call free_string). We convert
// via goStringAndFree below.

//nolint:gochecknoglobals // registered once at init, read-only thereafter
var (
	// Memory management — first arg is the raw C pointer we got back from
	// a string/bytes-returning function. We pass it back as *byte so
	// purego forwards the same address without extra copies.
	ffiFreeString func(p *byte)
	ffiFreeBytes  func(p *byte)

	// PdfDocument — read-side
	ffiPdfDocumentOpen             func(path string, errCode *int32) uintptr
	ffiPdfDocumentOpenFromBytes    func(data []byte, length uintptr, errCode *int32) uintptr
	ffiPdfDocumentOpenWithPassword func(path, password string, errCode *int32) uintptr
	ffiPdfDocumentFree             func(handle uintptr)
	ffiPdfDocumentGetPageCount     func(handle uintptr, errCode *int32) int32
	ffiPdfDocumentGetVersion       func(handle uintptr, major, minor *uint8)
	ffiPdfDocumentHasStructureTree func(handle uintptr) bool
	ffiPdfDocumentIsEncrypted      func(handle uintptr) bool
	ffiPdfDocumentAuthenticate     func(handle uintptr, password string, errCode *int32) bool
	ffiPdfDocumentExtractText      func(handle uintptr, pageIndex int32, errCode *int32) *byte
	ffiPdfDocumentExtractAllText   func(handle uintptr, errCode *int32) *byte
	ffiPdfDocumentToMarkdown       func(handle uintptr, pageIndex int32, errCode *int32) *byte
	ffiPdfDocumentToHtml           func(handle uintptr, pageIndex int32, errCode *int32) *byte
	ffiPdfDocumentToPlainText      func(handle uintptr, pageIndex int32, errCode *int32) *byte
	ffiPdfDocumentToMarkdownAll    func(handle uintptr, errCode *int32) *byte
	ffiPdfDocumentToHtmlAll        func(handle uintptr, errCode *int32) *byte
	ffiPdfDocumentToPlainTextAll   func(handle uintptr, errCode *int32) *byte

	// PdfCreator — minimal, enough to generate test fixtures via FromMarkdown.
	ffiPdfFromMarkdown func(markdown string, errCode *int32) uintptr
	ffiPdfFromHtml     func(html string, errCode *int32) uintptr
	ffiPdfFromText     func(text string, errCode *int32) uintptr
	ffiPdfSave         func(handle uintptr, path string, errCode *int32) int32
	ffiPdfGetPageCount func(handle uintptr, errCode *int32) int32
	ffiPdfFree         func(handle uintptr)

	// Bulk JSON extractors — Fonts / Annotations / Elements / Search.
	// Each returns an opaque list handle, then `<thing>_to_json` serialises
	// the whole list in one FFI call; we unmarshal on the Go side.
	ffiPdfDocumentGetEmbeddedFonts   func(handle uintptr, pageIndex int32, errCode *int32) uintptr
	ffiPdfOxideFontListFree          func(handle uintptr)
	ffiPdfOxideFontsToJSON           func(fonts uintptr, errCode *int32) *byte
	ffiPdfDocumentGetPageAnnotations func(handle uintptr, pageIndex int32, errCode *int32) uintptr
	ffiPdfOxideAnnotationListFree    func(handle uintptr)
	ffiPdfOxideAnnotationsToJSON     func(ann uintptr, errCode *int32) *byte
	ffiPdfPageGetElements            func(handle uintptr, pageIndex int32, errCode *int32) uintptr
	ffiPdfOxideElementsFree          func(handle uintptr)
	ffiPdfOxideElementsToJSON        func(elements uintptr, errCode *int32) *byte
	ffiPdfDocumentSearchPage         func(handle uintptr, pageIndex int32, term string, caseSensitive bool, errCode *int32) uintptr
	ffiPdfDocumentSearchAll          func(handle uintptr, term string, caseSensitive bool, errCode *int32) uintptr
	ffiPdfOxideSearchResultFree      func(handle uintptr)
	ffiPdfOxideSearchResultsToJSON   func(results uintptr, errCode *int32) *byte

	// Page info — width/height/rotation/boxes.
	ffiPdfPageGetWidth    func(handle uintptr, pageIndex int32, errCode *int32) float32
	ffiPdfPageGetHeight   func(handle uintptr, pageIndex int32, errCode *int32) float32
	ffiPdfPageGetRotation func(handle uintptr, pageIndex int32, errCode *int32) int32

	// Logging
	ffiSetLogLevel func(level int32)
	ffiGetLogLevel func() int32
)

func registerFFI(lib uintptr) {
	r := func(target any, name string) {
		purego.RegisterLibFunc(target, lib, name)
	}
	r(&ffiFreeString, "free_string")
	r(&ffiFreeBytes, "free_bytes")

	r(&ffiPdfDocumentOpen, "pdf_document_open")
	r(&ffiPdfDocumentOpenFromBytes, "pdf_document_open_from_bytes")
	r(&ffiPdfDocumentOpenWithPassword, "pdf_document_open_with_password")
	r(&ffiPdfDocumentFree, "pdf_document_free")
	r(&ffiPdfDocumentGetPageCount, "pdf_document_get_page_count")
	r(&ffiPdfDocumentGetVersion, "pdf_document_get_version")
	r(&ffiPdfDocumentHasStructureTree, "pdf_document_has_structure_tree")
	r(&ffiPdfDocumentIsEncrypted, "pdf_document_is_encrypted")
	r(&ffiPdfDocumentAuthenticate, "pdf_document_authenticate")
	r(&ffiPdfDocumentExtractText, "pdf_document_extract_text")
	r(&ffiPdfDocumentExtractAllText, "pdf_document_extract_all_text")
	r(&ffiPdfDocumentToMarkdown, "pdf_document_to_markdown")
	r(&ffiPdfDocumentToHtml, "pdf_document_to_html")
	r(&ffiPdfDocumentToPlainText, "pdf_document_to_plain_text")
	r(&ffiPdfDocumentToMarkdownAll, "pdf_document_to_markdown_all")
	r(&ffiPdfDocumentToHtmlAll, "pdf_document_to_html_all")
	r(&ffiPdfDocumentToPlainTextAll, "pdf_document_to_plain_text_all")

	r(&ffiPdfFromMarkdown, "pdf_from_markdown")
	r(&ffiPdfFromHtml, "pdf_from_html")
	r(&ffiPdfFromText, "pdf_from_text")
	r(&ffiPdfSave, "pdf_save")
	r(&ffiPdfGetPageCount, "pdf_get_page_count")
	r(&ffiPdfFree, "pdf_free")

	r(&ffiPdfDocumentGetEmbeddedFonts, "pdf_document_get_embedded_fonts")
	r(&ffiPdfOxideFontListFree, "pdf_oxide_font_list_free")
	r(&ffiPdfOxideFontsToJSON, "pdf_oxide_fonts_to_json")
	r(&ffiPdfDocumentGetPageAnnotations, "pdf_document_get_page_annotations")
	r(&ffiPdfOxideAnnotationListFree, "pdf_oxide_annotation_list_free")
	r(&ffiPdfOxideAnnotationsToJSON, "pdf_oxide_annotations_to_json")
	r(&ffiPdfPageGetElements, "pdf_page_get_elements")
	r(&ffiPdfOxideElementsFree, "pdf_oxide_elements_free")
	r(&ffiPdfOxideElementsToJSON, "pdf_oxide_elements_to_json")
	r(&ffiPdfDocumentSearchPage, "pdf_document_search_page")
	r(&ffiPdfDocumentSearchAll, "pdf_document_search_all")
	r(&ffiPdfOxideSearchResultFree, "pdf_oxide_search_result_free")
	r(&ffiPdfOxideSearchResultsToJSON, "pdf_oxide_search_results_to_json")

	r(&ffiPdfPageGetWidth, "pdf_page_get_width")
	r(&ffiPdfPageGetHeight, "pdf_page_get_height")
	r(&ffiPdfPageGetRotation, "pdf_page_get_rotation")

	r(&ffiSetLogLevel, "pdf_oxide_set_log_level")
	r(&ffiGetLogLevel, "pdf_oxide_get_log_level")
}

// goStringAndFree copies a NUL-terminated C string at p into a Go string and
// then calls free_string on p. Safe when p == nil; returns "".
func goStringAndFree(p *byte) string {
	if p == nil {
		return ""
	}
	// Walk until NUL using unsafe.Add (idiomatic pointer arithmetic).
	base := unsafe.Pointer(p)
	var n int
	for *(*byte)(unsafe.Add(base, n)) != 0 {
		n++
	}
	// unsafe.Slice is the Go 1.17+ way to build a slice from a C buffer.
	// string() copies the bytes into Go-managed memory so it's safe to
	// free the C side immediately after.
	s := string(unsafe.Slice(p, n))
	ffiFreeString(p)
	return s
}

// ─── PdfDocument ─────────────────────────────────────────────────────────────

// PdfDocument represents an open PDF document.
// It is safe for concurrent use by multiple goroutines.
type PdfDocument struct {
	mu     sync.RWMutex
	handle uintptr
	closed bool
}

func (doc *PdfDocument) acquireRead() error {
	doc.mu.Lock()
	if doc.closed {
		doc.mu.Unlock()
		return ErrDocumentClosed
	}
	return nil
}

// Open opens a PDF document from file path.
func Open(path string) (*PdfDocument, error) {
	if err := loadLib(); err != nil {
		return nil, err
	}
	var ec int32
	h := ffiPdfDocumentOpen(path, &ec)
	if ec != 0 {
		return nil, ffiErrorFromInt(int(ec))
	}
	if h == 0 {
		return nil, fmt.Errorf("pdf_oxide: failed to open document: %w", ErrInternal)
	}
	return &PdfDocument{handle: h}, nil
}

// OpenFromBytes opens a PDF document from an in-memory byte slice.
func OpenFromBytes(data []byte) (*PdfDocument, error) {
	if err := loadLib(); err != nil {
		return nil, err
	}
	if len(data) == 0 {
		return nil, ErrEmptyContent
	}
	var ec int32
	h := ffiPdfDocumentOpenFromBytes(data, uintptr(len(data)), &ec)
	if ec != 0 {
		return nil, ffiErrorFromInt(int(ec))
	}
	if h == 0 {
		return nil, fmt.Errorf("pdf_oxide: failed to open document from bytes: %w", ErrInternal)
	}
	return &PdfDocument{handle: h}, nil
}

// OpenWithPassword opens an encrypted PDF document with the given password.
func OpenWithPassword(path, password string) (*PdfDocument, error) {
	if err := loadLib(); err != nil {
		return nil, err
	}
	var ec int32
	h := ffiPdfDocumentOpenWithPassword(path, password, &ec)
	if ec != 0 {
		return nil, ffiErrorFromInt(int(ec))
	}
	if h == 0 {
		return nil, fmt.Errorf("pdf_oxide: failed to open document: %w", ErrInternal)
	}
	return &PdfDocument{handle: h}, nil
}

// Close releases document resources. Safe to call multiple times.
func (doc *PdfDocument) Close() error {
	doc.mu.Lock()
	defer doc.mu.Unlock()
	if !doc.closed && doc.handle != 0 {
		ffiPdfDocumentFree(doc.handle)
		doc.closed = true
		doc.handle = 0
	}
	return nil
}

// IsClosed returns whether the document is closed.
func (doc *PdfDocument) IsClosed() bool {
	doc.mu.Lock()
	defer doc.mu.Unlock()
	return doc.closed
}

// PageCount returns the number of pages in the document.
func (doc *PdfDocument) PageCount() (int, error) {
	if err := doc.acquireRead(); err != nil {
		return 0, err
	}
	defer doc.mu.Unlock()
	var ec int32
	n := ffiPdfDocumentGetPageCount(doc.handle, &ec)
	if ec != 0 {
		return 0, ffiErrorFromInt(int(ec))
	}
	return int(n), nil
}

// Version returns the PDF spec version as (major, minor).
func (doc *PdfDocument) Version() (uint8, uint8, error) {
	if err := doc.acquireRead(); err != nil {
		return 0, 0, err
	}
	defer doc.mu.Unlock()
	var major, minor uint8
	ffiPdfDocumentGetVersion(doc.handle, &major, &minor)
	return major, minor, nil
}

// HasStructureTree reports whether the document has a Tagged PDF structure tree.
func (doc *PdfDocument) HasStructureTree() (bool, error) {
	if err := doc.acquireRead(); err != nil {
		return false, err
	}
	defer doc.mu.Unlock()
	return ffiPdfDocumentHasStructureTree(doc.handle), nil
}

// IsEncrypted reports whether the document is encrypted.
func (doc *PdfDocument) IsEncrypted() (bool, error) {
	if err := doc.acquireRead(); err != nil {
		return false, err
	}
	defer doc.mu.Unlock()
	return ffiPdfDocumentIsEncrypted(doc.handle), nil
}

// Authenticate attempts to decrypt the document with the given password.
func (doc *PdfDocument) Authenticate(password string) (bool, error) {
	doc.mu.Lock()
	defer doc.mu.Unlock()
	if doc.closed {
		return false, ErrDocumentClosed
	}
	var ec int32
	ok := ffiPdfDocumentAuthenticate(doc.handle, password, &ec)
	if ec != 0 {
		return false, ffiErrorFromInt(int(ec))
	}
	return ok, nil
}

// ExtractText extracts plain text from the given page (0-based).
func (doc *PdfDocument) ExtractText(pageIndex int) (string, error) {
	if err := doc.acquireRead(); err != nil {
		return "", err
	}
	defer doc.mu.Unlock()
	var ec int32
	p := ffiPdfDocumentExtractText(doc.handle, int32(pageIndex), &ec)
	if ec != 0 {
		return "", ffiErrorFromInt(int(ec))
	}
	return goStringAndFree(p), nil
}

// ExtractAllText extracts plain text from every page, concatenated.
func (doc *PdfDocument) ExtractAllText() (string, error) {
	if err := doc.acquireRead(); err != nil {
		return "", err
	}
	defer doc.mu.Unlock()
	var ec int32
	p := ffiPdfDocumentExtractAllText(doc.handle, &ec)
	if ec != 0 {
		return "", ffiErrorFromInt(int(ec))
	}
	return goStringAndFree(p), nil
}

// ToMarkdown renders the given page as Markdown.
func (doc *PdfDocument) ToMarkdown(pageIndex int) (string, error) {
	if err := doc.acquireRead(); err != nil {
		return "", err
	}
	defer doc.mu.Unlock()
	var ec int32
	p := ffiPdfDocumentToMarkdown(doc.handle, int32(pageIndex), &ec)
	if ec != 0 {
		return "", ffiErrorFromInt(int(ec))
	}
	return goStringAndFree(p), nil
}

// ToHtml renders the given page as HTML.
func (doc *PdfDocument) ToHtml(pageIndex int) (string, error) {
	if err := doc.acquireRead(); err != nil {
		return "", err
	}
	defer doc.mu.Unlock()
	var ec int32
	p := ffiPdfDocumentToHtml(doc.handle, int32(pageIndex), &ec)
	if ec != 0 {
		return "", ffiErrorFromInt(int(ec))
	}
	return goStringAndFree(p), nil
}

// ToPlainText renders the given page as plain text.
func (doc *PdfDocument) ToPlainText(pageIndex int) (string, error) {
	if err := doc.acquireRead(); err != nil {
		return "", err
	}
	defer doc.mu.Unlock()
	var ec int32
	p := ffiPdfDocumentToPlainText(doc.handle, int32(pageIndex), &ec)
	if ec != 0 {
		return "", ffiErrorFromInt(int(ec))
	}
	return goStringAndFree(p), nil
}

// ToMarkdownAll renders all pages as Markdown, concatenated.
func (doc *PdfDocument) ToMarkdownAll() (string, error) {
	if err := doc.acquireRead(); err != nil {
		return "", err
	}
	defer doc.mu.Unlock()
	var ec int32
	p := ffiPdfDocumentToMarkdownAll(doc.handle, &ec)
	if ec != 0 {
		return "", ffiErrorFromInt(int(ec))
	}
	return goStringAndFree(p), nil
}

// ToHtmlAll renders all pages as HTML, concatenated.
func (doc *PdfDocument) ToHtmlAll() (string, error) {
	if err := doc.acquireRead(); err != nil {
		return "", err
	}
	defer doc.mu.Unlock()
	var ec int32
	p := ffiPdfDocumentToHtmlAll(doc.handle, &ec)
	if ec != 0 {
		return "", ffiErrorFromInt(int(ec))
	}
	return goStringAndFree(p), nil
}

// ToPlainTextAll renders all pages as plain text, concatenated.
func (doc *PdfDocument) ToPlainTextAll() (string, error) {
	if err := doc.acquireRead(); err != nil {
		return "", err
	}
	defer doc.mu.Unlock()
	var ec int32
	p := ffiPdfDocumentToPlainTextAll(doc.handle, &ec)
	if ec != 0 {
		return "", ffiErrorFromInt(int(ec))
	}
	return goStringAndFree(p), nil
}

// ─── Page — lightweight handle for a single page of a PdfDocument ──────────

// Page represents a single page of a PdfDocument.
type Page struct {
	doc   *PdfDocument
	Index int
}

// Page returns a handle to the page at the given zero-based index.
func (doc *PdfDocument) Page(index int) (*Page, error) {
	count, err := doc.PageCount()
	if err != nil {
		return nil, err
	}
	if index < 0 || index >= count {
		return nil, fmt.Errorf("page index %d out of range [0, %d)", index, count)
	}
	return &Page{doc: doc, Index: index}, nil
}

// Pages returns all pages as a slice.
func (doc *PdfDocument) Pages() ([]*Page, error) {
	count, err := doc.PageCount()
	if err != nil {
		return nil, err
	}
	pages := make([]*Page, count)
	for i := 0; i < count; i++ {
		pages[i] = &Page{doc: doc, Index: i}
	}
	return pages, nil
}

// Text extracts plain text from the page.
func (p *Page) Text() (string, error) { return p.doc.ExtractText(p.Index) }

// Markdown renders the page as Markdown.
func (p *Page) Markdown() (string, error) { return p.doc.ToMarkdown(p.Index) }

// Html renders the page as HTML.
func (p *Page) Html() (string, error) { return p.doc.ToHtml(p.Index) }

// PlainText renders the page as plain text.
func (p *Page) PlainText() (string, error) { return p.doc.ToPlainText(p.Index) }

// ─── PdfCreator — minimal subset for test fixtures ──────────────────────────

// PdfCreator represents a generated PDF. Only enough of the API is implemented
// under the purego backend to let test helpers build fixtures in-memory; the
// richer editor/builder APIs remain cgo-only.
type PdfCreator struct {
	mu     sync.Mutex
	handle uintptr
	closed bool
}

// FromMarkdown builds a PDF from a Markdown string.
func FromMarkdown(markdown string) (*PdfCreator, error) {
	if err := loadLib(); err != nil {
		return nil, err
	}
	if markdown == "" {
		return nil, ErrEmptyContent
	}
	var ec int32
	h := ffiPdfFromMarkdown(markdown, &ec)
	if ec != 0 {
		return nil, ffiErrorFromInt(int(ec))
	}
	if h == 0 {
		return nil, fmt.Errorf("pdf_oxide: failed to create pdf from markdown: %w", ErrInternal)
	}
	return &PdfCreator{handle: h}, nil
}

// FromHtml builds a PDF from an HTML string.
func FromHtml(html string) (*PdfCreator, error) {
	if err := loadLib(); err != nil {
		return nil, err
	}
	if html == "" {
		return nil, ErrEmptyContent
	}
	var ec int32
	h := ffiPdfFromHtml(html, &ec)
	if ec != 0 {
		return nil, ffiErrorFromInt(int(ec))
	}
	if h == 0 {
		return nil, fmt.Errorf("pdf_oxide: failed to create pdf from html: %w", ErrInternal)
	}
	return &PdfCreator{handle: h}, nil
}

// FromText builds a PDF from a plain text string.
func FromText(text string) (*PdfCreator, error) {
	if err := loadLib(); err != nil {
		return nil, err
	}
	if text == "" {
		return nil, ErrEmptyContent
	}
	var ec int32
	h := ffiPdfFromText(text, &ec)
	if ec != 0 {
		return nil, ffiErrorFromInt(int(ec))
	}
	if h == 0 {
		return nil, fmt.Errorf("pdf_oxide: failed to create pdf from text: %w", ErrInternal)
	}
	return &PdfCreator{handle: h}, nil
}

// Save writes the generated PDF to disk.
func (c *PdfCreator) Save(path string) error {
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.closed {
		return ErrCreatorClosed
	}
	var ec int32
	rc := ffiPdfSave(c.handle, path, &ec)
	if ec != 0 {
		return ffiErrorFromInt(int(ec))
	}
	if rc != 0 {
		return fmt.Errorf("pdf_oxide: save returned %d: %w", rc, ErrInternal)
	}
	return nil
}

// PageCount returns how many pages the generated PDF contains.
func (c *PdfCreator) PageCount() (int, error) {
	c.mu.Lock()
	defer c.mu.Unlock()
	if c.closed {
		return 0, ErrCreatorClosed
	}
	var ec int32
	n := ffiPdfGetPageCount(c.handle, &ec)
	if ec != 0 {
		return 0, ffiErrorFromInt(int(ec))
	}
	return int(n), nil
}

// Close releases creator resources. Safe to call multiple times.
func (c *PdfCreator) Close() error {
	c.mu.Lock()
	defer c.mu.Unlock()
	if !c.closed && c.handle != 0 {
		ffiPdfFree(c.handle)
		c.closed = true
		c.handle = 0
	}
	return nil
}

// ─── Bulk JSON extractors — Fonts / Annotations / PageElements / Search ────

// unmarshalListJSON is the common tail shared by every *-to-JSON extractor:
// decode the returned C string into dst, free_string the C pointer, and
// forward any error_code from the extractor.
func unmarshalListJSON(p *byte, ec int32, dst any) error {
	if ec != 0 {
		return ffiErrorFromInt(int(ec))
	}
	if p == nil {
		return nil
	}
	s := goStringAndFree(p)
	if s == "" {
		return nil
	}
	return json.Unmarshal([]byte(s), dst)
}

// Fonts returns all fonts embedded in or used by the given page. One FFI
// call per page — the list is JSON-encoded on the Rust side.
func (doc *PdfDocument) Fonts(pageIndex int) ([]Font, error) {
	if err := doc.acquireRead(); err != nil {
		return nil, err
	}
	defer doc.mu.Unlock()
	var ec int32
	h := ffiPdfDocumentGetEmbeddedFonts(doc.handle, int32(pageIndex), &ec)
	if ec != 0 {
		return nil, ffiErrorFromInt(int(ec))
	}
	if h == 0 {
		return nil, fmt.Errorf("pdf_oxide: failed to get fonts: %w", ErrInternal)
	}
	defer ffiPdfOxideFontListFree(h)

	var jec int32
	p := ffiPdfOxideFontsToJSON(h, &jec)
	var out []Font
	if err := unmarshalListJSON(p, jec, &out); err != nil {
		return nil, err
	}
	return out, nil
}

// Annotations returns all annotations on the given page.
func (doc *PdfDocument) Annotations(pageIndex int) ([]Annotation, error) {
	if err := doc.acquireRead(); err != nil {
		return nil, err
	}
	defer doc.mu.Unlock()
	var ec int32
	h := ffiPdfDocumentGetPageAnnotations(doc.handle, int32(pageIndex), &ec)
	if ec != 0 {
		return nil, ffiErrorFromInt(int(ec))
	}
	if h == 0 {
		return nil, fmt.Errorf("pdf_oxide: failed to get annotations: %w", ErrInternal)
	}
	defer ffiPdfOxideAnnotationListFree(h)

	var jec int32
	p := ffiPdfOxideAnnotationsToJSON(h, &jec)
	var out []Annotation
	if err := unmarshalListJSON(p, jec, &out); err != nil {
		return nil, err
	}
	return out, nil
}

// PageElements returns all layout elements (text spans with bbox) on the
// given page.
func (doc *PdfDocument) PageElements(pageIndex int) ([]Element, error) {
	if err := doc.acquireRead(); err != nil {
		return nil, err
	}
	defer doc.mu.Unlock()
	var ec int32
	h := ffiPdfPageGetElements(doc.handle, int32(pageIndex), &ec)
	if ec != 0 {
		return nil, ffiErrorFromInt(int(ec))
	}
	if h == 0 {
		return nil, fmt.Errorf("pdf_oxide: failed to get elements: %w", ErrInternal)
	}
	defer ffiPdfOxideElementsFree(h)

	var jec int32
	p := ffiPdfOxideElementsToJSON(h, &jec)
	var out []Element
	if err := unmarshalListJSON(p, jec, &out); err != nil {
		return nil, err
	}
	return out, nil
}

// SearchPage searches the given page for `term` and returns all matches.
func (doc *PdfDocument) SearchPage(pageIndex int, term string, caseSensitive bool) ([]SearchResult, error) {
	if err := doc.acquireRead(); err != nil {
		return nil, err
	}
	defer doc.mu.Unlock()
	var ec int32
	h := ffiPdfDocumentSearchPage(doc.handle, int32(pageIndex), term, caseSensitive, &ec)
	if ec != 0 {
		return nil, ffiErrorFromInt(int(ec))
	}
	if h == 0 {
		return nil, ErrInternal
	}
	defer ffiPdfOxideSearchResultFree(h)

	var jec int32
	p := ffiPdfOxideSearchResultsToJSON(h, &jec)
	var out []SearchResult
	if err := unmarshalListJSON(p, jec, &out); err != nil {
		return nil, err
	}
	return out, nil
}

// SearchAll searches the whole document for `term`.
func (doc *PdfDocument) SearchAll(term string, caseSensitive bool) ([]SearchResult, error) {
	if err := doc.acquireRead(); err != nil {
		return nil, err
	}
	defer doc.mu.Unlock()
	var ec int32
	h := ffiPdfDocumentSearchAll(doc.handle, term, caseSensitive, &ec)
	if ec != 0 {
		return nil, ffiErrorFromInt(int(ec))
	}
	if h == 0 {
		return nil, ErrInternal
	}
	defer ffiPdfOxideSearchResultFree(h)

	var jec int32
	p := ffiPdfOxideSearchResultsToJSON(h, &jec)
	var out []SearchResult
	if err := unmarshalListJSON(p, jec, &out); err != nil {
		return nil, err
	}
	return out, nil
}

// ─── Page dimensions ─────────────────────────────────────────────────────────

// PageWidth returns the given page's width in PDF points.
func (doc *PdfDocument) PageWidth(pageIndex int) (float32, error) {
	if err := doc.acquireRead(); err != nil {
		return 0, err
	}
	defer doc.mu.Unlock()
	var ec int32
	w := ffiPdfPageGetWidth(doc.handle, int32(pageIndex), &ec)
	if ec != 0 {
		return 0, ffiErrorFromInt(int(ec))
	}
	return w, nil
}

// PageHeight returns the given page's height in PDF points.
func (doc *PdfDocument) PageHeight(pageIndex int) (float32, error) {
	if err := doc.acquireRead(); err != nil {
		return 0, err
	}
	defer doc.mu.Unlock()
	var ec int32
	h := ffiPdfPageGetHeight(doc.handle, int32(pageIndex), &ec)
	if ec != 0 {
		return 0, ffiErrorFromInt(int(ec))
	}
	return h, nil
}

// PageRotation returns the rotation (degrees) of the given page.
func (doc *PdfDocument) PageRotation(pageIndex int) (int, error) {
	if err := doc.acquireRead(); err != nil {
		return 0, err
	}
	defer doc.mu.Unlock()
	var ec int32
	r := ffiPdfPageGetRotation(doc.handle, int32(pageIndex), &ec)
	if ec != 0 {
		return 0, ffiErrorFromInt(int(ec))
	}
	return int(r), nil
}

// ─── Logging ─────────────────────────────────────────────────────────────────

// SetLogLevel sets the global log level.
func SetLogLevel(level LogLevel) {
	if err := loadLib(); err != nil {
		return
	}
	ffiSetLogLevel(int32(level))
}

// GetLogLevel returns the current log level.
func GetLogLevel() LogLevel {
	if err := loadLib(); err != nil {
		return LogOff
	}
	return LogLevel(ffiGetLogLevel())
}
