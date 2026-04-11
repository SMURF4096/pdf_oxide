# Open a PDF, modify metadata, delete a page, and save.
# Run: python main.py input.pdf output.pdf

import sys

from pdf_oxide import DocumentEditor


def main():
    if len(sys.argv) < 3:
        print("Usage: python main.py <input.pdf> <output.pdf>")
        sys.exit(1)

    input_path, output_path = sys.argv[1], sys.argv[2]

    editor = DocumentEditor(input_path)
    print(f"Opened: {input_path}")

    editor.set_title("Edited Document")
    print('Set title: "Edited Document"')

    editor.set_author("pdf_oxide")
    print('Set author: "pdf_oxide"')

    editor.delete_page(1)  # 0-indexed, deletes page 2
    print("Deleted page 2")

    editor.save(output_path)
    print(f"Saved: {output_path}")

if __name__ == "__main__":
    main()
