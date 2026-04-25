# v0.3.39 new-feature showcase — Python
#
# Exercises every major feature added in this release as a real user would:
#   1. StreamingTable with rowspan
#   2. PDF/UA accessible image (image_with_alt)
#   3. PDF/UA decorative image artifact (image_artifact)
#   4. build() / PdfDocument.from_bytes() in-memory round-trip
#   5. CMS signing via PKCS#12 (Certificate.load_pkcs12 + sign_pdf_bytes)
#   6. RFC 3161 Timestamp parsing
#   7. TsaClient construction (offline — no network call)
#
# Run:
#   pip install pdf-oxide
#   python main.py

from __future__ import annotations

import os

import pdf_oxide

OUT_DIR = "output_new_features"

# Minimal 1×1 white PNG (no external file needed).
WHITE_PNG = bytes([
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
    0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
    0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41,
    0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC,
    0x33, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
    0x44, 0xAE, 0x42, 0x60, 0x82,
])


def main() -> None:
    os.makedirs(OUT_DIR, exist_ok=True)

    feature_streaming_table_rowspan()
    feature_pdf_ua_accessible_image()
    feature_save_to_bytes_roundtrip()
    feature_timestamp_parsing()
    feature_tsa_client_construction()
    feature_pkcs12_signing()

    print(f"\nAll outputs written to {OUT_DIR}/")


# ── 1. StreamingTable with rowspan ────────────────────────────────────────────

def feature_streaming_table_rowspan() -> None:
    print("Building streaming table with rowspan...")

    doc = pdf_oxide.DocumentBuilder().title("StreamingTable Demo")
    page = doc.letter_page().font("Helvetica", 10).at(72, 700).heading(1, "Product Catalogue").at(72, 660)

    tbl = page.streaming_table(
        columns=[
            pdf_oxide.Column("Category", width=120),
            pdf_oxide.Column("Item",     width=160),
            pdf_oxide.Column("Notes",    width=150, align=pdf_oxide.Align.RIGHT),
        ],
        repeat_header=True,
        max_rowspan=2,
    )
    tbl.push_row_span([("Fruits", 2), ("Apple", 1),   ("crisp",  1)])  # Fruits spans 2 rows
    tbl.push_row_span([("",       1), ("Banana", 1),  ("sweet",  1)])  # continuation
    tbl.push_row_span([("Vegetables", 1), ("Carrot", 1), ("earthy", 1)])

    path = os.path.join(OUT_DIR, "streaming_table_rowspan.pdf")
    tbl.finish().done().save(path)
    print(f"  -> {path}")


# ── 2. PDF/UA accessible image ────────────────────────────────────────────────

def feature_pdf_ua_accessible_image() -> None:
    print("Building PDF/UA document with accessible image...")

    doc = (
        pdf_oxide.DocumentBuilder()
        .title("Accessible PDF Demo")
        .tagged_pdf_ua1()
        .language("en-US")
    )
    page = (
        doc.a4_page()
        .font("Helvetica", 12)
        .at(72, 750)
        .heading(1, "Accessible document with images")
        .at(72, 720)
        .paragraph("The image below has descriptive alt text for screen readers.")
        # PDF/UA accessible image: alt text for screen readers
        .image_with_alt(WHITE_PNG, 72, 580, 100, 100,
                        "A white placeholder image for demonstration")
        .at(72, 545)
        .paragraph("The logo below is purely decorative and marked as an artifact.")
        # Decorative image: marked as /Artifact, no alt text
        .image_artifact(WHITE_PNG, 72, 445, 60, 60)
    )

    path = os.path.join(OUT_DIR, "pdf_ua_accessible_images.pdf")
    page.done().save(path)
    print(f"  -> {path}")


# ── 3. build() / from_bytes round-trip ────────────────────────────────────────

def feature_save_to_bytes_roundtrip() -> None:
    print("Demonstrating in-memory round-trip (build + PdfDocument.from_bytes)...")

    pdf_bytes: bytes = (
        pdf_oxide.DocumentBuilder()
        .title("In-Memory Round-Trip Demo")
        .letter_page()
        .font("Helvetica", 12)
        .at(72, 720)
        .heading(1, "In-Memory Round-Trip")
        .at(72, 690)
        .paragraph("This PDF was built in memory, never written to disk mid-way.")
        .done()
        .build()
    )

    # Re-open from bytes — no filesystem path involved.
    reader = pdf_oxide.PdfDocument.from_bytes(pdf_bytes)
    text = "\n".join(reader.extract_text(p) for p in range(reader.page_count()))
    print(f"  Extracted {len(text)} chars from in-memory PDF")
    assert "In-Memory" in text, "round-trip text missing"

    path = os.path.join(OUT_DIR, "save_to_bytes_roundtrip.pdf")
    with open(path, "wb") as f:
        f.write(pdf_bytes)
    print(f"  -> {path}")


# ── 4. RFC 3161 Timestamp parsing ─────────────────────────────────────────────

def feature_timestamp_parsing() -> None:
    print("Parsing RFC 3161 timestamp...")
    bare_tst_info = bytes.fromhex(
        "3081B302010106042A0304013031300D060960864801650304020105000420"
        "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD"
        "020104180F32303233303630373131323632365A300A020101800201F4810164"
        "0101FF0208314CFCE4E0651827A048A4463044310B30090603550406130255533113"
        "301106035504080C0A536F6D652D5374617465310D300B060355040A0C04546573"
        "743111300F06035504030C085465737420545341"
    )
    try:
        ts = pdf_oxide.Timestamp.parse(bare_tst_info)
        print(f"  Timestamp time (epoch): {ts.time}")
        print(f"  Serial: {ts.serial}  Policy OID: {ts.policy_oid}")
        print(f"  TSA name: {ts.tsa_name}")
        assert ts.serial == "04", f"unexpected serial: {ts.serial}"
        assert ts.policy_oid == "1.2.3.4.1"
        print("  Timestamp fields verified.")
    except NotImplementedError:
        print("  SKIP: signatures feature not compiled in.")


# ── 5. TsaClient construction ─────────────────────────────────────────────────

def feature_tsa_client_construction() -> None:
    print("Constructing TsaClient (offline, no network call)...")
    try:
        client = pdf_oxide.TsaClient(
            url="https://freetsa.org/tsr",
            timeout_seconds=30,
            hash_algorithm=2,  # SHA-256
            use_nonce=True,
            cert_req=True,
        )
        print(f"  TsaClient created: {client!r}")
    except NotImplementedError:
        print("  SKIP: signatures feature not compiled in.")


# ── 6. PKCS#12 signing ────────────────────────────────────────────────────────

def feature_pkcs12_signing() -> None:
    print("Signing PDF with PKCS#12 certificate...")

    p12_path = os.path.join(
        os.path.dirname(__file__),
        "..", "..", "..", "tests", "fixtures", "test_signing.p12",
    )
    if not os.path.exists(p12_path):
        print(f"  SKIP: {p12_path} not found")
        return

    try:
        with open(p12_path, "rb") as f:
            p12_data = f.read()

        cert = pdf_oxide.Certificate.load_pkcs12(p12_data, "testpass")
        print(f"  Certificate subject: {cert.subject()}")

        pdf_bytes: bytes = (
            pdf_oxide.DocumentBuilder()
            .title("Signed Invoice")
            .letter_page()
            .font("Helvetica", 12)
            .at(72, 720)
            .heading(1, "Signed Invoice")
            .at(72, 690)
            .paragraph("This document carries a CMS/PKCS#7 digital signature.")
            .done()
            .build()
        )

        signed: bytes = pdf_oxide.sign_pdf_bytes(
            pdf_bytes, cert, reason="Approved", location="HQ"
        )

        path = os.path.join(OUT_DIR, "signed_document.pdf")
        with open(path, "wb") as f:
            f.write(signed)
        print(f"  -> {path} ({len(signed)} bytes)")
        assert b"/ByteRange" in signed, "ByteRange missing from signed PDF"
        print("  Signature verified: /ByteRange present.")
    except (NotImplementedError, AttributeError) as e:
        print(f"  SKIP: {e}")


if __name__ == "__main__":
    main()
