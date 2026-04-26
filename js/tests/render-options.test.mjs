// Tests for renderPageWithOptions + estimateRenderTime TS wrappers
// over the pdf_render_page_with_options N-API export.

import assert from 'node:assert/strict';
import { test } from 'node:test';

let Pdf, PdfDocument;
try {
  ({ Pdf, PdfDocument } = await import('../lib/index.js'));
} catch {
  // library not built — all tests will be skipped
}

const skip = !Pdf;

function makeDoc() {
  const bytes = Pdf.fromMarkdown('# Render\n\nBody.').saveToBytes();
  return PdfDocument.openFromBuffer(Buffer.from(bytes));
}

function isPng(b) {
  return b.length >= 8 && b[0] === 0x89 && b[1] === 0x50 && b[2] === 0x4e && b[3] === 0x47;
}

function isJpeg(b) {
  return b.length >= 3 && b[0] === 0xff && b[1] === 0xd8 && b[2] === 0xff;
}

test('renderPageWithOptions defaults produce PNG bytes', { skip }, () => {
  const doc = makeDoc();
  const bytes = doc.renderPageWithOptions(0);
  assert.ok(isPng(bytes), 'default format should be PNG');
  assert.ok(bytes.length > 128);
});

test('renderPageWithOptions with format=jpeg emits JPEG', { skip }, () => {
  const doc = makeDoc();
  const bytes = doc.renderPageWithOptions(0, { format: 'jpeg' });
  assert.ok(isJpeg(bytes));
});

test('renderPageWithOptions higher DPI → more bytes', { skip }, () => {
  const doc = makeDoc();
  const small = doc.renderPageWithOptions(0, { dpi: 72 });
  const large = doc.renderPageWithOptions(0, { dpi: 300 });
  assert.ok(isPng(small) && isPng(large));
  assert.ok(large.length > small.length);
});

test('renderPageWithOptions transparentBackground still PNG', { skip }, () => {
  const doc = makeDoc();
  const bytes = doc.renderPageWithOptions(0, { transparentBackground: true });
  assert.ok(isPng(bytes));
});

test('renderPageWithOptions RGB background accepted', { skip }, () => {
  const doc = makeDoc();
  const bytes = doc.renderPageWithOptions(0, { background: [0.2, 0.2, 0.2, 1] });
  assert.ok(isPng(bytes));
});

test('renderPageWithOptions renderAnnotations=false accepted', { skip }, () => {
  const doc = makeDoc();
  const bytes = doc.renderPageWithOptions(0, { renderAnnotations: false });
  assert.ok(isPng(bytes));
});

test('renderPageWithOptions rejects invalid dpi', { skip }, () => {
  const doc = makeDoc();
  assert.throws(() => doc.renderPageWithOptions(0, { dpi: 0 }), /dpi/);
});

test('renderPageWithOptions rejects invalid jpegQuality', { skip }, () => {
  const doc = makeDoc();
  assert.throws(
    () => doc.renderPageWithOptions(0, { format: 'jpeg', jpegQuality: 0 }),
    /jpegQuality/
  );
});

test('estimateRenderTime returns a non-negative number', { skip }, () => {
  const doc = makeDoc();
  const ms = doc.estimateRenderTime(0, 150);
  assert.equal(typeof ms, 'number');
  assert.ok(ms >= 0);
});
