// Convert PDF pages to Markdown, HTML, and plain text files.
// Run: node index.js document.pdf

const fs = require("fs");
const { PdfDocument } = require("pdf-oxide");

function main() {
  const path = process.argv[2];
  if (!path) {
    console.error("Usage: node index.js <file.pdf>");
    process.exit(1);
  }

  const doc = new PdfDocument(path);
  fs.mkdirSync("output", { recursive: true });

  const pages = doc.getPageCount();
  console.log(`Converting ${pages} pages from ${path}...`);

  for (let i = 0; i < pages; i++) {
    const n = i + 1;
    const formats = [
      ["md", doc.toMarkdown(i)],
      ["html", doc.toHtml(i)],
      ["txt", doc.extractText(i)],
    ];
    for (const [ext, content] of formats) {
      const filename = `output/page_${n}.${ext}`;
      fs.writeFileSync(filename, content);
      console.log(`Saved: ${filename}`);
    }
  }

  doc.close();
  console.log("Done. Files written to output/");
}

main();
