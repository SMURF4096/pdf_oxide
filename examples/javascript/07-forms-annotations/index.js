// Extract form fields and annotations from a PDF.
// Run: node index.js form.pdf

const binding = require("pdf-oxide");

const path = process.argv[2];
if (!path) {
  console.error("Usage: node index.js <form.pdf>");
  process.exit(1);
}

const handle = binding.open(path);
try {
  console.log(`Opened: ${path}`);

  const pageCount = binding.pageCount(handle);
  for (let page = 0; page < pageCount; page++) {
    // Form fields
    const fields = binding.getFormFields(handle, page);
    if (fields.length > 0) {
      console.log(`\n--- Form Fields (page ${page + 1}) ---`);
      for (const f of fields) {
        console.log(
          `  Name: ${JSON.stringify(f.name).padEnd(20)} ` +
            `Type: ${f.type.padEnd(12)} ` +
            `Value: ${JSON.stringify(f.value).padEnd(16)} ` +
            `Required: ${f.required}`
        );
      }
    }

    // Annotations
    const annotations = binding.getAnnotations(handle, page);
    if (annotations.length > 0) {
      console.log(`\n--- Annotations (page ${page + 1}) ---`);
      for (const a of annotations) {
        console.log(
          `  Type: ${a.type.padEnd(14)} Page: ${page + 1}   ` +
            `Contents: "${a.contents || ""}"`
        );
      }
    }
  }
} finally {
  binding.close(handle);
}
