//! PDF Digital Signatures module.
//!
//! This module provides functionality for creating and verifying digital signatures
//! in PDF documents according to the PDF specification and PAdES (PDF Advanced
//! Electronic Signatures) standards.
//!
//! ## Features
//!
//! - **Signature Creation**: Sign PDFs with X.509 certificates
//! - **Signature Verification**: Verify existing PDF signatures
//! - **Certificate Handling**: Parse and validate X.509 certificate chains
//! - **ByteRange Calculation**: Proper handling of PDF byte ranges for signing
//!
//! ## Signature Types Supported
//!
//! - PKCS#7 detached signatures (adbe.pkcs7.detached)
//! - PKCS#7 SHA-1 signatures (adbe.pkcs7.sha1)
//! - PAdES signatures (ETSI.CAdES.detached)
//!
//! ## Example
//!
//! ```ignore
//! use pdf_oxide::api::Pdf;
//! use pdf_oxide::signatures::{SigningCredentials, SignOptions};
//!
//! let mut pdf = Pdf::open("document.pdf")?;
//!
//! // Load signing credentials
//! let credentials = SigningCredentials::from_pkcs12("cert.p12", "password")?;
//!
//! // Sign the document
//! pdf.sign(&credentials, SignOptions::default())?;
//! pdf.save("signed_document.pdf")?;
//! ```
//!
//! ## PDF Specification Reference
//!
//! - ISO 32000-1:2008 Section 12.8 - Digital Signatures
//! - ISO 32000-2:2020 Section 12.8 - Digital Signatures
//! - ETSI TS 102 778 - PAdES
//!
//! Requires the `signatures` feature to be enabled.

mod byterange;
mod enumerate;
mod pdf_date;
mod signer;
#[cfg(feature = "signatures")]
mod timestamp;
#[cfg(all(feature = "signatures", feature = "tsa-client"))]
mod tsa_client;
mod types;
mod verifier;

pub use byterange::ByteRangeCalculator;
pub use enumerate::{count_signatures, enumerate_signatures};
pub use pdf_date::parse_pdf_date_to_epoch;
#[cfg(feature = "signatures")]
pub use timestamp::{HashAlgorithm, Timestamp};
#[cfg(all(feature = "signatures", feature = "tsa-client"))]
pub use tsa_client::{TsaClient, TsaClientConfig};
pub use signer::PdfSigner;
pub use types::{
    DigestAlgorithm, SignOptions, SignatureAppearance, SignatureInfo, SignatureSubFilter,
    SigningCredentials, VerificationResult, VerificationStatus,
};
pub use verifier::SignatureVerifier;
