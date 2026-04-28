// Encrypted PDF output — v0.3.40
// Run: node index.js

import path from "node:path";
import { fileURLToPath } from "node:url";
import fs from "node:fs";
import { DocumentBuilder, PdfDocument } from "pdf-oxide";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const OUT_DIR = path.join(__dirname, "output");
fs.mkdirSync(OUT_DIR, { recursive: true });

const builder = DocumentBuilder.create().title("Encrypted PDF Demo");
builder.letterPage()
  .font("Helvetica", 12)
  .at(72, 720).heading(1, "Encrypted PDF")
  .at(72, 690).paragraph("This PDF is encrypted with a user password.")
  .done();

const pdfBytes = builder.build();
console.log(`  Original PDF size: ${pdfBytes.length} bytes`);

const doc = PdfDocument.openFromBuffer(pdfBytes);
const encrypted = doc.saveEncryptedToBytes("user123", undefined, true, false, false, false);

if (encrypted.length === 0) throw new Error("encrypted output is empty");
console.log(`  Encrypted PDF size: ${encrypted.length} bytes`);

const outPath = path.join(OUT_DIR, "encrypted.pdf");
fs.writeFileSync(outPath, encrypted);
console.log(`Written: ${outPath}`);
process.exit(0);
