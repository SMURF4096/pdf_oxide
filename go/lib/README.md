# Native Libraries

This directory contains prebuilt native `pdf_oxide` libraries that are
statically linked into Go binaries at build time (see [#334]). They are
populated by the release CI pipeline and committed to the Go module so that
`go get github.com/yfedoseev/pdf_oxide/go` works without requiring users to
build Rust themselves.

Starting with **v0.3.26**, each platform ships a static archive
(`libpdf_oxide.a`) rather than a shared object. CGo links the archive
directly via `#cgo ... LDFLAGS` (see `go/pdf_oxide.go`), so the resulting Go
binary is self-contained — no `LD_LIBRARY_PATH` / `DYLD_LIBRARY_PATH` / `PATH`
configuration required at runtime.

## Directory Structure

```
lib/
  linux_amd64/    libpdf_oxide.a     (staticlib)
  linux_arm64/    libpdf_oxide.a     (staticlib)
  darwin_amd64/   libpdf_oxide.a     (staticlib)
  darwin_arm64/   libpdf_oxide.a     (staticlib)
  windows_amd64/  libpdf_oxide.a     (staticlib, MinGW-compatible)
  windows_arm64/  pdf_oxide.dll      (dynamic — see note below)
```

### Windows ARM64 caveat

Windows ARM64 temporarily remains on dynamic linking in v0.3.26 because
Rust's `aarch64-pc-windows-gnullvm` target is Tier 3 and not yet production-
ready for our CI. Go binaries built for Windows ARM64 must still ship
`pdf_oxide.dll` alongside the executable. Tracked as a follow-up; Linux,
macOS, and Windows x64 are all fully self-contained.

## Building from source

If you prefer to build the native library yourself:

```bash
# From the pdf_oxide root directory
cargo build --release --lib
cp target/release/libpdf_oxide.a go/lib/linux_amd64/     # Linux x64
cp target/release/libpdf_oxide.a go/lib/darwin_arm64/    # macOS (Apple Silicon)

# Windows x64 (cross-compile from Linux with mingw-w64):
rustup target add x86_64-pc-windows-gnu
cargo build --release --lib --target x86_64-pc-windows-gnu
cp target/x86_64-pc-windows-gnu/release/libpdf_oxide.a go/lib/windows_amd64/
```

## Regenerating the CGo system-library list

Rust's `staticlib` output doesn't embed references to its own platform-
specific system dependencies (libm, pthread, Security framework, bcrypt,
…); those must be listed explicitly in each `#cgo ... LDFLAGS` directive in
`go/pdf_oxide.go`. If a dependency bump introduces a new linker symbol, get
the authoritative list from rustc:

```bash
cargo rustc --release --lib --target x86_64-unknown-linux-gnu \
  -- --print native-static-libs
```

and copy the reported `-l…` flags into the matching `#cgo linux,amd64
LDFLAGS` line.

[#334]: https://github.com/yfedoseev/pdf_oxide/issues/334
