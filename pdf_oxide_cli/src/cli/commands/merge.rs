use pdf_oxide::editor::{DocumentEditor, EditableDocument};
use std::path::Path;

pub fn run(files: &[std::path::PathBuf], output: Option<&Path>) -> pdf_oxide::Result<()> {
    if files.len() < 2 {
        return Err(pdf_oxide::Error::InvalidOperation(
            "Merge requires at least 2 PDF files".to_string(),
        ));
    }

    let mut editor = DocumentEditor::open(&files[0])?;

    for source in &files[1..] {
        let pages_added = editor.merge_from(source)?;
        eprintln!("Merged {} pages from {}", pages_added, source.display());
    }

    let default_out;
    let out_path = match output {
        Some(p) => p,
        None => {
            default_out = super::output_dir_beside(&files[0]).join("merged.pdf");
            &default_out
        },
    };
    editor.save(out_path)?;
    eprintln!("Saved to {}", out_path.display());

    Ok(())
}
