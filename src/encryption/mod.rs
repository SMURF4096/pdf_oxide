//! PDF encryption support.
//!
//! This module implements PDF encryption/decryption according to the PDF specification
//! (ISO 32000-1:2008, Section 7.6). It supports:
//!
//! - RC4 encryption (40-bit and 128-bit) for PDF 1.4-1.5
//! - AES encryption (128-bit and 256-bit) for PDF 1.6+
//! - Standard Security Handler (password validation, permissions)
//!
//! # Encryption Algorithms
//!
//! ## RC4 (PDF 1.4-1.5)
//! - RC4-40: 40-bit key length (weak, legacy)
//! - RC4-128: 128-bit key length
//!
//! ## AES (PDF 1.6+)
//! - AES-128: 128-bit key length with CBC mode
//! - AES-256: 256-bit key length with CBC mode (PDF 2.0)
//!
//! # Security Considerations
//!
//! - RC4-40 is cryptographically weak and should only be used for legacy documents
//! - Password validation uses constant-time comparison to prevent timing attacks
//! - Key derivation follows PDF specification algorithms (using MD5 or SHA-256)
//!
//! # References
//!
//! - PDF Spec Section 7.6: Encryption
//! - PDF Spec Section 7.6.3: Standard Security Handler
//! - PDF Spec Section 7.6.5: Algorithm 2 (Key Derivation)

use crate::error::{Error, Result};
use crate::object::Object;

mod aes;
mod algorithms;
mod certificate;
mod handler;
mod rc4;
mod write_handler;

pub use algorithms::{
    compute_encryption_key, compute_owner_password_hash, compute_user_password_hash,
};
pub use certificate::{
    CertEncryptDict, CertSubFilter, CertificateEncryption, CertificateEncryptionHandler,
    KeyTransportAlgorithm, RecipientInfo, RecipientPermissions,
};
pub use handler::EncryptionHandler;
pub use write_handler::EncryptionWriteHandler;

/// Encryption algorithm used in the PDF.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Algorithm {
    /// No encryption
    None,
    /// RC4 with 40-bit key (PDF 1.4, V=1, R=2)
    RC4_40,
    /// RC4 with 128-bit key (PDF 1.5, V=2, R=3)
    Rc4_128,
    /// AES with 128-bit key in CBC mode (PDF 1.6, V=4, R=4)
    Aes128,
    /// AES with 256-bit key in CBC mode (PDF 2.0, V=5, R=5/6)
    Aes256,
}

impl Algorithm {
    /// Get the key length in bytes for this algorithm.
    pub fn key_length(&self) -> usize {
        match self {
            Algorithm::None => 0,
            Algorithm::RC4_40 => 5,   // 40 bits = 5 bytes
            Algorithm::Rc4_128 => 16, // 128 bits = 16 bytes
            Algorithm::Aes128 => 16,  // 128 bits = 16 bytes
            Algorithm::Aes256 => 32,  // 256 bits = 32 bytes
        }
    }

    /// Check if this is an AES algorithm.
    pub fn is_aes(&self) -> bool {
        matches!(self, Algorithm::Aes128 | Algorithm::Aes256)
    }

    /// Check if this is an RC4 algorithm.
    pub fn is_rc4(&self) -> bool {
        matches!(self, Algorithm::RC4_40 | Algorithm::Rc4_128)
    }
}

/// PDF encryption dictionary (/Encrypt entry in trailer).
///
/// PDF Spec: Section 7.6.1 - General
#[derive(Debug, Clone)]
pub struct EncryptDict {
    /// Filter name (should be "Standard")
    pub filter: String,
    /// SubFilter name (optional, for public-key security)
    pub sub_filter: Option<String>,
    /// Algorithm version (V): 1=RC4-40, 2=RC4-128, 4=AES-128, 5=AES-256
    pub version: u32,
    /// Key length in bits (Length): 40-128 for RC4, 128/256 for AES
    pub length: Option<u32>,
    /// Revision number (R): 2, 3, 4, 5, or 6
    pub revision: u32,
    /// Owner password hash (O): 32 or 48 bytes
    pub owner_password: Vec<u8>,
    /// User password hash (U): 32 or 48 bytes
    pub user_password: Vec<u8>,
    /// User permissions (P): 32-bit integer
    pub permissions: i32,
    /// Encrypt metadata flag (EncryptMetadata): true by default
    pub encrypt_metadata: bool,
    /// Additional encryption parameters (for V=5/R=6)
    pub owner_encryption: Option<Vec<u8>>, // OE
    /// User encryption key (UE entry, for V=5/R=6)
    pub user_encryption: Option<Vec<u8>>, // UE
    /// Encrypted permissions (Perms entry, for V=5/R=6)
    pub perms: Option<Vec<u8>>, // Perms (encrypted permissions)
}

impl EncryptDict {
    /// Parse an encryption dictionary from a PDF object.
    ///
    /// PDF Spec: Section 7.6.1 - General
    pub fn from_object(obj: &Object) -> Result<Self> {
        let dict = obj
            .as_dict()
            .ok_or_else(|| Error::InvalidPdf("Encrypt entry is not a dictionary".to_string()))?;

        // Extract required fields
        let filter = dict
            .get("Filter")
            .and_then(|o| o.as_name())
            .ok_or_else(|| Error::InvalidPdf("Encrypt dictionary missing /Filter".to_string()))?
            .to_string();

        let version = dict
            .get("V")
            .and_then(|o| match o {
                Object::Integer(i) => Some(*i as u32),
                _ => None,
            })
            .ok_or_else(|| Error::InvalidPdf("Encrypt dictionary missing /V".to_string()))?;

        let revision = dict
            .get("R")
            .and_then(|o| match o {
                Object::Integer(i) => Some(*i as u32),
                _ => None,
            })
            .ok_or_else(|| Error::InvalidPdf("Encrypt dictionary missing /R".to_string()))?;

        let owner_password = dict
            .get("O")
            .and_then(|o| o.as_string())
            .ok_or_else(|| Error::InvalidPdf("Encrypt dictionary missing /O".to_string()))?
            .to_vec();

        let user_password = dict
            .get("U")
            .and_then(|o| o.as_string())
            .ok_or_else(|| Error::InvalidPdf("Encrypt dictionary missing /U".to_string()))?
            .to_vec();

        let permissions = dict
            .get("P")
            .and_then(|o| match o {
                Object::Integer(i) => Some(*i as i32),
                _ => None,
            })
            .ok_or_else(|| Error::InvalidPdf("Encrypt dictionary missing /P".to_string()))?;

        // Optional fields
        let sub_filter = dict
            .get("SubFilter")
            .and_then(|o| o.as_name())
            .map(|s| s.to_string());

        let length = dict.get("Length").and_then(|o| match o {
            Object::Integer(i) => Some(*i as u32),
            _ => None,
        });

        let encrypt_metadata = dict
            .get("EncryptMetadata")
            .and_then(|o| match o {
                Object::Boolean(b) => Some(*b),
                _ => None,
            })
            .unwrap_or(true);

        // V=5/R=6 additional fields
        let owner_encryption = dict
            .get("OE")
            .and_then(|o| o.as_string())
            .map(|s| s.to_vec());

        let user_encryption = dict
            .get("UE")
            .and_then(|o| o.as_string())
            .map(|s| s.to_vec());

        let perms = dict
            .get("Perms")
            .and_then(|o| o.as_string())
            .map(|s| s.to_vec());

        Ok(EncryptDict {
            filter,
            sub_filter,
            version,
            length,
            revision,
            owner_password,
            user_password,
            permissions,
            encrypt_metadata,
            owner_encryption,
            user_encryption,
            perms,
        })
    }

    /// Determine the encryption algorithm from V and R values.
    ///
    /// PDF Spec: Table 20 - Encryption dictionary entries
    pub fn algorithm(&self) -> Result<Algorithm> {
        match (self.version, self.revision) {
            (1, 2) => Ok(Algorithm::RC4_40),
            (2, 3) => Ok(Algorithm::Rc4_128),
            (4, 4) => Ok(Algorithm::Aes128),
            (5, 5) | (5, 6) => Ok(Algorithm::Aes256),
            // Lenient: V determines algorithm, R may be non-standard
            (1, r) => {
                log::warn!("Non-standard encryption V=1, R={}, using RC4-40", r);
                Ok(Algorithm::RC4_40)
            },
            (2, r) => {
                log::warn!("Non-standard encryption V=2, R={}, using RC4-128", r);
                Ok(Algorithm::Rc4_128)
            },
            (4, r) => {
                log::warn!("Non-standard encryption V=4, R={}, using AES-128", r);
                Ok(Algorithm::Aes128)
            },
            _ => Err(Error::Unsupported(format!(
                "Unsupported encryption version V={}, R={}",
                self.version, self.revision
            ))),
        }
    }

    /// Get the effective key length in bytes.
    pub fn key_length_bytes(&self) -> usize {
        if let Some(length) = self.length {
            (length / 8) as usize
        } else {
            // Default key lengths based on version
            match self.version {
                1 => 5,  // 40 bits
                2 => 16, // 128 bits
                4 => 16, // 128 bits (AES-128)
                5 => 32, // 256 bits (AES-256)
                _ => 16, // Default to 128 bits
            }
        }
    }

    /// Serialize the encryption dictionary to a PDF Object.
    ///
    /// This creates a dictionary object suitable for the /Encrypt entry in the trailer.
    pub fn to_object(&self) -> Object {
        use std::collections::HashMap;

        let mut dict: HashMap<String, Object> = HashMap::new();

        // Required entries
        dict.insert("Filter".to_string(), Object::Name(self.filter.clone()));
        dict.insert("V".to_string(), Object::Integer(self.version as i64));
        dict.insert("R".to_string(), Object::Integer(self.revision as i64));
        dict.insert("O".to_string(), Object::String(self.owner_password.clone()));
        dict.insert("U".to_string(), Object::String(self.user_password.clone()));
        dict.insert("P".to_string(), Object::Integer(self.permissions as i64));

        // Optional entries
        if let Some(ref sub_filter) = self.sub_filter {
            dict.insert("SubFilter".to_string(), Object::Name(sub_filter.clone()));
        }

        if let Some(length) = self.length {
            dict.insert("Length".to_string(), Object::Integer(length as i64));
        }

        if !self.encrypt_metadata {
            dict.insert("EncryptMetadata".to_string(), Object::Boolean(false));
        }

        // V=5/R=6 specific entries
        if let Some(ref oe) = self.owner_encryption {
            dict.insert("OE".to_string(), Object::String(oe.clone()));
        }

        if let Some(ref ue) = self.user_encryption {
            dict.insert("UE".to_string(), Object::String(ue.clone()));
        }

        if let Some(ref perms) = self.perms {
            dict.insert("Perms".to_string(), Object::String(perms.clone()));
        }

        // For V=4 (AES-128), add crypt filter entries
        if self.version == 4 {
            // Standard crypt filter dictionary
            let mut cf_dict: HashMap<String, Object> = HashMap::new();
            let mut std_cf: HashMap<String, Object> = HashMap::new();
            std_cf.insert("CFM".to_string(), Object::Name("AESV2".to_string()));
            std_cf.insert("AuthEvent".to_string(), Object::Name("DocOpen".to_string()));
            std_cf.insert("Length".to_string(), Object::Integer(16));
            cf_dict.insert("StdCF".to_string(), Object::Dictionary(std_cf));
            dict.insert("CF".to_string(), Object::Dictionary(cf_dict));
            dict.insert("StmF".to_string(), Object::Name("StdCF".to_string()));
            dict.insert("StrF".to_string(), Object::Name("StdCF".to_string()));
        }

        // For V=5 (AES-256), add crypt filter entries
        if self.version == 5 {
            let mut cf_dict: HashMap<String, Object> = HashMap::new();
            let mut std_cf: HashMap<String, Object> = HashMap::new();
            std_cf.insert("CFM".to_string(), Object::Name("AESV3".to_string()));
            std_cf.insert("AuthEvent".to_string(), Object::Name("DocOpen".to_string()));
            std_cf.insert("Length".to_string(), Object::Integer(32));
            cf_dict.insert("StdCF".to_string(), Object::Dictionary(std_cf));
            dict.insert("CF".to_string(), Object::Dictionary(cf_dict));
            dict.insert("StmF".to_string(), Object::Name("StdCF".to_string()));
            dict.insert("StrF".to_string(), Object::Name("StdCF".to_string()));
        }

        Object::Dictionary(dict)
    }
}

/// Builder for creating encryption dictionaries.
///
/// This provides a convenient way to create properly configured encryption
/// for writing encrypted PDFs.
pub struct EncryptDictBuilder {
    algorithm: Algorithm,
    user_password: Vec<u8>,
    owner_password: Vec<u8>,
    permissions: i32,
    encrypt_metadata: bool,
}

impl EncryptDictBuilder {
    /// Create a new builder with the specified algorithm.
    pub fn new(algorithm: Algorithm) -> Self {
        Self {
            algorithm,
            user_password: Vec::new(),
            owner_password: Vec::new(),
            permissions: -1, // All permissions granted by default
            encrypt_metadata: true,
        }
    }

    /// Set the user password (required for opening the document).
    pub fn user_password(mut self, password: &[u8]) -> Self {
        self.user_password = password.to_vec();
        self
    }

    /// Set the owner password (required for full access).
    pub fn owner_password(mut self, password: &[u8]) -> Self {
        self.owner_password = password.to_vec();
        self
    }

    /// Set user permissions (P value).
    pub fn permissions(mut self, permissions: i32) -> Self {
        self.permissions = permissions;
        self
    }

    /// Set whether to encrypt metadata.
    pub fn encrypt_metadata(mut self, encrypt: bool) -> Self {
        self.encrypt_metadata = encrypt;
        self
    }

    /// Build the encryption dictionary.
    ///
    /// This computes all required hashes and returns the complete dictionary.
    ///
    /// # Arguments
    /// * `file_id` - The first element of the PDF file identifier array
    pub fn build(self, file_id: &[u8]) -> EncryptDict {
        let (version, revision) = match self.algorithm {
            Algorithm::None => (0, 0),
            Algorithm::RC4_40 => (1, 2),
            Algorithm::Rc4_128 => (2, 3),
            Algorithm::Aes128 => (4, 4),
            Algorithm::Aes256 => (5, 6),
        };

        let key_length = self.algorithm.key_length();

        // Use owner password if provided, otherwise use user password
        let owner_pass = if self.owner_password.is_empty() {
            &self.user_password
        } else {
            &self.owner_password
        };

        // Compute owner password hash (O value)
        let owner_hash = algorithms::compute_owner_password_hash(
            owner_pass,
            &self.user_password,
            revision,
            key_length,
        );

        // Compute encryption key from user password
        let encryption_key = algorithms::compute_encryption_key(
            &self.user_password,
            &owner_hash,
            self.permissions,
            file_id,
            revision,
            key_length,
            self.encrypt_metadata,
        );

        // Compute user password hash (U value)
        let user_hash = algorithms::compute_user_password_hash(&encryption_key, file_id, revision);

        EncryptDict {
            filter: "Standard".to_string(),
            sub_filter: None,
            version,
            length: Some((key_length * 8) as u32),
            revision,
            owner_password: owner_hash,
            user_password: user_hash,
            permissions: self.permissions,
            encrypt_metadata: self.encrypt_metadata,
            owner_encryption: None,
            user_encryption: None,
            perms: None,
        }
    }
}

/// PDF encryption permissions (P field).
///
/// PDF Spec: Table 22 - User access permissions
#[derive(Debug, Clone, Copy)]
pub struct Permissions {
    bits: i32,
}

impl Permissions {
    /// Create permissions from the P field value.
    pub fn from_bits(bits: i32) -> Self {
        Self { bits }
    }

    /// Check if printing is allowed.
    pub fn can_print(&self) -> bool {
        (self.bits & (1 << 2)) != 0
    }

    /// Check if modifying the document is allowed.
    pub fn can_modify(&self) -> bool {
        (self.bits & (1 << 3)) != 0
    }

    /// Check if copying text/graphics is allowed.
    pub fn can_copy(&self) -> bool {
        (self.bits & (1 << 4)) != 0
    }

    /// Check if adding/modifying annotations is allowed.
    pub fn can_annotate(&self) -> bool {
        (self.bits & (1 << 5)) != 0
    }

    /// Check if filling form fields is allowed (R>=3).
    pub fn can_fill_forms(&self) -> bool {
        (self.bits & (1 << 8)) != 0
    }

    /// Check if content extraction for accessibility is allowed (R>=3).
    pub fn can_extract_accessibility(&self) -> bool {
        (self.bits & (1 << 9)) != 0
    }

    /// Check if assembling the document is allowed (R>=3).
    pub fn can_assemble(&self) -> bool {
        (self.bits & (1 << 10)) != 0
    }

    /// Check if high-quality printing is allowed (R>=3).
    pub fn can_print_high_quality(&self) -> bool {
        (self.bits & (1 << 11)) != 0
    }
}

/// Generate a unique file ID for the PDF.
///
/// PDF Spec: Section 14.4 - File Identifiers
///
/// The file identifier array contains two strings:
/// - First string: A permanent identifier based on file contents at creation
/// - Second string: A changing identifier updated each time the file is saved
///
/// This function generates both strings as the same value (for new PDFs).
/// For incremental updates, the first ID should be preserved.
///
/// # Returns
///
/// A tuple of (permanent_id, changing_id) as 16-byte vectors
pub fn generate_file_id() -> (Vec<u8>, Vec<u8>) {
    use md5::{Digest, Md5};

    // Generate a UUID v4 and hash it with MD5 to get 16 bytes
    let uuid = uuid::Uuid::new_v4();
    let uuid_bytes = uuid.as_bytes();

    let mut hasher = Md5::new();
    hasher.update(uuid_bytes);

    // Add current time for extra uniqueness
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    hasher.update(now.as_nanos().to_le_bytes());

    let id = hasher.finalize().to_vec();

    // For new PDFs, both IDs are the same
    (id.clone(), id)
}
