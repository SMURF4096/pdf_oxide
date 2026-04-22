using System;
using PdfOxide.Exceptions;
using PdfOxide.Internal;

namespace PdfOxide.Core
{
    /// <summary>
    /// Inspection-only X.509 certificate loaded from raw DER bytes.
    /// Closes #384 audit gap G.
    ///
    /// Rust-core task #71 makes the DER path fully functional against
    /// the `signatures` feature; PKCS#12 (certificate + private key)
    /// remains a Rust-core stub, so Load() currently only accepts raw
    /// DER-encoded X.509 bytes. Full PKCS#12 support will follow when
    /// tasks #72/#73/#74 land.
    /// </summary>
    public sealed class Certificate : IDisposable
    {
        private NativeHandle _handle;
        private bool _disposed;

        private Certificate(NativeHandle handle)
        {
            _handle = handle;
        }

        /// <summary>
        /// Load a certificate from raw DER-encoded X.509 bytes. If the
        /// blob is a PKCS#12 (.p12/.pfx), <paramref name="password"/>
        /// should be the matching password; otherwise pass null/empty.
        /// </summary>
        public static Certificate Load(byte[] data, string? password = null)
        {
            if (data == null) throw new ArgumentNullException(nameof(data));
            if (data.Length == 0)
                throw new ArgumentException("Certificate data must not be empty.", nameof(data));

            var handle = NativeMethods.pdf_certificate_load_from_bytes(
                data, data.Length, password, out int err);
            if (handle.IsInvalid)
            {
                ExceptionMapper.ThrowIfError(err);
                throw new PdfException("pdf_certificate_load_from_bytes returned null with no error code");
            }
            return new Certificate(handle);
        }

        /// <summary>Certificate subject Distinguished Name (RFC 5280).</summary>
        public string Subject
        {
            get
            {
                ThrowIfDisposed();
                var ptr = NativeMethods.pdf_certificate_get_subject(_handle, out int err);
                ExceptionMapper.ThrowIfError(err);
                if (ptr == IntPtr.Zero) return string.Empty;
                try { return StringMarshaler.PtrToString(ptr); }
                finally { NativeMethods.FreeString(ptr); }
            }
        }

        /// <summary>Certificate issuer Distinguished Name.</summary>
        public string Issuer
        {
            get
            {
                ThrowIfDisposed();
                var ptr = NativeMethods.pdf_certificate_get_issuer(_handle, out int err);
                ExceptionMapper.ThrowIfError(err);
                if (ptr == IntPtr.Zero) return string.Empty;
                try { return StringMarshaler.PtrToString(ptr); }
                finally { NativeMethods.FreeString(ptr); }
            }
        }

        /// <summary>Certificate serial number as a hex string.</summary>
        public string Serial
        {
            get
            {
                ThrowIfDisposed();
                var ptr = NativeMethods.pdf_certificate_get_serial(_handle, out int err);
                ExceptionMapper.ThrowIfError(err);
                if (ptr == IntPtr.Zero) return string.Empty;
                try { return StringMarshaler.PtrToString(ptr); }
                finally { NativeMethods.FreeString(ptr); }
            }
        }

        /// <summary>Certificate validity window as Unix timestamps.</summary>
        public (DateTimeOffset NotBefore, DateTimeOffset NotAfter) Validity
        {
            get
            {
                ThrowIfDisposed();
                NativeMethods.pdf_certificate_get_validity(
                    _handle, out long notBefore, out long notAfter, out int err);
                ExceptionMapper.ThrowIfError(err);
                return (
                    DateTimeOffset.FromUnixTimeSeconds(notBefore),
                    DateTimeOffset.FromUnixTimeSeconds(notAfter));
            }
        }

        /// <summary>
        /// True iff the certificate is within its validity window as
        /// of right now (system clock).
        /// </summary>
        public bool IsValid
        {
            get
            {
                ThrowIfDisposed();
                bool valid = NativeMethods.pdf_certificate_is_valid(_handle, out int err);
                ExceptionMapper.ThrowIfError(err);
                return valid;
            }
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

        private void ThrowIfDisposed()
        {
            if (_disposed) throw new ObjectDisposedException(nameof(Certificate));
        }
    }
}
