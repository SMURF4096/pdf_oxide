// Search for a term across all pages of a PDF and print matches.
// Run: node index.js document.pdf "query"

const { PdfDocument, binding } = require("pdf-oxide");

function main() {
  const path = process.argv[2];
  const query = process.argv[3];
  if (!path || !query) {
    console.error('Usage: node index.js <file.pdf> "query"');
    process.exit(1);
  }

  const doc = new PdfDocument(path);
  const pages = doc.getPageCount();
  console.log(`Searching for "${query}" in ${path} (${pages} pages)...\n`);

  let total = 0;
  let pagesWithHits = 0;

  for (let i = 0; i < pages; i++) {
    const results = binding.searchPage(doc._handle, i, query, false);
    if (!results || results.length === 0) continue;

    pagesWithHits++;
    console.log(`Page ${i + 1}: ${results.length} match(es)`);
    for (const r of results) {
      console.log(`  - "...${r.context}..."`);
      total++;
    }
    console.log();
  }

  doc.close();
  console.log(`Found ${total} total matches across ${pagesWithHits} pages.`);
}

main();
