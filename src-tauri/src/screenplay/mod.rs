// Module declarations for screenplay document model and business logic

/// Document structs for the .screenplay file format (content, meta, settings).
pub mod document;

/// Fountain export: converts ProseMirror JSON to Fountain plain text.
pub mod fountain;

/// Fountain import: parses .fountain text into ScreenplayDocument + summary.
pub mod fountain_import;

/// Final Draft (FDX) import: parses .fdx XML into ScreenplayDocument + summary.
pub mod fdx_import;

/// Plain text export: converts ProseMirror JSON to formatted screenplay text.
pub mod plaintext;

/// Typst-based PDF generation: converts ProseMirror JSON to Typst markup and PDF bytes.
pub mod pdf;
