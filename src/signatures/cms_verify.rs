//! CMS signer verification (RFC 5652) — first cryptographic slice of #77.
//!
//! `verify_signer` parses a PDF signature `/Contents` blob (a CMS
//! `SignedData` structure), then runs the **signer-attributes crypto
//! check** from RFC 5652 §5.4:
//!
//! 1. Re-encode the signer's `signed_attrs` field as a universal
//!    `SET OF Attribute` (the on-wire tag is `IMPLICIT [0]`; the hash
//!    input is the bare SET).
//! 2. Hash that encoding with the OID in `signer.digest_alg`.
//! 3. Verify the signer's signature against the certificate's public
//!    key using RSA-PKCS#1 v1.5.
//!
//! This proves authenticity of the signed attributes (the signer held
//! the private key matching the cert, and those attributes haven't been
//! tampered with). It does **not** yet check the `messageDigest`
//! attribute against the document's byte-range content hash — that
//! step needs the raw PDF bytes plumbed through the caller and is the
//! next slice of #77. Callers that need the full check must also
//! compare the `messageDigest` attribute against their own content
//! hash for now.
//!
//! Supported today: RSA-PKCS#1 v1.5 over SHA-1 / SHA-256 / SHA-384 /
//! SHA-512. RSA-PSS and ECDSA return `SignerVerify::Unknown` until the
//! respective RustCrypto verifiers are wired up.

#![cfg(feature = "signatures")]

use crate::error::{Error, Result};
use cms::cert::CertificateChoices;
use cms::content_info::ContentInfo;
use cms::signed_data::{SignedData, SignerIdentifier, SignerInfo};
use der::oid::db::rfc5912::{ID_SHA_1, ID_SHA_256, ID_SHA_384, ID_SHA_512};
use der::oid::ObjectIdentifier;
use der::{Decode, Encode};
use rsa::pkcs8::DecodePublicKey;
use rsa::{Pkcs1v15Sign, RsaPublicKey};
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha384, Sha512};
use cms::cert::x509::Certificate;

/// Outcome of `verify_signer()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignerVerify {
    /// RSA-PKCS#1 v1.5 check over `signed_attrs` succeeded against
    /// the signer certificate's public key. The `messageDigest`
    /// attribute inside `signed_attrs` must still be compared against
    /// the caller's document hash for a full validity claim.
    Valid,
    /// CMS parsed but the RSA-PKCS#1 v1.5 signature did not match —
    /// tampering or a wrong-key scenario.
    Invalid,
    /// CMS parses and is structurally plausible but we cannot run the
    /// crypto check: RSA-PSS, ECDSA, a non-RSA key, or a digest OID we
    /// do not know how to hash. Callers should treat this as
    /// "unverified" rather than "verified".
    Unknown,
}

// PKCS#1 v1.5 `DigestInfo` prefixes (RFC 8017 §9.2 "EMSA-PKCS1-v1_5").
// Each is the DER encoding of `DigestInfo { digestAlgorithm, OCTET STRING }`
// with an empty OCTET STRING; the hash bytes are appended at the end.
const DIGEST_INFO_SHA1: &[u8] = &[
    0x30, 0x21, 0x30, 0x09, 0x06, 0x05, 0x2b, 0x0e, 0x03, 0x02, 0x1a, 0x05, 0x00, 0x04, 0x14,
];
const DIGEST_INFO_SHA256: &[u8] = &[
    0x30, 0x31, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x01,
    0x05, 0x00, 0x04, 0x20,
];
const DIGEST_INFO_SHA384: &[u8] = &[
    0x30, 0x41, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x02,
    0x05, 0x00, 0x04, 0x30,
];
const DIGEST_INFO_SHA512: &[u8] = &[
    0x30, 0x51, 0x30, 0x0d, 0x06, 0x09, 0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02, 0x03,
    0x05, 0x00, 0x04, 0x40,
];

// rsaEncryption OID (1.2.840.113549.1.1.1) — the key-type OID that
// appears in a certificate's SubjectPublicKeyInfo. Distinct from the
// signatureAlgorithm OID on the SignerInfo, which names the padding +
// hash (e.g. sha256WithRSAEncryption, 1.2.840.113549.1.1.11).
const OID_RSA_ENCRYPTION: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.1");

// PKCS#1 v1.5 signature OIDs — a cert signed with any of these uses
// RSA + PKCS#1 v1.5 padding and the indicated hash. The hash is also
// redundantly named by `signer.digest_alg`, so we drive off that and
// only use this set to recognise "this is an RSA-PKCS1v15 signer".
const OID_SHA1_RSA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.5");
const OID_SHA256_RSA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.11");
const OID_SHA384_RSA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.12");
const OID_SHA512_RSA: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.13");
// rsaEncryption also shows up as a signatureAlgorithm in some PDFs —
// treated as "use the digest from digest_alg".
const OID_RSA_SIG_GENERIC: ObjectIdentifier = OID_RSA_ENCRYPTION;

fn digest_info_prefix(oid: ObjectIdentifier) -> Option<&'static [u8]> {
    if oid == ID_SHA_1 {
        Some(DIGEST_INFO_SHA1)
    } else if oid == ID_SHA_256 {
        Some(DIGEST_INFO_SHA256)
    } else if oid == ID_SHA_384 {
        Some(DIGEST_INFO_SHA384)
    } else if oid == ID_SHA_512 {
        Some(DIGEST_INFO_SHA512)
    } else {
        None
    }
}

fn hash_with_oid(oid: ObjectIdentifier, msg: &[u8]) -> Option<Vec<u8>> {
    if oid == ID_SHA_1 {
        Some(Sha1::digest(msg).to_vec())
    } else if oid == ID_SHA_256 {
        Some(Sha256::digest(msg).to_vec())
    } else if oid == ID_SHA_384 {
        Some(Sha384::digest(msg).to_vec())
    } else if oid == ID_SHA_512 {
        Some(Sha512::digest(msg).to_vec())
    } else {
        None
    }
}

fn is_rsa_pkcs1v15_sig_oid(oid: ObjectIdentifier) -> bool {
    oid == OID_SHA1_RSA
        || oid == OID_SHA256_RSA
        || oid == OID_SHA384_RSA
        || oid == OID_SHA512_RSA
        || oid == OID_RSA_SIG_GENERIC
}

/// Find the certificate whose issuer+serial (or SKI) matches the
/// `SignerInfo.sid`. PDF signatures usually embed exactly one cert and
/// use IssuerAndSerialNumber; SubjectKeyIdentifier is rarer but
/// supported for parity.
fn find_signer_certificate<'a>(
    sd: &'a SignedData,
    signer: &SignerInfo,
) -> Option<&'a Certificate> {
    let certs = sd.certificates.as_ref()?;
    for choice in certs.0.iter() {
        let CertificateChoices::Certificate(cert) = choice else {
            continue;
        };
        match &signer.sid {
            SignerIdentifier::IssuerAndSerialNumber(isn) => {
                if cert.tbs_certificate.issuer == isn.issuer
                    && cert.tbs_certificate.serial_number == isn.serial_number
                {
                    return Some(cert);
                }
            },
            SignerIdentifier::SubjectKeyIdentifier(_) => {
                // PDF signers overwhelmingly use IssuerAndSerialNumber;
                // matching on SKI needs parsing the cert's subjectKeyIdentifier
                // extension. Deferred to a follow-up slice — for now we
                // fall back to the first cert, which is the PDF spec's
                // conventional slot for the signer.
                return Some(cert);
            },
        }
    }
    // Fallback: the first cert. PKCS#7 spec-compliant blobs put the
    // signer first.
    for choice in certs.0.iter() {
        if let CertificateChoices::Certificate(cert) = choice {
            return Some(cert);
        }
    }
    None
}

/// Verify a PDF CMS signature. Returns
/// [`SignerVerify::Valid`]/[`SignerVerify::Invalid`] when RSA-PKCS#1
/// v1.5 can be checked, [`SignerVerify::Unknown`] for structurally
/// valid blobs we cannot yet crypto-verify (PSS, ECDSA, unrecognised
/// digest OIDs), and an error for non-CMS / non-SignedData bytes.
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

    // signed_attrs must be present — PDF /Contents blobs are detached
    // signatures and signing the detached content directly is disallowed
    // by RFC 5652 when attributes are absent for non-Data content types.
    let Some(signed_attrs) = signer.signed_attrs.as_ref() else {
        return Ok(SignerVerify::Unknown);
    };

    let digest_oid = signer.digest_alg.oid;
    let Some(hash) = hash_with_oid(digest_oid, &signed_attrs.to_der().map_err(|e| {
        Error::InvalidPdf(format!("failed to re-encode signed_attrs: {e}"))
    })?) else {
        return Ok(SignerVerify::Unknown);
    };

    let sig_alg_oid = signer.signature_algorithm.oid;
    if !is_rsa_pkcs1v15_sig_oid(sig_alg_oid) {
        // RSA-PSS and ECDSA land here — scaffold for future slices.
        return Ok(SignerVerify::Unknown);
    }

    // Build the PKCS#1 v1.5 DigestInfo (prefix + hash). Passing this
    // through `new_unprefixed()` makes rsa 0.9 compare it directly
    // against the decrypted signature bytes, which sidesteps the
    // `Digest + AssociatedOid` trait-bound mismatch between rsa 0.9's
    // digest 0.10 and our sha2 0.11.
    let Some(prefix) = digest_info_prefix(digest_oid) else {
        return Ok(SignerVerify::Unknown);
    };
    let mut digest_info = Vec::with_capacity(prefix.len() + hash.len());
    digest_info.extend_from_slice(prefix);
    digest_info.extend_from_slice(&hash);

    let Some(cert) = find_signer_certificate(&sd, signer) else {
        return Ok(SignerVerify::Unknown);
    };

    // Only RSA keys can verify PKCS#1 v1.5 signatures.
    if cert.tbs_certificate.subject_public_key_info.algorithm.oid != OID_RSA_ENCRYPTION {
        return Ok(SignerVerify::Unknown);
    }

    let spki_der = cert
        .tbs_certificate
        .subject_public_key_info
        .to_der()
        .map_err(|e| Error::InvalidPdf(format!("failed to re-encode signer SPKI: {e}")))?;
    let pub_key = match RsaPublicKey::from_public_key_der(&spki_der) {
        Ok(k) => k,
        Err(_) => return Ok(SignerVerify::Unknown),
    };

    let sig_bytes = signer.signature.as_bytes();
    match pub_key.verify(Pkcs1v15Sign::new_unprefixed(), &digest_info, sig_bytes) {
        Ok(()) => Ok(SignerVerify::Valid),
        Err(_) => Ok(SignerVerify::Invalid),
    }
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
