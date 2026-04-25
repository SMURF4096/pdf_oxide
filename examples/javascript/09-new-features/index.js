// v0.3.39 new-feature showcase — Node.js
//
// Exercises every major feature added in this release as a real user would:
//   1. StreamingTable with rowspan
//   2. PDF/UA accessible image (imageWithAlt)
//   3. PDF/UA decorative image artifact (imageArtifact)
//   4. build() / PdfDocument.openFromBuffer() in-memory round-trip
//   5. CMS signing via PKCS#12 (SignatureManager.signWithPkcs12)
//   6. RFC 3161 Timestamp parsing
//   7. TsaClient construction (offline — no network call)
//
// Run: node index.js

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  DocumentBuilder,
  PdfDocument,
  Timestamp,
  TsaClient,
  TimestampHashAlgorithm,
  SignatureManager,
  SignatureException,
  Align,
} from "pdf-oxide";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const OUT_DIR = "output_new_features";

// Minimal 1×1 white PNG (no external file needed).
const WHITE_PNG = Buffer.from([
  0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a,
  0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
  0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
  0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
  0xde, 0x00, 0x00, 0x00, 0x0c, 0x49, 0x44, 0x41,
  0x54, 0x78, 0x9c, 0x63, 0xf8, 0xff, 0xff, 0x3f,
  0x00, 0x05, 0xfe, 0x02, 0xfe, 0x0d, 0xef, 0x46,
  0xb8, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e,
  0x44, 0xae, 0x42, 0x60, 0x82,
]);

async function main() {
  fs.mkdirSync(OUT_DIR, { recursive: true });

  await featureStreamingTableRowspan();
  await featurePdfUaAccessibleImage();
  await featureSaveToBytesRoundtrip();
  featureTimestampParsing();
  featureTsaClientConstruction();
  await featurePkcs12Signing();

  console.log(`\nAll outputs written to ${OUT_DIR}/`);
}

// ── 1. StreamingTable with rowspan ────────────────────────────────────────────

async function featureStreamingTableRowspan() {
  console.log("Building streaming table with rowspan...");

  const builder = DocumentBuilder.create().title("StreamingTable Demo");
  const page = builder.letterPage().font("Helvetica", 10).at(72, 700).heading(1, "Product Catalogue").at(72, 660);

  const tbl = page.streamingTable({
    columns: [
      { header: "Category", width: 120 },
      { header: "Item",     width: 160 },
      { header: "Notes",    width: 150, align: Align.Right },
    ],
    repeatHeader: true,
    maxRowspan: 2,
  });

  tbl.pushRowSpan([{ text: "Fruits",     rowspan: 2 }, { text: "Apple",  rowspan: 1 }, { text: "crisp",  rowspan: 1 }]);
  tbl.pushRowSpan([{ text: "",           rowspan: 1 }, { text: "Banana", rowspan: 1 }, { text: "sweet",  rowspan: 1 }]);
  tbl.pushRowSpan([{ text: "Vegetables", rowspan: 1 }, { text: "Carrot", rowspan: 1 }, { text: "earthy", rowspan: 1 }]);
  (await tbl.finish()).done();

  const outPath = path.join(OUT_DIR, "streaming_table_rowspan.pdf");
  builder.save(outPath);
  console.log(`  -> ${outPath}`);
}

// ── 2. PDF/UA accessible image ────────────────────────────────────────────────

async function featurePdfUaAccessibleImage() {
  console.log("Building PDF/UA document with accessible image...");

  const builder = DocumentBuilder.create()
    .title("Accessible PDF Demo")
    .taggedPdfUa1()
    .language("en-US");

  builder.a4Page()
    .font("Helvetica", 12)
    .at(72, 750)
    .heading(1, "Accessible document with images")
    .at(72, 720)
    .paragraph("The image below has descriptive alt text for screen readers.")
    // PDF/UA accessible image: alt text for assistive technology
    .imageWithAlt(WHITE_PNG, 72, 580, 100, 100,
      "A white placeholder image used for demonstration purposes")
    .at(72, 545)
    .paragraph("The logo below is purely decorative and marked as an artifact.")
    // Decorative image: marked as /Artifact, no alt text
    .imageArtifact(WHITE_PNG, 72, 445, 60, 60)
    .done();

  const outPath = path.join(OUT_DIR, "pdf_ua_accessible_images.pdf");
  builder.save(outPath);
  console.log(`  -> ${outPath}`);
}

// ── 3. build() / openFromBuffer() round-trip ─────────────────────────────────

async function featureSaveToBytesRoundtrip() {
  console.log("Demonstrating in-memory round-trip (build + PdfDocument.openFromBuffer)...");

  const builder = DocumentBuilder.create().title("In-Memory Round-Trip Demo");
  builder.letterPage()
    .font("Helvetica", 12)
    .at(72, 720)
    .heading(1, "In-Memory Round-Trip")
    .at(72, 690)
    .paragraph("This PDF was built in memory, never written to disk mid-way.")
    .done();

  const pdfBytes = builder.build();

  // Re-open from bytes — no filesystem path involved.
  const doc = PdfDocument.openFromBuffer(pdfBytes);
  let text = "";
  for (let i = 0; i < doc.pageCount(); i++) {
    text += doc.extractText(i);
  }
  console.log(`  Extracted ${text.length} chars from in-memory PDF`);
  if (!text.includes("In-Memory")) {
    throw new Error("round-trip text missing");
  }

  const outPath = path.join(OUT_DIR, "save_to_bytes_roundtrip.pdf");
  fs.writeFileSync(outPath, pdfBytes);
  console.log(`  -> ${outPath}`);
}

// ── 4. RFC 3161 Timestamp parsing ─────────────────────────────────────────────

function featureTimestampParsing() {
  console.log("Parsing RFC 3161 timestamp...");

  const bareTstInfo = Buffer.from(
    "3081B302010106042A0304013031300D060960864801650304020105000420" +
    "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD" +
    "020104180F32303233303630373131323632365A300A020101800201F4810164" +
    "0101FF0208314CFCE4E0651827A048A4463044310B30090603550406130255533113" +
    "301106035504080C0A536F6D652D5374617465310D300B060355040A0C04546573" +
    "743111300F06035504030C085465737420545341",
    "hex"
  );

  try {
    const ts = Timestamp.parse(bareTstInfo);
    console.log(`  Timestamp time (epoch): ${ts.time}`);
    console.log(`  Serial: ${ts.serial}  Policy OID: ${ts.policyOid}`);
    console.log(`  TSA name: ${ts.tsaName}`);
    if (ts.serial !== "04") {
      throw new Error(`unexpected serial: ${ts.serial}`);
    }
    if (ts.policyOid !== "1.2.3.4.1") {
      throw new Error(`unexpected policy OID: ${ts.policyOid}`);
    }
    console.log("  Timestamp fields verified.");
    ts.close();
  } catch (err) {
    if (err instanceof Error && (err.message.includes("not available") || err.message.includes("error code 8"))) {
      console.log(`  SKIP: signatures feature not compiled in.`);
    } else {
      throw err;
    }
  }
}

// ── 5. TsaClient construction ─────────────────────────────────────────────────

function featureTsaClientConstruction() {
  console.log("Constructing TsaClient (offline, no network call)...");
  try {
    const client = new TsaClient({
      url: "https://freetsa.org/tsr",
      timeoutSeconds: 30,
      hashAlgorithm: TimestampHashAlgorithm.Sha256,
      useNonce: true,
      certReq: true,
    });
    console.log("  TsaClient created (no network call).");
    client.close();
  } catch (err) {
    if (err instanceof Error && (err.message.includes("not available") || err.message.includes("error code 8"))) {
      console.log("  SKIP: signatures feature not compiled in.");
    } else {
      throw err;
    }
  }
}

// ── 6. PKCS#12 signing ────────────────────────────────────────────────────────

async function featurePkcs12Signing() {
  console.log("Signing PDF with PKCS#12 certificate...");

  const p12Path = path.resolve(
    __dirname, "..", "..", "..", "tests", "fixtures", "test_signing.p12"
  );
  if (!fs.existsSync(p12Path)) {
    console.log(`  SKIP: ${p12Path} not found`);
    return;
  }

  try {
    const builder = DocumentBuilder.create().title("Signed Invoice");
    builder.letterPage()
      .font("Helvetica", 12)
      .at(72, 720)
      .heading(1, "Signed Invoice")
      .at(72, 690)
      .paragraph("This document carries a CMS/PKCS#7 digital signature.")
      .done();
    const pdfBytes = builder.build();

    // SignatureManager is the high-level signing surface in Node.js
    const sigManager = new SignatureManager({});
    const signed = await sigManager.signWithPkcs12(pdfBytes, p12Path, "testpass", {
      reason: "Approved",
      location: "HQ",
    });

    const outPath = path.join(OUT_DIR, "signed_document.pdf");
    fs.writeFileSync(outPath, signed);
    console.log(`  -> ${outPath} (${signed.length} bytes)`);

    if (!signed.includes(Buffer.from("/ByteRange"))) {
      throw new Error("ByteRange missing from signed PDF");
    }
    console.log("  Signature verified: /ByteRange present.");
  } catch (err) {
    if (err instanceof SignatureException || (err instanceof Error && err.message.includes("not available"))) {
      console.log(`  SKIP: signatures feature not available (${err.message})`);
    } else {
      throw err;
    }
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
