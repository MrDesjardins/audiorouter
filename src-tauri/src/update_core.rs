//! Decision-independent core of a signed in-app update. It is inert: nothing
//! in the shell calls it, and it has no network code, installer launch, Tauri
//! command, REST/MCP/UI surface or key of any kind.
//!
//! The pieces are plain functions over plain inputs, so they can be tested off
//! Windows and wired later without a redesign:
//!
//! * [`evaluate_manifest`] checks a manifest's size, signature, shape, fixed
//!   hosts, platform, version rules, minimum app version and kill switch.
//! * [`ArtifactVerification`] checks a streamed download against the declared
//!   size, SHA-256 and signature, and yields a [`VerifiedArtifact`].
//! * [`LifecycleState::check_install`] refuses while a session, recording or
//!   driver operation is active or unknown.
//!
//! Signature verification sits behind [`SignatureVerifier`]. No suitable
//! signature crate is in `Cargo.lock` and no dependency may be added here, so
//! the only non-test implementation is [`NoTrustedKey`], which refuses
//! everything. There is no real signature verification in this module.
//!
//! The manifest schema and the size caps below are PROVISIONAL: they wait on
//! the release-pipeline and feed decisions in
//! `docs/plans/future/auto-update-tauri.md`. This module does not satisfy
//! SEC-11 and must not be described as an updater.
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;

/// Largest manifest accepted before any parsing. Provisional.
pub const MAX_MANIFEST_BYTES: usize = 64 * 1024;
/// Largest installer accepted. Provisional.
pub const MAX_ARTIFACT_BYTES: u64 = 512 * 1024 * 1024;
const MAX_ARTIFACTS: usize = 8;
const MAX_URL_BYTES: usize = 2048;
const MAX_SIGNATURE_BYTES: usize = 4096;
const MAX_FIELD_BYTES: usize = 64;
const MAX_HOST_BYTES: usize = 253;
const SCHEMA_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Signature seam
// ---------------------------------------------------------------------------

/// Why a signature was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignatureError {
    /// No trusted key is configured, so nothing can be trusted.
    NoTrustedKey,
    /// The signature text is missing, oversized or not parseable.
    Malformed,
    /// The signature does not match the signed bytes.
    Invalid,
}

/// Checks signatures against whatever key policy the product eventually
/// adopts. What exactly is signed (raw file bytes, a prehash, the manifest
/// alone or both) and how keys are rotated or revoked wait on the key and
/// format decision, so the artifact side is a streaming session that sees the
/// same bytes as the hasher and can implement any of them.
pub trait SignatureVerifier {
    /// Verifies `signature` over the raw manifest bytes.
    fn verify_manifest(&self, manifest: &[u8], signature: &str) -> Result<(), SignatureError>;
    /// Starts checking one artifact. Refusing here (for example with no key)
    /// stops the download before any byte is accepted.
    fn begin_artifact(
        &self,
        signature: &str,
    ) -> Result<Box<dyn ArtifactSignatureCheck>, SignatureError>;
}

/// One artifact's signature check, fed every chunk the hasher sees.
pub trait ArtifactSignatureCheck {
    fn update(&mut self, chunk: &[u8]);
    fn finish(self: Box<Self>) -> Result<(), SignatureError>;
}

/// The only non-test verifier: no key is trusted, so every check refuses.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoTrustedKey;

impl SignatureVerifier for NoTrustedKey {
    fn verify_manifest(&self, _manifest: &[u8], _signature: &str) -> Result<(), SignatureError> {
        Err(SignatureError::NoTrustedKey)
    }

    fn begin_artifact(
        &self,
        _signature: &str,
    ) -> Result<Box<dyn ArtifactSignatureCheck>, SignatureError> {
        Err(SignatureError::NoTrustedKey)
    }
}

// ---------------------------------------------------------------------------
// Versions
// ---------------------------------------------------------------------------

/// `MAJOR.MINOR.PATCH` with an optional `-prerelease`. Build metadata (`+`) is
/// refused. Ordering follows semantic-version precedence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    major: u32,
    minor: u32,
    patch: u32,
    pre: Vec<PreIdentifier>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum PreIdentifier {
    // Declared first so a numeric identifier sorts below an alphanumeric one.
    Numeric(u32),
    Alpha(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VersionError;

fn parse_number(text: &str) -> Result<u32, VersionError> {
    let valid = !text.is_empty()
        && text.len() <= 9
        && text.bytes().all(|b| b.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'));
    if !valid {
        return Err(VersionError);
    }
    text.parse().map_err(|_| VersionError)
}

impl Version {
    pub fn parse(text: &str) -> Result<Self, VersionError> {
        if text.is_empty() || text.len() > MAX_FIELD_BYTES {
            return Err(VersionError);
        }
        let (core, pre_text) = match text.split_once('-') {
            Some((core, pre)) => (core, Some(pre)),
            None => (text, None),
        };
        let mut numbers = core.split('.');
        let major = parse_number(numbers.next().ok_or(VersionError)?)?;
        let minor = parse_number(numbers.next().ok_or(VersionError)?)?;
        let patch = parse_number(numbers.next().ok_or(VersionError)?)?;
        if numbers.next().is_some() {
            return Err(VersionError);
        }
        let mut pre = Vec::new();
        if let Some(pre_text) = pre_text {
            for identifier in pre_text.split('.') {
                let valid = !identifier.is_empty()
                    && identifier
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-');
                if !valid {
                    return Err(VersionError);
                }
                if identifier.bytes().all(|b| b.is_ascii_digit()) {
                    pre.push(PreIdentifier::Numeric(parse_number(identifier)?));
                } else {
                    pre.push(PreIdentifier::Alpha(identifier.to_owned()));
                }
            }
        }
        Ok(Self {
            major,
            minor,
            patch,
            pre,
        })
    }

    pub fn is_prerelease(&self) -> bool {
        !self.pre.is_empty()
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                // A release is newer than any prerelease of the same version.
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => self.pre.cmp(&other.pre),
            })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ---------------------------------------------------------------------------
// Fixed-host HTTPS URLs
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UrlRefusal {
    TooLong,
    ForbiddenCharacter,
    NotHttps,
    Credentials,
    WrongHost,
    MissingPath,
    Fragment,
}

/// A host the caller allows, lowercase DNS labels only. The caller supplies
/// it; this module has no default and so picks no feed host.
fn host_is_valid(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= MAX_HOST_BYTES
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}

/// Accepts only `https://<allowed_host>/<path>` with no credentials, port,
/// fragment, whitespace, backslash or non-ASCII byte. Redirects are a
/// downloader concern and are not decided here.
fn check_https_url(url: &str, allowed_host: &str) -> Result<(), UrlRefusal> {
    if url.len() > MAX_URL_BYTES {
        return Err(UrlRefusal::TooLong);
    }
    if !url.bytes().all(|b| b.is_ascii_graphic()) || url.contains('\\') {
        return Err(UrlRefusal::ForbiddenCharacter);
    }
    let rest = url.strip_prefix("https://").ok_or(UrlRefusal::NotHttps)?;
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, remainder) = rest.split_at(authority_end);
    if authority.contains('@') {
        return Err(UrlRefusal::Credentials);
    }
    if !authority.eq_ignore_ascii_case(allowed_host) {
        return Err(UrlRefusal::WrongHost);
    }
    if !remainder.starts_with('/') {
        return Err(UrlRefusal::MissingPath);
    }
    if remainder.contains('#') {
        return Err(UrlRefusal::Fragment);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Manifest
// ---------------------------------------------------------------------------

/// Release channel. Which channels exist waits on the channel decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Stable,
    Prerelease,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawManifest {
    schema_version: u32,
    channel: Channel,
    version: String,
    published_at: String,
    notes_url: String,
    min_app_version: String,
    min_driver_version: String,
    withdrawn: bool,
    artifacts: Vec<RawArtifact>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RawArtifact {
    platform: String,
    url: String,
    size_bytes: u64,
    sha256: String,
    signature: String,
}

/// Hosts the caller allows, one per URL role. Both may be the same value.
#[derive(Clone, Copy, Debug)]
pub struct AllowedHosts<'a> {
    pub artifact: &'a str,
    pub release_page: &'a str,
}

/// Everything the caller knows about this installation and its preferences.
#[derive(Clone, Debug)]
pub struct UpdatePolicy<'a> {
    pub hosts: AllowedHosts<'a>,
    /// For example `windows-x86_64`; matched exactly against each artifact.
    pub platform: &'a str,
    pub channel: Channel,
    pub installed_version: Version,
    /// Highest version this installation has already seen offered, so an older
    /// manifest cannot be replayed even when it is newer than what is installed.
    pub highest_seen_version: Option<Version>,
    pub allow_prerelease: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateRefusal {
    InvalidPolicy(&'static str),
    ManifestTooLarge,
    Signature(SignatureError),
    /// Not the expected JSON: unknown, missing, duplicate or mistyped field.
    Malformed,
    UnsupportedSchema,
    /// The kill switch: the channel withdrew this release.
    Withdrawn,
    ChannelMismatch,
    BadField(&'static str),
    BadUrl(&'static str, UrlRefusal),
    NoArtifactForPlatform,
    AmbiguousArtifact,
    /// Same or older than the installed version.
    NotNewer,
    /// Older than a version this installation has already seen.
    Replayed,
    PrereleaseNotAllowed,
    /// The installed app is older than the manifest's minimum app version.
    AppTooOld,
}

/// The one artifact chosen for this platform, with its checked expectations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactPlan {
    url: String,
    size_bytes: u64,
    sha256: [u8; 32],
    signature: String,
}

impl ArtifactPlan {
    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    pub fn sha256(&self) -> &[u8; 32] {
        &self.sha256
    }
}

/// A manifest that passed every check. It authorizes starting a download, not
/// installing: the download must still pass [`ArtifactVerification`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcceptedUpdate {
    pub version: Version,
    pub channel: Channel,
    pub notes_url: String,
    pub min_app_version: Version,
    /// Parsed only. How a too-old driver is handled waits on the driver
    /// compatibility decision.
    pub min_driver_version: Version,
    pub artifact: ArtifactPlan,
}

fn is_signature_text(signature: &str) -> bool {
    !signature.is_empty()
        && signature.len() <= MAX_SIGNATURE_BYTES
        && signature
            .bytes()
            .all(|b| b == b'\n' || (0x20..=0x7e).contains(&b))
}

fn is_platform_tag(platform: &str) -> bool {
    !platform.is_empty()
        && platform.len() <= MAX_FIELD_BYTES
        && platform
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

fn parse_sha256(text: &str) -> Option<[u8; 32]> {
    if text.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (slot, pair) in out.iter_mut().zip(text.as_bytes().chunks(2)) {
        let high = nibble(pair[0])?;
        let low = nibble(pair[1])?;
        *slot = (high << 4) | low;
    }
    Some(out)
}

fn nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

fn parse_version_field(text: &str, field: &'static str) -> Result<Version, UpdateRefusal> {
    Version::parse(text).map_err(|_| UpdateRefusal::BadField(field))
}

/// Decides whether a manifest may start a download. The order is: policy,
/// size bound, signature over the raw bytes, then parsing, so unauthenticated
/// input is never interpreted.
pub fn evaluate_manifest(
    manifest: &[u8],
    manifest_signature: &str,
    policy: &UpdatePolicy<'_>,
    verifier: &dyn SignatureVerifier,
) -> Result<AcceptedUpdate, UpdateRefusal> {
    if !host_is_valid(policy.hosts.artifact) {
        return Err(UpdateRefusal::InvalidPolicy("artifact host"));
    }
    if !host_is_valid(policy.hosts.release_page) {
        return Err(UpdateRefusal::InvalidPolicy("release page host"));
    }
    if !is_platform_tag(policy.platform) {
        return Err(UpdateRefusal::InvalidPolicy("platform"));
    }
    if manifest.len() > MAX_MANIFEST_BYTES {
        return Err(UpdateRefusal::ManifestTooLarge);
    }
    if !is_signature_text(manifest_signature) {
        return Err(UpdateRefusal::Signature(SignatureError::Malformed));
    }
    verifier
        .verify_manifest(manifest, manifest_signature)
        .map_err(UpdateRefusal::Signature)?;

    let raw: RawManifest =
        serde_json::from_slice(manifest).map_err(|_| UpdateRefusal::Malformed)?;
    if raw.schema_version != SCHEMA_VERSION {
        return Err(UpdateRefusal::UnsupportedSchema);
    }
    if raw.withdrawn {
        return Err(UpdateRefusal::Withdrawn);
    }
    if raw.channel != policy.channel {
        return Err(UpdateRefusal::ChannelMismatch);
    }

    let version = parse_version_field(&raw.version, "version")?;
    let min_app_version = parse_version_field(&raw.min_app_version, "minAppVersion")?;
    let min_driver_version = parse_version_field(&raw.min_driver_version, "minDriverVersion")?;
    if raw.published_at.is_empty()
        || raw.published_at.len() > MAX_FIELD_BYTES
        || !raw.published_at.bytes().all(|b| b.is_ascii_graphic())
    {
        return Err(UpdateRefusal::BadField("publishedAt"));
    }
    if min_app_version > version {
        return Err(UpdateRefusal::BadField("minAppVersion"));
    }
    check_https_url(&raw.notes_url, policy.hosts.release_page)
        .map_err(|reason| UpdateRefusal::BadUrl("notesUrl", reason))?;

    // Version rules. Downgrade and replay come before prerelease so a stale
    // manifest is reported as stale, whatever its channel.
    if version <= policy.installed_version {
        return Err(UpdateRefusal::NotNewer);
    }
    if let Some(seen) = &policy.highest_seen_version {
        if version < *seen {
            return Err(UpdateRefusal::Replayed);
        }
    }
    if version.is_prerelease() && !policy.allow_prerelease {
        return Err(UpdateRefusal::PrereleaseNotAllowed);
    }
    if policy.installed_version < min_app_version {
        return Err(UpdateRefusal::AppTooOld);
    }

    if raw.artifacts.is_empty() || raw.artifacts.len() > MAX_ARTIFACTS {
        return Err(UpdateRefusal::BadField("artifacts"));
    }
    let mut chosen: Option<ArtifactPlan> = None;
    for artifact in &raw.artifacts {
        if !is_platform_tag(&artifact.platform) {
            return Err(UpdateRefusal::BadField("platform"));
        }
        if artifact.size_bytes == 0 || artifact.size_bytes > MAX_ARTIFACT_BYTES {
            return Err(UpdateRefusal::BadField("sizeBytes"));
        }
        let sha256 = parse_sha256(&artifact.sha256).ok_or(UpdateRefusal::BadField("sha256"))?;
        if !is_signature_text(&artifact.signature) {
            return Err(UpdateRefusal::BadField("signature"));
        }
        check_https_url(&artifact.url, policy.hosts.artifact)
            .map_err(|reason| UpdateRefusal::BadUrl("url", reason))?;
        if artifact.platform == policy.platform {
            if chosen.is_some() {
                return Err(UpdateRefusal::AmbiguousArtifact);
            }
            chosen = Some(ArtifactPlan {
                url: artifact.url.clone(),
                size_bytes: artifact.size_bytes,
                sha256,
                signature: artifact.signature.clone(),
            });
        }
    }
    let artifact = chosen.ok_or(UpdateRefusal::NoArtifactForPlatform)?;

    Ok(AcceptedUpdate {
        version,
        channel: raw.channel,
        notes_url: raw.notes_url,
        min_app_version,
        min_driver_version,
        artifact,
    })
}

// ---------------------------------------------------------------------------
// Streamed artifact verification
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactRefusal {
    /// More bytes arrived than the manifest declared.
    TooLarge,
    /// The stream ended with fewer bytes than declared.
    SizeMismatch,
    HashMismatch,
    Signature(SignatureError),
}

/// Proof that an artifact's bytes matched the declared size, SHA-256 and
/// signature. Only [`ArtifactVerification::finish`] can build one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedArtifact {
    size_bytes: u64,
    sha256: [u8; 32],
}

impl VerifiedArtifact {
    pub fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    pub fn sha256(&self) -> &[u8; 32] {
        &self.sha256
    }
}

/// Verifies one download as it streams, without holding it in memory. The
/// caller writes the same chunks to its temporary file. Any refusal is sticky:
/// later chunks and `finish` return it again.
pub struct ArtifactVerification {
    expected_size: u64,
    expected_sha256: [u8; 32],
    received: u64,
    hasher: Sha256,
    signature: Box<dyn ArtifactSignatureCheck>,
    refused: Option<ArtifactRefusal>,
}

impl ArtifactVerification {
    /// Refuses at once when the verifier trusts no key.
    pub fn begin(
        plan: &ArtifactPlan,
        verifier: &dyn SignatureVerifier,
    ) -> Result<Self, ArtifactRefusal> {
        let signature = verifier
            .begin_artifact(&plan.signature)
            .map_err(ArtifactRefusal::Signature)?;
        Ok(Self {
            expected_size: plan.size_bytes,
            expected_sha256: plan.sha256,
            received: 0,
            hasher: Sha256::new(),
            signature,
            refused: None,
        })
    }

    pub fn push(&mut self, chunk: &[u8]) -> Result<(), ArtifactRefusal> {
        if let Some(refusal) = self.refused {
            return Err(refusal);
        }
        let total = self
            .received
            .checked_add(chunk.len() as u64)
            .filter(|total| *total <= self.expected_size);
        let Some(total) = total else {
            self.refused = Some(ArtifactRefusal::TooLarge);
            return Err(ArtifactRefusal::TooLarge);
        };
        self.received = total;
        self.hasher.update(chunk);
        self.signature.update(chunk);
        Ok(())
    }

    /// Checks size, then SHA-256, then the signature.
    pub fn finish(self) -> Result<VerifiedArtifact, ArtifactRefusal> {
        if let Some(refusal) = self.refused {
            return Err(refusal);
        }
        if self.received != self.expected_size {
            return Err(ArtifactRefusal::SizeMismatch);
        }
        let digest: [u8; 32] = self.hasher.finalize().into();
        if digest != self.expected_sha256 {
            return Err(ArtifactRefusal::HashMismatch);
        }
        self.signature
            .finish()
            .map_err(ArtifactRefusal::Signature)?;
        Ok(VerifiedArtifact {
            size_bytes: self.received,
            sha256: digest,
        })
    }
}

// ---------------------------------------------------------------------------
// Lifecycle guards
// ---------------------------------------------------------------------------

/// What the caller knows about one kind of activity. A caller that cannot tell
/// reports `Unknown`, which refuses like `Active`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    Idle,
    Active,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blocker {
    Recording,
    DriverOperation,
    Session,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleRefusal {
    Active(Blocker),
    Unknown(Blocker),
}

/// Plain inputs for the install guard. Whether a running session may instead
/// be stopped on request, or the install deferred to a later quit, waits on
/// the "Later" and install-on-quit decisions; this guard only refuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LifecycleState {
    pub session: Activity,
    pub recording: Activity,
    pub driver_operation: Activity,
}

impl LifecycleState {
    /// Reports the first blocker, most serious first: a recording, a driver
    /// operation, then a running session. Check again right before installing;
    /// the state can change after a download.
    pub fn check_install(&self) -> Result<(), LifecycleRefusal> {
        for (blocker, activity) in [
            (Blocker::Recording, self.recording),
            (Blocker::DriverOperation, self.driver_operation),
            (Blocker::Session, self.session),
        ] {
            match activity {
                Activity::Idle => {}
                Activity::Active => return Err(LifecycleRefusal::Active(blocker)),
                Activity::Unknown => return Err(LifecycleRefusal::Unknown(blocker)),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::cell::Cell;
    use std::rc::Rc;

    const ARTIFACT_HOST: &str = "updates.example.invalid";
    const PAGE_HOST: &str = "releases.example.invalid";
    const PLATFORM: &str = "windows-x86_64";
    const INSTALLER: &[u8] = b"pretend installer bytes for tests only";

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn sha_of(bytes: &[u8]) -> String {
        hex(&Sha256::digest(bytes))
    }

    /// A plain checksum label, not a key: the fake verifier compares it with
    /// the bytes it saw, so a changed byte or length changes the label.
    fn marker(bytes: &[u8]) -> String {
        format!(
            "test-marker-{}-{}",
            bytes.len(),
            bytes.iter().map(|b| u64::from(*b)).sum::<u64>()
        )
    }

    /// Accepts a signature equal to the marker of the exact bytes it is shown.
    #[derive(Default)]
    struct MarkerVerifier {
        manifest_calls: Rc<Cell<usize>>,
    }

    struct MarkerStream {
        signature: String,
        seen: Vec<u8>,
    }

    impl ArtifactSignatureCheck for MarkerStream {
        fn update(&mut self, chunk: &[u8]) {
            self.seen.extend_from_slice(chunk);
        }

        fn finish(self: Box<Self>) -> Result<(), SignatureError> {
            if self.signature == marker(&self.seen) {
                Ok(())
            } else {
                Err(SignatureError::Invalid)
            }
        }
    }

    impl SignatureVerifier for MarkerVerifier {
        fn verify_manifest(&self, manifest: &[u8], signature: &str) -> Result<(), SignatureError> {
            self.manifest_calls.set(self.manifest_calls.get() + 1);
            if signature == marker(manifest) {
                Ok(())
            } else {
                Err(SignatureError::Invalid)
            }
        }

        fn begin_artifact(
            &self,
            signature: &str,
        ) -> Result<Box<dyn ArtifactSignatureCheck>, SignatureError> {
            Ok(Box::new(MarkerStream {
                signature: signature.to_owned(),
                seen: Vec::new(),
            }))
        }
    }

    /// Accepts every signature, to isolate the check a test is about.
    struct AcceptAll;

    struct AcceptAllStream;

    impl ArtifactSignatureCheck for AcceptAllStream {
        fn update(&mut self, _chunk: &[u8]) {}

        fn finish(self: Box<Self>) -> Result<(), SignatureError> {
            Ok(())
        }
    }

    impl SignatureVerifier for AcceptAll {
        fn verify_manifest(
            &self,
            _manifest: &[u8],
            _signature: &str,
        ) -> Result<(), SignatureError> {
            Ok(())
        }

        fn begin_artifact(
            &self,
            _signature: &str,
        ) -> Result<Box<dyn ArtifactSignatureCheck>, SignatureError> {
            Ok(Box::new(AcceptAllStream))
        }
    }

    fn version(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    fn policy() -> UpdatePolicy<'static> {
        UpdatePolicy {
            hosts: AllowedHosts {
                artifact: ARTIFACT_HOST,
                release_page: PAGE_HOST,
            },
            platform: PLATFORM,
            channel: Channel::Stable,
            installed_version: version("1.1.0"),
            highest_seen_version: None,
            allow_prerelease: false,
        }
    }

    fn good_manifest() -> Value {
        json!({
            "schemaVersion": 1,
            "channel": "stable",
            "version": "1.2.0",
            "publishedAt": "2026-11-24T00:00:00Z",
            "notesUrl": format!("https://{PAGE_HOST}/notes/1.2.0"),
            "minAppVersion": "1.0.0",
            "minDriverVersion": "0.1.0",
            "withdrawn": false,
            "artifacts": [{
                "platform": PLATFORM,
                "url": format!("https://{ARTIFACT_HOST}/audiorouter-1.2.0-setup.exe"),
                "sizeBytes": INSTALLER.len(),
                "sha256": sha_of(INSTALLER),
                "signature": marker(INSTALLER),
            }],
        })
    }

    fn bytes(manifest: &Value) -> Vec<u8> {
        serde_json::to_vec(manifest).unwrap()
    }

    /// Evaluates a manifest the marker verifier signs correctly.
    fn evaluate_signed(
        manifest: &Value,
        policy: &UpdatePolicy<'_>,
    ) -> Result<AcceptedUpdate, UpdateRefusal> {
        let raw = bytes(manifest);
        evaluate_manifest(&raw, &marker(&raw), policy, &MarkerVerifier::default())
    }

    fn accepted() -> AcceptedUpdate {
        evaluate_signed(&good_manifest(), &policy()).unwrap()
    }

    fn mutated(mutate: impl FnOnce(&mut Value)) -> Value {
        let mut manifest = good_manifest();
        mutate(&mut manifest);
        manifest
    }

    // ---- versions ----

    #[test]
    fn version_parser_accepts_the_shell_version() {
        let own = Version::parse(env!("CARGO_PKG_VERSION")).unwrap();
        assert!(!own.is_prerelease());
    }

    #[test]
    fn version_parser_rejects_malformed_text() {
        for text in [
            "",
            "1",
            "1.2",
            "1.2.3.4",
            "01.2.3",
            "1.02.3",
            "1.2.03",
            "a.b.c",
            "1.2.3-",
            "1.2.3-a..b",
            "1.2.3+build",
            "1.2.3-rc_1",
            " 1.2.3",
            "1.2.3 ",
            "-1.2.3",
            "1.2.3-01",
            "99999999999.0.0",
            "1.2.3-rc.é",
        ] {
            assert_eq!(Version::parse(text), Err(VersionError), "{text:?}");
        }
        assert_eq!(Version::parse(&"1".repeat(65)), Err(VersionError));
    }

    #[test]
    fn version_order_follows_semantic_precedence() {
        let ascending = [
            "0.0.9",
            "0.1.0",
            "1.0.0-alpha",
            "1.0.0-alpha.1",
            "1.0.0-alpha.beta",
            "1.0.0-beta",
            "1.0.0-beta.2",
            "1.0.0-beta.11",
            "1.0.0-rc.1",
            "1.0.0",
            "1.0.1",
            "1.10.0",
            "2.0.0",
        ];
        for pair in ascending.windows(2) {
            assert!(
                version(pair[0]) < version(pair[1]),
                "{} < {}",
                pair[0],
                pair[1]
            );
        }
        assert_eq!(version("1.2.3"), version("1.2.3"));
        assert!(version("1.0.0-rc.1").is_prerelease());
    }

    // ---- URLs and hosts ----

    #[test]
    fn url_check_accepts_only_https_on_the_allowed_host() {
        assert_eq!(
            check_https_url("https://updates.example.invalid/a.exe", ARTIFACT_HOST),
            Ok(())
        );
        assert_eq!(
            check_https_url("https://UPDATES.example.invalid/a.exe?x=1", ARTIFACT_HOST),
            Ok(())
        );
        let cases = [
            ("http://updates.example.invalid/a.exe", UrlRefusal::NotHttps),
            ("ftp://updates.example.invalid/a.exe", UrlRefusal::NotHttps),
            ("//updates.example.invalid/a.exe", UrlRefusal::NotHttps),
            ("https://other.example.invalid/a.exe", UrlRefusal::WrongHost),
            (
                "https://updates.example.invalid.evil.invalid/a.exe",
                UrlRefusal::WrongHost,
            ),
            (
                "https://evil.invalid/updates.example.invalid/a.exe",
                UrlRefusal::WrongHost,
            ),
            (
                "https://updates.example.invalid:8443/a.exe",
                UrlRefusal::WrongHost,
            ),
            (
                "https://updates.example.invalid./a.exe",
                UrlRefusal::WrongHost,
            ),
            (
                "https://user@updates.example.invalid/a.exe",
                UrlRefusal::Credentials,
            ),
            (
                "https://updates.example.invalid@evil.invalid/a.exe",
                UrlRefusal::Credentials,
            ),
            ("https://updates.example.invalid", UrlRefusal::MissingPath),
            (
                "https://updates.example.invalid?x=1",
                UrlRefusal::MissingPath,
            ),
            (
                "https://updates.example.invalid/a.exe#frag",
                UrlRefusal::Fragment,
            ),
            (
                "https://updates.example.invalid/a b.exe",
                UrlRefusal::ForbiddenCharacter,
            ),
            (
                "https://updates.example.invalid/a\\b.exe",
                UrlRefusal::ForbiddenCharacter,
            ),
            (
                "https://updates.example.invalid/a\n.exe",
                UrlRefusal::ForbiddenCharacter,
            ),
            (
                "https://updates.example.invalid/é.exe",
                UrlRefusal::ForbiddenCharacter,
            ),
        ];
        for (url, expected) in cases {
            assert_eq!(
                check_https_url(url, ARTIFACT_HOST),
                Err(expected),
                "{url:?}"
            );
        }
        let long = format!("https://{ARTIFACT_HOST}/{}", "a".repeat(MAX_URL_BYTES));
        assert_eq!(
            check_https_url(&long, ARTIFACT_HOST),
            Err(UrlRefusal::TooLong)
        );
    }

    #[test]
    fn host_validation_rejects_anything_but_lowercase_dns_labels() {
        for host in [
            "",
            "UPPER.example",
            "a b.example",
            "a..example",
            ".example",
            "example.",
            "-a.example",
            "a-.example",
            "host:443",
            "user@host",
            "ex_ample.invalid",
            "é.example",
        ] {
            assert!(!host_is_valid(host), "{host:?}");
        }
        assert!(host_is_valid("updates.example.invalid"));
        assert!(!host_is_valid(&"a".repeat(MAX_HOST_BYTES + 1)));
    }

    #[test]
    fn an_invalid_policy_refuses_before_looking_at_the_manifest() {
        let raw = bytes(&good_manifest());
        let mut bad = policy();
        bad.hosts.artifact = "";
        assert_eq!(
            evaluate_manifest(&raw, &marker(&raw), &bad, &MarkerVerifier::default()),
            Err(UpdateRefusal::InvalidPolicy("artifact host"))
        );
        let mut bad = policy();
        bad.hosts.release_page = "Not A Host";
        assert_eq!(
            evaluate_manifest(&raw, &marker(&raw), &bad, &MarkerVerifier::default()),
            Err(UpdateRefusal::InvalidPolicy("release page host"))
        );
        let mut bad = policy();
        bad.platform = "Windows x64";
        assert_eq!(
            evaluate_manifest(&raw, &marker(&raw), &bad, &MarkerVerifier::default()),
            Err(UpdateRefusal::InvalidPolicy("platform"))
        );
    }

    // ---- manifest: accept ----

    #[test]
    fn a_good_manifest_is_accepted_and_selects_this_platforms_artifact() {
        let update = accepted();
        assert_eq!(update.version, version("1.2.0"));
        assert_eq!(update.channel, Channel::Stable);
        assert_eq!(update.min_app_version, version("1.0.0"));
        assert_eq!(update.min_driver_version, version("0.1.0"));
        assert_eq!(update.notes_url, format!("https://{PAGE_HOST}/notes/1.2.0"));
        assert_eq!(
            update.artifact.url(),
            format!("https://{ARTIFACT_HOST}/audiorouter-1.2.0-setup.exe")
        );
        assert_eq!(update.artifact.size_bytes(), INSTALLER.len() as u64);
        assert_eq!(hex(update.artifact.sha256()), sha_of(INSTALLER));
    }

    #[test]
    fn other_platforms_are_ignored_and_the_matching_one_is_chosen() {
        let manifest = mutated(|m| {
            let mut other = m["artifacts"][0].clone();
            other["platform"] = json!("linux-x86_64");
            other["url"] = json!(format!("https://{ARTIFACT_HOST}/other.bin"));
            m["artifacts"].as_array_mut().unwrap().insert(0, other);
        });
        let update = evaluate_signed(&manifest, &policy()).unwrap();
        assert!(update.artifact.url().ends_with("setup.exe"));
    }

    #[test]
    fn a_newer_prerelease_is_accepted_only_when_allowed_on_a_matching_channel() {
        let manifest = mutated(|m| {
            m["channel"] = json!("prerelease");
            m["version"] = json!("1.2.0-rc.1");
        });
        let mut allowed = policy();
        allowed.channel = Channel::Prerelease;
        allowed.allow_prerelease = true;
        assert_eq!(
            evaluate_signed(&manifest, &allowed).unwrap().version,
            version("1.2.0-rc.1")
        );
    }

    #[test]
    fn a_prerelease_installation_may_move_to_its_release() {
        let mut current = policy();
        current.installed_version = version("1.2.0-rc.1");
        assert_eq!(
            evaluate_signed(&good_manifest(), &current).unwrap().version,
            version("1.2.0")
        );
    }

    #[test]
    fn a_manifest_equal_to_the_highest_seen_version_is_not_a_replay() {
        let mut current = policy();
        current.highest_seen_version = Some(version("1.2.0"));
        assert!(evaluate_signed(&good_manifest(), &current).is_ok());
    }

    // ---- manifest: refuse ----

    #[test]
    fn with_no_trusted_key_every_manifest_path_refuses() {
        let good = bytes(&good_manifest());
        let attempts: Vec<(Vec<u8>, String)> = vec![
            (good.clone(), marker(&good)),
            (good.clone(), "anything".to_owned()),
            (b"not json".to_vec(), "anything".to_owned()),
            (Vec::new(), "anything".to_owned()),
            (
                bytes(&mutated(|m| m["withdrawn"] = json!(true))),
                "anything".to_owned(),
            ),
        ];
        for (raw, signature) in attempts {
            assert_eq!(
                evaluate_manifest(&raw, &signature, &policy(), &NoTrustedKey),
                Err(UpdateRefusal::Signature(SignatureError::NoTrustedKey))
            );
        }
        // An oversized or unsigned manifest is refused before the verifier too.
        assert_eq!(
            evaluate_manifest(&good, "", &policy(), &NoTrustedKey),
            Err(UpdateRefusal::Signature(SignatureError::Malformed))
        );
        let huge = vec![b' '; MAX_MANIFEST_BYTES + 1];
        assert_eq!(
            evaluate_manifest(&huge, "sig", &policy(), &NoTrustedKey),
            Err(UpdateRefusal::ManifestTooLarge)
        );
    }

    #[test]
    fn with_no_trusted_key_no_download_can_begin() {
        let plan = accepted().artifact;
        assert_eq!(
            ArtifactVerification::begin(&plan, &NoTrustedKey).err(),
            Some(ArtifactRefusal::Signature(SignatureError::NoTrustedKey))
        );
        assert_eq!(
            NoTrustedKey.begin_artifact("sig").err(),
            Some(SignatureError::NoTrustedKey)
        );
        assert_eq!(
            NoTrustedKey.verify_manifest(b"{}", "sig"),
            Err(SignatureError::NoTrustedKey)
        );
    }

    #[test]
    fn an_unsigned_or_wrongly_signed_manifest_is_refused() {
        let raw = bytes(&good_manifest());
        let verifier = MarkerVerifier::default();
        assert_eq!(
            evaluate_manifest(&raw, "", &policy(), &verifier),
            Err(UpdateRefusal::Signature(SignatureError::Malformed))
        );
        assert_eq!(
            evaluate_manifest(&raw, "bad\u{7}control", &policy(), &verifier),
            Err(UpdateRefusal::Signature(SignatureError::Malformed))
        );
        assert_eq!(
            evaluate_manifest(
                &raw,
                &"x".repeat(MAX_SIGNATURE_BYTES + 1),
                &policy(),
                &verifier
            ),
            Err(UpdateRefusal::Signature(SignatureError::Malformed))
        );
        assert_eq!(
            evaluate_manifest(&raw, "test-marker-0-0", &policy(), &verifier),
            Err(UpdateRefusal::Signature(SignatureError::Invalid))
        );
    }

    #[test]
    fn a_tampered_manifest_fails_its_signature() {
        let raw = bytes(&good_manifest());
        let signature = marker(&raw);
        let mut tampered = raw.clone();
        let position = tampered.iter().position(|b| *b == b'2').unwrap();
        tampered[position] = b'3';
        assert_eq!(tampered.len(), raw.len());
        assert_eq!(
            evaluate_manifest(&tampered, &signature, &policy(), &MarkerVerifier::default()),
            Err(UpdateRefusal::Signature(SignatureError::Invalid))
        );
    }

    #[test]
    fn the_signature_is_checked_before_parsing_and_after_the_size_bound() {
        let calls = Rc::new(Cell::new(0));
        let verifier = MarkerVerifier {
            manifest_calls: Rc::clone(&calls),
        };
        // Garbage with a valid marker reaches the parser, so it is "malformed".
        let garbage = b"not json at all".to_vec();
        assert_eq!(
            evaluate_manifest(&garbage, &marker(&garbage), &policy(), &verifier),
            Err(UpdateRefusal::Malformed)
        );
        assert_eq!(calls.get(), 1);
        // Garbage with a bad signature never reaches the parser.
        assert_eq!(
            evaluate_manifest(&garbage, "wrong", &policy(), &verifier),
            Err(UpdateRefusal::Signature(SignatureError::Invalid))
        );
        // An oversized manifest never reaches the verifier at all.
        let before = calls.get();
        let huge = vec![b' '; MAX_MANIFEST_BYTES + 1];
        assert_eq!(
            evaluate_manifest(&huge, &marker(&huge), &policy(), &verifier),
            Err(UpdateRefusal::ManifestTooLarge)
        );
        assert_eq!(calls.get(), before);
        // A manifest exactly at the cap is still read.
        let at_cap = vec![b' '; MAX_MANIFEST_BYTES];
        assert_eq!(
            evaluate_manifest(&at_cap, &marker(&at_cap), &policy(), &verifier),
            Err(UpdateRefusal::Malformed)
        );
    }

    #[test]
    fn unknown_missing_duplicate_and_mistyped_fields_are_malformed() {
        let valid = good_manifest();
        let mut cases: Vec<Vec<u8>> = Vec::new();
        cases.push(bytes(&mutated(|m| m["surprise"] = json!(true))));
        cases.push(bytes(&mutated(|m| m["artifacts"][0]["extra"] = json!(1))));
        for key in [
            "schemaVersion",
            "channel",
            "version",
            "publishedAt",
            "notesUrl",
            "minAppVersion",
            "minDriverVersion",
            "withdrawn",
            "artifacts",
        ] {
            let mut missing = valid.clone();
            missing.as_object_mut().unwrap().remove(key);
            cases.push(bytes(&missing));
        }
        for key in ["platform", "url", "sizeBytes", "sha256", "signature"] {
            let mut missing = valid.clone();
            missing["artifacts"][0].as_object_mut().unwrap().remove(key);
            cases.push(bytes(&missing));
        }
        cases.push(bytes(&mutated(|m| m["version"] = json!(12))));
        cases.push(bytes(&mutated(|m| m["withdrawn"] = json!("no"))));
        cases.push(bytes(&mutated(|m| m["channel"] = json!("nightly"))));
        cases.push(bytes(&mutated(|m| {
            m["artifacts"][0]["sizeBytes"] = json!(-1)
        })));
        cases.push(bytes(&mutated(|m| {
            m["artifacts"][0]["sizeBytes"] = json!(1.5)
        })));
        cases.push(b"[]".to_vec());
        cases.push(b"null".to_vec());
        cases.push(b"".to_vec());
        // A repeated key must not let a later value override the checked one.
        let mut duplicate = bytes(&valid);
        duplicate.truncate(duplicate.len() - 1);
        duplicate.extend_from_slice(br#","version":"9.9.9"}"#);
        cases.push(duplicate);
        for raw in cases {
            assert_eq!(
                evaluate_manifest(&raw, &marker(&raw), &policy(), &MarkerVerifier::default()),
                Err(UpdateRefusal::Malformed),
                "{}",
                String::from_utf8_lossy(&raw)
            );
        }
    }

    #[test]
    fn an_unsupported_schema_version_is_refused() {
        for schema in [0, 2, 99] {
            let manifest = mutated(|m| m["schemaVersion"] = json!(schema));
            assert_eq!(
                evaluate_signed(&manifest, &policy()),
                Err(UpdateRefusal::UnsupportedSchema)
            );
        }
    }

    #[test]
    fn the_kill_switch_refuses_an_otherwise_good_manifest() {
        let manifest = mutated(|m| m["withdrawn"] = json!(true));
        assert_eq!(
            evaluate_signed(&manifest, &policy()),
            Err(UpdateRefusal::Withdrawn)
        );
    }

    #[test]
    fn a_manifest_for_another_channel_is_refused() {
        let manifest = mutated(|m| m["channel"] = json!("prerelease"));
        assert_eq!(
            evaluate_signed(&manifest, &policy()),
            Err(UpdateRefusal::ChannelMismatch)
        );
        let mut beta = policy();
        beta.channel = Channel::Prerelease;
        assert_eq!(
            evaluate_signed(&good_manifest(), &beta),
            Err(UpdateRefusal::ChannelMismatch)
        );
    }

    #[test]
    fn a_manifest_without_this_platforms_artifact_is_refused() {
        let manifest = mutated(|m| m["artifacts"][0]["platform"] = json!("windows-aarch64"));
        assert_eq!(
            evaluate_signed(&manifest, &policy()),
            Err(UpdateRefusal::NoArtifactForPlatform)
        );
    }

    #[test]
    fn two_artifacts_for_one_platform_are_ambiguous() {
        let manifest = mutated(|m| {
            let copy = m["artifacts"][0].clone();
            m["artifacts"].as_array_mut().unwrap().push(copy);
        });
        assert_eq!(
            evaluate_signed(&manifest, &policy()),
            Err(UpdateRefusal::AmbiguousArtifact)
        );
    }

    #[test]
    fn the_same_or_an_older_version_is_never_offered() {
        for offered in ["1.1.0", "1.0.9", "0.9.0", "1.1.0-rc.1"] {
            let manifest = mutated(|m| {
                m["version"] = json!(offered);
                m["minAppVersion"] = json!("0.0.1");
            });
            let mut allow = policy();
            allow.allow_prerelease = true;
            assert_eq!(
                evaluate_signed(&manifest, &allow),
                Err(UpdateRefusal::NotNewer),
                "{offered}"
            );
        }
    }

    #[test]
    fn an_older_manifest_than_one_already_seen_is_a_replay() {
        let mut current = policy();
        current.highest_seen_version = Some(version("1.3.0"));
        assert_eq!(
            evaluate_signed(&good_manifest(), &current),
            Err(UpdateRefusal::Replayed)
        );
    }

    #[test]
    fn a_prerelease_is_refused_unless_allowed() {
        let manifest = mutated(|m| m["version"] = json!("1.2.0-rc.1"));
        assert_eq!(
            evaluate_signed(&manifest, &policy()),
            Err(UpdateRefusal::PrereleaseNotAllowed)
        );
        let mut allowed = policy();
        allowed.allow_prerelease = true;
        assert!(evaluate_signed(&manifest, &allowed).is_ok());
    }

    #[test]
    fn an_app_older_than_the_minimum_is_refused() {
        let manifest = mutated(|m| m["minAppVersion"] = json!("1.1.5"));
        assert_eq!(
            evaluate_signed(&manifest, &policy()),
            Err(UpdateRefusal::AppTooOld)
        );
        let at_minimum = mutated(|m| m["minAppVersion"] = json!("1.1.0"));
        assert!(evaluate_signed(&at_minimum, &policy()).is_ok());
    }

    #[test]
    fn a_minimum_app_version_above_the_offered_version_is_malformed() {
        let manifest = mutated(|m| m["minAppVersion"] = json!("9.0.0"));
        assert_eq!(
            evaluate_signed(&manifest, &policy()),
            Err(UpdateRefusal::BadField("minAppVersion"))
        );
    }

    #[test]
    fn bad_versions_and_text_fields_are_refused() {
        let cases: Vec<(&'static str, Value)> = vec![
            ("version", json!("v1.2.0")),
            ("version", json!("1.2")),
            ("minAppVersion", json!("")),
            ("minDriverVersion", json!("x.y.z")),
            ("publishedAt", json!("")),
            ("publishedAt", json!("a".repeat(MAX_FIELD_BYTES + 1))),
            ("publishedAt", json!("has space")),
        ];
        for (field, value) in cases {
            let manifest = mutated(|m| m[field] = value.clone());
            assert_eq!(
                evaluate_signed(&manifest, &policy()),
                Err(UpdateRefusal::BadField(field)),
                "{field} = {value}"
            );
        }
    }

    #[test]
    fn urls_off_the_allowed_hosts_or_over_http_are_refused() {
        let notes = mutated(|m| m["notesUrl"] = json!(format!("http://{PAGE_HOST}/n")));
        assert_eq!(
            evaluate_signed(&notes, &policy()),
            Err(UpdateRefusal::BadUrl("notesUrl", UrlRefusal::NotHttps))
        );
        // The release page and the artifact have separate allowed hosts.
        let notes_on_feed =
            mutated(|m| m["notesUrl"] = json!(format!("https://{ARTIFACT_HOST}/n")));
        assert_eq!(
            evaluate_signed(&notes_on_feed, &policy()),
            Err(UpdateRefusal::BadUrl("notesUrl", UrlRefusal::WrongHost))
        );
        let installer = mutated(|m| {
            m["artifacts"][0]["url"] = json!("https://evil.invalid/audiorouter-setup.exe");
        });
        assert_eq!(
            evaluate_signed(&installer, &policy()),
            Err(UpdateRefusal::BadUrl("url", UrlRefusal::WrongHost))
        );
        let credentials = mutated(|m| {
            m["artifacts"][0]["url"] = json!(format!("https://{ARTIFACT_HOST}@evil.invalid/a.exe"));
        });
        assert_eq!(
            evaluate_signed(&credentials, &policy()),
            Err(UpdateRefusal::BadUrl("url", UrlRefusal::Credentials))
        );
        // One caller value may serve both roles.
        let mut same_host = policy();
        same_host.hosts.release_page = ARTIFACT_HOST;
        let single = mutated(|m| m["notesUrl"] = json!(format!("https://{ARTIFACT_HOST}/n")));
        assert!(evaluate_signed(&single, &same_host).is_ok());
    }

    #[test]
    fn bad_artifact_fields_are_refused() {
        let cases: Vec<(&'static str, Value)> = vec![
            ("sizeBytes", json!(0)),
            ("sizeBytes", json!(MAX_ARTIFACT_BYTES + 1)),
            ("sha256", json!("abc")),
            ("sha256", json!("A".repeat(64))),
            ("sha256", json!("g".repeat(64))),
            ("sha256", json!("é".repeat(32))),
            ("signature", json!("")),
            ("signature", json!("tab\there")),
            ("platform", json!("Windows")),
        ];
        for (field, value) in cases {
            let manifest = mutated(|m| m["artifacts"][0][field] = value.clone());
            assert_eq!(
                evaluate_signed(&manifest, &policy()),
                Err(UpdateRefusal::BadField(field)),
                "{field} = {value}"
            );
        }
        let none = mutated(|m| m["artifacts"] = json!([]));
        assert_eq!(
            evaluate_signed(&none, &policy()),
            Err(UpdateRefusal::BadField("artifacts"))
        );
        let many = mutated(|m| {
            let one = m["artifacts"][0].clone();
            m["artifacts"] = Value::Array(vec![one; MAX_ARTIFACTS + 1]);
        });
        assert_eq!(
            evaluate_signed(&many, &policy()),
            Err(UpdateRefusal::BadField("artifacts"))
        );
        // A bad entry for another platform still refuses: fail closed.
        let other_bad = mutated(|m| {
            let mut other = m["artifacts"][0].clone();
            other["platform"] = json!("linux-x86_64");
            other["sha256"] = json!("nope");
            m["artifacts"].as_array_mut().unwrap().push(other);
        });
        assert_eq!(
            evaluate_signed(&other_bad, &policy()),
            Err(UpdateRefusal::BadField("sha256"))
        );
    }

    // ---- streamed artifact ----

    fn begin(plan: &ArtifactPlan) -> ArtifactVerification {
        ArtifactVerification::begin(plan, &MarkerVerifier::default()).unwrap()
    }

    #[test]
    fn a_correct_download_in_any_chunking_is_verified() {
        let plan = accepted().artifact;
        for chunk_size in [1, 3, 7, INSTALLER.len()] {
            let mut stream = begin(&plan);
            for chunk in INSTALLER.chunks(chunk_size) {
                stream.push(chunk).unwrap();
            }
            let verified = stream.finish().unwrap();
            assert_eq!(verified.size_bytes(), INSTALLER.len() as u64);
            assert_eq!(hex(verified.sha256()), sha_of(INSTALLER));
        }
    }

    #[test]
    fn a_download_with_a_different_hash_is_refused() {
        // Same length, accepting verifier: only the hash check can refuse.
        let plan = accepted().artifact;
        let mut tampered = INSTALLER.to_vec();
        tampered[0] ^= 1;
        let mut stream = ArtifactVerification::begin(&plan, &AcceptAll).unwrap();
        stream.push(&tampered).unwrap();
        assert_eq!(stream.finish().err(), Some(ArtifactRefusal::HashMismatch));
    }

    #[test]
    fn a_download_signed_for_other_bytes_is_refused() {
        // The hash is right, the signature is for something else.
        let manifest = mutated(|m| m["artifacts"][0]["signature"] = json!(marker(b"another file")));
        let plan = evaluate_signed(&manifest, &policy()).unwrap().artifact;
        let mut stream = begin(&plan);
        stream.push(INSTALLER).unwrap();
        assert_eq!(
            stream.finish().err(),
            Some(ArtifactRefusal::Signature(SignatureError::Invalid))
        );
    }

    #[test]
    fn a_download_with_the_wrong_size_is_refused() {
        let plan = accepted().artifact;
        let mut short = begin(&plan);
        short.push(&INSTALLER[..INSTALLER.len() - 1]).unwrap();
        assert_eq!(short.finish().err(), Some(ArtifactRefusal::SizeMismatch));
        let empty = begin(&plan);
        assert_eq!(empty.finish().err(), Some(ArtifactRefusal::SizeMismatch));
    }

    #[test]
    fn extra_bytes_stop_the_download_and_the_refusal_sticks() {
        let plan = accepted().artifact;
        let mut stream = begin(&plan);
        stream.push(INSTALLER).unwrap();
        assert_eq!(stream.push(b"x"), Err(ArtifactRefusal::TooLarge));
        assert_eq!(stream.push(b""), Err(ArtifactRefusal::TooLarge));
        assert_eq!(stream.finish().err(), Some(ArtifactRefusal::TooLarge));
        // A single oversized chunk is refused whole.
        let mut stream = begin(&plan);
        assert_eq!(
            stream.push(&vec![0u8; INSTALLER.len() + 1]),
            Err(ArtifactRefusal::TooLarge)
        );
    }

    #[test]
    fn a_failing_signature_check_refuses_even_with_a_correct_size_and_hash() {
        struct RefuseAtEnd;
        struct RefusingStream;
        impl ArtifactSignatureCheck for RefusingStream {
            fn update(&mut self, _chunk: &[u8]) {}
            fn finish(self: Box<Self>) -> Result<(), SignatureError> {
                Err(SignatureError::Invalid)
            }
        }
        impl SignatureVerifier for RefuseAtEnd {
            fn verify_manifest(&self, _m: &[u8], _s: &str) -> Result<(), SignatureError> {
                Ok(())
            }
            fn begin_artifact(
                &self,
                _signature: &str,
            ) -> Result<Box<dyn ArtifactSignatureCheck>, SignatureError> {
                Ok(Box::new(RefusingStream))
            }
        }
        let plan = accepted().artifact;
        let mut stream = ArtifactVerification::begin(&plan, &RefuseAtEnd).unwrap();
        stream.push(INSTALLER).unwrap();
        assert_eq!(
            stream.finish().err(),
            Some(ArtifactRefusal::Signature(SignatureError::Invalid))
        );
    }

    // ---- lifecycle ----

    fn idle() -> LifecycleState {
        LifecycleState {
            session: Activity::Idle,
            recording: Activity::Idle,
            driver_operation: Activity::Idle,
        }
    }

    #[test]
    fn install_is_allowed_only_when_everything_is_idle() {
        assert_eq!(idle().check_install(), Ok(()));
    }

    #[test]
    fn a_session_a_recording_or_a_driver_operation_blocks_install() {
        let mut state = idle();
        state.session = Activity::Active;
        assert_eq!(
            state.check_install(),
            Err(LifecycleRefusal::Active(Blocker::Session))
        );
        let mut state = idle();
        state.recording = Activity::Active;
        assert_eq!(
            state.check_install(),
            Err(LifecycleRefusal::Active(Blocker::Recording))
        );
        let mut state = idle();
        state.driver_operation = Activity::Active;
        assert_eq!(
            state.check_install(),
            Err(LifecycleRefusal::Active(Blocker::DriverOperation))
        );
    }

    #[test]
    fn unknown_activity_blocks_install_like_active_activity() {
        for (blocker, state) in [
            (
                Blocker::Session,
                LifecycleState {
                    session: Activity::Unknown,
                    ..idle()
                },
            ),
            (
                Blocker::Recording,
                LifecycleState {
                    recording: Activity::Unknown,
                    ..idle()
                },
            ),
            (
                Blocker::DriverOperation,
                LifecycleState {
                    driver_operation: Activity::Unknown,
                    ..idle()
                },
            ),
        ] {
            assert_eq!(
                state.check_install(),
                Err(LifecycleRefusal::Unknown(blocker))
            );
        }
    }

    #[test]
    fn the_most_serious_blocker_is_reported_first() {
        let everything = LifecycleState {
            session: Activity::Active,
            recording: Activity::Active,
            driver_operation: Activity::Active,
        };
        assert_eq!(
            everything.check_install(),
            Err(LifecycleRefusal::Active(Blocker::Recording))
        );
        let driver_and_session = LifecycleState {
            session: Activity::Active,
            driver_operation: Activity::Unknown,
            ..idle()
        };
        assert_eq!(
            driver_and_session.check_install(),
            Err(LifecycleRefusal::Unknown(Blocker::DriverOperation))
        );
    }

    // ---- the whole path ----

    #[test]
    fn the_default_verifier_refuses_the_whole_path_even_for_a_perfect_update() {
        let raw = bytes(&good_manifest());
        // The same inputs succeed with a verifier that has a marker check...
        assert!(
            evaluate_manifest(&raw, &marker(&raw), &policy(), &MarkerVerifier::default()).is_ok()
        );
        // ...and are refused at the first step with the default one.
        assert_eq!(
            evaluate_manifest(&raw, &marker(&raw), &policy(), &NoTrustedKey),
            Err(UpdateRefusal::Signature(SignatureError::NoTrustedKey))
        );
    }

    #[test]
    fn an_accepted_manifest_streams_to_a_verified_artifact_then_waits_for_an_idle_app() {
        let update = accepted();
        let mut stream = begin(&update.artifact);
        stream.push(INSTALLER).unwrap();
        let verified = stream.finish().unwrap();
        assert_eq!(verified.size_bytes(), update.artifact.size_bytes());
        let busy = LifecycleState {
            session: Activity::Active,
            ..idle()
        };
        assert!(busy.check_install().is_err());
        assert!(idle().check_install().is_ok());
    }
}
