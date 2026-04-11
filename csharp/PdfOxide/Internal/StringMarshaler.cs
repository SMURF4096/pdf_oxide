using System;
using System.Runtime.InteropServices;
using System.Text;

namespace PdfOxide.Internal
{
    /// <summary>
    /// Utility for marshaling strings between C# and Rust.
    /// </summary>
    internal static class StringMarshaler
    {
        /// <summary>
        /// Converts a native UTF-8 pointer to a managed string.
        /// </summary>
        /// <param name="ptr">Pointer to UTF-8 null-terminated string.</param>
        /// <returns>The managed string, or empty string if pointer is null.</returns>
        public static string PtrToString(IntPtr ptr)
        {
            if (ptr == IntPtr.Zero)
                return string.Empty;

            return PtrToStringUtf8(ptr) ?? string.Empty;
        }

        /// <summary>
        /// Polyfill for Marshal.PtrToStringUTF8 for older frameworks.
        /// </summary>
        internal static string? PtrToStringUtf8(IntPtr ptr)
        {
            if (ptr == IntPtr.Zero)
                return null;

#if NET5_0_OR_GREATER
            return Marshal.PtrToStringUTF8(ptr);
#else
            // Find the null terminator
            int length = 0;
            while (Marshal.ReadByte(ptr, length) != 0)
                length++;

            if (length == 0)
                return string.Empty;

            byte[] buffer = new byte[length];
            Marshal.Copy(ptr, buffer, 0, length);
            return Encoding.UTF8.GetString(buffer);
#endif
        }

        /// <summary>
        /// Converts a native UTF-8 pointer to a managed string and frees the memory.
        /// </summary>
        /// <param name="ptr">Pointer to UTF-8 null-terminated string.</param>
        /// <returns>The managed string, or empty string if pointer is null.</returns>
        public static string PtrToStringAndFree(IntPtr ptr)
        {
            try
            {
                return PtrToString(ptr);
            }
            finally
            {
                if (ptr != IntPtr.Zero)
                {
                    NativeMethods.FreeString(ptr);
                }
            }
        }
    }
}
