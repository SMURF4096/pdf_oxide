pub mod bookmarks;
pub mod compress;
pub mod create;
pub mod crop;
pub mod decrypt;
pub mod delete;
pub mod encrypt;
pub mod flatten;
pub mod forms;
pub mod html;
pub mod images;
pub mod info;
pub mod markdown;
pub mod merge;
pub mod metadata;
pub mod paths;
pub mod render;
pub mod reorder;
pub mod rotate;
pub mod search;
pub mod split;
pub mod text;
pub mod watermark;

use pdf_oxide::PdfDocument;
use std::path::Path;

/// Open a PDF, optionally authenticating with a password.
pub fn open_doc(path: &Path, password: Option<&str>) -> pdf_oxide::Result<PdfDocument> {
    let doc = PdfDocument::open(path)?;
    if let Some(pw) = password {
        doc.authenticate(pw.as_bytes())?;
    }
    Ok(doc)
}

/// Get page indices to process: either from --pages flag or all pages.
pub fn resolve_pages(pages_arg: Option<&str>, page_count: usize) -> pdf_oxide::Result<Vec<usize>> {
    match pages_arg {
        Some(ranges) => {
            super::pages::parse_page_ranges(ranges).map_err(pdf_oxide::Error::InvalidOperation)
        },
        None => Ok((0..page_count).collect()),
    }
}

/// Write output to file or stdout.
pub fn write_output(content: &str, output: Option<&Path>) -> pdf_oxide::Result<()> {
    use std::io::Write;
    match output {
        Some(path) => Ok(std::fs::write(path, content)?),
        None => {
            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            handle.write_all(content.as_bytes())?;
            // Ensure trailing newline for terminal
            if !content.ends_with('\n') {
                handle.write_all(b"\n")?;
            }
            Ok(())
        },
    }
}
