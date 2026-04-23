/**
 * Common type definitions and utilities
 */

/** A table extracted from a PDF page. */
export interface Table {
  /** Number of rows. */
  rows: number;
  /** Number of columns. */
  cols: number;
  /** True if the first row is a header row. */
  hasHeader: boolean;
  /** Cell text: cells[row][col]. Individual cells may be `null` when the
   *  native binding has no text for that position (missing cell, decoding
   *  failure, etc.). */
  cells: (string | null)[][];
}

// Re-export commonly used native types
export type {
  Annotation,
  CircleAnnotation,
  Color,
  DocumentInfo,
  EmbeddedFile,
  HighlightAnnotation,
  InkAnnotation,
  LineAnnotation,
  LinkAnnotation,
  Metadata,
  NativePdf,
  NativePdfDocument,
  NativePdfPage,
  PdfElement,
  PdfImage,
  PdfPath,
  PdfTable,
  PdfTableCell,
  PdfText,
  Point,
  PolygonAnnotation,
  Rect,
  SearchOptions,
  SearchResult,
  SquareAnnotation,
  TextAnnotation,
} from './native-bindings';

/**
 * Page range specification for document operations
 */
export interface PageRange {
  startPage?: number;
  endPage?: number;
  pages?: number[];
}

/**
 * Generic extraction result with metadata
 */
export interface ExtractionResult<T> {
  data: T;
  pageIndex: number;
  timestamp: Date;
  processingTimeMs?: number;
}

/**
 * Async operation callback function type
 */
export type AsyncOperationCallback<T> = (err: Error | null, result?: T) => void;

/**
 * Manager configuration interface for all managers
 */
export interface ManagerConfig {
  maxCacheSize?: number;
  cacheExpirationMs?: number;
  enableCaching?: boolean;
  timeout?: number;
  retryAttempts?: number;
  retryDelayMs?: number;
}

/**
 * Batch operation options
 */
export interface BatchOptions {
  batchSize?: number;
  parallel?: boolean;
  maxParallel?: number;
  progressCallback?: (processed: number, total: number) => void;
  continueOnError?: boolean;
}

/**
 * Error details for exception context
 */
export interface PdfErrorDetails {
  timestamp?: string;
  operation?: string;
  context?: Record<string, any>;
  originalError?: Error;
  stack?: string;
}

/**
 * Optional content (layers) information
 */
export interface OptionalContent {
  id: string;
  name: string;
  visible: boolean;
  locked?: boolean;
  printable?: boolean;
  exportable?: boolean;
  viewState?: string;
}

/**
 * Form field value map for filling forms
 */
export type FormFieldValues = Record<string, string | number | boolean | string[]>;

/**
 * Type for validation result
 */
export interface ValidationResult {
  isValid: boolean;
  errors: string[];
  warnings: string[];
}

/**
 * Stream operation callback
 */
export type StreamCallback<T> = (data: T) => void;

/**
 * Stream error callback
 */
export type StreamErrorCallback = (error: Error) => void;

/**
 * Stream end callback
 */
export type StreamEndCallback = () => void;
