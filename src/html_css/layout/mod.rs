//! Layout pipeline — turns a styled DOM into positioned boxes.
//!
//! Phase LAYOUT in the v0.3.35 plan, the largest remaining unknown.
//! Lands incrementally:
//!
//! - **LAYOUT-1** (this commit) — box tree construction. DOM ×
//!   ComputedStyles → [`BoxTree`]. No positioning yet; just the
//!   semantic tree the next sub-tasks size and place.
//! - **LAYOUT-2** — `ComputedStyles → taffy::Style` mapping for
//!   block/flex/grid/table modes; Taffy delegates inline measurement
//!   back to us.
//! - **LAYOUT-3** — inline formatting context (line boxes, BiDi, UAX
//!   #14 line breaks, justify, vertical-align, decorations,
//!   `::first-line`/`::first-letter`).
//! - **LAYOUT-4..7** — floats, margin collapsing, multi-column, tables.

pub mod box_tree;
pub mod inline;
pub mod taffy_style;

pub use box_tree::{
    build_box_tree, BoxId, BoxKind, BoxNode, BoxTree, BoxTreeError, DisplayInside,
    DisplayOutside,
};
pub use inline::{layout_paragraph, InlineItem, LineBox, LineFragment, TextAlign, WhiteSpace};
pub use taffy_style::{run_layout, style_to_taffy, LayoutBox, LayoutResult};
