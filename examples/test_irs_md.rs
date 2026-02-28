use pdf_oxide::converters::ConversionOptions;
use pdf_oxide::editor::form_fields::FormFieldValue;
use pdf_oxide::editor::{DocumentEditor, EditableDocument, SaveOptions};
use pdf_oxide::extractors::forms::FormExtractor;
use pdf_oxide::PdfDocument;

fn test_irs_form(path: &str, label: &str) {
    println!("\n{}", "=".repeat(60));
    println!("  Testing: {} ({})", label, path);
    println!("{}\n", "=".repeat(60));

    // --- Unfilled: list fields, extract_text, to_markdown ---
    println!("--- Unfilled: FormExtractor ---");
    let mut doc = match PdfDocument::open(path) {
        Ok(d) => d,
        Err(e) => {
            println!("  SKIP: Cannot open {}: {}", path, e);
            return;
        },
    };
    let page_count = doc.page_count().unwrap_or(0);
    println!("  Page count: {}", page_count);

    let fields = FormExtractor::extract_fields(&mut doc).unwrap_or_default();
    println!("  Total fields: {}", fields.len());
    if fields.is_empty() {
        println!("  WARN: No form fields found — skipping fill test");
        return;
    }

    // Show first 10 field names
    for (i, f) in fields.iter().take(10).enumerate() {
        println!("    [{}] {} ({:?}) = {:?}", i, f.full_name, f.field_type, f.value);
    }
    if fields.len() > 10 {
        println!("    ... and {} more", fields.len() - 10);
    }

    // Count checkboxes in extract_text page 0
    println!("\n--- Unfilled: extract_text page 0 ---");
    let text_p0 = doc
        .extract_text(0)
        .unwrap_or_else(|e| format!("ERROR: {}", e));
    let unchecked = text_p0.matches("[ ]").count();
    let checked = text_p0.matches("[x]").count();
    println!("  [ ] count: {}  [x] count: {}", unchecked, checked);

    // Also check page 1 if exists
    if page_count > 1 {
        let text_p1 = doc
            .extract_text(1)
            .unwrap_or_else(|e| format!("ERROR: {}", e));
        let u1 = text_p1.matches("[ ]").count();
        let c1 = text_p1.matches("[x]").count();
        println!("  Page 1: [ ] count: {}  [x] count: {}", u1, c1);
    }

    // to_markdown page 0
    println!("\n--- Unfilled: to_markdown page 0 ---");
    let opts = ConversionOptions {
        include_form_fields: true,
        ..Default::default()
    };
    let md = doc
        .to_markdown(0, &opts)
        .unwrap_or_else(|e| format!("ERROR: {}", e));
    let md_u = md.matches("[ ]").count();
    let md_c = md.matches("[x]").count();
    println!("  [ ] count: {}  [x] count: {}", md_u, md_c);

    // --- Fill form, save incremental, reopen & verify ---
    println!("\n--- Fill form & incremental save ---");
    let mut editor = match DocumentEditor::open(path) {
        Ok(e) => e,
        Err(e) => {
            println!("  SKIP: Cannot open editor: {}", e);
            return;
        },
    };

    // Pick a few text fields and a checkbox to fill
    let text_fields: Vec<&_> = fields
        .iter()
        .filter(|f| matches!(f.field_type, pdf_oxide::extractors::forms::FieldType::Text))
        .take(3)
        .collect();
    let checkbox_fields: Vec<&_> = fields
        .iter()
        .filter(|f| matches!(f.field_type, pdf_oxide::extractors::forms::FieldType::Button))
        .take(1)
        .collect();

    let mut fills: Vec<(String, FormFieldValue)> = Vec::new();
    let test_values = ["TEST-VALUE-1", "TEST-VALUE-2", "TEST-VALUE-3"];
    for (i, f) in text_fields.iter().enumerate() {
        let val = FormFieldValue::Text(test_values[i].into());
        match editor.set_form_field_value(&f.full_name, val.clone()) {
            Ok(()) => {
                println!(
                    "  Set {} = {}",
                    f.full_name.rsplit('.').next().unwrap_or(&f.full_name),
                    test_values[i]
                );
                fills.push((f.full_name.clone(), val));
            },
            Err(e) => println!(
                "  FAIL set {}: {}",
                f.full_name.rsplit('.').next().unwrap_or(&f.full_name),
                e
            ),
        }
    }
    for f in &checkbox_fields {
        let val = FormFieldValue::Boolean(true);
        match editor.set_form_field_value(&f.full_name, val.clone()) {
            Ok(()) => {
                println!(
                    "  Set {} = true (checkbox)",
                    f.full_name.rsplit('.').next().unwrap_or(&f.full_name)
                );
                fills.push((f.full_name.clone(), val));
            },
            Err(e) => println!(
                "  FAIL set {}: {}",
                f.full_name.rsplit('.').next().unwrap_or(&f.full_name),
                e
            ),
        }
    }

    if fills.is_empty() {
        println!("  WARN: No fields were set — skipping save test");
        return;
    }

    let tmp_path = format!("/tmp/filled_{}.pdf", label.replace(' ', "_").to_lowercase());
    match editor.save_with_options(&tmp_path, SaveOptions::incremental()) {
        Ok(()) => println!("  Saved to {}", tmp_path),
        Err(e) => {
            println!("  FAIL save: {}", e);
            return;
        },
    }

    // Reopen and verify
    let mut filled = match PdfDocument::open(&tmp_path) {
        Ok(d) => d,
        Err(e) => {
            println!("  FAIL reopen: {}", e);
            return;
        },
    };

    // Verify via FormExtractor
    println!("\n--- Verify: FormExtractor after reopen ---");
    match FormExtractor::extract_fields(&mut filled) {
        Ok(refields) => {
            let mut pass = 0;
            let mut fail = 0;
            for (name, expected) in &fills {
                let short = name.rsplit('.').next().unwrap_or(name);
                if let Some(f) = refields.iter().find(|f| f.full_name == *name) {
                    let ok = match (expected, &f.value) {
                        (
                            FormFieldValue::Text(e),
                            pdf_oxide::extractors::forms::FieldValue::Text(v),
                        ) => e == v,
                        (
                            FormFieldValue::Boolean(true),
                            pdf_oxide::extractors::forms::FieldValue::Name(n),
                        ) => n == "Yes",
                        (
                            FormFieldValue::Boolean(true),
                            pdf_oxide::extractors::forms::FieldValue::Boolean(b),
                        ) => *b,
                        _ => false,
                    };
                    if ok {
                        println!("  ✓ {} = {:?}", short, f.value);
                        pass += 1;
                    } else {
                        println!("  ✗ {} expected {:?}, got {:?}", short, expected, f.value);
                        fail += 1;
                    }
                } else {
                    println!("  ✗ {} NOT FOUND", short);
                    fail += 1;
                }
            }
            println!("  Result: {}/{} passed", pass, pass + fail);
        },
        Err(e) => println!("  FormExtractor FAILED: {}", e),
    }

    // Verify via extract_text — check ALL pages to find the right one
    let filled_page_count = filled.page_count().unwrap_or(0);
    println!("\n--- Verify: extract_text across all {} pages ---", filled_page_count);
    let mut all_text = String::new();
    for p in 0..filled_page_count {
        if let Ok(t) = filled.extract_text(p) {
            all_text.push_str(&t);
            all_text.push('\n');
        }
    }
    for (name, expected) in &fills {
        let short = name.rsplit('.').next().unwrap_or(name);
        match expected {
            FormFieldValue::Text(t) => {
                if all_text.contains(t.as_str()) {
                    println!("  ✓ '{}' found in extract_text ({})", t, short);
                } else {
                    println!("  ✗ '{}' NOT found in extract_text ({})", t, short);
                }
            },
            FormFieldValue::Boolean(true) => {
                if all_text.contains("[x]") {
                    println!("  ✓ [x] found in extract_text ({})", short);
                } else {
                    println!("  ✗ [x] NOT found in extract_text ({})", short);
                }
            },
            _ => {},
        }
    }

    // Verify via to_markdown — check ALL pages
    println!("\n--- Verify: to_markdown across all {} pages ---", filled_page_count);
    let mut all_md = String::new();
    for p in 0..filled_page_count {
        if let Ok(m) = filled.to_markdown(p, &opts) {
            all_md.push_str(&m);
            all_md.push('\n');
        }
    }
    for (name, expected) in &fills {
        let short = name.rsplit('.').next().unwrap_or(name);
        match expected {
            FormFieldValue::Text(t) => {
                if all_md.contains(t.as_str()) {
                    println!("  ✓ '{}' found in to_markdown ({})", t, short);
                } else {
                    println!("  ✗ '{}' NOT found in to_markdown ({})", t, short);
                }
            },
            FormFieldValue::Boolean(true) => {
                if all_md.contains("[x]") {
                    println!("  ✓ [x] found in to_markdown ({})", short);
                } else {
                    println!("  ✗ [x] NOT found in to_markdown ({})", short);
                }
            },
            _ => {},
        }
    }

    // Verify to_markdown with include_form_fields=false — check ALL pages
    println!("\n--- Verify: to_markdown include_form_fields=false ---");
    let opts_no = ConversionOptions {
        include_form_fields: false,
        ..Default::default()
    };
    let mut md_no_all = String::new();
    for p in 0..filled_page_count {
        if let Ok(m) = filled.to_markdown(p, &opts_no) {
            md_no_all.push_str(&m);
            md_no_all.push('\n');
        }
    }
    let leaked = fills.iter().any(|(_, v)| {
        if let FormFieldValue::Text(t) = v {
            md_no_all.contains(t.as_str())
        } else {
            false
        }
    });
    if !leaked {
        println!("  PASS: No filled values leak with include_form_fields=false");
    } else {
        println!("  FAIL: Filled values leaked with include_form_fields=false");
    }
}

fn main() {
    let irs_dir = "/home/yfedoseev/projects/pdf_oxide_tests/irs";

    test_irs_form(&format!("{}/fw2.pdf", irs_dir), "W-2 (fw2)");
    test_irs_form(&format!("{}/fw2_2024.pdf", irs_dir), "W-2 2024");
    test_irs_form(&format!("{}/w2upload.pdf", irs_dir), "W-2 Upload");
}
