//! Tests for separation plate rendering.
//!
//! Verifies that individual ink separation plates are rendered correctly
//! as grayscale images where pixel intensity = tint percentage.

#[cfg(feature = "rendering")]
mod tests {
    use pdf_oxide::document::PdfDocument;
    use pdf_oxide::rendering::{render_separation, render_separations};

    /// Build a minimal PDF with a Separation color space and a filled rectangle.
    ///
    /// The page is 100x100 pt with a 50x50 pt rectangle centered at (25,25)
    /// filled with the given ink at the given tint.
    fn build_separation_pdf(ink_name: &str, tint: f32) -> Vec<u8> {
        let content = format!("/CS1 cs\n{} scn\n25 25 50 50 re f\n", tint);
        let content_bytes = content.as_bytes();

        let mut buf = Vec::new();
        let mut offsets = Vec::new();

        // Header
        buf.extend_from_slice(b"%PDF-1.4\n");

        // Obj 1: Catalog
        offsets.push(buf.len());
        buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        // Obj 2: Pages
        offsets.push(buf.len());
        buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

        // Obj 3: Page
        offsets.push(buf.len());
        buf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Resources << /ColorSpace << /CS1 5 0 R >> >> >>\nendobj\n",
        );

        // Obj 4: Content stream
        offsets.push(buf.len());
        let stream_header = format!("4 0 obj\n<< /Length {} >>\nstream\n", content_bytes.len());
        buf.extend_from_slice(stream_header.as_bytes());
        buf.extend_from_slice(content_bytes);
        buf.extend_from_slice(b"\nendstream\nendobj\n");

        // Obj 5: Separation color space
        offsets.push(buf.len());
        let cs = format!("5 0 obj\n[/Separation /{} /DeviceGray 6 0 R]\nendobj\n", ink_name);
        buf.extend_from_slice(cs.as_bytes());

        // Obj 6: Tint transform (identity: input tint -> output tint)
        offsets.push(buf.len());
        buf.extend_from_slice(
            b"6 0 obj\n<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [0] /C1 [1] >>\nendobj\n",
        );

        // Xref table
        let xref_offset = buf.len();
        buf.extend_from_slice(b"xref\n");
        let line = format!("0 {}\n", offsets.len() + 1);
        buf.extend_from_slice(line.as_bytes());
        buf.extend_from_slice(b"0000000000 65535 f \n");
        for offset in &offsets {
            let entry = format!("{:010} 00000 n \n", offset);
            buf.extend_from_slice(entry.as_bytes());
        }

        // Trailer
        let trailer = format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            offsets.len() + 1,
            xref_offset
        );
        buf.extend_from_slice(trailer.as_bytes());

        buf
    }

    /// Build a PDF with DeviceCMYK content (a filled rectangle).
    fn build_cmyk_pdf(c: f32, m: f32, y: f32, k: f32) -> Vec<u8> {
        let content = format!("{} {} {} {} k\n25 25 50 50 re f\n", c, m, y, k);
        let content_bytes = content.as_bytes();

        let mut buf = Vec::new();
        let mut offsets = Vec::new();

        buf.extend_from_slice(b"%PDF-1.4\n");

        offsets.push(buf.len());
        buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        offsets.push(buf.len());
        buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

        offsets.push(buf.len());
        buf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Resources << >> >>\nendobj\n",
        );

        offsets.push(buf.len());
        let stream_header = format!("4 0 obj\n<< /Length {} >>\nstream\n", content_bytes.len());
        buf.extend_from_slice(stream_header.as_bytes());
        buf.extend_from_slice(content_bytes);
        buf.extend_from_slice(b"\nendstream\nendobj\n");

        let xref_offset = buf.len();
        buf.extend_from_slice(b"xref\n");
        let line = format!("0 {}\n", offsets.len() + 1);
        buf.extend_from_slice(line.as_bytes());
        buf.extend_from_slice(b"0000000000 65535 f \n");
        for offset in &offsets {
            let entry = format!("{:010} 00000 n \n", offset);
            buf.extend_from_slice(entry.as_bytes());
        }

        let trailer = format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            offsets.len() + 1,
            xref_offset
        );
        buf.extend_from_slice(trailer.as_bytes());

        buf
    }

    /// Build a PDF with a DeviceN color space containing multiple inks.
    fn build_devicen_pdf(ink_names: &[&str], tints: &[f32]) -> Vec<u8> {
        assert_eq!(ink_names.len(), tints.len());

        // Build the scn components string
        let tint_str: String = tints.iter().map(|t| format!("{} ", t)).collect();
        let content = format!("/CS1 cs\n{}scn\n25 25 50 50 re f\n", tint_str);
        let content_bytes = content.as_bytes();

        // Build ink name array string
        let inks_str: String = ink_names.iter().map(|n| format!("/{} ", n)).collect();

        let mut buf = Vec::new();
        let mut offsets = Vec::new();

        buf.extend_from_slice(b"%PDF-1.4\n");

        offsets.push(buf.len());
        buf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        offsets.push(buf.len());
        buf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

        offsets.push(buf.len());
        buf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Contents 4 0 R /Resources << /ColorSpace << /CS1 5 0 R >> >> >>\nendobj\n",
        );

        offsets.push(buf.len());
        let stream_header = format!("4 0 obj\n<< /Length {} >>\nstream\n", content_bytes.len());
        buf.extend_from_slice(stream_header.as_bytes());
        buf.extend_from_slice(content_bytes);
        buf.extend_from_slice(b"\nendstream\nendobj\n");

        // DeviceN color space: [/DeviceN [/Ink1 /Ink2 ...] /DeviceGray /TintTransform]
        offsets.push(buf.len());
        let cs = format!("5 0 obj\n[/DeviceN [{}] /DeviceGray 6 0 R]\nendobj\n", inks_str.trim());
        buf.extend_from_slice(cs.as_bytes());

        offsets.push(buf.len());
        buf.extend_from_slice(
            b"6 0 obj\n<< /FunctionType 2 /Domain [0 1] /N 1 /C0 [0] /C1 [1] >>\nendobj\n",
        );

        let xref_offset = buf.len();
        buf.extend_from_slice(b"xref\n");
        let line = format!("0 {}\n", offsets.len() + 1);
        buf.extend_from_slice(line.as_bytes());
        buf.extend_from_slice(b"0000000000 65535 f \n");
        for offset in &offsets {
            let entry = format!("{:010} 00000 n \n", offset);
            buf.extend_from_slice(entry.as_bytes());
        }

        let trailer = format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            offsets.len() + 1,
            xref_offset
        );
        buf.extend_from_slice(trailer.as_bytes());

        buf
    }

    #[test]
    fn separation_ink_appears_in_plate() {
        let pdf_bytes = build_separation_pdf("Dieline", 0.8);
        let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");

        let plate = render_separation(&doc, 0, "Dieline", 72).expect("render Dieline plate");

        assert_eq!(plate.ink_name, "Dieline");
        assert_eq!(plate.width, 100);
        assert_eq!(plate.height, 100);
        assert_eq!(plate.data.len(), 100 * 100);

        // The rectangle is from (25,25) to (75,75) in PDF coords.
        // At 72 DPI, 1pt = 1px, so check pixel at center of rectangle.
        // PDF y=50 -> image y = 100 - 50 = 50 (flipped y)
        let center_x = 50usize;
        let center_y = 50usize;
        let center_val = plate.data[center_y * plate.width as usize + center_x];

        // Tint 0.8 should give ~204 (0.8 * 255)
        assert!(center_val > 180, "Expected tint ~204 at rectangle center, got {}", center_val);

        // Check outside the rectangle is empty (no ink)
        let outside_val = plate.data[5 * plate.width as usize + 5];
        assert_eq!(outside_val, 0, "Expected zero tint outside rectangle, got {}", outside_val);
    }

    #[test]
    fn cmyk_content_appears_in_process_plates() {
        let pdf_bytes = build_cmyk_pdf(0.5, 0.0, 0.0, 0.0); // 50% Cyan
        let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");

        let plates = render_separations(&doc, 0, 72).expect("render separations");

        // Should have CMYK plates
        let ink_names: Vec<&str> = plates.iter().map(|p| p.ink_name.as_str()).collect();
        assert!(ink_names.contains(&"Cyan"), "Expected Cyan plate, got {:?}", ink_names);
        assert!(ink_names.contains(&"Magenta"), "Expected Magenta plate, got {:?}", ink_names);
        assert!(ink_names.contains(&"Yellow"), "Expected Yellow plate, got {:?}", ink_names);
        assert!(ink_names.contains(&"Black"), "Expected Black plate, got {:?}", ink_names);

        let cyan_plate = plates.iter().find(|p| p.ink_name == "Cyan").unwrap();
        let center_val = cyan_plate.data[50 * cyan_plate.width as usize + 50];
        // 50% cyan should give ~128
        assert!(
            center_val > 100 && center_val < 160,
            "Expected ~128 for 50% cyan, got {}",
            center_val
        );

        // Magenta plate should be empty (0% magenta)
        let magenta_plate = plates.iter().find(|p| p.ink_name == "Magenta").unwrap();
        let magenta_center = magenta_plate.data[50 * magenta_plate.width as usize + 50];
        assert_eq!(magenta_center, 0, "Expected zero magenta, got {}", magenta_center);
    }

    #[test]
    fn empty_plate_for_missing_ink() {
        let pdf_bytes = build_separation_pdf("Varnish", 1.0);
        let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");

        // Request a plate for an ink that doesn't exist on the page
        let plate = render_separation(&doc, 0, "Dieline", 72).expect("render Dieline plate");

        assert_eq!(plate.ink_name, "Dieline");
        // All pixels should be zero
        let non_zero = plate.data.iter().filter(|&&v| v > 0).count();
        assert_eq!(
            non_zero, 0,
            "Expected all-zero plate for missing ink, got {} non-zero pixels",
            non_zero
        );
    }

    #[test]
    fn devicen_ink_routing() {
        let pdf_bytes = build_devicen_pdf(&["SpotRed", "SpotBlue"], &[0.7, 0.3]);
        let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");

        let red_plate = render_separation(&doc, 0, "SpotRed", 72).expect("render SpotRed plate");
        let blue_plate = render_separation(&doc, 0, "SpotBlue", 72).expect("render SpotBlue plate");

        let red_center = red_plate.data[50 * red_plate.width as usize + 50];
        let blue_center = blue_plate.data[50 * blue_plate.width as usize + 50];

        // SpotRed at tint 0.7 -> ~179
        assert!(red_center > 150, "Expected SpotRed tint ~179, got {}", red_center);
        // SpotBlue at tint 0.3 -> ~77
        assert!(
            blue_center > 50 && blue_center < 110,
            "Expected SpotBlue tint ~77, got {}",
            blue_center
        );
    }

    #[test]
    fn render_separations_returns_all_inks() {
        let pdf_bytes = build_separation_pdf("Dieline", 1.0);
        let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");

        let plates = render_separations(&doc, 0, 72).expect("render all separations");

        let ink_names: Vec<&str> = plates.iter().map(|p| p.ink_name.as_str()).collect();
        assert!(
            ink_names.contains(&"Dieline"),
            "Expected Dieline in plates, got {:?}",
            ink_names
        );
    }

    #[test]
    fn full_tint_separation_plate() {
        let pdf_bytes = build_separation_pdf("FullInk", 1.0);
        let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");

        let plate = render_separation(&doc, 0, "FullInk", 72).expect("render plate");
        let center_val = plate.data[50 * plate.width as usize + 50];

        // Full tint (1.0) -> 255
        assert!(center_val > 240, "Expected ~255 for full tint, got {}", center_val);
    }

    #[test]
    fn zero_tint_separation_plate() {
        let pdf_bytes = build_separation_pdf("ZeroInk", 0.0);
        let doc = PdfDocument::from_bytes(pdf_bytes).expect("parse PDF");

        let plate = render_separation(&doc, 0, "ZeroInk", 72).expect("render plate");
        let center_val = plate.data[50 * plate.width as usize + 50];

        assert_eq!(center_val, 0, "Expected 0 for zero tint, got {}", center_val);
    }
}
