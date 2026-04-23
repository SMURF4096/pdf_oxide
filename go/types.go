// Package pdfoxide provides Go bindings to the pdf_oxide Rust PDF toolkit.
//
// This file holds the build-tag-agnostic type surface: error sentinels,
// the structured *Error value, and error-code mapping. Both the cgo
// backend (pdf_oxide.go, //go:build cgo) and the purego backend
// (pdf_oxide_purego.go, //go:build !cgo) build on top of these.
package pdfoxide

import (
	"errors"
	"fmt"
)

// Sentinel errors for errors.Is comparisons. Every failure path in this
// package reports one of these wrapped in an *Error for FFI errors, or
// returns the sentinel directly for non-FFI failures.
var (
	// ErrInvalidPath indicates the path argument was invalid. FFI code 1.
	ErrInvalidPath = errors.New("pdf_oxide: invalid path")
	// ErrDocumentNotFound indicates the document could not be opened. FFI code 2.
	ErrDocumentNotFound = errors.New("pdf_oxide: document not found")
	// ErrInvalidFormat indicates the PDF could not be parsed. FFI code 3.
	ErrInvalidFormat = errors.New("pdf_oxide: invalid PDF format")
	// ErrExtractionFailed indicates extraction failed. FFI code 4.
	ErrExtractionFailed = errors.New("pdf_oxide: extraction failed")
	// ErrParseError indicates a parse failure. FFI code 5.
	ErrParseError = errors.New("pdf_oxide: parse error")
	// ErrInvalidPageIndex indicates an out-of-range page index. FFI code 6.
	ErrInvalidPageIndex = errors.New("pdf_oxide: invalid page index")
	// ErrSearchFailed indicates a search operation failed. FFI code 7.
	ErrSearchFailed = errors.New("pdf_oxide: search failed")
	// ErrInternal indicates an internal/unknown error. FFI code 8.
	ErrInternal = errors.New("pdf_oxide: internal error")

	// ErrDocumentClosed indicates the document has been closed.
	ErrDocumentClosed = errors.New("pdf_oxide: document is closed")
	// ErrEditorClosed indicates the editor has been closed.
	ErrEditorClosed = errors.New("pdf_oxide: editor is closed")
	// ErrCreatorClosed indicates the PDF creator has been closed.
	ErrCreatorClosed = errors.New("pdf_oxide: creator is closed")
	// ErrIndexOutOfBounds indicates an out-of-range index.
	ErrIndexOutOfBounds = errors.New("pdf_oxide: index out of bounds")
	// ErrEmptyContent indicates required content was empty.
	ErrEmptyContent = errors.New("pdf_oxide: content must not be empty")

	// ErrNotImplementedInPurego is returned by methods that exist in the
	// cgo backend but have not yet been ported to the purego backend.
	// Build with CGO_ENABLED=1 to use them.
	ErrNotImplementedInPurego = errors.New("pdf_oxide: not implemented in pure-Go (purego) build; rebuild with CGO_ENABLED=1")
)

// Error is a structured PDF error that carries an FFI error code alongside a
// canonical sentinel. It implements Unwrap so errors.Is works with the exported
// Err* sentinels, and Is so two *Error values with the same Code compare equal.
type Error struct {
	Code     int
	Message  string
	sentinel error
}

// Error returns a human-readable description of the error.
func (e *Error) Error() string {
	if e.Message == "" {
		return fmt.Sprintf("pdf_oxide: error %d", e.Code)
	}
	return fmt.Sprintf("pdf_oxide: %s (code %d)", e.Message, e.Code)
}

// Unwrap returns the canonical sentinel so errors.Is(err, ErrInvalidPath) works.
func (e *Error) Unwrap() error { return e.sentinel }

// Is reports whether target is the same canonical sentinel, or another *Error
// carrying the same Code.
func (e *Error) Is(target error) bool {
	if e.sentinel != nil && target == e.sentinel {
		return true
	}
	var other *Error
	if errors.As(target, &other) {
		return e.Code == other.Code
	}
	return false
}

// ffiErrorFromInt wraps a plain int FFI error code into a fully populated
// *Error. Used by the purego backend, which speaks plain int32 rather
// than C.int. The cgo backend has its own typed wrapper (ffiError) that
// converts C.int before calling this.
func ffiErrorFromInt(code int) error {
	sentinel := sentinelForCode(code)
	return &Error{
		Code:     code,
		Message:  sentinel.Error(),
		sentinel: sentinel,
	}
}

// sentinelForCode returns the canonical sentinel for an FFI error code.
func sentinelForCode(code int) error {
	switch code {
	case 1:
		return ErrInvalidPath
	case 2:
		return ErrDocumentNotFound
	case 3:
		return ErrInvalidFormat
	case 4:
		return ErrExtractionFailed
	case 5:
		return ErrParseError
	case 6:
		return ErrInvalidPageIndex
	case 7:
		return ErrSearchFailed
	case 8:
		return ErrInternal
	default:
		return ErrInternal
	}
}

// ─── Extraction result types ────────────────────────────────────────────────
//
// These types are marshaled from JSON payloads returned by the Rust FFI's
// bulk extractors (`pdf_oxide_*_to_json`). The JSON tags match the Rust
// schema so one FFI call per list is enough for the Go layer.

// SearchResult represents a single search hit.
type SearchResult struct {
	Text   string  `json:"text"`
	Page   int     `json:"page"`
	X      float32 `json:"x"`
	Y      float32 `json:"y"`
	Width  float32 `json:"width"`
	Height float32 `json:"height"`
}

// Font represents a font embedded in or used by a PDF page.
type Font struct {
	Name       string  `json:"name"`
	Type       string  `json:"type"`
	Encoding   string  `json:"encoding"`
	IsEmbedded bool    `json:"isEmbedded"`
	IsSubset   bool    `json:"isSubset"`
	Size       float32 `json:"size"`
}

// Annotation represents a single annotation on a PDF page with all its
// metadata already materialized.
type Annotation struct {
	Type             string  `json:"type"`
	Subtype          string  `json:"subtype"`
	Content          string  `json:"content"`
	X                float32 `json:"x"`
	Y                float32 `json:"y"`
	Width            float32 `json:"width"`
	Height           float32 `json:"height"`
	Author           string  `json:"author"`
	BorderWidth      float32 `json:"borderWidth"`
	Color            uint32  `json:"color"`
	CreationDate     int64   `json:"creationDate"`
	ModificationDate int64   `json:"modificationDate"`
	LinkURI          string  `json:"linkURI"`
	TextIconName     string  `json:"textIconName"`
	IsHidden         bool    `json:"isHidden"`
	IsPrintable      bool    `json:"isPrintable"`
	IsReadOnly       bool    `json:"isReadOnly"`
	IsMarkedDeleted  bool    `json:"isMarkedDeleted"`
}

// Element represents a layout element on a PDF page (text block, image, etc.).
type Element struct {
	Type   string  `json:"type"`
	Text   string  `json:"text"`
	X      float32 `json:"x"`
	Y      float32 `json:"y"`
	Width  float32 `json:"width"`
	Height float32 `json:"height"`
}

// LogLevel represents the log verbosity level.
type LogLevel int

const (
	// LogOff disables all logging.
	LogOff LogLevel = 0
	// LogError enables error messages only.
	LogError LogLevel = 1
	// LogWarn enables warnings and errors.
	LogWarn LogLevel = 2
	// LogInfo enables informational messages.
	LogInfo LogLevel = 3
	// LogDebug enables debug messages.
	LogDebug LogLevel = 4
	// LogTrace enables verbose trace messages.
	LogTrace LogLevel = 5
)
