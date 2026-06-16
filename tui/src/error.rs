//! Error types and the project-wide `Result` alias.
//!
//! Most error handling in M1 uses `eyre::Report` directly via the alias below.
//! A structured `MtuiError` enum can be added later if needed; right now the
//! few error sites are clear enough with `eyre`.

pub type Result<T> = std::result::Result<T, color_eyre::eyre::Error>;
