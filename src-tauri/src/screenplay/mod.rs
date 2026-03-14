// Module declarations for screenplay document model and business logic

/// Document structs for the .screenplay file format (content, meta, settings).
pub mod document;

/// Fountain export: converts ProseMirror JSON to Fountain plain text.
pub mod fountain;

/// Plain text export: converts ProseMirror JSON to formatted screenplay text.
pub mod plaintext;

/// Typst-based PDF generation: converts ProseMirror JSON to Typst markup and PDF bytes.
pub mod pdf;
