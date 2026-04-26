// Font loading utilities for Typst PDF embedding (Noto Sans Malayalam,
// Manjari, Courier Prime). Courier Prime is bundled even though it's
// not exposed as a user-selectable body font — the PDF templates use
// it for accent typography (wordmarks, slugs, hero numerals, credit
// names, scene-card eyebrows) and need it available to the Typst font
// resolver alongside whatever body font the user picked.

/// A font bundled into the application binary at compile time.
///
/// The `regular` and `bold` fields contain the raw bytes of the font files,
/// embedded using `include_bytes!` so they're part of the compiled binary.
/// This means no external font files are needed at runtime.
pub struct BundledFont {
    /// Human-readable name for the font family (e.g. "Noto Sans Malayalam")
    pub name: &'static str,

    /// Raw bytes of the regular-weight font file, embedded at compile time.
    /// `&'static [u8]` means: a reference to a byte slice that lives for the
    /// entire duration of the program (because it's baked into the binary).
    pub regular: &'static [u8],

    /// Raw bytes of the bold-weight font file, embedded at compile time.
    pub bold: &'static [u8],
}

/// Returns all fonts bundled with the application.
///
/// Each font's file bytes are embedded at compile time via `include_bytes!`,
/// so they're available without any file system access at runtime.
pub fn bundled_fonts() -> Vec<BundledFont> {
    // `include_bytes!` is a Rust macro that reads a file at compile time and
    // embeds its contents as a `&'static [u8]` (a static byte slice).
    // The path is relative to the source file where the macro is invoked.
    // Here, `../../fonts/` navigates from src/fonts/ up to src-tauri/fonts/.
    vec![
        BundledFont {
            name: "Noto Sans Malayalam",
            regular: include_bytes!("../../fonts/NotoSansMalayalam-Regular.ttf"),
            bold: include_bytes!("../../fonts/NotoSansMalayalam-Bold.ttf"),
        },
        BundledFont {
            name: "Manjari",
            regular: include_bytes!("../../fonts/Manjari-Regular.otf"),
            bold: include_bytes!("../../fonts/Manjari-Bold.otf"),
        },
        BundledFont {
            name: "Courier Prime",
            regular: include_bytes!("../../fonts/CourierPrime-Regular.ttf"),
            bold: include_bytes!("../../fonts/CourierPrime-Bold.ttf"),
        },
    ]
}

/// Bytes of the Courier Prime font (regular + bold), always loaded
/// into the PDF compiler's font world even when the user has picked
/// Manjari or Noto as the body font. The PDF templates reference
/// "Courier Prime" by name in accent positions (wordmarks, slugs,
/// hero numerals, credit names) and will silently fall back to the
/// body font if Typst can't resolve it — making the editorial-Courier
/// vocabulary disappear from the printed PDF. Calling this from the
/// PDF compile path ensures the font is always present.
pub fn courier_prime_bytes() -> (&'static [u8], &'static [u8]) {
    (
        include_bytes!("../../fonts/CourierPrime-Regular.ttf"),
        include_bytes!("../../fonts/CourierPrime-Bold.ttf"),
    )
}
