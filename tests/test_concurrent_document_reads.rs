//! Thread-safety test: multiple threads open and extract text from the same
//! PDF bytes concurrently (#398).
#![allow(clippy::missing_safety_doc)]
#![allow(unused_unsafe)]

use std::ffi::CString;
use pdf_oxide::ffi::*;

fn cstring(s: &str) -> CString {
    CString::new(s).unwrap()
}

#[test]
fn concurrent_document_reads_no_panic() {
    use std::sync::Arc;

    let mut ec: i32 = -1;
    let builder = unsafe { pdf_document_builder_create(&mut ec) };
    assert_eq!(ec, 0);
    let page = unsafe { pdf_document_builder_letter_page(builder, &mut ec) };
    assert_eq!(ec, 0);
    assert_eq!(
        unsafe { pdf_page_builder_font(page, cstring("Helvetica").as_ptr(), 12.0, &mut ec) },
        0
    );
    assert_eq!(unsafe { pdf_page_builder_at(page, 72.0, 720.0, &mut ec) }, 0);
    let t = cstring("Concurrent read test");
    assert_eq!(unsafe { pdf_page_builder_text(page, t.as_ptr(), &mut ec) }, 0);
    assert_eq!(unsafe { pdf_page_builder_done(page, &mut ec) }, 0);
    let mut pdf_len: usize = 0;
    let pdf_ptr = unsafe { pdf_document_builder_build(builder, &mut pdf_len, &mut ec) };
    assert_eq!(ec, 0);
    let pdf_bytes: Arc<Vec<u8>> =
        Arc::new(unsafe { std::slice::from_raw_parts(pdf_ptr as *const u8, pdf_len) }.to_vec());
    unsafe { free_bytes(pdf_ptr) };
    unsafe { pdf_document_builder_free(builder) };

    let handles: Vec<_> = (0..8)
        .map(|_| {
            let bytes = Arc::clone(&pdf_bytes);
            std::thread::spawn(move || {
                let mut ec: i32 = -1;
                let doc =
                    unsafe { pdf_document_open_from_bytes(bytes.as_ptr(), bytes.len(), &mut ec) };
                assert_eq!(ec, 0, "open failed in thread");
                let text_ptr = unsafe { pdf_document_extract_text(doc, -1, &mut ec) };
                assert_eq!(ec, 0, "extract_text failed in thread");
                let text = unsafe { std::ffi::CStr::from_ptr(text_ptr) }
                    .to_string_lossy()
                    .to_string();
                unsafe { free_string(text_ptr) };
                unsafe { pdf_document_free(doc) };
                assert!(
                    text.contains("Concurrent"),
                    "unexpected text content: {text:.100}"
                );
            })
        })
        .collect();

    for h in handles {
        h.join().expect("thread panicked");
    }
}
