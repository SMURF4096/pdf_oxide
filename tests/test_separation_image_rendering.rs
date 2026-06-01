//! Tests for raster image XObject routing in the separation-plate renderer
//! (ISO 32000-1 §8.9 image XObjects, §11.7.4 overprint placement).
//!
//! Spatial convention: 100×100 page rendered at 72 DPI = 100×100 pixel
//! plates. Images are placed with `50 0 0 50 25 25 cm` — a 50×50 user-space
//! square centred on the page, occupying image rows 25..75, cols 25..75.

#![cfg(feature = "rendering")]

use pdf_oxide::document::PdfDocument;
use pdf_oxide::rendering::{render_separations, SeparationPlate};

fn sample(plate: &SeparationPlate, x: u32, y: u32) -> u8 {
    plate.data[(y * plate.width + x) as usize]
}

fn plate<'a>(plates: &'a [SeparationPlate], name: &str) -> &'a SeparationPlate {
    plates
        .iter()
        .find(|p| p.ink_name == name)
        .unwrap_or_else(|| {
            panic!(
                "missing plate {name:?}; have {:?}",
                plates
                    .iter()
                    .map(|p| p.ink_name.as_str())
                    .collect::<Vec<_>>()
            )
        })
}

fn finalize_pdf(mut buf: Vec<u8>, offsets: Vec<usize>) -> Vec<u8> {
    let xref_offset = buf.len();
    buf.extend_from_slice(b"xref\n");
    buf.extend_from_slice(format!("0 {}\n", offsets.len() + 1).as_bytes());
    buf.extend_from_slice(b"0000000000 65535 f \n");
    for off in &offsets {
        buf.extend_from_slice(format!("{:010} 00000 n \n", off).as_bytes());
    }
    buf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            offsets.len() + 1,
            xref_offset
        )
        .as_bytes(),
    );
    buf
}

/// Build a single-page PDF where /Im1 is a `width × height` DeviceCMYK image
/// painted at the unit square via `50 0 0 50 25 25 cm`. `cmyk_samples` is
/// interleaved 8-bpc CMYK (W*H*4 bytes).
fn build_pdf_with_cmyk_image(cmyk_samples: &[u8], width: u32, height: u32) -> Vec<u8> {
    let content = b"q\n50 0 0 50 25 25 cm\n/Im1 Do\nQ\n";
    let mut buf = Vec::new();
    let mut offsets = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");

    offsets.push(buf.len());
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
           /Contents 4 0 R /Resources << /XObject << /Im1 5 0 R >> >> >>\nendobj\n",
    );
    offsets.push(buf.len());
    let hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(hdr.as_bytes());
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    let img_hdr = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width {w} /Height {h} \
         /ColorSpace /DeviceCMYK /BitsPerComponent 8 /Length {len} >>\nstream\n",
        w = width,
        h = height,
        len = cmyk_samples.len()
    );
    buf.extend_from_slice(img_hdr.as_bytes());
    buf.extend_from_slice(cmyk_samples);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    finalize_pdf(buf, offsets)
}

#[test]
fn cmyk_image_routes_channels_to_process_plates() {
    // 2×2 image:
    //   pixel 0 (top-left, image origin)     : Cyan = 255
    //   pixel 1 (top-right)                  : Magenta = 255
    //   pixel 2 (bottom-left)                : Yellow = 255
    //   pixel 3 (bottom-right)               : Black = 255
    // After the unit-square Y flip the image's top-left lands at the
    // top-left of the painted region in PDF user space (since the cm
    // matrix `50 0 0 50 25 25` translates the unit square to
    // (25, 25)–(75, 75)). The plate is rendered with PDF y flipped to
    // image-row order, so the C pixel lands at image-row 25..50, col 25..50;
    // K lands at image-row 50..75, col 50..75.
    let cmyk: Vec<u8> = vec![
        255, 0, 0, 0, // C
        0, 255, 0, 0, // M
        0, 0, 255, 0, // Y
        0, 0, 0, 255, // K
    ];
    let doc = PdfDocument::from_bytes(build_pdf_with_cmyk_image(&cmyk, 2, 2)).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let c = plate(&plates, "Cyan");
    let m = plate(&plates, "Magenta");
    let y = plate(&plates, "Yellow");
    let k = plate(&plates, "Black");

    // Sample inside each quadrant of the painted region (25..75 × 25..75).
    // Image top-left cell (Cyan) → plate top-left within the region.
    assert!(
        sample(c, 35, 35) > 200,
        "Cyan channel lands on Cyan plate (top-left quadrant); got {}",
        sample(c, 35, 35)
    );
    assert!(
        sample(m, 60, 35) > 200,
        "Magenta channel lands on Magenta plate (top-right quadrant); got {}",
        sample(m, 60, 35)
    );
    assert!(
        sample(y, 35, 60) > 200,
        "Yellow channel lands on Yellow plate (bottom-left quadrant); got {}",
        sample(y, 35, 60)
    );
    assert!(
        sample(k, 60, 60) > 200,
        "Black channel lands on Black plate (bottom-right quadrant); got {}",
        sample(k, 60, 60)
    );

    // Outside the image bbox, plates stay at zero.
    assert_eq!(sample(c, 5, 5), 0, "Cyan untouched outside image bbox");
    assert_eq!(sample(k, 5, 5), 0, "Black untouched outside image bbox");
}

#[test]
fn cmyk_image_inside_form_xobject_routes_to_plates() {
    // Page invokes a Form that contains the CMYK image — verify the recursion
    // through Operator::Do reaches the image branch.
    let content = b"/F1 Do\n";
    let form_content = b"q\n50 0 0 50 25 25 cm\n/Im1 Do\nQ\n";
    // 1×1 image with K = 255.
    let cmyk: Vec<u8> = vec![0, 0, 0, 255];

    let mut buf = Vec::new();
    let mut offsets = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
           /Contents 4 0 R /Resources << /XObject << /F1 5 0 R >> >> >>\nendobj\n",
    );
    offsets.push(buf.len());
    let hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(hdr.as_bytes());
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    let form_hdr = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] \
            /Resources << /XObject << /Im1 6 0 R >> >> /Length {} >>\nstream\n",
        form_content.len()
    );
    buf.extend_from_slice(form_hdr.as_bytes());
    buf.extend_from_slice(form_content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    let img_hdr = format!(
        "6 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 \
         /ColorSpace /DeviceCMYK /BitsPerComponent 8 /Length {} >>\nstream\n",
        cmyk.len()
    );
    buf.extend_from_slice(img_hdr.as_bytes());
    buf.extend_from_slice(&cmyk);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    let pdf = finalize_pdf(buf, offsets);

    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let k = plate(&plates, "Black");
    assert!(
        sample(k, 50, 50) > 200,
        "K channel of nested-form CMYK image reaches the Black plate; got {}",
        sample(k, 50, 50)
    );
}

#[test]
fn separation_image_routes_to_spot_plate() {
    // 1×1 Separation /Pantone-185 image at full tint.
    // /ColorSpace is declared at page-level as /CS1 → indirect ref to
    // [/Separation /Pantone-185 /DeviceCMYK <tint transform>].
    let content = b"q\n50 0 0 50 25 25 cm\n/Im1 Do\nQ\n";
    let samples: Vec<u8> = vec![255];

    let mut buf = Vec::new();
    let mut offsets = Vec::new();
    buf.extend_from_slice(b"%PDF-1.4\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] \
           /Contents 4 0 R \
           /Resources << /XObject << /Im1 5 0 R >> \
                        /ColorSpace << /CS1 6 0 R >> >> >>\nendobj\n",
    );
    offsets.push(buf.len());
    let hdr = format!("4 0 obj\n<< /Length {} >>\nstream\n", content.len());
    buf.extend_from_slice(hdr.as_bytes());
    buf.extend_from_slice(content);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    let img_hdr = format!(
        "5 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 \
         /ColorSpace /CS1 /BitsPerComponent 8 /Length {} >>\nstream\n",
        samples.len()
    );
    buf.extend_from_slice(img_hdr.as_bytes());
    buf.extend_from_slice(&samples);
    buf.extend_from_slice(b"\nendstream\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(b"6 0 obj\n[/Separation /Pantone-185 /DeviceCMYK 7 0 R]\nendobj\n");
    offsets.push(buf.len());
    buf.extend_from_slice(
        b"7 0 obj\n<< /FunctionType 2 /Domain [0 1] /N 1 \
            /C0 [0 0 0 0] /C1 [0 0.85 0.45 0] >>\nendobj\n",
    );
    let pdf = finalize_pdf(buf, offsets);

    let doc = PdfDocument::from_bytes(pdf).expect("parse");
    let plates = render_separations(&doc, 0, 72).expect("render");
    let pantone = plate(&plates, "Pantone-185");
    assert!(
        sample(pantone, 50, 50) > 200,
        "Separation image lands on its named plate; got {}",
        sample(pantone, 50, 50)
    );
    // Process plates receive no ink from the spot image (no OPM, no
    // knockout — see commit message for the overprint-deferred scope).
    let cyan = plate(&plates, "Cyan");
    assert_eq!(sample(cyan, 50, 50), 0, "Spot image leaves process plates untouched");
}
