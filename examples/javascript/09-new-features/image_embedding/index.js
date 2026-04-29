// Image embedding — v0.3.40
//
// Demonstrates embedding JPEG/PNG images into a PDF using raw bytes.
// No pixel dimensions needed — the library auto-detects them from the
// image header. Just supply the display rectangle in PDF points (72 pt = 1 inch).
//
// Addresses issue #425: ImageContent::new() required explicit width/height;
// PageBuilder.image() does not.
//
// Run: node index.js

import path from "node:path";
import { fileURLToPath } from "node:url";
import fs from "node:fs";
import { DocumentBuilder } from "pdf-oxide";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const OUT_DIR = path.join(__dirname, "output");
fs.mkdirSync(OUT_DIR, { recursive: true });

// 1×1 white PNG (68 bytes) — embedded so the example needs no external files.
const WHITE_PNG = Buffer.from([
  0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d,
  0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
  0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xde, 0x00, 0x00, 0x00,
  0x0c, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0xf8, 0xff, 0xff, 0x3f,
  0x00, 0x05, 0xfe, 0x02, 0xfe, 0x0d, 0xef, 0x46, 0xb8, 0x00, 0x00, 0x00,
  0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
]);

// page.image(bytes, x, y, w, h) — no pixel dims needed.
// x, y, w, h are the on-page display rectangle in PDF points (72 pt = 1 inch).
const builder = DocumentBuilder.create().title("Image Embedding Demo");
builder.letterPage()
  .font("Helvetica", 12)
  .at(72, 720).heading(1, "Image embedding with auto-detected dimensions")
  .at(72, 690).paragraph("No pixel dims needed — the library reads them from the image header.")
  .image(WHITE_PNG, 72, 480, 200, 200)
  .at(72, 460).paragraph("Image displayed 200×200 pt — pixel resolution is auto-detected.")
  .done();

const outPath = path.join(OUT_DIR, "image_embedding.pdf");
builder.save(outPath);
console.log(`Written: ${outPath}`);
process.exit(0);
