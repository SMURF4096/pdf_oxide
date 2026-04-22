/**
 * DocumentEditor — thin TS wrapper around the N-API `editor*` exports
 * in `binding.cc`. Mirrors the C# `DocumentEditor` / Go
 * `DocumentEditor` surface (#384 gap K).
 *
 * Every mutation is a synchronous call into the Rust core; the same
 * handle is carried until {@link DocumentEditor.close}. Throws plain
 * `Error` with the native message on failure.
 *
 * ```ts
 * import { DocumentEditor } from 'pdf-oxide';
 *
 * const editor = DocumentEditor.open('in.pdf');
 * try {
 *   editor.mergeFrom('other.pdf');
 *   editor.deletePage(0);
 *   editor.movePage(0, 2);
 *   editor.setPageRotation(0, 90);
 *   editor.save('out.pdf');
 * } finally {
 *   editor.close();
 * }
 * ```
 */
// Load the native addon directly — going through `./index.js` would
// create an ESM require cycle (index.js imports us back). Mirrors the
// `createRequire` pattern used in `builders/document-builder.ts`.
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const native = require('../build/Release/pdf_oxide.node');

/**
 * Page rotation angles valid for {@link DocumentEditor.setPageRotation}.
 */
export type PageRotation = 0 | 90 | 180 | 270;

/**
 * PDF editor bound to a concrete file on disk. Open via
 * {@link DocumentEditor.open}; always pair with {@link close} (or use
 * `using` / the explicit-resource-management protocol once it's
 * stable in your toolchain).
 */
export class DocumentEditor {
  private _handle: any;
  private _closed = false;

  private constructor(handle: any) {
    this._handle = handle;
  }

  /** Open a PDF file for editing. */
  static open(path: string): DocumentEditor {
    if (typeof path !== 'string' || path.length === 0) {
      throw new TypeError('path must be a non-empty string');
    }
    const handle = native.editorOpen(path);
    return new DocumentEditor(handle);
  }

  /** True if the editor has been closed. Subsequent calls will throw. */
  get closed(): boolean {
    return this._closed;
  }

  private _throwIfClosed(): void {
    if (this._closed) throw new Error('DocumentEditor is closed');
  }

  /** Current page count. */
  pageCount(): number {
    this._throwIfClosed();
    return native.editorGetPageCount(this._handle);
  }

  /** True if the editor has unsaved modifications. */
  isModified(): boolean {
    this._throwIfClosed();
    return native.editorIsModified(this._handle);
  }

  // ----- metadata ---------------------------------------------------

  setTitle(title: string): void {
    this._throwIfClosed();
    native.editorSetTitle(this._handle, title);
  }

  setAuthor(author: string): void {
    this._throwIfClosed();
    native.editorSetAuthor(this._handle, author);
  }

  getProducer(): string {
    this._throwIfClosed();
    return native.editorGetProducer(this._handle);
  }

  setProducer(producer: string): void {
    this._throwIfClosed();
    // NOTE: today this is a no-op in Rust core (src/ffi.rs:532-586).
    // See task #70 for the core fix; the wrapper is in place so the
    // API surface matches Python / C# / Go.
    if (native.editorSetProducer) {
      native.editorSetProducer(this._handle, producer);
    }
  }

  getCreationDate(): string {
    this._throwIfClosed();
    return native.editorGetCreationDate(this._handle);
  }

  setCreationDate(date: string): void {
    this._throwIfClosed();
    // Same Rust-core stub note as setProducer.
    native.editorSetCreationDate(this._handle, date);
  }

  // ----- page mutations ---------------------------------------------

  /** Delete the page at `pageIndex` (zero-based). */
  deletePage(pageIndex: number): void {
    this._throwIfClosed();
    native.editorDeletePage(this._handle, pageIndex);
  }

  /** Move a page. Indices refer to positions before the move. */
  movePage(fromIndex: number, toIndex: number): void {
    this._throwIfClosed();
    native.editorMovePage(this._handle, fromIndex, toIndex);
  }

  /** Set rotation on a page (0/90/180/270). */
  setPageRotation(pageIndex: number, degrees: PageRotation): void {
    this._throwIfClosed();
    native.editorSetPageRotation(this._handle, pageIndex, degrees);
  }

  // ----- document-level mutations -----------------------------------

  /**
   * Append every page of another PDF to the end of this document.
   * Answers Reddit user u/Raccoon12's direct question (2026-04-21).
   */
  mergeFrom(sourcePath: string): void {
    this._throwIfClosed();
    if (typeof sourcePath !== 'string' || sourcePath.length === 0) {
      throw new TypeError('sourcePath must be a non-empty string');
    }
    native.editorMergeFrom(this._handle, sourcePath);
  }

  /** Flatten form fields across the entire document. */
  flattenForms(): void {
    this._throwIfClosed();
    native.editorFlattenForms(this._handle);
  }

  /** Flatten annotations. If `pageIndex` is omitted, flattens all pages. */
  flattenAnnotations(pageIndex?: number): void {
    this._throwIfClosed();
    if (pageIndex === undefined) {
      native.editorFlattenAnnotations(this._handle);
    } else {
      native.editorFlattenAnnotations(this._handle, pageIndex);
    }
  }

  /** Set a form field value by fully-qualified field name. */
  setFormFieldValue(fieldName: string, value: string): void {
    this._throwIfClosed();
    native.editorSetFormFieldValue(this._handle, fieldName, value);
  }

  /** Import an FDF file (bytes) into the document's form. */
  importFdfBytes(fdf: Buffer | Uint8Array): void {
    this._throwIfClosed();
    native.editorImportFdfBytes(this._handle, fdf);
  }

  /** Import an XFDF file (bytes) into the document's form. */
  importXfdfBytes(xfdf: Buffer | Uint8Array): void {
    this._throwIfClosed();
    native.editorImportXfdfBytes(this._handle, xfdf);
  }

  // ----- save paths -------------------------------------------------

  /** Save the document to `path`. */
  save(path: string): void {
    this._throwIfClosed();
    if (typeof path !== 'string' || path.length === 0) {
      throw new TypeError('path must be a non-empty string');
    }
    native.editorSave(this._handle, path);
  }

  /** Save with AES-256 encryption (user + owner passwords). */
  saveEncrypted(path: string, userPassword: string, ownerPassword: string): void {
    this._throwIfClosed();
    native.editorSaveEncrypted(this._handle, path, userPassword, ownerPassword);
  }

  // ----- lifecycle --------------------------------------------------

  /** Release the native handle. Safe to call multiple times. */
  close(): void {
    if (!this._closed) {
      if (native.editorFree) {
        native.editorFree(this._handle);
      }
      this._closed = true;
    }
  }
}

export default DocumentEditor;
