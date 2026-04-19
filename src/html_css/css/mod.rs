//! Hand-rolled CSS engine — tokenizer, parser, selectors, cascade.
//!
//! See `docs/v0.3.35-html-css-pdf-plan.md` for the supported CSS surface
//! and the rationale for hand-rolling instead of depending on the
//! Mozilla stack (MPL-2.0 — denied by `deny.toml`).

pub mod parser;
pub mod selectors;
pub mod tokenizer;

pub use parser::{
    parse_declaration_list, parse_stylesheet, AtRule, AtRuleBlock, ComponentValue, Declaration,
    QualifiedRule, Rule, Stylesheet,
};
pub use selectors::{
    parse_selector_list, AnPlusB, AttributeCase, AttributeOp, AttributeSelector, Combinator,
    ComplexSelector, CompoundSelector, ElementSelector, PseudoClass, PseudoElement,
    SelectorList, SelectorParseError, Specificity, SubclassSelector,
};
pub use tokenizer::{tokenize, SourceLocation, Token, TokenizerError};
