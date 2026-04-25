// TDD tests for Pdf.fromHtmlCss / Pdf.fromHtmlCssWithFonts
// These functions exist in binding.cc (pdfFromHtmlCss / pdfFromHtmlCssWithFonts)
// but were never wrapped in the TypeScript layer. This file defines the expected
// API before the implementation is added.

import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test } from 'node:test';

const __dir = dirname(fileURLToPath(import.meta.url));

// Minimal font for embedding — use DejaVuSans from fixtures if available,
// otherwise skip font-cascade tests.
async function loadFont() {
  const candidates = [
    join(__dir, '../../tools/benchmark-harness/fixtures/fonts/DejaVuSans.ttf'),
    join(__dir, '../fixtures/DejaVuSans.ttf'),
    '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf',
  ];
  for (const p of candidates) {
    try {
      return await readFile(p);
    } catch {
      // try next
    }
  }
  return null;
}

let Pdf;
try {
  ({ Pdf } = await import('../lib/index.js'));
} catch {
  // library not built yet — all tests below will be skipped
}

function isPdf(buf) {
  return buf && buf.length > 4 &&
    buf[0] === 0x25 && buf[1] === 0x50 && buf[2] === 0x44 && buf[3] === 0x46;
}

test('Pdf.fromHtmlCss is exported', { skip: !Pdf }, () => {
  assert.strictEqual(typeof Pdf.fromHtmlCss, 'function');
});

test('Pdf.fromHtmlCssWithFonts is exported', { skip: !Pdf }, () => {
  assert.strictEqual(typeof Pdf.fromHtmlCssWithFonts, 'function');
});

test('fromHtmlCss produces a valid PDF', { skip: !Pdf }, async () => {
  const font = await loadFont();
  assert.ok(font, 'need a font file to test HTML+CSS → PDF');

  const html = '<html><body><h1>Hello CSS</h1><p>World</p></body></html>';
  const css = 'body { font-size: 14px; } h1 { color: black; }';

  const pdf = Pdf.fromHtmlCss(html, css, font);
  const bytes = pdf.saveToBytes();
  assert.ok(isPdf(bytes), 'output should start with %PDF-');
  assert.ok(bytes.length > 200, 'output should be a non-trivial PDF');
  pdf.close();
});

test('fromHtmlCss returns a Pdf with a positive byte count', { skip: !Pdf }, async () => {
  const font = await loadFont();
  assert.ok(font, 'need a font file');

  const pdf = Pdf.fromHtmlCss('<p>test</p>', 'p { font-size: 12px; }', font);
  const bytes = pdf.saveToBytes();
  assert.ok(bytes.length > 0);
  pdf.close();
});

test('fromHtmlCssWithFonts produces a valid PDF', { skip: !Pdf }, async () => {
  const font = await loadFont();
  assert.ok(font, 'need a font file');

  const html = '<p>Multi-font</p>';
  const css = 'p { font-family: Body; }';
  const pdf = Pdf.fromHtmlCssWithFonts(html, css, ['Body'], [font]);
  const bytes = pdf.saveToBytes();
  assert.ok(isPdf(bytes));
  pdf.close();
});

test('fromHtmlCss throws on null html', { skip: !Pdf }, async () => {
  const font = await loadFont();
  assert.ok(font);
  assert.throws(() => Pdf.fromHtmlCss(null, '', font));
});

test('fromHtmlCss throws on null font bytes', { skip: !Pdf }, () => {
  assert.throws(() => Pdf.fromHtmlCss('<p>hi</p>', '', null));
});

test('fromHtmlCssWithFonts throws when families/fonts arrays length mismatch', { skip: !Pdf }, async () => {
  const font = await loadFont();
  assert.ok(font);
  assert.throws(() => Pdf.fromHtmlCssWithFonts('<p>hi</p>', '', ['A', 'B'], [font]));
});
