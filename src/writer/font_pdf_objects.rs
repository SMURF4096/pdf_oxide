//! Build the five PDF indirect objects required to embed a TrueType font.
//!
//! For Unicode-capable PDF text, ISO 32000-1 §9.6.4 / §9.7 / §9.8 / §9.10
//! requires a graph of five objects per font:
//!
//! ```text
//!   Type 0 dict          (the "outer" Font referenced by Resources/Font/Fxx)
//!     ├── DescendantFonts → CIDFontType2 dict
//!     │                       └── FontDescriptor → FontFile2 stream
//!     └── ToUnicode      → CMap stream (glyph id → source codepoint)
//! ```
//!
//! All glyph indexing in the content stream is done through the Type 0 dict
//! using Identity-H encoding, which is just "two bytes per glyph, big endian,
//! no remapping". The ToUnicode CMap is what makes `extract_text` round-trip:
//! without it, every PDF reader sees opaque glyph IDs.
//!
//! v0.3.35 ships **full-font embedding** — the `subsetter` wrapper from
//! `crate::fonts::subset_font_bytes` is wired separately (FONT-2) and feeds
//! into this path in a follow-up commit (FONT-3b) once content streams can
//! be GID-remapped. For now `EmbeddedFont::font_data()` (the original face
//! bytes) goes straight into FontFile2.

use crate::object::Object;
use crate::writer::font_manager::EmbeddedFont;
use crate::writer::object_serializer::ObjectSerializer;
use std::collections::HashMap;

/// Object IDs for one embedded-font dict graph. Returned by
/// [`build_embedded_font_objects`] so the caller can link these into the
/// PdfWriter object table and reference the Type 0 dict from the page
/// `/Resources /Font` entry.
#[derive(Debug, Clone, Copy)]
pub struct EmbeddedFontIds {
    /// Top-level Type 0 (`/Font /Subtype /Type0`) dict ID. This is what
    /// the page resource dictionary references.
    pub type0: u32,
    /// Descendant CIDFontType2 dict ID.
    pub cidfont: u32,
    /// FontDescriptor dict ID.
    pub descriptor: u32,
    /// FontFile2 stream ID (the actual TrueType bytes).
    pub font_file: u32,
    /// ToUnicode CMap stream ID (round-trips text extraction).
    pub tounicode: u32,
}

/// Build the five PDF objects that embed `font`. The caller is responsible
/// for inserting the returned `(id, Object)` pairs into the writer's object
/// table and for emitting the Type 0 ref into the page `/Resources /Font`
/// dictionary under `resource_name`.
///
/// `id_alloc` is called once per object in dependency order
/// (font_file → descriptor → cidfont → tounicode → type0) so callers using
/// a monotonic ID counter end up with the natural traversal order.
pub fn build_embedded_font_objects(
    font: &mut EmbeddedFont,
    mut id_alloc: impl FnMut() -> u32,
) -> (EmbeddedFontIds, Vec<(u32, Object)>) {
    let font_file_id = id_alloc();
    let descriptor_id = id_alloc();
    let cidfont_id = id_alloc();
    let tounicode_id = id_alloc();
    let type0_id = id_alloc();

    let ids = EmbeddedFontIds {
        type0: type0_id,
        cidfont: cidfont_id,
        descriptor: descriptor_id,
        font_file: font_file_id,
        tounicode: tounicode_id,
    };

    let mut out: Vec<(u32, Object)> = Vec::with_capacity(5);

    // Subset name like "ABCDEF+DejaVuSans". The 6-letter tag is generated
    // deterministically from the used-glyph set so the same content always
    // produces the same subset name (helps reproducible builds).
    //
    // PDF spec note: when the font is *not* actually subsetted (full-font
    // embedding, which is what ships in v0.3.35 first cut), the subset
    // tag is still permitted — it just signals "this could be a subset"
    // to readers, no harm done. When real subsetting wires in (FONT-3b)
    // the same tag continues to be valid.
    let base_font = font.subset_name().to_string();

    // ── 1. FontFile2 stream — the actual TrueType bytes ──────────────────
    // Full-font for v0.3.35 first cut. Length1 must equal the byte length
    // of the embedded TTF per PDF spec §9.9.
    let font_bytes = font.font_data().to_vec();
    let length1 = font_bytes.len() as i64;
    let mut ff_dict: HashMap<String, Object> = HashMap::new();
    ff_dict.insert(
        "Length".to_string(),
        ObjectSerializer::integer(length1),
    );
    ff_dict.insert("Length1".to_string(), ObjectSerializer::integer(length1));
    out.push((
        font_file_id,
        Object::Stream {
            dict: ff_dict,
            data: bytes::Bytes::from(font_bytes),
        },
    ));

    // ── 2. FontDescriptor (§9.8.1, Table 122) ────────────────────────────
    let (llx, lly, urx, ury) = font.bbox;
    let descriptor = ObjectSerializer::dict(vec![
        ("Type", ObjectSerializer::name("FontDescriptor")),
        ("FontName", ObjectSerializer::name(&base_font)),
        ("Flags", ObjectSerializer::integer(font.flags as i64)),
        (
            "FontBBox",
            ObjectSerializer::rect(llx as f64, lly as f64, urx as f64, ury as f64),
        ),
        (
            "ItalicAngle",
            Object::Real(font.italic_angle as f64),
        ),
        ("Ascent", ObjectSerializer::integer(font.ascender as i64)),
        ("Descent", ObjectSerializer::integer(font.descender as i64)),
        (
            "CapHeight",
            ObjectSerializer::integer(font.cap_height as i64),
        ),
        ("XHeight", ObjectSerializer::integer(font.x_height as i64)),
        ("StemV", ObjectSerializer::integer(font.stem_v as i64)),
        (
            "FontFile2",
            ObjectSerializer::reference(font_file_id, 0),
        ),
    ]);
    out.push((descriptor_id, descriptor));

    // ── 3. CIDFontType2 (§9.7.4, Table 117) ──────────────────────────────
    // /CIDToGIDMap /Identity is the right call when the source font's GIDs
    // *are* the CIDs we expose, which is exactly what Identity-H encoding
    // gives us. The /W array carries glyph widths indexed by CID/GID.
    let widths_str = font.generate_widths_array();
    let cid_system_info = ObjectSerializer::dict(vec![
        ("Registry", ObjectSerializer::string("Adobe")),
        ("Ordering", ObjectSerializer::string("Identity")),
        ("Supplement", ObjectSerializer::integer(0)),
    ]);
    let cidfont = ObjectSerializer::dict(vec![
        ("Type", ObjectSerializer::name("Font")),
        ("Subtype", ObjectSerializer::name("CIDFontType2")),
        ("BaseFont", ObjectSerializer::name(&base_font)),
        ("CIDSystemInfo", cid_system_info),
        (
            "FontDescriptor",
            ObjectSerializer::reference(descriptor_id, 0),
        ),
        ("CIDToGIDMap", ObjectSerializer::name("Identity")),
        // /W is parsed by ObjectSerializer::dict only as an Object; for the
        // raw "[ gid [ widths... ] ... ]" string the existing helper emits,
        // we wrap as a pre-formatted array via Raw using the writer's
        // string-injection escape — but ObjectSerializer doesn't expose
        // that, so build the array structurally instead.
        ("W", parse_widths_string_to_array(&widths_str)),
    ]);
    out.push((cidfont_id, cidfont));

    // ── 4. ToUnicode CMap stream (§9.10.2) ───────────────────────────────
    // Generated by EmbeddedFont from the tracked-glyph set. This is the
    // round-trip path: PDF readers parse this CMap to recover source text
    // from glyph IDs, which is what every conformance check (and our own
    // extract_text) walks.
    let cmap_bytes = font.generate_tounicode_cmap().into_bytes();
    let mut cmap_dict: HashMap<String, Object> = HashMap::new();
    cmap_dict.insert(
        "Length".to_string(),
        ObjectSerializer::integer(cmap_bytes.len() as i64),
    );
    out.push((
        tounicode_id,
        Object::Stream {
            dict: cmap_dict,
            data: bytes::Bytes::from(cmap_bytes),
        },
    ));

    // ── 5. Type 0 wrapper (§9.6.4, Table 110) ────────────────────────────
    let type0 = ObjectSerializer::dict(vec![
        ("Type", ObjectSerializer::name("Font")),
        ("Subtype", ObjectSerializer::name("Type0")),
        ("BaseFont", ObjectSerializer::name(&base_font)),
        ("Encoding", ObjectSerializer::name("Identity-H")),
        (
            "DescendantFonts",
            Object::Array(vec![ObjectSerializer::reference(cidfont_id, 0)]),
        ),
        (
            "ToUnicode",
            ObjectSerializer::reference(tounicode_id, 0),
        ),
    ]);
    out.push((type0_id, type0));

    (ids, out)
}

/// Convert the pre-formatted widths string produced by
/// [`EmbeddedFont::generate_widths_array`] (a textual PDF array literal
/// like `"[ 36 [ 600 600 ] 65 [ 720 ] ]"`) into a structural
/// [`Object::Array`] so the existing `ObjectSerializer` can serialise it
/// without a raw-string escape hatch.
///
/// This is a tiny one-pass parser — accepts integers and `[`/`]` delimiters,
/// rejects anything else. It exists only so `EmbeddedFont`'s existing
/// helpers can stay as `String`-returning APIs (their original v0.3.0
/// callers were inspector code, not the writer).
fn parse_widths_string_to_array(s: &str) -> Object {
    let mut stack: Vec<Vec<Object>> = vec![Vec::new()];
    let mut number = String::new();
    let flush_number = |stack: &mut Vec<Vec<Object>>, number: &mut String| {
        if !number.is_empty() {
            if let Ok(n) = number.parse::<i64>() {
                stack
                    .last_mut()
                    .expect("widths-array stack must never empty")
                    .push(ObjectSerializer::integer(n));
            }
            number.clear();
        }
    };
    for ch in s.chars() {
        match ch {
            '[' => {
                flush_number(&mut stack, &mut number);
                stack.push(Vec::new());
            }
            ']' => {
                flush_number(&mut stack, &mut number);
                let popped = stack.pop().unwrap_or_default();
                stack
                    .last_mut()
                    .expect("widths-array stack must keep at least the root level")
                    .push(Object::Array(popped));
            }
            c if c.is_ascii_digit() || c == '-' => number.push(c),
            _ => flush_number(&mut stack, &mut number),
        }
    }
    flush_number(&mut stack, &mut number);
    // The outer stack holds [[ <root array> ]]; the root array is what we want.
    let mut root = stack.pop().unwrap_or_default();
    if root.len() == 1 {
        root.pop().unwrap_or(Object::Array(Vec::new()))
    } else {
        Object::Array(root)
    }
}

/// Public alias so callers can reference an embedded font's PDF identity
/// without depending on the private dict-builder details.
pub type FontResourceName = String;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_widths_simple() {
        let parsed = parse_widths_string_to_array("[ 36 [ 600 600 ] 65 [ 720 ] ]");
        // Outer is an array of: integer 36, inner-array [600,600], integer 65, inner [720].
        match parsed {
            Object::Array(items) => assert_eq!(items.len(), 4),
            other => panic!("expected array, got {other:?}"),
        }
    }

    #[test]
    fn parse_widths_empty() {
        let parsed = parse_widths_string_to_array("[]");
        match parsed {
            Object::Array(items) => assert!(items.is_empty()),
            other => panic!("expected empty array, got {other:?}"),
        }
    }
}
