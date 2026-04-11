/**
 * Common type definitions and utilities
 */

import type {
  SearchOptions,
  SearchResult,
  Metadata,
  DocumentInfo,
  EmbeddedFile,
  Annotation,
} from './native-bindings';

// Re-export commonly used native types
export type {
  SearchOptions,
  SearchResult,
  Metadata,
  DocumentInfo,
  EmbeddedFile,
  Annotation,
  NativePdfDocument,
  NativePdf,
  NativePdfPage,
  Rect,
  Point,
  Color,
  PdfElement,
  PdfText,
  PdfImage,
  PdfPath,
  PdfTable,
  PdfTableCell,
  TextAnnotation,
  HighlightAnnotation,
  LinkAnnotation,
  InkAnnotation,
  SquareAnnotation,
  CircleAnnotation,
  LineAnnotation,
  PolygonAnnotation,
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
