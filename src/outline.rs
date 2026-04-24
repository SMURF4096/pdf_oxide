//! PDF document outline (bookmarks) support.
//!
//! Provides access to the PDF document outline, also known as bookmarks,
//! which allow users to navigate a PDF document.

use crate::document::PdfDocument;
use crate::error::{Error, Result};
use crate::object::Object;

/// A single outline item (bookmark) in the document hierarchy.
#[derive(Debug, Clone)]
pub struct OutlineItem {
    /// The title of this bookmark
    pub title: String,

    /// The destination (page number or named destination)
    /// None if destination cannot be determined
    pub dest: Option<Destination>,

    /// Child bookmarks under this item
    pub children: Vec<OutlineItem>,
}

/// Destination in the PDF
#[derive(Debug, Clone)]
pub enum Destination {
    /// Direct page reference (page index, 0-based)
    PageIndex(usize),

    /// Named destination (string identifier)
    Named(String),
}

impl PdfDocument {
    /// Get the document outline (bookmarks) if present.
    ///
    /// Returns a hierarchical structure of bookmarks that can be used
    /// for document navigation.
    ///
    /// # Returns
    ///
    /// - `Ok(Some(Vec<OutlineItem>))` - Bookmarks found and parsed
    /// - `Ok(None)` - No bookmarks in document
    /// - `Err` - Error parsing bookmarks
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdf_oxide::PdfDocument;
    ///
    /// let mut doc = PdfDocument::open("sample.pdf")?;
    /// if let Some(outline) = doc.get_outline()? {
    ///     for item in outline {
    ///         println!("Bookmark: {}", item.title);
    ///     }
    /// }
    /// # Ok::<(), pdf_oxide::error::Error>(())
    /// ```
    pub fn get_outline(&self) -> Result<Option<Vec<OutlineItem>>> {
        // Get catalog
        let catalog = self.catalog()?;

        // Check if catalog has /Outlines entry
        let outlines_ref = match catalog.as_dict() {
            Some(dict) => match dict.get("Outlines") {
                Some(Object::Reference(obj_ref)) => *obj_ref,
                _ => return Ok(None),
            },
            None => return Ok(None),
        };

        // Load the outlines dictionary
        let outlines_dict = self.load_object(outlines_ref)?;

        // Get the first outline item
        let first_ref = match outlines_dict.as_dict() {
            Some(dict) => match dict.get("First") {
                Some(Object::Reference(obj_ref)) => *obj_ref,
                _ => return Ok(None),
            },
            None => return Ok(None),
        };

        // Parse outline items at root level
        let mut items = Vec::new();
        let mut current_ref = Some(first_ref);

        while let Some(item_ref) = current_ref {
            if let Ok(item) = self.parse_outline_item(item_ref) {
                items.push(item);
            }

            // Get next sibling
            let item_obj = self.load_object(item_ref)?;
            current_ref = match item_obj.as_dict() {
                Some(dict) => match dict.get("Next") {
                    Some(Object::Reference(obj_ref)) => Some(*obj_ref),
                    _ => None,
                },
                None => None,
            };
        }

        if items.is_empty() {
            Ok(None)
        } else {
            Ok(Some(items))
        }
    }

    /// Parse a single outline item and its children recursively.
    fn parse_outline_item(&self, item_ref: crate::object::ObjectRef) -> Result<OutlineItem> {
        let item_obj = self.load_object(item_ref)?;

        // Extract title
        let title = match item_obj.as_dict() {
            Some(dict) => match dict.get("Title") {
                Some(Object::String(s)) => String::from_utf8_lossy(s).to_string(),
                _ => "(No Title)".to_string(),
            },
            None => "(No Title)".to_string(),
        };

        // Extract destination
        let dest = self.parse_outline_destination(&item_obj)?;

        // Parse children if present
        let mut children = Vec::new();
        if let Some(dict) = item_obj.as_dict() {
            if let Some(Object::Reference(first_child_ref)) = dict.get("First") {
                let mut child_ref = Some(*first_child_ref);

                while let Some(c_ref) = child_ref {
                    if let Ok(child) = self.parse_outline_item(c_ref) {
                        children.push(child);
                    }

                    // Get next sibling
                    let child_obj = self.load_object(c_ref)?;
                    child_ref = match child_obj.as_dict() {
                        Some(dict) => match dict.get("Next") {
                            Some(Object::Reference(obj_ref)) => Some(*obj_ref),
                            _ => None,
                        },
                        None => None,
                    };
                }
            }
        }

        Ok(OutlineItem {
            title,
            dest,
            children,
        })
    }

    /// Parse destination from an outline item.
    fn parse_outline_destination(&self, item: &Object) -> Result<Option<Destination>> {
        let dict = match item.as_dict() {
            Some(d) => d,
            None => return Ok(None),
        };

        // Try /Dest entry first
        if let Some(dest_obj) = dict.get("Dest") {
            return self.resolve_destination(dest_obj);
        }

        // Try /A (action) entry
        let mut action = dict.get("A");

        // Resolve indirect reference to action
        let obj;
        if let Some(Object::Reference(obj_ref)) = action {
            obj = self.load_object(*obj_ref)?;
            action = Some(&obj);
        }

        // Look for destination under /D key
        if let Some(Object::Dictionary(action)) = action {
            if let Some(dest_obj) = action.get("D") {
                return self.resolve_destination(dest_obj);
            }
        }

        Ok(None)
    }

    /// Resolve a destination object to a Destination enum.
    fn resolve_destination(&self, dest_obj: &Object) -> Result<Option<Destination>> {
        match dest_obj {
            // Named destination (string)
            Object::String(name) => {
                Ok(Some(Destination::Named(String::from_utf8_lossy(name).to_string())))
            },

            // Direct destination (array)
            Object::Array(arr) if !arr.is_empty() => {
                // First element is page reference
                match &arr[0] {
                    Object::Reference(page_ref) => {
                        // Try to find which page this is
                        if let Ok(page_index) = self.find_page_index(*page_ref) {
                            Ok(Some(Destination::PageIndex(page_index)))
                        } else {
                            Ok(None)
                        }
                    },
                    _ => Ok(None),
                }
            },

            // Indirect reference to destination
            Object::Reference(dest_ref) => {
                let resolved = self.load_object(*dest_ref)?;
                self.resolve_destination(&resolved)
            },

            _ => Ok(None),
        }
    }

    /// Find the page index for a given page object reference.
    fn find_page_index(&self, page_ref: crate::object::ObjectRef) -> Result<usize> {
        // Get page count to iterate through pages
        let count = self.page_count()?;

        for i in 0..count {
            // Get page object reference for this index
            if let Ok(page_obj_ref) = self.get_page_ref(i) {
                if page_obj_ref == page_ref {
                    return Ok(i);
                }
            }
        }

        Err(Error::InvalidPdf(format!("Page reference {:?} not found", page_ref)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_outline_item_creation() {
        let item = OutlineItem {
            title: "Chapter 1".to_string(),
            dest: Some(Destination::PageIndex(0)),
            children: vec![],
        };

        assert_eq!(item.title, "Chapter 1");
        assert!(matches!(item.dest, Some(Destination::PageIndex(0))));
        assert!(item.children.is_empty());
    }

    #[test]
    fn test_outline_hierarchy() {
        let child = OutlineItem {
            title: "Section 1.1".to_string(),
            dest: Some(Destination::PageIndex(1)),
            children: vec![],
        };

        let parent = OutlineItem {
            title: "Chapter 1".to_string(),
            dest: Some(Destination::PageIndex(0)),
            children: vec![child],
        };

        assert_eq!(parent.children.len(), 1);
        assert_eq!(parent.children[0].title, "Section 1.1");
    }
}
