# Native Libraries

This directory contains prebuilt native `libpdf_oxide` shared libraries for each platform.
They are populated by the release CI pipeline and committed to the Go module so that
`go get github.com/yfedoseev/pdfoxide` works without requiring users to build Rust.

## Directory Structure

```
lib/
  linux_amd64/    libpdf_oxide.so
  linux_arm64/    libpdf_oxide.so
  darwin_amd64/   libpdf_oxide.dylib
  darwin_arm64/   libpdf_oxide.dylib
  windows_amd64/  pdf_oxide.dll
  windows_arm64/  pdf_oxide.dll
```

## Building from source

If you prefer to build the native library yourself:

```bash
# From the pdf_oxide root directory
cargo build --release --lib
# Copy to the appropriate go/lib/ subdirectory
cp target/release/libpdf_oxide.so go/lib/linux_amd64/   # Linux
cp target/release/libpdf_oxide.dylib go/lib/darwin_arm64/ # macOS
```
