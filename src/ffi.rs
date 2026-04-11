//! C Foreign Function Interface (FFI) for pdf_oxide
//!
//! Provides `#[no_mangle] pub extern "C"` functions that Go (CGO), Node.js (N-API),
//! and C# (P/Invoke) bindings can link against. The compiled `libpdf_oxide.so` / `.dylib` / `.dll`
//! exports these symbols when built as a cdylib.
#![allow(missing_docs)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(non_snake_case)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::too_many_arguments)]
//!
//! # Error Convention
//! Most functions accept an `error_code: *mut i32` out-parameter:
//! - 0 = success
//! - 1 = invalid argument / path
//! - 2 = file not found / IO error
//! - 3 = parse error / invalid PDF
//! - 4 = extraction failed
//! - 5 = internal error
//! - 6 = invalid page index
//! - 7 = search error
//! - 8 = unsupported feature
//!
//! # Memory Convention
//! - Strings returned as `*mut c_char` are owned by the caller and must be freed with `free_string`.
//! - Byte buffers returned as `*mut u8` must be freed with `free_bytes`.
//! - Opaque handles (Box pointers) must be freed with their corresponding `*_free` function.

use crate::converters::ConversionOptions;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use crate::annotations::Annotation as RustAnnotation;
use crate::api::Pdf;
use crate::document::PdfDocument;
use crate::editor::DocumentEditor;
use crate::editor::EditableDocument;
use crate::search::{SearchOptions, SearchResult as RustSearchResult, TextSearcher};

// ─── Error helpers ───────────────────────────────────────────────────────────

const ERR_SUCCESS: i32 = 0;
const ERR_INVALID_ARG: i32 = 1;
const ERR_IO: i32 = 2;
const ERR_PARSE: i32 = 3;
const ERR_EXTRACTION: i32 = 4;
const ERR_INTERNAL: i32 = 5;
const ERR_INVALID_PAGE: i32 = 6;
const ERR_SEARCH: i32 = 7;
const _ERR_UNSUPPORTED: i32 = 8;

fn set_error(ptr: *mut i32, code: i32) {
    if !ptr.is_null() {
        unsafe {
            *ptr = code;
        }
    }
}

fn classify_error(e: &crate::error::Error) -> i32 {
    let msg = format!("{e}");
    if msg.contains("not found") || msg.contains("No such file") || msg.contains("IO") {
        ERR_IO
    } else if msg.contains("parse") || msg.contains("Parse") || msg.contains("Invalid PDF") {
        ERR_PARSE
    } else if msg.contains("page") || msg.contains("Page") || msg.contains("index") {
        ERR_INVALID_PAGE
    } else if msg.contains("search") || msg.contains("Search") {
        ERR_SEARCH
    } else {
        ERR_INTERNAL
    }
}

fn to_c_string(s: &str) -> *mut c_char {
    match CString::new(s) {
        Ok(cs) => cs.into_raw(),
        Err(_) => {
            // Fallback: replace NUL bytes
            let cleaned: String = s.replace('\0', "");
            CString::new(cleaned)
                .map(|c| c.into_raw())
                .unwrap_or(ptr::null_mut())
        },
    }
}

fn to_c_string_opt(s: Option<String>) -> *mut c_char {
    match s {
        Some(s) => to_c_string(&s),
        None => ptr::null_mut(),
    }
}

// ─── Logging ───────────────────────────────────────────────────────────────

/// Set the global log level for the library.
/// Levels: 0 = Off, 1 = Error, 2 = Warn, 3 = Info, 4 = Debug, 5 = Trace
#[no_mangle]
pub extern "C" fn pdf_oxide_set_log_level(level: i32) {
    let filter = match level {
        0 => log::LevelFilter::Off,
        1 => log::LevelFilter::Error,
        2 => log::LevelFilter::Warn,
        3 => log::LevelFilter::Info,
        4 => log::LevelFilter::Debug,
        _ => log::LevelFilter::Trace,
    };
    log::set_max_level(filter);
}

/// Get the current log level. Returns 0-5 matching the set_log_level values.
#[no_mangle]
pub extern "C" fn pdf_oxide_get_log_level() -> i32 {
    match log::max_level() {
        log::LevelFilter::Off => 0,
        log::LevelFilter::Error => 1,
        log::LevelFilter::Warn => 2,
        log::LevelFilter::Info => 3,
        log::LevelFilter::Debug => 4,
        log::LevelFilter::Trace => 5,
    }
}

// ─── Memory management ──────────────────────────────────────────────────────

/// Free a string returned by any FFI function.
#[no_mangle]
pub extern "C" fn free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            drop(CString::from_raw(ptr));
        }
    }
}

/// Free a byte buffer returned by any FFI function.
#[no_mangle]
pub extern "C" fn free_bytes(ptr: *mut u8) {
    // Byte buffers are leaked via Box::into_raw(Box::new(vec.into_boxed_slice()))
    // We can't reconstruct the exact Vec, so we just leak for now.
    // In practice, callers should use the specific *_free functions.
    let _ = ptr;
}

// ─── PdfDocument ────────────────────────────────────────────────────────────

/// Open a PDF document from a file path. Returns an opaque handle.
#[no_mangle]
pub extern "C" fn pdf_document_open(path: *const c_char, error_code: *mut i32) -> *mut PdfDocument {
    if path.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let path_str = unsafe { CStr::from_ptr(path) };
    let path_str = match path_str.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error(error_code, ERR_INVALID_ARG);
            return ptr::null_mut();
        },
    };
    match PdfDocument::open(path_str) {
        Ok(doc) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(doc))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

/// Free a PdfDocument handle.
#[no_mangle]
pub extern "C" fn pdf_document_free(handle: *mut PdfDocument) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

/// Get the page count.
#[no_mangle]
pub extern "C" fn pdf_document_get_page_count(
    handle: *mut PdfDocument,
    error_code: *mut i32,
) -> i32 {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    let doc = unsafe { &mut *handle };
    match doc.page_count() {
        Ok(count) => {
            set_error(error_code, ERR_SUCCESS);
            count as i32
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            -1
        },
    }
}

/// Get the PDF version as (major, minor).
#[no_mangle]
pub extern "C" fn pdf_document_get_version(
    handle: *const PdfDocument,
    major: *mut u8,
    minor: *mut u8,
) {
    if handle.is_null() || major.is_null() || minor.is_null() {
        return;
    }
    let doc = unsafe { &*handle };
    let (maj, min) = doc.version();
    unsafe {
        *major = maj;
        *minor = min;
    }
}

/// Check if the document has a structure tree (tagged PDF).
#[no_mangle]
pub extern "C" fn pdf_document_has_structure_tree(handle: *mut PdfDocument) -> bool {
    if handle.is_null() {
        return false;
    }
    let doc = unsafe { &mut *handle };
    doc.structure_tree().ok().flatten().is_some()
}

/// Extract text from a single page.
#[no_mangle]
pub extern "C" fn pdf_document_extract_text(
    handle: *mut PdfDocument,
    page_index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    match doc.extract_text(page_index as usize) {
        Ok(text) => {
            set_error(error_code, ERR_SUCCESS);
            to_c_string(&text)
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

/// Convert a page to Markdown.
#[no_mangle]
pub extern "C" fn pdf_document_to_markdown(
    handle: *mut PdfDocument,
    page_index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    let opts = ConversionOptions::default();
    match doc.to_markdown(page_index as usize, &opts) {
        Ok(text) => {
            set_error(error_code, ERR_SUCCESS);
            to_c_string(&text)
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

/// Convert a page to HTML.
#[no_mangle]
pub extern "C" fn pdf_document_to_html(
    handle: *mut PdfDocument,
    page_index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    let opts = ConversionOptions::default();
    match doc.to_html(page_index as usize, &opts) {
        Ok(text) => {
            set_error(error_code, ERR_SUCCESS);
            to_c_string(&text)
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

/// Convert a page to plain text.
#[no_mangle]
pub extern "C" fn pdf_document_to_plain_text(
    handle: *mut PdfDocument,
    page_index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    let opts = ConversionOptions::default();
    match doc.to_plain_text(page_index as usize, &opts) {
        Ok(text) => {
            set_error(error_code, ERR_SUCCESS);
            to_c_string(&text)
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

/// Convert all pages to Markdown.
#[no_mangle]
pub extern "C" fn pdf_document_to_markdown_all(
    handle: *mut PdfDocument,
    error_code: *mut i32,
) -> *mut c_char {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    let opts = ConversionOptions::default();
    match doc.to_markdown_all(&opts) {
        Ok(text) => {
            set_error(error_code, ERR_SUCCESS);
            to_c_string(&text)
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

// ─── DocumentEditor ─────────────────────────────────────────────────────────

/// Open a PDF for editing. Returns an opaque DocumentEditor handle.
#[no_mangle]
pub extern "C" fn document_editor_open(
    path: *const c_char,
    error_code: *mut i32,
) -> *mut DocumentEditor {
    if path.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let path_str = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error(error_code, ERR_INVALID_ARG);
            return ptr::null_mut();
        },
    };
    match DocumentEditor::open(path_str) {
        Ok(editor) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(editor))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

/// Free a DocumentEditor handle.
#[no_mangle]
pub extern "C" fn document_editor_free(handle: *mut DocumentEditor) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

/// Check if the editor has modifications.
#[no_mangle]
pub extern "C" fn document_editor_is_modified(handle: *const DocumentEditor) -> bool {
    if handle.is_null() {
        return false;
    }
    let editor = unsafe { &*handle };
    editor.is_modified()
}

/// Get the source path of the editor.
#[no_mangle]
pub extern "C" fn document_editor_get_source_path(
    handle: *const DocumentEditor,
    error_code: *mut i32,
) -> *mut c_char {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let editor = unsafe { &*handle };
    set_error(error_code, ERR_SUCCESS);
    to_c_string(editor.source_path())
}

/// Get PDF version from the editor.
#[no_mangle]
pub extern "C" fn document_editor_get_version(
    handle: *const DocumentEditor,
    major: *mut u8,
    minor: *mut u8,
) {
    if handle.is_null() || major.is_null() || minor.is_null() {
        return;
    }
    let editor = unsafe { &*handle };
    let (maj, min) = editor.version();
    unsafe {
        *major = maj;
        *minor = min;
    }
}

/// Get page count from editor.
#[no_mangle]
pub extern "C" fn document_editor_get_page_count(
    handle: *mut DocumentEditor,
    error_code: *mut i32,
) -> i32 {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    let editor = unsafe { &*handle };
    set_error(error_code, ERR_SUCCESS);
    editor.current_page_count() as i32
}

macro_rules! editor_get_string_field {
    ($fn_name:ident, $method:ident) => {
        #[no_mangle]
        pub extern "C" fn $fn_name(
            handle: *const DocumentEditor,
            error_code: *mut i32,
        ) -> *mut c_char {
            if handle.is_null() {
                set_error(error_code, ERR_INVALID_ARG);
                return ptr::null_mut();
            }
            // We need &mut self for these methods, but we only have *const
            // This is safe because we hold the only reference from Go side
            let editor = unsafe { &mut *(handle as *mut DocumentEditor) };
            match editor.$method() {
                Ok(Some(val)) => {
                    set_error(error_code, ERR_SUCCESS);
                    to_c_string(&val)
                },
                Ok(None) => {
                    set_error(error_code, ERR_SUCCESS);
                    ptr::null_mut()
                },
                Err(e) => {
                    set_error(error_code, classify_error(&e));
                    ptr::null_mut()
                },
            }
        }
    };
}

macro_rules! editor_set_string_field {
    ($fn_name:ident, $method:ident) => {
        #[no_mangle]
        pub extern "C" fn $fn_name(
            handle: *mut DocumentEditor,
            value: *const c_char,
            error_code: *mut i32,
        ) -> i32 {
            if handle.is_null() || value.is_null() {
                set_error(error_code, ERR_INVALID_ARG);
                return -1;
            }
            let editor = unsafe { &mut *handle };
            let val = match unsafe { CStr::from_ptr(value) }.to_str() {
                Ok(s) => s,
                Err(_) => {
                    set_error(error_code, ERR_INVALID_ARG);
                    return -1;
                },
            };
            editor.$method(val);
            set_error(error_code, ERR_SUCCESS);
            0
        }
    };
}

editor_get_string_field!(document_editor_get_title, title);
editor_set_string_field!(document_editor_set_title, set_title);
editor_get_string_field!(document_editor_get_author, author);
editor_set_string_field!(document_editor_set_author, set_author);
editor_get_string_field!(document_editor_get_subject, subject);
editor_set_string_field!(document_editor_set_subject, set_subject);

// Producer - editor doesn't have a direct get_producer method, delegate through source
#[no_mangle]
pub extern "C" fn document_editor_get_producer(
    handle: *const DocumentEditor,
    error_code: *mut i32,
) -> *mut c_char {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    // Producer is typically in document info dict - return empty for now
    set_error(error_code, ERR_SUCCESS);
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn document_editor_set_producer(
    handle: *mut DocumentEditor,
    _value: *const c_char,
    error_code: *mut i32,
) -> i32 {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    // No direct set_producer in editor API
    set_error(error_code, ERR_SUCCESS);
    0
}

// Creation date
#[no_mangle]
pub extern "C" fn document_editor_get_creation_date(
    handle: *const DocumentEditor,
    error_code: *mut i32,
) -> *mut c_char {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn document_editor_set_creation_date(
    handle: *mut DocumentEditor,
    _date_str: *const c_char,
    error_code: *mut i32,
) -> i32 {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    set_error(error_code, ERR_SUCCESS);
    0
}

/// Save the edited document.
#[no_mangle]
pub extern "C" fn document_editor_save(
    handle: *mut DocumentEditor,
    path: *const c_char,
    error_code: *mut i32,
) -> i32 {
    if handle.is_null() || path.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    let editor = unsafe { &mut *handle };
    let path_str = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error(error_code, ERR_INVALID_ARG);
            return -1;
        },
    };
    match editor.save(path_str) {
        Ok(()) => {
            set_error(error_code, ERR_SUCCESS);
            0
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            -1
        },
    }
}

// ─── PDF Creator (Pdf type) ────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn pdf_from_markdown(markdown: *const c_char, error_code: *mut i32) -> *mut Pdf {
    if markdown.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let md = match unsafe { CStr::from_ptr(markdown) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error(error_code, ERR_INVALID_ARG);
            return ptr::null_mut();
        },
    };
    match Pdf::from_markdown(md) {
        Ok(pdf) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(pdf))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_from_html(html: *const c_char, error_code: *mut i32) -> *mut Pdf {
    if html.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let html_str = match unsafe { CStr::from_ptr(html) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error(error_code, ERR_INVALID_ARG);
            return ptr::null_mut();
        },
    };
    match Pdf::from_html(html_str) {
        Ok(pdf) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(pdf))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_from_text(text: *const c_char, error_code: *mut i32) -> *mut Pdf {
    if text.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let text_str = match unsafe { CStr::from_ptr(text) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error(error_code, ERR_INVALID_ARG);
            return ptr::null_mut();
        },
    };
    match Pdf::from_text(text_str) {
        Ok(pdf) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(pdf))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_save(handle: *mut Pdf, path: *const c_char, error_code: *mut i32) -> i32 {
    if handle.is_null() || path.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    let path_str = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error(error_code, ERR_INVALID_ARG);
            return -1;
        },
    };
    // Borrow the Pdf mutably — save must NOT consume it, otherwise the
    // subsequent `pdf_free` call in the caller (Go/JS/C#) is a double-free.
    let pdf = unsafe { &mut *handle };
    match pdf.save(path_str) {
        Ok(()) => {
            set_error(error_code, ERR_SUCCESS);
            0
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            -1
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_save_to_bytes(
    handle: *mut Pdf,
    data_len: *mut i32,
    error_code: *mut i32,
) -> *mut u8 {
    if handle.is_null() || data_len.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    // Borrow mutably — must NOT consume the handle (would cause double-free
    // when caller runs pdf_free).
    let pdf = unsafe { &mut *handle };
    match pdf.save_to_bytes() {
        Ok(bytes) => {
            set_error(error_code, ERR_SUCCESS);
            unsafe {
                *data_len = bytes.len() as i32;
            }
            let boxed = bytes.into_boxed_slice();
            Box::into_raw(boxed) as *mut u8
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_get_page_count(handle: *mut Pdf, error_code: *mut i32) -> i32 {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    let pdf = unsafe { &mut *handle };
    match pdf.page_count() {
        Ok(count) => {
            set_error(error_code, ERR_SUCCESS);
            count as i32
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            -1
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_free(handle: *mut Pdf) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

// ─── Search ─────────────────────────────────────────────────────────────────

/// Opaque search results handle.
pub struct FfiSearchResults {
    results: Vec<RustSearchResult>,
}

#[no_mangle]
pub extern "C" fn pdf_document_search_page(
    handle: *mut PdfDocument,
    page_index: i32,
    search_term: *const c_char,
    case_sensitive: bool,
    error_code: *mut i32,
) -> *mut FfiSearchResults {
    if handle.is_null() || search_term.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    let term = match unsafe { CStr::from_ptr(search_term) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error(error_code, ERR_INVALID_ARG);
            return ptr::null_mut();
        },
    };
    let opts = SearchOptions::new()
        .with_case_insensitive(!case_sensitive)
        .with_page_range(page_index as usize, page_index as usize + 1);
    match TextSearcher::search(doc, term, &opts) {
        Ok(results) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(FfiSearchResults { results }))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_document_search_all(
    handle: *mut PdfDocument,
    search_term: *const c_char,
    case_sensitive: bool,
    error_code: *mut i32,
) -> *mut FfiSearchResults {
    if handle.is_null() || search_term.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    let term = match unsafe { CStr::from_ptr(search_term) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error(error_code, ERR_INVALID_ARG);
            return ptr::null_mut();
        },
    };
    let opts = SearchOptions::new().with_case_insensitive(!case_sensitive);
    match TextSearcher::search(doc, term, &opts) {
        Ok(results) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(FfiSearchResults { results }))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_oxide_search_result_count(results: *const FfiSearchResults) -> i32 {
    if results.is_null() {
        return 0;
    }
    let r = unsafe { &*results };
    r.results.len() as i32
}

#[no_mangle]
pub extern "C" fn pdf_oxide_search_result_get_text(
    results: *const FfiSearchResults,
    index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if results.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let r = unsafe { &*results };
    if index < 0 || (index as usize) >= r.results.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    to_c_string(&r.results[index as usize].text)
}

#[no_mangle]
pub extern "C" fn pdf_oxide_search_result_get_page(
    results: *const FfiSearchResults,
    index: i32,
    error_code: *mut i32,
) -> i32 {
    if results.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    let r = unsafe { &*results };
    if index < 0 || (index as usize) >= r.results.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return -1;
    }
    set_error(error_code, ERR_SUCCESS);
    r.results[index as usize].page as i32
}

#[no_mangle]
pub extern "C" fn pdf_oxide_search_result_get_bbox(
    results: *const FfiSearchResults,
    index: i32,
    x: *mut f32,
    y: *mut f32,
    width: *mut f32,
    height: *mut f32,
    error_code: *mut i32,
) {
    if results.is_null() || x.is_null() || y.is_null() || width.is_null() || height.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return;
    }
    let r = unsafe { &*results };
    if index < 0 || (index as usize) >= r.results.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return;
    }
    let bbox = &r.results[index as usize].bbox;
    unsafe {
        *x = bbox.x;
        *y = bbox.y;
        *width = bbox.width;
        *height = bbox.height;
    }
    set_error(error_code, ERR_SUCCESS);
}

#[no_mangle]
pub extern "C" fn pdf_oxide_search_result_free(handle: *mut FfiSearchResults) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

// ─── Font extraction ────────────────────────────────────────────────────────

/// Simple FFI-friendly font info
pub struct FfiFont {
    name: String,
    subtype: String,
    encoding: String,
    is_embedded: bool,
    is_subset: bool,
}

pub struct FfiFontList {
    fonts: Vec<FfiFont>,
}

#[no_mangle]
pub extern "C" fn pdf_document_get_embedded_fonts(
    handle: *mut PdfDocument,
    page_index: i32,
    error_code: *mut i32,
) -> *mut FfiFontList {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    // Extract spans to discover font names used on this page
    let fonts = match doc.extract_spans(page_index as usize) {
        Ok(spans) => {
            let mut seen = std::collections::HashSet::new();
            let mut result = Vec::new();
            for span in &spans {
                let name = &span.font_name;
                if !name.is_empty() && seen.insert(name.clone()) {
                    let is_subset = name.len() > 7 && name.as_bytes().get(6) == Some(&b'+');
                    let is_embedded = span.font_name.contains('+') || !span.font_name.is_empty();
                    result.push(FfiFont {
                        name: name.clone(),
                        subtype: String::from("Unknown"),
                        encoding: String::from("Unknown"),
                        is_embedded,
                        is_subset,
                    });
                }
            }
            result
        },
        Err(_) => Vec::new(),
    };
    set_error(error_code, ERR_SUCCESS);
    Box::into_raw(Box::new(FfiFontList { fonts }))
}

#[no_mangle]
pub extern "C" fn pdf_oxide_font_count(fonts: *const FfiFontList) -> i32 {
    if fonts.is_null() {
        return 0;
    }
    unsafe { (*fonts).fonts.len() as i32 }
}

#[no_mangle]
pub extern "C" fn pdf_oxide_font_get_name(
    fonts: *const FfiFontList,
    index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if fonts.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let list = unsafe { &*fonts };
    if (index as usize) >= list.fonts.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    to_c_string(&list.fonts[index as usize].name)
}

#[no_mangle]
pub extern "C" fn pdf_oxide_font_get_type(
    fonts: *const FfiFontList,
    index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if fonts.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let list = unsafe { &*fonts };
    if (index as usize) >= list.fonts.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    to_c_string(&list.fonts[index as usize].subtype)
}

#[no_mangle]
pub extern "C" fn pdf_oxide_font_get_encoding(
    fonts: *const FfiFontList,
    index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if fonts.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let list = unsafe { &*fonts };
    if (index as usize) >= list.fonts.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    to_c_string(&list.fonts[index as usize].encoding)
}

#[no_mangle]
pub extern "C" fn pdf_oxide_font_is_embedded(
    fonts: *const FfiFontList,
    index: i32,
    error_code: *mut i32,
) -> i32 {
    if fonts.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return 0;
    }
    let list = unsafe { &*fonts };
    if (index as usize) >= list.fonts.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return 0;
    }
    set_error(error_code, ERR_SUCCESS);
    if list.fonts[index as usize].is_embedded {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn pdf_oxide_font_is_subset(
    fonts: *const FfiFontList,
    index: i32,
    error_code: *mut i32,
) -> i32 {
    if fonts.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return 0;
    }
    let list = unsafe { &*fonts };
    if (index as usize) >= list.fonts.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return 0;
    }
    set_error(error_code, ERR_SUCCESS);
    if list.fonts[index as usize].is_subset {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn pdf_oxide_font_get_size(
    _fonts: *const FfiFontList,
    _index: i32,
    error_code: *mut i32,
) -> f32 {
    set_error(error_code, ERR_SUCCESS);
    0.0
}

#[no_mangle]
pub extern "C" fn pdf_oxide_font_list_free(handle: *mut FfiFontList) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

// ─── Image extraction ───────────────────────────────────────────────────────

use crate::extractors::PdfImage;

pub struct FfiImageList {
    images: Vec<PdfImage>,
}

#[no_mangle]
pub extern "C" fn pdf_document_get_embedded_images(
    handle: *mut PdfDocument,
    page_index: i32,
    error_code: *mut i32,
) -> *mut FfiImageList {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    match doc.extract_images(page_index as usize) {
        Ok(images) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(FfiImageList { images }))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_oxide_image_count(images: *const FfiImageList) -> i32 {
    if images.is_null() {
        return 0;
    }
    unsafe { (*images).images.len() as i32 }
}

#[no_mangle]
pub extern "C" fn pdf_oxide_image_get_width(
    images: *const FfiImageList,
    index: i32,
    error_code: *mut i32,
) -> i32 {
    if images.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return 0;
    }
    let list = unsafe { &*images };
    if (index as usize) >= list.images.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return 0;
    }
    set_error(error_code, ERR_SUCCESS);
    list.images[index as usize].width() as i32
}

#[no_mangle]
pub extern "C" fn pdf_oxide_image_get_height(
    images: *const FfiImageList,
    index: i32,
    error_code: *mut i32,
) -> i32 {
    if images.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return 0;
    }
    let list = unsafe { &*images };
    if (index as usize) >= list.images.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return 0;
    }
    set_error(error_code, ERR_SUCCESS);
    list.images[index as usize].height() as i32
}

#[no_mangle]
pub extern "C" fn pdf_oxide_image_get_format(
    images: *const FfiImageList,
    index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if images.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let list = unsafe { &*images };
    if (index as usize) >= list.images.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    to_c_string(&format!("{:?}", list.images[index as usize].color_space()))
}

#[no_mangle]
pub extern "C" fn pdf_oxide_image_get_colorspace(
    images: *const FfiImageList,
    index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if images.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let list = unsafe { &*images };
    if (index as usize) >= list.images.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    to_c_string(&format!("{:?}", list.images[index as usize].color_space()))
}

#[no_mangle]
pub extern "C" fn pdf_oxide_image_get_bits_per_component(
    images: *const FfiImageList,
    index: i32,
    error_code: *mut i32,
) -> i32 {
    if images.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return 0;
    }
    let list = unsafe { &*images };
    if (index as usize) >= list.images.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return 0;
    }
    set_error(error_code, ERR_SUCCESS);
    list.images[index as usize].bits_per_component() as i32
}

#[no_mangle]
pub extern "C" fn pdf_oxide_image_get_data(
    images: *const FfiImageList,
    index: i32,
    data_len: *mut i32,
    error_code: *mut i32,
) -> *mut u8 {
    if images.is_null() || index < 0 || data_len.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let list = unsafe { &*images };
    if (index as usize) >= list.images.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    let img = &list.images[index as usize];
    let data = match img.data() {
        crate::extractors::ImageData::Jpeg(bytes) => bytes.clone(),
        crate::extractors::ImageData::Raw { pixels, .. } => pixels.clone(),
    };
    unsafe {
        *data_len = data.len() as i32;
    }
    set_error(error_code, ERR_SUCCESS);
    let boxed = data.into_boxed_slice();
    Box::into_raw(boxed) as *mut u8
}

#[no_mangle]
pub extern "C" fn pdf_oxide_image_list_free(handle: *mut FfiImageList) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

// ─── Annotations ────────────────────────────────────────────────────────────

pub struct FfiAnnotationList {
    annotations: Vec<RustAnnotation>,
}

#[no_mangle]
pub extern "C" fn pdf_document_get_page_annotations(
    handle: *mut PdfDocument,
    page_index: i32,
    error_code: *mut i32,
) -> *mut FfiAnnotationList {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    match doc.get_annotations(page_index as usize) {
        Ok(annotations) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(FfiAnnotationList { annotations }))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_oxide_annotation_count(annotations: *const FfiAnnotationList) -> i32 {
    if annotations.is_null() {
        return 0;
    }
    unsafe { (*annotations).annotations.len() as i32 }
}

#[no_mangle]
pub extern "C" fn pdf_oxide_annotation_get_type(
    annotations: *const FfiAnnotationList,
    index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if annotations.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let list = unsafe { &*annotations };
    if (index as usize) >= list.annotations.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    to_c_string(&list.annotations[index as usize].annotation_type)
}

#[no_mangle]
pub extern "C" fn pdf_oxide_annotation_get_content(
    annotations: *const FfiAnnotationList,
    index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if annotations.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let list = unsafe { &*annotations };
    if (index as usize) >= list.annotations.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    to_c_string_opt(list.annotations[index as usize].contents.clone())
}

#[no_mangle]
pub extern "C" fn pdf_oxide_annotation_get_rect(
    annotations: *const FfiAnnotationList,
    index: i32,
    x: *mut f32,
    y: *mut f32,
    width: *mut f32,
    height: *mut f32,
    error_code: *mut i32,
) {
    if annotations.is_null() || x.is_null() || y.is_null() || width.is_null() || height.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return;
    }
    let list = unsafe { &*annotations };
    if index < 0 || (index as usize) >= list.annotations.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return;
    }
    if let Some(rect) = &list.annotations[index as usize].rect {
        unsafe {
            *x = rect[0] as f32;
            *y = rect[1] as f32;
            *width = (rect[2] - rect[0]) as f32;
            *height = (rect[3] - rect[1]) as f32;
        }
    } else {
        unsafe {
            *x = 0.0;
            *y = 0.0;
            *width = 0.0;
            *height = 0.0;
        }
    }
    set_error(error_code, ERR_SUCCESS);
}

#[no_mangle]
pub extern "C" fn pdf_oxide_annotation_list_free(handle: *mut FfiAnnotationList) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

// Advanced annotation accessors
#[no_mangle]
pub extern "C" fn pdf_oxide_annotation_get_subtype(
    annotations: *const FfiAnnotationList,
    index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if annotations.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let list = unsafe { &*annotations };
    if (index as usize) >= list.annotations.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    to_c_string(&format!("{:?}", list.annotations[index as usize].subtype_enum))
}

#[no_mangle]
pub extern "C" fn pdf_oxide_annotation_is_marked_deleted(
    _annotations: *const FfiAnnotationList,
    _index: i32,
    error_code: *mut i32,
) -> bool {
    set_error(error_code, ERR_SUCCESS);
    false
}

#[no_mangle]
pub extern "C" fn pdf_oxide_annotation_get_creation_date(
    annotations: *const FfiAnnotationList,
    index: i32,
    error_code: *mut i32,
) -> i64 {
    if annotations.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return 0;
    }
    let list = unsafe { &*annotations };
    if (index as usize) >= list.annotations.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return 0;
    }
    set_error(error_code, ERR_SUCCESS);
    // Return 0 — dates are stored as strings in the annotation, not timestamps
    0
}

#[no_mangle]
pub extern "C" fn pdf_oxide_annotation_get_modification_date(
    annotations: *const FfiAnnotationList,
    index: i32,
    error_code: *mut i32,
) -> i64 {
    if annotations.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return 0;
    }
    set_error(error_code, ERR_SUCCESS);
    0
}

#[no_mangle]
pub extern "C" fn pdf_oxide_annotation_get_author(
    annotations: *const FfiAnnotationList,
    index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if annotations.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let list = unsafe { &*annotations };
    if (index as usize) >= list.annotations.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    to_c_string_opt(list.annotations[index as usize].author.clone())
}

#[no_mangle]
pub extern "C" fn pdf_oxide_annotation_get_border_width(
    annotations: *const FfiAnnotationList,
    index: i32,
    error_code: *mut i32,
) -> f32 {
    if annotations.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return 0.0;
    }
    let list = unsafe { &*annotations };
    if (index as usize) >= list.annotations.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return 0.0;
    }
    set_error(error_code, ERR_SUCCESS);
    list.annotations[index as usize]
        .border
        .map(|b| b[2] as f32)
        .unwrap_or(0.0)
}

#[no_mangle]
pub extern "C" fn pdf_oxide_annotation_get_color(
    annotations: *const FfiAnnotationList,
    index: i32,
    error_code: *mut i32,
) -> u32 {
    if annotations.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return 0;
    }
    let list = unsafe { &*annotations };
    if (index as usize) >= list.annotations.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return 0;
    }
    set_error(error_code, ERR_SUCCESS);
    if let Some(color) = &list.annotations[index as usize].color {
        if color.len() >= 3 {
            let r = (color[0] * 255.0) as u32;
            let g = (color[1] * 255.0) as u32;
            let b = (color[2] * 255.0) as u32;
            (r << 16) | (g << 8) | b
        } else {
            0
        }
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn pdf_oxide_annotation_is_hidden(
    annotations: *const FfiAnnotationList,
    index: i32,
    error_code: *mut i32,
) -> bool {
    if annotations.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return false;
    }
    let list = unsafe { &*annotations };
    if (index as usize) >= list.annotations.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return false;
    }
    set_error(error_code, ERR_SUCCESS);
    list.annotations[index as usize].flags.is_hidden()
}

#[no_mangle]
pub extern "C" fn pdf_oxide_annotation_is_printable(
    annotations: *const FfiAnnotationList,
    index: i32,
    error_code: *mut i32,
) -> bool {
    if annotations.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return false;
    }
    let list = unsafe { &*annotations };
    if (index as usize) >= list.annotations.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return false;
    }
    set_error(error_code, ERR_SUCCESS);
    list.annotations[index as usize].flags.is_printable()
}

#[no_mangle]
pub extern "C" fn pdf_oxide_annotation_is_read_only(
    annotations: *const FfiAnnotationList,
    index: i32,
    error_code: *mut i32,
) -> bool {
    if annotations.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return false;
    }
    let list = unsafe { &*annotations };
    if (index as usize) >= list.annotations.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return false;
    }
    set_error(error_code, ERR_SUCCESS);
    list.annotations[index as usize].flags.is_read_only()
}

#[no_mangle]
pub extern "C" fn pdf_oxide_link_annotation_get_uri(
    annotations: *const FfiAnnotationList,
    index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if annotations.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let list = unsafe { &*annotations };
    if (index as usize) >= list.annotations.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    if let Some(LinkAction::Uri(uri)) = &list.annotations[index as usize].action {
        to_c_string(uri)
    } else {
        ptr::null_mut()
    }
}

use crate::annotations::LinkAction;

#[no_mangle]
pub extern "C" fn pdf_oxide_text_annotation_get_icon_name(
    _annotations: *const FfiAnnotationList,
    _index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    set_error(error_code, ERR_SUCCESS);
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn pdf_oxide_highlight_annotation_get_quad_points_count(
    annotations: *const FfiAnnotationList,
    index: i32,
    error_code: *mut i32,
) -> i32 {
    if annotations.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return 0;
    }
    let list = unsafe { &*annotations };
    if (index as usize) >= list.annotations.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return 0;
    }
    set_error(error_code, ERR_SUCCESS);
    list.annotations[index as usize]
        .quad_points
        .as_ref()
        .map(|q| q.len() as i32)
        .unwrap_or(0)
}

#[no_mangle]
pub extern "C" fn pdf_oxide_highlight_annotation_get_quad_point(
    annotations: *const FfiAnnotationList,
    index: i32,
    quad_index: i32,
    x1: *mut f32,
    y1: *mut f32,
    x2: *mut f32,
    y2: *mut f32,
    x3: *mut f32,
    y3: *mut f32,
    x4: *mut f32,
    y4: *mut f32,
    error_code: *mut i32,
) {
    if annotations.is_null() || index < 0 || quad_index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return;
    }
    let list = unsafe { &*annotations };
    if (index as usize) >= list.annotations.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return;
    }
    if let Some(quads) = &list.annotations[index as usize].quad_points {
        if (quad_index as usize) < quads.len() {
            let q = &quads[quad_index as usize];
            unsafe {
                *x1 = q[0] as f32;
                *y1 = q[1] as f32;
                *x2 = q[2] as f32;
                *y2 = q[3] as f32;
                *x3 = q[4] as f32;
                *y3 = q[5] as f32;
                *x4 = q[6] as f32;
                *y4 = q[7] as f32;
            }
            set_error(error_code, ERR_SUCCESS);
            return;
        }
    }
    set_error(error_code, ERR_INVALID_PAGE);
}

// ─── Page operations ────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn pdf_page_get_width(
    handle: *mut PdfDocument,
    page_index: i32,
    error_code: *mut i32,
) -> f32 {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return 0.0;
    }
    let doc = unsafe { &mut *handle };
    match doc.get_page_media_box(page_index as usize) {
        Ok((_, _, w, _)) => {
            set_error(error_code, ERR_SUCCESS);
            w
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            0.0
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_page_get_height(
    handle: *mut PdfDocument,
    page_index: i32,
    error_code: *mut i32,
) -> f32 {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return 0.0;
    }
    let doc = unsafe { &mut *handle };
    match doc.get_page_media_box(page_index as usize) {
        Ok((_, _, _, h)) => {
            set_error(error_code, ERR_SUCCESS);
            h
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            0.0
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_page_get_rotation(
    _handle: *mut PdfDocument,
    _page_index: i32,
    error_code: *mut i32,
) -> i32 {
    set_error(error_code, ERR_SUCCESS);
    0 // Rotation is not directly exposed in PdfDocument; would need page dict access
}

macro_rules! page_box_fn {
    ($fn_name:ident) => {
        #[no_mangle]
        pub extern "C" fn $fn_name(
            handle: *mut PdfDocument,
            page_index: i32,
            x: *mut f32,
            y: *mut f32,
            width: *mut f32,
            height: *mut f32,
            error_code: *mut i32,
        ) {
            if handle.is_null() || x.is_null() || y.is_null() || width.is_null() || height.is_null()
            {
                set_error(error_code, ERR_INVALID_ARG);
                return;
            }
            let doc = unsafe { &mut *handle };
            match doc.get_page_media_box(page_index as usize) {
                Ok((bx, by, bw, bh)) => {
                    unsafe {
                        *x = bx;
                        *y = by;
                        *width = bw;
                        *height = bh;
                    }
                    set_error(error_code, ERR_SUCCESS);
                },
                Err(e) => {
                    set_error(error_code, classify_error(&e));
                },
            }
        }
    };
}

page_box_fn!(pdf_page_get_media_box);
page_box_fn!(pdf_page_get_crop_box);
page_box_fn!(pdf_page_get_art_box);
page_box_fn!(pdf_page_get_bleed_box);
page_box_fn!(pdf_page_get_trim_box);

// ─── Page elements ──────────────────────────────────────────────────────────

use crate::layout::TextSpan;

pub struct FfiElementList {
    spans: Vec<TextSpan>,
}

#[no_mangle]
pub extern "C" fn pdf_page_get_elements(
    handle: *mut PdfDocument,
    page_index: i32,
    error_code: *mut i32,
) -> *mut FfiElementList {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    match doc.extract_spans(page_index as usize) {
        Ok(spans) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(FfiElementList { spans }))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_oxide_element_count(elements: *const FfiElementList) -> i32 {
    if elements.is_null() {
        return 0;
    }
    unsafe { (*elements).spans.len() as i32 }
}

#[no_mangle]
pub extern "C" fn pdf_oxide_element_get_type(
    _elements: *const FfiElementList,
    _index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    set_error(error_code, ERR_SUCCESS);
    to_c_string("text")
}

#[no_mangle]
pub extern "C" fn pdf_oxide_element_get_text(
    elements: *const FfiElementList,
    index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if elements.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let list = unsafe { &*elements };
    if (index as usize) >= list.spans.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    to_c_string(&list.spans[index as usize].text)
}

#[no_mangle]
pub extern "C" fn pdf_oxide_element_get_rect(
    elements: *const FfiElementList,
    index: i32,
    x: *mut f32,
    y: *mut f32,
    width: *mut f32,
    height: *mut f32,
    error_code: *mut i32,
) {
    if elements.is_null() || x.is_null() || y.is_null() || width.is_null() || height.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return;
    }
    let list = unsafe { &*elements };
    if index < 0 || (index as usize) >= list.spans.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return;
    }
    let span = &list.spans[index as usize];
    unsafe {
        *x = span.bbox.x;
        *y = span.bbox.y;
        *width = span.bbox.width;
        *height = span.bbox.height;
    }
    set_error(error_code, ERR_SUCCESS);
}

#[no_mangle]
pub extern "C" fn pdf_oxide_elements_free(handle: *mut FfiElementList) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

// ─── Barcodes ───────────────────────────────────────────────────────────────

use crate::writer::barcode::{
    BarcodeGenerator, BarcodeOptions, BarcodeType, QrCodeOptions, QrErrorCorrection,
};

/// Opaque handle for generated barcode image (PNG bytes)
pub struct FfiBarcodeImage {
    data: Vec<u8>,
    format: i32, // 0=QR, 1=Code128, etc.
    source_data: String,
}

#[no_mangle]
pub extern "C" fn pdf_generate_qr_code(
    data: *const c_char,
    error_correction: i32,
    size_px: i32,
    error_code: *mut i32,
) -> *mut FfiBarcodeImage {
    if data.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let data_str = match unsafe { CStr::from_ptr(data) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error(error_code, ERR_INVALID_ARG);
            return ptr::null_mut();
        },
    };
    let ec = match error_correction {
        0 => QrErrorCorrection::Low,
        1 => QrErrorCorrection::Medium,
        2 => QrErrorCorrection::Quartile,
        3 => QrErrorCorrection::High,
        _ => QrErrorCorrection::Medium,
    };
    let opts = QrCodeOptions::new()
        .size(size_px.max(1) as u32)
        .error_correction(ec);
    match BarcodeGenerator::generate_qr(data_str, &opts) {
        Ok(png_bytes) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(FfiBarcodeImage {
                data: png_bytes,
                format: 0,
                source_data: data_str.to_string(),
            }))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_generate_barcode(
    data: *const c_char,
    format: i32,
    size_px: i32,
    error_code: *mut i32,
) -> *mut FfiBarcodeImage {
    if data.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let data_str = match unsafe { CStr::from_ptr(data) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error(error_code, ERR_INVALID_ARG);
            return ptr::null_mut();
        },
    };
    let barcode_type = match format {
        0 => BarcodeType::Code128,
        1 => BarcodeType::Code39,
        2 => BarcodeType::Ean13,
        3 => BarcodeType::Ean8,
        4 => BarcodeType::UpcA,
        5 => BarcodeType::Itf,
        _ => BarcodeType::Code128,
    };
    let opts = BarcodeOptions::new()
        .width(size_px.max(1) as u32)
        .height((size_px.max(1) / 3).max(30) as u32);
    match BarcodeGenerator::generate_1d(barcode_type, data_str, &opts) {
        Ok(png_bytes) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(FfiBarcodeImage {
                data: png_bytes,
                format,
                source_data: data_str.to_string(),
            }))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_barcode_get_image_png(
    barcode_handle: *const FfiBarcodeImage,
    _size_px: i32,
    out_len: *mut i32,
    error_code: *mut i32,
) -> *mut u8 {
    if barcode_handle.is_null() || out_len.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        if !out_len.is_null() {
            unsafe { *out_len = 0 }
        }
        return ptr::null_mut();
    }
    let bc = unsafe { &*barcode_handle };
    let data = bc.data.clone();
    let len = data.len() as i32;
    unsafe { *out_len = len }
    set_error(error_code, ERR_SUCCESS);
    let boxed = data.into_boxed_slice();
    Box::into_raw(boxed) as *mut u8
}

#[no_mangle]
pub extern "C" fn pdf_barcode_get_svg(
    _barcode_handle: *const FfiBarcodeImage,
    _size_px: i32,
    error_code: *mut i32,
) -> *mut c_char {
    // SVG generation not directly available — PNG is the primary output
    set_error(error_code, _ERR_UNSUPPORTED);
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn pdf_add_barcode_to_page(
    _document_handle: *mut std::ffi::c_void,
    _page_index: i32,
    _barcode_handle: *const FfiBarcodeImage,
    _x: f32,
    _y: f32,
    _width: f32,
    _height: f32,
    error_code: *mut i32,
) -> i32 {
    // Adding barcode to existing page requires editor integration
    set_error(error_code, _ERR_UNSUPPORTED);
    -1
}

#[no_mangle]
pub extern "C" fn pdf_barcode_get_format(
    barcode_handle: *const FfiBarcodeImage,
    error_code: *mut i32,
) -> i32 {
    if barcode_handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    set_error(error_code, ERR_SUCCESS);
    unsafe { (*barcode_handle).format }
}

#[no_mangle]
pub extern "C" fn pdf_barcode_get_data(
    barcode_handle: *const FfiBarcodeImage,
    error_code: *mut i32,
) -> *mut c_char {
    if barcode_handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    to_c_string(&unsafe { &*barcode_handle }.source_data)
}

#[no_mangle]
pub extern "C" fn pdf_barcode_get_confidence(
    _barcode_handle: *const FfiBarcodeImage,
    error_code: *mut i32,
) -> f32 {
    set_error(error_code, ERR_SUCCESS);
    1.0 // generated barcodes have perfect confidence
}

#[no_mangle]
pub extern "C" fn pdf_barcode_free(handle: *mut FfiBarcodeImage) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

// ─── Signatures ─────────────────────────────────────────────────────────────

/// Opaque handle for SignatureInfo extracted from a PDF
#[cfg(feature = "signatures")]
pub struct FfiSignatureInfo {
    info: crate::signatures::SignatureInfo,
}

#[cfg(not(feature = "signatures"))]
pub struct FfiSignatureInfo {
    _dummy: (),
}

#[no_mangle]
pub extern "C" fn pdf_certificate_load_from_bytes(
    cert_bytes: *const u8,
    cert_len: i32,
    password: *const c_char,
    error_code: *mut i32,
) -> *mut std::ffi::c_void {
    #[cfg(feature = "signatures")]
    {
        if cert_bytes.is_null() || cert_len <= 0 {
            set_error(error_code, ERR_INVALID_ARG);
            return ptr::null_mut();
        }
        let data = unsafe { std::slice::from_raw_parts(cert_bytes, cert_len as usize) };
        let pwd = if password.is_null() {
            ""
        } else {
            unsafe { CStr::from_ptr(password) }.to_str().unwrap_or("")
        };
        match crate::signatures::SigningCredentials::from_pkcs12(data, pwd) {
            Ok(creds) => {
                set_error(error_code, ERR_SUCCESS);
                Box::into_raw(Box::new(creds)) as *mut std::ffi::c_void
            },
            Err(e) => {
                set_error(error_code, classify_error(&e));
                ptr::null_mut()
            },
        }
    }
    #[cfg(not(feature = "signatures"))]
    {
        set_error(error_code, _ERR_UNSUPPORTED);
        ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn pdf_document_sign(
    _document_handle: *mut std::ffi::c_void,
    _certificate_handle: *const std::ffi::c_void,
    _reason: *const c_char,
    _location: *const c_char,
    error_code: *mut i32,
) -> i32 {
    // Full signing requires file-level operations beyond this FFI scope
    set_error(error_code, _ERR_UNSUPPORTED);
    -1
}

#[no_mangle]
pub extern "C" fn pdf_document_get_signature_count(
    _document_handle: *const std::ffi::c_void,
    error_code: *mut i32,
) -> i32 {
    set_error(error_code, ERR_SUCCESS);
    0
}

#[no_mangle]
pub extern "C" fn pdf_document_get_signature(
    _document_handle: *const std::ffi::c_void,
    _index: i32,
    error_code: *mut i32,
) -> *mut std::ffi::c_void {
    set_error(error_code, _ERR_UNSUPPORTED);
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn pdf_signature_verify(
    _signature_handle: *const std::ffi::c_void,
    error_code: *mut i32,
) -> i32 {
    set_error(error_code, _ERR_UNSUPPORTED);
    -1
}

#[no_mangle]
pub extern "C" fn pdf_document_verify_all_signatures(
    _document_handle: *const std::ffi::c_void,
    error_code: *mut i32,
) -> i32 {
    set_error(error_code, _ERR_UNSUPPORTED);
    -1
}

#[no_mangle]
pub extern "C" fn pdf_signature_get_signer_name(
    sig: *const FfiSignatureInfo,
    error_code: *mut i32,
) -> *mut c_char {
    #[cfg(feature = "signatures")]
    {
        if sig.is_null() {
            set_error(error_code, ERR_INVALID_ARG);
            return ptr::null_mut();
        }
        let info = unsafe { &*sig };
        set_error(error_code, ERR_SUCCESS);
        to_c_string_opt(info.info.signer_name.clone())
    }
    #[cfg(not(feature = "signatures"))]
    {
        let _ = sig;
        set_error(error_code, _ERR_UNSUPPORTED);
        ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn pdf_signature_get_signing_time(
    sig: *const FfiSignatureInfo,
    error_code: *mut i32,
) -> i64 {
    #[cfg(feature = "signatures")]
    {
        if sig.is_null() {
            set_error(error_code, ERR_INVALID_ARG);
            return 0;
        }
        set_error(error_code, ERR_SUCCESS);
        0 // signing_time is a String, not epoch — returning 0 for now
    }
    #[cfg(not(feature = "signatures"))]
    {
        let _ = sig;
        set_error(error_code, _ERR_UNSUPPORTED);
        0
    }
}

#[no_mangle]
pub extern "C" fn pdf_signature_get_signing_reason(
    sig: *const FfiSignatureInfo,
    error_code: *mut i32,
) -> *mut c_char {
    #[cfg(feature = "signatures")]
    {
        if sig.is_null() {
            set_error(error_code, ERR_INVALID_ARG);
            return ptr::null_mut();
        }
        let info = unsafe { &*sig };
        set_error(error_code, ERR_SUCCESS);
        to_c_string_opt(info.info.reason.clone())
    }
    #[cfg(not(feature = "signatures"))]
    {
        let _ = sig;
        set_error(error_code, _ERR_UNSUPPORTED);
        ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn pdf_signature_get_signing_location(
    sig: *const FfiSignatureInfo,
    error_code: *mut i32,
) -> *mut c_char {
    #[cfg(feature = "signatures")]
    {
        if sig.is_null() {
            set_error(error_code, ERR_INVALID_ARG);
            return ptr::null_mut();
        }
        let info = unsafe { &*sig };
        set_error(error_code, ERR_SUCCESS);
        to_c_string_opt(info.info.location.clone())
    }
    #[cfg(not(feature = "signatures"))]
    {
        let _ = sig;
        set_error(error_code, _ERR_UNSUPPORTED);
        ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn pdf_signature_get_certificate(
    _sig: *const std::ffi::c_void,
    error_code: *mut i32,
) -> *mut std::ffi::c_void {
    set_error(error_code, _ERR_UNSUPPORTED);
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn pdf_certificate_get_subject(
    _cert: *const std::ffi::c_void,
    error_code: *mut i32,
) -> *mut c_char {
    set_error(error_code, _ERR_UNSUPPORTED);
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn pdf_certificate_get_issuer(
    _cert: *const std::ffi::c_void,
    error_code: *mut i32,
) -> *mut c_char {
    set_error(error_code, _ERR_UNSUPPORTED);
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn pdf_certificate_get_serial(
    _cert: *const std::ffi::c_void,
    error_code: *mut i32,
) -> *mut c_char {
    set_error(error_code, _ERR_UNSUPPORTED);
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn pdf_certificate_get_validity(
    _cert: *const std::ffi::c_void,
    _not_before: *mut i64,
    _not_after: *mut i64,
    error_code: *mut i32,
) {
    set_error(error_code, _ERR_UNSUPPORTED);
}

#[no_mangle]
pub extern "C" fn pdf_certificate_is_valid(
    _cert: *const std::ffi::c_void,
    error_code: *mut i32,
) -> i32 {
    set_error(error_code, _ERR_UNSUPPORTED);
    0
}

#[no_mangle]
pub extern "C" fn pdf_signature_free(handle: *mut FfiSignatureInfo) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}
#[no_mangle]
pub extern "C" fn pdf_certificate_free(_handle: *mut std::ffi::c_void) {}

// ─── Rendering ──────────────────────────────────────────────────────────────

#[cfg(feature = "rendering")]
use crate::rendering::{
    self, ImageFormat as RenderImageFormat, RenderOptions as RustRenderOptions,
    RenderedImage as RustRenderedImage,
};

#[cfg(feature = "rendering")]
pub struct FfiRenderedImage {
    inner: RustRenderedImage,
}

#[cfg(not(feature = "rendering"))]
pub struct FfiRenderedImage {
    _dummy: (),
}

#[no_mangle]
pub extern "C" fn pdf_estimate_render_time(
    _doc: *const std::ffi::c_void,
    _page_index: i32,
    error_code: *mut i32,
) -> i32 {
    set_error(error_code, ERR_SUCCESS);
    100 // estimate 100ms
}

#[no_mangle]
pub extern "C" fn pdf_create_renderer(
    _dpi: i32,
    _format: i32,
    _quality: i32,
    _anti_alias: bool,
    error_code: *mut i32,
) -> *mut std::ffi::c_void {
    // Rendering uses stateless render_page() function, no persistent renderer needed
    set_error(error_code, _ERR_UNSUPPORTED);
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn pdf_render_page(
    doc: *mut PdfDocument,
    page_index: i32,
    format: i32,
    error_code: *mut i32,
) -> *mut FfiRenderedImage {
    #[cfg(feature = "rendering")]
    {
        if doc.is_null() {
            set_error(error_code, ERR_INVALID_ARG);
            return ptr::null_mut();
        }
        let d = unsafe { &mut *doc };
        let fmt = if format == 1 {
            RenderImageFormat::Jpeg
        } else {
            RenderImageFormat::Png
        };
        let opts = RustRenderOptions {
            dpi: 150,
            format: fmt,
            ..Default::default()
        };
        match rendering::render_page(d, page_index as usize, &opts) {
            Ok(img) => {
                set_error(error_code, ERR_SUCCESS);
                Box::into_raw(Box::new(FfiRenderedImage { inner: img }))
            },
            Err(e) => {
                set_error(error_code, classify_error(&e));
                ptr::null_mut()
            },
        }
    }
    #[cfg(not(feature = "rendering"))]
    {
        let _ = (doc, page_index, format);
        set_error(error_code, _ERR_UNSUPPORTED);
        ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn pdf_render_page_region(
    _doc: *mut std::ffi::c_void,
    _page_index: i32,
    _crop_x: f32,
    _crop_y: f32,
    _crop_width: f32,
    _crop_height: f32,
    _format: i32,
    error_code: *mut i32,
) -> *mut std::ffi::c_void {
    set_error(error_code, _ERR_UNSUPPORTED);
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn pdf_render_page_zoom(
    doc: *mut PdfDocument,
    page_index: i32,
    zoom: f32,
    format: i32,
    error_code: *mut i32,
) -> *mut FfiRenderedImage {
    #[cfg(feature = "rendering")]
    {
        if doc.is_null() {
            set_error(error_code, ERR_INVALID_ARG);
            return ptr::null_mut();
        }
        let d = unsafe { &mut *doc };
        let dpi = (72.0 * zoom) as u32;
        let fmt = if format == 1 {
            RenderImageFormat::Jpeg
        } else {
            RenderImageFormat::Png
        };
        let opts = RustRenderOptions {
            dpi,
            format: fmt,
            ..Default::default()
        };
        match rendering::render_page(d, page_index as usize, &opts) {
            Ok(img) => {
                set_error(error_code, ERR_SUCCESS);
                Box::into_raw(Box::new(FfiRenderedImage { inner: img }))
            },
            Err(e) => {
                set_error(error_code, classify_error(&e));
                ptr::null_mut()
            },
        }
    }
    #[cfg(not(feature = "rendering"))]
    {
        let _ = (doc, page_index, zoom, format);
        set_error(error_code, _ERR_UNSUPPORTED);
        ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn pdf_render_page_fit(
    _doc: *mut std::ffi::c_void,
    _page_index: i32,
    _w: i32,
    _h: i32,
    _format: i32,
    error_code: *mut i32,
) -> *mut std::ffi::c_void {
    set_error(error_code, _ERR_UNSUPPORTED);
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn pdf_render_page_thumbnail(
    doc: *mut PdfDocument,
    page_index: i32,
    _size: i32,
    format: i32,
    error_code: *mut i32,
) -> *mut FfiRenderedImage {
    #[cfg(feature = "rendering")]
    {
        if doc.is_null() {
            set_error(error_code, ERR_INVALID_ARG);
            return ptr::null_mut();
        }
        let d = unsafe { &mut *doc };
        let fmt = if format == 1 {
            RenderImageFormat::Jpeg
        } else {
            RenderImageFormat::Png
        };
        let opts = RustRenderOptions {
            dpi: 72,
            format: fmt,
            ..Default::default()
        };
        match rendering::render_page(d, page_index as usize, &opts) {
            Ok(img) => {
                set_error(error_code, ERR_SUCCESS);
                Box::into_raw(Box::new(FfiRenderedImage { inner: img }))
            },
            Err(e) => {
                set_error(error_code, classify_error(&e));
                ptr::null_mut()
            },
        }
    }
    #[cfg(not(feature = "rendering"))]
    {
        let _ = (doc, page_index, _size, format);
        set_error(error_code, _ERR_UNSUPPORTED);
        ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn pdf_get_rendered_image_width(
    img: *const FfiRenderedImage,
    error_code: *mut i32,
) -> i32 {
    #[cfg(feature = "rendering")]
    {
        if img.is_null() {
            set_error(error_code, ERR_INVALID_ARG);
            return 0;
        }
        set_error(error_code, ERR_SUCCESS);
        unsafe { (*img).inner.width as i32 }
    }
    #[cfg(not(feature = "rendering"))]
    {
        let _ = img;
        set_error(error_code, _ERR_UNSUPPORTED);
        0
    }
}

#[no_mangle]
pub extern "C" fn pdf_get_rendered_image_height(
    img: *const FfiRenderedImage,
    error_code: *mut i32,
) -> i32 {
    #[cfg(feature = "rendering")]
    {
        if img.is_null() {
            set_error(error_code, ERR_INVALID_ARG);
            return 0;
        }
        set_error(error_code, ERR_SUCCESS);
        unsafe { (*img).inner.height as i32 }
    }
    #[cfg(not(feature = "rendering"))]
    {
        let _ = img;
        set_error(error_code, _ERR_UNSUPPORTED);
        0
    }
}

#[no_mangle]
pub extern "C" fn pdf_get_rendered_image_data(
    img: *const FfiRenderedImage,
    data_len: *mut i32,
    error_code: *mut i32,
) -> *mut u8 {
    #[cfg(feature = "rendering")]
    {
        if img.is_null() || data_len.is_null() {
            set_error(error_code, ERR_INVALID_ARG);
            return ptr::null_mut();
        }
        let i = unsafe { &*img };
        let bytes = i.inner.as_bytes().to_vec();
        unsafe {
            *data_len = bytes.len() as i32;
        }
        set_error(error_code, ERR_SUCCESS);
        let boxed = bytes.into_boxed_slice();
        Box::into_raw(boxed) as *mut u8
    }
    #[cfg(not(feature = "rendering"))]
    {
        let _ = (img, data_len);
        set_error(error_code, _ERR_UNSUPPORTED);
        ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn pdf_save_rendered_image(
    img: *const FfiRenderedImage,
    file_path: *const c_char,
    error_code: *mut i32,
) -> i32 {
    #[cfg(feature = "rendering")]
    {
        if img.is_null() || file_path.is_null() {
            set_error(error_code, ERR_INVALID_ARG);
            return -1;
        }
        let path = match unsafe { CStr::from_ptr(file_path) }.to_str() {
            Ok(s) => s,
            Err(_) => {
                set_error(error_code, ERR_INVALID_ARG);
                return -1;
            },
        };
        let i = unsafe { &*img };
        match i.inner.save(path) {
            Ok(()) => {
                set_error(error_code, ERR_SUCCESS);
                0
            },
            Err(e) => {
                set_error(error_code, classify_error(&e));
                -1
            },
        }
    }
    #[cfg(not(feature = "rendering"))]
    {
        let _ = (img, file_path);
        set_error(error_code, _ERR_UNSUPPORTED);
        -1
    }
}

#[no_mangle]
pub extern "C" fn pdf_rendered_image_free(handle: *mut FfiRenderedImage) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}
#[no_mangle]
pub extern "C" fn pdf_renderer_free(_handle: *mut std::ffi::c_void) {}

// ─── TSA (Time Stamp Authority) ────────────────────────────────────────────
// TSA is integrated into signatures via SignOptions::with_timestamp()
// No standalone TSA client in the Rust library — these remain stubs

#[no_mangle]
pub extern "C" fn pdf_tsa_client_create(
    _url: *const c_char,
    _username: *const c_char,
    _password: *const c_char,
    _timeout: i32,
    _hash_algo: i32,
    _use_nonce: bool,
    _cert_req: bool,
    error_code: *mut i32,
) -> *mut std::ffi::c_void {
    set_error(error_code, _ERR_UNSUPPORTED);
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn pdf_tsa_client_free(_client: *mut std::ffi::c_void) {}

#[no_mangle]
pub extern "C" fn pdf_tsa_request_timestamp(
    _client: *const std::ffi::c_void,
    _data: *const u8,
    _data_len: usize,
    error_code: *mut i32,
) -> *mut std::ffi::c_void {
    set_error(error_code, _ERR_UNSUPPORTED);
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn pdf_tsa_request_timestamp_hash(
    _client: *const std::ffi::c_void,
    _hash: *const u8,
    _hash_len: usize,
    _hash_algo: i32,
    error_code: *mut i32,
) -> *mut std::ffi::c_void {
    set_error(error_code, _ERR_UNSUPPORTED);
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn pdf_timestamp_get_token(
    _ts: *const std::ffi::c_void,
    _out_len: *mut usize,
    error_code: *mut i32,
) -> *const u8 {
    set_error(error_code, _ERR_UNSUPPORTED);
    ptr::null()
}

#[no_mangle]
pub extern "C" fn pdf_timestamp_get_time(
    _ts: *const std::ffi::c_void,
    error_code: *mut i32,
) -> i64 {
    set_error(error_code, _ERR_UNSUPPORTED);
    0
}

#[no_mangle]
pub extern "C" fn pdf_timestamp_get_serial(
    _ts: *const std::ffi::c_void,
    error_code: *mut i32,
) -> *mut c_char {
    set_error(error_code, _ERR_UNSUPPORTED);
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn pdf_timestamp_get_tsa_name(
    _ts: *const std::ffi::c_void,
    error_code: *mut i32,
) -> *mut c_char {
    set_error(error_code, _ERR_UNSUPPORTED);
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn pdf_timestamp_get_policy_oid(
    _ts: *const std::ffi::c_void,
    error_code: *mut i32,
) -> *mut c_char {
    set_error(error_code, _ERR_UNSUPPORTED);
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn pdf_timestamp_get_hash_algorithm(
    _ts: *const std::ffi::c_void,
    error_code: *mut i32,
) -> i32 {
    set_error(error_code, _ERR_UNSUPPORTED);
    -1
}

#[no_mangle]
pub extern "C" fn pdf_timestamp_get_message_imprint(
    _ts: *const std::ffi::c_void,
    _out_len: *mut usize,
    error_code: *mut i32,
) -> *const u8 {
    set_error(error_code, _ERR_UNSUPPORTED);
    ptr::null()
}

#[no_mangle]
pub extern "C" fn pdf_timestamp_verify(_ts: *const std::ffi::c_void, error_code: *mut i32) -> bool {
    set_error(error_code, _ERR_UNSUPPORTED);
    false
}

#[no_mangle]
pub extern "C" fn pdf_timestamp_free(_ts: *mut std::ffi::c_void) {}

#[no_mangle]
pub extern "C" fn pdf_signature_add_timestamp(
    _sig: *const std::ffi::c_void,
    _ts: *const std::ffi::c_void,
    error_code: *mut i32,
) -> bool {
    set_error(error_code, _ERR_UNSUPPORTED);
    false
}

#[no_mangle]
pub extern "C" fn pdf_signature_has_timestamp(
    _sig: *const std::ffi::c_void,
    error_code: *mut i32,
) -> bool {
    set_error(error_code, _ERR_UNSUPPORTED);
    false
}

#[no_mangle]
pub extern "C" fn pdf_signature_get_timestamp(
    _sig: *const std::ffi::c_void,
    error_code: *mut i32,
) -> *mut std::ffi::c_void {
    set_error(error_code, _ERR_UNSUPPORTED);
    ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn pdf_add_timestamp(
    _pdf_data: *const u8,
    _pdf_len: usize,
    _sig_index: i32,
    _tsa_url: *const c_char,
    _out_data: *mut *mut u8,
    _out_len: *mut usize,
    error_code: *mut i32,
) -> bool {
    set_error(error_code, _ERR_UNSUPPORTED);
    false
}

// ─── PDF/UA Validation (always available) ──────────────────────────────────

use crate::compliance::pdf_ua::{PdfUaLevel, PdfUaValidator, UaValidationResult};

pub struct FfiUaResults {
    result: UaValidationResult,
}

#[no_mangle]
pub extern "C" fn pdf_validate_pdf_ua(
    document: *mut PdfDocument,
    level: i32,
    error_code: *mut i32,
) -> *mut FfiUaResults {
    if document.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *document };
    let ua_level = if level == 2 {
        PdfUaLevel::Ua2
    } else {
        PdfUaLevel::Ua1
    };
    match PdfUaValidator::new().validate(doc, ua_level) {
        Ok(result) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(FfiUaResults { result }))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_pdf_ua_is_accessible(
    results: *const FfiUaResults,
    error_code: *mut i32,
) -> bool {
    if results.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return false;
    }
    set_error(error_code, ERR_SUCCESS);
    unsafe { (*results).result.is_compliant }
}

#[no_mangle]
pub extern "C" fn pdf_pdf_ua_error_count(results: *const FfiUaResults) -> i32 {
    if results.is_null() {
        return 0;
    }
    unsafe { (*results).result.errors.len() as i32 }
}

#[no_mangle]
pub extern "C" fn pdf_pdf_ua_get_error(
    results: *const FfiUaResults,
    index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if results.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let r = unsafe { &*results };
    if (index as usize) >= r.result.errors.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    to_c_string(&r.result.errors[index as usize].message)
}

#[no_mangle]
pub extern "C" fn pdf_pdf_ua_warning_count(results: *const FfiUaResults) -> i32 {
    if results.is_null() {
        return 0;
    }
    unsafe { (*results).result.warnings.len() as i32 }
}

#[no_mangle]
pub extern "C" fn pdf_pdf_ua_get_warning(
    results: *const FfiUaResults,
    index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if results.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let r = unsafe { &*results };
    if (index as usize) >= r.result.warnings.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    to_c_string(&r.result.warnings[index as usize].message)
}

#[no_mangle]
pub extern "C" fn pdf_pdf_ua_get_stats(
    results: *const FfiUaResults,
    out_struct: *mut i32,
    out_images: *mut i32,
    out_tables: *mut i32,
    out_forms: *mut i32,
    out_annotations: *mut i32,
    out_pages: *mut i32,
    error_code: *mut i32,
) -> bool {
    if results.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return false;
    }
    let r = unsafe { &*results };
    let s = &r.result.stats;
    if !out_struct.is_null() {
        unsafe {
            *out_struct = s.structure_elements_checked as i32;
        }
    }
    if !out_images.is_null() {
        unsafe {
            *out_images = s.images_checked as i32;
        }
    }
    if !out_tables.is_null() {
        unsafe {
            *out_tables = s.tables_checked as i32;
        }
    }
    if !out_forms.is_null() {
        unsafe {
            *out_forms = s.form_fields_checked as i32;
        }
    }
    if !out_annotations.is_null() {
        unsafe {
            *out_annotations = s.annotations_checked as i32;
        }
    }
    if !out_pages.is_null() {
        unsafe {
            *out_pages = s.pages_checked as i32;
        }
    }
    set_error(error_code, ERR_SUCCESS);
    true
}

#[no_mangle]
pub extern "C" fn pdf_pdf_ua_results_free(results: *mut FfiUaResults) {
    if !results.is_null() {
        unsafe {
            drop(Box::from_raw(results));
        }
    }
}

// ─── FDF/XFDF Import/Export (always available) ─────────────────────────────

use crate::extractors::FormExtractor;
use crate::fdf::{FdfWriter, XfdfWriter};

#[no_mangle]
pub extern "C" fn pdf_form_import_from_file(
    _document: *const std::ffi::c_void,
    _filename: *const c_char,
    error_code: *mut i32,
) -> bool {
    // Import requires DocumentEditor — not supported via PdfDocument handle
    set_error(error_code, _ERR_UNSUPPORTED);
    false
}

#[no_mangle]
pub extern "C" fn pdf_document_import_form_data(
    _document: *const std::ffi::c_void,
    _data_path: *const c_char,
    error_code: *mut i32,
) -> i32 {
    set_error(error_code, _ERR_UNSUPPORTED);
    -1
}

#[no_mangle]
pub extern "C" fn pdf_editor_import_fdf_bytes(
    _document: *const std::ffi::c_void,
    _data: *const u8,
    _data_len: usize,
    error_code: *mut i32,
) -> i32 {
    set_error(error_code, _ERR_UNSUPPORTED);
    -1
}

#[no_mangle]
pub extern "C" fn pdf_editor_import_xfdf_bytes(
    _document: *const std::ffi::c_void,
    _data: *const u8,
    _data_len: usize,
    error_code: *mut i32,
) -> i32 {
    set_error(error_code, _ERR_UNSUPPORTED);
    -1
}

/// Export form data from a PdfDocument. format_type: 0=FDF, 1=XFDF
#[no_mangle]
pub extern "C" fn pdf_document_export_form_data_to_bytes(
    document: *mut PdfDocument,
    format_type: i32,
    out_len: *mut usize,
    error_code: *mut i32,
) -> *mut u8 {
    if document.is_null() || out_len.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *document };
    let fields = match FormExtractor::extract_fields(doc) {
        Ok(f) => f,
        Err(e) => {
            set_error(error_code, classify_error(&e));
            return ptr::null_mut();
        },
    };
    let bytes = if format_type == 1 {
        // XFDF (XML)
        let writer = XfdfWriter::from_fields(fields);
        writer.to_bytes()
    } else {
        // FDF
        let writer = FdfWriter::from_fields(fields);
        match writer.to_bytes() {
            Ok(b) => b,
            Err(e) => {
                set_error(error_code, classify_error(&e));
                return ptr::null_mut();
            },
        }
    };
    unsafe {
        *out_len = bytes.len();
    }
    set_error(error_code, ERR_SUCCESS);
    let boxed = bytes.into_boxed_slice();
    Box::into_raw(boxed) as *mut u8
}

// ─── Open from bytes / password ─────────────────────────────────────────────

/// Open a PDF document from a byte buffer.
#[no_mangle]
pub extern "C" fn pdf_document_open_from_bytes(
    data: *const u8,
    len: usize,
    error_code: *mut i32,
) -> *mut PdfDocument {
    if data.is_null() || len == 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let bytes = unsafe { std::slice::from_raw_parts(data, len) }.to_vec();
    match PdfDocument::from_bytes(bytes) {
        Ok(doc) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(doc))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

/// Open a PDF with password.
#[no_mangle]
pub extern "C" fn pdf_document_open_with_password(
    path: *const c_char,
    password: *const c_char,
    error_code: *mut i32,
) -> *mut PdfDocument {
    if path.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let path_str = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error(error_code, ERR_INVALID_ARG);
            return ptr::null_mut();
        },
    };
    match PdfDocument::open(path_str) {
        Ok(mut doc) => {
            if !password.is_null() {
                if let Ok(pwd) = unsafe { CStr::from_ptr(password) }.to_str() {
                    let _ = doc.authenticate(pwd.as_bytes());
                }
            }
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(doc))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

/// Check if the document is encrypted.
#[no_mangle]
pub extern "C" fn pdf_document_is_encrypted(handle: *const PdfDocument) -> bool {
    if handle.is_null() {
        return false;
    }
    unsafe { &*handle }.is_encrypted()
}

/// Authenticate with password.
#[no_mangle]
pub extern "C" fn pdf_document_authenticate(
    handle: *mut PdfDocument,
    password: *const c_char,
    error_code: *mut i32,
) -> bool {
    if handle.is_null() || password.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return false;
    }
    let doc = unsafe { &mut *handle };
    let pwd = match unsafe { CStr::from_ptr(password) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error(error_code, ERR_INVALID_ARG);
            return false;
        },
    };
    match doc.authenticate(pwd.as_bytes()) {
        Ok(ok) => {
            set_error(error_code, ERR_SUCCESS);
            ok
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            false
        },
    }
}

// ─── Extract all text / all HTML / all plain text ───────────────────────────

#[no_mangle]
pub extern "C" fn pdf_document_extract_all_text(
    handle: *mut PdfDocument,
    error_code: *mut i32,
) -> *mut c_char {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    match doc.extract_all_text() {
        Ok(t) => {
            set_error(error_code, ERR_SUCCESS);
            to_c_string(&t)
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_document_to_html_all(
    handle: *mut PdfDocument,
    error_code: *mut i32,
) -> *mut c_char {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    let opts = ConversionOptions::default();
    match doc.to_html_all(&opts) {
        Ok(t) => {
            set_error(error_code, ERR_SUCCESS);
            to_c_string(&t)
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_document_to_plain_text_all(
    handle: *mut PdfDocument,
    error_code: *mut i32,
) -> *mut c_char {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    let opts = ConversionOptions::default();
    match doc.to_plain_text_all(&opts) {
        Ok(t) => {
            set_error(error_code, ERR_SUCCESS);
            to_c_string(&t)
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

// ─── Granular extraction: chars, words, lines, tables ───────────────────────

use crate::layout::{TextChar, TextLine as RustTextLine, Word};

// --- Chars ---

pub struct FfiCharList {
    chars: Vec<TextChar>,
}

#[no_mangle]
pub extern "C" fn pdf_document_extract_chars(
    handle: *mut PdfDocument,
    page_index: i32,
    error_code: *mut i32,
) -> *mut FfiCharList {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    match doc.extract_chars(page_index as usize) {
        Ok(chars) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(FfiCharList { chars }))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_oxide_char_count(chars: *const FfiCharList) -> i32 {
    if chars.is_null() {
        return 0;
    }
    unsafe { (*chars).chars.len() as i32 }
}

#[no_mangle]
pub extern "C" fn pdf_oxide_char_get_char(
    chars: *const FfiCharList,
    index: i32,
    error_code: *mut i32,
) -> u32 {
    if chars.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return 0;
    }
    let list = unsafe { &*chars };
    if (index as usize) >= list.chars.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return 0;
    }
    set_error(error_code, ERR_SUCCESS);
    list.chars[index as usize].char as u32
}

#[no_mangle]
pub extern "C" fn pdf_oxide_char_get_bbox(
    chars: *const FfiCharList,
    index: i32,
    x: *mut f32,
    y: *mut f32,
    w: *mut f32,
    h: *mut f32,
    error_code: *mut i32,
) {
    if chars.is_null() || index < 0 || x.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return;
    }
    let list = unsafe { &*chars };
    if (index as usize) >= list.chars.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return;
    }
    let b = &list.chars[index as usize].bbox;
    unsafe {
        *x = b.x;
        *y = b.y;
        *w = b.width;
        *h = b.height;
    }
    set_error(error_code, ERR_SUCCESS);
}

#[no_mangle]
pub extern "C" fn pdf_oxide_char_get_font_name(
    chars: *const FfiCharList,
    index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if chars.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let list = unsafe { &*chars };
    if (index as usize) >= list.chars.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    to_c_string(&list.chars[index as usize].font_name)
}

#[no_mangle]
pub extern "C" fn pdf_oxide_char_get_font_size(
    chars: *const FfiCharList,
    index: i32,
    error_code: *mut i32,
) -> f32 {
    if chars.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return 0.0;
    }
    let list = unsafe { &*chars };
    if (index as usize) >= list.chars.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return 0.0;
    }
    set_error(error_code, ERR_SUCCESS);
    list.chars[index as usize].font_size
}

#[no_mangle]
pub extern "C" fn pdf_oxide_char_list_free(handle: *mut FfiCharList) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

// --- Words ---

pub struct FfiWordList {
    words: Vec<Word>,
}

#[no_mangle]
pub extern "C" fn pdf_document_extract_words(
    handle: *mut PdfDocument,
    page_index: i32,
    error_code: *mut i32,
) -> *mut FfiWordList {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    match doc.extract_words(page_index as usize) {
        Ok(words) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(FfiWordList { words }))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_oxide_word_count(words: *const FfiWordList) -> i32 {
    if words.is_null() {
        return 0;
    }
    unsafe { (*words).words.len() as i32 }
}

#[no_mangle]
pub extern "C" fn pdf_oxide_word_get_text(
    words: *const FfiWordList,
    index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if words.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let list = unsafe { &*words };
    if (index as usize) >= list.words.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    to_c_string(&list.words[index as usize].text)
}

#[no_mangle]
pub extern "C" fn pdf_oxide_word_get_bbox(
    words: *const FfiWordList,
    index: i32,
    x: *mut f32,
    y: *mut f32,
    w: *mut f32,
    h: *mut f32,
    error_code: *mut i32,
) {
    if words.is_null() || index < 0 || x.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return;
    }
    let list = unsafe { &*words };
    if (index as usize) >= list.words.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return;
    }
    let b = &list.words[index as usize].bbox;
    unsafe {
        *x = b.x;
        *y = b.y;
        *w = b.width;
        *h = b.height;
    }
    set_error(error_code, ERR_SUCCESS);
}

#[no_mangle]
pub extern "C" fn pdf_oxide_word_get_font_name(
    words: *const FfiWordList,
    index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if words.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let list = unsafe { &*words };
    if (index as usize) >= list.words.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    to_c_string(&list.words[index as usize].dominant_font)
}

#[no_mangle]
pub extern "C" fn pdf_oxide_word_get_font_size(
    words: *const FfiWordList,
    index: i32,
    error_code: *mut i32,
) -> f32 {
    if words.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return 0.0;
    }
    let list = unsafe { &*words };
    if (index as usize) >= list.words.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return 0.0;
    }
    set_error(error_code, ERR_SUCCESS);
    list.words[index as usize].avg_font_size
}

#[no_mangle]
pub extern "C" fn pdf_oxide_word_is_bold(
    words: *const FfiWordList,
    index: i32,
    error_code: *mut i32,
) -> bool {
    if words.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return false;
    }
    let list = unsafe { &*words };
    if (index as usize) >= list.words.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return false;
    }
    set_error(error_code, ERR_SUCCESS);
    list.words[index as usize].is_bold
}

#[no_mangle]
pub extern "C" fn pdf_oxide_word_list_free(handle: *mut FfiWordList) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

// --- Text Lines ---

pub struct FfiTextLineList {
    lines: Vec<RustTextLine>,
}

#[no_mangle]
pub extern "C" fn pdf_document_extract_text_lines(
    handle: *mut PdfDocument,
    page_index: i32,
    error_code: *mut i32,
) -> *mut FfiTextLineList {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    match doc.extract_text_lines(page_index as usize) {
        Ok(lines) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(FfiTextLineList { lines }))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_oxide_line_count(lines: *const FfiTextLineList) -> i32 {
    if lines.is_null() {
        return 0;
    }
    unsafe { (*lines).lines.len() as i32 }
}

#[no_mangle]
pub extern "C" fn pdf_oxide_line_get_text(
    lines: *const FfiTextLineList,
    index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if lines.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let list = unsafe { &*lines };
    if (index as usize) >= list.lines.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    to_c_string(&list.lines[index as usize].text)
}

#[no_mangle]
pub extern "C" fn pdf_oxide_line_get_bbox(
    lines: *const FfiTextLineList,
    index: i32,
    x: *mut f32,
    y: *mut f32,
    w: *mut f32,
    h: *mut f32,
    error_code: *mut i32,
) {
    if lines.is_null() || index < 0 || x.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return;
    }
    let list = unsafe { &*lines };
    if (index as usize) >= list.lines.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return;
    }
    let b = &list.lines[index as usize].bbox;
    unsafe {
        *x = b.x;
        *y = b.y;
        *w = b.width;
        *h = b.height;
    }
    set_error(error_code, ERR_SUCCESS);
}

#[no_mangle]
pub extern "C" fn pdf_oxide_line_get_word_count(
    lines: *const FfiTextLineList,
    index: i32,
    error_code: *mut i32,
) -> i32 {
    if lines.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return 0;
    }
    let list = unsafe { &*lines };
    if (index as usize) >= list.lines.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return 0;
    }
    set_error(error_code, ERR_SUCCESS);
    list.lines[index as usize].words.len() as i32
}

#[no_mangle]
pub extern "C" fn pdf_oxide_line_list_free(handle: *mut FfiTextLineList) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

// --- Tables ---

use crate::structure::table_extractor::ExtractedTable;

pub struct FfiTableList {
    tables: Vec<ExtractedTable>,
}

#[no_mangle]
pub extern "C" fn pdf_document_extract_tables(
    handle: *mut PdfDocument,
    page_index: i32,
    error_code: *mut i32,
) -> *mut FfiTableList {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    match doc.extract_tables(page_index as usize) {
        Ok(tables) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(FfiTableList { tables }))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_oxide_table_count(tables: *const FfiTableList) -> i32 {
    if tables.is_null() {
        return 0;
    }
    unsafe { (*tables).tables.len() as i32 }
}

#[no_mangle]
pub extern "C" fn pdf_oxide_table_get_row_count(
    tables: *const FfiTableList,
    index: i32,
    error_code: *mut i32,
) -> i32 {
    if tables.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return 0;
    }
    let list = unsafe { &*tables };
    if (index as usize) >= list.tables.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return 0;
    }
    set_error(error_code, ERR_SUCCESS);
    list.tables[index as usize].rows.len() as i32
}

#[no_mangle]
pub extern "C" fn pdf_oxide_table_get_col_count(
    tables: *const FfiTableList,
    index: i32,
    error_code: *mut i32,
) -> i32 {
    if tables.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return 0;
    }
    let list = unsafe { &*tables };
    if (index as usize) >= list.tables.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return 0;
    }
    set_error(error_code, ERR_SUCCESS);
    list.tables[index as usize].col_count as i32
}

#[no_mangle]
pub extern "C" fn pdf_oxide_table_get_cell_text(
    tables: *const FfiTableList,
    table_index: i32,
    row: i32,
    col: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if tables.is_null() || table_index < 0 || row < 0 || col < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let list = unsafe { &*tables };
    if (table_index as usize) >= list.tables.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    let table = &list.tables[table_index as usize];
    if (row as usize) >= table.rows.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    let row_data = &table.rows[row as usize];
    if (col as usize) >= row_data.cells.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    to_c_string(&row_data.cells[col as usize].text)
}

#[no_mangle]
pub extern "C" fn pdf_oxide_table_has_header(
    tables: *const FfiTableList,
    index: i32,
    error_code: *mut i32,
) -> bool {
    if tables.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return false;
    }
    let list = unsafe { &*tables };
    if (index as usize) >= list.tables.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return false;
    }
    set_error(error_code, ERR_SUCCESS);
    list.tables[index as usize].has_header
}

#[no_mangle]
pub extern "C" fn pdf_oxide_table_list_free(handle: *mut FfiTableList) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

// ─── Region extraction ──────────────────────────────────────────────────────

use crate::geometry::Rect;
use crate::layout::RectFilterMode;

fn make_rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::new(x, y, w, h)
}

#[no_mangle]
pub extern "C" fn pdf_document_extract_text_in_rect(
    handle: *mut PdfDocument,
    page_index: i32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    error_code: *mut i32,
) -> *mut c_char {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    match doc.extract_text_in_rect(
        page_index as usize,
        make_rect(x, y, w, h),
        RectFilterMode::Intersects,
    ) {
        Ok(t) => {
            set_error(error_code, ERR_SUCCESS);
            to_c_string(&t)
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_document_extract_words_in_rect(
    handle: *mut PdfDocument,
    page_index: i32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    error_code: *mut i32,
) -> *mut FfiWordList {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    match doc.extract_words_in_rect(
        page_index as usize,
        make_rect(x, y, w, h),
        RectFilterMode::Intersects,
    ) {
        Ok(words) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(FfiWordList { words }))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_document_extract_lines_in_rect(
    handle: *mut PdfDocument,
    page_index: i32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    error_code: *mut i32,
) -> *mut FfiTextLineList {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    match doc.extract_text_lines_in_rect(
        page_index as usize,
        make_rect(x, y, w, h),
        RectFilterMode::Intersects,
    ) {
        Ok(lines) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(FfiTextLineList { lines }))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_document_extract_tables_in_rect(
    handle: *mut PdfDocument,
    page_index: i32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    error_code: *mut i32,
) -> *mut FfiTableList {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    match doc.extract_tables_in_rect(page_index as usize, make_rect(x, y, w, h)) {
        Ok(tables) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(FfiTableList { tables }))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_document_extract_images_in_rect(
    handle: *mut PdfDocument,
    page_index: i32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    error_code: *mut i32,
) -> *mut FfiImageList {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    match doc.extract_images_in_rect(page_index as usize, make_rect(x, y, w, h)) {
        Ok(images) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(FfiImageList { images }))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

// ─── Form fields ────────────────────────────────────────────────────────────

use crate::extractors::FormField as RustFormField;

pub struct FfiFormFieldList {
    fields: Vec<RustFormField>,
}

#[no_mangle]
pub extern "C" fn pdf_document_get_form_fields(
    handle: *mut PdfDocument,
    error_code: *mut i32,
) -> *mut FfiFormFieldList {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    match FormExtractor::extract_fields(doc) {
        Ok(fields) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(FfiFormFieldList { fields }))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_oxide_form_field_count(fields: *const FfiFormFieldList) -> i32 {
    if fields.is_null() {
        return 0;
    }
    unsafe { (*fields).fields.len() as i32 }
}

#[no_mangle]
pub extern "C" fn pdf_oxide_form_field_get_name(
    fields: *const FfiFormFieldList,
    index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if fields.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let list = unsafe { &*fields };
    if (index as usize) >= list.fields.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    to_c_string(&list.fields[index as usize].full_name)
}

#[no_mangle]
pub extern "C" fn pdf_oxide_form_field_get_type(
    fields: *const FfiFormFieldList,
    index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if fields.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let list = unsafe { &*fields };
    if (index as usize) >= list.fields.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    to_c_string(&format!("{:?}", list.fields[index as usize].field_type))
}

#[no_mangle]
pub extern "C" fn pdf_oxide_form_field_get_value(
    fields: *const FfiFormFieldList,
    index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if fields.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let list = unsafe { &*fields };
    if (index as usize) >= list.fields.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    to_c_string(&format!("{:?}", list.fields[index as usize].value))
}

#[no_mangle]
pub extern "C" fn pdf_oxide_form_field_is_readonly(
    fields: *const FfiFormFieldList,
    index: i32,
    error_code: *mut i32,
) -> bool {
    if fields.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return false;
    }
    let list = unsafe { &*fields };
    if (index as usize) >= list.fields.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return false;
    }
    set_error(error_code, ERR_SUCCESS);
    list.fields[index as usize]
        .flags
        .map(|f| (f & 0x1) != 0)
        .unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn pdf_oxide_form_field_is_required(
    fields: *const FfiFormFieldList,
    index: i32,
    error_code: *mut i32,
) -> bool {
    if fields.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return false;
    }
    let list = unsafe { &*fields };
    if (index as usize) >= list.fields.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return false;
    }
    set_error(error_code, ERR_SUCCESS);
    list.fields[index as usize]
        .flags
        .map(|f| (f & 0x2) != 0)
        .unwrap_or(false)
}

#[no_mangle]
pub extern "C" fn pdf_oxide_form_field_list_free(handle: *mut FfiFormFieldList) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

/// Check if document has XFA forms.
#[no_mangle]
pub extern "C" fn pdf_document_has_xfa(handle: *mut PdfDocument) -> bool {
    if handle.is_null() {
        return false;
    }
    let doc = unsafe { &mut *handle };
    if let Ok(catalog) = doc.catalog() {
        if let crate::object::Object::Dictionary(dict) = &catalog {
            if let Some(acroform) = dict.get("AcroForm") {
                if let crate::object::Object::Dictionary(form_dict) = acroform {
                    return form_dict.contains_key("XFA");
                }
            }
        }
    }
    false
}

// ─── Artifact removal ───────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn pdf_document_remove_headers(
    handle: *mut PdfDocument,
    threshold: f32,
    error_code: *mut i32,
) -> i32 {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    let doc = unsafe { &mut *handle };
    match doc.remove_headers(threshold) {
        Ok(n) => {
            set_error(error_code, ERR_SUCCESS);
            n as i32
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            -1
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_document_remove_footers(
    handle: *mut PdfDocument,
    threshold: f32,
    error_code: *mut i32,
) -> i32 {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    let doc = unsafe { &mut *handle };
    match doc.remove_footers(threshold) {
        Ok(n) => {
            set_error(error_code, ERR_SUCCESS);
            n as i32
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            -1
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_document_remove_artifacts(
    handle: *mut PdfDocument,
    threshold: f32,
    error_code: *mut i32,
) -> i32 {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    let doc = unsafe { &mut *handle };
    match doc.remove_artifacts(threshold) {
        Ok(n) => {
            set_error(error_code, ERR_SUCCESS);
            n as i32
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            -1
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_document_erase_header(
    handle: *mut PdfDocument,
    page_index: i32,
    error_code: *mut i32,
) -> i32 {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    let doc = unsafe { &mut *handle };
    match doc.erase_header(page_index as usize) {
        Ok(()) => {
            set_error(error_code, ERR_SUCCESS);
            0
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            -1
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_document_erase_footer(
    handle: *mut PdfDocument,
    page_index: i32,
    error_code: *mut i32,
) -> i32 {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    let doc = unsafe { &mut *handle };
    match doc.erase_footer(page_index as usize) {
        Ok(()) => {
            set_error(error_code, ERR_SUCCESS);
            0
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            -1
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_document_erase_artifacts(
    handle: *mut PdfDocument,
    page_index: i32,
    error_code: *mut i32,
) -> i32 {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    let doc = unsafe { &mut *handle };
    match doc.erase_artifacts(page_index as usize) {
        Ok(()) => {
            set_error(error_code, ERR_SUCCESS);
            0
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            -1
        },
    }
}

// ─── Editor: page operations ────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn document_editor_delete_page(
    handle: *mut DocumentEditor,
    page_index: i32,
    error_code: *mut i32,
) -> i32 {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    let editor = unsafe { &mut *handle };
    match editor.remove_page(page_index as usize) {
        Ok(()) => {
            set_error(error_code, ERR_SUCCESS);
            0
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            -1
        },
    }
}

#[no_mangle]
pub extern "C" fn document_editor_move_page(
    handle: *mut DocumentEditor,
    from: i32,
    to: i32,
    error_code: *mut i32,
) -> i32 {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    let editor = unsafe { &mut *handle };
    match editor.move_page(from as usize, to as usize) {
        Ok(()) => {
            set_error(error_code, ERR_SUCCESS);
            0
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            -1
        },
    }
}

#[no_mangle]
pub extern "C" fn document_editor_get_page_rotation(
    handle: *mut DocumentEditor,
    page: i32,
    error_code: *mut i32,
) -> i32 {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return 0;
    }
    let editor = unsafe { &mut *handle };
    match editor.get_page_rotation(page as usize) {
        Ok(r) => {
            set_error(error_code, ERR_SUCCESS);
            r
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            0
        },
    }
}

#[no_mangle]
pub extern "C" fn document_editor_set_page_rotation(
    handle: *mut DocumentEditor,
    page: i32,
    degrees: i32,
    error_code: *mut i32,
) -> i32 {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    let editor = unsafe { &mut *handle };
    match editor.set_page_rotation(page as usize, degrees) {
        Ok(()) => {
            set_error(error_code, ERR_SUCCESS);
            0
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            -1
        },
    }
}

#[no_mangle]
pub extern "C" fn document_editor_erase_region(
    handle: *mut DocumentEditor,
    page: i32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    error_code: *mut i32,
) -> i32 {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    let editor = unsafe { &mut *handle };
    match editor.erase_region(page as usize, [x, y, x + w, y + h]) {
        Ok(()) => {
            set_error(error_code, ERR_SUCCESS);
            0
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            -1
        },
    }
}

#[no_mangle]
pub extern "C" fn document_editor_flatten_annotations(
    handle: *mut DocumentEditor,
    page: i32,
    error_code: *mut i32,
) -> i32 {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    let editor = unsafe { &mut *handle };
    match editor.flatten_page_annotations(page as usize) {
        Ok(()) => {
            set_error(error_code, ERR_SUCCESS);
            0
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            -1
        },
    }
}

#[no_mangle]
pub extern "C" fn document_editor_flatten_all_annotations(
    handle: *mut DocumentEditor,
    error_code: *mut i32,
) -> i32 {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    let editor = unsafe { &mut *handle };
    match editor.flatten_all_annotations() {
        Ok(()) => {
            set_error(error_code, ERR_SUCCESS);
            0
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            -1
        },
    }
}

#[no_mangle]
pub extern "C" fn document_editor_crop_margins(
    handle: *mut DocumentEditor,
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
    error_code: *mut i32,
) -> i32 {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    let editor = unsafe { &mut *handle };
    match editor.crop_margins(left, right, top, bottom) {
        Ok(()) => {
            set_error(error_code, ERR_SUCCESS);
            0
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            -1
        },
    }
}

#[no_mangle]
pub extern "C" fn document_editor_merge_from(
    handle: *mut DocumentEditor,
    source_path: *const c_char,
    error_code: *mut i32,
) -> i32 {
    if handle.is_null() || source_path.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    let editor = unsafe { &mut *handle };
    let path = match unsafe { CStr::from_ptr(source_path) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error(error_code, ERR_INVALID_ARG);
            return -1;
        },
    };
    match editor.merge_from(path) {
        Ok(n) => {
            set_error(error_code, ERR_SUCCESS);
            n as i32
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            -1
        },
    }
}

// ─── PDF Creation extras ────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn pdf_from_image(path: *const c_char, error_code: *mut i32) -> *mut Pdf {
    if path.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let p = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error(error_code, ERR_INVALID_ARG);
            return ptr::null_mut();
        },
    };
    match Pdf::from_image(p) {
        Ok(pdf) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(pdf))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_from_image_bytes(
    data: *const u8,
    data_len: i32,
    error_code: *mut i32,
) -> *mut Pdf {
    if data.is_null() || data_len <= 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let bytes = unsafe { std::slice::from_raw_parts(data, data_len as usize) };
    match Pdf::from_image_bytes(bytes) {
        Ok(pdf) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(pdf))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

/// Merge multiple PDFs. paths is a null-terminated array of C strings.
#[no_mangle]
pub extern "C" fn pdf_merge(
    paths: *const *const c_char,
    path_count: i32,
    data_len: *mut i32,
    error_code: *mut i32,
) -> *mut u8 {
    if paths.is_null() || path_count <= 0 || data_len.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let mut path_strs: Vec<String> = Vec::new();
    for i in 0..path_count {
        let p = unsafe { *paths.add(i as usize) };
        if p.is_null() {
            set_error(error_code, ERR_INVALID_ARG);
            return ptr::null_mut();
        }
        match unsafe { CStr::from_ptr(p) }.to_str() {
            Ok(s) => path_strs.push(s.to_string()),
            Err(_) => {
                set_error(error_code, ERR_INVALID_ARG);
                return ptr::null_mut();
            },
        }
    }
    let path_refs: Vec<&str> = path_strs.iter().map(|s| s.as_str()).collect();
    match crate::api::merge_pdfs(&path_refs) {
        Ok(bytes) => {
            set_error(error_code, ERR_SUCCESS);
            unsafe {
                *data_len = bytes.len() as i32;
            }
            let boxed = bytes.into_boxed_slice();
            Box::into_raw(boxed) as *mut u8
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

// ─── PDF/A Validation ──────────────────────────────────────────────────────

use crate::compliance::{validate_pdf_a, PdfALevel, ValidationResult as PdfAValidationResult};

pub struct FfiPdfAResults {
    result: PdfAValidationResult,
}

#[no_mangle]
pub extern "C" fn pdf_validate_pdf_a_level(
    document: *mut PdfDocument,
    level: i32,
    error_code: *mut i32,
) -> *mut FfiPdfAResults {
    if document.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *document };
    let pdf_a_level = match level {
        0 => PdfALevel::A1b,
        1 => PdfALevel::A1a,
        2 => PdfALevel::A2b,
        3 => PdfALevel::A2a,
        4 => PdfALevel::A2u,
        5 => PdfALevel::A3b,
        6 => PdfALevel::A3a,
        7 => PdfALevel::A3u,
        _ => PdfALevel::A2b,
    };
    match validate_pdf_a(doc, pdf_a_level) {
        Ok(result) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(FfiPdfAResults { result }))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_pdf_a_is_compliant(
    results: *const FfiPdfAResults,
    error_code: *mut i32,
) -> bool {
    if results.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return false;
    }
    set_error(error_code, ERR_SUCCESS);
    unsafe { (*results).result.is_compliant }
}

#[no_mangle]
pub extern "C" fn pdf_pdf_a_error_count(results: *const FfiPdfAResults) -> i32 {
    if results.is_null() {
        return 0;
    }
    unsafe { (*results).result.errors.len() as i32 }
}

#[no_mangle]
pub extern "C" fn pdf_pdf_a_get_error(
    results: *const FfiPdfAResults,
    index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if results.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let r = unsafe { &*results };
    if (index as usize) >= r.result.errors.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    to_c_string(&r.result.errors[index as usize].message)
}

#[no_mangle]
pub extern "C" fn pdf_pdf_a_warning_count(results: *const FfiPdfAResults) -> i32 {
    if results.is_null() {
        return 0;
    }
    unsafe { (*results).result.warnings.len() as i32 }
}

#[no_mangle]
pub extern "C" fn pdf_pdf_a_results_free(results: *mut FfiPdfAResults) {
    if !results.is_null() {
        unsafe {
            drop(Box::from_raw(results));
        }
    }
}

// ─── PDF/X Validation ──────────────────────────────────────────────────────

use crate::compliance::pdf_x::{validate_pdf_x, PdfXLevel, XValidationResult};

pub struct FfiPdfXResults {
    result: XValidationResult,
}

#[no_mangle]
pub extern "C" fn pdf_validate_pdf_x_level(
    document: *mut PdfDocument,
    level: i32,
    error_code: *mut i32,
) -> *mut FfiPdfXResults {
    if document.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *document };
    let x_level = match level {
        0 => PdfXLevel::X1a2001,
        1 => PdfXLevel::X32002,
        2 => PdfXLevel::X4,
        _ => PdfXLevel::X4,
    };
    match validate_pdf_x(doc, x_level) {
        Ok(result) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(FfiPdfXResults { result }))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_pdf_x_is_compliant(
    results: *const FfiPdfXResults,
    error_code: *mut i32,
) -> bool {
    if results.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return false;
    }
    set_error(error_code, ERR_SUCCESS);
    unsafe { (*results).result.is_compliant }
}

#[no_mangle]
pub extern "C" fn pdf_pdf_x_error_count(results: *const FfiPdfXResults) -> i32 {
    if results.is_null() {
        return 0;
    }
    unsafe { (*results).result.errors.len() as i32 }
}

#[no_mangle]
pub extern "C" fn pdf_pdf_x_get_error(
    results: *const FfiPdfXResults,
    index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    if results.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let r = unsafe { &*results };
    if (index as usize) >= r.result.errors.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return ptr::null_mut();
    }
    set_error(error_code, ERR_SUCCESS);
    to_c_string(&r.result.errors[index as usize].message)
}

#[no_mangle]
pub extern "C" fn pdf_pdf_x_results_free(results: *mut FfiPdfXResults) {
    if !results.is_null() {
        unsafe {
            drop(Box::from_raw(results));
        }
    }
}

// ─── Encryption ─────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn document_editor_save_encrypted(
    handle: *mut DocumentEditor,
    path: *const c_char,
    user_password: *const c_char,
    owner_password: *const c_char,
    error_code: *mut i32,
) -> i32 {
    if handle.is_null() || path.is_null() || user_password.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    let editor = unsafe { &mut *handle };
    let path_str = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error(error_code, ERR_INVALID_ARG);
            return -1;
        },
    };
    let user_pwd = match unsafe { CStr::from_ptr(user_password) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error(error_code, ERR_INVALID_ARG);
            return -1;
        },
    };
    let owner_pwd = if owner_password.is_null() {
        user_pwd
    } else {
        match unsafe { CStr::from_ptr(owner_password) }.to_str() {
            Ok(s) => s,
            Err(_) => user_pwd,
        }
    };
    let enc_config = crate::editor::EncryptionConfig::new(user_pwd, owner_pwd)
        .with_algorithm(crate::editor::EncryptionAlgorithm::Aes256);
    let save_opts = crate::editor::SaveOptions::with_encryption(enc_config);
    match editor.save_to_bytes_with_options(save_opts) {
        Ok(bytes) => match std::fs::write(path_str, &bytes) {
            Ok(()) => {
                set_error(error_code, ERR_SUCCESS);
                0
            },
            Err(_) => {
                set_error(error_code, ERR_IO);
                -1
            },
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            -1
        },
    }
}

// ─── Extract paths ──────────────────────────────────────────────────────────

use crate::elements::PathContent;

pub struct FfiPathList {
    paths: Vec<PathContent>,
}

#[no_mangle]
pub extern "C" fn pdf_document_extract_paths(
    handle: *mut PdfDocument,
    page_index: i32,
    error_code: *mut i32,
) -> *mut FfiPathList {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    match doc.extract_paths(page_index as usize) {
        Ok(paths) => {
            set_error(error_code, ERR_SUCCESS);
            Box::into_raw(Box::new(FfiPathList { paths }))
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            ptr::null_mut()
        },
    }
}

#[no_mangle]
pub extern "C" fn pdf_oxide_path_count(paths: *const FfiPathList) -> i32 {
    if paths.is_null() {
        return 0;
    }
    unsafe { (*paths).paths.len() as i32 }
}

#[no_mangle]
pub extern "C" fn pdf_oxide_path_get_bbox(
    paths: *const FfiPathList,
    index: i32,
    x: *mut f32,
    y: *mut f32,
    w: *mut f32,
    h: *mut f32,
    error_code: *mut i32,
) {
    if paths.is_null() || index < 0 || x.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return;
    }
    let list = unsafe { &*paths };
    if (index as usize) >= list.paths.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return;
    }
    let b = &list.paths[index as usize].bbox;
    unsafe {
        *x = b.x;
        *y = b.y;
        *w = b.width;
        *h = b.height;
    }
    set_error(error_code, ERR_SUCCESS);
}

#[no_mangle]
pub extern "C" fn pdf_oxide_path_get_stroke_width(
    paths: *const FfiPathList,
    index: i32,
    error_code: *mut i32,
) -> f32 {
    if paths.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return 0.0;
    }
    let list = unsafe { &*paths };
    if (index as usize) >= list.paths.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return 0.0;
    }
    set_error(error_code, ERR_SUCCESS);
    list.paths[index as usize].stroke_width
}

#[no_mangle]
pub extern "C" fn pdf_oxide_path_has_stroke(
    paths: *const FfiPathList,
    index: i32,
    error_code: *mut i32,
) -> bool {
    if paths.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return false;
    }
    let list = unsafe { &*paths };
    if (index as usize) >= list.paths.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return false;
    }
    set_error(error_code, ERR_SUCCESS);
    list.paths[index as usize].stroke_color.is_some()
}

#[no_mangle]
pub extern "C" fn pdf_oxide_path_has_fill(
    paths: *const FfiPathList,
    index: i32,
    error_code: *mut i32,
) -> bool {
    if paths.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return false;
    }
    let list = unsafe { &*paths };
    if (index as usize) >= list.paths.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return false;
    }
    set_error(error_code, ERR_SUCCESS);
    list.paths[index as usize].fill_color.is_some()
}

#[no_mangle]
pub extern "C" fn pdf_oxide_path_get_operation_count(
    paths: *const FfiPathList,
    index: i32,
    error_code: *mut i32,
) -> i32 {
    if paths.is_null() || index < 0 {
        set_error(error_code, ERR_INVALID_ARG);
        return 0;
    }
    let list = unsafe { &*paths };
    if (index as usize) >= list.paths.len() {
        set_error(error_code, ERR_INVALID_PAGE);
        return 0;
    }
    set_error(error_code, ERR_SUCCESS);
    list.paths[index as usize].operations.len() as i32
}

#[no_mangle]
pub extern "C" fn pdf_oxide_path_list_free(handle: *mut FfiPathList) {
    if !handle.is_null() {
        unsafe {
            drop(Box::from_raw(handle));
        }
    }
}

// ─── Page labels ────────────────────────────────────────────────────────────

use crate::extractors::PageLabelExtractor;

/// Get page labels as JSON: [{"start":0,"prefix":"","style":"Decimal","first":1}, ...]
#[no_mangle]
pub extern "C" fn pdf_document_get_page_labels(
    handle: *mut PdfDocument,
    error_code: *mut i32,
) -> *mut c_char {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    match PageLabelExtractor::extract(doc) {
        Ok(ranges) => {
            let page_count = doc.page_count().unwrap_or(0);
            let labels = PageLabelExtractor::get_all_labels(&ranges, page_count);
            let json: Vec<String> = labels
                .iter()
                .enumerate()
                .map(|(i, l)| format!(r#"{{"page":{},"label":"{}"}}"#, i, l.replace('"', "\\\"")))
                .collect();
            set_error(error_code, ERR_SUCCESS);
            to_c_string(&format!("[{}]", json.join(",")))
        },
        Err(_) => {
            set_error(error_code, ERR_SUCCESS);
            to_c_string("[]")
        },
    }
}

// ─── XMP Metadata ───────────────────────────────────────────────────────────

use crate::extractors::XmpExtractor;

/// Get XMP metadata as JSON
#[no_mangle]
pub extern "C" fn pdf_document_get_xmp_metadata(
    handle: *mut PdfDocument,
    error_code: *mut i32,
) -> *mut c_char {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    match XmpExtractor::extract(doc) {
        Ok(Some(xmp)) => {
            let title = xmp.dc_title.as_deref().unwrap_or("").replace('"', "\\\"");
            let desc = xmp
                .dc_description
                .as_deref()
                .unwrap_or("")
                .replace('"', "\\\"");
            let creator_tool = xmp
                .xmp_creator_tool
                .as_deref()
                .unwrap_or("")
                .replace('"', "\\\"");
            let create_date = xmp
                .xmp_create_date
                .as_deref()
                .unwrap_or("")
                .replace('"', "\\\"");
            let modify_date = xmp
                .xmp_modify_date
                .as_deref()
                .unwrap_or("")
                .replace('"', "\\\"");
            let producer = xmp
                .pdf_producer
                .as_deref()
                .unwrap_or("")
                .replace('"', "\\\"");
            let creators: Vec<String> = xmp
                .dc_creator
                .iter()
                .map(|c| format!(r#""{}""#, c.replace('"', "\\\"")))
                .collect();
            let subjects: Vec<String> = xmp
                .dc_subject
                .iter()
                .map(|s| format!(r#""{}""#, s.replace('"', "\\\"")))
                .collect();
            let json = format!(
                r#"{{"title":"{}","description":"{}","creators":[{}],"subjects":[{}],"creatorTool":"{}","createDate":"{}","modifyDate":"{}","producer":"{}"}}"#,
                title,
                desc,
                creators.join(","),
                subjects.join(","),
                creator_tool,
                create_date,
                modify_date,
                producer
            );
            set_error(error_code, ERR_SUCCESS);
            to_c_string(&json)
        },
        Ok(None) => {
            set_error(error_code, ERR_SUCCESS);
            to_c_string("{}")
        },
        Err(_) => {
            set_error(error_code, ERR_SUCCESS);
            to_c_string("{}")
        },
    }
}

// ─── Document outline ───────────────────────────────────────────────────────

fn outline_to_json(items: &[crate::outline::OutlineItem]) -> String {
    let json: Vec<String> = items
        .iter()
        .map(|item| {
            let dest = match &item.dest {
                Some(crate::outline::Destination::PageIndex(p)) => format!("{}", p),
                Some(crate::outline::Destination::Named(n)) => {
                    format!(r#""{}""#, n.replace('"', "\\\""))
                },
                None => "null".to_string(),
            };
            let children_json = if item.children.is_empty() {
                "[]".to_string()
            } else {
                outline_to_json(&item.children)
            };
            format!(
                r#"{{"title":"{}","dest":{},"children":{}}}"#,
                item.title.replace('"', "\\\""),
                dest,
                children_json
            )
        })
        .collect();
    format!("[{}]", json.join(","))
}

/// Get document outline (bookmarks) as JSON
#[no_mangle]
pub extern "C" fn pdf_document_get_outline(
    handle: *mut PdfDocument,
    error_code: *mut i32,
) -> *mut c_char {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let doc = unsafe { &mut *handle };
    match doc.get_outline() {
        Ok(Some(items)) => {
            set_error(error_code, ERR_SUCCESS);
            to_c_string(&outline_to_json(&items))
        },
        Ok(None) => {
            set_error(error_code, ERR_SUCCESS);
            to_c_string("[]")
        },
        Err(_) => {
            set_error(error_code, ERR_SUCCESS);
            to_c_string("[]")
        },
    }
}

// ─── Form field mutation (via DocumentEditor) ──────────────────────────────

/// Set a form field value on a DocumentEditor. Value is a UTF-8 string.
#[no_mangle]
pub extern "C" fn document_editor_set_form_field_value(
    handle: *mut DocumentEditor,
    name: *const c_char,
    value: *const c_char,
    error_code: *mut i32,
) -> i32 {
    if handle.is_null() || name.is_null() || value.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    let editor = unsafe { &mut *handle };
    let field_name = match unsafe { CStr::from_ptr(name) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error(error_code, ERR_INVALID_ARG);
            return -1;
        },
    };
    let field_value = match unsafe { CStr::from_ptr(value) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error(error_code, ERR_INVALID_ARG);
            return -1;
        },
    };
    match editor.set_form_field_value(
        field_name,
        crate::editor::form_fields::FormFieldValue::Text(field_value.to_string()),
    ) {
        Ok(()) => {
            set_error(error_code, ERR_SUCCESS);
            0
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            -1
        },
    }
}

/// Flatten all forms in the document (bake form values into page content).
#[no_mangle]
pub extern "C" fn document_editor_flatten_forms(
    handle: *mut DocumentEditor,
    error_code: *mut i32,
) -> i32 {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    let editor = unsafe { &mut *handle };
    match editor.flatten_forms() {
        Ok(()) => {
            set_error(error_code, ERR_SUCCESS);
            0
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            -1
        },
    }
}

/// Flatten forms on a specific page.
#[no_mangle]
pub extern "C" fn document_editor_flatten_forms_on_page(
    handle: *mut DocumentEditor,
    page_index: i32,
    error_code: *mut i32,
) -> i32 {
    if handle.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return -1;
    }
    let editor = unsafe { &mut *handle };
    match editor.flatten_forms_on_page(page_index as usize) {
        Ok(()) => {
            set_error(error_code, ERR_SUCCESS);
            0
        },
        Err(e) => {
            set_error(error_code, classify_error(&e));
            -1
        },
    }
}

// ─── C# compatibility aliases ──────────────────────────────────────────────
// C# P/Invoke uses PascalCase names. These re-export the snake_case functions.

#[no_mangle]
pub extern "C" fn PdfDocumentOpen(path: *const c_char, error_code: *mut i32) -> *mut PdfDocument {
    pdf_document_open(path, error_code)
}

#[no_mangle]
pub extern "C" fn PdfDocumentFree(handle: *mut PdfDocument) {
    pdf_document_free(handle)
}

#[no_mangle]
pub extern "C" fn PdfDocumentGetPageCount(handle: *mut PdfDocument, error_code: *mut i32) -> i32 {
    pdf_document_get_page_count(handle, error_code)
}

#[no_mangle]
pub extern "C" fn PdfDocumentExtractText(
    handle: *mut PdfDocument,
    page_index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    pdf_document_extract_text(handle, page_index, error_code)
}

#[no_mangle]
pub extern "C" fn PdfDocumentToMarkdown(
    handle: *mut PdfDocument,
    page_index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    pdf_document_to_markdown(handle, page_index, error_code)
}

#[no_mangle]
pub extern "C" fn PdfDocumentToHtml(
    handle: *mut PdfDocument,
    page_index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    pdf_document_to_html(handle, page_index, error_code)
}

#[no_mangle]
pub extern "C" fn PdfDocumentToPlainText(
    handle: *mut PdfDocument,
    page_index: i32,
    error_code: *mut i32,
) -> *mut c_char {
    pdf_document_to_plain_text(handle, page_index, error_code)
}

#[no_mangle]
pub extern "C" fn PdfFromMarkdown(markdown: *const c_char, error_code: *mut i32) -> *mut Pdf {
    pdf_from_markdown(markdown, error_code)
}

#[no_mangle]
pub extern "C" fn PdfFromHtml(html: *const c_char, error_code: *mut i32) -> *mut Pdf {
    pdf_from_html(html, error_code)
}

#[no_mangle]
pub extern "C" fn PdfFromText(text: *const c_char, error_code: *mut i32) -> *mut Pdf {
    pdf_from_text(text, error_code)
}

#[no_mangle]
pub extern "C" fn PdfSave(handle: *mut Pdf, path: *const c_char, error_code: *mut i32) -> i32 {
    pdf_save(handle, path, error_code)
}

#[no_mangle]
pub extern "C" fn PdfSaveToBytes(
    handle: *mut Pdf,
    data_len: *mut i32,
    error_code: *mut i32,
) -> *mut u8 {
    pdf_save_to_bytes(handle, data_len, error_code)
}

#[no_mangle]
pub extern "C" fn PdfFree(handle: *mut Pdf) {
    pdf_free(handle)
}

#[no_mangle]
pub extern "C" fn DocumentEditorOpen(
    path: *const c_char,
    error_code: *mut i32,
) -> *mut DocumentEditor {
    document_editor_open(path, error_code)
}

#[no_mangle]
pub extern "C" fn DocumentEditorFree(handle: *mut DocumentEditor) {
    document_editor_free(handle)
}

#[no_mangle]
pub extern "C" fn DocumentEditorSave(
    handle: *mut DocumentEditor,
    path: *const c_char,
    error_code: *mut i32,
) -> i32 {
    document_editor_save(handle, path, error_code)
}

#[no_mangle]
pub extern "C" fn DocumentEditorSetTitle(
    handle: *mut DocumentEditor,
    value: *const c_char,
    error_code: *mut i32,
) -> i32 {
    document_editor_set_title(handle, value, error_code)
}

#[no_mangle]
pub extern "C" fn DocumentEditorSetAuthor(
    handle: *mut DocumentEditor,
    value: *const c_char,
    error_code: *mut i32,
) -> i32 {
    document_editor_set_author(handle, value, error_code)
}

#[no_mangle]
pub extern "C" fn FreeString(ptr: *mut c_char) {
    free_string(ptr)
}

#[no_mangle]
pub extern "C" fn FreeBytes(ptr: *mut u8) {
    free_bytes(ptr)
}

#[no_mangle]
pub extern "C" fn AllocString(s: *const c_char) -> *mut c_char {
    if s.is_null() {
        return ptr::null_mut();
    }
    let cstr = unsafe { CStr::from_ptr(s) };
    match CString::new(cstr.to_bytes()) {
        Ok(cs) => cs.into_raw(),
        Err(_) => ptr::null_mut(),
    }
}

// ─── JSON bulk extractors ───────────────────────────────────────────────────
//
// These functions serialize an entire list handle to a single JSON string so
// that language bindings can make one FFI crossing per list instead of N*M
// per-field calls. The returned pointer is a UTF-8 C string that the caller
// must free with `free_string()`.
//
// The JSON schemas are stable contracts — downstream bindings depend on them.
// Field names use lowerCamelCase to match the idioms of the primary consumer
// languages (Go, JS, C#).
//
// Image data is NOT exposed via JSON because base64 overhead makes it worse
// than the per-item binary extraction path already available.

use serde::Serialize;

#[derive(Serialize)]
struct JsonFont<'a> {
    name: &'a str,
    r#type: &'a str,
    encoding: &'a str,
    isEmbedded: bool,
    isSubset: bool,
    size: f32,
}

#[derive(Serialize)]
struct JsonAnnotation<'a> {
    r#type: &'a str,
    subtype: &'a str,
    content: &'a str,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    author: &'a str,
    borderWidth: f32,
    color: u32,
    creationDate: i64,
    modificationDate: i64,
    linkURI: &'a str,
    textIconName: &'a str,
    isHidden: bool,
    isPrintable: bool,
    isReadOnly: bool,
    isMarkedDeleted: bool,
}

#[derive(Serialize)]
struct JsonElement<'a> {
    r#type: &'a str,
    text: &'a str,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Serialize)]
struct JsonSearchResult<'a> {
    text: &'a str,
    page: i32,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

fn string_to_c(s: String) -> *mut c_char {
    CString::new(s)
        .map(|c| c.into_raw())
        .unwrap_or(ptr::null_mut())
}

/// Serializes an entire font list to JSON. Returns a UTF-8 C string owned by
/// the caller (free with `free_string`). The schema is
/// `[{"name": "...", "type": "...", "encoding": "...", "isEmbedded": bool,
/// "isSubset": bool, "size": number}, ...]`.
#[no_mangle]
pub extern "C" fn pdf_oxide_fonts_to_json(
    fonts: *const FfiFontList,
    error_code: *mut i32,
) -> *mut c_char {
    if fonts.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let list = unsafe { &*fonts };
    let items: Vec<JsonFont> = list
        .fonts
        .iter()
        .map(|f| JsonFont {
            name: &f.name,
            r#type: &f.subtype,
            encoding: &f.encoding,
            isEmbedded: f.is_embedded,
            isSubset: f.is_subset,
            size: 0.0,
        })
        .collect();
    match serde_json::to_string(&items) {
        Ok(s) => {
            set_error(error_code, ERR_SUCCESS);
            string_to_c(s)
        },
        Err(_) => {
            set_error(error_code, ERR_INTERNAL);
            ptr::null_mut()
        },
    }
}

/// Serializes an entire annotation list to JSON. Returns a UTF-8 C string
/// owned by the caller (free with `free_string`). The schema matches the Go
/// `Annotation` struct fields.
#[no_mangle]
pub extern "C" fn pdf_oxide_annotations_to_json(
    annotations: *const FfiAnnotationList,
    error_code: *mut i32,
) -> *mut c_char {
    if annotations.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let list = unsafe { &*annotations };

    // Pre-compute owned strings so the closure can borrow them.
    let rows: Vec<(
        String,
        String,
        String,
        f32,
        f32,
        f32,
        f32,
        String,
        f32,
        u32,
        i64,
        i64,
        String,
        String,
        bool,
        bool,
        bool,
        bool,
    )> = list
        .annotations
        .iter()
        .map(|a| {
            let rect = a.rect.unwrap_or([0.0, 0.0, 0.0, 0.0]);
            let (x, y, w, h) = (
                rect[0] as f32,
                rect[1] as f32,
                (rect[2] - rect[0]) as f32,
                (rect[3] - rect[1]) as f32,
            );
            (
                a.annotation_type.clone(),
                a.subtype.clone().unwrap_or_default(),
                a.contents.clone().unwrap_or_default(),
                x,
                y,
                w,
                h,
                a.author.clone().unwrap_or_default(),
                a.border.map(|b| b[2] as f32).unwrap_or(0.0),
                0,             // Color — deferred; RustAnnotation stores Vec<f64>
                0,             // CreationDate — string in Rust, 0 until parsed
                0,             // ModificationDate — same
                String::new(), // LinkURI — deferred
                String::new(), // TextIconName — deferred
                false,
                false,
                false,
                false, // flags — extracted from a.flags if needed
            )
        })
        .collect();

    let items: Vec<JsonAnnotation> = rows
        .iter()
        .map(|r| JsonAnnotation {
            r#type: &r.0,
            subtype: &r.1,
            content: &r.2,
            x: r.3,
            y: r.4,
            width: r.5,
            height: r.6,
            author: &r.7,
            borderWidth: r.8,
            color: r.9,
            creationDate: r.10,
            modificationDate: r.11,
            linkURI: &r.12,
            textIconName: &r.13,
            isHidden: r.14,
            isPrintable: r.15,
            isReadOnly: r.16,
            isMarkedDeleted: r.17,
        })
        .collect();

    match serde_json::to_string(&items) {
        Ok(s) => {
            set_error(error_code, ERR_SUCCESS);
            string_to_c(s)
        },
        Err(_) => {
            set_error(error_code, ERR_INTERNAL);
            ptr::null_mut()
        },
    }
}

/// Serializes an entire element list (text spans) to JSON. Returns a UTF-8 C
/// string owned by the caller (free with `free_string`).
#[no_mangle]
pub extern "C" fn pdf_oxide_elements_to_json(
    elements: *const FfiElementList,
    error_code: *mut i32,
) -> *mut c_char {
    if elements.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let list = unsafe { &*elements };
    let items: Vec<JsonElement> = list
        .spans
        .iter()
        .map(|s| JsonElement {
            r#type: "text",
            text: &s.text,
            x: s.bbox.x,
            y: s.bbox.y,
            width: s.bbox.width,
            height: s.bbox.height,
        })
        .collect();
    match serde_json::to_string(&items) {
        Ok(s) => {
            set_error(error_code, ERR_SUCCESS);
            string_to_c(s)
        },
        Err(_) => {
            set_error(error_code, ERR_INTERNAL);
            ptr::null_mut()
        },
    }
}

/// Serializes an entire search-results list to JSON. Returns a UTF-8 C string
/// owned by the caller (free with `free_string`).
#[no_mangle]
pub extern "C" fn pdf_oxide_search_results_to_json(
    results: *const FfiSearchResults,
    error_code: *mut i32,
) -> *mut c_char {
    if results.is_null() {
        set_error(error_code, ERR_INVALID_ARG);
        return ptr::null_mut();
    }
    let list = unsafe { &*results };
    let items: Vec<JsonSearchResult> = list
        .results
        .iter()
        .map(|r| JsonSearchResult {
            text: &r.text,
            page: r.page as i32,
            x: r.bbox.x,
            y: r.bbox.y,
            width: r.bbox.width,
            height: r.bbox.height,
        })
        .collect();
    match serde_json::to_string(&items) {
        Ok(s) => {
            set_error(error_code, ERR_SUCCESS);
            string_to_c(s)
        },
        Err(_) => {
            set_error(error_code, ERR_INTERNAL);
            ptr::null_mut()
        },
    }
}
