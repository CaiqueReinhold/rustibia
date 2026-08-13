//! Embeds the application icon into the Windows executable.
//!
//! Inno Setup's `SetupIconFile` only dresses up the setup program; every shortcut
//! it creates takes its icon from the target exe instead, and a Rust binary ships
//! without an icon resource. Embedding one here is what gives the desktop and
//! start-menu shortcuts, the taskbar, alt-tab and Explorer the same icon.
//!
//! Resource compiler: `llvm-rc` for the MSVC target, `x86_64-w64-mingw32-windres`
//! for the GNU one. `packaging/build-windows.sh --setup` installs whichever the
//! chosen toolchain needs.

const ICON: &str = "packaging/icon.ico";

fn main() {
    // Only PE binaries carry resources; this is a no-op for a native Linux build.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    println!("cargo:rerun-if-changed={ICON}");

    winresource::WindowsResource::new()
        .set_icon(ICON)
        .compile()
        .expect("failed to embed the Windows icon resource");
}
