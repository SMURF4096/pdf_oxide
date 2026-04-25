/**
 * pdf_oxide C API - v0.3.24
 *
 * C-compatible Foreign Function Interface for pdf_oxide.
 * Used by Go (CGO), Node.js (N-API), and C# (P/Invoke) bindings.
 *
 * Error Convention:
 *   Most functions accept an `int* error_code` out-parameter.
 *   0 = success, 1 = invalid arg, 2 = IO error, 3 = parse error,
 *   4 = extraction failed, 5 = internal error, 6 = invalid page index,
 *   7 = search error, 8 = unsupported feature.
 *
 * Memory Convention:
 *   - Strings returned as `char*` must be freed with `free_string()`.
 *   - Byte buffers returned as `uint8_t*` must be freed with `free_bytes()`.
 *   - Opaque handles must be freed with their corresponding `*_free()` function.
 */

#ifndef PDF_OXIDE_H
#define PDF_OXIDE_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ─── Error codes ─────────────────────────────────────────────────────────── */

#define PDF_ERROR_SUCCESS          0
#define PDF_ERROR_INVALID_ARG      1
#define PDF_ERROR_IO_ERROR         2
#define PDF_ERROR_PARSE_ERROR      3
#define PDF_ERROR_EXTRACTION       4
#define PDF_ERROR_INTERNAL         5
#define PDF_ERROR_INVALID_PAGE     6
#define PDF_ERROR_SEARCH           7
#define PDF_ERROR_UNSUPPORTED      8

/* ─── Memory management ──────────────────────────────────────────────────── */

void free_string(char* ptr);
void free_bytes(void* ptr);
char* AllocString(const char* s);

/* ─── PdfDocument ────────────────────────────────────────────────────────── */

void* pdf_document_open(const char* path, int* error_code);
void  pdf_document_free(void* handle);
int32_t pdf_document_get_page_count(void* handle, int* error_code);
void  pdf_document_get_version(const void* handle, uint8_t* major, uint8_t* minor);
bool  pdf_document_has_structure_tree(void* handle);
char* pdf_document_extract_text(void* handle, int32_t page_index, int* error_code);
char* pdf_document_to_markdown(void* handle, int32_t page_index, int* error_code);
char* pdf_document_to_html(void* handle, int32_t page_index, int* error_code);
char* pdf_document_to_plain_text(void* handle, int32_t page_index, int* error_code);
char* pdf_document_to_markdown_all(void* handle, int* error_code);

/* ─── DocumentEditor ─────────────────────────────────────────────────────── */

void* document_editor_open(const char* path, int* error_code);
void  document_editor_free(void* handle);
bool  document_editor_is_modified(const void* handle);
char* document_editor_get_source_path(const void* handle, int* error_code);
void  document_editor_get_version(const void* handle, uint8_t* major, uint8_t* minor);
int32_t document_editor_get_page_count(void* handle, int* error_code);
char* document_editor_get_title(const void* handle, int* error_code);
int   document_editor_set_title(void* handle, const char* title, int* error_code);
char* document_editor_get_author(const void* handle, int* error_code);
int   document_editor_set_author(void* handle, const char* author, int* error_code);
char* document_editor_get_subject(const void* handle, int* error_code);
int   document_editor_set_subject(void* handle, const char* subject, int* error_code);
char* document_editor_get_producer(const void* handle, int* error_code);
int   document_editor_set_producer(void* handle, const char* producer, int* error_code);
char* document_editor_get_creation_date(const void* handle, int* error_code);
int   document_editor_set_creation_date(void* handle, const char* date_str, int* error_code);
int   document_editor_save(void* handle, const char* path, int* error_code);

/* Open from bytes */
void* document_editor_open_from_bytes(const uint8_t* data, size_t len, int* error_code);

/* Save to bytes (returns buffer freed with free_bytes; *out_len receives size) */
uint8_t* document_editor_save_to_bytes(void* handle, size_t* out_len, int* error_code);
uint8_t* document_editor_save_to_bytes_with_options(void* handle, bool compress, bool garbage_collect, bool linearize, size_t* out_len, int* error_code);

/* Keywords */
char* document_editor_get_keywords(const void* handle, int* error_code);
int   document_editor_set_keywords(void* handle, const char* keywords, int* error_code);

/* Merge from bytes */
int   document_editor_merge_from_bytes(void* handle, const uint8_t* data, size_t len, int* error_code);

/* Embed file attachment */
int   document_editor_embed_file(void* handle, const char* name, const uint8_t* data, size_t len, int* error_code);

/* Redactions */
int   document_editor_apply_page_redactions(void* handle, size_t page, int* error_code);
int   document_editor_apply_all_redactions(void* handle, int* error_code);

/* Rotation helpers */
int   document_editor_rotate_all_pages(void* handle, int32_t degrees, int* error_code);
int   document_editor_rotate_page_by(void* handle, size_t page, int32_t degrees, int* error_code);

/* MediaBox editor variants */
int   document_editor_get_page_media_box(void* handle, size_t page, double* x, double* y, double* w, double* h, int* error_code);
int   document_editor_set_page_media_box(void* handle, size_t page, double x, double y, double w, double h, int* error_code);

/* CropBox editor variants */
int   document_editor_get_page_crop_box(void* handle, size_t page, double* x, double* y, double* w, double* h, int* error_code);
int   document_editor_set_page_crop_box(void* handle, size_t page, double x, double y, double w, double h, int* error_code);

/* Bulk erase regions (rects is flat [x,y,w,h,...] array; rects_count = number of rects) */
int   document_editor_erase_regions(void* handle, size_t page, const double* rects, size_t rects_count, int* error_code);
int   document_editor_clear_erase_regions(void* handle, size_t page, int* error_code);

/* Page-mark state queries */
int32_t document_editor_is_page_marked_for_flatten(const void* handle, size_t page);
int   document_editor_unmark_page_for_flatten(void* handle, size_t page, int* error_code);
int32_t document_editor_is_page_marked_for_redaction(const void* handle, size_t page);
int   document_editor_unmark_page_for_redaction(void* handle, size_t page, int* error_code);

/* Form flattening */
int   document_editor_flatten_forms(void* handle, int* error_code);
int   document_editor_flatten_forms_on_page(void* handle, int32_t page_index, int* error_code);
/* Returns number of warnings from the last flatten save; -1 if handle is null */
int32_t document_editor_flatten_warnings_count(const void* handle);
/* Returns the index-th warning as a C string (free with free_string); null on error */
char* document_editor_flatten_warning(const void* handle, int32_t index, int* error_code);

/* ─── PDF Creator ────────────────────────────────────────────────────────── */

void* pdf_from_markdown(const char* markdown, int* error_code);
void* pdf_from_html(const char* html, int* error_code);
void* pdf_from_text(const char* text, int* error_code);
int   pdf_save(void* handle, const char* path, int* error_code);
void* pdf_save_to_bytes(void* handle, int* data_len, int* error_code);
int32_t pdf_get_page_count(void* handle, int* error_code);
void  pdf_free(void* handle);

/* ─── Search ─────────────────────────────────────────────────────────────── */

void* pdf_document_search_page(void* handle, int32_t page_index, const char* search_term, bool case_sensitive, int* error_code);
void* pdf_document_search_all(void* handle, const char* search_term, bool case_sensitive, int* error_code);
int32_t pdf_oxide_search_result_count(const void* results);
char* pdf_oxide_search_result_get_text(const void* results, int32_t index, int* error_code);
int32_t pdf_oxide_search_result_get_page(const void* results, int32_t index, int* error_code);
void  pdf_oxide_search_result_get_bbox(const void* results, int32_t index, float* x, float* y, float* width, float* height, int* error_code);
void  pdf_oxide_search_result_free(void* handle);

/* ─── Font extraction ────────────────────────────────────────────────────── */

void* pdf_document_get_embedded_fonts(void* handle, int32_t page_index, int* error_code);
int32_t pdf_oxide_font_count(const void* fonts);
char* pdf_oxide_font_get_name(const void* fonts, int32_t index, int* error_code);
char* pdf_oxide_font_get_type(const void* fonts, int32_t index, int* error_code);
char* pdf_oxide_font_get_encoding(const void* fonts, int32_t index, int* error_code);
int   pdf_oxide_font_is_embedded(const void* fonts, int32_t index, int* error_code);
int   pdf_oxide_font_is_subset(const void* fonts, int32_t index, int* error_code);
float pdf_oxide_font_get_size(const void* fonts, int32_t index, int* error_code);
void  pdf_oxide_font_list_free(void* handle);

/* ─── Image extraction ───────────────────────────────────────────────────── */

void* pdf_document_get_embedded_images(void* handle, int32_t page_index, int* error_code);
int32_t pdf_oxide_image_count(const void* images);
int32_t pdf_oxide_image_get_width(const void* images, int32_t index, int* error_code);
int32_t pdf_oxide_image_get_height(const void* images, int32_t index, int* error_code);
char* pdf_oxide_image_get_format(const void* images, int32_t index, int* error_code);
char* pdf_oxide_image_get_colorspace(const void* images, int32_t index, int* error_code);
int32_t pdf_oxide_image_get_bits_per_component(const void* images, int32_t index, int* error_code);
void* pdf_oxide_image_get_data(const void* images, int32_t index, int* data_len, int* error_code);
void  pdf_oxide_image_list_free(void* handle);

/* ─── Annotations ────────────────────────────────────────────────────────── */

void* pdf_document_get_page_annotations(void* handle, int32_t page_index, int* error_code);
int32_t pdf_oxide_annotation_count(const void* annotations);
char* pdf_oxide_annotation_get_type(const void* annotations, int32_t index, int* error_code);
char* pdf_oxide_annotation_get_content(const void* annotations, int32_t index, int* error_code);
void  pdf_oxide_annotation_get_rect(const void* annotations, int32_t index, float* x, float* y, float* width, float* height, int* error_code);
void  pdf_oxide_annotation_list_free(void* handle);

/* Advanced annotation accessors */
char* pdf_oxide_annotation_get_subtype(const void* annotations, int32_t index, int* error_code);
bool  pdf_oxide_annotation_is_marked_deleted(const void* annotations, int32_t index, int* error_code);
int64_t pdf_oxide_annotation_get_creation_date(const void* annotations, int32_t index, int* error_code);
int64_t pdf_oxide_annotation_get_modification_date(const void* annotations, int32_t index, int* error_code);
char* pdf_oxide_annotation_get_author(const void* annotations, int32_t index, int* error_code);
float pdf_oxide_annotation_get_border_width(const void* annotations, int32_t index, int* error_code);
uint32_t pdf_oxide_annotation_get_color(const void* annotations, int32_t index, int* error_code);
bool  pdf_oxide_annotation_is_hidden(const void* annotations, int32_t index, int* error_code);
bool  pdf_oxide_annotation_is_printable(const void* annotations, int32_t index, int* error_code);
bool  pdf_oxide_annotation_is_read_only(const void* annotations, int32_t index, int* error_code);
char* pdf_oxide_link_annotation_get_uri(const void* annotations, int32_t index, int* error_code);
char* pdf_oxide_text_annotation_get_icon_name(const void* annotations, int32_t index, int* error_code);
int32_t pdf_oxide_highlight_annotation_get_quad_points_count(const void* annotations, int32_t index, int* error_code);
void  pdf_oxide_highlight_annotation_get_quad_point(const void* annotations, int32_t index, int32_t quad_index, float* x1, float* y1, float* x2, float* y2, float* x3, float* y3, float* x4, float* y4, int* error_code);

/* ─── Page operations ────────────────────────────────────────────────────── */

float pdf_page_get_width(void* handle, int32_t page_index, int* error_code);
float pdf_page_get_height(void* handle, int32_t page_index, int* error_code);
int32_t pdf_page_get_rotation(void* handle, int32_t page_index, int* error_code);
void  pdf_page_get_media_box(void* handle, int32_t page_index, float* x, float* y, float* width, float* height, int* error_code);
void  pdf_page_get_crop_box(void* handle, int32_t page_index, float* x, float* y, float* width, float* height, int* error_code);
void  pdf_page_get_art_box(void* handle, int32_t page_index, float* x, float* y, float* width, float* height, int* error_code);
void  pdf_page_get_bleed_box(void* handle, int32_t page_index, float* x, float* y, float* width, float* height, int* error_code);
void  pdf_page_get_trim_box(void* handle, int32_t page_index, float* x, float* y, float* width, float* height, int* error_code);

/* Page elements */
void* pdf_page_get_elements(void* handle, int32_t page_index, int* error_code);
int32_t pdf_oxide_element_count(const void* elements);
char* pdf_oxide_element_get_type(const void* elements, int32_t index, int* error_code);
char* pdf_oxide_element_get_text(const void* elements, int32_t index, int* error_code);
void  pdf_oxide_element_get_rect(const void* elements, int32_t index, float* x, float* y, float* width, float* height, int* error_code);
void  pdf_oxide_elements_free(void* handle);

/* ─── Barcodes (feature-gated, stubs return UNSUPPORTED) ────────────────── */

void* pdf_generate_qr_code(const char* data, int error_correction, int32_t size_px, int* error_code);
void* pdf_generate_barcode(const char* data, int format, int32_t size_px, int* error_code);
uint8_t* pdf_barcode_get_image_png(const void* barcode_handle, int32_t size_px, int32_t* out_len, int* error_code);
char* pdf_barcode_get_svg(const void* barcode_handle, int32_t size_px, int* error_code);
int   pdf_add_barcode_to_page(void* document_handle, int32_t page_index, const void* barcode_handle, float x, float y, float width, float height, int* error_code);
int   pdf_barcode_get_format(const void* barcode_handle, int* error_code);
char* pdf_barcode_get_data(const void* barcode_handle, int* error_code);
float pdf_barcode_get_confidence(const void* barcode_handle, int* error_code);
void  pdf_barcode_free(void* handle);

/* ─── Signatures (feature-gated, stubs return UNSUPPORTED) ──────────────── */

void* pdf_certificate_load_from_bytes(const uint8_t* cert_bytes, int32_t cert_len, const char* password, int* error_code);
void* pdf_certificate_load_from_pem(const char* cert_pem, const char* key_pem, int* error_code);
int      pdf_document_sign(void* document_handle, const void* certificate_handle, const char* reason, const char* location, int* error_code);
uint8_t* pdf_sign_bytes(const uint8_t* pdf_data, size_t pdf_len, const void* certificate_handle, const char* reason, const char* location, size_t* out_len, int* error_code);
int32_t pdf_document_get_signature_count(const void* document_handle, int* error_code);
void* pdf_document_get_signature(const void* document_handle, int32_t index, int* error_code);
int   pdf_signature_verify(const void* signature_handle, int* error_code);
int   pdf_signature_verify_detached(const void* signature_handle, const uint8_t* pdf_data, size_t pdf_len, int* error_code);
int   pdf_document_verify_all_signatures(const void* document_handle, int* error_code);
char* pdf_signature_get_signer_name(const void* signature_handle, int* error_code);
int64_t pdf_signature_get_signing_time(const void* signature_handle, int* error_code);
char* pdf_signature_get_signing_reason(const void* signature_handle, int* error_code);
char* pdf_signature_get_signing_location(const void* signature_handle, int* error_code);
void* pdf_signature_get_certificate(const void* signature_handle, int* error_code);
char* pdf_certificate_get_subject(const void* certificate_handle, int* error_code);
char* pdf_certificate_get_issuer(const void* certificate_handle, int* error_code);
char* pdf_certificate_get_serial(const void* certificate_handle, int* error_code);
void  pdf_certificate_get_validity(const void* certificate_handle, int64_t* not_before, int64_t* not_after, int* error_code);
int   pdf_certificate_is_valid(const void* certificate_handle, int* error_code);
void  pdf_signature_free(void* handle);
void  pdf_certificate_free(void* handle);

/* ─── Rendering (feature-gated, stubs return UNSUPPORTED) ───────────────── */

int32_t pdf_estimate_render_time(const void* document_handle, int32_t page_index, int* error_code);
void* pdf_create_renderer(int32_t dpi, int32_t format, int32_t quality, bool anti_alias, int* error_code);
void* pdf_render_page(void* document_handle, int32_t page_index, int32_t format, int* error_code);
void* pdf_render_page_region(void* document_handle, int32_t page_index, float crop_x, float crop_y, float crop_width, float crop_height, int32_t format, int* error_code);
void* pdf_render_page_zoom(void* document_handle, int32_t page_index, float zoom_level, int32_t format, int* error_code);
void* pdf_render_page_fit(void* document_handle, int32_t page_index, int32_t fit_width, int32_t fit_height, int32_t format, int* error_code);
void* pdf_render_page_thumbnail(void* document_handle, int32_t page_index, int32_t thumbnail_size, int32_t format, int* error_code);
int32_t pdf_get_rendered_image_width(const void* image_handle, int* error_code);
int32_t pdf_get_rendered_image_height(const void* image_handle, int* error_code);
void* pdf_get_rendered_image_data(const void* image_handle, int32_t* data_len, int* error_code);
int   pdf_save_rendered_image(const void* image_handle, const char* file_path, int* error_code);
void  pdf_rendered_image_free(void* handle);
void  pdf_renderer_free(void* handle);

/* ─── TSA (Time Stamp Authority) ────────────────────────────────────────── */

void* pdf_tsa_client_create(const char* url, const char* username, const char* password, int32_t timeout, int32_t hash_algo, bool use_nonce, bool cert_req, int* error_code);
void  pdf_tsa_client_free(void* client);
void* pdf_tsa_request_timestamp(const void* client, const uint8_t* data, size_t data_len, int* error_code);
void* pdf_tsa_request_timestamp_hash(const void* client, const uint8_t* hash, size_t hash_len, int32_t hash_algo, int* error_code);
const uint8_t* pdf_timestamp_get_token(const void* timestamp, size_t* out_len, int* error_code);
int64_t pdf_timestamp_get_time(const void* timestamp, int* error_code);
char* pdf_timestamp_get_serial(const void* timestamp, int* error_code);
char* pdf_timestamp_get_tsa_name(const void* timestamp, int* error_code);
char* pdf_timestamp_get_policy_oid(const void* timestamp, int* error_code);
int32_t pdf_timestamp_get_hash_algorithm(const void* timestamp, int* error_code);
const uint8_t* pdf_timestamp_get_message_imprint(const void* timestamp, size_t* out_len, int* error_code);
bool  pdf_timestamp_verify(const void* timestamp, int* error_code);
void  pdf_timestamp_free(void* timestamp);
bool  pdf_signature_add_timestamp(const void* signature, const void* timestamp, int* error_code);
bool  pdf_signature_has_timestamp(const void* signature, int* error_code);
void* pdf_signature_get_timestamp(const void* signature, int* error_code);
bool  pdf_add_timestamp(const uint8_t* pdf_data, size_t pdf_len, int32_t signature_index, const char* tsa_url, uint8_t** out_data, size_t* out_len, int* error_code);

/* ─── PDF/UA Validation ─────────────────────────────────────────────────── */

void* pdf_validate_pdf_ua(const void* document, int32_t level, int* error_code);
bool  pdf_pdf_ua_is_accessible(const void* results, int* error_code);
int32_t pdf_pdf_ua_error_count(const void* results);
void* pdf_pdf_ua_get_error(const void* results, int32_t index, int* error_code);
int32_t pdf_pdf_ua_warning_count(const void* results);
void* pdf_pdf_ua_get_warning(const void* results, int32_t index, int* error_code);
bool  pdf_pdf_ua_get_stats(const void* results, int32_t* out_struct, int32_t* out_images, int32_t* out_tables, int32_t* out_forms, int32_t* out_annotations, int32_t* out_pages, int* error_code);
void  pdf_pdf_ua_results_free(void* results);

/* ─── FDF/XFDF Import/Export ────────────────────────────────────────────── */

bool  pdf_form_import_from_file(const void* document, const char* filename, int* error_code);
int32_t pdf_document_import_form_data(const void* document, const char* data_path, int* error_code);
int32_t pdf_editor_import_fdf_bytes(const void* document, const uint8_t* data, size_t data_len, int* error_code);
int32_t pdf_editor_import_xfdf_bytes(const void* document, const uint8_t* data, size_t data_len, int* error_code);
uint8_t* pdf_document_export_form_data_to_bytes(const void* document, int32_t format_type, size_t* out_len, int* error_code);

/* ─── C# PascalCase aliases ─────────────────────────────────────────────── */

void* PdfDocumentOpen(const char* path, int* error_code);
void  PdfDocumentFree(void* handle);
int32_t PdfDocumentGetPageCount(void* handle, int* error_code);
char* PdfDocumentExtractText(void* handle, int32_t page_index, int* error_code);
char* PdfDocumentToMarkdown(void* handle, int32_t page_index, int* error_code);
char* PdfDocumentToHtml(void* handle, int32_t page_index, int* error_code);
char* PdfDocumentToPlainText(void* handle, int32_t page_index, int* error_code);
void* PdfFromMarkdown(const char* markdown, int* error_code);
void* PdfFromHtml(const char* html, int* error_code);
void* PdfFromText(const char* text, int* error_code);
int   PdfSave(void* handle, const char* path, int* error_code);
void* PdfSaveToBytes(void* handle, int* data_len, int* error_code);
void  PdfFree(void* handle);
void* DocumentEditorOpen(const char* path, int* error_code);
void  DocumentEditorFree(void* handle);
int   DocumentEditorSave(void* handle, const char* path, int* error_code);
int   DocumentEditorSetTitle(void* handle, const char* value, int* error_code);
int   DocumentEditorSetAuthor(void* handle, const char* value, int* error_code);
void  FreeString(char* ptr);
void  FreeBytes(void* ptr);

/* ─── Logging ────────────────────────────────────────────────────────────── */
/** Set log level: 0=Off, 1=Error, 2=Warn, 3=Info, 4=Debug, 5=Trace */
void  pdf_oxide_set_log_level(int level);
/** Get current log level (0-5). */
int   pdf_oxide_get_log_level(void);

/* ─── Write-side API ─────────────────────────────────────────────────────── *
 *
 * DocumentBuilder + PageBuilder + EmbeddedFont mirror the Rust fluent
 * builder. Error conventions depend on the return type:
 *   - `int`-returning methods: 0 on success, -1 on error.
 *   - Pointer-returning methods (`void*` handles, `uint8_t*` byte buffers):
 *     a valid pointer on success, NULL on error.
 *   - `void`-returning methods: no return-side failure channel (free fns).
 * In every case the `error_code` out-param carries the specific code.
 *
 * Handle-lifetime contract:
 *
 *  1. `pdf_document_builder_create` returns a handle. The finalisation
 *     methods below consume the *builder state* but leave the handle
 *     itself allocated (a zombie wrapper). Callers MUST still call
 *     `pdf_document_builder_free` to release it:
 *        pdf_document_builder_build        → call `_free` after
 *        pdf_document_builder_save         → call `_free` after
 *        pdf_document_builder_save_encrypted → call `_free` after
 *        pdf_document_builder_to_bytes_encrypted → call `_free` after
 *     A second finalisation on the same handle (double-build, etc.)
 *     is invalid; it returns NULL / -1 with errno = invalid-arg.
 *     Calling `pdf_document_builder_free` on an already-finalised
 *     handle is the normal teardown path and is always safe.
 *
 *  2. `pdf_document_builder_a4_page` / `_letter_page` / `_page` returns
 *     a page sub-handle. Only ONE page handle may be outstanding per
 *     builder; a second call before the prior `pdf_page_builder_done`
 *     returns NULL with error_code = 1 (invalid arg).
 *
 *  3. `pdf_page_builder_done` commits the page and invalidates the
 *     page handle. `pdf_page_builder_free` is for error-recovery only
 *     (drop without committing).
 *
 *  4. `pdf_document_builder_register_embedded_font` CONSUMES the font
 *     handle on success. Do not call `pdf_embedded_font_free` after.
 *
 *  5. Byte buffers returned from `_build` and `_to_bytes_encrypted`
 *     must be freed with `free_bytes`.
 */

/* EmbeddedFont */
void* pdf_embedded_font_from_file(const char* path, int* error_code);
void* pdf_embedded_font_from_bytes(const uint8_t* data, size_t len,
                                   const char* name /* nullable */,
                                   int* error_code);
void  pdf_embedded_font_free(void* handle);

/* DocumentBuilder — lifecycle */
void* pdf_document_builder_create(int* error_code);
void  pdf_document_builder_free(void* handle);

/* DocumentBuilder — metadata */
int   pdf_document_builder_set_title(void* handle, const char* title, int* error_code);
int   pdf_document_builder_set_author(void* handle, const char* author, int* error_code);
int   pdf_document_builder_set_subject(void* handle, const char* subject, int* error_code);
int   pdf_document_builder_set_keywords(void* handle, const char* keywords, int* error_code);
int   pdf_document_builder_set_creator(void* handle, const char* creator, int* error_code);
int   pdf_document_builder_on_open(void* handle, const char* script, int* error_code);

/* DocumentBuilder — PDF/UA-1 accessibility (Bundle F-1/F-2/F-4) */
int   pdf_document_builder_tagged_pdf_ua1(void* handle, int* error_code);
int   pdf_document_builder_language(void* handle, const char* lang, int* error_code);
int   pdf_document_builder_role_map(void* handle, const char* custom,
                                    const char* standard, int* error_code);

/* DocumentBuilder — font registration (CONSUMES font on success) */
int   pdf_document_builder_register_embedded_font(void* handle, const char* name,
                                                  void* font, int* error_code);

/* DocumentBuilder — open page */
void* pdf_document_builder_a4_page(void* handle, int* error_code);
void* pdf_document_builder_letter_page(void* handle, int* error_code);
void* pdf_document_builder_page(void* handle, float width, float height, int* error_code);

/* PageBuilder — content */
int   pdf_page_builder_font(void* page, const char* name, float size, int* error_code);
int   pdf_page_builder_at(void* page, float x, float y, int* error_code);
int   pdf_page_builder_text(void* page, const char* text, int* error_code);
int   pdf_page_builder_heading(void* page, unsigned char level, const char* text,
                               int* error_code);
int   pdf_page_builder_paragraph(void* page, const char* text, int* error_code);
int   pdf_page_builder_space(void* page, float points, int* error_code);
int   pdf_page_builder_horizontal_rule(void* page, int* error_code);

/* PageBuilder — annotations (attach to previous text element) */
int   pdf_page_builder_link_url(void* page, const char* url, int* error_code);
int   pdf_page_builder_link_page(void* page, size_t target_page, int* error_code);
int   pdf_page_builder_link_named(void* page, const char* destination, int* error_code);
int   pdf_page_builder_link_javascript(void* page, const char* script, int* error_code);
int   pdf_page_builder_on_open(void* page, const char* script, int* error_code);
int   pdf_page_builder_on_close(void* page, const char* script, int* error_code);
/* Field validation AA dict — call after text_field / combo_box / list_box */
int   pdf_page_builder_field_keystroke(void* page, const char* script, int* error_code);
int   pdf_page_builder_field_format(void* page, const char* script, int* error_code);
int   pdf_page_builder_field_validate(void* page, const char* script, int* error_code);
int   pdf_page_builder_field_calculate(void* page, const char* script, int* error_code);
int   pdf_page_builder_highlight(void* page, float r, float g, float b, int* error_code);
int   pdf_page_builder_underline(void* page, float r, float g, float b, int* error_code);
int   pdf_page_builder_strikeout(void* page, float r, float g, float b, int* error_code);
int   pdf_page_builder_squiggly(void* page, float r, float g, float b, int* error_code);
int   pdf_page_builder_sticky_note(void* page, const char* text, int* error_code);
int   pdf_page_builder_sticky_note_at(void* page, float x, float y, const char* text,
                                      int* error_code);
int   pdf_page_builder_watermark(void* page, const char* text, int* error_code);
int   pdf_page_builder_watermark_confidential(void* page, int* error_code);
int   pdf_page_builder_watermark_draft(void* page, int* error_code);
int   pdf_page_builder_stamp(void* page, const char* type_name, int* error_code);
int   pdf_page_builder_freetext(void* page, float x, float y, float w, float h,
                                const char* text, int* error_code);

/* Form-field widget creation. Each method adds a widget to the page's
 * /AcroForm entry at finalize time. `default_value` may be NULL for a
 * blank field. */
int   pdf_page_builder_text_field(void* page, const char* name,
                                  float x, float y, float w, float h,
                                  const char* default_value /* nullable */,
                                  int* error_code);
int   pdf_page_builder_checkbox(void* page, const char* name,
                                float x, float y, float w, float h,
                                int checked,
                                int* error_code);

/* Dropdown combo-box with a fixed list of string options.
 * `options` is an array of `options_count` C-strings.
 * `selected` may be NULL for no initial selection. */
int   pdf_page_builder_combo_box(void* page, const char* name,
                                 float x, float y, float w, float h,
                                 const char* const* options,
                                 size_t options_count,
                                 const char* selected /* nullable */,
                                 int* error_code);

/* Radio-button group. `values`, `xs`, `ys`, `ws`, `hs` are parallel
 * arrays of length `count`. `selected` may be NULL. */
int   pdf_page_builder_radio_group(void* page, const char* name,
                                   const char* const* values,
                                   const float* xs, const float* ys,
                                   const float* ws, const float* hs,
                                   size_t count,
                                   const char* selected /* nullable */,
                                   int* error_code);

/* Clickable push button with a visible caption. */
int   pdf_page_builder_push_button(void* page, const char* name,
                                   float x, float y, float w, float h,
                                   const char* caption,
                                   int* error_code);
int   pdf_page_builder_signature_field(void* page, const char* name,
                                       float x, float y, float w, float h,
                                       int* error_code);

/* Footnote: inline ref mark + page-end body text with separator line. */
int   pdf_page_builder_footnote(void* page, const char* ref_mark,
                                const char* note_text, int* error_code);

/* Multi-column text flow: wrap `text` into `column_count` balanced columns
 * with `gap_pt` points between them, starting at the current cursor. */
int   pdf_page_builder_columns(void* page, unsigned int column_count,
                               float gap_pt, const char* text,
                               int* error_code);

/* Rich text inline runs — advance cursor_x only (no y-advance). */
int   pdf_page_builder_inline(void* page, const char* text, int* error_code);
int   pdf_page_builder_inline_bold(void* page, const char* text, int* error_code);
int   pdf_page_builder_inline_italic(void* page, const char* text, int* error_code);
int   pdf_page_builder_inline_color(void* page, float r, float g, float b,
                                    const char* text, int* error_code);
/* Advance cursor_y one line-height and reset cursor_x to 72 pt. */
int   pdf_page_builder_newline(void* page, int* error_code);

/* Barcode / QR-code image placement.
 * barcode_type: 0=Code128 1=Code39 2=EAN13 3=EAN8 4=UPCA 5=ITF 6=Code93 7=Codabar */
int   pdf_page_builder_barcode_1d(void* page, int barcode_type, const char* data,
                                  float x, float y, float w, float h,
                                  int* error_code);
int   pdf_page_builder_barcode_qr(void* page, const char* data,
                                  float x, float y, float size,
                                  int* error_code);

/* PDF/UA-1 image accessibility */
int   pdf_page_builder_image_with_alt(void* page,
                                      const uint8_t* bytes, size_t len,
                                      float x, float y, float w, float h,
                                      const char* alt_text, int* error_code);
int   pdf_page_builder_image_artifact(void* page,
                                      const uint8_t* bytes, size_t len,
                                      float x, float y, float w, float h,
                                      int* error_code);

/* Low-level graphics primitives (PdfWriter exposure) */
int   pdf_page_builder_rect(void* page, float x, float y, float w, float h,
                            int* error_code);
int   pdf_page_builder_filled_rect(void* page, float x, float y, float w, float h,
                                   float r, float g, float b, int* error_code);
int   pdf_page_builder_line(void* page, float x1, float y1, float x2, float y2,
                            int* error_code);

/* v0.3.39 primitives for the buffered Table surface (#393). */

/* Stroked rectangle outline with caller-supplied width + colour. */
int   pdf_page_builder_stroke_rect(void* page, float x, float y, float w, float h,
                                   float width, float r, float g, float b,
                                   int* error_code);
/* Straight line with caller-supplied width + colour. */
int   pdf_page_builder_stroke_line(void* page, float x1, float y1, float x2, float y2,
                                   float width, float r, float g, float b,
                                   int* error_code);

/* Place wrapped text inside a rectangle with horizontal alignment.
 * `align`: 0=Left, 1=Center, 2=Right. Anything else treated as Left. */
int   pdf_page_builder_text_in_rect(void* page, float x, float y, float w, float h,
                                    const char* text, int align,
                                    int* error_code);

/* Transition to a new page with the SAME dimensions. text_config
 * carries over; cursor resets to the top-left margin. Does not
 * re-draw any header — callers wanting header-repeat-on-break must
 * re-emit explicitly. */
int   pdf_page_builder_new_page_same_size(void* page, int* error_code);

/* Place a buffered table at the current cursor.
 *
 *  n_columns:    column count.
 *  widths:       array of length n_columns — column width in points.
 *  aligns:       array of length n_columns — per-column alignment
 *                (0=Left, 1=Center, 2=Right).
 *  n_rows:       number of rows (not counting the header implicitly
 *                — if has_header != 0 the first row in `cell_strings`
 *                becomes the header).
 *  cell_strings: row-major array of C strings, length n_rows * n_columns.
 *                Each pointer must be a valid UTF-8 C string. NULL
 *                pointers are treated as empty strings.
 *  has_header:   0 = body-only, non-0 = promote first row to header
 *                (bold + default header background).
 *
 * Returns 0 on success, -1 on failure. Streaming (O(cols) memory)
 * is tracked separately; this buffers the whole row matrix. */
int   pdf_page_builder_table(void* page,
                             size_t n_columns,
                             const float* widths,
                             const int* aligns,
                             size_t n_rows,
                             const char* const* cell_strings,
                             int has_header,
                             int* error_code);

/* Streaming table (row-at-a-time, true O(cols) memory at the Rust core).
 * FFI layer currently buffers rows between _begin and _finish, replaying
 * them against a live StreamingTable on _done. Full FFI row-by-row
 * streaming is tracked under issue #400 for v0.3.40. */

/* Open a streaming table. `headers`, `widths`, `aligns` are parallel
 * arrays of length n_columns (aligns: 0=Left, 1=Center, 2=Right). */
int   pdf_page_builder_streaming_table_begin(void* page,
                                             size_t n_columns,
                                             const char* const* headers,
                                             const float* widths,
                                             const int* aligns,
                                             int repeat_header,
                                             int* error_code);

/* Same as _begin but exposes column-sizing mode.
 * mode: 0=Fixed, 1=Sample(sample_rows,min_w,max_w), 2=AutoAll (error). */
int   pdf_page_builder_streaming_table_begin_v2(void* page,
                                                size_t n_columns,
                                                const char* const* headers,
                                                const float* widths,
                                                const int* aligns,
                                                int repeat_header,
                                                int mode,
                                                size_t sample_rows,
                                                float min_col_width_pt,
                                                float max_col_width_pt,
                                                size_t max_rowspan,
                                                int* error_code);

/* Push one row (all rowspan=1). `cells` must have length matching
 * n_columns from _begin. NULL cell pointers become empty strings. */
int   pdf_page_builder_streaming_table_push_row(void* page,
                                                size_t n_cells,
                                                const char* const* cells,
                                                int* error_code);

/* Push one row with per-cell rowspan values. `rowspans` is a length-
 * n_cells array of size_t values (1 = normal, >=2 = span). Pass NULL
 * to treat all cells as rowspan=1. */
int   pdf_page_builder_streaming_table_push_row_v2(void* page,
                                                   size_t n_cells,
                                                   const char* const* cells,
                                                   const size_t* rowspans,
                                                   int* error_code);

/* Close the open streaming table. Auto-closed by _done if not
 * explicit. */
int   pdf_page_builder_streaming_table_finish(void* page, int* error_code);

/* PageBuilder — commit / drop */
int   pdf_page_builder_done(void* page, int* error_code);
void  pdf_page_builder_free(void* page);

/* DocumentBuilder — finalisation. Each consumes the builder *state*
 * but leaves the handle wrapper allocated; callers must still call
 * `pdf_document_builder_free` to release it (see handle-lifetime
 * contract above). Calling any of these twice on the same handle
 * returns an error. */
uint8_t* pdf_document_builder_build(void* handle, size_t* out_len, int* error_code);
int      pdf_document_builder_save(void* handle, const char* path, int* error_code);
int      pdf_document_builder_save_encrypted(void* handle, const char* path,
                                             const char* user_password,
                                             const char* owner_password,
                                             int* error_code);
uint8_t* pdf_document_builder_to_bytes_encrypted(void* handle,
                                                 const char* user_password,
                                                 const char* owner_password,
                                                 size_t* out_len,
                                                 int* error_code);

/* HTML+CSS pipeline */
void* pdf_from_html_css(const char* html, const char* css,
                        const uint8_t* font_bytes, size_t font_len,
                        int* error_code);
/* Multi-font cascade — `families`, `font_bytes`, and `font_lens` are
 * parallel arrays of length `count`. The FFI copies the bytes. */
void* pdf_from_html_css_with_fonts(const char* html, const char* css,
                                   const char* const* families,
                                   const uint8_t* const* font_bytes,
                                   const size_t* font_lens,
                                   size_t count,
                                   int* error_code);

#ifdef __cplusplus
}
#endif

#endif /* PDF_OXIDE_H */
