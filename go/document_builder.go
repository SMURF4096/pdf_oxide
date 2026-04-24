//go:build cgo

package pdfoxide

// Write-side API: DocumentBuilder, PageBuilder, EmbeddedFont, plus
// HTML+CSS pipeline wrappers.
//
// Handle-lifetime contract (mirrors the C FFI documented in
// include/pdf_oxide_c/pdf_oxide.h):
//
//   - NewDocumentBuilder returns a handle that becomes invalid after
//     Build / Save / SaveEncrypted / ToBytesEncrypted or explicit Close.
//   - A4Page / LetterPage / Page return a *PageBuilder; only ONE may be
//     outstanding per builder. Calling Done commits + invalidates the
//     page. Close drops the page without committing (error-recovery).
//   - RegisterEmbeddedFont CONSUMES the *EmbeddedFont — do not Close it
//     afterwards.

/*
#include <stdlib.h>
#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

extern void* pdf_embedded_font_from_file(const char* path, int* error_code);
extern void* pdf_embedded_font_from_bytes(const uint8_t* data, size_t len,
                                          const char* name, int* error_code);
extern void  pdf_embedded_font_free(void* handle);

extern void* pdf_document_builder_create(int* error_code);
extern void  pdf_document_builder_free(void* handle);

extern int   pdf_document_builder_set_title(void* handle, const char* title, int* error_code);
extern int   pdf_document_builder_set_author(void* handle, const char* author, int* error_code);
extern int   pdf_document_builder_set_subject(void* handle, const char* subject, int* error_code);
extern int   pdf_document_builder_set_keywords(void* handle, const char* keywords, int* error_code);
extern int   pdf_document_builder_set_creator(void* handle, const char* creator, int* error_code);

extern int   pdf_document_builder_register_embedded_font(void* handle, const char* name,
                                                         void* font, int* error_code);

extern void* pdf_document_builder_a4_page(void* handle, int* error_code);
extern void* pdf_document_builder_letter_page(void* handle, int* error_code);
extern void* pdf_document_builder_page(void* handle, float width, float height, int* error_code);

extern int   pdf_page_builder_font(void* page, const char* name, float size, int* error_code);
extern int   pdf_page_builder_at(void* page, float x, float y, int* error_code);
extern int   pdf_page_builder_text(void* page, const char* text, int* error_code);
extern int   pdf_page_builder_heading(void* page, unsigned char level, const char* text, int* error_code);
extern int   pdf_page_builder_paragraph(void* page, const char* text, int* error_code);
extern int   pdf_page_builder_space(void* page, float points, int* error_code);
extern int   pdf_page_builder_horizontal_rule(void* page, int* error_code);

extern int   pdf_page_builder_link_url(void* page, const char* url, int* error_code);
extern int   pdf_page_builder_link_page(void* page, size_t target_page, int* error_code);
extern int   pdf_page_builder_link_named(void* page, const char* destination, int* error_code);
extern int   pdf_page_builder_highlight(void* page, float r, float g, float b, int* error_code);
extern int   pdf_page_builder_underline(void* page, float r, float g, float b, int* error_code);
extern int   pdf_page_builder_strikeout(void* page, float r, float g, float b, int* error_code);
extern int   pdf_page_builder_squiggly(void* page, float r, float g, float b, int* error_code);
extern int   pdf_page_builder_sticky_note(void* page, const char* text, int* error_code);
extern int   pdf_page_builder_sticky_note_at(void* page, float x, float y, const char* text, int* error_code);
extern int   pdf_page_builder_watermark(void* page, const char* text, int* error_code);
extern int   pdf_page_builder_watermark_confidential(void* page, int* error_code);
extern int   pdf_page_builder_watermark_draft(void* page, int* error_code);
extern int   pdf_page_builder_stamp(void* page, const char* type_name, int* error_code);
extern int   pdf_page_builder_freetext(void* page, float x, float y, float w, float h,
                                       const char* text, int* error_code);

// Form fields
extern int   pdf_page_builder_text_field(void* page, const char* name,
                                         float x, float y, float w, float h,
                                         const char* default_value, int* error_code);
extern int   pdf_page_builder_checkbox(void* page, const char* name,
                                       float x, float y, float w, float h,
                                       int checked, int* error_code);
extern int   pdf_page_builder_combo_box(void* page, const char* name,
                                        float x, float y, float w, float h,
                                        const char* const* options,
                                        size_t options_count,
                                        const char* selected,
                                        int* error_code);
extern int   pdf_page_builder_radio_group(void* page, const char* name,
                                          const char* const* values,
                                          const float* xs, const float* ys,
                                          const float* ws, const float* hs,
                                          size_t count,
                                          const char* selected,
                                          int* error_code);
extern int   pdf_page_builder_push_button(void* page, const char* name,
                                          float x, float y, float w, float h,
                                          const char* caption, int* error_code);
extern int   pdf_page_builder_signature_field(void* page, const char* name,
                                              float x, float y, float w, float h,
                                              int* error_code);
extern int   pdf_page_builder_rect(void* page, float x, float y, float w, float h, int* error_code);
extern int   pdf_page_builder_filled_rect(void* page, float x, float y, float w, float h,
                                          float r, float g, float b, int* error_code);
extern int   pdf_page_builder_line(void* page, float x1, float y1, float x2, float y2, int* error_code);

// v0.3.39 primitives backing Go's Table / StreamingTable + friends.
extern int   pdf_page_builder_stroke_rect(void* page, float x, float y, float w, float h,
                                          float width, float r, float g, float b,
                                          int* error_code);
extern int   pdf_page_builder_stroke_line(void* page, float x1, float y1, float x2, float y2,
                                          float width, float r, float g, float b,
                                          int* error_code);
extern int   pdf_page_builder_text_in_rect(void* page, float x, float y, float w, float h,
                                           const char* text, int align, int* error_code);
extern int   pdf_page_builder_new_page_same_size(void* page, int* error_code);
extern int   pdf_page_builder_table(void* page,
                                    size_t n_columns,
                                    const float* widths,
                                    const int* aligns,
                                    size_t n_rows,
                                    const char* const* cell_strings,
                                    int has_header,
                                    int* error_code);

extern int   pdf_page_builder_done(void* page, int* error_code);
extern void  pdf_page_builder_free(void* page);

extern uint8_t* pdf_document_builder_build(void* handle, size_t* out_len, int* error_code);
extern int      pdf_document_builder_save(void* handle, const char* path, int* error_code);
extern int      pdf_document_builder_save_encrypted(void* handle, const char* path,
                                                    const char* user_password,
                                                    const char* owner_password,
                                                    int* error_code);
extern uint8_t* pdf_document_builder_to_bytes_encrypted(void* handle,
                                                        const char* user_password,
                                                        const char* owner_password,
                                                        size_t* out_len, int* error_code);

extern void* pdf_from_html_css(const char* html, const char* css,
                               const uint8_t* font_bytes, size_t font_len,
                               int* error_code);

extern void* pdf_from_html_css_with_fonts(const char* html, const char* css,
                                          const char* const* families,
                                          const uint8_t* const* font_bytes,
                                          const size_t* font_lens,
                                          size_t count,
                                          int* error_code);

// Byte buffers returned by `_build` / `_to_bytes_encrypted` are freed
// via the same `free_bytes` helper the rest of the package uses — but
// cgo preambles don't cross files, so we re-declare it here.
extern void free_bytes(void* ptr);
*/
import "C"

import (
	"errors"
	"fmt"
	"sync"
	"unsafe"
)

// ErrBuilderConsumed is returned when a DocumentBuilder is used after a
// terminal method (Build, Save, SaveEncrypted, ToBytesEncrypted) already
// consumed its handle.
var ErrBuilderConsumed = errors.New("pdf_oxide: DocumentBuilder has been consumed")

// ErrPageAlreadyCommitted is returned when Done is called twice on the
// same PageBuilder.
var ErrPageAlreadyCommitted = errors.New("pdf_oxide: page already committed")

// ErrBuilderHasOpenPage is returned when a DocumentBuilder operation is
// attempted while a PageBuilder is outstanding.
var ErrBuilderHasOpenPage = errors.New("pdf_oxide: a PageBuilder is still open — call Done first")

// ErrFontConsumed is returned when a consumed EmbeddedFont is re-used.
var ErrFontConsumed = errors.New("pdf_oxide: EmbeddedFont has been consumed")

// -----------------------------------------------------------------------------
// EmbeddedFont
// -----------------------------------------------------------------------------

// EmbeddedFont is a TTF/OTF font handle registerable with a DocumentBuilder.
// Single-use: RegisterEmbeddedFont transfers ownership and the handle becomes
// invalid. Always call Close unless the font has been registered.
type EmbeddedFont struct {
	mu       sync.Mutex
	handle   unsafe.Pointer
	consumed bool
}

// EmbeddedFontFromFile loads a TTF/OTF font from disk.
func EmbeddedFontFromFile(path string) (*EmbeddedFont, error) {
	cPath := C.CString(path)
	defer C.free(unsafe.Pointer(cPath))
	var ec C.int
	h := C.pdf_embedded_font_from_file(cPath, &ec)
	if h == nil {
		return nil, ffiError(ec)
	}
	return &EmbeddedFont{handle: h}, nil
}

// EmbeddedFontFromBytes loads a TTF/OTF font from a byte slice. Pass
// name="" to use the PostScript name from the font face.
func EmbeddedFontFromBytes(data []byte, name string) (*EmbeddedFont, error) {
	if len(data) == 0 {
		return nil, fmt.Errorf("pdf_oxide: EmbeddedFontFromBytes: data is empty")
	}
	var namePtr *C.char
	if name != "" {
		namePtr = C.CString(name)
		defer C.free(unsafe.Pointer(namePtr))
	}
	var ec C.int
	h := C.pdf_embedded_font_from_bytes(
		(*C.uint8_t)(unsafe.Pointer(&data[0])),
		C.size_t(len(data)),
		namePtr,
		&ec,
	)
	if h == nil {
		return nil, ffiError(ec)
	}
	return &EmbeddedFont{handle: h}, nil
}

// Close releases the native handle. No-op if the font was consumed by a
// DocumentBuilder.
func (f *EmbeddedFont) Close() error {
	f.mu.Lock()
	defer f.mu.Unlock()
	if !f.consumed && f.handle != nil {
		C.pdf_embedded_font_free(f.handle)
		f.handle = nil
		f.consumed = true
	}
	return nil
}

func (f *EmbeddedFont) takeHandle() (unsafe.Pointer, error) {
	f.mu.Lock()
	defer f.mu.Unlock()
	if f.consumed || f.handle == nil {
		return nil, ErrFontConsumed
	}
	h := f.handle
	f.handle = nil
	f.consumed = true
	return h, nil
}

// -----------------------------------------------------------------------------
// DocumentBuilder
// -----------------------------------------------------------------------------

// DocumentBuilder is the fluent top-level API for programmatic PDF
// construction. Not safe for concurrent use — one goroutine per builder.
type DocumentBuilder struct {
	mu       sync.Mutex
	handle   unsafe.Pointer
	consumed bool
	openPage *PageBuilder
}

// NewDocumentBuilder creates a fresh empty builder.
func NewDocumentBuilder() (*DocumentBuilder, error) {
	var ec C.int
	h := C.pdf_document_builder_create(&ec)
	if h == nil {
		return nil, ffiError(ec)
	}
	return &DocumentBuilder{handle: h}, nil
}

func (b *DocumentBuilder) checkUsable() error {
	if b.consumed || b.handle == nil {
		return ErrBuilderConsumed
	}
	if b.openPage != nil {
		return ErrBuilderHasOpenPage
	}
	return nil
}

// Close releases the builder's handle if it hasn't been consumed. Safe to
// call multiple times.
func (b *DocumentBuilder) Close() error {
	b.mu.Lock()
	defer b.mu.Unlock()
	if !b.consumed && b.handle != nil {
		C.pdf_document_builder_free(b.handle)
		b.handle = nil
		b.consumed = true
	}
	return nil
}

// --- Metadata setters -------------------------------------------------------

func (b *DocumentBuilder) setString(
	fn func(h unsafe.Pointer, s *C.char, ec *C.int) C.int,
	name string, value string,
) error {
	b.mu.Lock()
	defer b.mu.Unlock()
	if err := b.checkUsable(); err != nil {
		return err
	}
	cs := C.CString(value)
	defer C.free(unsafe.Pointer(cs))
	var ec C.int
	if fn(b.handle, cs, &ec) != 0 {
		return ffiError(ec)
	}
	return nil
}

// Title sets the document title.
func (b *DocumentBuilder) Title(title string) error {
	return b.setString(
		func(h unsafe.Pointer, s *C.char, ec *C.int) C.int {
			return C.pdf_document_builder_set_title(h, s, ec)
		}, "title", title)
}

// Author sets the document author.
func (b *DocumentBuilder) Author(author string) error {
	return b.setString(
		func(h unsafe.Pointer, s *C.char, ec *C.int) C.int {
			return C.pdf_document_builder_set_author(h, s, ec)
		}, "author", author)
}

// Subject sets the document subject.
func (b *DocumentBuilder) Subject(subject string) error {
	return b.setString(
		func(h unsafe.Pointer, s *C.char, ec *C.int) C.int {
			return C.pdf_document_builder_set_subject(h, s, ec)
		}, "subject", subject)
}

// Keywords sets the document keywords (comma-separated).
func (b *DocumentBuilder) Keywords(keywords string) error {
	return b.setString(
		func(h unsafe.Pointer, s *C.char, ec *C.int) C.int {
			return C.pdf_document_builder_set_keywords(h, s, ec)
		}, "keywords", keywords)
}

// Creator sets the creator application name.
func (b *DocumentBuilder) Creator(creator string) error {
	return b.setString(
		func(h unsafe.Pointer, s *C.char, ec *C.int) C.int {
			return C.pdf_document_builder_set_creator(h, s, ec)
		}, "creator", creator)
}

// OnOpen sets a JavaScript script to run when the document is opened (/OpenAction).
func (b *DocumentBuilder) OnOpen(script string) error {
	return b.setString(
		func(h unsafe.Pointer, s *C.char, ec *C.int) C.int {
			return C.pdf_document_builder_on_open(h, s, ec)
		}, "onOpen", script)
}

// TaggedPdfUa1 enables PDF/UA-1 tagged PDF mode.
//
// When enabled, Build emits /MarkInfo, /StructTreeRoot, /Lang, and
// /ViewerPreferences in the catalog. Opt-in — no effect unless called.
// Bundle F-1/F-2.
func (b *DocumentBuilder) TaggedPdfUa1() error {
	b.mu.Lock()
	defer b.mu.Unlock()
	if err := b.checkUsable(); err != nil {
		return err
	}
	var ec C.int
	if C.pdf_document_builder_tagged_pdf_ua1(b.handle, &ec) != 0 {
		return fmt.Errorf("taggedPdfUa1: error code %d", int(ec))
	}
	return nil
}

// Language sets the document's natural language tag, e.g. "en-US".
// Emitted as /Lang in the catalog when TaggedPdfUa1 is set. Bundle F-2.
func (b *DocumentBuilder) Language(lang string) error {
	return b.setString(
		func(h unsafe.Pointer, s *C.char, ec *C.int) C.int {
			return C.pdf_document_builder_language(h, s, ec)
		}, "language", lang)
}

// RoleMap adds a role-map entry: custom structure type → standard PDF type.
// Emitted in /RoleMap inside the StructTreeRoot when TaggedPdfUa1 is set.
// Multiple calls accumulate entries. Bundle F-4.
func (b *DocumentBuilder) RoleMap(custom, standard string) error {
	b.mu.Lock()
	defer b.mu.Unlock()
	if err := b.checkUsable(); err != nil {
		return err
	}
	cCustom := C.CString(custom)
	defer C.free(unsafe.Pointer(cCustom))
	cStandard := C.CString(standard)
	defer C.free(unsafe.Pointer(cStandard))
	var ec C.int
	if C.pdf_document_builder_role_map(b.handle, cCustom, cStandard, &ec) != 0 {
		return fmt.Errorf("roleMap: error code %d", int(ec))
	}
	return nil
}

// RegisterEmbeddedFont registers a TTF/OTF font under name. CONSUMES the
// EmbeddedFont on success — do not Close the font after.
func (b *DocumentBuilder) RegisterEmbeddedFont(name string, font *EmbeddedFont) error {
	b.mu.Lock()
	defer b.mu.Unlock()
	if err := b.checkUsable(); err != nil {
		return err
	}
	fontHandle, err := font.takeHandle()
	if err != nil {
		return err
	}
	cName := C.CString(name)
	defer C.free(unsafe.Pointer(cName))
	var ec C.int
	if C.pdf_document_builder_register_embedded_font(b.handle, cName, fontHandle, &ec) != 0 {
		// FFI did NOT consume the font on failure — restore the handle so
		// caller can still Close it. Document this in the method contract.
		font.mu.Lock()
		font.handle = fontHandle
		font.consumed = false
		font.mu.Unlock()
		return ffiError(ec)
	}
	return nil
}

// --- Page opening ----------------------------------------------------------

// A4Page starts a new A4 page. Only one page may be outstanding per builder.
func (b *DocumentBuilder) A4Page() (*PageBuilder, error) {
	return b.openPageInternal(func(h unsafe.Pointer, ec *C.int) unsafe.Pointer {
		return C.pdf_document_builder_a4_page(h, ec)
	})
}

// LetterPage starts a new US Letter page.
func (b *DocumentBuilder) LetterPage() (*PageBuilder, error) {
	return b.openPageInternal(func(h unsafe.Pointer, ec *C.int) unsafe.Pointer {
		return C.pdf_document_builder_letter_page(h, ec)
	})
}

// Page starts a page with custom dimensions in PDF points (72 pt = 1 inch).
func (b *DocumentBuilder) Page(width, height float32) (*PageBuilder, error) {
	return b.openPageInternal(func(h unsafe.Pointer, ec *C.int) unsafe.Pointer {
		return C.pdf_document_builder_page(h, C.float(width), C.float(height), ec)
	})
}

func (b *DocumentBuilder) openPageInternal(
	open func(h unsafe.Pointer, ec *C.int) unsafe.Pointer,
) (*PageBuilder, error) {
	b.mu.Lock()
	defer b.mu.Unlock()
	if err := b.checkUsable(); err != nil {
		return nil, err
	}
	var ec C.int
	page := open(b.handle, &ec)
	if page == nil {
		return nil, ffiError(ec)
	}
	p := &PageBuilder{parent: b, handle: page}
	b.openPage = p
	return p, nil
}

// --- Finalisation ----------------------------------------------------------

func (b *DocumentBuilder) consume() (unsafe.Pointer, error) {
	b.mu.Lock()
	defer b.mu.Unlock()
	if err := b.checkUsable(); err != nil {
		return nil, err
	}
	h := b.handle
	b.handle = nil
	b.consumed = true
	return h, nil
}

// Build compiles the PDF and returns the bytes. CONSUMES the builder.
func (b *DocumentBuilder) Build() ([]byte, error) {
	h, err := b.consume()
	if err != nil {
		return nil, err
	}
	defer C.pdf_document_builder_free(h)
	var outLen C.size_t
	var ec C.int
	ptr := C.pdf_document_builder_build(h, &outLen, &ec)
	if ptr == nil {
		return nil, ffiError(ec)
	}
	bytes := C.GoBytes(unsafe.Pointer(ptr), C.int(outLen))
	C.free_bytes(unsafe.Pointer(ptr))
	return bytes, nil
}

// Save writes the PDF to path. CONSUMES the builder.
func (b *DocumentBuilder) Save(path string) error {
	h, err := b.consume()
	if err != nil {
		return err
	}
	defer C.pdf_document_builder_free(h)
	cPath := C.CString(path)
	defer C.free(unsafe.Pointer(cPath))
	var ec C.int
	if C.pdf_document_builder_save(h, cPath, &ec) != 0 {
		return ffiError(ec)
	}
	return nil
}

// SaveEncrypted writes the PDF to path with AES-256 encryption.
// CONSUMES the builder.
func (b *DocumentBuilder) SaveEncrypted(path, userPassword, ownerPassword string) error {
	h, err := b.consume()
	if err != nil {
		return err
	}
	defer C.pdf_document_builder_free(h)
	cPath := C.CString(path)
	cUser := C.CString(userPassword)
	cOwner := C.CString(ownerPassword)
	defer C.free(unsafe.Pointer(cPath))
	defer C.free(unsafe.Pointer(cUser))
	defer C.free(unsafe.Pointer(cOwner))
	var ec C.int
	if C.pdf_document_builder_save_encrypted(h, cPath, cUser, cOwner, &ec) != 0 {
		return ffiError(ec)
	}
	return nil
}

// ToBytesEncrypted returns the PDF as encrypted bytes (AES-256).
// CONSUMES the builder.
func (b *DocumentBuilder) ToBytesEncrypted(userPassword, ownerPassword string) ([]byte, error) {
	h, err := b.consume()
	if err != nil {
		return nil, err
	}
	defer C.pdf_document_builder_free(h)
	cUser := C.CString(userPassword)
	cOwner := C.CString(ownerPassword)
	defer C.free(unsafe.Pointer(cUser))
	defer C.free(unsafe.Pointer(cOwner))
	var outLen C.size_t
	var ec C.int
	ptr := C.pdf_document_builder_to_bytes_encrypted(h, cUser, cOwner, &outLen, &ec)
	if ptr == nil {
		return nil, ffiError(ec)
	}
	bytes := C.GoBytes(unsafe.Pointer(ptr), C.int(outLen))
	C.free_bytes(unsafe.Pointer(ptr))
	return bytes, nil
}

// -----------------------------------------------------------------------------
// PageBuilder
// -----------------------------------------------------------------------------

// PageBuilder is the fluent per-page API returned by A4Page / LetterPage / Page.
// Each operation returns the same receiver so chains work idiomatically, but
// errors are stored and returned on Done — avoiding `if err != nil` after
// every chain link.
type PageBuilder struct {
	parent *DocumentBuilder
	handle unsafe.Pointer
	err    error // first error in the chain, if any
	done   bool
}

func (p *PageBuilder) checkUsable() bool {
	if p.err != nil || p.done || p.handle == nil {
		return false
	}
	return true
}

func (p *PageBuilder) callInt(fn func(h unsafe.Pointer, ec *C.int) C.int) *PageBuilder {
	if !p.checkUsable() {
		return p
	}
	var ec C.int
	if fn(p.handle, &ec) != 0 {
		p.err = ffiError(ec)
	}
	return p
}

// Font sets the font + size for subsequent text on this page.
func (p *PageBuilder) Font(name string, size float32) *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		cName := C.CString(name)
		defer C.free(unsafe.Pointer(cName))
		return C.pdf_page_builder_font(h, cName, C.float(size), ec)
	})
}

// At moves the cursor to absolute coordinates (PDF points from lower-left).
func (p *PageBuilder) At(x, y float32) *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		return C.pdf_page_builder_at(h, C.float(x), C.float(y), ec)
	})
}

// Text emits a line of text at the current cursor position.
func (p *PageBuilder) Text(text string) *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		cs := C.CString(text)
		defer C.free(unsafe.Pointer(cs))
		return C.pdf_page_builder_text(h, cs, ec)
	})
}

// Heading emits a heading. level is 1-6.
func (p *PageBuilder) Heading(level uint8, text string) *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		cs := C.CString(text)
		defer C.free(unsafe.Pointer(cs))
		return C.pdf_page_builder_heading(h, C.uchar(level), cs, ec)
	})
}

// Paragraph emits a paragraph with automatic line wrapping.
func (p *PageBuilder) Paragraph(text string) *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		cs := C.CString(text)
		defer C.free(unsafe.Pointer(cs))
		return C.pdf_page_builder_paragraph(h, cs, ec)
	})
}

// Space advances the cursor down by the given number of points.
func (p *PageBuilder) Space(points float32) *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		return C.pdf_page_builder_space(h, C.float(points), ec)
	})
}

// HorizontalRule draws a horizontal rule across the page.
func (p *PageBuilder) HorizontalRule() *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		return C.pdf_page_builder_horizontal_rule(h, ec)
	})
}

// --- Annotations (Phase 3) -------------------------------------------------

// LinkURL attaches a URL link to the previous text element.
func (p *PageBuilder) LinkURL(url string) *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		cs := C.CString(url)
		defer C.free(unsafe.Pointer(cs))
		return C.pdf_page_builder_link_url(h, cs, ec)
	})
}

// LinkPage links the previous text to an internal page (zero-based).
func (p *PageBuilder) LinkPage(pageIndex uint) *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		return C.pdf_page_builder_link_page(h, C.size_t(pageIndex), ec)
	})
}

// LinkNamed links the previous text to a named destination.
func (p *PageBuilder) LinkNamed(destination string) *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		cs := C.CString(destination)
		defer C.free(unsafe.Pointer(cs))
		return C.pdf_page_builder_link_named(h, cs, ec)
	})
}

// LinkJavascript links the previous text to a JavaScript action.
func (p *PageBuilder) LinkJavascript(script string) *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		cs := C.CString(script)
		defer C.free(unsafe.Pointer(cs))
		return C.pdf_page_builder_link_javascript(h, cs, ec)
	})
}

// OnOpen sets a JavaScript script to run when the page is opened (/AA /O).
func (p *PageBuilder) OnOpen(script string) *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		cs := C.CString(script)
		defer C.free(unsafe.Pointer(cs))
		return C.pdf_page_builder_on_open(h, cs, ec)
	})
}

// OnClose sets a JavaScript script to run when the page is closed (/AA /C).
func (p *PageBuilder) OnClose(script string) *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		cs := C.CString(script)
		defer C.free(unsafe.Pointer(cs))
		return C.pdf_page_builder_on_close(h, cs, ec)
	})
}

// FieldKeystroke sets a keystroke JS action (/AA /K) on the last form field.
func (p *PageBuilder) FieldKeystroke(script string) *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		cs := C.CString(script)
		defer C.free(unsafe.Pointer(cs))
		return C.pdf_page_builder_field_keystroke(h, cs, ec)
	})
}

// FieldFormat sets a format JS action (/AA /F) on the last form field.
func (p *PageBuilder) FieldFormat(script string) *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		cs := C.CString(script)
		defer C.free(unsafe.Pointer(cs))
		return C.pdf_page_builder_field_format(h, cs, ec)
	})
}

// FieldValidate sets a validate JS action (/AA /V) on the last form field.
func (p *PageBuilder) FieldValidate(script string) *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		cs := C.CString(script)
		defer C.free(unsafe.Pointer(cs))
		return C.pdf_page_builder_field_validate(h, cs, ec)
	})
}

// FieldCalculate sets a calculate JS action (/AA /C) on the last form field.
func (p *PageBuilder) FieldCalculate(script string) *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		cs := C.CString(script)
		defer C.free(unsafe.Pointer(cs))
		return C.pdf_page_builder_field_calculate(h, cs, ec)
	})
}

// Highlight highlights the previous text with an RGB colour (channels 0-1).
func (p *PageBuilder) Highlight(r, g, b float32) *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		return C.pdf_page_builder_highlight(h, C.float(r), C.float(g), C.float(b), ec)
	})
}

// Underline draws an underline under the previous text.
func (p *PageBuilder) Underline(r, g, b float32) *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		return C.pdf_page_builder_underline(h, C.float(r), C.float(g), C.float(b), ec)
	})
}

// Strikeout draws a strikethrough through the previous text.
func (p *PageBuilder) Strikeout(r, g, b float32) *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		return C.pdf_page_builder_strikeout(h, C.float(r), C.float(g), C.float(b), ec)
	})
}

// Squiggly draws a squiggly underline under the previous text.
func (p *PageBuilder) Squiggly(r, g, b float32) *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		return C.pdf_page_builder_squiggly(h, C.float(r), C.float(g), C.float(b), ec)
	})
}

// StickyNote attaches a sticky-note annotation to the previous text.
func (p *PageBuilder) StickyNote(text string) *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		cs := C.CString(text)
		defer C.free(unsafe.Pointer(cs))
		return C.pdf_page_builder_sticky_note(h, cs, ec)
	})
}

// StickyNoteAt places a sticky-note at an absolute position on the page.
func (p *PageBuilder) StickyNoteAt(x, y float32, text string) *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		cs := C.CString(text)
		defer C.free(unsafe.Pointer(cs))
		return C.pdf_page_builder_sticky_note_at(h, C.float(x), C.float(y), cs, ec)
	})
}

// Watermark applies a text watermark to the page.
func (p *PageBuilder) Watermark(text string) *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		cs := C.CString(text)
		defer C.free(unsafe.Pointer(cs))
		return C.pdf_page_builder_watermark(h, cs, ec)
	})
}

// WatermarkConfidential applies the standard "CONFIDENTIAL" diagonal watermark.
func (p *PageBuilder) WatermarkConfidential() *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		return C.pdf_page_builder_watermark_confidential(h, ec)
	})
}

// WatermarkDraft applies the standard "DRAFT" diagonal watermark.
func (p *PageBuilder) WatermarkDraft() *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		return C.pdf_page_builder_watermark_draft(h, ec)
	})
}

// Stamp attaches a standard stamp annotation at the cursor (150×50 default).
// typeName matches the PDF spec names — unknown names become custom stamps.
func (p *PageBuilder) Stamp(typeName string) *PageBuilder {
	return p.callInt(func(h unsafe.Pointer, ec *C.int) C.int {
		cs := C.CString(typeName)
		defer C.free(unsafe.Pointer(cs))
		return C.pdf_page_builder_stamp(h, cs, ec)
	})
}

// FreeText places a free-flowing text annotation inside (x, y, w, h).
func (p *PageBuilder) FreeText(x, y, w, h float32, text string) *PageBuilder {
	return p.callInt(func(hp unsafe.Pointer, ec *C.int) C.int {
		cs := C.CString(text)
		defer C.free(unsafe.Pointer(cs))
		return C.pdf_page_builder_freetext(hp, C.float(x), C.float(y), C.float(w), C.float(h), cs, ec)
	})
}

// TextField adds a single-line text form field at (x, y, w, h).
// Pass defaultValue="" for a blank field.
func (p *PageBuilder) TextField(name string, x, y, w, h float32, defaultValue string) *PageBuilder {
	return p.callInt(func(hp unsafe.Pointer, ec *C.int) C.int {
		cn := C.CString(name)
		defer C.free(unsafe.Pointer(cn))
		var cd *C.char
		if defaultValue != "" {
			cd = C.CString(defaultValue)
			defer C.free(unsafe.Pointer(cd))
		}
		return C.pdf_page_builder_text_field(hp, cn, C.float(x), C.float(y), C.float(w), C.float(h), cd, ec)
	})
}

// Checkbox adds a checkbox form field at (x, y, w, h).
func (p *PageBuilder) Checkbox(name string, x, y, w, h float32, checked bool) *PageBuilder {
	return p.callInt(func(hp unsafe.Pointer, ec *C.int) C.int {
		cn := C.CString(name)
		defer C.free(unsafe.Pointer(cn))
		var c C.int
		if checked {
			c = 1
		}
		return C.pdf_page_builder_checkbox(hp, cn, C.float(x), C.float(y), C.float(w), C.float(h), c, ec)
	})
}

// ComboBox adds a dropdown combo-box form field. Pass selected="" for no
// initial selection.
func (p *PageBuilder) ComboBox(name string, x, y, w, h float32, options []string, selected string) *PageBuilder {
	return p.callInt(func(hp unsafe.Pointer, ec *C.int) C.int {
		cn := C.CString(name)
		defer C.free(unsafe.Pointer(cn))
		cOpts := make([]*C.char, len(options))
		for i, s := range options {
			cOpts[i] = C.CString(s)
		}
		defer func() {
			for _, c := range cOpts {
				C.free(unsafe.Pointer(c))
			}
		}()
		var cSel *C.char
		if selected != "" {
			cSel = C.CString(selected)
			defer C.free(unsafe.Pointer(cSel))
		}
		var optsPtr **C.char
		if len(cOpts) > 0 {
			optsPtr = (**C.char)(unsafe.Pointer(&cOpts[0]))
		}
		return C.pdf_page_builder_combo_box(hp, cn, C.float(x), C.float(y), C.float(w), C.float(h),
			optsPtr, C.size_t(len(cOpts)), cSel, ec)
	})
}

// RadioButton describes one option of a radio group.
type RadioButton struct {
	Value      string
	X, Y, W, H float32
}

// RadioGroup adds a radio-button group. Each entry of buttons is one
// option with its own rect. Pass selected="" for no initial selection.
func (p *PageBuilder) RadioGroup(name string, buttons []RadioButton, selected string) *PageBuilder {
	return p.callInt(func(hp unsafe.Pointer, ec *C.int) C.int {
		cn := C.CString(name)
		defer C.free(unsafe.Pointer(cn))
		n := len(buttons)
		cVals := make([]*C.char, n)
		xs := make([]C.float, n)
		ys := make([]C.float, n)
		ws := make([]C.float, n)
		hs := make([]C.float, n)
		for i, b := range buttons {
			cVals[i] = C.CString(b.Value)
			xs[i] = C.float(b.X)
			ys[i] = C.float(b.Y)
			ws[i] = C.float(b.W)
			hs[i] = C.float(b.H)
		}
		defer func() {
			for _, c := range cVals {
				C.free(unsafe.Pointer(c))
			}
		}()
		var cSel *C.char
		if selected != "" {
			cSel = C.CString(selected)
			defer C.free(unsafe.Pointer(cSel))
		}
		var valsPtr **C.char
		var xsPtr, ysPtr, wsPtr, hsPtr *C.float
		if n > 0 {
			valsPtr = (**C.char)(unsafe.Pointer(&cVals[0]))
			xsPtr = (*C.float)(unsafe.Pointer(&xs[0]))
			ysPtr = (*C.float)(unsafe.Pointer(&ys[0]))
			wsPtr = (*C.float)(unsafe.Pointer(&ws[0]))
			hsPtr = (*C.float)(unsafe.Pointer(&hs[0]))
		}
		return C.pdf_page_builder_radio_group(hp, cn, valsPtr, xsPtr, ysPtr, wsPtr, hsPtr,
			C.size_t(n), cSel, ec)
	})
}

// PushButton adds a clickable push button with a visible caption.
func (p *PageBuilder) PushButton(name string, x, y, w, h float32, caption string) *PageBuilder {
	return p.callInt(func(hp unsafe.Pointer, ec *C.int) C.int {
		cn := C.CString(name)
		cc := C.CString(caption)
		defer C.free(unsafe.Pointer(cn))
		defer C.free(unsafe.Pointer(cc))
		return C.pdf_page_builder_push_button(hp, cn, C.float(x), C.float(y), C.float(w), C.float(h), cc, ec)
	})
}

// SignatureField adds an unsigned signature placeholder field (/FT /Sig) at the given bounds.
func (p *PageBuilder) SignatureField(name string, x, y, w, h float32) *PageBuilder {
	return p.callInt(func(hp unsafe.Pointer, ec *C.int) C.int {
		cn := C.CString(name)
		defer C.free(unsafe.Pointer(cn))
		return C.pdf_page_builder_signature_field(hp, cn, C.float(x), C.float(y), C.float(w), C.float(h), ec)
	})
}

// Barcode1d places a 1-D barcode image on the page.
// barcodeType: 0=Code128 1=Code39 2=EAN13 3=EAN8 4=UPCA 5=ITF 6=Code93 7=Codabar.
func (p *PageBuilder) Barcode1d(barcodeType int, data string, x, y, w, h float32) *PageBuilder {
	return p.callInt(func(hp unsafe.Pointer, ec *C.int) C.int {
		cd := C.CString(data)
		defer C.free(unsafe.Pointer(cd))
		return C.pdf_page_builder_barcode_1d(hp, C.int(barcodeType), cd,
			C.float(x), C.float(y), C.float(w), C.float(h), ec)
	})
}

// BarcodeQr places a QR-code image on the page (square: size × size pt).
func (p *PageBuilder) BarcodeQr(data string, x, y, size float32) *PageBuilder {
	return p.callInt(func(hp unsafe.Pointer, ec *C.int) C.int {
		cd := C.CString(data)
		defer C.free(unsafe.Pointer(cd))
		return C.pdf_page_builder_barcode_qr(hp, cd,
			C.float(x), C.float(y), C.float(size), ec)
	})
}

// Rect draws a stroked rectangle outline (1pt black).
func (p *PageBuilder) Rect(x, y, w, h float32) *PageBuilder {
	return p.callInt(func(hp unsafe.Pointer, ec *C.int) C.int {
		return C.pdf_page_builder_rect(hp, C.float(x), C.float(y), C.float(w), C.float(h), ec)
	})
}

// FilledRect draws a filled rectangle in RGB colour (channels 0-1).
func (p *PageBuilder) FilledRect(x, y, w, h, r, g, b float32) *PageBuilder {
	return p.callInt(func(hp unsafe.Pointer, ec *C.int) C.int {
		return C.pdf_page_builder_filled_rect(hp, C.float(x), C.float(y), C.float(w), C.float(h),
			C.float(r), C.float(g), C.float(b), ec)
	})
}

// Line draws a straight line from (x1, y1) to (x2, y2).
func (p *PageBuilder) Line(x1, y1, x2, y2 float32) *PageBuilder {
	return p.callInt(func(hp unsafe.Pointer, ec *C.int) C.int {
		return C.pdf_page_builder_line(hp, C.float(x1), C.float(y1), C.float(x2), C.float(y2), ec)
	})
}

// StrokeRect draws a stroked rectangle outline with the given line
// width and RGB colour (channels 0-1). Unlike Rect (which uses the
// writer's 1pt default), this primitive exposes full line style so it
// can back Table borders, highlight boxes, etc.
func (p *PageBuilder) StrokeRect(x, y, w, h, width, r, g, b float32) *PageBuilder {
	return p.callInt(func(hp unsafe.Pointer, ec *C.int) C.int {
		return C.pdf_page_builder_stroke_rect(hp, C.float(x), C.float(y), C.float(w), C.float(h),
			C.float(width), C.float(r), C.float(g), C.float(b), ec)
	})
}

// StrokeLine draws a straight line from (x1, y1) to (x2, y2) with the
// given width and RGB colour.
func (p *PageBuilder) StrokeLine(x1, y1, x2, y2, width, r, g, b float32) *PageBuilder {
	return p.callInt(func(hp unsafe.Pointer, ec *C.int) C.int {
		return C.pdf_page_builder_stroke_line(hp, C.float(x1), C.float(y1), C.float(x2), C.float(y2),
			C.float(width), C.float(r), C.float(g), C.float(b), ec)
	})
}

// TextInRect places wrapped text inside (x, y, w, h) with the given
// horizontal alignment. Unknown Alignment values fall back to AlignLeft
// on the writer side.
func (p *PageBuilder) TextInRect(x, y, w, h float32, text string, align Alignment) *PageBuilder {
	return p.callInt(func(hp unsafe.Pointer, ec *C.int) C.int {
		cs := C.CString(text)
		defer C.free(unsafe.Pointer(cs))
		return C.pdf_page_builder_text_in_rect(hp, C.float(x), C.float(y), C.float(w), C.float(h),
			cs, C.int(align), ec)
	})
}

// NewPageSameSize starts a fresh page with identical dimensions. The
// text configuration carries over; the cursor resets to the top-left
// margin. Callers wanting header-repeat-on-break must re-emit the
// header explicitly — this primitive does not do it automatically.
func (p *PageBuilder) NewPageSameSize() *PageBuilder {
	return p.callInt(func(hp unsafe.Pointer, ec *C.int) C.int {
		return C.pdf_page_builder_new_page_same_size(hp, ec)
	})
}

// Measure returns an estimated rendered width for text at the current
// font + size. v0.3.39 does not yet expose the Rust measure() method
// through the FFI, so this is a managed-side approximation using a
// conservative average glyph width in ems. It exists to match the
// binding surface documented in the v0.3.39 research doc; a native FFI
// path will replace it in a later release without changing the shape.
//
// The returned value is a rough estimate only — do not rely on it for
// precise layout; prefer TextInRect / Table for wrapped output.
func (p *PageBuilder) Measure(text string) float32 {
	// 0.5em per glyph is a safe upper bound for proportional fonts; the
	// caller usually just wants a sanity-check baseline.
	if !p.checkUsable() {
		return 0
	}
	// Without an FFI accessor we can't read the current font size back;
	// callers who need real measurement should wait for the native path.
	return float32(len(text)) * 0.5
}

// RemainingSpace is a placeholder for the managed height-budget
// accessor described in the v0.3.39 research doc. No FFI surface yet;
// returns 0 so callers written against the documented shape still
// compile. Replace with the native accessor when it lands.
func (p *PageBuilder) RemainingSpace() float32 {
	return 0
}

// Table places a buffered table at the current cursor. The whole row
// matrix is shipped to native in one FFI call; for very large tables
// (10k+ rows) prefer StreamingTable.
//
// Returns the same *PageBuilder so callers can chain further output
// below the table. Errors are deferred to Done like every other
// PageBuilder method; a nil/empty TableSpec or a row whose length
// disagrees with len(Columns) stops the chain with an error.
func (p *PageBuilder) Table(spec TableSpec) *PageBuilder {
	if !p.checkUsable() {
		return p
	}
	nCols := len(spec.Columns)
	if nCols == 0 {
		p.err = errors.New("pdf_oxide: Table: at least one column required")
		return p
	}
	for i, row := range spec.Rows {
		if len(row) != nCols {
			p.err = fmt.Errorf(
				"pdf_oxide: Table: row %d has %d cells, expected %d",
				i, len(row), nCols)
			return p
		}
	}

	// Build widths + aligns parallel arrays.
	widths := make([]C.float, nCols)
	aligns := make([]C.int, nCols)
	for i, c := range spec.Columns {
		widths[i] = C.float(c.Width)
		aligns[i] = C.int(c.Align)
	}

	// Build row-major cell-string matrix. If HasHeader is true the
	// first native row is the header, synthesised from Columns[i].Header.
	var nRows int
	if spec.HasHeader {
		nRows = 1 + len(spec.Rows)
	} else {
		nRows = len(spec.Rows)
	}
	total := nRows * nCols
	cStrs := make([]*C.char, total)
	defer func() {
		for _, s := range cStrs {
			if s != nil {
				C.free(unsafe.Pointer(s))
			}
		}
	}()
	off := 0
	if spec.HasHeader {
		for i, c := range spec.Columns {
			cStrs[off+i] = C.CString(c.Header)
		}
		off += nCols
	}
	for _, row := range spec.Rows {
		for i, cell := range row {
			cStrs[off+i] = C.CString(cell)
		}
		off += nCols
	}

	var widthsPtr *C.float
	var alignsPtr *C.int
	var cellsPtr **C.char
	if nCols > 0 {
		widthsPtr = (*C.float)(unsafe.Pointer(&widths[0]))
		alignsPtr = (*C.int)(unsafe.Pointer(&aligns[0]))
	}
	if total > 0 {
		cellsPtr = (**C.char)(unsafe.Pointer(&cStrs[0]))
	}
	var hasHeader C.int
	if spec.HasHeader {
		hasHeader = 1
	}
	var ec C.int
	if C.pdf_page_builder_table(
		p.handle,
		C.size_t(nCols),
		widthsPtr,
		alignsPtr,
		C.size_t(nRows),
		cellsPtr,
		hasHeader,
		&ec,
	) != 0 {
		p.err = ffiError(ec)
	}
	return p
}

// StreamingTable opens a row-at-a-time table adapter on this page.
//
// In v0.3.39 the Go adapter is managed-buffered: PushRow appends to an
// in-memory slice and Finish flushes a single Table() call. It exists
// so consumer code written against the streaming shape today keeps
// working when the native streaming FFI lands.
func (p *PageBuilder) StreamingTable(cfg StreamingTableConfig) *StreamingTable {
	return &StreamingTable{
		page:    p,
		columns: append([]Column(nil), cfg.Columns...),
	}
}

// StreamingTable is a row-at-a-time table adapter. Obtain one from
// PageBuilder.StreamingTable, feed rows with PushRow, and finalise with
// Finish to resume the page chain.
//
// The v0.3.39 implementation buffers rows in Go memory and flushes a
// single Table() call at Finish(). Finish is idempotent; subsequent
// calls return the same parent PageBuilder without re-emitting.
type StreamingTable struct {
	page     *PageBuilder
	columns  []Column
	rows     [][]string
	finished bool
}

// PushRow appends a row. len(cells) must equal the column count the
// adapter was opened with. Errors surface through the returned value
// AND through the parent PageBuilder (visible at Done()).
func (t *StreamingTable) PushRow(cells []string) error {
	if t == nil {
		return errors.New("pdf_oxide: PushRow on nil StreamingTable")
	}
	if t.finished {
		return errors.New("pdf_oxide: PushRow on finished StreamingTable")
	}
	if t.page == nil {
		return errors.New("pdf_oxide: StreamingTable has no parent page")
	}
	if len(cells) != len(t.columns) {
		return fmt.Errorf(
			"pdf_oxide: StreamingTable.PushRow: got %d cells, expected %d",
			len(cells), len(t.columns))
	}
	// Store a defensive copy so callers can reuse their row slice.
	row := make([]string, len(cells))
	copy(row, cells)
	t.rows = append(t.rows, row)
	return nil
}

// Finish flushes the buffered rows as a Table() call and returns the
// parent PageBuilder so callers can continue chaining. If the table
// was never populated Finish is a no-op (it still returns the parent).
func (t *StreamingTable) Finish() *PageBuilder {
	if t == nil || t.page == nil {
		return nil
	}
	if t.finished {
		return t.page
	}
	t.finished = true
	// Empty streaming tables emit nothing.
	if len(t.columns) == 0 && len(t.rows) == 0 {
		return t.page
	}
	spec := TableSpec{
		Columns:   t.columns,
		Rows:      t.rows,
		HasHeader: hasAnyHeader(t.columns),
	}
	return t.page.Table(spec)
}

func hasAnyHeader(cols []Column) bool {
	for _, c := range cols {
		if c.Header != "" {
			return true
		}
	}
	return false
}

// Done commits the page to the parent DocumentBuilder and returns any
// error accumulated during the chain. After Done the PageBuilder is
// invalid; reuse returns ErrPageAlreadyCommitted.
func (p *PageBuilder) Done() (*DocumentBuilder, error) {
	if p.done {
		return p.parent, ErrPageAlreadyCommitted
	}
	p.done = true
	parent := p.parent
	parent.mu.Lock()
	parent.openPage = nil
	parent.mu.Unlock()
	if p.err != nil {
		C.pdf_page_builder_free(p.handle)
		p.handle = nil
		return parent, p.err
	}
	var ec C.int
	rc := C.pdf_page_builder_done(p.handle, &ec)
	p.handle = nil
	if rc != 0 {
		return parent, ffiError(ec)
	}
	return parent, nil
}

// Close drops an uncommitted page handle. Use for error recovery —
// after Close, the parent's open-page slot is released so the next
// A4Page / etc. succeeds.
func (p *PageBuilder) Close() error {
	if p.done {
		return nil
	}
	p.done = true
	if p.handle != nil {
		C.pdf_page_builder_free(p.handle)
		p.handle = nil
	}
	if p.parent != nil {
		p.parent.mu.Lock()
		p.parent.openPage = nil
		p.parent.mu.Unlock()
	}
	return nil
}

// -----------------------------------------------------------------------------
// HTML+CSS pipeline
// -----------------------------------------------------------------------------

// FromHTMLCSS builds a PDF by rendering html with css applied, embedding
// fontBytes as the body font. Returns a *PdfCreator matching what
// FromMarkdown / FromHtml produce — same Save / SaveToBytes methods.
func FromHTMLCSS(html, css string, fontBytes []byte) (*PdfCreator, error) {
	if len(fontBytes) == 0 {
		return nil, fmt.Errorf("pdf_oxide: FromHTMLCSS: fontBytes is empty")
	}
	cHtml := C.CString(html)
	cCss := C.CString(css)
	defer C.free(unsafe.Pointer(cHtml))
	defer C.free(unsafe.Pointer(cCss))
	var ec C.int
	handle := C.pdf_from_html_css(
		cHtml,
		cCss,
		(*C.uint8_t)(unsafe.Pointer(&fontBytes[0])),
		C.size_t(len(fontBytes)),
		&ec,
	)
	if handle == nil {
		return nil, ffiError(ec)
	}
	return &PdfCreator{handle: handle}, nil
}

// FontEntry pairs a CSS font-family name with the TTF/OTF bytes that
// should back it. Used by FromHTMLCSSWithFonts.
type FontEntry struct {
	Family string
	Bytes  []byte
}

// FromHTMLCSSWithFonts builds a PDF from HTML+CSS with a multi-font
// cascade. The first entry is the default used when a CSS
// `font-family` doesn't match any registered family.
func FromHTMLCSSWithFonts(html, css string, fonts []FontEntry) (*PdfCreator, error) {
	if len(fonts) == 0 {
		return nil, fmt.Errorf("pdf_oxide: FromHTMLCSSWithFonts: fonts is empty")
	}
	cHtml := C.CString(html)
	cCss := C.CString(css)
	defer C.free(unsafe.Pointer(cHtml))
	defer C.free(unsafe.Pointer(cCss))

	n := len(fonts)
	cNames := make([]*C.char, n)
	cBytesPtrs := make([]*C.uint8_t, n)
	cLens := make([]C.size_t, n)
	for i, f := range fonts {
		if len(f.Bytes) == 0 {
			for j := 0; j < i; j++ {
				C.free(unsafe.Pointer(cNames[j]))
			}
			return nil, fmt.Errorf("pdf_oxide: FromHTMLCSSWithFonts: fonts[%d] has empty bytes", i)
		}
		cNames[i] = C.CString(f.Family)
		cBytesPtrs[i] = (*C.uint8_t)(unsafe.Pointer(&f.Bytes[0]))
		cLens[i] = C.size_t(len(f.Bytes))
	}
	defer func() {
		for _, n := range cNames {
			C.free(unsafe.Pointer(n))
		}
	}()

	var ec C.int
	handle := C.pdf_from_html_css_with_fonts(
		cHtml, cCss,
		(**C.char)(unsafe.Pointer(&cNames[0])),
		(**C.uint8_t)(unsafe.Pointer(&cBytesPtrs[0])),
		(*C.size_t)(unsafe.Pointer(&cLens[0])),
		C.size_t(n),
		&ec,
	)
	if handle == nil {
		return nil, ffiError(ec)
	}
	return &PdfCreator{handle: handle}, nil
}
