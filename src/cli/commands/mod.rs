pub mod text;
pub mod markdown;
pub mod html;
pub mod info;
pub mod merge;
pub mod split;
pub mod create;
pub mod compress;
pub mod encrypt;
pub mod decrypt;
pub mod search;
pub mod images;
pub mod rotate;
pub mod delete;
pub mod reorder;
pub mod metadata;
pub mod watermark;
pub mod bookmarks;
pub mod flatten;
pub mod crop;
pub mod forms;

use crate::PdfDocument;
use std::path::Path;

/// Open a PDF, optionally authenticating with a password.
pub fn open_doc(path: &Path, password: Option<&str>) -> crate::Result<PdfDocument> {
    let mut doc = PdfDocument::open(path)?;
    if let Some(pw) = password {
        doc.authenticate(pw.as_bytes())?;
    }
    Ok(doc)
}

/// Get page indices to process: either from --pages flag or all pages.
pub fn resolve_pages(
    pages_arg: Option<&str>,
    page_count: usize,
) -> crate::Result<Vec<usize>> {
    match pages_arg {
        Some(ranges) => super::pages::parse_page_ranges(ranges)
            .map_err(|e| crate::Error::InvalidOperation(e)),
        None => Ok((0..page_count).collect()),
    }
}

/// Write output to file or stdout.
pub fn write_output(content: &str, output: Option<&Path>) -> crate::Result<()> {
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
        }
    }
}
