//! PDF/X validator implementation.
//!
//! This module provides the main validator that coordinates all PDF/X compliance checks.

use super::types::{PdfXLevel, XComplianceError, XErrorCode, XValidationResult};
use crate::document::PdfDocument;
use crate::error::{Error, Result};
use crate::object::{Object, ObjectRef};
use std::collections::HashMap;

/// Type alias for PDF dictionary.
type Dictionary = HashMap<String, Object>;

/// PDF/X compliance validator.
///
/// This validator checks PDF documents against PDF/X standards (ISO 15930)
/// for print production workflows.
///
/// # Example
///
/// ```ignore
/// use pdf_oxide::api::Pdf;
/// use pdf_oxide::compliance::{PdfXValidator, PdfXLevel};
///
/// let pdf = Pdf::open("document.pdf")?;
/// let validator = PdfXValidator::new(PdfXLevel::X1a2003);
/// let result = validator.validate(&mut pdf.document())?;
///
/// if result.is_compliant {
///     println!("Document is PDF/X-1a:2003 compliant");
/// } else {
///     for error in &result.errors {
///         println!("Violation: {}", error);
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct PdfXValidator {
    /// Target PDF/X level
    level: PdfXLevel,
    /// Whether to stop on first error
    stop_on_first_error: bool,
    /// Whether to include warnings
    include_warnings: bool,
}

impl PdfXValidator {
    /// Create a new PDF/X validator for the specified level.
    pub fn new(level: PdfXLevel) -> Self {
        Self {
            level,
            stop_on_first_error: false,
            include_warnings: true,
        }
    }

    /// Configure whether to stop validation on the first error.
    pub fn stop_on_first_error(mut self, stop: bool) -> Self {
        self.stop_on_first_error = stop;
        self
    }

    /// Configure whether to include warnings in the validation result.
    pub fn include_warnings(mut self, include: bool) -> Self {
        self.include_warnings = include;
        self
    }

    /// Validate a PDF document against the configured PDF/X level.
    pub fn validate(&self, document: &mut PdfDocument) -> Result<XValidationResult> {
        let mut result = XValidationResult::new(self.level);

        // Run all validation checks
        self.validate_output_intent(document, &mut result)?;
        if self.should_stop(&result) {
            return Ok(self.finalize_result(result));
        }

        self.validate_metadata(document, &mut result)?;
        if self.should_stop(&result) {
            return Ok(self.finalize_result(result));
        }

        self.validate_info_dict(document, &mut result)?;
        if self.should_stop(&result) {
            return Ok(self.finalize_result(result));
        }

        self.validate_encryption(document, &mut result)?;
        if self.should_stop(&result) {
            return Ok(self.finalize_result(result));
        }

        self.validate_page_boxes(document, &mut result)?;
        if self.should_stop(&result) {
            return Ok(self.finalize_result(result));
        }

        self.validate_transparency(document, &mut result)?;
        if self.should_stop(&result) {
            return Ok(self.finalize_result(result));
        }

        self.validate_colors(document, &mut result)?;
        if self.should_stop(&result) {
            return Ok(self.finalize_result(result));
        }

        self.validate_fonts(document, &mut result)?;
        if self.should_stop(&result) {
            return Ok(self.finalize_result(result));
        }

        self.validate_annotations(document, &mut result)?;
        if self.should_stop(&result) {
            return Ok(self.finalize_result(result));
        }

        self.validate_actions(document, &mut result)?;

        Ok(self.finalize_result(result))
    }

    /// Validate output intent requirements.
    fn validate_output_intent(
        &self,
        document: &mut PdfDocument,
        result: &mut XValidationResult,
    ) -> Result<()> {
        let catalog = match self.get_catalog_dict(document)? {
            Some(d) => d,
            None => {
                result.add_error(XComplianceError::new(
                    XErrorCode::OutputIntentMissing,
                    "Document catalog is invalid",
                ));
                return Ok(());
            },
        };

        // Check for OutputIntents array
        let output_intents = match catalog.get("OutputIntents") {
            Some(Object::Array(arr)) => arr.clone(),
            Some(Object::Reference(r)) => {
                // Dereference if needed
                match document.load_object(*r)? {
                    Object::Array(arr) => arr,
                    _ => {
                        result.add_error(
                            XComplianceError::new(
                                XErrorCode::OutputIntentMissing,
                                "OutputIntents must be an array",
                            )
                            .with_clause("6.2.2"),
                        );
                        return Ok(());
                    },
                }
            },
            _ => {
                result.add_error(
                    XComplianceError::new(
                        XErrorCode::OutputIntentMissing,
                        "OutputIntents array is required for PDF/X",
                    )
                    .with_clause("6.2.2"),
                );
                return Ok(());
            },
        };

        if output_intents.is_empty() {
            result.add_error(
                XComplianceError::new(
                    XErrorCode::OutputIntentMissing,
                    "OutputIntents array is empty",
                )
                .with_clause("6.2.2"),
            );
            return Ok(());
        }

        // Check for GTS_PDFX output intent
        let mut found_pdfx_intent = false;
        for intent_obj in &output_intents {
            let intent = match intent_obj {
                Object::Dictionary(d) => d.clone(),
                Object::Reference(r) => match document.load_object(*r)? {
                    Object::Dictionary(d) => d,
                    _ => continue,
                },
                _ => continue,
            };

            // Check S (subtype) key
            if let Some(Object::Name(s)) = intent.get("S") {
                if s == "GTS_PDFX" {
                    found_pdfx_intent = true;

                    // Check OutputConditionIdentifier
                    if !intent.contains_key("OutputConditionIdentifier") {
                        result.add_error(
                            XComplianceError::new(
                                XErrorCode::OutputConditionMissing,
                                "OutputConditionIdentifier is required in output intent",
                            )
                            .with_clause("6.2.2"),
                        );
                    }

                    // Store output intent info in stats
                    if let Some(Object::String(oci)) = intent.get("OutputConditionIdentifier") {
                        result.stats.output_intent = Some(String::from_utf8_lossy(oci).to_string());
                    }

                    break;
                }
            }
        }

        if !found_pdfx_intent {
            result.add_error(
                XComplianceError::new(
                    XErrorCode::OutputIntentInvalid,
                    "GTS_PDFX output intent is required",
                )
                .with_clause("6.2.2"),
            );
        }

        Ok(())
    }

    /// Validate XMP metadata requirements.
    fn validate_metadata(
        &self,
        document: &mut PdfDocument,
        result: &mut XValidationResult,
    ) -> Result<()> {
        let catalog = match self.get_catalog_dict(document)? {
            Some(d) => d,
            None => return Ok(()),
        };

        // Check for Metadata entry
        if !catalog.contains_key("Metadata") {
            result.add_error(
                XComplianceError::new(
                    XErrorCode::XmpMetadataMissing,
                    "XMP metadata stream is required for PDF/X",
                )
                .with_clause("6.7.2"),
            );
        }

        // Parse XMP and validate PDF/X identification (pdfxid:GTS_PDFXVersion)
        match crate::extractors::xmp::XmpExtractor::extract(document) {
            Ok(Some(xmp)) => {
                let pdfx_version = xmp.custom.get("pdfxid:GTS_PDFXVersion");

                if pdfx_version.is_none() {
                    result.add_warning(
                        XComplianceError::warning(
                            XErrorCode::XmpMetadataInvalid,
                            "XMP metadata missing pdfxid:GTS_PDFXVersion identification",
                        )
                        .with_clause("6.7.2"),
                    );
                } else if let Some(version_str) = pdfx_version {
                    // Validate version matches declared level
                    let expected = self.level.xmp_version();
                    if version_str != expected {
                        result.add_warning(
                            XComplianceError::warning(
                                XErrorCode::XmpMetadataInvalid,
                                format!(
                                    "XMP pdfxid:GTS_PDFXVersion is '{}' but validating against {} (expected '{}')",
                                    version_str, self.level, expected
                                ),
                            )
                            .with_clause("6.7.2"),
                        );
                    }

                    // Try to detect level from XMP
                    if let Some(detected) = PdfXLevel::from_gts_version(version_str) {
                        if result.detected_level.is_none() {
                            result.detected_level = Some(detected);
                        }
                    }
                }
            },
            Ok(None) => {
                result.add_warning(
                    XComplianceError::warning(
                        XErrorCode::XmpMetadataInvalid,
                        "Could not extract XMP metadata for PDF/X identification",
                    )
                    .with_clause("6.7.2"),
                );
            },
            Err(_) => {
                result.add_warning(
                    XComplianceError::warning(
                        XErrorCode::XmpMetadataInvalid,
                        "Failed to parse XMP metadata for PDF/X identification",
                    )
                    .with_clause("6.7.2"),
                );
            },
        }

        Ok(())
    }

    /// Validate Info dictionary requirements.
    fn validate_info_dict(
        &self,
        document: &mut PdfDocument,
        result: &mut XValidationResult,
    ) -> Result<()> {
        let info = match self.get_info_dict(document)? {
            Some(d) => d,
            None => {
                // Info dict is recommended but we'll add a warning
                result.add_warning(XComplianceError::warning(
                    XErrorCode::GtsPdfxVersionMissing,
                    "Info dictionary not found",
                ));
                return Ok(());
            },
        };

        // Check GTS_PDFXVersion
        if !info.contains_key("GTS_PDFXVersion") {
            result.add_error(
                XComplianceError::new(
                    XErrorCode::GtsPdfxVersionMissing,
                    "GTS_PDFXVersion key is required in Info dictionary",
                )
                .with_clause("6.7.5"),
            );
        } else {
            // Try to detect the PDF/X level
            if let Some(Object::String(version)) = info.get("GTS_PDFXVersion") {
                let version_str = String::from_utf8_lossy(version);
                if let Some(detected) = PdfXLevel::from_gts_version(&version_str) {
                    result.detected_level = Some(detected);
                }
            }
        }

        // Check GTS_PDFXConformance for X-1a and X-3
        if matches!(
            self.level,
            PdfXLevel::X1a2001 | PdfXLevel::X1a2003 | PdfXLevel::X32002 | PdfXLevel::X32003
        ) && !info.contains_key("GTS_PDFXConformance")
        {
            result.add_error(
                XComplianceError::new(
                    XErrorCode::GtsPdfxConformanceMissing,
                    "GTS_PDFXConformance key is required for PDF/X-1a and PDF/X-3",
                )
                .with_clause("6.7.5"),
            );
        }

        // Check Trapped key (required for PDF/X)
        if !info.contains_key("Trapped") {
            result.add_warning(
                XComplianceError::warning(
                    XErrorCode::TrappedKeyMissing,
                    "Trapped key should be present in Info dictionary",
                )
                .with_clause("6.7.5"),
            );
        }

        Ok(())
    }

    /// Validate encryption (not allowed in PDF/X).
    fn validate_encryption(
        &self,
        document: &mut PdfDocument,
        result: &mut XValidationResult,
    ) -> Result<()> {
        // Check the trailer for /Encrypt entry
        let trailer = document.trailer();
        let is_encrypted = if let Object::Dictionary(trailer_dict) = trailer {
            trailer_dict.contains_key("Encrypt")
        } else {
            false
        };

        if is_encrypted {
            result.add_error(
                XComplianceError::new(
                    XErrorCode::EncryptionNotAllowed,
                    "Encryption is not allowed in PDF/X documents",
                )
                .with_clause("6.1.12"),
            );
        }
        Ok(())
    }

    /// Validate page box requirements.
    fn validate_page_boxes(
        &self,
        document: &mut PdfDocument,
        result: &mut XValidationResult,
    ) -> Result<()> {
        let page_count = document.page_count()?;
        result.stats.pages_checked = page_count;

        for page_num in 0..page_count {
            if let Ok(page_dict) = self.get_page_dict(document, page_num) {
                // Check MediaBox (required)
                if !page_dict.contains_key("MediaBox") {
                    result.add_error(
                        XComplianceError::new(XErrorCode::MediaBoxMissing, "MediaBox is required")
                            .with_page(page_num)
                            .with_clause("6.1.1"),
                    );
                }

                // Check TrimBox or ArtBox (at least one required for PDF/X)
                let has_trim = page_dict.contains_key("TrimBox");
                let has_art = page_dict.contains_key("ArtBox");

                if !has_trim && !has_art {
                    result.add_error(
                        XComplianceError::new(
                            XErrorCode::TrimOrArtBoxMissing,
                            "Either TrimBox or ArtBox is required for PDF/X",
                        )
                        .with_page(page_num)
                        .with_clause("6.1.1"),
                    );
                }

                // Validate box relationships: TrimBox ⊆ BleedBox ⊆ MediaBox
                if let Some(media_box) = Self::parse_box(page_dict.get("MediaBox")) {
                    let bleed_box = Self::parse_box(page_dict.get("BleedBox"));
                    let trim_box = Self::parse_box(page_dict.get("TrimBox"));
                    let art_box = Self::parse_box(page_dict.get("ArtBox"));

                    // BleedBox must be within MediaBox
                    if let Some(bb) = bleed_box {
                        if !Self::box_contains(&media_box, &bb) {
                            result.add_error(
                                XComplianceError::new(
                                    XErrorCode::BleedBoxInvalid,
                                    "BleedBox extends beyond MediaBox",
                                )
                                .with_page(page_num)
                                .with_clause("6.1.1"),
                            );
                        }

                        // TrimBox must be within BleedBox (if BleedBox exists)
                        if let Some(tb) = trim_box {
                            if !Self::box_contains(&bb, &tb) {
                                result.add_error(
                                    XComplianceError::new(
                                        XErrorCode::TrimBoxInvalid,
                                        "TrimBox extends beyond BleedBox",
                                    )
                                    .with_page(page_num)
                                    .with_clause("6.1.1"),
                                );
                            }
                        }
                    }

                    // TrimBox must be within MediaBox
                    if let Some(tb) = trim_box {
                        if !Self::box_contains(&media_box, &tb) {
                            result.add_error(
                                XComplianceError::new(
                                    XErrorCode::TrimBoxInvalid,
                                    "TrimBox extends beyond MediaBox",
                                )
                                .with_page(page_num)
                                .with_clause("6.1.1"),
                            );
                        }
                    }

                    // ArtBox must be within MediaBox
                    if let Some(ab) = art_box {
                        if !Self::box_contains(&media_box, &ab) {
                            result.add_error(
                                XComplianceError::new(
                                    XErrorCode::BoxesInconsistent,
                                    "ArtBox extends beyond MediaBox",
                                )
                                .with_page(page_num)
                                .with_clause("6.1.1"),
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate transparency requirements.
    fn validate_transparency(
        &self,
        document: &mut PdfDocument,
        result: &mut XValidationResult,
    ) -> Result<()> {
        // Only check if transparency is not allowed for this level
        if self.level.allows_transparency() {
            return Ok(());
        }

        // Check catalog for OutputIntents with transparency group
        let _catalog = match self.get_catalog_dict(document)? {
            Some(d) => d,
            None => return Ok(()),
        };

        // Check for transparency-related entries
        // Pages with Group entry having S=Transparency
        let page_count = document.page_count()?;
        for page_num in 0..page_count {
            if let Ok(page_dict) = self.get_page_dict(document, page_num) {
                if let Some(Object::Dictionary(group)) = page_dict.get("Group") {
                    if let Some(Object::Name(s)) = group.get("S") {
                        if s == "Transparency" {
                            result.add_error(
                                XComplianceError::new(
                                    XErrorCode::TransparencyNotAllowed,
                                    "Transparency groups are not allowed in this PDF/X level",
                                )
                                .with_page(page_num)
                                .with_clause("6.3"),
                            );
                            result.stats.has_transparency = true;
                        }
                    }
                }
            }
        }

        // Check ExtGState for transparency-related entries (SMask, CA, ca, BM)
        let page_count2 = document.page_count()?;
        for page_num in 0..page_count2 {
            if let Ok(page_dict) = self.get_page_dict(document, page_num) {
                let resources = match page_dict.get("Resources") {
                    Some(Object::Dictionary(d)) => d.clone(),
                    Some(Object::Reference(r)) => match document.load_object(*r)? {
                        Object::Dictionary(d) => d,
                        _ => continue,
                    },
                    _ => continue,
                };

                let ext_gstate = match resources.get("ExtGState") {
                    Some(Object::Dictionary(d)) => d.clone(),
                    Some(Object::Reference(r)) => match document.load_object(*r)? {
                        Object::Dictionary(d) => d,
                        _ => continue,
                    },
                    _ => continue,
                };

                for (gs_name, gs_obj) in &ext_gstate {
                    let gs_dict = match gs_obj {
                        Object::Dictionary(d) => d.clone(),
                        Object::Reference(r) => match document.load_object(*r)? {
                            Object::Dictionary(d) => d,
                            _ => continue,
                        },
                        _ => continue,
                    };

                    // Check SMask (not /None means transparency)
                    if let Some(smask) = gs_dict.get("SMask") {
                        let is_none = matches!(smask, Object::Name(n) if n == "None");
                        if !is_none {
                            result.add_error(
                                XComplianceError::new(
                                    XErrorCode::SMaskNotAllowed,
                                    format!(
                                        "ExtGState '{}' has SMask (transparency not allowed)",
                                        gs_name
                                    ),
                                )
                                .with_page(page_num)
                                .with_clause("6.3"),
                            );
                            result.stats.has_transparency = true;
                        }
                    }

                    // Check CA (fill opacity) < 1.0
                    if let Some(ca_val) = gs_dict.get("CA") {
                        let opacity = match ca_val {
                            Object::Real(v) => Some(*v),
                            Object::Integer(v) => Some(*v as f64),
                            _ => None,
                        };
                        if let Some(op) = opacity {
                            if op < 1.0 {
                                result.add_error(
                                    XComplianceError::new(
                                        XErrorCode::TransparencyNotAllowed,
                                        format!(
                                            "ExtGState '{}' has non-opaque CA={} (transparency not allowed)",
                                            gs_name, op
                                        ),
                                    )
                                    .with_page(page_num)
                                    .with_clause("6.3"),
                                );
                                result.stats.has_transparency = true;
                            }
                        }
                    }

                    // Check ca (stroke opacity) < 1.0
                    if let Some(ca_val) = gs_dict.get("ca") {
                        let opacity = match ca_val {
                            Object::Real(v) => Some(*v),
                            Object::Integer(v) => Some(*v as f64),
                            _ => None,
                        };
                        if let Some(op) = opacity {
                            if op < 1.0 {
                                result.add_error(
                                    XComplianceError::new(
                                        XErrorCode::TransparencyNotAllowed,
                                        format!(
                                            "ExtGState '{}' has non-opaque ca={} (transparency not allowed)",
                                            gs_name, op
                                        ),
                                    )
                                    .with_page(page_num)
                                    .with_clause("6.3"),
                                );
                                result.stats.has_transparency = true;
                            }
                        }
                    }

                    // Check BM (blend mode) not Normal/Compatible
                    if let Some(Object::Name(bm)) = gs_dict.get("BM") {
                        if bm != "Normal" && bm != "Compatible" {
                            result.add_error(
                                XComplianceError::new(
                                    XErrorCode::BlendModeNotAllowed,
                                    format!(
                                        "ExtGState '{}' has blend mode '{}' (only Normal/Compatible allowed)",
                                        gs_name, bm
                                    ),
                                )
                                .with_page(page_num)
                                .with_clause("6.3"),
                            );
                            result.stats.has_transparency = true;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate color space requirements.
    fn validate_colors(
        &self,
        document: &mut PdfDocument,
        result: &mut XValidationResult,
    ) -> Result<()> {
        // For PDF/X-1a, only CMYK and spot colors are allowed
        if !self.level.allows_rgb() {
            // Check page resources for RGB color spaces
            let page_count = document.page_count()?;
            for page_num in 0..page_count {
                if let Ok(page_dict) = self.get_page_dict(document, page_num) {
                    if let Some(Object::Dictionary(resources)) = page_dict.get("Resources") {
                        if let Some(Object::Dictionary(colorspaces)) = resources.get("ColorSpace") {
                            for (name, cs) in colorspaces {
                                let cs_name = self.get_colorspace_name(cs, document)?;
                                result.stats.color_spaces_found.push(cs_name.clone());

                                if cs_name == "DeviceRGB" || cs_name == "CalRGB" {
                                    result.add_error(
                                        XComplianceError::new(
                                            XErrorCode::RgbColorNotAllowed,
                                            format!(
                                                "RGB color space '{}' not allowed in PDF/X-1a",
                                                name
                                            ),
                                        )
                                        .with_page(page_num)
                                        .with_clause("6.2.3"),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check for device-dependent colors without output intent
        // and validate ICC profiles in color space resources
        let has_output_intent = result.stats.output_intent.is_some();
        let page_count2 = document.page_count()?;
        for page_num in 0..page_count2 {
            if let Ok(page_dict) = self.get_page_dict(document, page_num) {
                let resources = match page_dict.get("Resources") {
                    Some(Object::Dictionary(d)) => d.clone(),
                    Some(Object::Reference(r)) => match document.load_object(*r)? {
                        Object::Dictionary(d) => d,
                        _ => continue,
                    },
                    _ => continue,
                };

                if let Some(colorspaces_obj) = resources.get("ColorSpace") {
                    let colorspaces = match colorspaces_obj {
                        Object::Dictionary(d) => d.clone(),
                        Object::Reference(r) => match document.load_object(*r)? {
                            Object::Dictionary(d) => d,
                            _ => continue,
                        },
                        _ => continue,
                    };

                    for (cs_name, cs_obj) in &colorspaces {
                        let cs_type = self.get_colorspace_name(cs_obj, document)?;

                        // Device-dependent colors without output intent
                        if !has_output_intent
                            && (cs_type == "DeviceRGB"
                                || cs_type == "DeviceCMYK"
                                || cs_type == "DeviceGray")
                        {
                            result.add_error(
                                XComplianceError::new(
                                    XErrorCode::DeviceColorWithoutIntent,
                                    format!(
                                        "Device color space '{}' ({}) used without output intent",
                                        cs_name, cs_type
                                    ),
                                )
                                .with_page(page_num)
                                .with_clause("6.2.3"),
                            );
                        }

                        // Validate ICCBased color space profiles
                        if cs_type == "ICCBased" {
                            self.validate_icc_profile(cs_obj, cs_name, page_num, document, result)?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate font requirements.
    fn validate_fonts(
        &self,
        document: &mut PdfDocument,
        result: &mut XValidationResult,
    ) -> Result<()> {
        // Check page resources for fonts
        let page_count = document.page_count()?;
        for page_num in 0..page_count {
            if let Ok(page_dict) = self.get_page_dict(document, page_num) {
                if let Some(Object::Dictionary(resources)) = page_dict.get("Resources") {
                    if let Some(Object::Dictionary(fonts)) = resources.get("Font") {
                        for (name, font_ref) in fonts {
                            result.stats.fonts_checked += 1;

                            let font_dict = match font_ref {
                                Object::Dictionary(d) => d.clone(),
                                Object::Reference(r) => match document.load_object(*r)? {
                                    Object::Dictionary(d) => d,
                                    _ => continue,
                                },
                                _ => continue,
                            };

                            // Check font type
                            if let Some(Object::Name(subtype)) = font_dict.get("Subtype") {
                                if subtype == "Type3" {
                                    result.add_error(
                                        XComplianceError::new(
                                            XErrorCode::Type3FontNotAllowed,
                                            format!("Type 3 font '{}' not allowed in PDF/X", name),
                                        )
                                        .with_page(page_num)
                                        .with_clause("6.3.5"),
                                    );
                                }
                            }

                            // Check if font is embedded
                            let is_embedded = font_dict.contains_key("FontFile")
                                || font_dict.contains_key("FontFile2")
                                || font_dict.contains_key("FontFile3");

                            // Also check FontDescriptor
                            let descriptor_embedded = if let Some(Object::Dictionary(fd)) =
                                font_dict.get("FontDescriptor")
                            {
                                fd.contains_key("FontFile")
                                    || fd.contains_key("FontFile2")
                                    || fd.contains_key("FontFile3")
                            } else if let Some(Object::Reference(r)) =
                                font_dict.get("FontDescriptor")
                            {
                                if let Object::Dictionary(fd) = document.load_object(*r)? {
                                    fd.contains_key("FontFile")
                                        || fd.contains_key("FontFile2")
                                        || fd.contains_key("FontFile3")
                                } else {
                                    false
                                }
                            } else {
                                false
                            };

                            if is_embedded || descriptor_embedded {
                                result.stats.fonts_embedded += 1;
                            } else {
                                // Standard 14 fonts might not be embedded but should have widths
                                let is_standard14 = self.is_standard14_font(&font_dict);
                                if !is_standard14 {
                                    result.add_error(
                                        XComplianceError::new(
                                            XErrorCode::FontNotEmbedded,
                                            format!("Font '{}' must be embedded", name),
                                        )
                                        .with_page(page_num)
                                        .with_clause("6.3.5"),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate annotation requirements.
    fn validate_annotations(
        &self,
        document: &mut PdfDocument,
        result: &mut XValidationResult,
    ) -> Result<()> {
        let page_count = document.page_count()?;
        for page_num in 0..page_count {
            if let Ok(page_dict) = self.get_page_dict(document, page_num) {
                let annots = match page_dict.get("Annots") {
                    Some(Object::Array(arr)) => arr.clone(),
                    Some(Object::Reference(r)) => match document.load_object(*r)? {
                        Object::Array(arr) => arr,
                        _ => continue,
                    },
                    _ => continue,
                };

                for annot_obj in annots {
                    result.stats.annotations_checked += 1;

                    let annot = match annot_obj {
                        Object::Dictionary(d) => d,
                        Object::Reference(r) => match document.load_object(r)? {
                            Object::Dictionary(d) => d,
                            _ => continue,
                        },
                        _ => continue,
                    };

                    // Check annotation subtype
                    if let Some(Object::Name(subtype)) = annot.get("Subtype") {
                        // Only TrapNet and PrinterMark are allowed in PDF/X
                        // Other allowed: Link (with restrictions), Widget (form fields)
                        match subtype.as_str() {
                            "TrapNet" | "PrinterMark" => {
                                // Allowed
                            },
                            "Link" | "Widget" => {
                                // Allowed with restrictions - check appearance
                                if !annot.contains_key("AP") {
                                    result.add_warning(
                                        XComplianceError::warning(
                                            XErrorCode::AnnotationNotAllowed,
                                            format!(
                                                "{} annotation should have appearance stream",
                                                subtype
                                            ),
                                        )
                                        .with_page(page_num),
                                    );
                                }
                            },
                            _ => {
                                // Other annotation types may not be allowed
                                // (depends on specific PDF/X level requirements)
                                result.add_warning(
                                    XComplianceError::warning(
                                        XErrorCode::AnnotationNotAllowed,
                                        format!(
                                            "Annotation type '{}' may not be allowed in PDF/X",
                                            subtype
                                        ),
                                    )
                                    .with_page(page_num),
                                );
                            },
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate action requirements.
    fn validate_actions(
        &self,
        document: &mut PdfDocument,
        result: &mut XValidationResult,
    ) -> Result<()> {
        let catalog = match self.get_catalog_dict(document)? {
            Some(d) => d,
            None => return Ok(()),
        };

        // Check for JavaScript in Names dictionary
        if let Some(names_obj) = catalog.get("Names") {
            let names = match names_obj {
                Object::Dictionary(d) => d.clone(),
                Object::Reference(r) => match document.load_object(*r)? {
                    Object::Dictionary(d) => d,
                    _ => HashMap::new(),
                },
                _ => HashMap::new(),
            };
            if names.contains_key("JavaScript") {
                result.add_error(
                    XComplianceError::new(
                        XErrorCode::JavaScriptNotAllowed,
                        "JavaScript is not allowed in PDF/X documents",
                    )
                    .with_clause("6.6.1"),
                );
            }
        }

        // Check OpenAction
        if let Some(action) = catalog.get("OpenAction") {
            self.check_action(action, document, result)?;
        }

        // Check AA (Additional Actions)
        if catalog.contains_key("AA") {
            result.add_warning(XComplianceError::warning(
                XErrorCode::ActionNotAllowed,
                "Additional actions (AA) may not be compatible with PDF/X",
            ));
        }

        Ok(())
    }

    // Helper methods

    fn get_catalog_dict(&self, document: &mut PdfDocument) -> Result<Option<Dictionary>> {
        let catalog = document.catalog()?;
        match catalog {
            Object::Dictionary(d) => Ok(Some(d)),
            _ => Ok(None),
        }
    }

    /// Get the Info dictionary from the trailer.
    fn get_info_dict(&self, document: &mut PdfDocument) -> Result<Option<Dictionary>> {
        let trailer = document.trailer();
        let trailer_dict = match trailer {
            Object::Dictionary(d) => d,
            _ => return Ok(None),
        };

        // Get /Info reference
        let info_ref = match trailer_dict.get("Info") {
            Some(Object::Reference(r)) => *r,
            Some(Object::Dictionary(d)) => return Ok(Some(d.clone())),
            _ => return Ok(None),
        };

        // Load the Info dictionary
        let info_obj = document.load_object(info_ref)?;
        match info_obj {
            Object::Dictionary(d) => Ok(Some(d)),
            _ => Ok(None),
        }
    }

    /// Get a page dictionary by index by walking the page tree.
    fn get_page_dict(&self, document: &mut PdfDocument, page_num: usize) -> Result<Dictionary> {
        // Get catalog and pages tree
        let catalog = match self.get_catalog_dict(document)? {
            Some(d) => d,
            None => {
                return Err(Error::InvalidPdf("Invalid catalog".to_string()));
            },
        };

        // Get /Pages reference
        let pages_ref = match catalog.get("Pages") {
            Some(Object::Reference(r)) => *r,
            _ => {
                return Err(Error::InvalidPdf("Pages entry missing or invalid".to_string()));
            },
        };

        // Walk the page tree to find the specific page
        self.get_page_from_tree(document, pages_ref, page_num, &mut 0)
    }

    /// Recursively find a page in the page tree.
    #[allow(clippy::only_used_in_recursion)]
    fn get_page_from_tree(
        &self,
        document: &mut PdfDocument,
        node_ref: ObjectRef,
        target_index: usize,
        current_index: &mut usize,
    ) -> Result<Dictionary> {
        let node = document.load_object(node_ref)?;
        let node_dict = match node {
            Object::Dictionary(d) => d,
            _ => return Err(Error::InvalidPdf("Invalid page tree node".to_string())),
        };

        // Check node type
        let node_type = node_dict
            .get("Type")
            .and_then(|o| {
                if let Object::Name(n) = o {
                    Some(n.as_str())
                } else {
                    None
                }
            })
            .unwrap_or("");

        if node_type == "Page" {
            // This is a page node
            if *current_index == target_index {
                return Ok(node_dict);
            }
            *current_index += 1;
            return Err(Error::InvalidPdf("Page not found".to_string()));
        }

        // This is a Pages node - iterate through Kids
        let kids = match node_dict.get("Kids") {
            Some(Object::Array(arr)) => arr.clone(),
            Some(Object::Reference(r)) => match document.load_object(*r)? {
                Object::Array(arr) => arr,
                _ => return Err(Error::InvalidPdf("Invalid Kids array".to_string())),
            },
            _ => return Err(Error::InvalidPdf("Missing Kids array".to_string())),
        };

        for kid in kids {
            let kid_ref = match kid {
                Object::Reference(r) => r,
                _ => continue,
            };

            // Try to get the page count for this subtree (optimization)
            let kid_obj = document.load_object(kid_ref)?;
            if let Object::Dictionary(kid_dict) = &kid_obj {
                if let Some(Object::Integer(count)) = kid_dict.get("Count") {
                    let count = *count as usize;
                    if *current_index + count <= target_index {
                        // The target page is not in this subtree
                        *current_index += count;
                        continue;
                    }
                }
            }

            // Recursively search this subtree
            match self.get_page_from_tree(document, kid_ref, target_index, current_index) {
                Ok(page) => return Ok(page),
                Err(_) => continue,
            }
        }

        Err(Error::InvalidPdf(format!("Page {} not found", target_index)))
    }

    #[allow(clippy::only_used_in_recursion)]
    fn get_colorspace_name(&self, cs: &Object, document: &mut PdfDocument) -> Result<String> {
        match cs {
            Object::Name(n) => Ok(n.clone()),
            Object::Array(arr) => {
                if let Some(Object::Name(n)) = arr.first() {
                    Ok(n.clone())
                } else {
                    Ok("Unknown".to_string())
                }
            },
            Object::Reference(r) => {
                let resolved = document.load_object(*r)?;
                self.get_colorspace_name(&resolved, document)
            },
            _ => Ok("Unknown".to_string()),
        }
    }

    fn is_standard14_font(&self, font_dict: &Dictionary) -> bool {
        if let Some(Object::Name(base_font)) = font_dict.get("BaseFont") {
            let standard14 = [
                "Courier",
                "Courier-Bold",
                "Courier-Oblique",
                "Courier-BoldOblique",
                "Helvetica",
                "Helvetica-Bold",
                "Helvetica-Oblique",
                "Helvetica-BoldOblique",
                "Times-Roman",
                "Times-Bold",
                "Times-Italic",
                "Times-BoldItalic",
                "Symbol",
                "ZapfDingbats",
            ];
            return standard14.contains(&base_font.as_str());
        }
        false
    }

    fn check_action(
        &self,
        action: &Object,
        document: &mut PdfDocument,
        result: &mut XValidationResult,
    ) -> Result<()> {
        let action_dict = match action {
            Object::Dictionary(d) => d.clone(),
            Object::Reference(r) => match document.load_object(*r)? {
                Object::Dictionary(d) => d,
                _ => return Ok(()),
            },
            _ => return Ok(()),
        };

        if let Some(Object::Name(action_type)) = action_dict.get("S") {
            match action_type.as_str() {
                "JavaScript" => {
                    result.add_error(
                        XComplianceError::new(
                            XErrorCode::JavaScriptNotAllowed,
                            "JavaScript actions are not allowed in PDF/X",
                        )
                        .with_clause("6.6.1"),
                    );
                },
                "Launch" | "Sound" | "Movie" | "ImportData" | "ResetForm" | "SubmitForm" => {
                    result.add_error(
                        XComplianceError::new(
                            XErrorCode::ActionNotAllowed,
                            format!("Action type '{}' is not allowed in PDF/X", action_type),
                        )
                        .with_clause("6.6.1"),
                    );
                },
                _ => {},
            }
        }

        Ok(())
    }

    /// Parse a PDF rectangle array [llx, lly, urx, ury] into [f64; 4].
    fn parse_box(obj: Option<&Object>) -> Option<[f64; 4]> {
        let arr = match obj? {
            Object::Array(a) => a,
            _ => return None,
        };
        if arr.len() < 4 {
            return None;
        }
        let to_f64 = |o: &Object| -> Option<f64> {
            match o {
                Object::Real(v) => Some(*v),
                Object::Integer(v) => Some(*v as f64),
                _ => None,
            }
        };
        Some([
            to_f64(&arr[0])?,
            to_f64(&arr[1])?,
            to_f64(&arr[2])?,
            to_f64(&arr[3])?,
        ])
    }

    /// Check if outer box fully contains inner box (with 0.01pt tolerance).
    fn box_contains(outer: &[f64; 4], inner: &[f64; 4]) -> bool {
        const TOLERANCE: f64 = 0.01;
        // outer[0] <= inner[0] (left)
        // outer[1] <= inner[1] (bottom)
        // outer[2] >= inner[2] (right)
        // outer[3] >= inner[3] (top)
        (outer[0] - TOLERANCE) <= inner[0]
            && (outer[1] - TOLERANCE) <= inner[1]
            && (outer[2] + TOLERANCE) >= inner[2]
            && (outer[3] + TOLERANCE) >= inner[3]
    }

    /// Validate an ICCBased color space profile stream.
    fn validate_icc_profile(
        &self,
        cs_obj: &Object,
        cs_name: &str,
        page_num: usize,
        document: &mut PdfDocument,
        result: &mut XValidationResult,
    ) -> Result<()> {
        // ICCBased is [/ICCBased, stream_ref]
        let arr = match cs_obj {
            Object::Array(a) => a.clone(),
            Object::Reference(r) => match document.load_object(*r)? {
                Object::Array(a) => a,
                _ => return Ok(()),
            },
            _ => return Ok(()),
        };
        if arr.len() < 2 {
            return Ok(());
        }

        // Get the ICC profile stream dictionary
        let profile_dict = match &arr[1] {
            Object::Dictionary(d) => d.clone(),
            Object::Reference(r) => match document.load_object(*r)? {
                Object::Dictionary(d) => d,
                _ => {
                    result.add_error(
                        XComplianceError::new(
                            XErrorCode::IccProfileInvalid,
                            format!("ICC profile for '{}' is not a valid stream", cs_name),
                        )
                        .with_page(page_num)
                        .with_clause("6.2.3"),
                    );
                    return Ok(());
                },
            },
            _ => return Ok(()),
        };

        // Check /N entry (number of color components)
        if !profile_dict.contains_key("N") {
            result.add_error(
                XComplianceError::new(
                    XErrorCode::IccProfileInvalid,
                    format!("ICC profile for '{}' missing required /N entry", cs_name),
                )
                .with_page(page_num)
                .with_clause("6.2.3"),
            );
        }

        Ok(())
    }

    fn should_stop(&self, result: &XValidationResult) -> bool {
        self.stop_on_first_error && result.has_errors()
    }

    fn finalize_result(&self, mut result: XValidationResult) -> XValidationResult {
        result.is_compliant = !result.has_errors();

        if !self.include_warnings {
            result.warnings.clear();
        }

        result
    }
}

/// Quick validation function for common use cases.
///
/// # Example
///
/// ```ignore
/// use pdf_oxide::compliance::{validate_pdf_x, PdfXLevel};
///
/// let result = validate_pdf_x(&mut document, PdfXLevel::X1a2003)?;
/// println!("Compliant: {}", result.is_compliant);
/// ```
pub fn validate_pdf_x(document: &mut PdfDocument, level: PdfXLevel) -> Result<XValidationResult> {
    PdfXValidator::new(level).validate(document)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_creation() {
        let validator = PdfXValidator::new(PdfXLevel::X1a2003);
        assert_eq!(validator.level, PdfXLevel::X1a2003);
        assert!(!validator.stop_on_first_error);
        assert!(validator.include_warnings);
    }

    #[test]
    fn test_validator_builder() {
        let validator = PdfXValidator::new(PdfXLevel::X4)
            .stop_on_first_error(true)
            .include_warnings(false);

        assert!(validator.stop_on_first_error);
        assert!(!validator.include_warnings);
    }

    #[test]
    fn test_standard14_fonts() {
        let validator = PdfXValidator::new(PdfXLevel::X1a2003);

        let mut font_dict = HashMap::new();
        font_dict.insert("BaseFont".to_string(), Object::Name("Helvetica".to_string()));
        assert!(validator.is_standard14_font(&font_dict));

        font_dict.insert("BaseFont".to_string(), Object::Name("CustomFont".to_string()));
        assert!(!validator.is_standard14_font(&font_dict));
    }

    #[test]
    fn test_finalize_result() {
        let validator = PdfXValidator::new(PdfXLevel::X1a2003);
        let result = XValidationResult::new(PdfXLevel::X1a2003);
        let finalized = validator.finalize_result(result);
        assert!(finalized.is_compliant);

        let mut result_with_error = XValidationResult::new(PdfXLevel::X1a2003);
        result_with_error
            .add_error(XComplianceError::new(XErrorCode::FontNotEmbedded, "Test error"));
        let finalized = validator.finalize_result(result_with_error);
        assert!(!finalized.is_compliant);
    }

    #[test]
    fn test_finalize_without_warnings() {
        let validator = PdfXValidator::new(PdfXLevel::X1a2003).include_warnings(false);
        let mut result = XValidationResult::new(PdfXLevel::X1a2003);
        result
            .add_warning(XComplianceError::warning(XErrorCode::TrappedKeyMissing, "Test warning"));

        let finalized = validator.finalize_result(result);
        assert!(finalized.warnings.is_empty());
    }
}
