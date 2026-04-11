using PdfOxide.Exceptions;

namespace PdfOxide.Internal
{
    /// <summary>
    /// Maps native error codes to .NET exceptions.
    /// </summary>
    internal static class ExceptionMapper
    {
        /// <summary>
        /// Creates an exception from a non-zero error code. Code 0 is success and is invalid input.
        /// </summary>
        /// <param name="errorCode">The error code from the Rust FFI layer. Must not be 0.</param>
        /// <returns>An appropriate <see cref="PdfOxide.Exceptions.PdfException"/> subclass.</returns>
        /// <exception cref="System.ArgumentOutOfRangeException">Thrown when <paramref name="errorCode"/> is 0.</exception>
        public static PdfOxide.Exceptions.PdfException CreateException(int errorCode)
        {
            if (errorCode == 0)
            {
                throw new System.ArgumentOutOfRangeException(nameof(errorCode), "Cannot create an exception from success code 0.");
            }
            return errorCode switch
            {
                1 => new IoException("I/O error: File not found, permission denied, or read/write failed"),
                2 => new ParseException("Parse error: Invalid PDF structure or content stream"),
                3 => new EncryptionException("Encryption error: Incorrect password or unsupported encryption"),
                4 => new InvalidStateException("Invalid state: Operation not allowed in current document state"),
                5 => new UnsupportedFeatureException("rendering"),
                6 => new UnsupportedFeatureException("ocr"),
                7 => new InvalidStateException("Invalid argument: One or more arguments were invalid"),
                8 => new SignatureException("Signature error: Certificate loading, signing, or verification failed"),
                9 => new RedactionException("Redaction error: Content redaction or metadata scrubbing failed"),
                10 => new ComplianceException("Compliance error: PDF/A, PDF/X, or PDF/UA conversion or validation failed"),
                11 => new AccessibilityException("Accessibility error: Tagging, structure tree, or alt text operation failed"),
                12 => new OptimizationException("Optimization error: Font subsetting, image downsampling, or deduplication failed"),
                _ => new UnknownError($"Unknown error (code: {errorCode})")
            };
        }

        /// <summary>
        /// Checks if an error code represents success.
        /// </summary>
        /// <param name="errorCode">The error code.</param>
        /// <returns>True if the error code indicates success (0), false otherwise.</returns>
        public static bool IsSuccess(int errorCode) => errorCode == 0;

        /// <summary>
        /// Throws an exception if the error code indicates an error.
        /// </summary>
        /// <param name="errorCode">The error code to check.</param>
        /// <exception cref="PdfOxide.Exceptions.PdfException">Thrown if the error code indicates an error.</exception>
        public static void ThrowIfError(int errorCode)
        {
            if (!IsSuccess(errorCode))
            {
                throw CreateException(errorCode);
            }
        }
    }
}
