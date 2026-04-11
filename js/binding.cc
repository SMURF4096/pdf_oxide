#include <napi.h>
#include <string>
#include <cstring>
#include <cstdint>

// ============================================================
// External FFI declarations from Rust
// ============================================================

extern "C" {
  // Error codes enum
  enum PdfErrorCode {
    PDF_ERROR_SUCCESS = 0,
    PDF_ERROR_INVALID_ARG = 1,
    PDF_ERROR_IO_ERROR = 2,
    PDF_ERROR_PARSE_ERROR = 3,
    PDF_ERROR_NOT_FOUND = 4,
    PDF_ERROR_PERMISSION_DENIED = 5,
    PDF_ERROR_UNSUPPORTED = 6,
    PDF_ERROR_INTERNAL = 7
  };

  // Image format enum
  enum ImageFormat {
    IMAGE_FORMAT_PNG = 0,
    IMAGE_FORMAT_JPEG = 1,
    IMAGE_FORMAT_WEBP = 2
  };

  // PDF/A levels
  enum PdfALevel {
    PDF_A_LEVEL_1B = 0,
    PDF_A_LEVEL_1A = 1,
    PDF_A_LEVEL_2B = 2,
    PDF_A_LEVEL_2A = 3,
    PDF_A_LEVEL_2U = 4,
    PDF_A_LEVEL_3B = 5,
    PDF_A_LEVEL_3A = 6,
    PDF_A_LEVEL_3U = 7
  };

  // PDF/X levels
  enum PdfXLevel {
    PDF_X_LEVEL_1A_2001 = 0,
    PDF_X_LEVEL_1A_2003 = 1,
    PDF_X_LEVEL_3_2003 = 2,
    PDF_X_LEVEL_4 = 3,
    PDF_X_LEVEL_5 = 4,
    PDF_X_LEVEL_6 = 5
  };

  // PDF/UA levels
  enum PdfUaLevel {
    PDF_UA_LEVEL_1 = 0
  };

  // Barcode formats
  enum BarcodeFormat {
    BARCODE_FORMAT_QR_CODE = 0,
    BARCODE_FORMAT_EAN13 = 1,
    BARCODE_FORMAT_EAN8 = 2,
    BARCODE_FORMAT_UPC_A = 3,
    BARCODE_FORMAT_UPC_E = 4,
    BARCODE_FORMAT_CODE128 = 5,
    BARCODE_FORMAT_CODE39 = 6,
    BARCODE_FORMAT_CODABAR = 7,
    BARCODE_FORMAT_ITF = 8
  };

  // QR error correction levels
  enum QrErrorCorrectionLevel {
    QR_ERROR_CORRECTION_L = 0,
    QR_ERROR_CORRECTION_M = 1,
    QR_ERROR_CORRECTION_Q = 2,
    QR_ERROR_CORRECTION_H = 3
  };

  // Page complexity levels
  enum PageComplexity {
    PAGE_COMPLEXITY_SIMPLE = 0,
    PAGE_COMPLEXITY_MODERATE = 1,
    PAGE_COMPLEXITY_COMPLEX = 2,
    PAGE_COMPLEXITY_VERY_COMPLEX = 3
  };

  // Content types
  enum ContentType {
    CONTENT_TYPE_TEXT_ONLY = 0,
    CONTENT_TYPE_TEXT_IMAGES = 1,
    CONTENT_TYPE_TABLES = 2,
    CONTENT_TYPE_MIXED_LAYOUT = 3,
    CONTENT_TYPE_SCANNED = 4,
    CONTENT_TYPE_FORM = 5,
    CONTENT_TYPE_VECTOR_GRAPHICS = 6
  };

  // Structures
  struct PdfRenderOptions {
    int dpi;
    ImageFormat format;
    int jpeg_quality;
    int webp_quality;
    int color_space;
    bool antialias;
    uint32_t background_color;
    int max_width;
    int max_height;
    bool apply_color_profile;
  };

  struct PdfOcrConfig {
    float detection_threshold;
    float recognition_threshold;
    int max_side_len;
    bool use_gpu;
    int gpu_device_id;
  };

  // Logging
  extern void pdf_oxide_set_log_level(int level);
  extern int pdf_oxide_get_log_level();

  // Document Operations
  extern void* pdf_document_open(const char* path, int* error_code);
  extern void* pdf_document_open_from_bytes(const uint8_t* data, size_t len, int* error_code);
  extern void* pdf_document_open_with_password(const char* path, const char* password, int* error_code);
  extern void pdf_document_free(void* handle);
  extern int32_t pdf_document_get_page_count(void* handle, int* error_code);
  extern void pdf_document_get_version(const void* handle, uint8_t* major, uint8_t* minor);
  extern bool pdf_document_has_structure_tree(void* handle);
  extern char* pdf_document_extract_text(void* handle, int32_t page_index, int* error_code);
  extern char* pdf_document_to_markdown(void* handle, int32_t page_index, int* error_code);
  extern char* pdf_document_to_html(void* handle, int32_t page_index, int* error_code);
  extern char* pdf_document_to_plain_text(void* handle, int32_t page_index, int* error_code);
  extern char* pdf_document_to_markdown_all(void* handle, int* error_code);
  extern void* pdf_document_search_page(void* handle, const char* text, int32_t page_index, int case_sensitive, int* error_code);
  extern void* pdf_document_search_all(void* handle, const char* text, int case_sensitive, int* error_code);
  extern void* pdf_document_get_embedded_fonts(void* handle, int32_t page_index, int* error_code);
  extern void* pdf_document_get_embedded_images(void* handle, int32_t page_index, int* error_code);
  extern void* pdf_document_get_annotations(void* handle, int32_t page_index, int* error_code);

  // Rendering Operations
  extern PdfRenderOptions pdf_render_options_default();
  extern void* pdf_page_renderer_create(const PdfRenderOptions* options, int* error_code);
  extern void pdf_page_renderer_free(void* renderer);
  extern void* pdf_render_page(const void* document, int page_index, const void* renderer, int* error_code);
  extern bool pdf_render_page_to_file(const void* document, int page_index, const char* file_path, const PdfRenderOptions* options, int* error_code);
  extern void* pdf_render_page_thumbnail(const void* document, int page_index, int thumbnail_size, int* error_code);
  extern int pdf_rendered_image_width(const void* image);
  extern int pdf_rendered_image_height(const void* image);
  extern size_t pdf_rendered_image_size(const void* image);
  extern const uint8_t* pdf_rendered_image_data(const void* image);
  extern bool pdf_rendered_image_save(const void* image, const char* file_path, int* error_code);
  extern char* pdf_rendered_image_to_base64(const void* image, bool with_mime_prefix, int* error_code);
  extern void pdf_rendered_image_free(void* image);

  // OCR Operations
  extern void* pdf_ocr_engine_create(const PdfOcrConfig* config, int* error_code);
  extern void pdf_ocr_engine_free(void* engine);
  extern bool pdf_ocr_page_needs_ocr(const void* document, int page_index, int* error_code);
  extern void* pdf_ocr_recognize_page(const void* document, int page_index, const void* engine, int* error_code);
  extern char* pdf_ocr_extract_text(const void* document, int page_index, const void* engine, bool force_ocr, int* error_code);
  extern char* pdf_ocr_results_get_text(const void* results, int* error_code);
  extern float pdf_ocr_results_average_confidence(const void* results);
  extern void pdf_ocr_results_free(void* results);

  // Compliance Operations
  extern void* pdf_validate_pdf_a(const void* document, PdfALevel level, int* error_code);
  extern bool pdf_pdf_a_is_compliant(const void* results);
  extern int pdf_pdf_a_error_count(const void* results);
  extern int pdf_pdf_a_warning_count(const void* results);
  extern char* pdf_pdf_a_get_report(const void* results, int* error_code);
  extern void pdf_pdf_a_results_free(void* results);
  extern void* pdf_validate_pdf_x(const void* document, PdfXLevel level, int* error_code);
  extern bool pdf_pdf_x_is_compliant(const void* results);
  extern void pdf_pdf_x_results_free(void* results);
  extern void* pdf_validate_pdf_ua(const void* document, PdfUaLevel level, int* error_code);
  extern bool pdf_pdf_ua_is_accessible(const void* results);
  extern void pdf_pdf_ua_results_free(void* results);
  extern bool pdf_convert_to_pdf_a(void* document, PdfALevel level, int* error_code);

  // Signature Operations
  extern int pdf_document_get_signature_count(const void* document, int* error_code);
  extern void* pdf_document_get_signature(const void* document, int index, int* error_code);
  extern void pdf_signature_free(void* signature);
  extern char* pdf_signature_get_signer_name(const void* signature, int* error_code);
  extern int64_t pdf_signature_get_signing_time(const void* signature, int* error_code);
  extern char* pdf_signature_get_signing_reason(const void* signature, int* error_code);
  extern char* pdf_signature_get_signing_location(const void* signature, int* error_code);
  extern int pdf_signature_verify(const void* signature, int* error_code);
  extern int pdf_document_verify_all_signatures(const void* document, int* error_code);
  extern void* pdf_signature_get_certificate(const void* signature, int* error_code);
  extern void pdf_certificate_free(void* cert);
  extern char* pdf_certificate_get_subject(const void* cert, int* error_code);
  extern char* pdf_certificate_get_issuer(const void* cert, int* error_code);
  extern char* pdf_certificate_get_serial(const void* cert, int* error_code);
  extern int pdf_certificate_is_valid(const void* cert, int* error_code);

  // Detailed Annotation Accessors
  extern char* pdf_oxide_annotation_get_subtype(const void* annotations, int32_t index, int* error_code);
  extern char* pdf_oxide_annotation_get_author(const void* annotations, int32_t index, int* error_code);
  extern int64_t pdf_oxide_annotation_get_creation_date(const void* annotations, int32_t index, int* error_code);
  extern int64_t pdf_oxide_annotation_get_modification_date(const void* annotations, int32_t index, int* error_code);
  extern float pdf_oxide_annotation_get_border_width(const void* annotations, int32_t index, int* error_code);
  extern uint32_t pdf_oxide_annotation_get_color(const void* annotations, int32_t index, int* error_code);
  extern bool pdf_oxide_annotation_is_hidden(const void* annotations, int32_t index, int* error_code);
  extern bool pdf_oxide_annotation_is_printable(const void* annotations, int32_t index, int* error_code);
  extern bool pdf_oxide_annotation_is_read_only(const void* annotations, int32_t index, int* error_code);
  extern bool pdf_oxide_annotation_is_marked_deleted(const void* annotations, int32_t index, int* error_code);

  // Rendering variants
  extern int pdf_estimate_render_time(const void* doc, int page_index, int* error_code);
  extern void* pdf_render_page_zoom(void* doc, int page_index, float zoom, int format, int* error_code);
  extern void* pdf_render_page_fit(void* doc, int page_index, int w, int h, int format, int* error_code);
  extern int pdf_save_rendered_image(const void* image, const char* path, int* error_code);

  // Barcode Operations
  extern void* pdf_generate_qr_code(const char* data, QrErrorCorrectionLevel error_correction, int* error_code);
  extern void* pdf_generate_barcode(BarcodeFormat format, const char* data, int* error_code);
  extern uint8_t* pdf_barcode_get_image_png(const void* barcode, int size_px, size_t* out_size, int* error_code);
  extern char* pdf_barcode_get_svg(const void* barcode, int size_px, int* error_code);
  extern void pdf_barcode_free(void* barcode);
  extern bool pdf_add_barcode_to_page(void* document, int page_num, const void* barcode, float x, float y, float width, float height, int* error_code);

  // XFA Operations
  extern bool pdf_document_has_xfa(const void* document, int* error_code);
  extern void* pdf_parse_xfa_form(const void* document, int* error_code);
  extern void pdf_xfa_form_free(void* form);
  extern int pdf_xfa_form_field_count(const void* form, int* error_code);
  extern void* pdf_xfa_form_get_field(const void* form, int index, int* error_code);
  extern char* pdf_xfa_field_get_name(const void* field, int* error_code);
  extern void pdf_xfa_field_free(void* field);
  extern bool pdf_convert_xfa_to_acroform(void* document, int* error_code);

  // Hybrid ML Operations
  extern void* pdf_analyze_page(const void* document, int page_index, int* error_code);
  extern PageComplexity pdf_analysis_get_complexity(const void* result);
  extern float pdf_analysis_get_complexity_score(const void* result);
  extern ContentType pdf_analysis_get_content_type(const void* result);
  extern float pdf_analysis_get_text_density(const void* result);
  extern float pdf_analysis_get_image_density(const void* result);
  extern void pdf_analysis_result_free(void* result);
  extern void* pdf_create_extraction_strategy(const void* document, int page_index, int* error_code);
  extern char* pdf_strategy_get_description(const void* strategy, int* error_code);
  extern bool pdf_strategy_recommends_ocr(const void* strategy);
  extern void pdf_strategy_free(void* strategy);
  extern char* pdf_ml_get_status(int* error_code);
  extern bool pdf_ml_model_available(const char* model_name);

  // Document Editor Operations
  extern void* document_editor_open(const char* path, int* error_code);
  extern void document_editor_free(void* handle);
  extern bool document_editor_is_modified(const void* handle);
  extern int32_t document_editor_get_page_count(void* handle, int* error_code);
  extern int document_editor_save(void* handle, const char* path, int* error_code);
  extern char* document_editor_get_source_path(const void* handle, int* error_code);
  extern int document_editor_set_title(void* handle, const char* value, int* error_code);
  extern int document_editor_set_author(void* handle, const char* value, int* error_code);
  extern int document_editor_set_subject(void* handle, const char* value, int* error_code);
  extern int document_editor_set_producer(void* handle, const char* value, int* error_code);
  extern int document_editor_delete_page(void* handle, int32_t page_index, int* error_code);
  extern int document_editor_move_page(void* handle, int32_t from, int32_t to, int* error_code);
  extern int document_editor_set_page_rotation(void* handle, int32_t page, int32_t degrees, int* error_code);
  extern int32_t document_editor_get_page_rotation(void* handle, int32_t page, int* error_code);
  extern int document_editor_erase_region(void* handle, int32_t page, float x, float y, float w, float h, int* error_code);
  extern int document_editor_flatten_annotations(void* handle, int32_t page, int* error_code);
  extern int document_editor_flatten_all_annotations(void* handle, int* error_code);
  extern int document_editor_crop_margins(void* handle, int32_t page, float top, float right, float bottom, float left, int* error_code);
  extern int document_editor_merge_from(void* handle, const char* source_path, int* error_code);
  extern int document_editor_flatten_forms(void* handle, int* error_code);
  extern int document_editor_flatten_forms_on_page(void* handle, int32_t page, int* error_code);
  // Missing document editor functions
  extern char* document_editor_get_creation_date(const void* handle, int* error_code);
  extern char* document_editor_get_producer(const void* handle, int* error_code);
  extern void document_editor_get_version(const void* handle, uint8_t* major, uint8_t* minor);
  extern int document_editor_save_encrypted(void* handle, const char* path, const char* user_password, const char* owner_password, int* error_code);
  extern int document_editor_set_creation_date(void* handle, const char* date_str, int* error_code);
  extern int document_editor_set_form_field_value(void* handle, const char* name, const char* value, int* error_code);

  // Form Fields
  extern void* pdf_document_get_form_fields(void* handle, int32_t page_index, int* error_code);
  extern int32_t pdf_oxide_form_field_count(const void* fields);
  extern char* pdf_oxide_form_field_get_name(const void* fields, int32_t index, int* error_code);
  extern char* pdf_oxide_form_field_get_type(const void* fields, int32_t index, int* error_code);
  extern char* pdf_oxide_form_field_get_value(const void* fields, int32_t index, int* error_code);
  extern bool pdf_oxide_form_field_is_readonly(const void* fields, int32_t index, int* error_code);
  extern bool pdf_oxide_form_field_is_required(const void* fields, int32_t index, int* error_code);
  extern void pdf_oxide_form_field_list_free(void* handle);

  // Advanced Text Extraction (chars, words, lines)
  extern void* pdf_document_extract_chars(void* handle, int32_t page_index, int* error_code);
  extern int32_t pdf_oxide_char_count(const void* chars);
  extern uint32_t pdf_oxide_char_get_char(const void* chars, int32_t index, int* error_code);
  extern void pdf_oxide_char_get_bbox(const void* chars, int32_t index, float* x, float* y, float* w, float* h, int* error_code);
  extern char* pdf_oxide_char_get_font_name(const void* chars, int32_t index, int* error_code);
  extern float pdf_oxide_char_get_font_size(const void* chars, int32_t index, int* error_code);
  extern void pdf_oxide_char_list_free(void* handle);

  extern void* pdf_document_extract_words(void* handle, int32_t page_index, int* error_code);
  extern int32_t pdf_oxide_word_count(const void* words);
  extern char* pdf_oxide_word_get_text(const void* words, int32_t index, int* error_code);
  extern void pdf_oxide_word_get_bbox(const void* words, int32_t index, float* x, float* y, float* w, float* h, int* error_code);
  extern char* pdf_oxide_word_get_font_name(const void* words, int32_t index, int* error_code);
  extern float pdf_oxide_word_get_font_size(const void* words, int32_t index, int* error_code);
  extern bool pdf_oxide_word_is_bold(const void* words, int32_t index, int* error_code);
  extern void pdf_oxide_word_list_free(void* handle);

  extern void* pdf_document_extract_text_lines(void* handle, int32_t page_index, int* error_code);
  extern int32_t pdf_oxide_line_count(const void* lines);
  extern char* pdf_oxide_line_get_text(const void* lines, int32_t index, int* error_code);
  extern void pdf_oxide_line_get_bbox(const void* lines, int32_t index, float* x, float* y, float* w, float* h, int* error_code);
  extern int32_t pdf_oxide_line_get_word_count(const void* lines, int32_t index, int* error_code);
  extern void pdf_oxide_line_list_free(void* handle);

  // Table Extraction
  extern void* pdf_document_extract_tables(void* handle, int32_t page_index, int* error_code);
  extern int32_t pdf_oxide_table_count(const void* tables);
  extern int32_t pdf_oxide_table_get_row_count(const void* tables, int32_t index, int* error_code);
  extern int32_t pdf_oxide_table_get_col_count(const void* tables, int32_t index, int* error_code);
  extern char* pdf_oxide_table_get_cell_text(const void* tables, int32_t table_idx, int32_t row, int32_t col, int* error_code);
  extern bool pdf_oxide_table_has_header(const void* tables, int32_t index, int* error_code);
  extern void pdf_oxide_table_list_free(void* handle);

  // Full Document Conversion
  extern char* pdf_document_extract_all_text(void* handle, int* error_code);
  extern char* pdf_document_to_html_all(void* handle, int* error_code);
  extern char* pdf_document_to_plain_text_all(void* handle, int* error_code);

  // Document properties
  extern bool pdf_document_is_encrypted(const void* handle);
  extern bool pdf_document_authenticate(void* handle, const char* password, int* error_code);
  // pdf_document_has_xfa is already declared above (line ~226) with the correct signature
  extern char* pdf_document_get_page_labels(void* handle, int* error_code);
  extern char* pdf_document_get_xmp_metadata(void* handle, int* error_code);
  extern char* pdf_document_get_outline(void* handle, int* error_code);

  // Search Result Accessors
  extern int pdf_oxide_search_result_count(const void* results);
  extern char* pdf_oxide_search_result_get_text(const void* results, int32_t index, int* error_code);
  extern int32_t pdf_oxide_search_result_get_page(const void* results, int32_t index);
  extern int32_t pdf_oxide_search_result_get_position(const void* results, int32_t index);
  extern void pdf_oxide_search_result_get_bbox(const void* results, int32_t index, float* x, float* y, float* width, float* height);
  extern void pdf_oxide_search_result_free(void* results);

  // Font Accessors
  extern int pdf_oxide_font_count(const void* fonts);
  extern char* pdf_oxide_font_get_name(const void* fonts, int32_t index, int* error_code);
  extern char* pdf_oxide_font_get_type(const void* fonts, int32_t index, int* error_code);
  extern bool pdf_oxide_font_is_embedded(const void* fonts, int32_t index);
  extern void pdf_oxide_font_free(void* fonts);

  // Image Accessors
  extern int pdf_oxide_image_count(const void* images);
  extern int pdf_oxide_image_get_width(const void* images, int32_t index);
  extern int pdf_oxide_image_get_height(const void* images, int32_t index);
  extern char* pdf_oxide_image_get_format(const void* images, int32_t index);
  extern void pdf_oxide_image_free(void* images);

  // Annotation Accessors
  extern int pdf_oxide_annotation_count(const void* annotations);
  extern char* pdf_oxide_annotation_get_type(const void* annotations, int32_t index);
  extern char* pdf_oxide_annotation_get_content(const void* annotations, int32_t index);
  extern void pdf_oxide_annotation_free(void* annotations);

  // PDF Document Editing (artifact removal, signing, form data)
  extern int pdf_document_erase_artifacts(void* handle, int32_t page_index, int* error_code);
  extern int pdf_document_erase_footer(void* handle, int32_t page_index, int* error_code);
  extern int pdf_document_erase_header(void* handle, int32_t page_index, int* error_code);
  extern uint8_t* pdf_document_export_form_data_to_bytes(void* handle, int32_t format_type, size_t* out_len, int* error_code);
  extern int pdf_document_import_form_data(const void* handle, const char* data_path, int* error_code);
  extern int pdf_document_remove_artifacts(void* handle, float threshold, int* error_code);
  extern int pdf_document_remove_footers(void* handle, float threshold, int* error_code);
  extern int pdf_document_remove_headers(void* handle, float threshold, int* error_code);
  extern int pdf_document_sign(void* handle, const void* certificate, const char* reason, const char* location, int* error_code);

  // Regional Extraction
  extern void* pdf_document_extract_images_in_rect(void* handle, int32_t page_index, float x, float y, float w, float h, int* error_code);
  extern void* pdf_document_extract_lines_in_rect(void* handle, int32_t page_index, float x, float y, float w, float h, int* error_code);
  extern void* pdf_document_extract_paths(void* handle, int32_t page_index, int* error_code);
  extern void* pdf_document_extract_tables_in_rect(void* handle, int32_t page_index, float x, float y, float w, float h, int* error_code);
  extern char* pdf_document_extract_text_in_rect(void* handle, int32_t page_index, float x, float y, float w, float h, int* error_code);
  extern void* pdf_document_extract_words_in_rect(void* handle, int32_t page_index, float x, float y, float w, float h, int* error_code);
  extern void* pdf_document_get_page_annotations(void* handle, int32_t page_index, int* error_code);

  // PDF Creation
  extern int pdf_editor_import_fdf_bytes(const void* handle, const uint8_t* data, size_t data_len, int* error_code);
  extern int pdf_editor_import_xfdf_bytes(const void* handle, const uint8_t* data, size_t data_len, int* error_code);
  extern bool pdf_form_import_from_file(const void* handle, const char* filename, int* error_code);
  extern void* pdf_from_html(const char* html, int* error_code);
  extern void* pdf_from_image(const char* path, int* error_code);
  extern void* pdf_from_image_bytes(const uint8_t* data, int32_t data_len, int* error_code);
  extern void* pdf_from_markdown(const char* markdown, int* error_code);
  extern void* pdf_from_text(const char* text, int* error_code);
  extern uint8_t* pdf_merge(const char** paths, int32_t path_count, int32_t* data_len, int* error_code);

  // Saving
  extern int pdf_save(void* handle, const char* path, int* error_code);
  extern uint8_t* pdf_save_to_bytes(void* handle, int32_t* data_len, int* error_code);

  // Rendering (additional)
  extern void* pdf_create_renderer(int dpi, int format, int quality, bool anti_alias, int* error_code);
  extern uint8_t* pdf_get_rendered_image_data(const void* image, int32_t* data_len, int* error_code);
  extern int pdf_get_rendered_image_height(const void* image, int* error_code);
  extern int pdf_get_rendered_image_width(const void* image, int* error_code);
  extern void pdf_renderer_free(void* renderer);
  extern void* pdf_render_page_region(void* handle, int32_t page_index, float crop_x, float crop_y, float crop_w, float crop_h, int format, int* error_code);

  // Barcode (additional accessors)
  extern float pdf_barcode_get_confidence(const void* barcode, int* error_code);
  extern char* pdf_barcode_get_data(const void* barcode, int* error_code);
  extern int pdf_barcode_get_format(const void* barcode, int* error_code);

  // Timestamp/TSA
  extern void pdf_certificate_get_validity(const void* cert, int64_t* not_before, int64_t* not_after, int* error_code);
  extern void* pdf_certificate_load_from_bytes(const uint8_t* data, int32_t len, const char* password, int* error_code);
  extern bool pdf_signature_add_timestamp(const void* signature, const void* timestamp, int* error_code);
  extern void* pdf_signature_get_timestamp(const void* signature, int* error_code);
  extern bool pdf_signature_has_timestamp(const void* signature, int* error_code);
  extern void pdf_timestamp_free(void* timestamp);
  extern int pdf_timestamp_get_hash_algorithm(const void* timestamp, int* error_code);
  extern const uint8_t* pdf_timestamp_get_message_imprint(const void* timestamp, size_t* out_len, int* error_code);
  extern char* pdf_timestamp_get_policy_oid(const void* timestamp, int* error_code);
  extern char* pdf_timestamp_get_serial(const void* timestamp, int* error_code);
  extern int64_t pdf_timestamp_get_time(const void* timestamp, int* error_code);
  extern const uint8_t* pdf_timestamp_get_token(const void* timestamp, size_t* out_len, int* error_code);
  extern char* pdf_timestamp_get_tsa_name(const void* timestamp, int* error_code);
  extern bool pdf_timestamp_verify(const void* timestamp, int* error_code);
  extern void* pdf_tsa_client_create(const char* url, const char* username, const char* password, int timeout, int hash_algo, bool use_nonce, bool cert_req, int* error_code);
  extern void pdf_tsa_client_free(void* client);
  extern void* pdf_tsa_request_timestamp(const void* client, const uint8_t* data, size_t data_len, int* error_code);
  extern void* pdf_tsa_request_timestamp_hash(const void* client, const uint8_t* hash, size_t hash_len, int hash_algo, int* error_code);

  // Compliance (additional)
  extern char* pdf_pdf_a_get_error(const void* results, int32_t index, int* error_code);
  extern int pdf_pdf_ua_error_count(const void* results);
  extern char* pdf_pdf_ua_get_error(const void* results, int32_t index, int* error_code);
  extern bool pdf_pdf_ua_get_stats(const void* results, int32_t* out_struct, int32_t* out_images, int32_t* out_tables, int32_t* out_forms, int32_t* out_annotations, int32_t* out_pages, int* error_code);
  extern char* pdf_pdf_ua_get_warning(const void* results, int32_t index, int* error_code);
  extern int pdf_pdf_ua_warning_count(const void* results);
  extern int pdf_pdf_x_error_count(const void* results);
  extern char* pdf_pdf_x_get_error(const void* results, int32_t index, int* error_code);
  extern void* pdf_validate_pdf_a_level(const void* document, int32_t level, int* error_code);
  extern void* pdf_validate_pdf_x_level(const void* document, int32_t level, int* error_code);

  // Page/Element/Accessor (additional)
  extern void pdf_oxide_annotation_get_rect(const void* annotations, int32_t index, float* x, float* y, float* w, float* h, int* error_code);
  extern void pdf_oxide_annotation_list_free(void* annotations);
  extern int32_t pdf_oxide_element_count(const void* elements);
  extern void pdf_oxide_element_get_rect(const void* elements, int32_t index, float* x, float* y, float* w, float* h, int* error_code);
  extern char* pdf_oxide_element_get_text(const void* elements, int32_t index, int* error_code);
  extern char* pdf_oxide_element_get_type(const void* elements, int32_t index, int* error_code);
  extern void pdf_oxide_elements_free(void* elements);
  extern char* pdf_oxide_font_get_encoding(const void* fonts, int32_t index, int* error_code);
  extern float pdf_oxide_font_get_size(const void* fonts, int32_t index, int* error_code);
  extern int pdf_oxide_font_is_subset(const void* fonts, int32_t index, int* error_code);
  extern void pdf_oxide_font_list_free(void* fonts);
  extern int32_t pdf_oxide_highlight_annotation_get_quad_point(const void* annotations, int32_t index, int32_t quad_index, float* x1, float* y1, float* x2, float* y2, float* x3, float* y3, float* x4, float* y4, int* error_code);
  extern int32_t pdf_oxide_highlight_annotation_get_quad_points_count(const void* annotations, int32_t index, int* error_code);
  extern int pdf_oxide_image_get_bits_per_component(const void* images, int32_t index, int* error_code);
  extern char* pdf_oxide_image_get_colorspace(const void* images, int32_t index, int* error_code);
  extern uint8_t* pdf_oxide_image_get_data(const void* images, int32_t index, int32_t* data_len, int* error_code);
  extern void pdf_oxide_image_list_free(void* images);
  extern char* pdf_oxide_link_annotation_get_uri(const void* annotations, int32_t index, int* error_code);
  extern int32_t pdf_oxide_path_count(const void* paths);
  extern void pdf_oxide_path_get_bbox(const void* paths, int32_t index, float* x, float* y, float* w, float* h, int* error_code);
  extern int32_t pdf_oxide_path_get_operation_count(const void* paths, int32_t index, int* error_code);
  extern float pdf_oxide_path_get_stroke_width(const void* paths, int32_t index, int* error_code);
  extern bool pdf_oxide_path_has_fill(const void* paths, int32_t index, int* error_code);
  extern bool pdf_oxide_path_has_stroke(const void* paths, int32_t index, int* error_code);
  extern void pdf_oxide_path_list_free(void* paths);
  extern char* pdf_oxide_text_annotation_get_icon_name(const void* annotations, int32_t index, int* error_code);
  extern void* pdf_page_get_elements(void* handle, int32_t page_index, int* error_code);
  extern float pdf_page_get_height(void* handle, int32_t page_index, int* error_code);
  extern int32_t pdf_page_get_rotation(void* handle, int32_t page_index, int* error_code);
  extern float pdf_page_get_width(void* handle, int32_t page_index, int* error_code);

  // Memory Management
  extern void free_string(char* ptr);
  extern void free_bytes(uint8_t* bytes);
}

// ============================================================
// Helper functions
// ============================================================

static std::string getErrorMessage(int errorCode) {
  switch (errorCode) {
    case PDF_ERROR_SUCCESS: return "success";
    case PDF_ERROR_INVALID_ARG: return "invalid argument";
    case PDF_ERROR_IO_ERROR: return "I/O error";
    case PDF_ERROR_PARSE_ERROR: return "parse error";
    case PDF_ERROR_NOT_FOUND: return "not found";
    case PDF_ERROR_PERMISSION_DENIED: return "permission denied";
    case PDF_ERROR_UNSUPPORTED: return "unsupported operation";
    case PDF_ERROR_INTERNAL: return "internal error";
    default: return "unknown error code " + std::to_string(errorCode);
  }
}

// ============================================================
// Document Operations
// ============================================================

Napi::Value OpenDocument(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsString()) {
    throw Napi::TypeError::New(env, "path must be a string");
  }

  std::string path = info[0].As<Napi::String>().Utf8Value();
  int errorCode = 0;
  void* handle = pdf_document_open(path.c_str(), &errorCode);

  if (errorCode != 0) {
    throw Napi::Error::New(env, "Failed to open document: " + getErrorMessage(errorCode));
  }

  if (!handle) {
    throw Napi::Error::New(env, "Failed to open document: internal error");
  }

  return Napi::External<void>::New(env, handle);
}

Napi::Value CloseDocument(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsExternal()) {
    throw Napi::TypeError::New(env, "handle must be an external pointer");
  }

  void* handle = info[0].As<Napi::External<void>>().Data();
  pdf_document_free(handle);

  return env.Undefined();
}

Napi::Value GetPageCount(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsExternal()) {
    throw Napi::TypeError::New(env, "handle must be an external pointer");
  }

  void* handle = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  int32_t count = pdf_document_get_page_count(handle, &errorCode);

  if (errorCode != 0) {
    throw Napi::Error::New(env, "Failed to get page count: " + getErrorMessage(errorCode));
  }

  return Napi::Number::New(env, count);
}

Napi::Value GetVersion(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsExternal()) {
    throw Napi::TypeError::New(env, "handle must be an external pointer");
  }

  void* handle = info[0].As<Napi::External<void>>().Data();
  uint8_t major = 0, minor = 0;
  pdf_document_get_version(handle, &major, &minor);

  Napi::Object version = Napi::Object::New(env);
  version.Set("major", Napi::Number::New(env, major));
  version.Set("minor", Napi::Number::New(env, minor));

  return version;
}

Napi::Value HasStructureTree(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsExternal()) {
    throw Napi::TypeError::New(env, "handle must be an external pointer");
  }

  void* handle = info[0].As<Napi::External<void>>().Data();
  bool hasTree = pdf_document_has_structure_tree(handle);

  return Napi::Boolean::New(env, hasTree);
}

Napi::Value ExtractText(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 2 || !info[0].IsExternal() || !info[1].IsNumber()) {
    throw Napi::TypeError::New(env, "invalid arguments");
  }

  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;

  char* text = pdf_document_extract_text(handle, pageIndex, &errorCode);

  if (errorCode != 0) {
    throw Napi::Error::New(env, "Failed to extract text: " + getErrorMessage(errorCode));
  }

  if (!text) {
    throw Napi::Error::New(env, "Failed to extract text: returned null");
  }

  std::string result(text);
  free_string(text);

  return Napi::String::New(env, result);
}

Napi::Value ToMarkdown(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 2 || !info[0].IsExternal() || !info[1].IsNumber()) {
    throw Napi::TypeError::New(env, "invalid arguments");
  }

  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;

  char* markdown = pdf_document_to_markdown(handle, pageIndex, &errorCode);

  if (errorCode != 0) {
    throw Napi::Error::New(env, "Failed to convert to Markdown: " + getErrorMessage(errorCode));
  }

  std::string result(markdown);
  free_string(markdown);

  return Napi::String::New(env, result);
}

Napi::Value ToHtml(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 2 || !info[0].IsExternal() || !info[1].IsNumber()) {
    throw Napi::TypeError::New(env, "invalid arguments");
  }

  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;

  char* html = pdf_document_to_html(handle, pageIndex, &errorCode);

  if (errorCode != 0) {
    throw Napi::Error::New(env, "Failed to convert to HTML: " + getErrorMessage(errorCode));
  }

  std::string result(html);
  free_string(html);

  return Napi::String::New(env, result);
}

Napi::Value ToPlainText(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 2 || !info[0].IsExternal() || !info[1].IsNumber()) {
    throw Napi::TypeError::New(env, "invalid arguments");
  }

  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;

  char* text = pdf_document_to_plain_text(handle, pageIndex, &errorCode);

  if (errorCode != 0) {
    throw Napi::Error::New(env, "Failed to convert to plain text: " + getErrorMessage(errorCode));
  }

  std::string result(text);
  free_string(text);

  return Napi::String::New(env, result);
}

Napi::Value ToMarkdownAll(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsExternal()) {
    throw Napi::TypeError::New(env, "handle must be an external pointer");
  }

  void* handle = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;

  char* markdown = pdf_document_to_markdown_all(handle, &errorCode);

  if (errorCode != 0) {
    throw Napi::Error::New(env, "Failed to convert to Markdown: " + getErrorMessage(errorCode));
  }

  std::string result(markdown);
  free_string(markdown);

  return Napi::String::New(env, result);
}

// ============================================================
// Search Operations
// ============================================================

Napi::Value SearchPage(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 4 || !info[0].IsExternal() || !info[1].IsString() ||
      !info[2].IsNumber() || !info[3].IsBoolean()) {
    throw Napi::TypeError::New(env, "invalid arguments: (handle, text, pageIndex, caseSensitive)");
  }

  void* handle = info[0].As<Napi::External<void>>().Data();
  std::string text = info[1].As<Napi::String>().Utf8Value();
  int32_t pageIndex = info[2].As<Napi::Number>().Int32Value();
  bool caseSensitive = info[3].As<Napi::Boolean>().Value();
  int errorCode = 0;

  void* results = pdf_document_search_page(handle, text.c_str(), pageIndex, caseSensitive ? 1 : 0, &errorCode);

  if (errorCode != 0) {
    throw Napi::Error::New(env, "Search failed: " + getErrorMessage(errorCode));
  }

  return Napi::External<void>::New(env, results);
}

Napi::Value SearchAll(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 3 || !info[0].IsExternal() || !info[1].IsString() || !info[2].IsBoolean()) {
    throw Napi::TypeError::New(env, "invalid arguments: (handle, text, caseSensitive)");
  }

  void* handle = info[0].As<Napi::External<void>>().Data();
  std::string text = info[1].As<Napi::String>().Utf8Value();
  bool caseSensitive = info[2].As<Napi::Boolean>().Value();
  int errorCode = 0;

  void* results = pdf_document_search_all(handle, text.c_str(), caseSensitive ? 1 : 0, &errorCode);

  if (errorCode != 0) {
    throw Napi::Error::New(env, "Search failed: " + getErrorMessage(errorCode));
  }

  return Napi::External<void>::New(env, results);
}

Napi::Value SearchResultCount(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsExternal()) {
    throw Napi::TypeError::New(env, "results must be an external pointer");
  }

  void* results = info[0].As<Napi::External<void>>().Data();
  int count = pdf_oxide_search_result_count(results);

  return Napi::Number::New(env, count);
}

Napi::Value SearchResultFree(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsExternal()) {
    throw Napi::TypeError::New(env, "results must be an external pointer");
  }

  void* results = info[0].As<Napi::External<void>>().Data();
  pdf_oxide_search_result_free(results);

  return env.Undefined();
}

// ============================================================
// Rendering Operations
// ============================================================

Napi::Value CreateRenderer(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  PdfRenderOptions options = pdf_render_options_default();

  if (info.Length() > 0 && info[0].IsObject()) {
    Napi::Object opts = info[0].As<Napi::Object>();

    if (opts.Has("dpi")) options.dpi = opts.Get("dpi").As<Napi::Number>().Int32Value();
    if (opts.Has("format")) options.format = (ImageFormat)opts.Get("format").As<Napi::Number>().Int32Value();
    if (opts.Has("jpegQuality")) options.jpeg_quality = opts.Get("jpegQuality").As<Napi::Number>().Int32Value();
    if (opts.Has("antialias")) options.antialias = opts.Get("antialias").As<Napi::Boolean>().Value();
  }

  int errorCode = 0;
  void* renderer = pdf_page_renderer_create(&options, &errorCode);

  if (errorCode != 0 || !renderer) {
    throw Napi::Error::New(env, "Failed to create renderer: " + getErrorMessage(errorCode));
  }

  return Napi::External<void>::New(env, renderer);
}

Napi::Value FreeRenderer(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsExternal()) {
    throw Napi::TypeError::New(env, "renderer must be an external pointer");
  }

  void* renderer = info[0].As<Napi::External<void>>().Data();
  pdf_page_renderer_free(renderer);

  return env.Undefined();
}

Napi::Value RenderPage(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 3 || !info[0].IsExternal() || !info[1].IsNumber() || !info[2].IsExternal()) {
    throw Napi::TypeError::New(env, "invalid arguments: (document, pageIndex, renderer)");
  }

  void* document = info[0].As<Napi::External<void>>().Data();
  int pageIndex = info[1].As<Napi::Number>().Int32Value();
  void* renderer = info[2].As<Napi::External<void>>().Data();
  int errorCode = 0;

  void* image = pdf_render_page(document, pageIndex, renderer, &errorCode);

  if (errorCode != 0 || !image) {
    throw Napi::Error::New(env, "Failed to render page: " + getErrorMessage(errorCode));
  }

  return Napi::External<void>::New(env, image);
}

Napi::Value RenderThumbnail(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 3 || !info[0].IsExternal() || !info[1].IsNumber() || !info[2].IsNumber()) {
    throw Napi::TypeError::New(env, "invalid arguments: (document, pageIndex, size)");
  }

  void* document = info[0].As<Napi::External<void>>().Data();
  int pageIndex = info[1].As<Napi::Number>().Int32Value();
  int size = info[2].As<Napi::Number>().Int32Value();
  int errorCode = 0;

  void* image = pdf_render_page_thumbnail(document, pageIndex, size, &errorCode);

  if (errorCode != 0 || !image) {
    throw Napi::Error::New(env, "Failed to render thumbnail: " + getErrorMessage(errorCode));
  }

  return Napi::External<void>::New(env, image);
}

Napi::Value RenderedImageToBase64(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 2 || !info[0].IsExternal() || !info[1].IsBoolean()) {
    throw Napi::TypeError::New(env, "invalid arguments: (image, withMimePrefix)");
  }

  void* image = info[0].As<Napi::External<void>>().Data();
  bool withMimePrefix = info[1].As<Napi::Boolean>().Value();
  int errorCode = 0;

  char* base64 = pdf_rendered_image_to_base64(image, withMimePrefix, &errorCode);

  if (errorCode != 0 || !base64) {
    throw Napi::Error::New(env, "Failed to convert to base64: " + getErrorMessage(errorCode));
  }

  std::string result(base64);
  free_string(base64);

  return Napi::String::New(env, result);
}

Napi::Value FreeRenderedImage(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsExternal()) {
    throw Napi::TypeError::New(env, "image must be an external pointer");
  }

  void* image = info[0].As<Napi::External<void>>().Data();
  pdf_rendered_image_free(image);

  return env.Undefined();
}

// ============================================================
// OCR Operations
// ============================================================

Napi::Value CreateOCREngine(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  PdfOcrConfig config = {
    .detection_threshold = 0.5f,
    .recognition_threshold = 0.5f,
    .max_side_len = 960,
    .use_gpu = false,
    .gpu_device_id = 0
  };

  if (info.Length() > 0 && info[0].IsObject()) {
    Napi::Object opts = info[0].As<Napi::Object>();

    if (opts.Has("detectionThreshold")) config.detection_threshold = opts.Get("detectionThreshold").As<Napi::Number>().FloatValue();
    if (opts.Has("recognitionThreshold")) config.recognition_threshold = opts.Get("recognitionThreshold").As<Napi::Number>().FloatValue();
    if (opts.Has("maxSideLen")) config.max_side_len = opts.Get("maxSideLen").As<Napi::Number>().Int32Value();
    if (opts.Has("useGpu")) config.use_gpu = opts.Get("useGpu").As<Napi::Boolean>().Value();
    if (opts.Has("gpuDeviceId")) config.gpu_device_id = opts.Get("gpuDeviceId").As<Napi::Number>().Int32Value();
  }

  int errorCode = 0;
  void* engine = pdf_ocr_engine_create(&config, &errorCode);

  if (errorCode != 0 || !engine) {
    throw Napi::Error::New(env, "Failed to create OCR engine: " + getErrorMessage(errorCode));
  }

  return Napi::External<void>::New(env, engine);
}

Napi::Value FreeOCREngine(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsExternal()) {
    throw Napi::TypeError::New(env, "engine must be an external pointer");
  }

  void* engine = info[0].As<Napi::External<void>>().Data();
  pdf_ocr_engine_free(engine);

  return env.Undefined();
}

Napi::Value PageNeedsOCR(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 2 || !info[0].IsExternal() || !info[1].IsNumber()) {
    throw Napi::TypeError::New(env, "invalid arguments: (document, pageIndex)");
  }

  void* document = info[0].As<Napi::External<void>>().Data();
  int pageIndex = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;

  bool needsOCR = pdf_ocr_page_needs_ocr(document, pageIndex, &errorCode);

  if (errorCode != 0) {
    throw Napi::Error::New(env, "Failed to check OCR needs: " + getErrorMessage(errorCode));
  }

  return Napi::Boolean::New(env, needsOCR);
}

Napi::Value OCRExtractText(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 4 || !info[0].IsExternal() || !info[1].IsNumber() ||
      !info[2].IsExternal() || !info[3].IsBoolean()) {
    throw Napi::TypeError::New(env, "invalid arguments: (document, pageIndex, engine, forceOCR)");
  }

  void* document = info[0].As<Napi::External<void>>().Data();
  int pageIndex = info[1].As<Napi::Number>().Int32Value();
  void* engine = info[2].As<Napi::External<void>>().Data();
  bool forceOCR = info[3].As<Napi::Boolean>().Value();
  int errorCode = 0;

  char* text = pdf_ocr_extract_text(document, pageIndex, engine, forceOCR, &errorCode);

  if (errorCode != 0 || !text) {
    throw Napi::Error::New(env, "OCR extraction failed: " + getErrorMessage(errorCode));
  }

  std::string result(text);
  free_string(text);

  return Napi::String::New(env, result);
}

// ============================================================
// Compliance Operations
// ============================================================

Napi::Value ValidatePdfA(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 2 || !info[0].IsExternal() || !info[1].IsNumber()) {
    throw Napi::TypeError::New(env, "invalid arguments: (document, level)");
  }

  void* document = info[0].As<Napi::External<void>>().Data();
  PdfALevel level = (PdfALevel)info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;

  void* results = pdf_validate_pdf_a(document, level, &errorCode);

  if (errorCode != 0 || !results) {
    throw Napi::Error::New(env, "PDF/A validation failed: " + getErrorMessage(errorCode));
  }

  return Napi::External<void>::New(env, results);
}

Napi::Value PdfAIsCompliant(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsExternal()) {
    throw Napi::TypeError::New(env, "results must be an external pointer");
  }

  void* results = info[0].As<Napi::External<void>>().Data();
  bool compliant = pdf_pdf_a_is_compliant(results);

  return Napi::Boolean::New(env, compliant);
}

Napi::Value PdfAGetReport(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsExternal()) {
    throw Napi::TypeError::New(env, "results must be an external pointer");
  }

  void* results = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;

  char* report = pdf_pdf_a_get_report(results, &errorCode);

  if (errorCode != 0 || !report) {
    throw Napi::Error::New(env, "Failed to get report: " + getErrorMessage(errorCode));
  }

  std::string result(report);
  free_string(report);

  return Napi::String::New(env, result);
}

Napi::Value FreePdfAResults(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsExternal()) {
    throw Napi::TypeError::New(env, "results must be an external pointer");
  }

  void* results = info[0].As<Napi::External<void>>().Data();
  pdf_pdf_a_results_free(results);

  return env.Undefined();
}

// ============================================================
// Signature Operations
// ============================================================

Napi::Value GetSignatureCount(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsExternal()) {
    throw Napi::TypeError::New(env, "document must be an external pointer");
  }

  void* document = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;

  int count = pdf_document_get_signature_count(document, &errorCode);

  if (errorCode != 0) {
    throw Napi::Error::New(env, "Failed to get signature count: " + getErrorMessage(errorCode));
  }

  return Napi::Number::New(env, count);
}

Napi::Value GetSignature(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 2 || !info[0].IsExternal() || !info[1].IsNumber()) {
    throw Napi::TypeError::New(env, "invalid arguments: (document, index)");
  }

  void* document = info[0].As<Napi::External<void>>().Data();
  int index = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;

  void* signature = pdf_document_get_signature(document, index, &errorCode);

  if (errorCode != 0 || !signature) {
    throw Napi::Error::New(env, "Failed to get signature: " + getErrorMessage(errorCode));
  }

  return Napi::External<void>::New(env, signature);
}

Napi::Value SignatureVerify(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsExternal()) {
    throw Napi::TypeError::New(env, "signature must be an external pointer");
  }

  void* signature = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;

  int result = pdf_signature_verify(signature, &errorCode);

  if (errorCode != 0) {
    throw Napi::Error::New(env, "Signature verification failed: " + getErrorMessage(errorCode));
  }

  return Napi::Number::New(env, result);
}

Napi::Value FreeSignature(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsExternal()) {
    throw Napi::TypeError::New(env, "signature must be an external pointer");
  }

  void* signature = info[0].As<Napi::External<void>>().Data();
  pdf_signature_free(signature);

  return env.Undefined();
}

// ============================================================
// Barcode Operations
// ============================================================

Napi::Value GenerateQRCode(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 2 || !info[0].IsString() || !info[1].IsNumber()) {
    throw Napi::TypeError::New(env, "invalid arguments: (data, errorCorrection)");
  }

  std::string data = info[0].As<Napi::String>().Utf8Value();
  QrErrorCorrectionLevel errorCorrection = (QrErrorCorrectionLevel)info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;

  void* barcode = pdf_generate_qr_code(data.c_str(), errorCorrection, &errorCode);

  if (errorCode != 0 || !barcode) {
    throw Napi::Error::New(env, "Failed to generate QR code: " + getErrorMessage(errorCode));
  }

  return Napi::External<void>::New(env, barcode);
}

Napi::Value GenerateBarcode(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 2 || !info[0].IsNumber() || !info[1].IsString()) {
    throw Napi::TypeError::New(env, "invalid arguments: (format, data)");
  }

  BarcodeFormat format = (BarcodeFormat)info[0].As<Napi::Number>().Int32Value();
  std::string data = info[1].As<Napi::String>().Utf8Value();
  int errorCode = 0;

  void* barcode = pdf_generate_barcode(format, data.c_str(), &errorCode);

  if (errorCode != 0 || !barcode) {
    throw Napi::Error::New(env, "Failed to generate barcode: " + getErrorMessage(errorCode));
  }

  return Napi::External<void>::New(env, barcode);
}

Napi::Value BarcodeGetSVG(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 2 || !info[0].IsExternal() || !info[1].IsNumber()) {
    throw Napi::TypeError::New(env, "invalid arguments: (barcode, size)");
  }

  void* barcode = info[0].As<Napi::External<void>>().Data();
  int size = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;

  char* svg = pdf_barcode_get_svg(barcode, size, &errorCode);

  if (errorCode != 0 || !svg) {
    throw Napi::Error::New(env, "Failed to get SVG: " + getErrorMessage(errorCode));
  }

  std::string result(svg);
  free_string(svg);

  return Napi::String::New(env, result);
}

Napi::Value FreeBarcode(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsExternal()) {
    throw Napi::TypeError::New(env, "barcode must be an external pointer");
  }

  void* barcode = info[0].As<Napi::External<void>>().Data();
  pdf_barcode_free(barcode);

  return env.Undefined();
}

// ============================================================
// XFA Operations
// ============================================================

Napi::Value HasXFA(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsExternal()) {
    throw Napi::TypeError::New(env, "document must be an external pointer");
  }

  void* document = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;

  bool hasXFA = pdf_document_has_xfa(document, &errorCode);

  if (errorCode != 0) {
    throw Napi::Error::New(env, "Failed to check XFA: " + getErrorMessage(errorCode));
  }

  return Napi::Boolean::New(env, hasXFA);
}

Napi::Value ParseXFAForm(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsExternal()) {
    throw Napi::TypeError::New(env, "document must be an external pointer");
  }

  void* document = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;

  void* form = pdf_parse_xfa_form(document, &errorCode);

  if (errorCode != 0 || !form) {
    throw Napi::Error::New(env, "Failed to parse XFA form: " + getErrorMessage(errorCode));
  }

  return Napi::External<void>::New(env, form);
}

Napi::Value FreeXFAForm(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsExternal()) {
    throw Napi::TypeError::New(env, "form must be an external pointer");
  }

  void* form = info[0].As<Napi::External<void>>().Data();
  pdf_xfa_form_free(form);

  return env.Undefined();
}

// ============================================================
// Hybrid ML Operations
// ============================================================

Napi::Value AnalyzePage(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 2 || !info[0].IsExternal() || !info[1].IsNumber()) {
    throw Napi::TypeError::New(env, "invalid arguments: (document, pageIndex)");
  }

  void* document = info[0].As<Napi::External<void>>().Data();
  int pageIndex = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;

  void* result = pdf_analyze_page(document, pageIndex, &errorCode);

  if (errorCode != 0 || !result) {
    throw Napi::Error::New(env, "Page analysis failed: " + getErrorMessage(errorCode));
  }

  return Napi::External<void>::New(env, result);
}

Napi::Value AnalysisGetComplexity(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsExternal()) {
    throw Napi::TypeError::New(env, "result must be an external pointer");
  }

  void* result = info[0].As<Napi::External<void>>().Data();
  PageComplexity complexity = pdf_analysis_get_complexity(result);

  return Napi::Number::New(env, (int)complexity);
}

Napi::Value AnalysisGetComplexityScore(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsExternal()) {
    throw Napi::TypeError::New(env, "result must be an external pointer");
  }

  void* result = info[0].As<Napi::External<void>>().Data();
  float score = pdf_analysis_get_complexity_score(result);

  return Napi::Number::New(env, score);
}

Napi::Value AnalysisGetContentType(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsExternal()) {
    throw Napi::TypeError::New(env, "result must be an external pointer");
  }

  void* result = info[0].As<Napi::External<void>>().Data();
  ContentType contentType = pdf_analysis_get_content_type(result);

  return Napi::Number::New(env, (int)contentType);
}

Napi::Value FreeAnalysisResult(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsExternal()) {
    throw Napi::TypeError::New(env, "result must be an external pointer");
  }

  void* result = info[0].As<Napi::External<void>>().Data();
  pdf_analysis_result_free(result);

  return env.Undefined();
}

Napi::Value MLGetStatus(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  int errorCode = 0;

  char* status = pdf_ml_get_status(&errorCode);

  if (errorCode != 0 || !status) {
    throw Napi::Error::New(env, "Failed to get ML status: " + getErrorMessage(errorCode));
  }

  std::string result(status);
  free_string(status);

  return Napi::String::New(env, result);
}

Napi::Value MLModelAvailable(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsString()) {
    throw Napi::TypeError::New(env, "modelName must be a string");
  }

  std::string modelName = info[0].As<Napi::String>().Utf8Value();
  bool available = pdf_ml_model_available(modelName.c_str());

  return Napi::Boolean::New(env, available);
}

// ============================================================
// Document Editor Operations
// ============================================================

Napi::Value EditorOpen(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  if (info.Length() < 1 || !info[0].IsString()) throw Napi::TypeError::New(env, "path must be a string");
  std::string path = info[0].As<Napi::String>().Utf8Value();
  int errorCode = 0;
  void* handle = document_editor_open(path.c_str(), &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to open editor: " + getErrorMessage(errorCode));
  return Napi::External<void>::New(env, handle);
}

Napi::Value EditorFree(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  if (info.Length() < 1 || !info[0].IsExternal()) throw Napi::TypeError::New(env, "handle required");
  document_editor_free(info[0].As<Napi::External<void>>().Data());
  return env.Undefined();
}

Napi::Value EditorSave(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  if (info.Length() < 2) throw Napi::TypeError::New(env, "Expected (handle, path)");
  void* handle = info[0].As<Napi::External<void>>().Data();
  std::string path = info[1].As<Napi::String>().Utf8Value();
  int errorCode = 0;
  document_editor_save(handle, path.c_str(), &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to save: " + getErrorMessage(errorCode));
  return env.Undefined();
}

Napi::Value EditorGetPageCount(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  int32_t count = document_editor_get_page_count(handle, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return Napi::Number::New(env, count);
}

Napi::Value EditorIsModified(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  return Napi::Boolean::New(env, document_editor_is_modified(handle));
}

Napi::Value EditorSetTitle(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  std::string val = info[1].As<Napi::String>().Utf8Value();
  int errorCode = 0;
  document_editor_set_title(handle, val.c_str(), &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return env.Undefined();
}

Napi::Value EditorSetAuthor(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  std::string val = info[1].As<Napi::String>().Utf8Value();
  int errorCode = 0;
  document_editor_set_author(handle, val.c_str(), &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return env.Undefined();
}

Napi::Value EditorDeletePage(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;
  document_editor_delete_page(handle, pageIndex, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return env.Undefined();
}

Napi::Value EditorMovePage(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t from = info[1].As<Napi::Number>().Int32Value();
  int32_t to = info[2].As<Napi::Number>().Int32Value();
  int errorCode = 0;
  document_editor_move_page(handle, from, to, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return env.Undefined();
}

Napi::Value EditorSetPageRotation(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t page = info[1].As<Napi::Number>().Int32Value();
  int32_t degrees = info[2].As<Napi::Number>().Int32Value();
  int errorCode = 0;
  document_editor_set_page_rotation(handle, page, degrees, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return env.Undefined();
}

Napi::Value EditorMergeFrom(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  std::string path = info[1].As<Napi::String>().Utf8Value();
  int errorCode = 0;
  document_editor_merge_from(handle, path.c_str(), &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return env.Undefined();
}

Napi::Value EditorFlattenForms(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  document_editor_flatten_forms(handle, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return env.Undefined();
}

Napi::Value EditorFlattenAnnotations(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  document_editor_flatten_all_annotations(handle, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return env.Undefined();
}

// ============================================================
// Form Fields
// ============================================================

Napi::Value GetFormFields(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* docHandle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;
  void* fields = pdf_document_get_form_fields(docHandle, pageIndex, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!fields) return Napi::Array::New(env, 0);

  int32_t count = pdf_oxide_form_field_count(fields);
  Napi::Array result = Napi::Array::New(env, count);
  for (int32_t i = 0; i < count; i++) {
    Napi::Object field = Napi::Object::New(env);
    char* name = pdf_oxide_form_field_get_name(fields, i, &errorCode);
    char* type = pdf_oxide_form_field_get_type(fields, i, &errorCode);
    char* value = pdf_oxide_form_field_get_value(fields, i, &errorCode);
    field.Set("name", name ? Napi::String::New(env, name) : env.Null());
    field.Set("type", type ? Napi::String::New(env, type) : env.Null());
    field.Set("value", value ? Napi::String::New(env, value) : env.Null());
    field.Set("readonly", Napi::Boolean::New(env, pdf_oxide_form_field_is_readonly(fields, i, &errorCode)));
    field.Set("required", Napi::Boolean::New(env, pdf_oxide_form_field_is_required(fields, i, &errorCode)));
    if (name) free_string(name);
    if (type) free_string(type);
    if (value) free_string(value);
    result.Set(i, field);
  }
  pdf_oxide_form_field_list_free(fields);
  return result;
}

// ============================================================
// Advanced Text Extraction
// ============================================================

Napi::Value ExtractWords(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;
  void* words = pdf_document_extract_words(handle, pageIndex, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!words) return Napi::Array::New(env, 0);

  int32_t count = pdf_oxide_word_count(words);
  Napi::Array result = Napi::Array::New(env, count);
  for (int32_t i = 0; i < count; i++) {
    Napi::Object word = Napi::Object::New(env);
    char* text = pdf_oxide_word_get_text(words, i, &errorCode);
    float x, y, w, h;
    pdf_oxide_word_get_bbox(words, i, &x, &y, &w, &h, &errorCode);
    char* fontName = pdf_oxide_word_get_font_name(words, i, &errorCode);
    word.Set("text", text ? Napi::String::New(env, text) : env.Null());
    word.Set("x", Napi::Number::New(env, x));
    word.Set("y", Napi::Number::New(env, y));
    word.Set("width", Napi::Number::New(env, w));
    word.Set("height", Napi::Number::New(env, h));
    word.Set("fontName", fontName ? Napi::String::New(env, fontName) : env.Null());
    word.Set("fontSize", Napi::Number::New(env, pdf_oxide_word_get_font_size(words, i, &errorCode)));
    word.Set("isBold", Napi::Boolean::New(env, pdf_oxide_word_is_bold(words, i, &errorCode)));
    if (text) free_string(text);
    if (fontName) free_string(fontName);
    result.Set(i, word);
  }
  pdf_oxide_word_list_free(words);
  return result;
}

Napi::Value ExtractTextLines(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;
  void* lines = pdf_document_extract_text_lines(handle, pageIndex, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!lines) return Napi::Array::New(env, 0);

  int32_t count = pdf_oxide_line_count(lines);
  Napi::Array result = Napi::Array::New(env, count);
  for (int32_t i = 0; i < count; i++) {
    Napi::Object line = Napi::Object::New(env);
    char* text = pdf_oxide_line_get_text(lines, i, &errorCode);
    float x, y, w, h;
    pdf_oxide_line_get_bbox(lines, i, &x, &y, &w, &h, &errorCode);
    line.Set("text", text ? Napi::String::New(env, text) : env.Null());
    line.Set("x", Napi::Number::New(env, x));
    line.Set("y", Napi::Number::New(env, y));
    line.Set("width", Napi::Number::New(env, w));
    line.Set("height", Napi::Number::New(env, h));
    line.Set("wordCount", Napi::Number::New(env, pdf_oxide_line_get_word_count(lines, i, &errorCode)));
    if (text) free_string(text);
    result.Set(i, line);
  }
  pdf_oxide_line_list_free(lines);
  return result;
}

Napi::Value ExtractTables(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;
  void* tables = pdf_document_extract_tables(handle, pageIndex, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!tables) return Napi::Array::New(env, 0);

  int32_t count = pdf_oxide_table_count(tables);
  Napi::Array result = Napi::Array::New(env, count);
  for (int32_t i = 0; i < count; i++) {
    Napi::Object table = Napi::Object::New(env);
    int32_t rows = pdf_oxide_table_get_row_count(tables, i, &errorCode);
    int32_t cols = pdf_oxide_table_get_col_count(tables, i, &errorCode);
    table.Set("rows", Napi::Number::New(env, rows));
    table.Set("cols", Napi::Number::New(env, cols));
    table.Set("hasHeader", Napi::Boolean::New(env, pdf_oxide_table_has_header(tables, i, &errorCode)));
    Napi::Array cells = Napi::Array::New(env, rows);
    for (int32_t r = 0; r < rows; r++) {
      Napi::Array row = Napi::Array::New(env, cols);
      for (int32_t c = 0; c < cols; c++) {
        char* cell = pdf_oxide_table_get_cell_text(tables, i, r, c, &errorCode);
        row.Set(c, cell ? Napi::String::New(env, cell) : env.Null());
        if (cell) free_string(cell);
      }
      cells.Set(r, row);
    }
    table.Set("cells", cells);
    result.Set(i, table);
  }
  pdf_oxide_table_list_free(tables);
  return result;
}

// ============================================================
// Full Document Conversion + Properties
// ============================================================

Napi::Value ExtractAllText(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  char* text = pdf_document_extract_all_text(handle, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  std::string result = text ? text : "";
  if (text) free_string(text);
  return Napi::String::New(env, result);
}

Napi::Value ToHtmlAll(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  char* text = pdf_document_to_html_all(handle, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  std::string result = text ? text : "";
  if (text) free_string(text);
  return Napi::String::New(env, result);
}

Napi::Value ToPlainTextAll(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  char* text = pdf_document_to_plain_text_all(handle, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  std::string result = text ? text : "";
  if (text) free_string(text);
  return Napi::String::New(env, result);
}

Napi::Value IsEncrypted(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  return Napi::Boolean::New(env, pdf_document_is_encrypted(handle));
}

Napi::Value GetPageLabels(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  char* text = pdf_document_get_page_labels(handle, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  std::string result = text ? text : "";
  if (text) free_string(text);
  return Napi::String::New(env, result);
}

Napi::Value GetXmpMetadata(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  char* text = pdf_document_get_xmp_metadata(handle, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  std::string result = text ? text : "";
  if (text) free_string(text);
  return Napi::String::New(env, result);
}

Napi::Value GetOutline(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  char* text = pdf_document_get_outline(handle, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  std::string result = text ? text : "";
  if (text) free_string(text);
  return Napi::String::New(env, result);
}

// ============================================================
// Signature Operations (comprehensive)
// ============================================================
// Note: GetSignatureCount is defined earlier (around line 1069); this block
// provides only the extended signature accessors.

Napi::Value GetSignatureInfo(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int index = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;
  void* sig = pdf_document_get_signature(handle, index, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!sig) return env.Null();

  Napi::Object result = Napi::Object::New(env);

  char* signer = pdf_signature_get_signer_name(sig, &errorCode);
  result.Set("signerName", signer ? Napi::String::New(env, signer) : env.Null());
  if (signer) free_string(signer);

  result.Set("signingTime", Napi::Number::New(env, (double)pdf_signature_get_signing_time(sig, &errorCode)));

  char* reason = pdf_signature_get_signing_reason(sig, &errorCode);
  result.Set("reason", reason ? Napi::String::New(env, reason) : env.Null());
  if (reason) free_string(reason);

  char* location = pdf_signature_get_signing_location(sig, &errorCode);
  result.Set("location", location ? Napi::String::New(env, location) : env.Null());
  if (location) free_string(location);

  int verifyResult = pdf_signature_verify(sig, &errorCode);
  result.Set("verifyResult", Napi::Number::New(env, verifyResult));

  // Certificate info
  void* cert = pdf_signature_get_certificate(sig, &errorCode);
  if (cert) {
    Napi::Object certObj = Napi::Object::New(env);
    char* subject = pdf_certificate_get_subject(cert, &errorCode);
    certObj.Set("subject", subject ? Napi::String::New(env, subject) : env.Null());
    if (subject) free_string(subject);

    char* issuer = pdf_certificate_get_issuer(cert, &errorCode);
    certObj.Set("issuer", issuer ? Napi::String::New(env, issuer) : env.Null());
    if (issuer) free_string(issuer);

    char* serial = pdf_certificate_get_serial(cert, &errorCode);
    certObj.Set("serial", serial ? Napi::String::New(env, serial) : env.Null());
    if (serial) free_string(serial);

    certObj.Set("isValid", Napi::Number::New(env, pdf_certificate_is_valid(cert, &errorCode)));
    result.Set("certificate", certObj);
    pdf_certificate_free(cert);
  } else {
    result.Set("certificate", env.Null());
  }

  pdf_signature_free(sig);
  return result;
}

Napi::Value VerifyAllSignatures(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  int result = pdf_document_verify_all_signatures(handle, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return Napi::Number::New(env, result);
}

// ============================================================
// Detailed Annotation Accessors
// ============================================================

Napi::Value GetAnnotationsDetailed(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;
  void* annotations = pdf_document_get_annotations(handle, pageIndex, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!annotations) return Napi::Array::New(env, 0);

  int32_t count = pdf_oxide_search_result_count(annotations);
  // Use annotation-specific count
  // The annotation list uses pdf_oxide_annotation_count but was wired to searchResultCount in Init
  // Let's just use the actual count from the annotation_count extern (already declared above as pdf_oxide_search_result_count)

  Napi::Array result = Napi::Array::New(env, count);
  for (int32_t i = 0; i < count; i++) {
    Napi::Object ann = Napi::Object::New(env);

    // Basic properties (already available)
    char* type = pdf_oxide_annotation_get_subtype(annotations, i, &errorCode);
    ann.Set("subtype", type ? Napi::String::New(env, type) : env.Null());
    if (type) free_string(type);

    char* author = pdf_oxide_annotation_get_author(annotations, i, &errorCode);
    ann.Set("author", author ? Napi::String::New(env, author) : env.Null());
    if (author) free_string(author);

    ann.Set("creationDate", Napi::Number::New(env, (double)pdf_oxide_annotation_get_creation_date(annotations, i, &errorCode)));
    ann.Set("modificationDate", Napi::Number::New(env, (double)pdf_oxide_annotation_get_modification_date(annotations, i, &errorCode)));
    ann.Set("borderWidth", Napi::Number::New(env, pdf_oxide_annotation_get_border_width(annotations, i, &errorCode)));
    ann.Set("color", Napi::Number::New(env, pdf_oxide_annotation_get_color(annotations, i, &errorCode)));
    ann.Set("isHidden", Napi::Boolean::New(env, pdf_oxide_annotation_is_hidden(annotations, i, &errorCode)));
    ann.Set("isPrintable", Napi::Boolean::New(env, pdf_oxide_annotation_is_printable(annotations, i, &errorCode)));
    ann.Set("isReadOnly", Napi::Boolean::New(env, pdf_oxide_annotation_is_read_only(annotations, i, &errorCode)));
    ann.Set("isDeleted", Napi::Boolean::New(env, pdf_oxide_annotation_is_marked_deleted(annotations, i, &errorCode)));

    result.Set(i, ann);
  }
  // Note: annotations handle lifetime managed by caller (already freed in existing flow)
  return result;
}

// ============================================================
// Rendering variants
// ============================================================

Napi::Value EstimateRenderTime(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int pageIndex = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;
  int ms = pdf_estimate_render_time(handle, pageIndex, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return Napi::Number::New(env, ms);
}

Napi::Value RenderPageZoom(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int pageIndex = info[1].As<Napi::Number>().Int32Value();
  float zoom = info[2].As<Napi::Number>().FloatValue();
  int format = info.Length() > 3 ? info[3].As<Napi::Number>().Int32Value() : 0;
  int errorCode = 0;
  void* image = pdf_render_page_zoom(handle, pageIndex, zoom, format, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!image) throw Napi::Error::New(env, "Rendering failed");
  return Napi::External<void>::New(env, image);
}

Napi::Value RenderPageToFile(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* image = info[0].As<Napi::External<void>>().Data();
  std::string path = info[1].As<Napi::String>().Utf8Value();
  int errorCode = 0;
  pdf_save_rendered_image(image, path.c_str(), &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return env.Undefined();
}

Napi::Value RenderedImageWidth(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* image = info[0].As<Napi::External<void>>().Data();
  return Napi::Number::New(env, pdf_rendered_image_width(image));
}

Napi::Value RenderedImageHeight(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* image = info[0].As<Napi::External<void>>().Data();
  return Napi::Number::New(env, pdf_rendered_image_height(image));
}

// ============================================================
// OpenFromBuffer - open PDF from Buffer/Uint8Array
// ============================================================

Napi::Value OpenFromBuffer(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1) {
    throw Napi::TypeError::New(env, "Expected a Buffer or Uint8Array argument");
  }

  const uint8_t* data;
  size_t length;

  if (info[0].IsBuffer()) {
    auto buf = info[0].As<Napi::Buffer<uint8_t>>();
    data = buf.Data();
    length = buf.Length();
  } else if (info[0].IsTypedArray()) {
    auto arr = info[0].As<Napi::Uint8Array>();
    data = arr.Data();
    length = arr.ByteLength();
  } else {
    throw Napi::TypeError::New(env, "Argument must be a Buffer or Uint8Array");
  }

  if (length == 0) {
    throw Napi::Error::New(env, "Buffer must not be empty");
  }

  int errorCode = 0;
  void* handle = pdf_document_open_from_bytes(data, length, &errorCode);

  if (errorCode != 0) {
    throw Napi::Error::New(env, "Failed to open document from buffer: " + getErrorMessage(errorCode));
  }

  if (!handle) {
    throw Napi::Error::New(env, "Failed to open document from buffer: internal error");
  }

  return Napi::External<void>::New(env, handle);
}

// ============================================================
// OpenWithPassword - open password-protected PDF
// ============================================================

Napi::Value OpenWithPassword(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 2 || !info[0].IsString() || !info[1].IsString()) {
    throw Napi::TypeError::New(env, "Expected (path: string, password: string)");
  }

  std::string path = info[0].As<Napi::String>().Utf8Value();
  std::string password = info[1].As<Napi::String>().Utf8Value();
  int errorCode = 0;
  void* handle = pdf_document_open_with_password(path.c_str(), password.c_str(), &errorCode);

  if (errorCode != 0) {
    throw Napi::Error::New(env, "Failed to open document: " + getErrorMessage(errorCode));
  }

  if (!handle) {
    throw Napi::Error::New(env, "Failed to open document: internal error");
  }

  return Napi::External<void>::New(env, handle);
}

// ============================================================
// Logging control
// ============================================================

Napi::Value SetLogLevel(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();

  if (info.Length() < 1 || !info[0].IsNumber()) {
    throw Napi::TypeError::New(env, "level must be a number (0=Off, 1=Error, 2=Warn, 3=Info, 4=Debug, 5=Trace)");
  }

  int level = info[0].As<Napi::Number>().Int32Value();
  pdf_oxide_set_log_level(level);

  return env.Undefined();
}

Napi::Value GetLogLevel(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  return Napi::Number::New(env, pdf_oxide_get_log_level());
}

// ============================================================
// Document Editor (missing wrappers)
// ============================================================

Napi::Value EditorGetCreationDate(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  char* date = document_editor_get_creation_date(handle, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!date) return env.Null();
  std::string result(date);
  free_string(date);
  return Napi::String::New(env, result);
}

Napi::Value EditorGetProducer(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  char* producer = document_editor_get_producer(handle, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!producer) return env.Null();
  std::string result(producer);
  free_string(producer);
  return Napi::String::New(env, result);
}

Napi::Value EditorGetVersion(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  uint8_t major = 0, minor = 0;
  document_editor_get_version(handle, &major, &minor);
  Napi::Object version = Napi::Object::New(env);
  version.Set("major", Napi::Number::New(env, major));
  version.Set("minor", Napi::Number::New(env, minor));
  return version;
}

Napi::Value EditorSaveEncrypted(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  if (info.Length() < 4) throw Napi::TypeError::New(env, "Expected (handle, path, userPassword, ownerPassword)");
  void* handle = info[0].As<Napi::External<void>>().Data();
  std::string path = info[1].As<Napi::String>().Utf8Value();
  std::string userPwd = info[2].As<Napi::String>().Utf8Value();
  std::string ownerPwd = info[3].As<Napi::String>().Utf8Value();
  int errorCode = 0;
  document_editor_save_encrypted(handle, path.c_str(), userPwd.c_str(), ownerPwd.c_str(), &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to save encrypted: " + getErrorMessage(errorCode));
  return env.Undefined();
}

Napi::Value EditorSetCreationDate(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  std::string date = info[1].As<Napi::String>().Utf8Value();
  int errorCode = 0;
  document_editor_set_creation_date(handle, date.c_str(), &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return env.Undefined();
}

Napi::Value EditorSetFormFieldValue(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  if (info.Length() < 3) throw Napi::TypeError::New(env, "Expected (handle, fieldName, value)");
  void* handle = info[0].As<Napi::External<void>>().Data();
  std::string name = info[1].As<Napi::String>().Utf8Value();
  std::string value = info[2].As<Napi::String>().Utf8Value();
  int errorCode = 0;
  document_editor_set_form_field_value(handle, name.c_str(), value.c_str(), &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to set form field: " + getErrorMessage(errorCode));
  return env.Undefined();
}

// ============================================================
// PDF Document Editing (artifact removal, signing, form data)
// ============================================================

Napi::Value DocumentEraseArtifacts(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;
  pdf_document_erase_artifacts(handle, pageIndex, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return env.Undefined();
}

Napi::Value DocumentEraseFooter(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;
  pdf_document_erase_footer(handle, pageIndex, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return env.Undefined();
}

Napi::Value DocumentEraseHeader(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;
  pdf_document_erase_header(handle, pageIndex, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return env.Undefined();
}

Napi::Value DocumentExportFormData(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t formatType = info.Length() > 1 ? info[1].As<Napi::Number>().Int32Value() : 0;
  size_t outLen = 0;
  int errorCode = 0;
  uint8_t* data = pdf_document_export_form_data_to_bytes(handle, formatType, &outLen, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to export form data: " + getErrorMessage(errorCode));
  if (!data || outLen == 0) return Napi::Buffer<uint8_t>::New(env, 0);
  auto buf = Napi::Buffer<uint8_t>::Copy(env, data, outLen);
  free_bytes(data);
  return buf;
}

Napi::Value DocumentImportFormData(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  std::string path = info[1].As<Napi::String>().Utf8Value();
  int errorCode = 0;
  pdf_document_import_form_data(handle, path.c_str(), &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to import form data: " + getErrorMessage(errorCode));
  return env.Undefined();
}

Napi::Value DocumentRemoveArtifacts(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  float threshold = info.Length() > 1 ? info[1].As<Napi::Number>().FloatValue() : 0.1f;
  int errorCode = 0;
  int count = pdf_document_remove_artifacts(handle, threshold, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return Napi::Number::New(env, count);
}

Napi::Value DocumentRemoveFooters(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  float threshold = info.Length() > 1 ? info[1].As<Napi::Number>().FloatValue() : 0.1f;
  int errorCode = 0;
  int count = pdf_document_remove_footers(handle, threshold, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return Napi::Number::New(env, count);
}

Napi::Value DocumentRemoveHeaders(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  float threshold = info.Length() > 1 ? info[1].As<Napi::Number>().FloatValue() : 0.1f;
  int errorCode = 0;
  int count = pdf_document_remove_headers(handle, threshold, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return Napi::Number::New(env, count);
}

Napi::Value DocumentSign(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  if (info.Length() < 2) throw Napi::TypeError::New(env, "Expected (document, certificate, [reason], [location])");
  void* handle = info[0].As<Napi::External<void>>().Data();
  void* cert = info[1].As<Napi::External<void>>().Data();
  std::string reason = info.Length() > 2 && info[2].IsString() ? info[2].As<Napi::String>().Utf8Value() : "";
  std::string location = info.Length() > 3 && info[3].IsString() ? info[3].As<Napi::String>().Utf8Value() : "";
  int errorCode = 0;
  pdf_document_sign(handle, cert, reason.c_str(), location.c_str(), &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to sign document: " + getErrorMessage(errorCode));
  return env.Undefined();
}

// ============================================================
// Regional Extraction
// ============================================================

Napi::Value ExtractImagesInRect(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  if (info.Length() < 6) throw Napi::TypeError::New(env, "Expected (handle, pageIndex, x, y, w, h)");
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  float x = info[2].As<Napi::Number>().FloatValue();
  float y = info[3].As<Napi::Number>().FloatValue();
  float w = info[4].As<Napi::Number>().FloatValue();
  float h = info[5].As<Napi::Number>().FloatValue();
  int errorCode = 0;
  void* images = pdf_document_extract_images_in_rect(handle, pageIndex, x, y, w, h, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!images) return Napi::Array::New(env, 0);
  int32_t count = pdf_oxide_image_count(images);
  Napi::Array result = Napi::Array::New(env, count);
  for (int32_t i = 0; i < count; i++) {
    Napi::Object img = Napi::Object::New(env);
    img.Set("width", Napi::Number::New(env, pdf_oxide_image_get_width(images, i)));
    img.Set("height", Napi::Number::New(env, pdf_oxide_image_get_height(images, i)));
    char* fmt = pdf_oxide_image_get_format(images, i);
    img.Set("format", fmt ? Napi::String::New(env, fmt) : env.Null());
    if (fmt) free_string(fmt);
    result.Set(i, img);
  }
  pdf_oxide_image_list_free(images);
  return result;
}

Napi::Value ExtractLinesInRect(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  if (info.Length() < 6) throw Napi::TypeError::New(env, "Expected (handle, pageIndex, x, y, w, h)");
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  float x = info[2].As<Napi::Number>().FloatValue();
  float y = info[3].As<Napi::Number>().FloatValue();
  float w = info[4].As<Napi::Number>().FloatValue();
  float h = info[5].As<Napi::Number>().FloatValue();
  int errorCode = 0;
  void* lines = pdf_document_extract_lines_in_rect(handle, pageIndex, x, y, w, h, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!lines) return Napi::Array::New(env, 0);
  int32_t count = pdf_oxide_line_count(lines);
  Napi::Array result = Napi::Array::New(env, count);
  for (int32_t i = 0; i < count; i++) {
    Napi::Object line = Napi::Object::New(env);
    char* text = pdf_oxide_line_get_text(lines, i, &errorCode);
    float lx, ly, lw, lh;
    pdf_oxide_line_get_bbox(lines, i, &lx, &ly, &lw, &lh, &errorCode);
    line.Set("text", text ? Napi::String::New(env, text) : env.Null());
    line.Set("x", Napi::Number::New(env, lx));
    line.Set("y", Napi::Number::New(env, ly));
    line.Set("width", Napi::Number::New(env, lw));
    line.Set("height", Napi::Number::New(env, lh));
    if (text) free_string(text);
    result.Set(i, line);
  }
  pdf_oxide_line_list_free(lines);
  return result;
}

Napi::Value ExtractPaths(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;
  void* paths = pdf_document_extract_paths(handle, pageIndex, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!paths) return Napi::Array::New(env, 0);
  int32_t count = pdf_oxide_path_count(paths);
  Napi::Array result = Napi::Array::New(env, count);
  for (int32_t i = 0; i < count; i++) {
    Napi::Object path = Napi::Object::New(env);
    float px, py, pw, ph;
    pdf_oxide_path_get_bbox(paths, i, &px, &py, &pw, &ph, &errorCode);
    path.Set("x", Napi::Number::New(env, px));
    path.Set("y", Napi::Number::New(env, py));
    path.Set("width", Napi::Number::New(env, pw));
    path.Set("height", Napi::Number::New(env, ph));
    path.Set("strokeWidth", Napi::Number::New(env, pdf_oxide_path_get_stroke_width(paths, i, &errorCode)));
    path.Set("hasStroke", Napi::Boolean::New(env, pdf_oxide_path_has_stroke(paths, i, &errorCode)));
    path.Set("hasFill", Napi::Boolean::New(env, pdf_oxide_path_has_fill(paths, i, &errorCode)));
    path.Set("operationCount", Napi::Number::New(env, pdf_oxide_path_get_operation_count(paths, i, &errorCode)));
    result.Set(i, path);
  }
  pdf_oxide_path_list_free(paths);
  return result;
}

Napi::Value ExtractTablesInRect(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  if (info.Length() < 6) throw Napi::TypeError::New(env, "Expected (handle, pageIndex, x, y, w, h)");
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  float x = info[2].As<Napi::Number>().FloatValue();
  float y = info[3].As<Napi::Number>().FloatValue();
  float w = info[4].As<Napi::Number>().FloatValue();
  float h = info[5].As<Napi::Number>().FloatValue();
  int errorCode = 0;
  void* tables = pdf_document_extract_tables_in_rect(handle, pageIndex, x, y, w, h, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!tables) return Napi::Array::New(env, 0);
  int32_t count = pdf_oxide_table_count(tables);
  Napi::Array result = Napi::Array::New(env, count);
  for (int32_t i = 0; i < count; i++) {
    Napi::Object table = Napi::Object::New(env);
    int32_t rows = pdf_oxide_table_get_row_count(tables, i, &errorCode);
    int32_t cols = pdf_oxide_table_get_col_count(tables, i, &errorCode);
    table.Set("rows", Napi::Number::New(env, rows));
    table.Set("cols", Napi::Number::New(env, cols));
    table.Set("hasHeader", Napi::Boolean::New(env, pdf_oxide_table_has_header(tables, i, &errorCode)));
    Napi::Array cells = Napi::Array::New(env, rows);
    for (int32_t r = 0; r < rows; r++) {
      Napi::Array row = Napi::Array::New(env, cols);
      for (int32_t c = 0; c < cols; c++) {
        char* cell = pdf_oxide_table_get_cell_text(tables, i, r, c, &errorCode);
        row.Set(c, cell ? Napi::String::New(env, cell) : env.Null());
        if (cell) free_string(cell);
      }
      cells.Set(r, row);
    }
    table.Set("cells", cells);
    result.Set(i, table);
  }
  pdf_oxide_table_list_free(tables);
  return result;
}

Napi::Value ExtractTextInRect(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  if (info.Length() < 6) throw Napi::TypeError::New(env, "Expected (handle, pageIndex, x, y, w, h)");
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  float x = info[2].As<Napi::Number>().FloatValue();
  float y = info[3].As<Napi::Number>().FloatValue();
  float w = info[4].As<Napi::Number>().FloatValue();
  float h = info[5].As<Napi::Number>().FloatValue();
  int errorCode = 0;
  char* text = pdf_document_extract_text_in_rect(handle, pageIndex, x, y, w, h, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  std::string result = text ? text : "";
  if (text) free_string(text);
  return Napi::String::New(env, result);
}

Napi::Value ExtractWordsInRect(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  if (info.Length() < 6) throw Napi::TypeError::New(env, "Expected (handle, pageIndex, x, y, w, h)");
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  float x = info[2].As<Napi::Number>().FloatValue();
  float y = info[3].As<Napi::Number>().FloatValue();
  float w = info[4].As<Napi::Number>().FloatValue();
  float h = info[5].As<Napi::Number>().FloatValue();
  int errorCode = 0;
  void* words = pdf_document_extract_words_in_rect(handle, pageIndex, x, y, w, h, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!words) return Napi::Array::New(env, 0);
  int32_t count = pdf_oxide_word_count(words);
  Napi::Array result = Napi::Array::New(env, count);
  for (int32_t i = 0; i < count; i++) {
    Napi::Object word = Napi::Object::New(env);
    char* text = pdf_oxide_word_get_text(words, i, &errorCode);
    float wx, wy, ww, wh;
    pdf_oxide_word_get_bbox(words, i, &wx, &wy, &ww, &wh, &errorCode);
    word.Set("text", text ? Napi::String::New(env, text) : env.Null());
    word.Set("x", Napi::Number::New(env, wx));
    word.Set("y", Napi::Number::New(env, wy));
    word.Set("width", Napi::Number::New(env, ww));
    word.Set("height", Napi::Number::New(env, wh));
    if (text) free_string(text);
    result.Set(i, word);
  }
  pdf_oxide_word_list_free(words);
  return result;
}

Napi::Value GetPageAnnotations(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;
  void* annotations = pdf_document_get_page_annotations(handle, pageIndex, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!annotations) return Napi::Array::New(env, 0);
  int32_t count = pdf_oxide_annotation_count(annotations);
  Napi::Array result = Napi::Array::New(env, count);
  for (int32_t i = 0; i < count; i++) {
    Napi::Object ann = Napi::Object::New(env);
    char* type = pdf_oxide_annotation_get_type(annotations, i);
    char* content = pdf_oxide_annotation_get_content(annotations, i);
    ann.Set("type", type ? Napi::String::New(env, type) : env.Null());
    ann.Set("content", content ? Napi::String::New(env, content) : env.Null());

    float ax, ay, aw, ah;
    pdf_oxide_annotation_get_rect(annotations, i, &ax, &ay, &aw, &ah, &errorCode);
    Napi::Object rect = Napi::Object::New(env);
    rect.Set("x", Napi::Number::New(env, ax));
    rect.Set("y", Napi::Number::New(env, ay));
    rect.Set("width", Napi::Number::New(env, aw));
    rect.Set("height", Napi::Number::New(env, ah));
    ann.Set("rect", rect);

    char* uri = pdf_oxide_link_annotation_get_uri(annotations, i, &errorCode);
    ann.Set("uri", uri ? Napi::String::New(env, uri) : env.Null());
    if (uri) free_string(uri);

    char* icon = pdf_oxide_text_annotation_get_icon_name(annotations, i, &errorCode);
    ann.Set("iconName", icon ? Napi::String::New(env, icon) : env.Null());
    if (icon) free_string(icon);

    int32_t quadCount = pdf_oxide_highlight_annotation_get_quad_points_count(annotations, i, &errorCode);
    ann.Set("quadPointsCount", Napi::Number::New(env, quadCount));

    if (type) free_string(type);
    if (content) free_string(content);
    result.Set(i, ann);
  }
  pdf_oxide_annotation_list_free(annotations);
  return result;
}

// ============================================================
// PDF Creation (missing wrappers)
// ============================================================

Napi::Value EditorImportFdfBytes(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  auto buf = info[1].As<Napi::Buffer<uint8_t>>();
  int errorCode = 0;
  pdf_editor_import_fdf_bytes(handle, buf.Data(), buf.Length(), &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to import FDF: " + getErrorMessage(errorCode));
  return env.Undefined();
}

Napi::Value EditorImportXfdfBytes(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  auto buf = info[1].As<Napi::Buffer<uint8_t>>();
  int errorCode = 0;
  pdf_editor_import_xfdf_bytes(handle, buf.Data(), buf.Length(), &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to import XFDF: " + getErrorMessage(errorCode));
  return env.Undefined();
}

Napi::Value FormImportFromFile(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  std::string filename = info[1].As<Napi::String>().Utf8Value();
  int errorCode = 0;
  pdf_form_import_from_file(handle, filename.c_str(), &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to import form: " + getErrorMessage(errorCode));
  return env.Undefined();
}

Napi::Value PdfFromHtml(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  std::string html = info[0].As<Napi::String>().Utf8Value();
  int errorCode = 0;
  void* handle = pdf_from_html(html.c_str(), &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to create PDF from HTML: " + getErrorMessage(errorCode));
  return Napi::External<void>::New(env, handle);
}

Napi::Value PdfFromImage(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  std::string path = info[0].As<Napi::String>().Utf8Value();
  int errorCode = 0;
  void* handle = pdf_from_image(path.c_str(), &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to create PDF from image: " + getErrorMessage(errorCode));
  return Napi::External<void>::New(env, handle);
}

Napi::Value PdfFromImageBytes(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  auto buf = info[0].As<Napi::Buffer<uint8_t>>();
  int errorCode = 0;
  void* handle = pdf_from_image_bytes(buf.Data(), (int32_t)buf.Length(), &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to create PDF from image bytes: " + getErrorMessage(errorCode));
  return Napi::External<void>::New(env, handle);
}

Napi::Value PdfFromMarkdown(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  std::string md = info[0].As<Napi::String>().Utf8Value();
  int errorCode = 0;
  void* handle = pdf_from_markdown(md.c_str(), &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to create PDF from Markdown: " + getErrorMessage(errorCode));
  return Napi::External<void>::New(env, handle);
}

Napi::Value PdfFromText(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  std::string text = info[0].As<Napi::String>().Utf8Value();
  int errorCode = 0;
  void* handle = pdf_from_text(text.c_str(), &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to create PDF from text: " + getErrorMessage(errorCode));
  return Napi::External<void>::New(env, handle);
}

Napi::Value PdfMerge(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  if (info.Length() < 1 || !info[0].IsArray()) throw Napi::TypeError::New(env, "Expected array of file paths");
  Napi::Array pathArr = info[0].As<Napi::Array>();
  uint32_t len = pathArr.Length();
  std::vector<std::string> pathStrs(len);
  std::vector<const char*> pathPtrs(len);
  for (uint32_t i = 0; i < len; i++) {
    pathStrs[i] = pathArr.Get(i).As<Napi::String>().Utf8Value();
    pathPtrs[i] = pathStrs[i].c_str();
  }
  int32_t dataLen = 0;
  int errorCode = 0;
  uint8_t* data = pdf_merge(pathPtrs.data(), (int32_t)len, &dataLen, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to merge PDFs: " + getErrorMessage(errorCode));
  if (!data || dataLen == 0) return Napi::Buffer<uint8_t>::New(env, 0);
  auto buf = Napi::Buffer<uint8_t>::Copy(env, data, dataLen);
  free_bytes(data);
  return buf;
}

// ============================================================
// Saving (missing wrappers)
// ============================================================

Napi::Value PdfSave(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  std::string path = info[1].As<Napi::String>().Utf8Value();
  int errorCode = 0;
  pdf_save(handle, path.c_str(), &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to save PDF: " + getErrorMessage(errorCode));
  return env.Undefined();
}

Napi::Value PdfSaveToBytes(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t dataLen = 0;
  int errorCode = 0;
  uint8_t* data = pdf_save_to_bytes(handle, &dataLen, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to save PDF to bytes: " + getErrorMessage(errorCode));
  if (!data || dataLen == 0) return Napi::Buffer<uint8_t>::New(env, 0);
  auto buf = Napi::Buffer<uint8_t>::Copy(env, data, dataLen);
  free_bytes(data);
  return buf;
}

// ============================================================
// Rendering (missing wrappers)
// ============================================================

Napi::Value PdfCreateRenderer(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  int dpi = info.Length() > 0 ? info[0].As<Napi::Number>().Int32Value() : 150;
  int format = info.Length() > 1 ? info[1].As<Napi::Number>().Int32Value() : 0;
  int quality = info.Length() > 2 ? info[2].As<Napi::Number>().Int32Value() : 90;
  bool antiAlias = info.Length() > 3 ? info[3].As<Napi::Boolean>().Value() : true;
  int errorCode = 0;
  void* renderer = pdf_create_renderer(dpi, format, quality, antiAlias, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to create renderer: " + getErrorMessage(errorCode));
  if (!renderer) return env.Null();
  return Napi::External<void>::New(env, renderer);
}

Napi::Value PdfGetRenderedImageData(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* image = info[0].As<Napi::External<void>>().Data();
  int32_t dataLen = 0;
  int errorCode = 0;
  uint8_t* data = pdf_get_rendered_image_data(image, &dataLen, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to get image data: " + getErrorMessage(errorCode));
  if (!data || dataLen == 0) return Napi::Buffer<uint8_t>::New(env, 0);
  auto buf = Napi::Buffer<uint8_t>::Copy(env, data, dataLen);
  free_bytes(data);
  return buf;
}

Napi::Value PdfGetRenderedImageHeight(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* image = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  int height = pdf_get_rendered_image_height(image, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return Napi::Number::New(env, height);
}

Napi::Value PdfGetRenderedImageWidth(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* image = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  int width = pdf_get_rendered_image_width(image, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return Napi::Number::New(env, width);
}

Napi::Value PdfRendererFree(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  if (info.Length() < 1 || !info[0].IsExternal()) return env.Undefined();
  void* renderer = info[0].As<Napi::External<void>>().Data();
  pdf_renderer_free(renderer);
  return env.Undefined();
}

Napi::Value PdfRenderPageRegion(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  if (info.Length() < 6) throw Napi::TypeError::New(env, "Expected (handle, pageIndex, x, y, w, h, [format])");
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  float x = info[2].As<Napi::Number>().FloatValue();
  float y = info[3].As<Napi::Number>().FloatValue();
  float w = info[4].As<Napi::Number>().FloatValue();
  float h = info[5].As<Napi::Number>().FloatValue();
  int format = info.Length() > 6 ? info[6].As<Napi::Number>().Int32Value() : 0;
  int errorCode = 0;
  void* image = pdf_render_page_region(handle, pageIndex, x, y, w, h, format, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to render page region: " + getErrorMessage(errorCode));
  if (!image) return env.Null();
  return Napi::External<void>::New(env, image);
}

// ============================================================
// Barcode (missing wrappers)
// ============================================================

Napi::Value BarcodeGetConfidence(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* barcode = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  float confidence = pdf_barcode_get_confidence(barcode, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return Napi::Number::New(env, confidence);
}

Napi::Value BarcodeGetData(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* barcode = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  char* data = pdf_barcode_get_data(barcode, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!data) return env.Null();
  std::string result(data);
  free_string(data);
  return Napi::String::New(env, result);
}

Napi::Value BarcodeGetFormat(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* barcode = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  int format = pdf_barcode_get_format(barcode, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return Napi::Number::New(env, format);
}

// ============================================================
// Timestamp/TSA (missing wrappers)
// ============================================================

Napi::Value CertificateGetValidity(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* cert = info[0].As<Napi::External<void>>().Data();
  int64_t notBefore = 0, notAfter = 0;
  int errorCode = 0;
  pdf_certificate_get_validity(cert, &notBefore, &notAfter, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  Napi::Object result = Napi::Object::New(env);
  result.Set("notBefore", Napi::Number::New(env, (double)notBefore));
  result.Set("notAfter", Napi::Number::New(env, (double)notAfter));
  return result;
}

Napi::Value CertificateLoadFromBytes(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  auto buf = info[0].As<Napi::Buffer<uint8_t>>();
  std::string password = info.Length() > 1 && info[1].IsString() ? info[1].As<Napi::String>().Utf8Value() : "";
  int errorCode = 0;
  void* cert = pdf_certificate_load_from_bytes(buf.Data(), (int32_t)buf.Length(), password.c_str(), &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to load certificate: " + getErrorMessage(errorCode));
  if (!cert) return env.Null();
  return Napi::External<void>::New(env, cert);
}

Napi::Value SignatureAddTimestamp(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* sig = info[0].As<Napi::External<void>>().Data();
  void* ts = info[1].As<Napi::External<void>>().Data();
  int errorCode = 0;
  bool ok = pdf_signature_add_timestamp(sig, ts, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return Napi::Boolean::New(env, ok);
}

Napi::Value SignatureGetTimestamp(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* sig = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  void* ts = pdf_signature_get_timestamp(sig, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!ts) return env.Null();
  return Napi::External<void>::New(env, ts);
}

Napi::Value SignatureHasTimestamp(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* sig = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  bool has = pdf_signature_has_timestamp(sig, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return Napi::Boolean::New(env, has);
}

Napi::Value TimestampFree(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  if (info.Length() < 1 || !info[0].IsExternal()) return env.Undefined();
  pdf_timestamp_free(info[0].As<Napi::External<void>>().Data());
  return env.Undefined();
}

Napi::Value TimestampGetHashAlgorithm(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* ts = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  int algo = pdf_timestamp_get_hash_algorithm(ts, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return Napi::Number::New(env, algo);
}

Napi::Value TimestampGetMessageImprint(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* ts = info[0].As<Napi::External<void>>().Data();
  size_t outLen = 0;
  int errorCode = 0;
  const uint8_t* data = pdf_timestamp_get_message_imprint(ts, &outLen, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!data || outLen == 0) return Napi::Buffer<uint8_t>::New(env, 0);
  return Napi::Buffer<uint8_t>::Copy(env, data, outLen);
}

Napi::Value TimestampGetPolicyOid(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* ts = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  char* oid = pdf_timestamp_get_policy_oid(ts, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!oid) return env.Null();
  std::string result(oid);
  free_string(oid);
  return Napi::String::New(env, result);
}

Napi::Value TimestampGetSerial(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* ts = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  char* serial = pdf_timestamp_get_serial(ts, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!serial) return env.Null();
  std::string result(serial);
  free_string(serial);
  return Napi::String::New(env, result);
}

Napi::Value TimestampGetTime(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* ts = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  int64_t time = pdf_timestamp_get_time(ts, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return Napi::Number::New(env, (double)time);
}

Napi::Value TimestampGetToken(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* ts = info[0].As<Napi::External<void>>().Data();
  size_t outLen = 0;
  int errorCode = 0;
  const uint8_t* data = pdf_timestamp_get_token(ts, &outLen, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!data || outLen == 0) return Napi::Buffer<uint8_t>::New(env, 0);
  return Napi::Buffer<uint8_t>::Copy(env, data, outLen);
}

Napi::Value TimestampGetTsaName(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* ts = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  char* name = pdf_timestamp_get_tsa_name(ts, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!name) return env.Null();
  std::string result(name);
  free_string(name);
  return Napi::String::New(env, result);
}

Napi::Value TimestampVerify(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* ts = info[0].As<Napi::External<void>>().Data();
  int errorCode = 0;
  bool valid = pdf_timestamp_verify(ts, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return Napi::Boolean::New(env, valid);
}

Napi::Value TsaClientCreate(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  std::string url = info[0].As<Napi::String>().Utf8Value();
  std::string username = info.Length() > 1 && info[1].IsString() ? info[1].As<Napi::String>().Utf8Value() : "";
  std::string password = info.Length() > 2 && info[2].IsString() ? info[2].As<Napi::String>().Utf8Value() : "";
  int timeout = info.Length() > 3 ? info[3].As<Napi::Number>().Int32Value() : 30;
  int hashAlgo = info.Length() > 4 ? info[4].As<Napi::Number>().Int32Value() : 0;
  bool useNonce = info.Length() > 5 ? info[5].As<Napi::Boolean>().Value() : true;
  bool certReq = info.Length() > 6 ? info[6].As<Napi::Boolean>().Value() : true;
  int errorCode = 0;
  void* client = pdf_tsa_client_create(url.c_str(),
    username.empty() ? nullptr : username.c_str(),
    password.empty() ? nullptr : password.c_str(),
    timeout, hashAlgo, useNonce, certReq, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to create TSA client: " + getErrorMessage(errorCode));
  if (!client) return env.Null();
  return Napi::External<void>::New(env, client);
}

Napi::Value TsaClientFree(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  if (info.Length() < 1 || !info[0].IsExternal()) return env.Undefined();
  pdf_tsa_client_free(info[0].As<Napi::External<void>>().Data());
  return env.Undefined();
}

Napi::Value TsaRequestTimestamp(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* client = info[0].As<Napi::External<void>>().Data();
  auto buf = info[1].As<Napi::Buffer<uint8_t>>();
  int errorCode = 0;
  void* ts = pdf_tsa_request_timestamp(client, buf.Data(), buf.Length(), &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to request timestamp: " + getErrorMessage(errorCode));
  if (!ts) return env.Null();
  return Napi::External<void>::New(env, ts);
}

Napi::Value TsaRequestTimestampHash(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* client = info[0].As<Napi::External<void>>().Data();
  auto buf = info[1].As<Napi::Buffer<uint8_t>>();
  int hashAlgo = info.Length() > 2 ? info[2].As<Napi::Number>().Int32Value() : 0;
  int errorCode = 0;
  void* ts = pdf_tsa_request_timestamp_hash(client, buf.Data(), buf.Length(), hashAlgo, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "Failed to request timestamp hash: " + getErrorMessage(errorCode));
  if (!ts) return env.Null();
  return Napi::External<void>::New(env, ts);
}

// ============================================================
// Compliance (missing wrappers)
// ============================================================

Napi::Value ValidatePdfALevel(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* document = info[0].As<Napi::External<void>>().Data();
  int32_t level = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;
  void* results = pdf_validate_pdf_a_level(document, level, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "PDF/A validation failed: " + getErrorMessage(errorCode));
  if (!results) return env.Null();

  Napi::Object obj = Napi::Object::New(env);
  int ec2 = 0;
  obj.Set("compliant", Napi::Boolean::New(env, pdf_pdf_a_is_compliant(results)));

  int errCount = pdf_pdf_a_error_count(results);
  Napi::Array errors = Napi::Array::New(env, errCount);
  for (int i = 0; i < errCount; i++) {
    char* msg = pdf_pdf_a_get_error(results, i, &ec2);
    errors.Set(i, msg ? Napi::String::New(env, msg) : env.Null());
    if (msg) free_string(msg);
  }
  obj.Set("errors", errors);

  int warnCount = pdf_pdf_a_warning_count(results);
  Napi::Array warnings = Napi::Array::New(env, warnCount);
  obj.Set("warnings", warnings);

  pdf_pdf_a_results_free(results);
  return obj;
}

Napi::Value ValidatePdfXLevel(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* document = info[0].As<Napi::External<void>>().Data();
  int32_t level = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;
  void* results = pdf_validate_pdf_x_level(document, level, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "PDF/X validation failed: " + getErrorMessage(errorCode));
  if (!results) return env.Null();

  Napi::Object obj = Napi::Object::New(env);
  int ec2 = 0;
  obj.Set("compliant", Napi::Boolean::New(env, pdf_pdf_x_is_compliant(results)));

  int errCount = pdf_pdf_x_error_count(results);
  Napi::Array errors = Napi::Array::New(env, errCount);
  for (int i = 0; i < errCount; i++) {
    char* msg = pdf_pdf_x_get_error(results, i, &ec2);
    errors.Set(i, msg ? Napi::String::New(env, msg) : env.Null());
    if (msg) free_string(msg);
  }
  obj.Set("errors", errors);
  obj.Set("warnings", Napi::Array::New(env, 0));

  pdf_pdf_x_results_free(results);
  return obj;
}

Napi::Value ValidatePdfUA(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* document = info[0].As<Napi::External<void>>().Data();
  PdfUaLevel level = info.Length() > 1 ? (PdfUaLevel)info[1].As<Napi::Number>().Int32Value() : PDF_UA_LEVEL_1;
  int errorCode = 0;
  void* results = pdf_validate_pdf_ua(document, level, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, "PDF/UA validation failed: " + getErrorMessage(errorCode));
  if (!results) return env.Null();

  Napi::Object obj = Napi::Object::New(env);
  int ec2 = 0;
  obj.Set("accessible", Napi::Boolean::New(env, pdf_pdf_ua_is_accessible(results)));

  int errCount = pdf_pdf_ua_error_count(results);
  Napi::Array errors = Napi::Array::New(env, errCount);
  for (int i = 0; i < errCount; i++) {
    char* msg = pdf_pdf_ua_get_error(results, i, &ec2);
    errors.Set(i, msg ? Napi::String::New(env, msg) : env.Null());
    if (msg) free_string(msg);
  }
  obj.Set("errors", errors);

  int warnCount = pdf_pdf_ua_warning_count(results);
  Napi::Array warnings = Napi::Array::New(env, warnCount);
  for (int i = 0; i < warnCount; i++) {
    char* msg = pdf_pdf_ua_get_warning(results, i, &ec2);
    warnings.Set(i, msg ? Napi::String::New(env, msg) : env.Null());
    if (msg) free_string(msg);
  }
  obj.Set("warnings", warnings);

  // Stats
  int32_t sStruct = 0, sImages = 0, sTables = 0, sForms = 0, sAnnot = 0, sPages = 0;
  pdf_pdf_ua_get_stats(results, &sStruct, &sImages, &sTables, &sForms, &sAnnot, &sPages, &ec2);
  Napi::Object stats = Napi::Object::New(env);
  stats.Set("structureElements", Napi::Number::New(env, sStruct));
  stats.Set("images", Napi::Number::New(env, sImages));
  stats.Set("tables", Napi::Number::New(env, sTables));
  stats.Set("formFields", Napi::Number::New(env, sForms));
  stats.Set("annotations", Napi::Number::New(env, sAnnot));
  stats.Set("pages", Napi::Number::New(env, sPages));
  obj.Set("stats", stats);

  pdf_pdf_ua_results_free(results);
  return obj;
}

// ============================================================
// Page/Element/Accessor (missing wrappers)
// ============================================================

Napi::Value GetPageElements(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;
  void* elements = pdf_page_get_elements(handle, pageIndex, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!elements) return Napi::Array::New(env, 0);
  int32_t count = pdf_oxide_element_count(elements);
  Napi::Array result = Napi::Array::New(env, count);
  for (int32_t i = 0; i < count; i++) {
    Napi::Object elem = Napi::Object::New(env);
    char* type = pdf_oxide_element_get_type(elements, i, &errorCode);
    char* text = pdf_oxide_element_get_text(elements, i, &errorCode);
    elem.Set("type", type ? Napi::String::New(env, type) : env.Null());
    elem.Set("text", text ? Napi::String::New(env, text) : env.Null());
    float ex, ey, ew, eh;
    pdf_oxide_element_get_rect(elements, i, &ex, &ey, &ew, &eh, &errorCode);
    elem.Set("x", Napi::Number::New(env, ex));
    elem.Set("y", Napi::Number::New(env, ey));
    elem.Set("width", Napi::Number::New(env, ew));
    elem.Set("height", Napi::Number::New(env, eh));
    if (type) free_string(type);
    if (text) free_string(text);
    result.Set(i, elem);
  }
  pdf_oxide_elements_free(elements);
  return result;
}

Napi::Value GetPageWidth(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;
  float width = pdf_page_get_width(handle, pageIndex, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return Napi::Number::New(env, width);
}

Napi::Value GetPageHeight(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;
  float height = pdf_page_get_height(handle, pageIndex, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return Napi::Number::New(env, height);
}

Napi::Value GetPageRotation(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;
  int32_t rotation = pdf_page_get_rotation(handle, pageIndex, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  return Napi::Number::New(env, rotation);
}

Napi::Value GetEmbeddedFonts(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;
  void* fonts = pdf_document_get_embedded_fonts(handle, pageIndex, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!fonts) return Napi::Array::New(env, 0);
  int32_t count = pdf_oxide_font_count(fonts);
  Napi::Array result = Napi::Array::New(env, count);
  for (int32_t i = 0; i < count; i++) {
    Napi::Object font = Napi::Object::New(env);
    char* name = pdf_oxide_font_get_name(fonts, i, &errorCode);
    char* type = pdf_oxide_font_get_type(fonts, i, &errorCode);
    char* encoding = pdf_oxide_font_get_encoding(fonts, i, &errorCode);
    font.Set("name", name ? Napi::String::New(env, name) : env.Null());
    font.Set("type", type ? Napi::String::New(env, type) : env.Null());
    font.Set("encoding", encoding ? Napi::String::New(env, encoding) : env.Null());
    font.Set("isEmbedded", Napi::Boolean::New(env, pdf_oxide_font_is_embedded(fonts, i)));
    font.Set("isSubset", Napi::Boolean::New(env, pdf_oxide_font_is_subset(fonts, i, &errorCode) != 0));
    font.Set("size", Napi::Number::New(env, pdf_oxide_font_get_size(fonts, i, &errorCode)));
    if (name) free_string(name);
    if (type) free_string(type);
    if (encoding) free_string(encoding);
    result.Set(i, font);
  }
  pdf_oxide_font_list_free(fonts);
  return result;
}

Napi::Value GetEmbeddedImages(const Napi::CallbackInfo& info) {
  Napi::Env env = info.Env();
  void* handle = info[0].As<Napi::External<void>>().Data();
  int32_t pageIndex = info[1].As<Napi::Number>().Int32Value();
  int errorCode = 0;
  void* images = pdf_document_get_embedded_images(handle, pageIndex, &errorCode);
  if (errorCode != 0) throw Napi::Error::New(env, getErrorMessage(errorCode));
  if (!images) return Napi::Array::New(env, 0);
  int32_t count = pdf_oxide_image_count(images);
  Napi::Array result = Napi::Array::New(env, count);
  for (int32_t i = 0; i < count; i++) {
    Napi::Object img = Napi::Object::New(env);
    img.Set("width", Napi::Number::New(env, pdf_oxide_image_get_width(images, i)));
    img.Set("height", Napi::Number::New(env, pdf_oxide_image_get_height(images, i)));
    char* fmt = pdf_oxide_image_get_format(images, i);
    img.Set("format", fmt ? Napi::String::New(env, fmt) : env.Null());
    if (fmt) free_string(fmt);
    char* cs = pdf_oxide_image_get_colorspace(images, i, &errorCode);
    img.Set("colorspace", cs ? Napi::String::New(env, cs) : env.Null());
    if (cs) free_string(cs);
    img.Set("bitsPerComponent", Napi::Number::New(env, pdf_oxide_image_get_bits_per_component(images, i, &errorCode)));
    result.Set(i, img);
  }
  pdf_oxide_image_list_free(images);
  return result;
}

// ============================================================
// Initialize the addon
// ============================================================

Napi::Object Init(Napi::Env env, Napi::Object exports) {
  // Logging
  exports.Set("setLogLevel", Napi::Function::New(env, SetLogLevel));
  exports.Set("getLogLevel", Napi::Function::New(env, GetLogLevel));

  // Document Operations
  exports.Set("openDocument", Napi::Function::New(env, OpenDocument));
  exports.Set("openFromBuffer", Napi::Function::New(env, OpenFromBuffer));
  exports.Set("openWithPassword", Napi::Function::New(env, OpenWithPassword));
  exports.Set("closeDocument", Napi::Function::New(env, CloseDocument));
  exports.Set("getPageCount", Napi::Function::New(env, GetPageCount));
  exports.Set("getVersion", Napi::Function::New(env, GetVersion));
  exports.Set("hasStructureTree", Napi::Function::New(env, HasStructureTree));
  exports.Set("extractText", Napi::Function::New(env, ExtractText));
  exports.Set("toMarkdown", Napi::Function::New(env, ToMarkdown));
  exports.Set("toHtml", Napi::Function::New(env, ToHtml));
  exports.Set("toPlainText", Napi::Function::New(env, ToPlainText));
  exports.Set("toMarkdownAll", Napi::Function::New(env, ToMarkdownAll));

  // Search Operations
  exports.Set("searchPage", Napi::Function::New(env, SearchPage));
  exports.Set("searchAll", Napi::Function::New(env, SearchAll));
  exports.Set("searchResultCount", Napi::Function::New(env, SearchResultCount));
  exports.Set("searchResultFree", Napi::Function::New(env, SearchResultFree));

  // Rendering Operations
  exports.Set("createRenderer", Napi::Function::New(env, CreateRenderer));
  exports.Set("freeRenderer", Napi::Function::New(env, FreeRenderer));
  exports.Set("renderPage", Napi::Function::New(env, RenderPage));
  exports.Set("renderThumbnail", Napi::Function::New(env, RenderThumbnail));
  exports.Set("renderedImageToBase64", Napi::Function::New(env, RenderedImageToBase64));
  exports.Set("freeRenderedImage", Napi::Function::New(env, FreeRenderedImage));

  // OCR Operations
  exports.Set("createOCREngine", Napi::Function::New(env, CreateOCREngine));
  exports.Set("freeOCREngine", Napi::Function::New(env, FreeOCREngine));
  exports.Set("pageNeedsOCR", Napi::Function::New(env, PageNeedsOCR));
  exports.Set("ocrExtractText", Napi::Function::New(env, OCRExtractText));

  // Compliance Operations
  exports.Set("validatePdfA", Napi::Function::New(env, ValidatePdfA));
  exports.Set("pdfAIsCompliant", Napi::Function::New(env, PdfAIsCompliant));
  exports.Set("pdfAGetReport", Napi::Function::New(env, PdfAGetReport));
  exports.Set("freePdfAResults", Napi::Function::New(env, FreePdfAResults));

  // Signature Operations (comprehensive)
  exports.Set("getSignatureCount", Napi::Function::New(env, GetSignatureCount));
  exports.Set("getSignatureInfo", Napi::Function::New(env, GetSignatureInfo));
  exports.Set("verifyAllSignatures", Napi::Function::New(env, VerifyAllSignatures));

  // Detailed Annotation Accessors
  exports.Set("getAnnotationsDetailed", Napi::Function::New(env, GetAnnotationsDetailed));

  // Rendering variants
  exports.Set("estimateRenderTime", Napi::Function::New(env, EstimateRenderTime));
  exports.Set("renderPageZoom", Napi::Function::New(env, RenderPageZoom));
  exports.Set("renderPageToFile", Napi::Function::New(env, RenderPageToFile));
  exports.Set("renderedImageWidth", Napi::Function::New(env, RenderedImageWidth));
  exports.Set("renderedImageHeight", Napi::Function::New(env, RenderedImageHeight));

  // Barcode Operations
  exports.Set("generateQRCode", Napi::Function::New(env, GenerateQRCode));
  exports.Set("generateBarcode", Napi::Function::New(env, GenerateBarcode));
  exports.Set("barcodeGetSVG", Napi::Function::New(env, BarcodeGetSVG));
  exports.Set("freeBarcode", Napi::Function::New(env, FreeBarcode));

  // Document Editor Operations
  exports.Set("editorOpen", Napi::Function::New(env, EditorOpen));
  exports.Set("editorFree", Napi::Function::New(env, EditorFree));
  exports.Set("editorSave", Napi::Function::New(env, EditorSave));
  exports.Set("editorGetPageCount", Napi::Function::New(env, EditorGetPageCount));
  exports.Set("editorIsModified", Napi::Function::New(env, EditorIsModified));
  exports.Set("editorSetTitle", Napi::Function::New(env, EditorSetTitle));
  exports.Set("editorSetAuthor", Napi::Function::New(env, EditorSetAuthor));
  exports.Set("editorDeletePage", Napi::Function::New(env, EditorDeletePage));
  exports.Set("editorMovePage", Napi::Function::New(env, EditorMovePage));
  exports.Set("editorSetPageRotation", Napi::Function::New(env, EditorSetPageRotation));
  exports.Set("editorMergeFrom", Napi::Function::New(env, EditorMergeFrom));
  exports.Set("editorFlattenForms", Napi::Function::New(env, EditorFlattenForms));
  exports.Set("editorFlattenAnnotations", Napi::Function::New(env, EditorFlattenAnnotations));
  // Missing Document Editor
  exports.Set("editorGetCreationDate", Napi::Function::New(env, EditorGetCreationDate));
  exports.Set("editorGetProducer", Napi::Function::New(env, EditorGetProducer));
  exports.Set("editorGetVersion", Napi::Function::New(env, EditorGetVersion));
  exports.Set("editorSaveEncrypted", Napi::Function::New(env, EditorSaveEncrypted));
  exports.Set("editorSetCreationDate", Napi::Function::New(env, EditorSetCreationDate));
  exports.Set("editorSetFormFieldValue", Napi::Function::New(env, EditorSetFormFieldValue));

  // PDF Document Editing (artifact removal, signing, form data)
  exports.Set("documentEraseArtifacts", Napi::Function::New(env, DocumentEraseArtifacts));
  exports.Set("documentEraseFooter", Napi::Function::New(env, DocumentEraseFooter));
  exports.Set("documentEraseHeader", Napi::Function::New(env, DocumentEraseHeader));
  exports.Set("documentExportFormData", Napi::Function::New(env, DocumentExportFormData));
  exports.Set("documentImportFormData", Napi::Function::New(env, DocumentImportFormData));
  exports.Set("documentRemoveArtifacts", Napi::Function::New(env, DocumentRemoveArtifacts));
  exports.Set("documentRemoveFooters", Napi::Function::New(env, DocumentRemoveFooters));
  exports.Set("documentRemoveHeaders", Napi::Function::New(env, DocumentRemoveHeaders));
  exports.Set("documentSign", Napi::Function::New(env, DocumentSign));

  // Regional Extraction
  exports.Set("extractImagesInRect", Napi::Function::New(env, ExtractImagesInRect));
  exports.Set("extractLinesInRect", Napi::Function::New(env, ExtractLinesInRect));
  exports.Set("extractPaths", Napi::Function::New(env, ExtractPaths));
  exports.Set("extractTablesInRect", Napi::Function::New(env, ExtractTablesInRect));
  exports.Set("extractTextInRect", Napi::Function::New(env, ExtractTextInRect));
  exports.Set("extractWordsInRect", Napi::Function::New(env, ExtractWordsInRect));
  exports.Set("getPageAnnotations", Napi::Function::New(env, GetPageAnnotations));

  // PDF Creation
  exports.Set("editorImportFdfBytes", Napi::Function::New(env, EditorImportFdfBytes));
  exports.Set("editorImportXfdfBytes", Napi::Function::New(env, EditorImportXfdfBytes));
  exports.Set("formImportFromFile", Napi::Function::New(env, FormImportFromFile));
  exports.Set("pdfFromHtml", Napi::Function::New(env, PdfFromHtml));
  exports.Set("pdfFromImage", Napi::Function::New(env, PdfFromImage));
  exports.Set("pdfFromImageBytes", Napi::Function::New(env, PdfFromImageBytes));
  exports.Set("pdfFromMarkdown", Napi::Function::New(env, PdfFromMarkdown));
  exports.Set("pdfFromText", Napi::Function::New(env, PdfFromText));
  exports.Set("pdfMerge", Napi::Function::New(env, PdfMerge));

  // Saving
  exports.Set("pdfSave", Napi::Function::New(env, PdfSave));
  exports.Set("pdfSaveToBytes", Napi::Function::New(env, PdfSaveToBytes));

  // Rendering (additional)
  exports.Set("pdfCreateRenderer", Napi::Function::New(env, PdfCreateRenderer));
  exports.Set("pdfGetRenderedImageData", Napi::Function::New(env, PdfGetRenderedImageData));
  exports.Set("pdfGetRenderedImageHeight", Napi::Function::New(env, PdfGetRenderedImageHeight));
  exports.Set("pdfGetRenderedImageWidth", Napi::Function::New(env, PdfGetRenderedImageWidth));
  exports.Set("pdfRendererFree", Napi::Function::New(env, PdfRendererFree));
  exports.Set("pdfRenderPageRegion", Napi::Function::New(env, PdfRenderPageRegion));

  // Barcode (additional)
  exports.Set("barcodeGetConfidence", Napi::Function::New(env, BarcodeGetConfidence));
  exports.Set("barcodeGetData", Napi::Function::New(env, BarcodeGetData));
  exports.Set("barcodeGetFormat", Napi::Function::New(env, BarcodeGetFormat));

  // Timestamp/TSA
  exports.Set("certificateGetValidity", Napi::Function::New(env, CertificateGetValidity));
  exports.Set("certificateLoadFromBytes", Napi::Function::New(env, CertificateLoadFromBytes));
  exports.Set("signatureAddTimestamp", Napi::Function::New(env, SignatureAddTimestamp));
  exports.Set("signatureGetTimestamp", Napi::Function::New(env, SignatureGetTimestamp));
  exports.Set("signatureHasTimestamp", Napi::Function::New(env, SignatureHasTimestamp));
  exports.Set("timestampFree", Napi::Function::New(env, TimestampFree));
  exports.Set("timestampGetHashAlgorithm", Napi::Function::New(env, TimestampGetHashAlgorithm));
  exports.Set("timestampGetMessageImprint", Napi::Function::New(env, TimestampGetMessageImprint));
  exports.Set("timestampGetPolicyOid", Napi::Function::New(env, TimestampGetPolicyOid));
  exports.Set("timestampGetSerial", Napi::Function::New(env, TimestampGetSerial));
  exports.Set("timestampGetTime", Napi::Function::New(env, TimestampGetTime));
  exports.Set("timestampGetToken", Napi::Function::New(env, TimestampGetToken));
  exports.Set("timestampGetTsaName", Napi::Function::New(env, TimestampGetTsaName));
  exports.Set("timestampVerify", Napi::Function::New(env, TimestampVerify));
  exports.Set("tsaClientCreate", Napi::Function::New(env, TsaClientCreate));
  exports.Set("tsaClientFree", Napi::Function::New(env, TsaClientFree));
  exports.Set("tsaRequestTimestamp", Napi::Function::New(env, TsaRequestTimestamp));
  exports.Set("tsaRequestTimestampHash", Napi::Function::New(env, TsaRequestTimestampHash));

  // Compliance (additional)
  exports.Set("validatePdfALevel", Napi::Function::New(env, ValidatePdfALevel));
  exports.Set("validatePdfXLevel", Napi::Function::New(env, ValidatePdfXLevel));
  exports.Set("validatePdfUA", Napi::Function::New(env, ValidatePdfUA));

  // Page/Element/Accessor
  exports.Set("getPageElements", Napi::Function::New(env, GetPageElements));
  exports.Set("getPageWidth", Napi::Function::New(env, GetPageWidth));
  exports.Set("getPageHeight", Napi::Function::New(env, GetPageHeight));
  exports.Set("getPageRotation", Napi::Function::New(env, GetPageRotation));
  exports.Set("getEmbeddedFonts", Napi::Function::New(env, GetEmbeddedFonts));
  exports.Set("getEmbeddedImages", Napi::Function::New(env, GetEmbeddedImages));

  // Form Fields
  exports.Set("getFormFields", Napi::Function::New(env, GetFormFields));

  // Advanced Extraction
  exports.Set("extractWords", Napi::Function::New(env, ExtractWords));
  exports.Set("extractTextLines", Napi::Function::New(env, ExtractTextLines));
  exports.Set("extractTables", Napi::Function::New(env, ExtractTables));

  // Full Document Conversion + Properties
  exports.Set("extractAllText", Napi::Function::New(env, ExtractAllText));
  exports.Set("toHtmlAll", Napi::Function::New(env, ToHtmlAll));
  exports.Set("toPlainTextAll", Napi::Function::New(env, ToPlainTextAll));
  exports.Set("isEncrypted", Napi::Function::New(env, IsEncrypted));
  exports.Set("getPageLabels", Napi::Function::New(env, GetPageLabels));
  exports.Set("getXmpMetadata", Napi::Function::New(env, GetXmpMetadata));
  exports.Set("getOutline", Napi::Function::New(env, GetOutline));

  // XFA Operations
  exports.Set("hasXFA", Napi::Function::New(env, HasXFA));
  exports.Set("parseXFAForm", Napi::Function::New(env, ParseXFAForm));
  exports.Set("freeXFAForm", Napi::Function::New(env, FreeXFAForm));

  // Hybrid ML Operations
  exports.Set("analyzePage", Napi::Function::New(env, AnalyzePage));
  exports.Set("analysisGetComplexity", Napi::Function::New(env, AnalysisGetComplexity));
  exports.Set("analysisGetComplexityScore", Napi::Function::New(env, AnalysisGetComplexityScore));
  exports.Set("analysisGetContentType", Napi::Function::New(env, AnalysisGetContentType));
  exports.Set("freeAnalysisResult", Napi::Function::New(env, FreeAnalysisResult));
  exports.Set("mlGetStatus", Napi::Function::New(env, MLGetStatus));
  exports.Set("mlModelAvailable", Napi::Function::New(env, MLModelAvailable));

  return exports;
}

NODE_API_MODULE(pdf_oxide, Init)
