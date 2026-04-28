fn main() {
    // LLVM may emit __memcmpeq@GLIBC_2.35 (an equality-only memcmp optimisation
    // added in glibc 2.35).  Amazon Linux 2023 ships glibc 2.34 and cannot load
    // a wheel that references that symbol.  Aliasing it to plain memcmp at link
    // time satisfies the reference without requiring the newer glibc at runtime.
    //
    // cargo:rustc-cdylib-link-arg applies only to the final cdylib link step,
    // so it does NOT affect build scripts (which would fail with lld because
    // memcmp is not exported from the build-script executable).  See issue #416.
    if std::env::var_os("CARGO_CFG_TARGET_OS").as_deref() == Some(std::ffi::OsStr::new("linux"))
        && std::env::var_os("CARGO_CFG_TARGET_ARCH").as_deref()
            == Some(std::ffi::OsStr::new("x86_64"))
    {
        println!("cargo:rustc-cdylib-link-arg=-Wl,--defsym=__memcmpeq=memcmp");
    }
}
