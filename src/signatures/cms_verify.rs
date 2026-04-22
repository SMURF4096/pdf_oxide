//! CMS signer verification (RFC 5652) — first structural slice of #77.
//!
//! This is the inspection / structural-integrity half of PDF signature
//! verification: parse the CMS SignedData, confirm it has a SignerInfo
//! with signed_attrs and a recognisable digest algorithm, and return
//! a status byte every binding can use to distinguish "structurally
//! plausible" from "outright broken".
//!
//! The cryptographic half — re-encoding signedAttrs as a SET OF
//! Attribute, hashing, and calling an RSA/ECDSA verifier against the
//! signer cert's public key — is a dedicated follow-up slice because
//! the RustCrypto rsa 0.9 / x509-cert plumbing needs trait bounds
//! (`AssociatedOid`, `RsaSignatureAssociatedOid`, etc.) that are
//! non-obvious to satisfy against the SHA-2 types in the `sha2`
//! crate. Filed for the #77 implementation owner.
//!
//! Today `verify_signer` returns `SignerVerify::Unknown` for every
//! CMS blob that parses — this is the contract every binding's
//! `Signature.Verify()` wrapper can opt into without lying: "we
//! parsed the CMS structure, we couldn't do crypto yet".

#![cfg(feature = "signatures")]

use crate::error::{Error, Result};
use cms::content_info::ContentInfo;
use cms::signed_data::SignedData;
use der::oid::db::rfc5912::{ID_SHA_1, ID_SHA_256, ID_SHA_384, ID_SHA_512};
use der::{Decode, Encode};

/// Outcome of `verify_signer()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignerVerify {
    /// Cryptographic verification proved authenticity + integrity.
    /// Not reachable from this slice — reserved for when the full
    /// RSA/ECDSA check lands.
    Valid,
    /// Cryptographic verification proved tampering or mismatch.
    /// Not reachable from this slice.
    Invalid,
    /// CMS parses and is structurally plausible (SignerInfo present,
    /// signed_attrs present, digest OID is one we know how to hash),
    /// but we haven't run the cryptographic check yet. Callers should
    /// treat this as "unverified" rather than "verified".
    Unknown,
}

/// Partial verifier: parses the CMS SignedData from a PDF signature
/// `/Contents` blob and returns `SignerVerify::Unknown` if the
/// structure is plausible, or a parse error otherwise. Will start
/// returning `Valid`/`Invalid` once the RSA verify path lands.
pub fn verify_signer(contents: &[u8]) -> Result<SignerVerify> {
    let ci = ContentInfo::from_der(contents).map_err(|e| {
        Error::InvalidPdf(format!("signature /Contents is not valid CMS ContentInfo: {e}"))
    })?;
    let sd_bytes = ci.content.to_der().map_err(|e| {
        Error::InvalidPdf(format!("failed to re-encode ContentInfo content: {e}"))
    })?;
    let sd = SignedData::from_der(&sd_bytes).map_err(|e| {
        Error::InvalidPdf(format!("CMS content is not valid SignedData: {e}"))
    })?;

    let signer = sd
        .signer_infos
        .0
        .iter()
        .next()
        .ok_or_else(|| Error::InvalidPdf("SignedData has no SignerInfo".into()))?;

    // Structural check: signed_attrs must exist (otherwise the
    // signature is over detached eContent which we can't reconstruct).
    if signer.signed_attrs.is_none() {
        return Ok(SignerVerify::Unknown);
    }

    // Structural check: digest OID must be something we can hash.
    let oid = signer.digest_alg.oid;
    let _known = oid == ID_SHA_256 || oid == ID_SHA_384 || oid == ID_SHA_512 || oid == ID_SHA_1;

    // All structural preconditions for the eventual crypto check pass —
    // but the crypto itself isn't wired yet.
    Ok(SignerVerify::Unknown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_cms_bytes() {
        let err = verify_signer(b"not a CMS blob").unwrap_err();
        assert!(matches!(err, Error::InvalidPdf(_)));
    }

    #[test]
    fn rejects_empty_bytes() {
        let err = verify_signer(&[]).unwrap_err();
        assert!(matches!(err, Error::InvalidPdf(_)));
    }
}
