// Open a PDF, modify metadata, delete a page, and save.
// Run: node index.js input.pdf output.pdf

const binding = require("pdf-oxide");

const input = process.argv[2];
const output = process.argv[3];
if (!input || !output) {
  console.error("Usage: node index.js <input.pdf> <output.pdf>");
  process.exit(1);
}

const handle = binding.editorOpen(input);
try {
  console.log(`Opened: ${input}`);

  binding.editorSetTitle(handle, "Edited Document");
  console.log('Set title: "Edited Document"');

  binding.editorSetAuthor(handle, "pdf_oxide");
  console.log('Set author: "pdf_oxide"');

  binding.editorDeletePage(handle, 1); // 0-indexed, deletes page 2
  console.log("Deleted page 2");

  binding.editorSave(handle, output);
  console.log(`Saved: ${output}`);
} finally {
  binding.editorFree(handle);
}
