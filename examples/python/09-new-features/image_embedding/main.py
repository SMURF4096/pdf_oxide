# Image embedding — v0.3.41
#
# Demonstrates embedding JPEG/PNG images into a PDF using raw bytes.
# No pixel dimensions needed — the library auto-detects them from the
# image header. Just supply the display rectangle in PDF points (72 pt = 1 inch).
#
# Addresses issue #425: ImageContent::new() required explicit width/height;
# image_from_bytes() does not.
#
# Run: python main.py

from __future__ import annotations

import os

import pdf_oxide


OUT_DIR = "output"
os.makedirs(OUT_DIR, exist_ok=True)

# 1×1 white PNG (68 bytes) — embedded so the example needs no external files.
WHITE_PNG = bytes(
    [
        0x89,
        0x50,
        0x4E,
        0x47,
        0x0D,
        0x0A,
        0x1A,
        0x0A,
        0x00,
        0x00,
        0x00,
        0x0D,
        0x49,
        0x48,
        0x44,
        0x52,
        0x00,
        0x00,
        0x00,
        0x01,
        0x00,
        0x00,
        0x00,
        0x01,
        0x08,
        0x02,
        0x00,
        0x00,
        0x00,
        0x90,
        0x77,
        0x53,
        0xDE,
        0x00,
        0x00,
        0x00,
        0x0C,
        0x49,
        0x44,
        0x41,
        0x54,
        0x78,
        0x9C,
        0x63,
        0xF8,
        0xFF,
        0xFF,
        0x3F,
        0x00,
        0x05,
        0xFE,
        0x02,
        0xFE,
        0x0D,
        0xEF,
        0x46,
        0xB8,
        0x00,
        0x00,
        0x00,
        0x00,
        0x49,
        0x45,
        0x4E,
        0x44,
        0xAE,
        0x42,
        0x60,
        0x82,
    ]
)

# page.image_with_alt(bytes, x, y, w, h, alt) embeds with accessibility alt text.
# page.image_artifact(bytes, x, y, w, h) embeds as decorative (no alt).
# x, y, w, h are the on-page display rectangle in PDF points (72 pt = 1 inch).
b = pdf_oxide.DocumentBuilder()
b.title("Image Embedding Demo")
page = b.letter_page()
page.font("Helvetica", 12)
page.at(72, 720).heading(1, "Image embedding with auto-detected dimensions")
page.at(72, 690).paragraph("No pixel dims needed — the library reads them from the image header.")
page.image_with_alt(WHITE_PNG, 72, 480, 200, 200, "white square test image")
page.at(72, 460).paragraph("Image displayed 200×200 pt — pixel resolution is auto-detected.")
page.done()

out_path = os.path.join(OUT_DIR, "image_embedding.pdf")
b.save(out_path)
print(f"Written: {out_path}")
