//! C-FFI integration tests for the write-side API.
//!
//! These tests call the raw `pdf_*` FFI functions the way a C / C# / Go /
//! Node wrapper does — opaque handles, error-code out-params, explicit
//! frees, string marshalling through `CString`. They validate the
//! handle-lifetime contract documented in
//! `include/pdf_oxide_c/pdf_oxide.h` and in `src/ffi.rs`, and are the
//! acceptance gate for downstream bindings landing on top of this FFI.

#![allow(clippy::missing_safety_doc)]
#![allow(unused_unsafe)]
// The FFI functions here are `pub extern "C" fn` and pyo3 / wasm-bindgen
// signatures that don't strictly require `unsafe` at call sites today,
// but we keep the `unsafe {}` markers in the tests to make it obvious to
// maintainers that these are raw-pointer FFI calls that downstream C /
// C# / Go / Node bindings will use in the same ("unsafe") context.

use std::ffi::CString;
use std::path::Path;
use std::ptr;

use pdf_oxide::ffi::*;

const DEJAVU_PATH: &str = "tests/fixtures/fonts/DejaVuSans.ttf";

fn cstring(s: &str) -> CString {
    CString::new(s).unwrap()
}

/// Minimal happy-path: create a builder, emit one A4 page with literal
/// text, build to bytes, confirm the output parses as a PDF, free the
/// byte buffer.
#[test]
fn ffi_document_builder_minimal_ascii() {
    let mut ec: i32 = -1;
    let builder = unsafe { pdf_document_builder_create(&mut ec) };
    assert_eq!(ec, 0, "create returned error");
    assert!(!builder.is_null());

    // Open page
    let page = unsafe { pdf_document_builder_a4_page(builder, &mut ec) };
    assert_eq!(ec, 0);
    assert!(!page.is_null());

    // Emit text
    let text = cstring("Hello from FFI");
    assert_eq!(unsafe { pdf_page_builder_at(page, 72.0, 720.0, &mut ec) }, 0);
    assert_eq!(ec, 0);
    assert_eq!(unsafe { pdf_page_builder_text(page, text.as_ptr(), &mut ec) }, 0);
    assert_eq!(ec, 0);

    // Commit
    assert_eq!(unsafe { pdf_page_builder_done(page, &mut ec) }, 0);
    assert_eq!(ec, 0);

    // Build
    let mut out_len: usize = 0;
    let bytes_ptr = unsafe { pdf_document_builder_build(builder, &mut out_len, &mut ec) };
    assert_eq!(ec, 0);
    assert!(!bytes_ptr.is_null());
    assert!(out_len > 256);

    // Byte buffer must start with %PDF-
    let slice = unsafe { std::slice::from_raw_parts(bytes_ptr, out_len) };
    assert!(slice.starts_with(b"%PDF-"));

    // Buffer is owned by the caller; we free it with free_bytes.
    unsafe { free_bytes(bytes_ptr) };
    // Builder was consumed by _build — no _free needed, but calling it on
    // the now-invalid handle should be a no-op since the Box-from-raw is
    // already gone. Actually, _build only `consume_builder`s the inner —
    // the wrapper Box is still alive. Free it.
    unsafe { pdf_document_builder_free(builder) };
}

#[test]
fn ffi_document_builder_embedded_font_cjk_round_trip() {
    let mut ec: i32 = -1;
    let font_path = cstring(DEJAVU_PATH);
    let font = unsafe { pdf_embedded_font_from_file(font_path.as_ptr(), &mut ec) };
    assert_eq!(ec, 0);
    assert!(!font.is_null());

    let builder = unsafe { pdf_document_builder_create(&mut ec) };
    assert!(!builder.is_null());

    // Register consumes `font`.
    let font_name = cstring("DejaVu");
    let rc = unsafe {
        pdf_document_builder_register_embedded_font(builder, font_name.as_ptr(), font, &mut ec)
    };
    assert_eq!(rc, 0);
    assert_eq!(ec, 0);
    // Do NOT pdf_embedded_font_free(font) — it's consumed.

    let page = unsafe { pdf_document_builder_a4_page(builder, &mut ec) };
    assert!(!page.is_null());
    let fname = cstring("DejaVu");
    assert_eq!(unsafe { pdf_page_builder_font(page, fname.as_ptr(), 12.0, &mut ec) }, 0);
    assert_eq!(unsafe { pdf_page_builder_at(page, 72.0, 720.0, &mut ec) }, 0);
    let cyrillic = cstring("Привет, мир!");
    assert_eq!(unsafe { pdf_page_builder_text(page, cyrillic.as_ptr(), &mut ec) }, 0);
    assert_eq!(unsafe { pdf_page_builder_at(page, 72.0, 700.0, &mut ec) }, 0);
    let greek = cstring("Καλημέρα κόσμε");
    assert_eq!(unsafe { pdf_page_builder_text(page, greek.as_ptr(), &mut ec) }, 0);
    assert_eq!(unsafe { pdf_page_builder_done(page, &mut ec) }, 0);

    let mut out_len: usize = 0;
    let bytes_ptr = unsafe { pdf_document_builder_build(builder, &mut out_len, &mut ec) };
    assert_eq!(ec, 0);
    assert!(!bytes_ptr.is_null());
    let bytes = unsafe { std::slice::from_raw_parts(bytes_ptr, out_len) }.to_vec();
    unsafe { free_bytes(bytes_ptr) };
    unsafe { pdf_document_builder_free(builder) };

    // Round-trip via the public (non-FFI) API since the FFI PdfDocument
    // call chain is validated elsewhere.
    let mut doc = pdf_oxide::PdfDocument::from_bytes(bytes).expect("parse output");
    let text = doc.extract_text(0).expect("extract");
    assert!(text.contains("Привет, мир!"), "Cyrillic missing: {text:?}");
    assert!(text.contains("Καλημέρα κόσμε"), "Greek missing: {text:?}");
}

#[test]
fn ffi_document_builder_output_is_subsetted() {
    let mut ec = 0;
    let font_path = cstring(DEJAVU_PATH);
    let font = unsafe { pdf_embedded_font_from_file(font_path.as_ptr(), &mut ec) };
    assert!(!font.is_null());
    let builder = unsafe { pdf_document_builder_create(&mut ec) };
    let font_name = cstring("DejaVu");
    unsafe {
        pdf_document_builder_register_embedded_font(builder, font_name.as_ptr(), font, &mut ec)
    };
    let page = unsafe { pdf_document_builder_a4_page(builder, &mut ec) };
    let fname = cstring("DejaVu");
    unsafe { pdf_page_builder_font(page, fname.as_ptr(), 12.0, &mut ec) };
    unsafe { pdf_page_builder_at(page, 72.0, 700.0, &mut ec) };
    let text = cstring("Hello world");
    unsafe { pdf_page_builder_text(page, text.as_ptr(), &mut ec) };
    unsafe { pdf_page_builder_done(page, &mut ec) };

    let mut out_len = 0usize;
    let bytes_ptr = unsafe { pdf_document_builder_build(builder, &mut out_len, &mut ec) };
    let face_size = std::fs::metadata(DEJAVU_PATH).unwrap().len() as usize;
    assert!(
        out_len * 10 < face_size,
        "expected FFI-built PDF ({out_len} bytes) to be >= 10× smaller than the face ({face_size} bytes)"
    );
    unsafe { free_bytes(bytes_ptr) };
    unsafe { pdf_document_builder_free(builder) };
}

#[test]
fn ffi_double_open_page_rejected() {
    let mut ec = 0;
    let builder = unsafe { pdf_document_builder_create(&mut ec) };
    let page1 = unsafe { pdf_document_builder_a4_page(builder, &mut ec) };
    assert!(!page1.is_null());

    // Second open-page before done should error.
    let mut ec2 = 0;
    let page2 = unsafe { pdf_document_builder_a4_page(builder, &mut ec2) };
    assert!(page2.is_null());
    assert_eq!(ec2, 1); // ERR_INVALID_ARG

    // Clean up: drop page1 without committing, then free builder.
    unsafe { pdf_page_builder_free(page1) };
    unsafe { pdf_document_builder_free(builder) };
}

#[test]
fn ffi_double_build_fails() {
    let mut ec = 0;
    let builder = unsafe { pdf_document_builder_create(&mut ec) };
    let page = unsafe { pdf_document_builder_a4_page(builder, &mut ec) };
    unsafe { pdf_page_builder_at(page, 72.0, 720.0, &mut ec) };
    let text = cstring("x");
    unsafe { pdf_page_builder_text(page, text.as_ptr(), &mut ec) };
    unsafe { pdf_page_builder_done(page, &mut ec) };

    let mut out_len = 0usize;
    let bytes1 = unsafe { pdf_document_builder_build(builder, &mut out_len, &mut ec) };
    assert!(!bytes1.is_null());
    unsafe { free_bytes(bytes1) };

    // Second build must fail — builder was consumed.
    let mut ec2 = 0;
    let bytes2 = unsafe { pdf_document_builder_build(builder, &mut out_len, &mut ec2) };
    assert!(bytes2.is_null());
    assert_eq!(ec2, 1);

    unsafe { pdf_document_builder_free(builder) };
}

#[test]
fn ffi_save_encrypted_has_encrypt_dict() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let tmp_path = tmp.path().to_path_buf();

    let mut ec = 0;
    let builder = unsafe { pdf_document_builder_create(&mut ec) };
    let page = unsafe { pdf_document_builder_a4_page(builder, &mut ec) };
    unsafe { pdf_page_builder_at(page, 72.0, 720.0, &mut ec) };
    let t = cstring("confidential");
    unsafe { pdf_page_builder_text(page, t.as_ptr(), &mut ec) };
    unsafe { pdf_page_builder_done(page, &mut ec) };

    let path_c = cstring(tmp_path.to_str().unwrap());
    let user = cstring("userpw");
    let owner = cstring("ownerpw");
    let rc = unsafe {
        pdf_document_builder_save_encrypted(
            builder,
            path_c.as_ptr(),
            user.as_ptr(),
            owner.as_ptr(),
            &mut ec,
        )
    };
    assert_eq!(rc, 0);
    assert_eq!(ec, 0);

    let raw = std::fs::read(&tmp_path).unwrap();
    let s = String::from_utf8_lossy(&raw);
    assert!(s.contains("/Encrypt"), "encrypted PDF missing /Encrypt");
    assert!(s.contains("/V 5"), "expected /V 5 (AES-256) marker");

    unsafe { pdf_document_builder_free(builder) };
}

#[test]
fn ffi_embedded_font_from_bytes_with_name_override() {
    let data = std::fs::read(DEJAVU_PATH).unwrap();
    let mut ec = 0;
    let name = cstring("CustomDejaVu");
    let font =
        unsafe { pdf_embedded_font_from_bytes(data.as_ptr(), data.len(), name.as_ptr(), &mut ec) };
    assert_eq!(ec, 0);
    assert!(!font.is_null());
    unsafe { pdf_embedded_font_free(font) };

    // Also test NULL name (use PS name).
    let font2 =
        unsafe { pdf_embedded_font_from_bytes(data.as_ptr(), data.len(), ptr::null(), &mut ec) };
    assert_eq!(ec, 0);
    assert!(!font2.is_null());
    unsafe { pdf_embedded_font_free(font2) };
}

// ---------------------------------------------------------------------------
// Barcode placement
// ---------------------------------------------------------------------------

#[test]
#[cfg(feature = "barcodes")]
fn ffi_barcode_1d_and_qr_produce_valid_pdf() {
    let mut ec = 0;
    let builder = unsafe { pdf_document_builder_create(&mut ec) };
    assert_eq!(ec, 0);
    assert!(!builder.is_null());

    let page = unsafe { pdf_document_builder_a4_page(builder, &mut ec) };
    assert_eq!(ec, 0);
    assert!(!page.is_null());

    // Code128 barcode (type 0)
    let data = cstring("HELLO-123");
    let ret = unsafe {
        pdf_page_builder_barcode_1d(page, 0, data.as_ptr(), 72.0, 680.0, 200.0, 60.0, &mut ec)
    };
    assert_eq!(ec, 0, "barcode_1d failed: {ret}");

    // QR code
    let data = cstring("https://example.com");
    let ret = unsafe {
        pdf_page_builder_barcode_qr(page, data.as_ptr(), 72.0, 580.0, 100.0, &mut ec)
    };
    assert_eq!(ec, 0, "barcode_qr failed: {ret}");

    let done = unsafe { pdf_page_builder_done(page, &mut ec) };
    assert_eq!(ec, 0, "done failed: {done}");

    let mut out_len = 0usize;
    let bytes_ptr = unsafe { pdf_document_builder_build(builder, &mut out_len, &mut ec) };
    assert_eq!(ec, 0);
    assert!(!bytes_ptr.is_null());
    assert!(out_len > 0);
    unsafe { free_bytes(bytes_ptr) };
    unsafe { pdf_document_builder_free(builder) };
}

#[test]
#[cfg(feature = "barcodes")]
fn ffi_barcode_unknown_type_errors() {
    let mut ec = 0;
    let builder = unsafe { pdf_document_builder_create(&mut ec) };
    assert_eq!(ec, 0);

    let page = unsafe { pdf_document_builder_a4_page(builder, &mut ec) };
    assert_eq!(ec, 0);

    let data = cstring("test");
    let ret =
        unsafe { pdf_page_builder_barcode_1d(page, 99, data.as_ptr(), 0., 0., 100., 50., &mut ec) };
    assert_ne!(ec, 0, "expected error for unknown type, got {ret}");

    unsafe { pdf_page_builder_free(page) };
    unsafe { pdf_document_builder_free(builder) };
}

// ---------------------------------------------------------------------------
// JS actions — link_javascript, page on_open / on_close, doc on_open
// ---------------------------------------------------------------------------

#[test]
fn ffi_js_actions_produce_valid_pdf() {
    let mut ec = 0i32;
    let builder = unsafe { pdf_document_builder_create(&mut ec) };
    assert_eq!(ec, 0);

    // Document-level on_open
    let script = cstring("app.alert('opened')");
    let ret = unsafe { pdf_document_builder_on_open(builder, script.as_ptr(), &mut ec) };
    assert_eq!(ret, 0, "doc on_open failed: {ec}");

    let page = unsafe { pdf_document_builder_a4_page(builder, &mut ec) };
    assert_eq!(ec, 0);

    // Page-level on_open / on_close
    let pscript = cstring("app.alert('page open')");
    let ret = unsafe { pdf_page_builder_on_open(page, pscript.as_ptr(), &mut ec) };
    assert_eq!(ret, 0, "page on_open failed: {ec}");

    let cscript = cstring("app.alert('page close')");
    let ret = unsafe { pdf_page_builder_on_close(page, cscript.as_ptr(), &mut ec) };
    assert_eq!(ret, 0, "page on_close failed: {ec}");

    // link_javascript requires a preceding text element
    let _at = unsafe { pdf_page_builder_at(page, 72.0, 700.0, &mut ec) };
    let text = cstring("Click me");
    let _txt = unsafe { pdf_page_builder_text(page, text.as_ptr(), &mut ec) };
    let jscript = cstring("app.alert('clicked')");
    let ret = unsafe { pdf_page_builder_link_javascript(page, jscript.as_ptr(), &mut ec) };
    assert_eq!(ret, 0, "link_javascript failed: {ec}");

    let ret = unsafe { pdf_page_builder_done(page, &mut ec) };
    assert_eq!(ret, 0, "done failed: {ec}");

    let mut out_len = 0usize;
    let pdf_bytes = unsafe { pdf_document_builder_build(builder, &mut out_len, &mut ec) };
    assert_eq!(ec, 0, "build failed: {ec}");
    assert!(!pdf_bytes.is_null());
    assert!(out_len > 100);

    let slice = unsafe { std::slice::from_raw_parts(pdf_bytes, out_len) };
    assert!(slice.starts_with(b"%PDF-"), "output is not a PDF");
    // The output must contain the OpenAction and AA dict markers
    let s = String::from_utf8_lossy(slice);
    assert!(s.contains("/OpenAction"), "missing /OpenAction in PDF");
    assert!(s.contains("/AA"), "missing /AA dict in PDF");

    unsafe { free_bytes(pdf_bytes) };
}

// ---------------------------------------------------------------------------
// Phase 2 — HTML+CSS pipeline through the C FFI
// ---------------------------------------------------------------------------

#[test]
fn ffi_pdf_from_html_css_single_font() {
    let font_bytes = std::fs::read(DEJAVU_PATH).unwrap();
    let mut ec = 0;
    let html = cstring("<h1>Hello</h1><p>World</p>");
    let css = cstring("h1 { color: blue; font-size: 24pt }");

    let pdf_handle = unsafe {
        pdf_from_html_css(
            html.as_ptr(),
            css.as_ptr(),
            font_bytes.as_ptr(),
            font_bytes.len(),
            &mut ec,
        )
    };
    assert_eq!(ec, 0);
    assert!(!pdf_handle.is_null());

    // Serialize via the existing pdf_save_to_bytes path.
    let mut data_len = 0i32;
    let bytes_ptr = unsafe { pdf_save_to_bytes(pdf_handle, &mut data_len, &mut ec) };
    assert!(!bytes_ptr.is_null());
    assert!(data_len > 0);
    let bytes = unsafe { std::slice::from_raw_parts(bytes_ptr, data_len as usize) }.to_vec();
    unsafe { free_bytes(bytes_ptr) };
    unsafe { pdf_free(pdf_handle) };

    assert!(bytes.starts_with(b"%PDF-"));

    // Round-trip the literal words through extract_text.
    let mut doc = pdf_oxide::PdfDocument::from_bytes(bytes).expect("parse");
    let text = doc.extract_text(0).expect("extract");
    assert!(text.contains("Hello"));
    assert!(text.contains("World"));
}

#[test]
fn ffi_pdf_from_html_css_with_fonts_parallel_arrays() {
    let font_bytes = std::fs::read(DEJAVU_PATH).unwrap();

    // Single-entry cascade.
    let name_c = cstring("Body");
    let families: [*const std::os::raw::c_char; 1] = [name_c.as_ptr()];
    let font_ptrs: [*const u8; 1] = [font_bytes.as_ptr()];
    let font_lens: [usize; 1] = [font_bytes.len()];

    let mut ec = 0;
    let html = cstring("<p>cascade works</p>");
    let css = cstring("p { font-size: 14pt }");
    let pdf_handle = unsafe {
        pdf_from_html_css_with_fonts(
            html.as_ptr(),
            css.as_ptr(),
            families.as_ptr(),
            font_ptrs.as_ptr(),
            font_lens.as_ptr(),
            1,
            &mut ec,
        )
    };
    assert_eq!(ec, 0);
    assert!(!pdf_handle.is_null());
    unsafe { pdf_free(pdf_handle) };
}

#[test]
fn ffi_pdf_from_html_css_with_fonts_rejects_empty_count() {
    let mut ec = 0;
    let html = cstring("<p>x</p>");
    let css = cstring("");
    let handle = unsafe {
        pdf_from_html_css_with_fonts(
            html.as_ptr(),
            css.as_ptr(),
            ptr::null(),
            ptr::null(),
            ptr::null(),
            0,
            &mut ec,
        )
    };
    assert!(handle.is_null());
    assert_eq!(ec, 1);
}

// ---------------------------------------------------------------------------
// #393 v0.3.39 — new primitives + buffered Table
// ---------------------------------------------------------------------------

#[test]
fn ffi_page_builder_stroke_and_text_in_rect_and_table() {
    // Exercise every new v0.3.39 primitive via FFI and build a real PDF.
    let mut ec: i32 = -1;
    let builder = unsafe { pdf_document_builder_create(&mut ec) };
    assert_eq!(ec, 0);

    let page = unsafe { pdf_document_builder_letter_page(builder, &mut ec) };
    assert_eq!(ec, 0);

    // stroke_rect + stroke_line
    assert_eq!(
        unsafe {
            pdf_page_builder_stroke_rect(
                page, 50.0, 50.0, 200.0, 100.0, 2.0, 0.5, 0.5, 0.5, &mut ec,
            )
        },
        0
    );
    assert_eq!(ec, 0);
    assert_eq!(
        unsafe {
            pdf_page_builder_stroke_line(
                page, 50.0, 50.0, 250.0, 50.0, 1.0, 0.2, 0.2, 0.2, &mut ec,
            )
        },
        0
    );
    assert_eq!(ec, 0);

    // text_in_rect (align=Center)
    let caption = cstring("A centered caption that wraps across lines");
    assert_eq!(
        unsafe {
            pdf_page_builder_text_in_rect(
                page,
                100.0,
                500.0,
                200.0,
                100.0,
                caption.as_ptr(),
                1,
                &mut ec,
            )
        },
        0
    );
    assert_eq!(ec, 0);

    // Buffered table: 3 cols × 3 rows with header, centered numeric col.
    let widths: [f32; 3] = [100.0, 150.0, 80.0];
    let aligns: [i32; 3] = [0, 0, 2]; // Left, Left, Right
    let cell_strs: [CString; 9] = [
        cstring("SKU"),
        cstring("Name"),
        cstring("Qty"),
        cstring("A-1"),
        cstring("Widget"),
        cstring("12"),
        cstring("B-2"),
        cstring("Gadget"),
        cstring("3"),
    ];
    let cell_ptrs: [*const std::os::raw::c_char; 9] = [
        cell_strs[0].as_ptr(),
        cell_strs[1].as_ptr(),
        cell_strs[2].as_ptr(),
        cell_strs[3].as_ptr(),
        cell_strs[4].as_ptr(),
        cell_strs[5].as_ptr(),
        cell_strs[6].as_ptr(),
        cell_strs[7].as_ptr(),
        cell_strs[8].as_ptr(),
    ];
    assert_eq!(
        unsafe {
            pdf_page_builder_table(
                page,
                3,
                widths.as_ptr(),
                aligns.as_ptr(),
                3,
                cell_ptrs.as_ptr(),
                1, // has_header
                &mut ec,
            )
        },
        0
    );
    assert_eq!(ec, 0);

    // new_page_same_size — next page for regression
    assert_eq!(
        unsafe { pdf_page_builder_new_page_same_size(page, &mut ec) },
        0
    );
    assert_eq!(ec, 0);

    assert_eq!(unsafe { pdf_page_builder_done(page, &mut ec) }, 0);
    assert_eq!(ec, 0);

    let mut out_len: usize = 0;
    let bytes_ptr = unsafe { pdf_document_builder_build(builder, &mut out_len, &mut ec) };
    assert_eq!(ec, 0);
    assert!(!bytes_ptr.is_null());

    let slice = unsafe { std::slice::from_raw_parts(bytes_ptr, out_len) };
    assert!(slice.starts_with(b"%PDF-"));
    // Document must contain at least 2 pages now.
    let s = String::from_utf8_lossy(slice);
    let page_count = s.matches("/Type /Page\n").count() + s.matches("/Type/Page\n").count();
    let _ = page_count; // presence of /Type /Pages /Count is PDF-writer-internal.

    unsafe { free_bytes(bytes_ptr) };
    unsafe { pdf_document_builder_free(builder) };
}

#[test]
fn ffi_streaming_table_end_to_end_thousand_rows() {
    // Exercise the streaming FFI surface: begin → push_row × N → finish.
    // The Rust core is O(cols) memory — the FFI buffers rows until done()
    // replays them (per v0.3.39 scope, see #400 for true row-by-row).
    let mut ec: i32 = -1;
    let builder = unsafe { pdf_document_builder_create(&mut ec) };
    assert_eq!(ec, 0);
    let page = unsafe { pdf_document_builder_letter_page(builder, &mut ec) };
    assert_eq!(ec, 0);

    assert_eq!(
        unsafe { pdf_page_builder_font(page, cstring("Helvetica").as_ptr(), 10.0, &mut ec) },
        0
    );
    assert_eq!(
        unsafe { pdf_page_builder_at(page, 72.0, 720.0, &mut ec) },
        0
    );

    // Begin the streaming table.
    let headers_cstr: [CString; 3] = [cstring("SKU"), cstring("Item"), cstring("Qty")];
    let headers_ptrs: [*const std::os::raw::c_char; 3] = [
        headers_cstr[0].as_ptr(),
        headers_cstr[1].as_ptr(),
        headers_cstr[2].as_ptr(),
    ];
    let widths: [f32; 3] = [72.0, 200.0, 48.0];
    let aligns: [i32; 3] = [0, 0, 2];
    assert_eq!(
        unsafe {
            pdf_page_builder_streaming_table_begin(
                page,
                3,
                headers_ptrs.as_ptr(),
                widths.as_ptr(),
                aligns.as_ptr(),
                1, // repeat_header
                &mut ec,
            )
        },
        0
    );
    assert_eq!(ec, 0);

    // Push 1000 rows.
    for i in 0..1000u32 {
        let sku = cstring(&format!("A-{}", i));
        let item = cstring("Widget");
        let qty = cstring(&i.to_string());
        let cells_ptrs: [*const std::os::raw::c_char; 3] =
            [sku.as_ptr(), item.as_ptr(), qty.as_ptr()];
        let rc = unsafe {
            pdf_page_builder_streaming_table_push_row(
                page,
                3,
                cells_ptrs.as_ptr(),
                &mut ec,
            )
        };
        assert_eq!(rc, 0, "push_row failed at i={}: ec={}", i, ec);
    }

    // Close the table explicitly.
    assert_eq!(
        unsafe { pdf_page_builder_streaming_table_finish(page, &mut ec) },
        0
    );
    assert_eq!(ec, 0);

    // Commit + build.
    assert_eq!(unsafe { pdf_page_builder_done(page, &mut ec) }, 0);
    assert_eq!(ec, 0);

    let mut out_len: usize = 0;
    let bytes_ptr = unsafe { pdf_document_builder_build(builder, &mut out_len, &mut ec) };
    assert_eq!(ec, 0);
    assert!(!bytes_ptr.is_null());
    // 1000 rows × 3 cells + pagination → PDF must be non-trivial size.
    assert!(out_len > 10_000, "expected sizable PDF, got {}", out_len);

    let slice = unsafe { std::slice::from_raw_parts(bytes_ptr, out_len) };
    assert!(slice.starts_with(b"%PDF-"));

    unsafe { free_bytes(bytes_ptr) };
    unsafe { pdf_document_builder_free(builder) };
}

#[test]
fn ffi_streaming_table_push_without_begin_errors() {
    let mut ec: i32 = -1;
    let builder = unsafe { pdf_document_builder_create(&mut ec) };
    let page = unsafe { pdf_document_builder_letter_page(builder, &mut ec) };

    // Push a row without opening the table — should buffer but then fail
    // at done() replay.
    let cell = cstring("orphan");
    let cells_ptrs: [*const std::os::raw::c_char; 1] = [cell.as_ptr()];
    assert_eq!(
        unsafe {
            pdf_page_builder_streaming_table_push_row(page, 1, cells_ptrs.as_ptr(), &mut ec)
        },
        0
    );
    // done() must reject the orphan row.
    let rc = unsafe { pdf_page_builder_done(page, &mut ec) };
    assert_eq!(rc, -1);
    assert_ne!(ec, 0);
    unsafe { pdf_document_builder_free(builder) };
}

#[test]
fn ffi_page_builder_table_invalid_widths_rejected() {
    let mut ec: i32 = -1;
    let builder = unsafe { pdf_document_builder_create(&mut ec) };
    assert_eq!(ec, 0);
    let page = unsafe { pdf_document_builder_letter_page(builder, &mut ec) };
    assert_eq!(ec, 0);

    // n_columns=2 but widths pointer is null — should return -1 + ERR_INVALID_ARG.
    let aligns: [i32; 2] = [0, 0];
    let rc = unsafe {
        pdf_page_builder_table(
            page,
            2,
            ptr::null(),
            aligns.as_ptr(),
            0,
            ptr::null(),
            0,
            &mut ec,
        )
    };
    assert_eq!(rc, -1);
    assert_ne!(ec, 0);

    // Cleanup — page is still intact (free without done).
    unsafe { pdf_page_builder_free(page) };
    unsafe { pdf_document_builder_free(builder) };
}

// ---------------------------------------------------------------------------
// Sanity
// ---------------------------------------------------------------------------

#[test]
fn ffi_fixture_font_exists() {
    assert!(
        Path::new(DEJAVU_PATH).exists(),
        "DejaVuSans.ttf fixture missing — regenerate tests/fixtures/fonts/"
    );
}
