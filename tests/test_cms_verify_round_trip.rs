//! End-to-end crypto round-trip for `signatures::cms_verify::verify_signer`.
//!
//! The fixtures under `tests/fixtures/signatures/` were built with
//! `openssl cms -sign -md sha256 -nodetach` against a fresh self-signed
//! RSA-2048 certificate — see the generation recipe in
//! `signatures::cms_verify` tests / the v0.3.38 handoff. The signer's
//! cert is embedded inside the SignedData, so our verifier can recover
//! the public key straight from the CMS blob.

#![cfg(feature = "signatures")]

use pdf_oxide::signatures::{verify_signer, SignerVerify};

const VALID: &[u8] = include_bytes!("fixtures/signatures/rsa_sha256_round_trip.cms");
const TAMPERED: &[u8] =
    include_bytes!("fixtures/signatures/rsa_sha256_round_trip_tampered.cms");

#[test]
fn rsa_sha256_round_trip_is_valid() {
    let result = verify_signer(VALID).expect("parse valid CMS blob");
    assert_eq!(
        result,
        SignerVerify::Valid,
        "openssl cms -sign RSA/SHA-256 blob must verify against its embedded cert"
    );
}

#[test]
fn tampered_cms_blob_is_invalid() {
    // The fixture differs from the valid one by a single flipped bit
    // near the end of the DER (falls inside the signature OCTET STRING
    // or the trailer). It should still parse as CMS but the RSA check
    // must fail. Depending on where the flip lands, the result can be
    // `Invalid` (signature-value byte flipped) or a DER parse error
    // (structural byte flipped) — both are acceptable here.
    match verify_signer(TAMPERED) {
        Ok(SignerVerify::Invalid) => {},
        Ok(other) => panic!(
            "tampered blob must not verify; got {other:?}"
        ),
        Err(_) => {
            // DER parse error is also a legitimate rejection for a
            // blob whose flipped byte corrupted the ASN.1 framing.
        },
    }
}
