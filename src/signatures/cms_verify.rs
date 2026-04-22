//! CMS signer verification (RFC 5652) — cryptographic path for #77.
//!
//! Two entry points, layered:
//!
//! - [`verify_signer`] runs the **signer-attributes crypto check**
//!   (RFC 5652 §5.4): it confirms that the signer held the private key
//!   matching the certificate embedded in the blob and signed the
//!   `signed_attrs` bundle. This proves authenticity of the attributes
//!   but *does not* prove that those attributes describe the document
//!   the caller has in hand.
//!
//! - [`verify_signer_detached`] layers the content-integrity check on
//!   top: it hashes the caller's detached content with the digest OID
//!   named in the CMS, then confirms that hash matches the signed
//!   `messageDigest` attribute (RFC 5652 §11.2). A detached PDF
//!   signature is only fully valid when both checks succeed; callers
//!   should prefer this entry point whenever they have the raw PDF
//!   byte-range available.
//!
//! Supported today: RSA-PKCS#1 v1.5 over SHA-1 / SHA-256 / SHA-384 /
//! SHA-512. RSA-PSS and ECDSA return [`SignerVerify::Unknown`] until
//! the respective RustCrypto verifiers are wired up.

#![cfg(feature = "signatures")]

use crate::error::{Error, Result};
use cms::cert::x509::Certificate;
use cms::cert::CertificateChoices;
use cms::content_info::ContentInfo;
use cms::signed_data::{SignedData, SignerIdentifier, SignerInfo};
use der::asn1::OctetString;
use der::oid::db::rfc5912::{ID_SHA_1, ID_SHA_256, ID_SHA_384, ID_SHA_512};
use der::oid::ObjectIdentifier;
use der::{Decode, Encode};
use rsa::pkcs8::DecodePublicKey;
use rsa::{Pkcs1v15Sign, RsaPublicKey};
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha384, Sha512};

/// Outcome of a `verify_signer*` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignerVerify {
    /// Every check we ran succeeded:
    /// - RSA-PKCS#1 v1.5 over `signed_attrs` matches the signer cert's
    ///   public key.
    /// - For [`verify_signer_detached`], additionally the `messageDigest`
    ///   signed attribute equals the hash of the caller's content.
    ///
    /// For the plain [`verify_signer`] entry point this means only the
    /// attribute bundle is authentic; callers relying on this must
    /// still compare `messageDigest` against their document byte-range
    /// hash themselves.
    Valid,
    /// CMS parsed, but a crypto check failed — tampering, wrong key,
    /// or wrong content. For [`verify_signer_detached`] this includes
    /// the "signer crypto is fine but messageDigest doesn't match the
    /// caller's content" case, which is the interesting one for PDFs
    /// (document bytes were altered after signing).
    Invalid,
    /// CMS parses and is structurally plausible but we cannot run the
    /// crypto check: RSA-PSS, ECDSA, a non-RSA key, an unrecognised
    /// digest OID, or a missing `messageDigest` attribute when one was
    /// required. Callers should treat this as "unverified".
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

// id-messageDigest (RFC 5652 §11.2) — the signed attribute that
// carries hash(content).
const OID_MESSAGE_DIGEST: ObjectIdentifier =
    ObjectIdentifier::new_unwrap("1.2.840.113549.1.9.4");

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

fn parse_signed_data(contents: &[u8]) -> Result<SignedData> {
    let ci = ContentInfo::from_der(contents).map_err(|e| {
        Error::InvalidPdf(format!("signature /Contents is not valid CMS ContentInfo: {e}"))
    })?;
    let sd_bytes = ci.content.to_der().map_err(|e| {
        Error::InvalidPdf(format!("failed to re-encode ContentInfo content: {e}"))
    })?;
    SignedData::from_der(&sd_bytes).map_err(|e| {
        Error::InvalidPdf(format!("CMS content is not valid SignedData: {e}"))
    })
}

/// Run the RSA-PKCS#1 v1.5 signer-attributes crypto check. Returns
/// the outcome plus the `SignerInfo`'s digest OID if the call reached
/// the attribute-hashing stage — callers that layer a `messageDigest`
/// check on top need the same digest OID to hash their content.
fn run_signer_crypto(sd: &SignedData) -> Result<(SignerVerify, Option<ObjectIdentifier>)> {
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
        return Ok((SignerVerify::Unknown, None));
    };

    let digest_oid = signer.digest_alg.oid;
    let Some(hash) = hash_with_oid(
        digest_oid,
        &signed_attrs.to_der().map_err(|e| {
            Error::InvalidPdf(format!("failed to re-encode signed_attrs: {e}"))
        })?,
    ) else {
        return Ok((SignerVerify::Unknown, Some(digest_oid)));
    };

    let sig_alg_oid = signer.signature_algorithm.oid;
    if !is_rsa_pkcs1v15_sig_oid(sig_alg_oid) {
        // RSA-PSS and ECDSA land here — scaffold for future slices.
        return Ok((SignerVerify::Unknown, Some(digest_oid)));
    }

    // Build the PKCS#1 v1.5 DigestInfo (prefix + hash). Passing this
    // through `new_unprefixed()` makes rsa 0.9 compare it directly
    // against the decrypted signature bytes, which sidesteps the
    // `Digest + AssociatedOid` trait-bound mismatch between rsa 0.9's
    // digest 0.10 and our sha2 0.11.
    let Some(prefix) = digest_info_prefix(digest_oid) else {
        return Ok((SignerVerify::Unknown, Some(digest_oid)));
    };
    let mut digest_info = Vec::with_capacity(prefix.len() + hash.len());
    digest_info.extend_from_slice(prefix);
    digest_info.extend_from_slice(&hash);

    let Some(cert) = find_signer_certificate(sd, signer) else {
        return Ok((SignerVerify::Unknown, Some(digest_oid)));
    };

    // Only RSA keys can verify PKCS#1 v1.5 signatures.
    if cert.tbs_certificate.subject_public_key_info.algorithm.oid != OID_RSA_ENCRYPTION {
        return Ok((SignerVerify::Unknown, Some(digest_oid)));
    }

    let spki_der = cert
        .tbs_certificate
        .subject_public_key_info
        .to_der()
        .map_err(|e| Error::InvalidPdf(format!("failed to re-encode signer SPKI: {e}")))?;
    let pub_key = match RsaPublicKey::from_public_key_der(&spki_der) {
        Ok(k) => k,
        Err(_) => return Ok((SignerVerify::Unknown, Some(digest_oid))),
    };

    let sig_bytes = signer.signature.as_bytes();
    let outcome = match pub_key.verify(Pkcs1v15Sign::new_unprefixed(), &digest_info, sig_bytes) {
        Ok(()) => SignerVerify::Valid,
        Err(_) => SignerVerify::Invalid,
    };
    Ok((outcome, Some(digest_oid)))
}

/// Extract the single-value `messageDigest` signed attribute from the
/// first `SignerInfo`. Returns `None` if the attribute is absent or
/// its value does not parse as an `OCTET STRING` — both cases are
/// disqualifying for a detached-content check.
fn extract_message_digest(sd: &SignedData) -> Option<Vec<u8>> {
    let signer = sd.signer_infos.0.iter().next()?;
    let signed_attrs = signer.signed_attrs.as_ref()?;
    for attr in signed_attrs.iter() {
        if attr.oid != OID_MESSAGE_DIGEST {
            continue;
        }
        let value = attr.values.iter().next()?;
        let value_der = value.to_der().ok()?;
        let octet = OctetString::from_der(&value_der).ok()?;
        return Some(octet.as_bytes().to_vec());
    }
    None
}

/// Verify only the signer-attribute RSA-PKCS#1 v1.5 signature of a CMS
/// blob. Use [`verify_signer_detached`] when you also have the
/// document bytes — a `Valid` result from this function only proves
/// the attributes are authentic, not that they describe your document.
pub fn verify_signer(contents: &[u8]) -> Result<SignerVerify> {
    let sd = parse_signed_data(contents)?;
    Ok(run_signer_crypto(&sd)?.0)
}

/// Verify a detached-content PDF signature end-to-end: the
/// signer-attribute crypto check **and** the `messageDigest` signed
/// attribute against `hash(content)` using the digest OID named in
/// the CMS blob.
///
/// `content` should be the exact bytes that were signed — for a PDF
/// this is the concatenation of the two byte-range segments around
/// `/Contents`, which [`crate::signatures::ByteRangeCalculator::extract_signed_bytes`]
/// will assemble for you.
///
/// Returns:
/// - [`SignerVerify::Valid`] — both the RSA check and the messageDigest
///   check pass.
/// - [`SignerVerify::Invalid`] — one of the two crypto checks failed.
///   Callers can't distinguish "wrong signer" from "tampered
///   content" from this enum alone; surface both possibilities to
///   the user.
/// - [`SignerVerify::Unknown`] — the signer crypto path could not run
///   (PSS, ECDSA, unknown digest) or the CMS blob lacks a
///   `messageDigest` attribute in the first place.
pub fn verify_signer_detached(contents: &[u8], content: &[u8]) -> Result<SignerVerify> {
    let sd = parse_signed_data(contents)?;
    let (crypto_outcome, digest_oid) = run_signer_crypto(&sd)?;

    // If the signer-attr crypto failed or was skipped, the
    // detached-content check can't lift the verdict to Valid.
    match crypto_outcome {
        SignerVerify::Valid => {},
        other => return Ok(other),
    }

    // Unwrap is safe here: crypto_outcome == Valid only reaches this
    // point when run_signer_crypto got far enough to know the digest.
    let digest_oid = digest_oid.expect("Valid outcome implies known digest OID");

    let Some(expected) = extract_message_digest(&sd) else {
        // Signer-attributes are authentic but the blob doesn't bind
        // them to any content hash — we can't call the signature
        // fully valid against the caller's document.
        return Ok(SignerVerify::Unknown);
    };
    let Some(actual) = hash_with_oid(digest_oid, content) else {
        return Ok(SignerVerify::Unknown);
    };

    // Constant-time equality isn't strictly necessary for a non-secret
    // hash comparison, but it costs nothing to use it.
    if actual.len() == expected.len()
        && actual.iter().zip(expected.iter()).fold(0u8, |acc, (a, b)| acc | (a ^ b)) == 0
    {
        Ok(SignerVerify::Valid)
    } else {
        Ok(SignerVerify::Invalid)
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

    #[test]
    fn detached_rejects_non_cms_bytes() {
        let err = verify_signer_detached(b"not a CMS blob", b"content").unwrap_err();
        assert!(matches!(err, Error::InvalidPdf(_)));
    }
}
