//! Compatibility-floor verification status and bounded ELF version-requirement inspection.
//!
//! The compatibility floor is producer policy. This module owns the finite
//! status model that says what was actually done with a declared floor, and
//! the bounded reader that computes the highest `GLIBC_x.y` symbol version a
//! produced ELF requires.
//!
//! Two rules bound everything here:
//!
//! - native execution is never floor proof (ADR-0005 §9), so the status is
//!   computed from the produced bytes or from the toolchain binding, never
//!   from a smoke run;
//! - the inspection is read-only, bounded at every table and offset, and
//!   fails closed: an unreadable, malformed, truncated, or over-large
//!   candidate is never `Verified`.

use crate::{BuildStrategy, CompatibilityFloor, TargetPolicy};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Why a declared compatibility floor could not be independently verified.
///
/// The reason set is closed: it is a bounded enumeration, never a free-form
/// string, so a recorded status always says exactly why a declared floor is
/// not independently confirmed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FloorUnverifiedReason {
    /// The floor is declared and awaits independent inspection of the produced bytes.
    ///
    /// This is the plan-time disposition of a declared floor that the build
    /// toolchain cannot bind: the bytes have not been inspected yet.
    PendingInspection,
    /// The candidate is not an ELF image, or its bytes could not be read.
    UnreadableCandidate,
    /// The candidate is a truncated, malformed, or over-large ELF image.
    MalformedCandidate,
    /// The inspected candidate declares no glibc symbol version requirement.
    ///
    /// Such a candidate is compatible with any declared floor, but it does not
    /// *confirm* a floor, so it is recorded specifically rather than as a
    /// generic pass.
    NoVersionRequirement,
    /// The candidate carries a glibc version requirement that is not a plain
    /// numeric version, so it cannot be compared against the declared floor.
    UnrecognizedVersionRequirement,
    /// The declared floor kind has no independent byte-level verification.
    ///
    /// A macOS deployment floor would be read from the Mach-O `minos` field,
    /// which this milestone does not implement.
    FloorKindNotVerifiable,
}

/// The verification status of a declared compatibility floor.
///
/// This is the **single** producer-side floor status model. It is finite and
/// closed: it is neither a free-form string nor an `Option<bool>`, and every
/// variant maps to one concrete observation about the produced bytes or about
/// the toolchain that produced them.
///
/// | Status | Concrete observation |
/// |---|---|
/// | `NotDeclared` | the target declares `CompatibilityFloor::None` |
/// | `ToolchainBound` | the floor was bound into the build toolchain by the cargo-zigbuild triple rewrite, and the toolchain is trusted to honour it |
/// | `Verified` | the produced bytes were independently inspected and their highest required `GLIBC_x.y` version is not newer than the declared floor |
/// | `Unverified(reason)` | a floor was declared and the produced bytes did not confirm it, for the bounded reason given |
/// | `NotSatisfied` | the produced bytes were independently inspected and require a newer runtime than the declared floor |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FloorVerificationStatus {
    /// The target declares `CompatibilityFloor::None`.
    NotDeclared,
    /// The floor was bound into the build toolchain and the toolchain is
    /// trusted to honour it.
    ToolchainBound,
    /// The produced bytes were independently inspected and satisfy the declared floor.
    Verified,
    /// A floor was declared but could not be independently verified.
    Unverified(FloorUnverifiedReason),
    /// The produced bytes contradict the declared floor: they require a newer
    /// runtime than the floor declares.
    ///
    /// This is never reported as `Unverified`: a declared floor that the bytes
    /// disprove is false, so it fails the qualification for that target.
    NotSatisfied,
}

impl Default for FloorVerificationStatus {
    /// A document that omits the status is read as `NotDeclared`.
    ///
    /// The omission is then refused for any target that declares a floor, so
    /// a missing status fails closed instead of defaulting to a claim.
    fn default() -> Self {
        FloorVerificationStatus::NotDeclared
    }
}

/// The floor verification status a plan states for its declared floor.
///
/// The status is a pure function of the policy, so it is deterministic and
/// cannot be chosen by a caller: a plan either states the status its policy
/// implies or it is rejected.
pub fn planned_floor_status(policy: &TargetPolicy) -> FloorVerificationStatus {
    match policy.floor {
        CompatibilityFloor::None => FloorVerificationStatus::NotDeclared,
        CompatibilityFloor::Glibc { .. } => match policy.strategy {
            // The cargo-zigbuild triple rewrite binds the floor into the
            // toolchain; the toolchain is trusted to honour it.
            BuildStrategy::CargoZigbuild => FloorVerificationStatus::ToolchainBound,
            // A native build cannot bind the floor into the toolchain, so the
            // floor is only ever confirmed by inspecting the produced bytes.
            BuildStrategy::NativeCargo => {
                FloorVerificationStatus::Unverified(FloorUnverifiedReason::PendingInspection)
            }
        },
        // A macOS deployment floor is read from the Mach-O `minos` field,
        // which this milestone does not implement.
        CompatibilityFloor::Macos { .. } => {
            FloorVerificationStatus::Unverified(FloorUnverifiedReason::FloorKindNotVerifiable)
        }
    }
}

/// Whether a floor verification status is admissible for a declared floor and
/// build strategy.
///
/// This is the cross-check that keeps a status from claiming more than the
/// declaration allows: a target without a declared floor can only be
/// `NotDeclared`, a toolchain-bound floor can only be `ToolchainBound`, and
/// only an independently inspected floor can be `Verified` or `NotSatisfied`.
pub fn floor_status_admissible(
    floor: &CompatibilityFloor,
    strategy: BuildStrategy,
    status: FloorVerificationStatus,
) -> bool {
    match (floor, strategy) {
        (CompatibilityFloor::None, _) => status == FloorVerificationStatus::NotDeclared,
        (CompatibilityFloor::Glibc { .. }, BuildStrategy::CargoZigbuild) => {
            status == FloorVerificationStatus::ToolchainBound
        }
        (CompatibilityFloor::Glibc { .. }, BuildStrategy::NativeCargo) => matches!(
            status,
            FloorVerificationStatus::Verified
                | FloorVerificationStatus::Unverified(_)
                | FloorVerificationStatus::NotSatisfied
        ),
        (CompatibilityFloor::Macos { .. }, _) => {
            matches!(status, FloorVerificationStatus::Unverified(_))
        }
    }
}

/// The largest ELF header window read from one candidate.
const MAX_ELF_HEADER: usize = 64;
/// The largest number of ELF program headers read from one candidate.
const MAX_ELF_PROGRAM_HEADERS: usize = 256;
/// The largest number of dynamic entries read from one candidate.
const MAX_ELF_DYNAMIC_ENTRIES: usize = 1024;
/// The largest number of version-requirement entries read from one candidate.
const MAX_ELF_VERSION_REQUIREMENTS: usize = 256;
/// The largest number of auxiliary version entries read for one requirement.
const MAX_ELF_VERSION_AUXILIARIES: usize = 256;
/// The largest version name read from the dynamic string table.
const MAX_ELF_VERSION_NAME: usize = 64;

/// The `PT_LOAD` program header type.
const PT_LOAD: u32 = 1;
/// The `PT_DYNAMIC` program header type.
const PT_DYNAMIC: u32 = 2;
/// The dynamic tag that terminates the dynamic table.
const DT_NULL: u64 = 0;
/// The dynamic tag holding the address of the dynamic string table.
const DT_STRTAB: u64 = 5;
/// The dynamic tag holding the address of the version-requirement table.
const DT_VERNEED: u64 = 0x6ffffffe;
/// The dynamic tag holding the number of version-requirement entries.
const DT_VERNEEDNUM: u64 = 0x6fffffff;

/// Why the version-requirement reader could not determine a requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FloorReadError {
    /// The candidate is not an ELF image.
    NotAnElf,
    /// The candidate bytes could not be read.
    Unreadable,
    /// The candidate is truncated or structurally malformed.
    Malformed,
    /// The candidate exceeds a declared parsing bound.
    BoundExceeded,
}

/// A glibc symbol version requirement, compared component by component.
///
/// The patch component is zero for a two-component version: `GLIBC_2.17` is
/// `2.17.0`, and `GLIBC_2.2.5` is `2.2.5`, which is newer than `2.2.0` and
/// older than `2.17.0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct GlibcVersion {
    /// Major version.
    major: u16,
    /// Minor version.
    minor: u16,
    /// Patch version.
    patch: u16,
}

/// The parsed ELF header fields this reader needs.
struct ElfHeader {
    /// Whether the image is a 64-bit ELF.
    class64: bool,
    /// Whether the image stores integers little-endian.
    little_endian: bool,
    /// File offset of the program header table.
    program_header_offset: u64,
    /// Size of one program header entry.
    program_header_size: u16,
    /// Number of program header entries.
    program_header_count: u16,
}

/// One program header segment.
struct ProgramSegment {
    /// Segment type.
    kind: u32,
    /// File offset of the segment.
    offset: u64,
    /// Virtual address of the segment.
    vaddr: u64,
    /// Number of file-backed bytes in the segment.
    filesz: u64,
}

/// Read exactly `buffer.len()` bytes at `offset`.
///
/// Every read in this module is exact-length and every offset is checked
/// against the file length before the read happens.
fn read_exact_at(file: &mut File, offset: u64, buffer: &mut [u8]) -> Result<(), FloorReadError> {
    file.seek(SeekFrom::Start(offset))
        .map_err(|_| FloorReadError::Unreadable)?;
    match file.read_exact(buffer) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
            Err(FloorReadError::Malformed)
        }
        Err(_) => Err(FloorReadError::Unreadable),
    }
}

/// Read a `u16` from a fixed-width buffer.
fn u16_at(bytes: &[u8], offset: usize, little_endian: bool) -> u16 {
    let raw = [bytes[offset], bytes[offset + 1]];
    if little_endian {
        u16::from_le_bytes(raw)
    } else {
        u16::from_be_bytes(raw)
    }
}

/// Read a `u32` from a fixed-width buffer.
fn u32_at(bytes: &[u8], offset: usize, little_endian: bool) -> u32 {
    let raw = [
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ];
    if little_endian {
        u32::from_le_bytes(raw)
    } else {
        u32::from_be_bytes(raw)
    }
}

/// Read a `u64` from a fixed-width buffer.
fn u64_at(bytes: &[u8], offset: usize, little_endian: bool) -> u64 {
    let mut raw = [0u8; 8];
    raw.copy_from_slice(&bytes[offset..offset + 8]);
    if little_endian {
        u64::from_le_bytes(raw)
    } else {
        u64::from_be_bytes(raw)
    }
}

/// Parse the ELF header of one candidate.
fn parse_elf_header(file: &mut File, length: u64) -> Result<ElfHeader, FloorReadError> {
    let mut header = [0u8; MAX_ELF_HEADER];
    let wanted = length.min(MAX_ELF_HEADER as u64) as usize;
    file.read_exact(&mut header[..wanted])
        .map_err(|_| FloorReadError::Malformed)?;
    if wanted < 6 {
        return Err(FloorReadError::Malformed);
    }
    if &header[..4] != b"\x7fELF" {
        return Err(FloorReadError::NotAnElf);
    }
    let class64 = match header[4] {
        1 => false,
        2 => true,
        _ => return Err(FloorReadError::Malformed),
    };
    let little_endian = match header[5] {
        1 => true,
        2 => false,
        _ => return Err(FloorReadError::Malformed),
    };
    // The header must be complete for the class it claims.
    let minimum = if class64 { 64 } else { 52 };
    if length < minimum as u64 || wanted < minimum {
        return Err(FloorReadError::Malformed);
    }
    let (program_header_offset, program_header_size, program_header_count) = if class64 {
        (
            u64_at(&header, 32, little_endian),
            u16_at(&header, 54, little_endian),
            u16_at(&header, 56, little_endian),
        )
    } else {
        (
            u32_at(&header, 28, little_endian) as u64,
            u16_at(&header, 42, little_endian),
            u16_at(&header, 44, little_endian),
        )
    };
    Ok(ElfHeader {
        class64,
        little_endian,
        program_header_offset,
        program_header_size,
        program_header_count,
    })
}

/// Read the bounded program header table of one candidate.
fn read_program_segments(
    file: &mut File,
    header: &ElfHeader,
    length: u64,
) -> Result<Vec<ProgramSegment>, FloorReadError> {
    if header.program_header_count == 0 {
        return Ok(Vec::new());
    }
    // `0xffff` marks the extended numbering scheme, which this reader does not
    // implement; the real count lives in the section header table.
    if header.program_header_count == 0xffff {
        return Err(FloorReadError::Malformed);
    }
    if header.program_header_count as usize > MAX_ELF_PROGRAM_HEADERS {
        return Err(FloorReadError::BoundExceeded);
    }
    let expected_size = if header.class64 { 56 } else { 32 };
    if header.program_header_size != expected_size {
        return Err(FloorReadError::Malformed);
    }
    let count = header.program_header_count as u64;
    let table_end = count
        .checked_mul(header.program_header_size as u64)
        .and_then(|bytes| header.program_header_offset.checked_add(bytes))
        .ok_or(FloorReadError::Malformed)?;
    if table_end > length {
        return Err(FloorReadError::Malformed);
    }
    let mut segments = Vec::with_capacity(count as usize);
    for index in 0..count {
        let offset = header.program_header_offset + index * header.program_header_size as u64;
        let mut buffer = [0u8; 56];
        read_exact_at(file, offset, &mut buffer[..expected_size as usize])?;
        let segment = if header.class64 {
            ProgramSegment {
                kind: u32_at(&buffer, 0, header.little_endian),
                offset: u64_at(&buffer, 8, header.little_endian),
                vaddr: u64_at(&buffer, 16, header.little_endian),
                filesz: u64_at(&buffer, 32, header.little_endian),
            }
        } else {
            ProgramSegment {
                kind: u32_at(&buffer, 0, header.little_endian),
                offset: u32_at(&buffer, 4, header.little_endian) as u64,
                vaddr: u32_at(&buffer, 8, header.little_endian) as u64,
                filesz: u32_at(&buffer, 16, header.little_endian) as u64,
            }
        };
        segments.push(segment);
    }
    Ok(segments)
}

/// Read the bounded dynamic table of one candidate.
///
/// Returns the `(tag, value)` pairs in table order, stopping at the
/// terminating `DT_NULL` entry.
fn read_dynamic_entries(
    file: &mut File,
    header: &ElfHeader,
    dynamic: &ProgramSegment,
    length: u64,
) -> Result<Vec<(u64, u64)>, FloorReadError> {
    let entry_size = if header.class64 { 16 } else { 8 };
    let count = dynamic.filesz / entry_size as u64;
    if count > MAX_ELF_DYNAMIC_ENTRIES as u64 {
        return Err(FloorReadError::BoundExceeded);
    }
    let table_bytes = count * entry_size as u64;
    let table_end = dynamic
        .offset
        .checked_add(table_bytes)
        .ok_or(FloorReadError::Malformed)?;
    if table_end > length {
        return Err(FloorReadError::Malformed);
    }
    let mut buffer = vec![0u8; table_bytes as usize];
    read_exact_at(file, dynamic.offset, &mut buffer)?;
    let mut entries = Vec::new();
    for index in 0..count as usize {
        let base = index * entry_size;
        let (tag, value) = if header.class64 {
            (
                u64_at(&buffer, base, header.little_endian),
                u64_at(&buffer, base + 8, header.little_endian),
            )
        } else {
            (
                u32_at(&buffer, base, header.little_endian) as u64,
                u32_at(&buffer, base + 4, header.little_endian) as u64,
            )
        };
        if tag == DT_NULL {
            break;
        }
        entries.push((tag, value));
    }
    Ok(entries)
}

/// Map a virtual address to a file offset through the loadable segments.
fn map_vaddr(segments: &[ProgramSegment], vaddr: u64) -> Result<u64, FloorReadError> {
    for segment in segments.iter().filter(|segment| segment.kind == PT_LOAD) {
        let end = segment
            .vaddr
            .checked_add(segment.filesz)
            .ok_or(FloorReadError::Malformed)?;
        if vaddr >= segment.vaddr && vaddr < end {
            // The segment offset comes from the ELF program header and is not
            // bounded against the file length here, so the translation back
            // into the file is overflow-checked like every other offset
            // computation in this module.
            return segment
                .offset
                .checked_add(vaddr - segment.vaddr)
                .ok_or(FloorReadError::Malformed);
        }
    }
    Err(FloorReadError::Malformed)
}

/// Read one bounded version name from the dynamic string table.
fn read_version_name(
    file: &mut File,
    strtab_offset: u64,
    name_offset: u64,
    length: u64,
) -> Result<String, FloorReadError> {
    let start = strtab_offset
        .checked_add(name_offset)
        .ok_or(FloorReadError::Malformed)?;
    if start >= length {
        return Err(FloorReadError::Malformed);
    }
    // The name is read within the file, never past its end, and never for
    // more than the declared name bound.
    let available = (length - start).min(MAX_ELF_VERSION_NAME as u64) as usize;
    let mut buffer = [0u8; MAX_ELF_VERSION_NAME];
    read_exact_at(file, start, &mut buffer[..available])?;
    let end = buffer[..available]
        .iter()
        .position(|byte| *byte == 0)
        .ok_or(FloorReadError::Malformed)?;
    Ok(String::from_utf8_lossy(&buffer[..end]).into_owned())
}

/// How one symbol version name classifies against a glibc floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GlibcVersionName {
    /// Not a glibc requirement at all: the name carries no `GLIBC_` prefix.
    NotGlibc,
    /// A numeric glibc version requirement, comparable against a floor.
    Numeric(GlibcVersion),
    /// A `GLIBC_`-prefixed name that is not a plain numeric version.
    ///
    /// `GLIBC_PRIVATE` and `GLIBC_ABI_DT_RELR` are real and common. Such a
    /// name cannot be compared against a declared floor, but it does not
    /// invalidate the *other* requirements in the same image, so it is
    /// reported rather than treated as a reason to abandon the scan.
    Unrecognized,
}

/// Parse a symbol version name into a glibc version requirement.
///
/// A name that does not start with `GLIBC_` is not a glibc requirement. A
/// `GLIBC_` name that is not a plain numeric version cannot be compared
/// against a declared floor, so it is classified as
/// [`GlibcVersionName::Unrecognized`] rather than ignored — and, critically,
/// rather than aborting the scan and discarding every other requirement the
/// image carries.
fn parse_glibc_version(name: &str) -> GlibcVersionName {
    let Some(rest) = name.strip_prefix("GLIBC_") else {
        return GlibcVersionName::NotGlibc;
    };
    let parts: Vec<&str> = rest.split('.').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return GlibcVersionName::Unrecognized;
    }
    let mut numbers = [0u16; 3];
    for (index, part) in parts.iter().enumerate() {
        if part.is_empty() || part.len() > 5 || !part.bytes().all(|byte| byte.is_ascii_digit()) {
            return GlibcVersionName::Unrecognized;
        }
        let Ok(number) = part.parse::<u16>() else {
            return GlibcVersionName::Unrecognized;
        };
        numbers[index] = number;
    }
    GlibcVersionName::Numeric(GlibcVersion {
        major: numbers[0],
        minor: numbers[1],
        patch: numbers[2],
    })
}

/// The glibc version requirements observed in one ELF candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GlibcRequirements {
    /// The highest numeric version requirement observed, if any.
    highest: Option<GlibcVersion>,
    /// Whether a `GLIBC_`-prefixed name was observed that is not a plain
    /// numeric version.
    ///
    /// Such a name cannot be compared against a declared floor, so the floor
    /// cannot be *verified* from this image. It does not contradict the floor
    /// either, and it must not suppress the numeric requirements observed
    /// alongside it.
    unrecognized: bool,
}

/// Read the glibc version requirements of one ELF candidate.
///
/// The scan visits **every** requirement in the image. A `GLIBC_`-prefixed
/// name that is not a plain numeric version is recorded as unrecognized and
/// the scan continues, because abandoning it at the first such name would
/// discard every other requirement the image carries — including one that
/// contradicts the declared floor.
fn read_glibc_requirement(path: &Path) -> Result<GlibcRequirements, FloorReadError> {
    let mut file = File::open(path).map_err(|_| FloorReadError::Unreadable)?;
    let length = file
        .metadata()
        .map_err(|_| FloorReadError::Unreadable)?
        .len();
    let header = parse_elf_header(&mut file, length)?;
    let segments = read_program_segments(&mut file, &header, length)?;
    let Some(dynamic) = segments.iter().find(|segment| segment.kind == PT_DYNAMIC) else {
        return Ok(GlibcRequirements {
            highest: None,
            unrecognized: false,
        });
    };
    let entries = read_dynamic_entries(&mut file, &header, dynamic, length)?;
    let find = |tag: u64| {
        entries
            .iter()
            .find(|(entry, _)| *entry == tag)
            .map(|(_, value)| *value)
    };
    let Some(verneed_vaddr) = find(DT_VERNEED) else {
        return Ok(GlibcRequirements {
            highest: None,
            unrecognized: false,
        });
    };
    let strtab_vaddr = find(DT_STRTAB).ok_or(FloorReadError::Malformed)?;
    let requirement_count = find(DT_VERNEEDNUM);
    if requirement_count.is_some_and(|count| count > MAX_ELF_VERSION_REQUIREMENTS as u64) {
        return Err(FloorReadError::BoundExceeded);
    }
    let strtab_offset = map_vaddr(&segments, strtab_vaddr)?;
    let verneed_offset = map_vaddr(&segments, verneed_vaddr)?;
    if verneed_offset >= length {
        return Err(FloorReadError::Malformed);
    }

    let mut highest: Option<GlibcVersion> = None;
    let mut unrecognized = false;
    let mut requirement_offset = verneed_offset;
    let mut requirements = 0usize;
    loop {
        if requirements >= MAX_ELF_VERSION_REQUIREMENTS {
            return Err(FloorReadError::BoundExceeded);
        }
        let mut requirement = [0u8; 16];
        read_exact_at(&mut file, requirement_offset, &mut requirement)?;
        let auxiliary_count = u16_at(&requirement, 2, header.little_endian);
        let auxiliary_offset = u32_at(&requirement, 8, header.little_endian) as u64;
        let next_requirement = u32_at(&requirement, 12, header.little_endian) as u64;
        let mut auxiliary_offset = requirement_offset
            .checked_add(auxiliary_offset)
            .ok_or(FloorReadError::Malformed)?;
        for index in 0..auxiliary_count as usize {
            if index >= MAX_ELF_VERSION_AUXILIARIES {
                return Err(FloorReadError::BoundExceeded);
            }
            let mut auxiliary = [0u8; 16];
            read_exact_at(&mut file, auxiliary_offset, &mut auxiliary)?;
            let name_offset = u32_at(&auxiliary, 8, header.little_endian) as u64;
            let next_auxiliary = u32_at(&auxiliary, 12, header.little_endian) as u64;
            let name = read_version_name(&mut file, strtab_offset, name_offset, length)?;
            // Every requirement is visited. An unrecognized name is recorded
            // and the scan continues, so a requirement observed alongside it
            // still reaches the comparison.
            match parse_glibc_version(&name) {
                GlibcVersionName::Numeric(version) => {
                    if highest.is_none_or(|current| version > current) {
                        highest = Some(version);
                    }
                }
                GlibcVersionName::Unrecognized => unrecognized = true,
                GlibcVersionName::NotGlibc => {}
            }
            if next_auxiliary == 0 {
                break;
            }
            auxiliary_offset = auxiliary_offset
                .checked_add(next_auxiliary)
                .ok_or(FloorReadError::Malformed)?;
        }
        if next_requirement == 0 {
            break;
        }
        requirement_offset = requirement_offset
            .checked_add(next_requirement)
            .ok_or(FloorReadError::Malformed)?;
        requirements += 1;
    }
    Ok(GlibcRequirements {
        highest,
        unrecognized,
    })
}

/// Inspect one candidate's bytes and compute its floor verification status.
///
/// The inspection is read-only and holds no lock: two concurrent inspections
/// of the same candidate observe the same bytes or report a changed-file
/// error, and each observation is self-consistent.
pub(crate) fn inspect_candidate_floor(
    path: &Path,
    floor: &CompatibilityFloor,
    strategy: BuildStrategy,
) -> FloorVerificationStatus {
    match floor {
        CompatibilityFloor::None => FloorVerificationStatus::NotDeclared,
        CompatibilityFloor::Macos { .. } => {
            FloorVerificationStatus::Unverified(FloorUnverifiedReason::FloorKindNotVerifiable)
        }
        CompatibilityFloor::Glibc { major, minor } => match strategy {
            BuildStrategy::CargoZigbuild => FloorVerificationStatus::ToolchainBound,
            BuildStrategy::NativeCargo => {
                let declared = GlibcVersion {
                    major: *major,
                    minor: *minor,
                    patch: 0,
                };
                match read_glibc_requirement(path) {
                    // The comparison direction is the point: a binary that
                    // requires a *newer* glibc than declared does not satisfy
                    // the declared floor.
                    //
                    // A contradiction is decided **first** and is not
                    // downgraded by an unrecognized name observed alongside
                    // it: a declared floor that the bytes contradict is a
                    // hard failure, never an unverified status.
                    Ok(requirements) => {
                        if let Some(required) = requirements.highest {
                            if required > declared {
                                FloorVerificationStatus::NotSatisfied
                            } else if requirements.unrecognized {
                                // The numeric requirements are compatible with
                                // the declared floor, but one requirement could
                                // not be compared, so the floor is not proven.
                                FloorVerificationStatus::Unverified(
                                    FloorUnverifiedReason::UnrecognizedVersionRequirement,
                                )
                            } else {
                                FloorVerificationStatus::Verified
                            }
                        } else if requirements.unrecognized {
                            FloorVerificationStatus::Unverified(
                                FloorUnverifiedReason::UnrecognizedVersionRequirement,
                            )
                        } else {
                            FloorVerificationStatus::Unverified(
                                FloorUnverifiedReason::NoVersionRequirement,
                            )
                        }
                    }
                    Err(FloorReadError::NotAnElf | FloorReadError::Unreadable) => {
                        FloorVerificationStatus::Unverified(
                            FloorUnverifiedReason::UnreadableCandidate,
                        )
                    }
                    Err(FloorReadError::Malformed | FloorReadError::BoundExceeded) => {
                        FloorVerificationStatus::Unverified(
                            FloorUnverifiedReason::MalformedCandidate,
                        )
                    }
                }
            }
        },
    }
}

/// The severity of an unverified reason, used to combine candidate results.
fn reason_rank(reason: FloorUnverifiedReason) -> u8 {
    match reason {
        FloorUnverifiedReason::UnreadableCandidate => 5,
        FloorUnverifiedReason::MalformedCandidate => 4,
        FloorUnverifiedReason::UnrecognizedVersionRequirement => 3,
        FloorUnverifiedReason::NoVersionRequirement => 2,
        FloorUnverifiedReason::FloorKindNotVerifiable => 1,
        FloorUnverifiedReason::PendingInspection => 0,
    }
}

/// The severity of a floor status, used to combine candidate results.
fn status_rank(status: FloorVerificationStatus) -> (u8, u8) {
    match status {
        FloorVerificationStatus::NotSatisfied => (5, 0),
        FloorVerificationStatus::Unverified(reason) => (4, reason_rank(reason)),
        FloorVerificationStatus::Verified => (3, 0),
        FloorVerificationStatus::ToolchainBound => (2, 0),
        FloorVerificationStatus::NotDeclared => (1, 0),
    }
}

/// Combine the per-candidate floor statuses of one target into one status.
///
/// The combination is deterministic and conservative: a contradiction
/// dominates, an unverified reason dominates a positive claim, and the most
/// severe reason wins among unverified candidates.
pub(crate) fn aggregate_floor_status(
    statuses: &[FloorVerificationStatus],
) -> FloorVerificationStatus {
    let mut combined: Option<FloorVerificationStatus> = None;
    for status in statuses {
        combined = Some(match combined {
            None => *status,
            Some(current) => {
                if status_rank(*status) > status_rank(current) {
                    *status
                } else {
                    current
                }
            }
        });
    }
    combined.unwrap_or(FloorVerificationStatus::Unverified(
        FloorUnverifiedReason::UnreadableCandidate,
    ))
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::{DT_NULL, DT_STRTAB, DT_VERNEED, DT_VERNEEDNUM};

    /// Builds minimal ELF images that carry symbol version requirements.
    ///
    /// The image is deliberately small and hand-built: one `PT_LOAD` segment
    /// that maps the whole file, one `PT_DYNAMIC` segment, a dynamic table,
    /// a version-requirement table, and a string table.
    pub(crate) struct ElfBuilder {
        class64: bool,
        little_endian: bool,
        versions: Vec<String>,
        program_header_count: Option<u16>,
        dynamic_filesz: Option<u64>,
        program_header_offset: Option<u64>,
    }

    impl ElfBuilder {
        /// Start a new image for the given ELF class and byte order.
        pub(crate) fn new(class64: bool, little_endian: bool) -> Self {
            Self {
                class64,
                little_endian,
                versions: Vec::new(),
                program_header_count: None,
                dynamic_filesz: None,
                program_header_offset: None,
            }
        }

        /// Set the version names carried by the version-requirement table.
        pub(crate) fn versions(mut self, versions: &[&str]) -> Self {
            self.versions = versions.iter().map(|value| (*value).to_owned()).collect();
            self
        }

        /// Override the declared program header count.
        pub(crate) fn program_header_count(mut self, count: u16) -> Self {
            self.program_header_count = Some(count);
            self
        }

        /// Override the declared size of the dynamic segment.
        pub(crate) fn dynamic_filesz(mut self, filesz: u64) -> Self {
            self.dynamic_filesz = Some(filesz);
            self
        }

        /// Override the declared program header table offset.
        pub(crate) fn program_header_offset(mut self, offset: u64) -> Self {
            self.program_header_offset = Some(offset);
            self
        }

        fn put_u16(&self, bytes: &mut [u8], offset: usize, value: u16) {
            let raw = if self.little_endian {
                value.to_le_bytes()
            } else {
                value.to_be_bytes()
            };
            bytes[offset..offset + 2].copy_from_slice(&raw);
        }

        fn put_u32(&self, bytes: &mut [u8], offset: usize, value: u32) {
            let raw = if self.little_endian {
                value.to_le_bytes()
            } else {
                value.to_be_bytes()
            };
            bytes[offset..offset + 4].copy_from_slice(&raw);
        }

        fn put_u64(&self, bytes: &mut [u8], offset: usize, value: u64) {
            let raw = if self.little_endian {
                value.to_le_bytes()
            } else {
                value.to_be_bytes()
            };
            bytes[offset..offset + 8].copy_from_slice(&raw);
        }

        /// Build the image bytes.
        pub(crate) fn build(self) -> Vec<u8> {
            let header_size = if self.class64 { 64 } else { 52 };
            let program_header_size = if self.class64 { 56 } else { 32 };
            let dynamic_entry_size = if self.class64 { 16 } else { 8 };
            let dynamic_entries = if self.versions.is_empty() { 2 } else { 4 };

            let mut bytes = vec![0u8; header_size];
            bytes.resize(bytes.len() + 2 * program_header_size, 0);
            let dynamic_offset = bytes.len() as u64;
            bytes.resize(bytes.len() + dynamic_entries * dynamic_entry_size, 0);
            let verneed_offset = bytes.len() as u64;
            let verneed_size = if self.versions.is_empty() {
                0
            } else {
                16 * (1 + self.versions.len())
            };
            bytes.resize(bytes.len() + verneed_size, 0);
            let strtab_offset = bytes.len() as u64;

            let mut string_table = Vec::new();
            let mut name_offsets = Vec::new();
            for version in &self.versions {
                name_offsets.push(string_table.len() as u32);
                string_table.extend_from_slice(version.as_bytes());
                string_table.push(0);
            }
            bytes.extend_from_slice(&string_table);
            let total = bytes.len() as u64;

            // ELF identification.
            bytes[..4].copy_from_slice(b"\x7fELF");
            bytes[4] = if self.class64 { 2 } else { 1 };
            bytes[5] = if self.little_endian { 1 } else { 2 };
            bytes[6] = 1;
            self.put_u16(&mut bytes, 16, 2); // e_type: ET_EXEC
            self.put_u16(&mut bytes, 18, if self.class64 { 62 } else { 40 }); // e_machine
            self.put_u32(&mut bytes, 20, 1); // e_version
            let program_header_offset = self.program_header_offset.unwrap_or(header_size as u64);
            if self.class64 {
                self.put_u64(&mut bytes, 32, program_header_offset);
                self.put_u16(&mut bytes, 52, header_size as u16);
                self.put_u16(&mut bytes, 54, program_header_size as u16);
                self.put_u16(&mut bytes, 56, self.program_header_count.unwrap_or(2));
            } else {
                self.put_u32(&mut bytes, 28, program_header_offset as u32);
                self.put_u16(&mut bytes, 40, header_size as u16);
                self.put_u16(&mut bytes, 42, program_header_size as u16);
                self.put_u16(&mut bytes, 44, self.program_header_count.unwrap_or(2));
            }

            // Program headers: one loadable segment and one dynamic segment.
            let first = header_size;
            let second = header_size + program_header_size;
            let put_segment = |bytes: &mut Vec<u8>,
                               at: usize,
                               kind: u32,
                               offset: u64,
                               vaddr: u64,
                               filesz: u64| {
                if self.class64 {
                    self.put_u32(bytes, at, kind);
                    self.put_u32(bytes, at + 4, 4);
                    self.put_u64(bytes, at + 8, offset);
                    self.put_u64(bytes, at + 16, vaddr);
                    self.put_u64(bytes, at + 24, vaddr);
                    self.put_u64(bytes, at + 32, filesz);
                    self.put_u64(bytes, at + 40, filesz);
                    self.put_u64(bytes, at + 48, 0x1000);
                } else {
                    self.put_u32(bytes, at, kind);
                    self.put_u32(bytes, at + 4, offset as u32);
                    self.put_u32(bytes, at + 8, vaddr as u32);
                    self.put_u32(bytes, at + 12, vaddr as u32);
                    self.put_u32(bytes, at + 16, filesz as u32);
                    self.put_u32(bytes, at + 20, filesz as u32);
                    self.put_u32(bytes, at + 24, 4);
                    self.put_u32(bytes, at + 28, 0x1000);
                }
            };
            put_segment(&mut bytes, first, 1, 0, 0, total);
            let dynamic_size = (dynamic_entries * dynamic_entry_size) as u64;
            put_segment(
                &mut bytes,
                second,
                2,
                dynamic_offset,
                dynamic_offset,
                self.dynamic_filesz.unwrap_or(dynamic_size),
            );

            // Dynamic table.
            let dynamic_base = dynamic_offset as usize;
            let put_dynamic = |bytes: &mut Vec<u8>, index: usize, tag: u64, value: u64| {
                let at = dynamic_base + index * dynamic_entry_size;
                if self.class64 {
                    self.put_u64(bytes, at, tag);
                    self.put_u64(bytes, at + 8, value);
                } else {
                    self.put_u32(bytes, at, tag as u32);
                    self.put_u32(bytes, at + 4, value as u32);
                }
            };
            put_dynamic(&mut bytes, 0, DT_STRTAB, strtab_offset);
            if self.versions.is_empty() {
                put_dynamic(&mut bytes, 1, DT_NULL, 0);
            } else {
                put_dynamic(&mut bytes, 1, DT_VERNEED, verneed_offset);
                put_dynamic(&mut bytes, 2, DT_VERNEEDNUM, 1);
                put_dynamic(&mut bytes, 3, DT_NULL, 0);
            }

            // Version-requirement table: one requirement with one auxiliary
            // entry per version name.
            if !self.versions.is_empty() {
                let base = verneed_offset as usize;
                self.put_u16(&mut bytes, base, 1); // vn_version
                self.put_u16(&mut bytes, base + 2, self.versions.len() as u16); // vn_cnt
                self.put_u32(&mut bytes, base + 4, 0); // vn_file
                self.put_u32(&mut bytes, base + 8, 16); // vn_aux
                self.put_u32(&mut bytes, base + 12, 0); // vn_next
                for (index, name_offset) in name_offsets.iter().enumerate() {
                    let at = base + 16 + index * 16;
                    self.put_u32(&mut bytes, at, 0); // vna_hash
                    self.put_u16(&mut bytes, at + 4, 0); // vna_flags
                    self.put_u16(&mut bytes, at + 6, (index + 1) as u16); // vna_other
                    self.put_u32(&mut bytes, at + 8, *name_offset); // vna_name
                                                                    // The auxiliary entries form a chain: each one points at
                                                                    // the next, and the last one ends the chain.
                    self.put_u32(
                        &mut bytes,
                        at + 12,
                        if index + 1 < self.versions.len() {
                            16
                        } else {
                            0
                        },
                    ); // vna_next
                }
            }
            bytes
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::ElfBuilder;
    use super::*;
    use crate::{
        BuildStrategy, CompatibilityFloor, HostArch, HostOs, Qualification, SupportTier,
        TargetPolicy, ToolchainRequirement,
    };

    fn policy(floor: CompatibilityFloor, strategy: BuildStrategy) -> TargetPolicy {
        TargetPolicy {
            target: "x86_64-unknown-linux-gnu".into(),
            strategy,
            host_os: HostOs::Linux,
            host_arch: HostArch::X86_64,
            qualification_host: None,
            toolchain: ToolchainRequirement {
                rust: "1.89.0".into(),
                cargo_zigbuild: None,
                zig: None,
            },
            floor,
            qualification: Qualification::Native,
            support: SupportTier::Required,
        }
    }

    fn glibc(major: u16, minor: u16) -> CompatibilityFloor {
        CompatibilityFloor::Glibc { major, minor }
    }

    fn write(bytes: &[u8]) -> std::path::PathBuf {
        let root = crate::test_temp_dir("floor");
        let path = root.join("candidate");
        std::fs::write(&path, bytes).unwrap();
        path
    }

    fn status_for(bytes: &[u8], floor: CompatibilityFloor) -> FloorVerificationStatus {
        let path = write(bytes);
        let status = inspect_candidate_floor(&path, &floor, BuildStrategy::NativeCargo);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
        status
    }

    #[test]
    fn reader_reports_the_highest_required_glibc_version() {
        let image = ElfBuilder::new(true, true)
            .versions(&["GLIBC_2.17", "GLIBC_2.34", "GLIBC_2.2.5"])
            .build();
        let path = write(&image);
        let required = read_glibc_requirement(&path).unwrap();
        assert_eq!(
            required.highest,
            Some(GlibcVersion {
                major: 2,
                minor: 34,
                patch: 0
            })
        );
        assert!(!required.unrecognized);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn reader_reports_no_requirement_when_the_image_has_none() {
        // No version-requirement table at all.
        let image = ElfBuilder::new(true, true).build();
        let path = write(&image);
        let required = read_glibc_requirement(&path).unwrap();
        assert_eq!(required.highest, None);
        assert!(!required.unrecognized);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
        // A version requirement that is not a glibc requirement.
        let image = ElfBuilder::new(true, true)
            .versions(&["CXXABI_1.3"])
            .build();
        let path = write(&image);
        let required = read_glibc_requirement(&path).unwrap();
        assert_eq!(required.highest, None);
        assert!(!required.unrecognized);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn reader_reads_32_bit_and_big_endian_images() {
        let image = ElfBuilder::new(false, true)
            .versions(&["GLIBC_2.17"])
            .build();
        let path = write(&image);
        assert_eq!(
            read_glibc_requirement(&path).unwrap().highest,
            Some(GlibcVersion {
                major: 2,
                minor: 17,
                patch: 0
            })
        );
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();

        let image = ElfBuilder::new(true, false)
            .versions(&["GLIBC_2.17"])
            .build();
        let path = write(&image);
        assert_eq!(
            read_glibc_requirement(&path).unwrap().highest,
            Some(GlibcVersion {
                major: 2,
                minor: 17,
                patch: 0
            })
        );
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn reader_refuses_truncated_malformed_and_non_elf_inputs() {
        // Not an ELF image at all.
        let path = write(b"this is not an elf image");
        assert_eq!(read_glibc_requirement(&path), Err(FloorReadError::NotAnElf));
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();

        // A truncated dynamic table.
        let image = ElfBuilder::new(true, true)
            .versions(&["GLIBC_2.17"])
            .build();
        let path = write(&image[..image.len() - 8]);
        assert!(read_glibc_requirement(&path).is_err());
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();

        // Truncated program headers.
        let image = ElfBuilder::new(true, true)
            .versions(&["GLIBC_2.17"])
            .build();
        let path = write(&image[..80]);
        assert!(read_glibc_requirement(&path).is_err());
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();

        // A program header table offset beyond the file.
        let image = ElfBuilder::new(true, true)
            .versions(&["GLIBC_2.17"])
            .program_header_offset(1 << 30)
            .build();
        let path = write(&image);
        assert!(read_glibc_requirement(&path).is_err());
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn reader_refuses_oversized_tables_instead_of_parsing_them() {
        let image = ElfBuilder::new(true, true)
            .versions(&["GLIBC_2.17"])
            .program_header_count(4096)
            .build();
        let path = write(&image);
        assert_eq!(
            read_glibc_requirement(&path),
            Err(FloorReadError::BoundExceeded)
        );
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();

        let image = ElfBuilder::new(true, true)
            .versions(&["GLIBC_2.17"])
            .dynamic_filesz(1 << 20)
            .build();
        let path = write(&image);
        assert_eq!(
            read_glibc_requirement(&path),
            Err(FloorReadError::BoundExceeded)
        );
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn comparison_direction_requires_the_newest_version_to_fit_under_the_floor() {
        let newer = ElfBuilder::new(true, true)
            .versions(&["GLIBC_2.34"])
            .build();
        let equal = ElfBuilder::new(true, true)
            .versions(&["GLIBC_2.17"])
            .build();
        let older = ElfBuilder::new(true, true)
            .versions(&["GLIBC_2.2.5"])
            .build();
        assert_eq!(
            status_for(&newer, glibc(2, 17)),
            FloorVerificationStatus::NotSatisfied
        );
        assert_eq!(
            status_for(&equal, glibc(2, 17)),
            FloorVerificationStatus::Verified
        );
        assert_eq!(
            status_for(&older, glibc(2, 17)),
            FloorVerificationStatus::Verified
        );
        // A binary that requires an older glibc satisfies a newer floor.
        assert_eq!(
            status_for(&older, glibc(2, 34)),
            FloorVerificationStatus::Verified
        );
        // A patch component is a real requirement: 2.2.5 is newer than 2.2.
        assert_eq!(
            status_for(&older, glibc(2, 2)),
            FloorVerificationStatus::NotSatisfied
        );
    }

    #[test]
    fn an_image_without_a_version_requirement_is_recorded_specifically() {
        let image = ElfBuilder::new(true, true).build();
        assert_eq!(
            status_for(&image, glibc(2, 17)),
            FloorVerificationStatus::Unverified(FloorUnverifiedReason::NoVersionRequirement)
        );
    }

    #[test]
    fn a_malformed_candidate_is_never_verified() {
        assert_eq!(
            status_for(b"not an elf", glibc(2, 17)),
            FloorVerificationStatus::Unverified(FloorUnverifiedReason::UnreadableCandidate)
        );
        let image = ElfBuilder::new(true, true)
            .versions(&["GLIBC_2.17"])
            .build();
        assert_eq!(
            status_for(&image[..image.len() - 8], glibc(2, 17)),
            FloorVerificationStatus::Unverified(FloorUnverifiedReason::MalformedCandidate)
        );
    }

    #[test]
    fn an_unrecognized_version_name_is_never_verified() {
        let image = ElfBuilder::new(true, true)
            .versions(&["GLIBC_ABI_DT_RELR"])
            .build();
        assert_eq!(
            status_for(&image, glibc(2, 17)),
            FloorVerificationStatus::Unverified(
                FloorUnverifiedReason::UnrecognizedVersionRequirement
            )
        );
    }

    #[test]
    fn an_unrecognized_version_name_does_not_mask_a_contradicted_floor() {
        // `GLIBC_PRIVATE` and `GLIBC_ABI_DT_RELR` are real, common version
        // names. A binary that requires a glibc newer than the declared floor
        // must fail closed even when it also carries such a name: the
        // unrecognized name is recorded, but it must not discard the numeric
        // requirement observed alongside it.
        for extra in ["GLIBC_PRIVATE", "GLIBC_ABI_DT_RELR"] {
            let image = ElfBuilder::new(true, true)
                .versions(&["GLIBC_2.34", extra])
                .build();
            assert_eq!(
                status_for(&image, glibc(2, 17)),
                FloorVerificationStatus::NotSatisfied,
                "{extra} must not mask the contradicted floor"
            );
            // The same image against a floor it does satisfy is unverified,
            // not verified: one requirement could not be compared.
            assert_eq!(
                status_for(&image, glibc(2, 34)),
                FloorVerificationStatus::Unverified(
                    FloorUnverifiedReason::UnrecognizedVersionRequirement
                )
            );
        }
        // An unrecognized name alone is still unverified, never verified.
        let image = ElfBuilder::new(true, true)
            .versions(&["GLIBC_PRIVATE"])
            .build();
        assert_eq!(
            status_for(&image, glibc(2, 17)),
            FloorVerificationStatus::Unverified(
                FloorUnverifiedReason::UnrecognizedVersionRequirement
            )
        );
    }

    #[test]
    fn an_unrecognized_version_name_does_not_hide_a_numeric_requirement() {
        // The reader visits every requirement, so a numeric requirement that
        // appears *after* an unrecognized one is still observed.
        let image = ElfBuilder::new(true, true)
            .versions(&["GLIBC_PRIVATE", "GLIBC_2.34"])
            .build();
        let path = write(&image);
        let required = read_glibc_requirement(&path).unwrap();
        assert_eq!(
            required.highest,
            Some(GlibcVersion {
                major: 2,
                minor: 34,
                patch: 0
            })
        );
        assert!(required.unrecognized);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_virtual_address_translation_cannot_overflow() {
        // A crafted `PT_LOAD` whose file offset is near the top of the `u64`
        // range must not overflow when the virtual address is translated
        // back into the file. The segment offset comes from the ELF program
        // header and is not bounded against the file length.
        let segments = vec![ProgramSegment {
            kind: PT_LOAD,
            offset: u64::MAX - 10,
            vaddr: 0,
            filesz: 1024,
        }];
        assert_eq!(map_vaddr(&segments, 20), Err(FloorReadError::Malformed));
    }

    #[test]
    fn the_plan_status_is_derived_from_the_policy() {
        assert_eq!(
            planned_floor_status(&policy(
                CompatibilityFloor::None,
                BuildStrategy::NativeCargo
            )),
            FloorVerificationStatus::NotDeclared
        );
        assert_eq!(
            planned_floor_status(&policy(glibc(2, 17), BuildStrategy::CargoZigbuild)),
            FloorVerificationStatus::ToolchainBound
        );
        assert_eq!(
            planned_floor_status(&policy(glibc(2, 17), BuildStrategy::NativeCargo)),
            FloorVerificationStatus::Unverified(FloorUnverifiedReason::PendingInspection)
        );
        assert_eq!(
            planned_floor_status(&policy(
                CompatibilityFloor::Macos {
                    major: 13,
                    minor: 0
                },
                BuildStrategy::NativeCargo
            )),
            FloorVerificationStatus::Unverified(FloorUnverifiedReason::FloorKindNotVerifiable)
        );
    }

    #[test]
    fn admissible_statuses_are_closed_per_declaration() {
        assert!(floor_status_admissible(
            &CompatibilityFloor::None,
            BuildStrategy::NativeCargo,
            FloorVerificationStatus::NotDeclared
        ));
        assert!(!floor_status_admissible(
            &CompatibilityFloor::None,
            BuildStrategy::NativeCargo,
            FloorVerificationStatus::Verified
        ));
        assert!(floor_status_admissible(
            &glibc(2, 17),
            BuildStrategy::CargoZigbuild,
            FloorVerificationStatus::ToolchainBound
        ));
        assert!(!floor_status_admissible(
            &glibc(2, 17),
            BuildStrategy::CargoZigbuild,
            FloorVerificationStatus::Verified
        ));
        assert!(floor_status_admissible(
            &glibc(2, 17),
            BuildStrategy::NativeCargo,
            FloorVerificationStatus::Verified
        ));
        assert!(floor_status_admissible(
            &glibc(2, 17),
            BuildStrategy::NativeCargo,
            FloorVerificationStatus::NotSatisfied
        ));
        assert!(!floor_status_admissible(
            &glibc(2, 17),
            BuildStrategy::NativeCargo,
            FloorVerificationStatus::ToolchainBound
        ));
    }

    #[test]
    fn combined_statuses_keep_the_most_severe_result() {
        assert_eq!(
            aggregate_floor_status(&[
                FloorVerificationStatus::Verified,
                FloorVerificationStatus::NotSatisfied
            ]),
            FloorVerificationStatus::NotSatisfied
        );
        assert_eq!(
            aggregate_floor_status(&[
                FloorVerificationStatus::Verified,
                FloorVerificationStatus::Unverified(FloorUnverifiedReason::NoVersionRequirement)
            ]),
            FloorVerificationStatus::Unverified(FloorUnverifiedReason::NoVersionRequirement)
        );
        assert_eq!(
            aggregate_floor_status(&[
                FloorVerificationStatus::Unverified(FloorUnverifiedReason::NoVersionRequirement),
                FloorVerificationStatus::Unverified(FloorUnverifiedReason::UnreadableCandidate)
            ]),
            FloorVerificationStatus::Unverified(FloorUnverifiedReason::UnreadableCandidate)
        );
        assert_eq!(
            aggregate_floor_status(&[FloorVerificationStatus::Verified]),
            FloorVerificationStatus::Verified
        );
    }
}
