
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use argon2::{Algorithm, Argon2, Params, Version};
use chrono::{Local, NaiveDate, Utc};
use data_encoding::BASE32_NOPAD;
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;
use thiserror::Error;
use uuid::Uuid;
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::SimpleFileOptions};

const MAGIC_V1: &[u8; 8] = b"PRVORA01";
const MAGIC_V2: &[u8; 8] = b"PRVORA02";
const MAX_HEADER_LEN: usize = 4 * 1024 * 1024;
const CHUNK_SIZE: usize = 1024 * 1024;
const TAG_LEN: usize = 16;
const MAX_ARCHIVE_BYTES: u64 = 64 * 1024 * 1024 * 1024;
const MAX_FILES: usize = 10_000;

#[derive(Debug, Error)]
pub enum VoraError {
    #[error("File operation failed: {0}")]
    Io(#[from] io::Error),
    #[error("Invalid package manifest: {0}")]
    Json(#[from] serde_json::Error),
    #[error("ZIP archive error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error(
        "Encryption or integrity check failed. Verify the license code and ensure the .vora file was not modified."
    )]
    Crypto,
    #[error("This is not a valid Privora .vora package.")]
    BadMagic,
    #[error("Unsupported .vora version.")]
    UnsupportedVersion,
    #[error("The license code is invalid or malformed.")]
    InvalidCode,
    #[error("The license expired on {0}.")]
    Expired(String),
    #[error(
        "The activation limit ({limit}) was reached on this installation. This offline limit is not global across devices."
    )]
    ActivationLimit { limit: u32 },
    #[error("Invalid package: {0}")]
    InvalidPackage(String),
    #[error("Key derivation failed.")]
    KeyDerivation,
    #[error("The operating system random-number generator failed.")]
    Randomness,
    #[error("Refusing to extract an unsafe path from the package.")]
    UnsafePath,
}

pub type Result<T> = std::result::Result<T, VoraError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub format_version: u32,
    pub package_id: String,
    pub name: String,
    pub created_at: String,
    pub expires_on: Option<String>,
    pub max_activations: u32,
    pub kdf: String,
    pub cipher: String,
    pub salt_hex: String,
    pub nonce_prefix_hex: String,
    pub plaintext_len: u64,
    pub chunk_size: u32,
    pub file_count: usize,
    pub total_file_bytes: u64,
}

#[derive(Debug, Clone)]
pub struct CreatedPackage {
    pub manifest: Manifest,
    pub access_code: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub format_version: u32,
    pub fingerprint: String,
    pub metadata_encrypted: bool,
}

#[derive(Debug, Clone)]
pub struct UnlockedPackage {
    pub manifest: Manifest,
    pub output_dir: PathBuf,
    pub written_files: Vec<PathBuf>,
    pub package_fingerprint: String,
}

struct SourceFile {
    path: PathBuf,
    archive_name: String,
    size: u64,
}

/// Create a `.vora` package from one or more source files.
/// The secret access code is returned only to the caller; it is not stored in the package.
pub fn create_package(
    input_paths: &[PathBuf],
    output_path: &Path,
    package_name: &str,
    expires_on: Option<NaiveDate>,
    max_activations: u32,
) -> Result<CreatedPackage> {
    if input_paths.is_empty() {
        return Err(VoraError::InvalidPackage(
            "Select at least one file.".into(),
        ));
    }
    if input_paths.len() > MAX_FILES {
        return Err(VoraError::InvalidPackage(format!(
            "A maximum of {MAX_FILES} files is supported."
        )));
    }
    let package_name = package_name.trim();
    if package_name.is_empty() || package_name.len() > 160 {
        return Err(VoraError::InvalidPackage(
            "Package name must contain 1–160 characters.".into(),
        ));
    }
    if max_activations == 0 || max_activations > 100_000 {
        return Err(VoraError::InvalidPackage(
            "Activation limit must be between 1 and 100,000.".into(),
        ));
    }

    let source_files = prepare_source_files(input_paths)?;
    let total_file_bytes = source_files
        .iter()
        .try_fold(0u64, |total, file| total.checked_add(file.size))
        .ok_or_else(|| {
            VoraError::InvalidPackage("Total input size overflowed the supported range.".into())
        })?;
    if total_file_bytes > MAX_ARCHIVE_BYTES {
        return Err(VoraError::InvalidPackage(
            "Combined input files exceed the 64 GiB safety limit.".into(),
        ));
    }
    let archive_tmp = NamedTempFile::new()?;
    create_zip(&source_files, archive_tmp.path())?;
    let plaintext_len = fs::metadata(archive_tmp.path())?.len();
    if plaintext_len == 0 || plaintext_len > MAX_ARCHIVE_BYTES {
        return Err(VoraError::InvalidPackage(
            "The resulting archive is empty or larger than 64 GiB.".into(),
        ));
    }

    let mut secret = [0u8; 32];
    let mut salt = [0u8; 16];
    let mut nonce_prefix = [0u8; 8];
    let mut metadata_nonce = [0u8; 12];
    getrandom::fill(&mut secret).map_err(|_| VoraError::Randomness)?;
    getrandom::fill(&mut salt).map_err(|_| VoraError::Randomness)?;
    getrandom::fill(&mut nonce_prefix).map_err(|_| VoraError::Randomness)?;
    getrandom::fill(&mut metadata_nonce).map_err(|_| VoraError::Randomness)?;

    let manifest = Manifest {
        format_version: 2,
        package_id: Uuid::new_v4().to_string(),
        name: package_name.to_owned(),
        created_at: Utc::now().to_rfc3339(),
        expires_on: expires_on.map(|d| d.format("%Y-%m-%d").to_string()),
        max_activations,
        kdf: "Argon2id v1.3 (32 MiB, 3 iterations) + HKDF-SHA256".into(),
        cipher: "AES-256-GCM (encrypted manifest + 1 MiB authenticated chunks)".into(),
        salt_hex: hex::encode(salt),
        nonce_prefix_hex: hex::encode(nonce_prefix),
        plaintext_len,
        chunk_size: CHUNK_SIZE as u32,
        file_count: source_files.len(),
        total_file_bytes,
    };
    validate_manifest(&manifest)?;
    let manifest_bytes = serde_json::to_vec(&manifest)?;
    if manifest_bytes.len() > MAX_HEADER_LEN {
        return Err(VoraError::InvalidPackage("Manifest is too large.".into()));
    }

    // Argon2id derives a master key from a high-entropy random access secret.
    // HKDF domain-separates metadata and payload keys so the two AES-GCM nonce
    // spaces are independent and can never collide across these uses.
    let master_key = derive_key(&secret, &salt)?;
    let metadata_key = derive_subkey(&master_key, b"privora/v2/metadata-key")?;
    let payload_key = derive_subkey(&master_key, b"privora/v2/payload-key")?;
    let metadata_cipher =
        Aes256Gcm::new_from_slice(&metadata_key).map_err(|_| VoraError::KeyDerivation)?;

    let encrypted_manifest_len = manifest_bytes
        .len()
        .checked_add(TAG_LEN)
        .ok_or_else(|| VoraError::InvalidPackage("Encrypted manifest length overflowed.".into()))?;
    let encrypted_manifest_len: u32 = encrypted_manifest_len
        .try_into()
        .map_err(|_| VoraError::InvalidPackage("Encrypted manifest is too large.".into()))?;
    let mut header_aad = Vec::with_capacity(8 + 16 + 12 + 4);
    header_aad.extend_from_slice(MAGIC_V2);
    header_aad.extend_from_slice(&salt);
    header_aad.extend_from_slice(&metadata_nonce);
    header_aad.extend_from_slice(&encrypted_manifest_len.to_le_bytes());
    let metadata_nonce_value: Nonce<_> = metadata_nonce
        .as_slice()
        .try_into()
        .map_err(|_| VoraError::InvalidPackage("Invalid metadata nonce length.".into()))?;
    let encrypted_manifest = metadata_cipher
        .encrypt(
            &metadata_nonce_value,
            Payload {
                msg: &manifest_bytes,
                aad: &header_aad,
            },
        )
        .map_err(|_| VoraError::Crypto)?;
    if encrypted_manifest.len() != encrypted_manifest_len as usize {
        return Err(VoraError::InvalidPackage(
            "Unexpected encrypted manifest length.".into(),
        ));
    }
    let mut fingerprint_material = header_aad.clone();
    fingerprint_material.extend_from_slice(&encrypted_manifest);
    let fingerprint = header_fingerprint(&fingerprint_material);

    let payload_cipher =
        Aes256Gcm::new_from_slice(&payload_key).map_err(|_| VoraError::KeyDerivation)?;
    let mut source = File::open(archive_tmp.path())?;
    let mut target = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(output_path)?;
    target.write_all(&header_aad)?;
    target.write_all(&encrypted_manifest)?;

    let mut remaining = plaintext_len;
    let mut chunk_index: u32 = 0;
    while remaining > 0 {
        let current_len = remaining.min(CHUNK_SIZE as u64) as usize;
        let mut plain_chunk = vec![0u8; current_len];
        source.read_exact(&mut plain_chunk)?;
        let nonce_bytes = make_nonce(&nonce_prefix, chunk_index);
        let nonce: Nonce<_> = nonce_bytes
            .as_slice()
            .try_into()
            .map_err(|_| VoraError::InvalidPackage("Invalid payload nonce length.".into()))?;
        let aad = make_aad(&manifest_bytes, chunk_index);
        let encrypted = payload_cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: &plain_chunk,
                    aad: &aad,
                },
            )
            .map_err(|_| VoraError::Crypto)?;
        target.write_all(&encrypted)?;
        remaining -= current_len as u64;
        chunk_index = chunk_index
            .checked_add(1)
            .ok_or_else(|| VoraError::InvalidPackage("Package has too many chunks.".into()))?;
    }
    target.flush()?;
    target.sync_all()?;
    drop(target);

    Ok(CreatedPackage {
        manifest,
        access_code: format_access_code(&secret),
        fingerprint,
    })
}

/// Inspect only the public envelope. In v2, name, ID, timestamps, expiry,
/// activation limits, sizes, file counts, and crypto labels are not exposed here.
pub fn inspect_package(path: &Path) -> Result<PackageInfo> {
    let mut file = File::open(path)?;
    match read_public_header(&mut file)? {
        PackageEnvelope::V1 {
            manifest,
            fingerprint,
            ..
        } => {
            validate_manifest(&manifest)?;
            Ok(PackageInfo {
                format_version: 1,
                fingerprint,
                metadata_encrypted: false,
            })
        }
        PackageEnvelope::V2 { fingerprint, .. } => Ok(PackageInfo {
            format_version: 2,
            fingerprint,
            metadata_encrypted: true,
        }),
    }
}

/// Decode and validate a package manifest after the user supplies its access code.
pub fn inspect_package_with_code(path: &Path, access_code: &str) -> Result<Manifest> {
    let code_bytes = decode_access_code(access_code)?;
    let mut file = File::open(path)?;
    let envelope = read_public_header(&mut file)?;
    let opened = open_manifest(envelope, &code_bytes)?;
    validate_manifest(&opened.manifest)?;
    Ok(opened.manifest)
}

/// Decrypt and safely extract the package to a new subdirectory beneath `output_root`.
/// The activation counter is supplied by the app's local state and is not a server-side counter.
pub fn unlock_package(
    package_path: &Path,
    access_code: &str,
    output_root: &Path,
    prior_activations_on_this_install: u32,
) -> Result<UnlockedPackage> {
    let code_bytes = decode_access_code(access_code)?;
    let mut package = File::open(package_path)?;
    let envelope = read_public_header(&mut package)?;
    let opened = open_manifest(envelope, &code_bytes)?;
    let manifest = opened.manifest;
    validate_manifest(&manifest)?;
    check_policy(&manifest, prior_activations_on_this_install)?;

    let nonce_vec = decode_hex(&manifest.nonce_prefix_hex)?;
    let nonce_prefix: [u8; 8] = nonce_vec
        .try_into()
        .map_err(|_| VoraError::InvalidPackage("Invalid nonce prefix length.".into()))?;
    let cipher =
        Aes256Gcm::new_from_slice(&opened.payload_key).map_err(|_| VoraError::KeyDerivation)?;

    let decrypted_tmp = NamedTempFile::new()?;
    let mut output = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(decrypted_tmp.path())?;
    let mut remaining = manifest.plaintext_len;
    let mut chunk_index: u32 = 0;
    while remaining > 0 {
        let current_len = remaining.min(manifest.chunk_size as u64) as usize;
        let encrypted_len = current_len + TAG_LEN;
        let mut encrypted = vec![0u8; encrypted_len];
        package.read_exact(&mut encrypted)?;
        let nonce_bytes = make_nonce(&nonce_prefix, chunk_index);
        let nonce: Nonce<_> = nonce_bytes
            .as_slice()
            .try_into()
            .map_err(|_| VoraError::InvalidPackage("Invalid payload nonce length.".into()))?;
        let aad = make_aad(&opened.manifest_bytes, chunk_index);
        let plaintext = cipher
            .decrypt(
                &nonce,
                Payload {
                    msg: &encrypted,
                    aad: &aad,
                },
            )
            .map_err(|_| VoraError::Crypto)?;
        if plaintext.len() != current_len {
            return Err(VoraError::InvalidPackage("Chunk length mismatch.".into()));
        }
        output.write_all(&plaintext)?;
        remaining -= current_len as u64;
        chunk_index = chunk_index
            .checked_add(1)
            .ok_or_else(|| VoraError::InvalidPackage("Package has too many chunks.".into()))?;
    }
    let mut trailing = [0u8; 1];
    if package.read(&mut trailing)? != 0 {
        return Err(VoraError::InvalidPackage(
            "Unexpected trailing bytes after encrypted payload.".into(),
        ));
    }
    output.flush()?;
    output.sync_all()?;
    drop(output);

    fs::create_dir_all(output_root)?;
    let output_dir = create_unique_output_dir(output_root, &manifest.name)?;
    let staging = tempfile::Builder::new()
        .prefix(".privora-unpack-")
        .tempdir_in(output_root)?;
    let zip_file = File::open(decrypted_tmp.path())?;
    let mut archive = ZipArchive::new(zip_file)?;
    if archive.len() > MAX_FILES || archive.len() != manifest.file_count {
        return Err(VoraError::InvalidPackage(
            "Archive file count does not match the manifest.".into(),
        ));
    }
    let mut written_files = Vec::new();
    let mut total_uncompressed = 0u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }
        let raw_name = entry.name().to_owned();
        if raw_name.contains('/') || raw_name.contains('\\') || raw_name == "." || raw_name == ".."
        {
            return Err(VoraError::UnsafePath);
        }
        let name = Path::new(&raw_name)
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or(VoraError::UnsafePath)?;
        if name.is_empty()
            || name
                .chars()
                .any(|c| c.is_control() || [':', '*', '?', '"', '<', '>', '|'].contains(&c))
        {
            return Err(VoraError::UnsafePath);
        }
        let remaining_capacity = MAX_ARCHIVE_BYTES.saturating_sub(total_uncompressed);
        let declared_size = entry.size();
        let destination = staging.path().join(name);
        let mut output_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&destination)?;
        let mut limited = (&mut entry).take(remaining_capacity.saturating_add(1));
        let copied = io::copy(&mut limited, &mut output_file)?;
        if copied > remaining_capacity || copied != declared_size {
            return Err(VoraError::InvalidPackage("Extracted size does not match the archive metadata or exceeds the 64 GiB safety limit.".into()));
        }
        total_uncompressed = total_uncompressed
            .checked_add(copied)
            .ok_or_else(|| VoraError::InvalidPackage("Extracted data size overflowed.".into()))?;
        output_file.flush()?;
        written_files.push(output_dir.join(name));
    }
    let staging_path = staging.keep();
    if let Err(error) = fs::rename(&staging_path, &output_dir) {
        let _ = fs::remove_dir_all(&staging_path);
        return Err(VoraError::Io(error));
    }
    Ok(UnlockedPackage {
        manifest,
        output_dir,
        written_files,
        package_fingerprint: opened.fingerprint,
    })
}

pub fn format_access_code(secret: &[u8]) -> String {
    let encoded = BASE32_NOPAD.encode(secret);
    let groups = encoded
        .as_bytes()
        .chunks(5)
        .map(|chunk| std::str::from_utf8(chunk).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("-");
    format!("PV2-{groups}")
}

fn decode_access_code(code: &str) -> Result<Vec<u8>> {
    let compact: String = code
        .trim()
        .to_ascii_uppercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    let payload = compact
        .strip_prefix("PV1")
        .or_else(|| compact.strip_prefix("PV2"))
        .unwrap_or(&compact);
    if payload.len() < 20 || payload.len() > 128 {
        return Err(VoraError::InvalidCode);
    }
    BASE32_NOPAD
        .decode(payload.as_bytes())
        .map_err(|_| VoraError::InvalidCode)
}

fn prepare_source_files(input_paths: &[PathBuf]) -> Result<Vec<SourceFile>> {
    let mut used = HashSet::new();
    let mut out = Vec::with_capacity(input_paths.len());
    for path in input_paths {
        if !path.is_file() {
            return Err(VoraError::InvalidPackage(format!(
                "Not a regular file: {}",
                path.display()
            )));
        }
        let raw = path.file_name().and_then(|s| s.to_str()).unwrap_or("file");
        let clean: String = raw
            .chars()
            .map(|c| {
                if c.is_control() || c == '/' || c == '\\' {
                    '_'
                } else {
                    c
                }
            })
            .collect();
        let clean = if clean.trim().is_empty() {
            "file".to_string()
        } else {
            clean
        };
        let archive_name = unique_name(&clean, &mut used);
        let size = fs::metadata(path)?.len();
        out.push(SourceFile {
            path: path.clone(),
            archive_name: archive_name.clone(),
            size,
        });
    }
    Ok(out)
}

fn unique_name(name: &str, used: &mut HashSet<String>) -> String {
    if used.insert(name.to_lowercase()) {
        return name.to_owned();
    }
    let path = Path::new(name);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = path.extension().and_then(|s| s.to_str());
    for n in 2u32..=100_000 {
        let candidate = match ext {
            Some(ext) if !ext.is_empty() => format!("{stem} ({n}).{ext}"),
            _ => format!("{stem} ({n})"),
        };
        if used.insert(candidate.to_lowercase()) {
            return candidate;
        }
    }
    format!("{uuid}.bin", uuid = Uuid::new_v4())
}

fn create_zip(files: &[SourceFile], path: &Path) -> Result<()> {
    let file = File::create(path)?;
    let mut writer = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o600);
    for source in files {
        writer.start_file(&source.archive_name, options)?;
        let mut input = File::open(&source.path)?;
        io::copy(&mut input, &mut writer)?;
    }
    let mut file = writer.finish()?;
    file.flush()?;
    file.sync_all()?;
    Ok(())
}

enum PackageEnvelope {
    V1 {
        manifest_bytes: Vec<u8>,
        manifest: Manifest,
        fingerprint: String,
    },
    V2 {
        salt: [u8; 16],
        metadata_nonce: [u8; 12],
        encrypted_manifest: Vec<u8>,
        header_aad: Vec<u8>,
        fingerprint: String,
    },
}

struct OpenedManifest {
    manifest_bytes: Vec<u8>,
    manifest: Manifest,
    payload_key: [u8; 32],
    fingerprint: String,
}

fn read_public_header(reader: &mut impl Read) -> Result<PackageEnvelope> {
    let mut magic = [0u8; 8];
    reader.read_exact(&mut magic)?;
    if &magic == MAGIC_V1 {
        let mut len_bytes = [0u8; 4];
        reader.read_exact(&mut len_bytes)?;
        let header_len = u32::from_le_bytes(len_bytes) as usize;
        if header_len == 0 || header_len > MAX_HEADER_LEN {
            return Err(VoraError::InvalidPackage(
                "Invalid v1 manifest length.".into(),
            ));
        }
        let mut manifest_bytes = vec![0u8; header_len];
        reader.read_exact(&mut manifest_bytes)?;
        let manifest: Manifest = serde_json::from_slice(&manifest_bytes)?;
        let mut fingerprint_material = magic.to_vec();
        fingerprint_material.extend_from_slice(&len_bytes);
        fingerprint_material.extend_from_slice(&manifest_bytes);
        Ok(PackageEnvelope::V1 {
            manifest_bytes,
            manifest,
            fingerprint: header_fingerprint(&fingerprint_material),
        })
    } else if &magic == MAGIC_V2 {
        let mut salt = [0u8; 16];
        let mut metadata_nonce = [0u8; 12];
        let mut len_bytes = [0u8; 4];
        reader.read_exact(&mut salt)?;
        reader.read_exact(&mut metadata_nonce)?;
        reader.read_exact(&mut len_bytes)?;
        let encrypted_len = u32::from_le_bytes(len_bytes) as usize;
        if encrypted_len < TAG_LEN || encrypted_len > MAX_HEADER_LEN + TAG_LEN {
            return Err(VoraError::InvalidPackage(
                "Invalid encrypted manifest length.".into(),
            ));
        }
        let mut encrypted_manifest = vec![0u8; encrypted_len];
        reader.read_exact(&mut encrypted_manifest)?;
        let mut header_aad = Vec::with_capacity(40);
        header_aad.extend_from_slice(&magic);
        header_aad.extend_from_slice(&salt);
        header_aad.extend_from_slice(&metadata_nonce);
        header_aad.extend_from_slice(&len_bytes);
        let mut fingerprint_material = header_aad.clone();
        fingerprint_material.extend_from_slice(&encrypted_manifest);
        Ok(PackageEnvelope::V2 {
            salt,
            metadata_nonce,
            encrypted_manifest,
            header_aad,
            fingerprint: header_fingerprint(&fingerprint_material),
        })
    } else if &magic[..6] == b"PRVORA" {
        Err(VoraError::UnsupportedVersion)
    } else {
        Err(VoraError::BadMagic)
    }
}

fn open_manifest(envelope: PackageEnvelope, secret: &[u8]) -> Result<OpenedManifest> {
    match envelope {
        PackageEnvelope::V1 {
            manifest_bytes,
            manifest,
            fingerprint,
        } => {
            validate_manifest(&manifest)?;
            if manifest.format_version != 1 {
                return Err(VoraError::UnsupportedVersion);
            }
            let salt_vec = decode_hex(&manifest.salt_hex)?;
            let salt: [u8; 16] = salt_vec
                .try_into()
                .map_err(|_| VoraError::InvalidPackage("Invalid salt length.".into()))?;
            let payload_key = derive_key(secret, &salt)?;
            Ok(OpenedManifest {
                manifest_bytes,
                manifest,
                payload_key,
                fingerprint,
            })
        }
        PackageEnvelope::V2 {
            salt,
            metadata_nonce,
            encrypted_manifest,
            header_aad,
            fingerprint,
        } => {
            let master_key = derive_key(secret, &salt)?;
            let metadata_key = derive_subkey(&master_key, b"privora/v2/metadata-key")?;
            let metadata_cipher =
                Aes256Gcm::new_from_slice(&metadata_key).map_err(|_| VoraError::KeyDerivation)?;
            let metadata_nonce_value: Nonce<_> = metadata_nonce
                .as_slice()
                .try_into()
                .map_err(|_| VoraError::InvalidPackage("Invalid metadata nonce length.".into()))?;
            let manifest_bytes = metadata_cipher
                .decrypt(
                    &metadata_nonce_value,
                    Payload {
                        msg: &encrypted_manifest,
                        aad: &header_aad,
                    },
                )
                .map_err(|_| VoraError::Crypto)?;
            if manifest_bytes.len() > MAX_HEADER_LEN {
                return Err(VoraError::InvalidPackage(
                    "Decrypted manifest is too large.".into(),
                ));
            }
            let manifest: Manifest = serde_json::from_slice(&manifest_bytes)?;
            validate_manifest(&manifest)?;
            if manifest.format_version != 2 || manifest.salt_hex != hex::encode(salt) {
                return Err(VoraError::InvalidPackage(
                    "Manifest does not match its authenticated envelope.".into(),
                ));
            }
            let payload_key = derive_subkey(&master_key, b"privora/v2/payload-key")?;
            Ok(OpenedManifest {
                manifest_bytes,
                manifest,
                payload_key,
                fingerprint,
            })
        }
    }
}

fn header_fingerprint(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn validate_manifest(manifest: &Manifest) -> Result<()> {
    if !(manifest.format_version == 1 || manifest.format_version == 2) {
        return Err(VoraError::UnsupportedVersion);
    }
    if manifest.name.trim().is_empty() || manifest.name.len() > 160 {
        return Err(VoraError::InvalidPackage("Invalid package name.".into()));
    }
    if manifest.package_id.len() > 80 || manifest.file_count == 0 || manifest.file_count > MAX_FILES
    {
        return Err(VoraError::InvalidPackage(
            "Manifest limits exceeded.".into(),
        ));
    }
    if manifest.total_file_bytes > MAX_ARCHIVE_BYTES {
        return Err(VoraError::InvalidPackage(
            "Original file size metadata exceeds the safety limit.".into(),
        ));
    }
    if manifest.chunk_size as usize != CHUNK_SIZE {
        return Err(VoraError::InvalidPackage("Unsupported chunk size.".into()));
    }
    if manifest.plaintext_len == 0 || manifest.plaintext_len > MAX_ARCHIVE_BYTES {
        return Err(VoraError::InvalidPackage("Invalid payload size.".into()));
    }
    if manifest.max_activations == 0 || manifest.max_activations > 100_000 {
        return Err(VoraError::InvalidPackage(
            "Invalid activation limit.".into(),
        ));
    }
    if decode_hex(&manifest.salt_hex)?.len() != 16
        || decode_hex(&manifest.nonce_prefix_hex)?.len() != 8
    {
        return Err(VoraError::InvalidPackage(
            "Invalid cryptographic metadata.".into(),
        ));
    }
    if let Some(date) = &manifest.expires_on {
        NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|_| VoraError::InvalidPackage("Invalid expiry date.".into()))?;
    }
    Ok(())
}

fn check_policy(manifest: &Manifest, prior_activations: u32) -> Result<()> {
    if let Some(date) = &manifest.expires_on {
        let expiry = NaiveDate::parse_from_str(date, "%Y-%m-%d")
            .map_err(|_| VoraError::InvalidPackage("Invalid expiry date.".into()))?;
        if Local::now().date_naive() > expiry {
            return Err(VoraError::Expired(date.clone()));
        }
    }
    if prior_activations >= manifest.max_activations {
        return Err(VoraError::ActivationLimit {
            limit: manifest.max_activations,
        });
    }
    Ok(())
}

fn derive_key(secret: &[u8], salt: &[u8; 16]) -> Result<[u8; 32]> {
    let params = Params::new(32 * 1024, 3, 1, Some(32)).map_err(|_| VoraError::KeyDerivation)?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(secret, salt, &mut key)
        .map_err(|_| VoraError::KeyDerivation)?;
    Ok(key)
}

fn derive_subkey(master_key: &[u8; 32], context: &[u8]) -> Result<[u8; 32]> {
    let hkdf = Hkdf::<Sha256>::new(None, master_key);
    let mut key = [0u8; 32];
    hkdf.expand(context, &mut key)
        .map_err(|_| VoraError::KeyDerivation)?;
    Ok(key)
}

fn make_nonce(prefix: &[u8; 8], index: u32) -> [u8; 12] {
    let mut nonce = [0u8; 12];
    nonce[..8].copy_from_slice(prefix);
    nonce[8..].copy_from_slice(&index.to_be_bytes());
    nonce
}

fn make_aad(manifest: &[u8], index: u32) -> Vec<u8> {
    let mut aad = Vec::with_capacity(manifest.len() + 4);
    aad.extend_from_slice(manifest);
    aad.extend_from_slice(&index.to_be_bytes());
    aad
}

fn decode_hex(input: &str) -> Result<Vec<u8>> {
    hex::decode(input)
        .map_err(|_| VoraError::InvalidPackage("Invalid hex field in manifest.".into()))
}

fn create_unique_output_dir(root: &Path, name: &str) -> Result<PathBuf> {
    let base: String = name
        .chars()
        .map(|c| {
            if c.is_control() || ['/', '\\', ':', '*', '?', '"', '<', '>', '|'].contains(&c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let base = base.trim().trim_matches('.');
    let base = if base.is_empty() {
        "Privora Package"
    } else {
        base
    };
    for n in 1u32..=100_000 {
        let candidate = if n == 1 {
            root.join(base)
        } else {
            root.join(format!("{base} ({n})"))
        };
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(VoraError::InvalidPackage(
        "Could not find a free extraction directory.".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn access_code_round_trip() {
        let mut secret = [0u8; 32];
        getrandom::fill(&mut secret).expect("system random source");
        let code = format_access_code(&secret);
        assert!(code.starts_with("PV2-"));
        assert_eq!(decode_access_code(&code).expect("decode"), secret);
        assert!(decode_access_code("PV1-not-a-valid-code").is_err());
    }

    #[test]
    fn create_and_unlock_single_file_and_hide_manifest_metadata() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("hello.txt");
        fs::write(&source, "Privora offline test 🟣").expect("write source");
        let package = temp.path().join("test.vora");
        let created = create_package(&[source], &package, "Highly Private Project Name", None, 2)
            .expect("create package");

        let info = inspect_package(&package).expect("inspect public envelope");
        assert_eq!(info.format_version, 2);
        assert!(info.metadata_encrypted);
        assert_eq!(info.fingerprint, created.fingerprint);

        let bytes = fs::read(&package).expect("read package");
        assert!(
            !bytes
                .windows(b"Highly Private Project Name".len())
                .any(|w| w == b"Highly Private Project Name")
        );
        assert!(
            !bytes
                .windows(b"max_activations".len())
                .any(|w| w == b"max_activations")
        );
        assert!(
            !bytes
                .windows(b"created_at".len())
                .any(|w| w == b"created_at")
        );
        assert!(!bytes.windows(b"Argon2id".len()).any(|w| w == b"Argon2id"));

        let manifest =
            inspect_package_with_code(&package, &created.access_code).expect("inspect with code");
        assert_eq!(manifest.name, "Highly Private Project Name");
        assert_eq!(manifest.format_version, 2);
        let output_root = temp.path().join("output");
        let opened =
            unlock_package(&package, &created.access_code, &output_root, 0).expect("unlock");
        assert_eq!(opened.package_fingerprint, info.fingerprint);
        assert_eq!(opened.written_files.len(), 1);
        let text = fs::read_to_string(&opened.written_files[0]).expect("read unlocked");
        assert_eq!(text, "Privora offline test 🟣");
        assert!(unlock_package(&package, "PV2-not-a-valid-code", &output_root, 0).is_err());
    }

    #[test]
    fn multi_chunk_payload_round_trip() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("random.bin");
        let mut bytes = vec![0u8; CHUNK_SIZE * 2 + 177];
        getrandom::fill(&mut bytes[..]).expect("system random source");
        fs::write(&source, &bytes).expect("write source");
        let package = temp.path().join("multi.vora");
        let created =
            create_package(&[source], &package, "Multi Chunk", None, 2).expect("create package");
        let out = temp.path().join("out");
        let unlocked =
            unlock_package(&package, &created.access_code, &out, 0).expect("unlock package");
        let restored = fs::read(&unlocked.written_files[0]).expect("read restored");
        assert_eq!(restored, bytes);
    }

    #[test]
    fn expiration_policy_is_checked() {
        let manifest = Manifest {
            format_version: 2,
            package_id: "test".into(),
            name: "expired".into(),
            created_at: Utc::now().to_rfc3339(),
            expires_on: Some(
                (Local::now().date_naive() - chrono::Duration::days(1))
                    .format("%Y-%m-%d")
                    .to_string(),
            ),
            max_activations: 1,
            kdf: "Argon2id".into(),
            cipher: "AES-256-GCM".into(),
            salt_hex: hex::encode([0u8; 16]),
            nonce_prefix_hex: hex::encode([0u8; 8]),
            plaintext_len: 1,
            chunk_size: CHUNK_SIZE as u32,
            file_count: 1,
            total_file_bytes: 1,
        };
        assert!(matches!(
            check_policy(&manifest, 0),
            Err(VoraError::Expired(_))
        ));
        let mut no_expiry = manifest;
        no_expiry.expires_on = None;
        assert!(check_policy(&no_expiry, 0).is_ok());
    }

    #[test]
    fn encrypted_manifest_tampering_is_rejected() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("a.txt");
        fs::write(&source, "alpha").expect("source");
        let package = temp.path().join("test.vora");
        let created = create_package(&[source], &package, "Original", None, 2).expect("create");
        let mut bytes = fs::read(&package).expect("read package");
        // V2 fixed header: magic (8), salt (16), metadata nonce (12), length (4).
        // Change the ciphertext while keeping the envelope parseable.
        let metadata_start = 40;
        bytes[metadata_start + 3] ^= 0x20;
        fs::write(&package, bytes).expect("tamper");
        let out = temp.path().join("out");
        assert!(unlock_package(&package, &created.access_code, &out, 0).is_err());
    }

    #[test]
    fn wrong_code_cannot_reveal_encrypted_manifest() {
        let temp = tempfile::tempdir().expect("tempdir");
        let source = temp.path().join("a.txt");
        fs::write(&source, "alpha").expect("source");
        let package = temp.path().join("test.vora");
        let _created = create_package(&[source], &package, "Secret Name", None, 2).expect("create");
        assert!(inspect_package_with_code(&package, "PV2-invalid-code").is_err());
    }
}
