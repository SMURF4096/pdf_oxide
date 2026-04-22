using System;
using PdfOxide.Exceptions;
using PdfOxide.Internal;

namespace PdfOxide.Core
{
    /// <summary>
    /// A single existing digital signature on a PDF document, obtained
    /// via <see cref="PdfDocument.Signatures"/>. Inspection-only for
    /// now — <see cref="Verify"/> currently surfaces the not-yet-landed
    /// CMS verification path as <see cref="UnsupportedFeatureException"/>.
    ///
    /// Closes #384 audit gap D (#51) — the instance-level accessors and
    /// document enumeration. Cryptographic verification and certificate
    /// retrieval will arrive as later slices of #72.
    /// </summary>
    public sealed class Signature : IDisposable
    {
        private NativeHandle _handle;
        private bool _disposed;

        private Signature(NativeHandle handle)
        {
            _handle = handle;
        }

        internal static Signature FromHandle(NativeHandle handle) => new(handle);

        /// <summary>
        /// The <c>/Name</c> of the signer as recorded in the signature
        /// dictionary, or <c>null</c> if the PDF left that field blank.
        /// </summary>
        public string? SignerName => ReadOptionalString(NativeMethods.pdf_signature_get_signer_name);

        /// <summary>
        /// The stated reason for signing (<c>/Reason</c>), or <c>null</c>
        /// if not supplied.
        /// </summary>
        public string? Reason => ReadOptionalString(NativeMethods.pdf_signature_get_signing_reason);

        /// <summary>
        /// The stated signing location (<c>/Location</c>), or <c>null</c>
        /// if not supplied.
        /// </summary>
        public string? Location => ReadOptionalString(NativeMethods.pdf_signature_get_signing_location);

        /// <summary>
        /// The parsed signing time (<c>/M</c>), or <c>null</c> if the
        /// dictionary had no <c>/M</c> entry or it was unparseable.
        /// </summary>
        public DateTimeOffset? SigningTime
        {
            get
            {
                ThrowIfDisposed();
                var epoch = NativeMethods.pdf_signature_get_signing_time(_handle, out int err);
                ExceptionMapper.ThrowIfError(err);
                return epoch == 0 ? null : DateTimeOffset.FromUnixTimeSeconds(epoch);
            }
        }

        /// <summary>
        /// Extract the signing certificate from the embedded
        /// PKCS#7/CMS SignedData blob. Parses the `/Contents` bytes
        /// using the Rust-core CMS helper and returns a live
        /// <see cref="Certificate"/> the caller owns and must dispose.
        /// </summary>
        /// <exception cref="PdfException">The signature dictionary had no <c>/Contents</c> entry
        /// or the bytes didn't parse as CMS SignedData.</exception>
        public Certificate GetCertificate()
        {
            ThrowIfDisposed();
            var certHandle = NativeMethods.pdf_signature_get_certificate(_handle, out int err);
            ExceptionMapper.ThrowIfError(err);
            if (certHandle.IsInvalid)
            {
                throw new PdfException("pdf_signature_get_certificate returned null with no error code");
            }
            return Certificate.FromHandle(certHandle);
        }

        /// <summary>
        /// Cryptographically verify the signature.
        /// Currently unsupported — requires the full PKCS#7 verification
        /// path landing as a later slice of #72.
        /// </summary>
        /// <exception cref="UnsupportedFeatureException">Always, until Rust-core verify lands.</exception>
        public bool Verify()
        {
            ThrowIfDisposed();
            var result = NativeMethods.pdf_signature_verify(_handle, out int err);
            ExceptionMapper.ThrowIfError(err);
            return result == 1;
        }

        /// <inheritdoc />
        public void Dispose()
        {
            if (!_disposed)
            {
                _handle?.Dispose();
                _disposed = true;
            }
        }

        private delegate IntPtr NativeStringAccessor(NativeHandle handle, out int errorCode);

        private string? ReadOptionalString(NativeStringAccessor accessor)
        {
            ThrowIfDisposed();
            var ptr = accessor(_handle, out int err);
            ExceptionMapper.ThrowIfError(err);
            if (ptr == IntPtr.Zero) return null;
            try { return StringMarshaler.PtrToString(ptr); }
            finally { NativeMethods.FreeString(ptr); }
        }

        private void ThrowIfDisposed()
        {
            if (_disposed) throw new ObjectDisposedException(nameof(Signature));
        }
    }
}
