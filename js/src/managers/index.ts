/**
 * PDF Oxide Managers - Specialized facades for domain-specific operations
 *
 * This module provides manager classes that encapsulate domain-specific
 * operations on PDF documents, offering a cleaner and more organized API
 * compared to working directly with documents and pages.
 *
 * @example
 * ```typescript
 * import {
 *   OutlineManager,
 *   MetadataManager,
 *   ExtractionManager,
 *   SearchManager,
 *   SecurityManager,
 *   AnnotationManager,
 *   LayerManager,
 *   RenderingManager,
 * } from 'pdf_oxide';
 *
 * const doc = PdfDocument.open('document.pdf');
 *
 * // Metadata operations
 * const metadataManager = new MetadataManager(doc);
 * console.log(metadataManager.getTitle());
 *
 * // Text extraction
 * const extractionManager = new ExtractionManager(doc);
 * const text = extractionManager.extractAllText();
 *
 * // Search operations
 * const searchManager = new SearchManager(doc);
 * const results = searchManager.searchAll('keyword');
 *
 * // Page annotations
 * const page = doc.getPage(0);
 * const annotationManager = new AnnotationManager(page);
 * const highlights = annotationManager.getHighlights();
 * ```
 */

// Core Managers
export { OutlineManager, type OutlineItem } from './outline-manager.js';
export {
  MetadataManager,
  type MetadataComparison,
  type ValidationResult,
} from './metadata-manager.js';
export {
  ExtractionManager,
  type ContentStatistics,
  type SearchMatch,
} from './extraction-manager.js';
export {
  SearchManager,
  type SearchResult,
  type SearchStatistics,
  type SearchCapabilities,
} from './search-manager.js';
export {
  SecurityManager,
  type PermissionsSummary,
  type SecurityLevel,
  type AccessibilityValidation,
} from './security-manager.js';
export {
  AnnotationManager,
  type Annotation,
  type AnnotationStatistics,
  type AnnotationValidation,
} from './annotation-manager.js';
export {
  LayerManager,
  type Layer,
  type LayerHierarchy,
  type LayerStatistics,
  type LayerValidation,
} from './layer-manager.js';
export {
  RenderingManager,
  RenderOptions,
  type RenderOptionsConfig,
  type PageDimensions,
  type PageBox,
  type RenderingStatistics,
  type PageResources,
} from './rendering-manager.js';
export {
  PageManager,
  type PageInfo,
  type PageRange,
  type PageStatistics,
} from './page-manager.js';
export {
  ContentManager,
  type ContentAnalysis,
} from './content-manager.js';

// Phase 2.4: Stream API support
export {
  SearchStream,
  ExtractionStream,
  MetadataStream,
  createSearchStream,
  createExtractionStream,
  createMetadataStream,
  type SearchResultData,
  type ExtractionProgressData,
  type PageMetadataData,
} from './streams.js';

// Phase 2.5: Batch Processing API
export {
  BatchManager,
  type BatchDocument,
  type BatchProgress,
  type BatchResult,
  type BatchOptions,
  type BatchStatistics,
} from './batch-manager.js';

// Phase 1 Expansion: Result Accessors and Forms
export {
  ResultAccessorsManager,
  type SearchResultProperties,
  type FontProperties,
  type ImageProperties,
  type AnnotationProperties,
} from '../result-accessors-manager.js';
export {
  FormFieldManager,
  FormFieldType,
  FieldVisibility,
  type FormField,
  type FormFieldConfig,
} from '../form-field-manager.js';

// Canonical Managers (Phase 9 consolidation)
export {
  OcrManager,
  OCRManager,
  OcrDetectionMode,
  type OcrConfig,
  type OcrSpan,
  type OcrPageAnalysis,
} from './ocr-manager.js';
export {
  SignatureManager,
  SignatureAlgorithm,
  DigestAlgorithm,
  SignatureType,
  CertificationPermission,
  CertificateFormat,
  TimestampStatus,
  FfiDigestAlgorithm,
  FfiSignatureSubFilter,
  type DigitalSignature,
  type SignatureField,
  type SignatureValidationResult,
  type SignatureConfig,
  type Certificate,
  type Signature,
  type CertificateInfo,
  type CertificateChain,
  type LoadedCertificate,
  type SignatureAppearance,
  type SignatureFieldConfig,
  type SigningOptions,
  type TimestampConfig,
  type SigningResult,
  type TimestampResult,
  type SigningCredentials,
  type SignOptions,
} from './signature-manager.js';
export {
  XfaManager,
  XFAManager,
  XfaFormType,
  XfaFieldType,
  XfaValidationType,
  XfaBindingType,
  type XfaField,
  type XfaDataset,
  type XfaFieldConfig,
  type XfaTemplateConfig,
  type XfaSubformConfig,
  type XfaScriptConfig,
  type XfaCreationResult,
  type XfaDataOptions,
  type XfaFieldHandle,
} from './xfa-manager.js';
export {
  ComplianceManager,
  PdfALevel,
  PdfXLevel,
  PdfUALevel,
  ComplianceIssueType,
  IssueSeverity,
  type ComplianceIssue,
  type ComplianceValidationResult,
} from './compliance-manager.js';
export {
  BarcodeManager,
  BarcodeFormat,
  BarcodeErrorCorrection,
  QrErrorCorrection,
  type DetectedBarcode,
  type BarcodeGenerationConfig,
} from './barcode-manager.js';
export {
  CacheManager,
  type CacheStatistics as CacheStats,
} from './cache-manager.js';
export {
  EditingManager,
  type RedactionRect,
  type RgbColor,
  type ApplyRedactionsOptions,
  type ScrubMetadataOptions,
} from './editing-manager.js';
export {
  AccessibilityManager,
  type StructureElement,
  type StructureTree,
  type AutoTagResult,
} from './accessibility-manager.js';
export {
  OptimizationManager,
  type OptimizationResult,
} from './optimization-manager.js';
export {
  EnterpriseManager,
  BatesPosition,
  StampAlignment,
  DifferenceType,
  type Difference,
  type PageComparisonResult,
  type DocumentComparisonResult,
} from './enterprise-manager.js';
