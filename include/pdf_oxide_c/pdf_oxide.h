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
int   pdf_document_sign(void* document_handle, const void* certificate_handle, const char* reason, const char* location, int* error_code);
int32_t pdf_document_get_signature_count(const void* document_handle, int* error_code);
void* pdf_document_get_signature(const void* document_handle, int32_t index, int* error_code);
int   pdf_signature_verify(const void* signature_handle, int* error_code);
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

#ifdef __cplusplus
}
#endif

#endif /* PDF_OXIDE_H */
