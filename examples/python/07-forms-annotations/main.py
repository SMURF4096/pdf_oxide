# Extract form fields and annotations from a PDF.
# Run: python main.py form.pdf

import sys

from pdf_oxide import PdfDocument


def main():
    if len(sys.argv) < 2:
        print("Usage: python main.py <form.pdf>")
        sys.exit(1)

    path = sys.argv[1]
    doc = PdfDocument(path)
    print(f"Opened: {path}")

    for page in range(doc.page_count):
        # Form fields
        fields = doc.get_form_fields(page)
        if fields:
            print(f"\n--- Form Fields (page {page + 1}) ---")
            for f in fields:
                print(f'  Name: {f.name!r:<20} Type: {f.type:<12} '
                      f'Value: {f.value!r:<16} Required: {f.required}')

        # Annotations
        annotations = doc.get_annotations(page)
        if annotations:
            print(f"\n--- Annotations (page {page + 1}) ---")
            for a in annotations:
                print(f'  Type: {a.type:<14} Page: {page + 1}   '
                      f'Contents: "{a.contents or ""}"')

if __name__ == "__main__":
    main()
