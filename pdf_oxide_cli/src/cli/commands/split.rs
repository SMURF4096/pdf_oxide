use pdf_oxide::editor::{DocumentEditor, EditableDocument, SaveOptions};
use std::path::Path;

pub fn run(
    file: &Path,
    pages: Option<&str>,
    output: Option<&Path>,
    password: Option<&str>,
) -> pdf_oxide::Result<()> {
    let doc = super::open_doc(file, password)?;
    let page_count = doc.page_count()?;
    drop(doc);

    let page_indices = super::resolve_pages(pages, page_count)?;

    let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("page");

    let default_dir;
    let out_dir = match output {
        Some(p) => p,
        None => {
            default_dir = super::output_dir_beside(file);
            &default_dir
        },
    };

    for &page_idx in &page_indices {
        let mut editor = DocumentEditor::open(file)?;

        // Remove pages from end to start to keep indices stable
        for i in (0..page_count).rev() {
            if i != page_idx {
                editor.remove_page(i)?;
            }
        }

        let out_path = out_dir.join(format!("{}_page_{}.pdf", stem, page_idx + 1));
        editor.save_with_options(
            &out_path,
            SaveOptions {
                compress: true,
                garbage_collect: true,
                ..Default::default()
            },
        )?;
        eprintln!("Saved page {} to {}", page_idx + 1, out_path.display());
    }

    Ok(())
}
