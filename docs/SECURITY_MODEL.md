# Security model and current limitations

## Protected assets
- File bytes before unlock.
- Integrity of the encrypted manifest and each authenticated payload chunk.
- Confidentiality against an attacker who has a `.vora` package but not its high-entropy access code.
- In v2, confidentiality of package name/ID, creation time, validity dates, recipient label, issuer note, activation limit, archive metadata, file count and crypto labels from inspection of the container header.

## Controls implemented
- Random 256-bit access secret generated per package.
- Argon2id key derivation with a random salt.
- HKDF-SHA256 separates metadata and payload encryption keys.
- AES-256-GCM authenticates the encrypted v2 manifest and every payload chunk.
- The v2 public header is used as manifest AAD; the decrypted manifest bytes are used as per-chunk AAD.
- A random payload nonce prefix and increasing chunk index provide unique payload nonces within a package.
- Defensive length/version checks, path safety checks and bounded extraction.
- Access codes are not written to the app's local history JSON. Creating a package no longer automatically writes a plaintext sidecar containing the code; users may explicitly save a separate `.license.txt` note.
- The encrypted issuer note is not duplicated into `state.json`; local history keeps only a boolean indicating that the note exists. Package name, recipient label, validity fields and filesystem path still exist in local history as ordinary JSON.

## Public information and metadata leakage
- V2 necessarily exposes the fixed format marker, random KDF salt, random manifest nonce, encrypted-manifest length and overall approximate package size.
- The external file name chosen in the file picker is outside the container and can reveal information if the user renames it descriptively. The save dialog uses a generic default file name.
- V1 packages keep their old plaintext manifest; only newly created v2 packages conceal the manifest.
- The KDF algorithm is not intended to be secret. Concealing its name is not a substitute for a high-entropy access secret and reviewed cryptographic design.
- A deliberately saved `.license.txt` contains the code in plaintext. The code must be delivered separately and stored carefully.

## Intended attacker
Someone who receives or copies the `.vora` file but does not know the access code, or attempts to modify the package bytes.

## Out of scope / not guaranteed
- A recipient who has both the package and code can decrypt it.
- The app currently exports plaintext files on unlock. Once extracted, their copying is outside app control.
- Activation counts are local to the application's state file and can be reset, copied or bypassed. There is no shared global count among recipients.
- Expiration uses the device clock and is vulnerable to clock rollback.
- There is no online revocation, device-bound licensing, publisher signature infrastructure or recipient public-key enrolment in this version.
- Package creation currently writes a temporary plaintext ZIP before encrypting it; unlock similarly uses a temporary decrypted ZIP before extraction. These temporary files are automatically removed by normal cleanup, but deletion is not guaranteed secure erasure on SSDs, journaling filesystems or backups. Do not treat the current alpha as a no-trace vault on a shared or compromised machine.
- The app creator-side local state file stores selected metadata for packages created on that computer as ordinary local JSON so the Licenses screen can display it. The secret access code and private note text are not saved there, but package name, recipient label, dates, path and other metadata can be read by a person with access to the local app profile.
- The protocol and application have not undergone an independent security audit or formal cryptographic protocol review. “Uncrackable” is not an appropriate claim.

## Before production
Keep the manifest-encryption key separate from the payload key; retain independent round-trip/tamper tests; fuzz parsers and ZIP handling; test malformed lengths and wrong codes; review secret lifetime and temporary-file handling; define separate signed license objects and recipient/device identity; determine how expiration and activation are enforced offline; and commission an independent security review.
