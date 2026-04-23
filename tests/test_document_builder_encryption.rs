//! Regression tests for issue #386 — `DocumentBuilder` encryption.
//!
//! v0.3.37 had `Pdf::save_encrypted` but it only worked for PDFs
//! *opened* via `DocumentEditor` (see
//! `src/api/pdf_builder.rs::save_encrypted` — it explicitly errors
//! "Encryption is only supported for opened PDFs" otherwise). Users
//! building PDFs programmatically through `DocumentBuilder::save` had
//! no way to produce an encrypted output.
//!
//! v0.3.38 adds `save_encrypted`, `save_with_encryption`, and
//! `to_bytes_encrypted` / `to_bytes_with_encryption` on
//! `DocumentBuilder`. Each routes the built bytes through
//! `DocumentEditor::from_bytes` → `save_with_options`, reusing the
//! tested production encryption pipeline.

use pdf_oxide::editor::{EncryptionAlgorithm, EncryptionConfig, Permissions};
use pdf_oxide::writer::{DocumentBuilder, DocumentMetadata, PageSize};
use std::fs;
use tempfile::tempdir;

fn make_builder(body: &str) -> DocumentBuilder {
    let mut builder =
        DocumentBuilder::new().metadata(DocumentMetadata::new().title("enc test").author("test"));
    {
        let page = builder.page(PageSize::Letter);
        page.at(72.0, 720.0).text(body).done();
    }
    builder
}

/// Default `save_encrypted` uses AES-256 (`/V 5 /R 6`) and writes the
/// expected Standard-security-handler dictionary entries.
#[test]
fn save_encrypted_produces_aes256_encrypt_dict() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("out.pdf");

    make_builder("confidential content for #386")
        .save_encrypted(&path, "userpw", "ownerpw")
        .expect("save_encrypted should succeed");

    assert!(path.exists());
    let raw = fs::read(&path).unwrap();
    let text = String::from_utf8_lossy(&raw);

    assert!(text.contains("/Encrypt"), "missing /Encrypt dict");
    assert!(text.contains("/Filter /Standard"), "missing /Filter /Standard");
    assert!(text.contains("/V 5"), "expected /V 5 (AES-256) — got no match");
    assert!(text.contains("/R 6"), "expected /R 6 (AES-256 revision)");
    assert!(text.contains("/O "), "missing /O (owner hash)");
    assert!(text.contains("/U "), "missing /U (user hash)");
    assert!(text.contains("/P "), "missing /P (permissions)");
}

/// `to_bytes_encrypted` returns the encrypted PDF as a byte vector.
/// Must match what `save_encrypted` writes to disk for the same input
/// (modulo timestamp-derived key material — the encryption dict itself
/// is a function of password + seed and will vary, but the outer
/// structural markers must be present).
#[test]
fn to_bytes_encrypted_includes_encrypt_dict() {
    let bytes = make_builder("in-memory encrypted build")
        .to_bytes_encrypted("user", "owner")
        .expect("to_bytes_encrypted should succeed");

    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("/Encrypt"), "bytes should include /Encrypt dict");
    assert!(text.contains("/V 5"), "bytes should use AES-256 by default");
}

/// `save_with_encryption` honours a custom algorithm choice.
#[test]
fn save_with_encryption_respects_aes128_config() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("out_aes128.pdf");

    let config = EncryptionConfig::new("u", "o")
        .with_algorithm(EncryptionAlgorithm::Aes128)
        .with_permissions(Permissions::all());

    make_builder("AES-128 test")
        .save_with_encryption(&path, config)
        .expect("save_with_encryption should succeed");

    let text = String::from_utf8_lossy(&fs::read(&path).unwrap()).to_string();
    assert!(text.contains("/Encrypt"), "missing /Encrypt dict");
    assert!(text.contains("/V 4"), "expected /V 4 (AES-128) — got no match in dict",);
}

/// Restricted permissions propagate into the `/P` permission bits.
/// `Permissions::read_only()` turns off print/modify/copy bits; we
/// check the resulting bits decoded from the `/P` integer.
#[test]
fn save_with_encryption_respects_restricted_permissions() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("out_readonly.pdf");

    let config = EncryptionConfig::new("u", "o")
        .with_algorithm(EncryptionAlgorithm::Aes256)
        .with_permissions(Permissions::read_only());

    make_builder("read-only PDF")
        .save_with_encryption(&path, config)
        .expect("save_with_encryption should succeed");

    let raw = fs::read(&path).unwrap();
    let text = String::from_utf8_lossy(&raw);
    assert!(text.contains("/Encrypt"), "missing /Encrypt dict");

    // Extract the `/P <signed-int>` from the encrypt dict. The value
    // sits between `/P ` and the next whitespace.
    let p_idx = text.find("/P ").expect("dict has /P entry");
    let tail = &text[p_idx + 3..];
    let end = tail
        .find(|c: char| c.is_whitespace() || c == '/')
        .expect("/P value has a terminator");
    let p_value: i32 = tail[..end].parse().expect("/P value should be an integer");

    // ISO 32000-1 Table 22: bits 3..=5 are print / modify / copy.
    // Bit 3 (print) in read-only should be 0. Bits are 1-based; Rust
    // shift is 0-based, so bit 3 = (1 << 2), bit 4 = (1 << 3), etc.
    let bit = |n: u32| (p_value >> (n - 1)) & 1;
    assert_eq!(bit(3), 0, "print bit should be clear for read-only (P={p_value})");
    assert_eq!(bit(4), 0, "modify bit should be clear for read-only (P={p_value})");
    assert_eq!(bit(5), 0, "copy bit should be clear for read-only (P={p_value})");
}

/// Encrypting a document that also embeds a custom font exercises both
/// v0.3.38 changes end to end: #385 (font subsetting) produces the
/// bytes, #386 (encryption) wraps them. This guards against any
/// layering bug where the encryption path re-parses the build output
/// and fails to handle the new content-stream ops.
#[test]
fn save_encrypted_works_with_embedded_font_subsetting() {
    use pdf_oxide::writer::EmbeddedFont;
    use std::path::Path;

    let dir = tempdir().unwrap();
    let path = dir.path().join("out_enc_embedded.pdf");

    let font = EmbeddedFont::from_file(Path::new("tests/fixtures/fonts/DejaVuSans.ttf"))
        .expect("DejaVuSans.ttf fixture available");
    let mut builder = DocumentBuilder::new()
        .metadata(DocumentMetadata::new().title("enc+subset"))
        .register_embedded_font("DejaVu", font);
    builder
        .a4_page()
        .font("DejaVu", 12.0)
        .at(72.0, 720.0)
        .text("Привет and Hello")
        .done();

    builder
        .save_encrypted(&path, "userpw", "ownerpw")
        .expect("save_encrypted with embedded font should succeed");

    let bytes = fs::read(&path).unwrap();
    let text = String::from_utf8_lossy(&bytes);

    // Both features present: encryption + subset tag.
    assert!(text.contains("/Encrypt"), "missing /Encrypt dict");
    assert!(text.contains("/V 5"), "missing /V 5 for AES-256");
    let has_subset_prefix = bytes
        .windows(8)
        .any(|w| w[0] == b'/' && w[1..7].iter().all(|&b| b.is_ascii_uppercase()) && w[7] == b'+');
    assert!(has_subset_prefix, "missing /XXXXXX+ subset-tag prefix");

    // And the encrypted PDF should still be much smaller than the
    // original face: encryption overhead doesn't re-embed the full
    // font bytes on top of the subset.
    let face_bytes = std::fs::metadata("tests/fixtures/fonts/DejaVuSans.ttf")
        .unwrap()
        .len() as usize;
    assert!(
        bytes.len() * 5 < face_bytes,
        "encrypted PDF ({} bytes) is not meaningfully smaller than the original face ({} bytes) — \
         subsetting likely not applied when encryption is on",
        bytes.len(),
        face_bytes,
    );
}
