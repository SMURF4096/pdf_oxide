//! Hand-rolled CSS engine — tokenizer, parser, selectors, cascade.
//!
//! See `docs/v0.3.35-html-css-pdf-plan.md` for the supported CSS surface
//! and the rationale for hand-rolling instead of depending on the
//! Mozilla stack (MPL-2.0 — denied by `deny.toml`).

pub mod tokenizer;

pub use tokenizer::{tokenize, SourceLocation, Token, TokenizerError};
