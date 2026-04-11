// Create PDFs from Markdown, HTML, and plain text.
// Run: node index.js

const { binding } = require("pdf-oxide");

function main() {
  console.log("Creating PDFs...");

  // From Markdown
  const markdown = `# Project Report

## Summary

This document was generated from **Markdown** using pdf_oxide.

- Fast rendering
- Clean typography
- Cross-platform
`;
  let handle = binding.pdfFromMarkdown(markdown);
  binding.pdfSave(handle, "from_markdown.pdf");
  console.log("Saved: from_markdown.pdf");

  // From HTML
  const html = `<html><body>
<h1>Invoice #1234</h1>
<p>Generated from <em>HTML</em> using pdf_oxide.</p>
<table><tr><th>Item</th><th>Price</th></tr>
<tr><td>Widget</td><td>$9.99</td></tr></table>
</body></html>`;
  handle = binding.pdfFromHtml(html);
  binding.pdfSave(handle, "from_html.pdf");
  console.log("Saved: from_html.pdf");

  // From plain text
  const text =
    "Hello, World!\n\nThis PDF was created from plain text using pdf_oxide.";
  handle = binding.pdfFromText(text);
  binding.pdfSave(handle, "from_text.pdf");
  console.log("Saved: from_text.pdf");

  console.log("Done. 3 PDFs created.");
}

main();
