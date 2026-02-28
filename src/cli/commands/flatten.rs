use crate::editor::{DocumentEditor, EditableDocument, SaveOptions};
use std::path::Path;

pub fn run(
    file: &Path,
    forms: bool,
    annotations: bool,
    output: Option<&Path>,
    password: Option<&str>,
) -> crate::Result<()> {
    let _ = password;
    let mut editor = DocumentEditor::open(file)?;

    // Default to annotations if neither flag is set
    let do_annotations = annotations || !forms;
    let do_forms = forms;

    if do_annotations {
        editor.flatten_all_annotations()?;
        eprintln!("Flattened annotations");
    }

    if do_forms {
        editor.flatten_forms()?;
        eprintln!("Flattened form fields");
    }

    let out_path = output.map(|p| p.to_path_buf()).unwrap_or_else(|| {
        let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
        Path::new(&format!("{stem}_flattened.pdf")).to_path_buf()
    });

    editor.save_with_options(&out_path, SaveOptions {
        compress: true,
        garbage_collect: true,
        ..Default::default()
    })?;

    eprintln!("Saved to {}", out_path.display());

    Ok(())
}
