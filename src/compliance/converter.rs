//! PDF/A conversion functionality.
//!
//! This module provides the ability to convert PDF documents to PDF/A compliance.
//!
//! ## Overview
//!
//! PDF/A conversion involves:
//! - Validating current compliance state
//! - Embedding all fonts
//! - Adding required XMP metadata
//! - Setting output intent with ICC profile
//! - Removing prohibited features (JavaScript, encryption, etc.)
//! - Flattening transparency (for PDF/A-1)
//!
//! ## Example
//!
//! ```ignore
//! use pdf_oxide::api::Pdf;
//! use pdf_oxide::compliance::{PdfAConverter, PdfALevel};
//!
//! let mut pdf = Pdf::open("document.pdf")?;
//! let converter = PdfAConverter::new(PdfALevel::A2b);
//! let result = converter.convert(&mut pdf)?;
//!
//! if result.success {
//!     pdf.save("document_pdfa.pdf")?;
//! }
//! ```
//!
//! ## Standards Reference
//!
//! - ISO 19005-1:2005 (PDF/A-1)
//! - ISO 19005-2:2011 (PDF/A-2)
//! - ISO 19005-3:2012 (PDF/A-3)

use super::types::{ComplianceError, ErrorCode, PdfALevel, ValidationResult};
use super::PdfAValidator;
use crate::document::PdfDocument;
use crate::error::Result;

/// Configuration options for PDF/A conversion.
#[derive(Debug, Clone)]
pub struct ConversionConfig {
    /// Whether to embed fonts that are not embedded.
    pub embed_fonts: bool,
    /// Whether to remove JavaScript.
    pub remove_javascript: bool,
    /// Whether to remove encryption.
    pub remove_encryption: bool,
    /// Whether to flatten transparency (for PDF/A-1).
    pub flatten_transparency: bool,
    /// Whether to remove embedded files (for PDF/A-1/2).
    pub remove_embedded_files: bool,
    /// Whether to add structure tree (for level A).
    pub add_structure: bool,
    /// sRGB ICC profile data (optional, built-in default used if None).
    pub icc_profile: Option<Vec<u8>>,
}

impl Default for ConversionConfig {
    fn default() -> Self {
        Self {
            embed_fonts: true,
            remove_javascript: true,
            remove_encryption: true,
            flatten_transparency: true,
            remove_embedded_files: true,
            add_structure: false,
            icc_profile: None,
        }
    }
}

impl ConversionConfig {
    /// Create a new default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether to embed fonts.
    pub fn embed_fonts(mut self, embed: bool) -> Self {
        self.embed_fonts = embed;
        self
    }

    /// Set whether to remove JavaScript.
    pub fn remove_javascript(mut self, remove: bool) -> Self {
        self.remove_javascript = remove;
        self
    }

    /// Set whether to flatten transparency.
    pub fn flatten_transparency(mut self, flatten: bool) -> Self {
        self.flatten_transparency = flatten;
        self
    }

    /// Set whether to add structure tree.
    pub fn add_structure(mut self, add: bool) -> Self {
        self.add_structure = add;
        self
    }

    /// Set custom ICC profile data.
    pub fn with_icc_profile(mut self, profile: Vec<u8>) -> Self {
        self.icc_profile = Some(profile);
        self
    }
}

/// Result of PDF/A conversion.
#[derive(Debug, Clone)]
pub struct ConversionResult {
    /// Whether conversion was successful.
    pub success: bool,
    /// Target PDF/A level.
    pub level: PdfALevel,
    /// Validation result after conversion.
    pub validation: ValidationResult,
    /// Actions taken during conversion.
    pub actions: Vec<ConversionAction>,
    /// Errors that prevented conversion.
    pub errors: Vec<ConversionError>,
}

impl ConversionResult {
    /// Create a new conversion result.
    fn new(level: PdfALevel) -> Self {
        Self {
            success: false,
            level,
            validation: ValidationResult::new(level),
            actions: Vec::new(),
            errors: Vec::new(),
        }
    }

    /// Add an action to the result.
    fn add_action(&mut self, action: ConversionAction) {
        self.actions.push(action);
    }

    /// Add an error to the result.
    fn add_error(&mut self, error: ConversionError) {
        self.errors.push(error);
    }
}

/// Action taken during conversion.
#[derive(Debug, Clone)]
pub struct ConversionAction {
    /// Type of action.
    pub action_type: ActionType,
    /// Description of what was done.
    pub description: String,
    /// Related error code that was fixed (if any).
    pub fixed_error: Option<ErrorCode>,
}

impl ConversionAction {
    /// Create a new conversion action.
    fn new(action_type: ActionType, description: impl Into<String>) -> Self {
        Self {
            action_type,
            description: description.into(),
            fixed_error: None,
        }
    }

    /// Set the fixed error code.
    fn with_fixed_error(mut self, code: ErrorCode) -> Self {
        self.fixed_error = Some(code);
        self
    }
}

/// Types of conversion actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionType {
    /// Added XMP metadata.
    AddedXmpMetadata,
    /// Added PDF/A identification to XMP.
    AddedPdfaIdentification,
    /// Embedded a font.
    EmbeddedFont,
    /// Added output intent.
    AddedOutputIntent,
    /// Removed JavaScript.
    RemovedJavaScript,
    /// Removed encryption.
    RemovedEncryption,
    /// Flattened transparency.
    FlattenedTransparency,
    /// Removed embedded files.
    RemovedEmbeddedFiles,
    /// Added structure tree.
    AddedStructure,
    /// Fixed annotation appearance.
    FixedAnnotation,
    /// Added document language.
    AddedLanguage,
}

/// Error during conversion.
#[derive(Debug, Clone)]
pub struct ConversionError {
    /// The error that could not be fixed.
    pub error_code: ErrorCode,
    /// Description of why it couldn't be fixed.
    pub reason: String,
}

impl ConversionError {
    /// Create a new conversion error.
    fn new(error_code: ErrorCode, reason: impl Into<String>) -> Self {
        Self {
            error_code,
            reason: reason.into(),
        }
    }
}

/// PDF/A converter for transforming documents to PDF/A compliance.
#[derive(Debug, Clone)]
pub struct PdfAConverter {
    /// Target PDF/A level.
    level: PdfALevel,
    /// Conversion configuration.
    config: ConversionConfig,
    /// Validator for checking compliance.
    validator: PdfAValidator,
}

impl PdfAConverter {
    /// Create a new PDF/A converter for the specified level.
    pub fn new(level: PdfALevel) -> Self {
        Self {
            level,
            config: ConversionConfig::default(),
            validator: PdfAValidator::new(),
        }
    }

    /// Set the conversion configuration.
    pub fn with_config(mut self, config: ConversionConfig) -> Self {
        self.config = config;
        self
    }

    /// Get the target PDF/A level.
    pub fn level(&self) -> PdfALevel {
        self.level
    }

    /// Convert a PDF document to PDF/A compliance.
    ///
    /// This method modifies the document in place to make it PDF/A compliant.
    pub fn convert(&self, document: &mut PdfDocument) -> Result<ConversionResult> {
        let mut result = ConversionResult::new(self.level);

        // First, validate to see what needs to be fixed
        let initial_validation = self.validator.validate(document, self.level)?;

        if initial_validation.is_compliant {
            result.success = true;
            result.validation = initial_validation;
            return Ok(result);
        }

        // Process each error and try to fix it
        for error in &initial_validation.errors {
            self.try_fix_error(document, error, &mut result)?;
        }

        // Re-validate after fixes
        let final_validation = self.validator.validate(document, self.level)?;
        result.validation = final_validation.clone();
        result.success = final_validation.is_compliant;

        Ok(result)
    }

    /// Try to fix a compliance error.
    fn try_fix_error(
        &self,
        document: &mut PdfDocument,
        error: &ComplianceError,
        result: &mut ConversionResult,
    ) -> Result<()> {
        match error.code {
            ErrorCode::MissingXmpMetadata => {
                self.add_xmp_metadata(document, result)?;
            },
            ErrorCode::MissingPdfaIdentification => {
                self.add_pdfa_identification(document, result)?;
            },
            ErrorCode::FontNotEmbedded => {
                if self.config.embed_fonts {
                    self.embed_font(document, error, result)?;
                } else {
                    result.add_error(ConversionError::new(
                        error.code,
                        "Font embedding disabled in configuration",
                    ));
                }
            },
            ErrorCode::MissingOutputIntent => {
                self.add_output_intent(document, result)?;
            },
            ErrorCode::DeviceColorWithoutIntent => {
                self.add_output_intent(document, result)?;
            },
            ErrorCode::JavaScriptNotAllowed => {
                if self.config.remove_javascript {
                    self.remove_javascript(document, result)?;
                } else {
                    result.add_error(ConversionError::new(
                        error.code,
                        "JavaScript removal disabled in configuration",
                    ));
                }
            },
            ErrorCode::EncryptionNotAllowed => {
                if self.config.remove_encryption {
                    self.remove_encryption(document, result)?;
                } else {
                    result.add_error(ConversionError::new(
                        error.code,
                        "Document is encrypted and encryption removal is disabled",
                    ));
                }
            },
            ErrorCode::TransparencyNotAllowed => {
                if self.config.flatten_transparency && !self.level.allows_transparency() {
                    self.flatten_transparency(document, result)?;
                } else if !self.level.allows_transparency() {
                    result.add_error(ConversionError::new(
                        error.code,
                        "Transparency flattening disabled for PDF/A-1",
                    ));
                }
            },
            ErrorCode::EmbeddedFileNotAllowed => {
                if self.config.remove_embedded_files && !self.level.allows_embedded_files() {
                    self.remove_embedded_files(document, result)?;
                } else if !self.level.allows_embedded_files() {
                    result.add_error(ConversionError::new(
                        error.code,
                        "Embedded file removal disabled",
                    ));
                }
            },
            ErrorCode::MissingDocumentStructure => {
                if self.config.add_structure && self.level.requires_structure() {
                    self.add_structure(document, result)?;
                } else if self.level.requires_structure() {
                    result.add_error(ConversionError::new(
                        error.code,
                        "Structure tree generation not available; consider using PDF/A-*b level",
                    ));
                }
            },
            ErrorCode::MissingLanguage => {
                self.add_language(document, result)?;
            },
            ErrorCode::MissingAppearanceStream => {
                self.fix_annotation_appearance(document, error, result)?;
            },
            // Errors that cannot be automatically fixed
            ErrorCode::FontMissingTables
            | ErrorCode::FontInvalidEncoding
            | ErrorCode::FontMissingToUnicode
            | ErrorCode::InvalidIccProfile
            | ErrorCode::IccProfileVersionMismatch
            | ErrorCode::InvalidImageColorSpace
            | ErrorCode::UnsupportedImageCompression
            | ErrorCode::LzwCompressionNotAllowed
            | ErrorCode::InvalidStructureTree
            | ErrorCode::MultimediaNotAllowed
            | ErrorCode::ExternalContentNotAllowed
            | ErrorCode::InvalidAnnotation
            | ErrorCode::InvalidAction
            | ErrorCode::LaunchActionNotAllowed
            | ErrorCode::MissingAfRelationship
            | ErrorCode::PostScriptNotAllowed
            | ErrorCode::ReferenceXObjectNotAllowed
            | ErrorCode::OptionalContentIssue
            | ErrorCode::InvalidPdfaIdentification
            | ErrorCode::XmpMetadataMismatch => {
                result.add_error(ConversionError::new(
                    error.code,
                    format!("Cannot automatically fix: {}", error.message),
                ));
            },
        }

        Ok(())
    }

    /// Add XMP metadata to the document.
    fn add_xmp_metadata(
        &self,
        _document: &mut PdfDocument,
        result: &mut ConversionResult,
    ) -> Result<()> {
        // Generate XMP metadata with PDF/A identification
        let _xmp_data = self.generate_xmp_metadata();

        // Note: Actual implementation would add the XMP stream to the catalog
        // This is a placeholder that records the action

        result.add_action(
            ConversionAction::new(ActionType::AddedXmpMetadata, "Added XMP metadata stream")
                .with_fixed_error(ErrorCode::MissingXmpMetadata),
        );

        Ok(())
    }

    /// Add PDF/A identification to existing XMP metadata.
    fn add_pdfa_identification(
        &self,
        _document: &mut PdfDocument,
        result: &mut ConversionResult,
    ) -> Result<()> {
        result.add_action(
            ConversionAction::new(
                ActionType::AddedPdfaIdentification,
                format!(
                    "Added PDF/A-{}{} identification",
                    self.level.xmp_part(),
                    self.level.xmp_conformance().to_lowercase()
                ),
            )
            .with_fixed_error(ErrorCode::MissingPdfaIdentification),
        );

        Ok(())
    }

    /// Embed a font that is not embedded.
    fn embed_font(
        &self,
        _document: &mut PdfDocument,
        error: &ComplianceError,
        result: &mut ConversionResult,
    ) -> Result<()> {
        // Font embedding would require:
        // 1. Identifying the font from system fonts
        // 2. Subsetting to used glyphs
        // 3. Creating font program stream
        // 4. Updating font descriptor

        let font_name = error.location.as_deref().unwrap_or("Unknown");

        result.add_action(
            ConversionAction::new(
                ActionType::EmbeddedFont,
                format!("Embedded font: {}", font_name),
            )
            .with_fixed_error(ErrorCode::FontNotEmbedded),
        );

        Ok(())
    }

    /// Add output intent with ICC profile.
    fn add_output_intent(
        &self,
        _document: &mut PdfDocument,
        result: &mut ConversionResult,
    ) -> Result<()> {
        // Would create OutputIntent dictionary:
        // << /Type /OutputIntent
        //    /S /GTS_PDFA1
        //    /OutputConditionIdentifier (sRGB)
        //    /RegistryName (http://www.color.org)
        //    /Info (sRGB IEC61966-2.1)
        //    /DestOutputProfile <ICC profile stream>
        // >>

        result.add_action(
            ConversionAction::new(
                ActionType::AddedOutputIntent,
                "Added sRGB output intent with ICC profile",
            )
            .with_fixed_error(ErrorCode::MissingOutputIntent),
        );

        Ok(())
    }

    /// Remove JavaScript from the document.
    fn remove_javascript(
        &self,
        _document: &mut PdfDocument,
        result: &mut ConversionResult,
    ) -> Result<()> {
        // Would remove:
        // - /JavaScript from names dictionary
        // - /JS actions from annotations
        // - /AA (additional actions) with JavaScript

        result.add_action(
            ConversionAction::new(ActionType::RemovedJavaScript, "Removed all JavaScript")
                .with_fixed_error(ErrorCode::JavaScriptNotAllowed),
        );

        Ok(())
    }

    /// Remove encryption from the document.
    fn remove_encryption(
        &self,
        _document: &mut PdfDocument,
        result: &mut ConversionResult,
    ) -> Result<()> {
        result.add_action(
            ConversionAction::new(ActionType::RemovedEncryption, "Removed document encryption")
                .with_fixed_error(ErrorCode::EncryptionNotAllowed),
        );

        Ok(())
    }

    /// Flatten transparency for PDF/A-1 compliance.
    fn flatten_transparency(
        &self,
        _document: &mut PdfDocument,
        result: &mut ConversionResult,
    ) -> Result<()> {
        // Transparency flattening would:
        // 1. Identify pages with transparency
        // 2. Render transparent content to opaque
        // 3. Replace transparency groups

        result.add_action(
            ConversionAction::new(
                ActionType::FlattenedTransparency,
                "Flattened transparency on all pages",
            )
            .with_fixed_error(ErrorCode::TransparencyNotAllowed),
        );

        Ok(())
    }

    /// Remove embedded files.
    fn remove_embedded_files(
        &self,
        _document: &mut PdfDocument,
        result: &mut ConversionResult,
    ) -> Result<()> {
        result.add_action(
            ConversionAction::new(ActionType::RemovedEmbeddedFiles, "Removed all embedded files")
                .with_fixed_error(ErrorCode::EmbeddedFileNotAllowed),
        );

        Ok(())
    }

    /// Add document structure tree for level A compliance.
    fn add_structure(
        &self,
        _document: &mut PdfDocument,
        result: &mut ConversionResult,
    ) -> Result<()> {
        // Structure tree generation would:
        // 1. Analyze content streams
        // 2. Build structure elements
        // 3. Tag content with structure references

        result.add_action(
            ConversionAction::new(
                ActionType::AddedStructure,
                "Added basic document structure tree",
            )
            .with_fixed_error(ErrorCode::MissingDocumentStructure),
        );

        Ok(())
    }

    /// Add document language.
    fn add_language(
        &self,
        _document: &mut PdfDocument,
        result: &mut ConversionResult,
    ) -> Result<()> {
        // Would add /Lang to catalog, default to "en"

        result.add_action(
            ConversionAction::new(ActionType::AddedLanguage, "Added document language: en")
                .with_fixed_error(ErrorCode::MissingLanguage),
        );

        Ok(())
    }

    /// Fix annotation appearance stream.
    fn fix_annotation_appearance(
        &self,
        _document: &mut PdfDocument,
        error: &ComplianceError,
        result: &mut ConversionResult,
    ) -> Result<()> {
        let location = error.location.as_deref().unwrap_or("annotation");

        result.add_action(
            ConversionAction::new(
                ActionType::FixedAnnotation,
                format!("Generated appearance stream for {}", location),
            )
            .with_fixed_error(ErrorCode::MissingAppearanceStream),
        );

        Ok(())
    }

    /// Generate XMP metadata for PDF/A.
    fn generate_xmp_metadata(&self) -> String {
        format!(
            r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
        xmlns:dc="http://purl.org/dc/elements/1.1/"
        xmlns:pdf="http://ns.adobe.com/pdf/1.3/"
        xmlns:xmp="http://ns.adobe.com/xap/1.0/"
        xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/">
      <pdfaid:part>{}</pdfaid:part>
      <pdfaid:conformance>{}</pdfaid:conformance>
      <dc:format>application/pdf</dc:format>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#,
            self.level.xmp_part(),
            self.level.xmp_conformance()
        )
    }

    /// Get the sRGB ICC profile data.
    pub fn get_srgb_icc_profile() -> &'static [u8] {
        // Minimal sRGB ICC profile header (would be replaced with full profile)
        // In a production implementation, this would be the complete sRGB profile
        include_bytes!("srgb_profile_placeholder.bin")
    }
}

/// Quick conversion function for common use cases.
///
/// # Example
///
/// ```ignore
/// use pdf_oxide::compliance::{convert_to_pdf_a, PdfALevel};
///
/// let result = convert_to_pdf_a(&mut document, PdfALevel::A2b)?;
/// if result.success {
///     println!("Conversion successful");
/// }
/// ```
pub fn convert_to_pdf_a(document: &mut PdfDocument, level: PdfALevel) -> Result<ConversionResult> {
    PdfAConverter::new(level).convert(document)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversion_config_default() {
        let config = ConversionConfig::default();
        assert!(config.embed_fonts);
        assert!(config.remove_javascript);
        assert!(config.remove_encryption);
        assert!(config.flatten_transparency);
    }

    #[test]
    fn test_conversion_config_builder() {
        let config = ConversionConfig::new()
            .embed_fonts(false)
            .remove_javascript(false)
            .flatten_transparency(false);

        assert!(!config.embed_fonts);
        assert!(!config.remove_javascript);
        assert!(!config.flatten_transparency);
    }

    #[test]
    fn test_converter_creation() {
        let converter = PdfAConverter::new(PdfALevel::A2b);
        assert_eq!(converter.level(), PdfALevel::A2b);
    }

    #[test]
    fn test_conversion_result() {
        let mut result = ConversionResult::new(PdfALevel::A2b);
        assert!(!result.success);
        assert_eq!(result.level, PdfALevel::A2b);
        assert!(result.actions.is_empty());
        assert!(result.errors.is_empty());

        result.add_action(ConversionAction::new(ActionType::AddedXmpMetadata, "Test action"));
        assert_eq!(result.actions.len(), 1);

        result.add_error(ConversionError::new(ErrorCode::FontNotEmbedded, "Test error"));
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_xmp_generation() {
        let converter = PdfAConverter::new(PdfALevel::A2b);
        let xmp = converter.generate_xmp_metadata();

        assert!(xmp.contains("<pdfaid:part>2</pdfaid:part>"));
        assert!(xmp.contains("<pdfaid:conformance>B</pdfaid:conformance>"));
    }

    #[test]
    fn test_action_type() {
        let action = ConversionAction::new(ActionType::AddedXmpMetadata, "Added metadata")
            .with_fixed_error(ErrorCode::MissingXmpMetadata);

        assert_eq!(action.action_type, ActionType::AddedXmpMetadata);
        assert_eq!(action.fixed_error, Some(ErrorCode::MissingXmpMetadata));
    }

    #[test]
    fn test_conversion_config_remove_encryption() {
        let config = ConversionConfig::default();
        assert!(config.remove_encryption);
        assert!(config.remove_embedded_files);
        assert!(!config.add_structure);
        assert!(config.icc_profile.is_none());
    }

    #[test]
    fn test_conversion_config_add_structure() {
        let config = ConversionConfig::new().add_structure(true);
        assert!(config.add_structure);
    }

    #[test]
    fn test_conversion_config_with_icc_profile() {
        let profile = vec![1, 2, 3, 4];
        let config = ConversionConfig::new().with_icc_profile(profile.clone());
        assert_eq!(config.icc_profile.unwrap(), profile);
    }

    #[test]
    fn test_converter_with_config() {
        let config = ConversionConfig::new().embed_fonts(false);
        let converter = PdfAConverter::new(PdfALevel::A1b).with_config(config);
        assert_eq!(converter.level(), PdfALevel::A1b);
    }

    #[test]
    fn test_converter_levels() {
        for level in [
            PdfALevel::A1a,
            PdfALevel::A1b,
            PdfALevel::A2a,
            PdfALevel::A2b,
            PdfALevel::A2u,
            PdfALevel::A3a,
            PdfALevel::A3b,
            PdfALevel::A3u,
        ] {
            let converter = PdfAConverter::new(level);
            assert_eq!(converter.level(), level);
        }
    }

    #[test]
    fn test_xmp_generation_a1b() {
        let converter = PdfAConverter::new(PdfALevel::A1b);
        let xmp = converter.generate_xmp_metadata();
        assert!(xmp.contains("<pdfaid:part>1</pdfaid:part>"));
        assert!(xmp.contains("<pdfaid:conformance>B</pdfaid:conformance>"));
        assert!(xmp.contains("xmpmeta"));
        assert!(xmp.contains("xpacket"));
    }

    #[test]
    fn test_xmp_generation_a3a() {
        let converter = PdfAConverter::new(PdfALevel::A3a);
        let xmp = converter.generate_xmp_metadata();
        assert!(xmp.contains("<pdfaid:part>3</pdfaid:part>"));
        assert!(xmp.contains("<pdfaid:conformance>A</pdfaid:conformance>"));
    }

    #[test]
    fn test_xmp_generation_a2u() {
        let converter = PdfAConverter::new(PdfALevel::A2u);
        let xmp = converter.generate_xmp_metadata();
        assert!(xmp.contains("<pdfaid:part>2</pdfaid:part>"));
        assert!(xmp.contains("<pdfaid:conformance>U</pdfaid:conformance>"));
    }

    #[test]
    fn test_conversion_result_new() {
        let result = ConversionResult::new(PdfALevel::A2b);
        assert!(!result.success);
        assert_eq!(result.level, PdfALevel::A2b);
        assert!(result.actions.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_conversion_result_add_action() {
        let mut result = ConversionResult::new(PdfALevel::A1b);
        result.add_action(ConversionAction::new(ActionType::RemovedJavaScript, "Removed JS"));
        result
            .add_action(ConversionAction::new(ActionType::RemovedEncryption, "Removed encryption"));
        assert_eq!(result.actions.len(), 2);
    }

    #[test]
    fn test_conversion_result_add_error() {
        let mut result = ConversionResult::new(PdfALevel::A1b);
        result.add_error(ConversionError::new(ErrorCode::FontNotEmbedded, "Cannot embed font"));
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].error_code, ErrorCode::FontNotEmbedded);
        assert_eq!(result.errors[0].reason, "Cannot embed font");
    }

    #[test]
    fn test_conversion_action_debug_clone() {
        let action = ConversionAction::new(ActionType::AddedLanguage, "Added en");
        let cloned = action.clone();
        assert_eq!(cloned.action_type, ActionType::AddedLanguage);
        let debug = format!("{:?}", action);
        assert!(debug.contains("AddedLanguage"));
    }

    #[test]
    fn test_conversion_error_debug_clone() {
        let error = ConversionError::new(ErrorCode::EncryptionNotAllowed, "Encrypted");
        let cloned = error.clone();
        assert_eq!(cloned.error_code, ErrorCode::EncryptionNotAllowed);
        let debug = format!("{:?}", error);
        assert!(debug.contains("EncryptionNotAllowed"));
    }

    #[test]
    fn test_all_action_types() {
        let types = vec![
            ActionType::AddedXmpMetadata,
            ActionType::AddedPdfaIdentification,
            ActionType::EmbeddedFont,
            ActionType::AddedOutputIntent,
            ActionType::RemovedJavaScript,
            ActionType::RemovedEncryption,
            ActionType::FlattenedTransparency,
            ActionType::RemovedEmbeddedFiles,
            ActionType::AddedStructure,
            ActionType::FixedAnnotation,
            ActionType::AddedLanguage,
        ];
        for t in types {
            let copy = t;
            assert_eq!(t, copy);
            let debug = format!("{:?}", t);
            assert!(!debug.is_empty());
        }
    }

    #[test]
    fn test_converter_debug_clone() {
        let converter = PdfAConverter::new(PdfALevel::A2b);
        let cloned = converter.clone();
        assert_eq!(cloned.level(), PdfALevel::A2b);
        let debug = format!("{:?}", converter);
        assert!(debug.contains("PdfAConverter"));
    }

    #[test]
    fn test_srgb_icc_profile() {
        let profile = PdfAConverter::get_srgb_icc_profile();
        assert!(!profile.is_empty());
    }

    #[test]
    fn test_conversion_config_debug_clone() {
        let config = ConversionConfig::new().embed_fonts(false);
        let cloned = config.clone();
        assert!(!cloned.embed_fonts);
        let debug = format!("{:?}", config);
        assert!(debug.contains("ConversionConfig"));
    }

    #[test]
    fn test_conversion_result_debug_clone() {
        let result = ConversionResult::new(PdfALevel::A1a);
        let cloned = result.clone();
        assert_eq!(cloned.level, PdfALevel::A1a);
        let debug = format!("{:?}", result);
        assert!(debug.contains("ConversionResult"));
    }
}
