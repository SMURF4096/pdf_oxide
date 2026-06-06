//! Read-only context the pipeline borrows for the duration of a single
//! resolution call.
//!
//! All the cross-cutting state the resolver stages need lives here:
//! - The document handle, for object resolution (tint-transform streams,
//!   ICC profiles, function dictionaries).
//! - The page's resolved colour-space dictionary, so `Spaced` logical
//!   colours can be evaluated against the spaces the resource map declared.
//!
//! The context is a struct of borrows so that the operator walker can build
//! it once per page (or once per Form XObject scope) and hand it to every
//! `resolve` call without per-intent allocation.
//!
//! The output-intent CMYK profile and rendering intent were previously
//! threaded through here, but no resolver stage reads them yet. They will
//! be added back when the colour stage grows an ICC code path that
//! actually consumes them; carrying dead fields just to forward them
//! through every callsite was net-negative.

use std::collections::HashMap;

use crate::document::PdfDocument;
use crate::object::Object;

/// Per-page (or per-Form XObject) context for the resolution pipeline.
///
/// Lifetime `'a` ties the context to the operator walker's owned state.
pub(crate) struct ResolutionContext<'a> {
    pub(crate) doc: &'a PdfDocument,
    pub(crate) color_spaces: &'a HashMap<String, Object>,
}

impl<'a> ResolutionContext<'a> {
    /// Build a context from the page-resource snapshot the operator walker
    /// already maintains. The walker computes `color_spaces` from
    /// `resources["ColorSpace"]` once per page; we just borrow it.
    pub(crate) fn new(doc: &'a PdfDocument, color_spaces: &'a HashMap<String, Object>) -> Self {
        Self { doc, color_spaces }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::fixture_doc;
    use super::*;

    #[test]
    fn context_carries_empty_color_spaces() {
        let doc = fixture_doc();
        let color_spaces = HashMap::new();
        let ctx = ResolutionContext::new(&doc, &color_spaces);
        assert!(ctx.color_spaces.is_empty());
    }

    #[test]
    fn context_borrows_color_space_map() {
        // The point of taking `&HashMap` is that the walker's page-scope
        // map is reused across intents; building a fresh context per
        // intent must be cheap (no clone).
        let doc = fixture_doc();
        let mut color_spaces = HashMap::new();
        color_spaces.insert("CS1".to_string(), Object::Name("DeviceCMYK".to_string()));

        let ctx = ResolutionContext::new(&doc, &color_spaces);
        assert!(ctx.color_spaces.contains_key("CS1"));
        // Re-build context — must still see the same entries through the
        // same borrow without any heap traffic.
        let ctx2 = ResolutionContext::new(&doc, &color_spaces);
        assert_eq!(ctx2.color_spaces.len(), 1);
    }
}
