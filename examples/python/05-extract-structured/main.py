# Extract words with bounding boxes and tables from a PDF page.
# Run: python main.py document.pdf

import sys

from pdf_oxide import PdfDocument


def main():
    if len(sys.argv) < 2:
        print("Usage: python main.py <file.pdf>")
        sys.exit(1)

    path = sys.argv[1]
    doc = PdfDocument(path)
    print(f"Opened: {path}")

    page = 0

    # Extract words with position data
    words = doc.extract_words(page)
    print(f"\n--- Words (page {page + 1}) ---")
    for w in words[:20]:
        quoted = '"' + w["text"] + '"'
        print(f'{quoted:<20} '
              f'x={w["x0"]:<7.1f} y={w["y0"]:<7.1f} '
              f'x1={w["x1"]:<7.1f} y1={w["y1"]:<7.1f} '
              f'font={w["fontname"]}  size={w["size"]:.1f}')
    if len(words) > 20:
        print(f"... ({len(words) - 20} more words)")

    # Extract tables
    tables = doc.extract_tables(page)
    print(f"\n--- Tables (page {page + 1}) ---")
    if not tables:
        print("(no tables found)")
    for i, table in enumerate(tables):
        rows, cols = len(table), len(table[0]) if table else 0
        print(f"Table {i + 1}: {rows} rows x {cols} cols")
        for r, row in enumerate(table[:5]):
            cells = "  ".join(f'[{r},{c}] "{v}"' for c, v in enumerate(row[:6]))
            print(f"  {cells}")

if __name__ == "__main__":
    main()
