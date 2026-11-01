//! Read-only access to the FM27 save container.
//!
//! A save is a 26-byte header, a run of zstd frames, and a trailer which holds a
//! compressed directory listing the named frames. Offsets in the directory are
//! relative to the end of the header (`FRAME_BASE`).

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

pub const FILE_MAGIC: &[u8] = b"\x02\x01fmf.";
pub const HEADER_SIZE: u64 = 26;
pub const TRAILER_POINTER_AT: u64 = 9;
pub const TRAILER_HEADER_SIZE: usize = 9;
pub const FRAME_BASE: u64 = 26;
pub const DEFAULT_FRAME_CAP: u64 = 1024 * 1024 * 1024;
pub const DEFAULT_TOTAL_CAP: u64 = 4 * 1024 * 1024 * 1024;
const MAX_TRAILER_COMPRESSED: u64 = 64 * 1024 * 1024;
const MAX_TRAILER_DECOMPRESSED: usize = 64 * 1024 * 1024;
const MAX_SAVE_NAME_BYTES: usize = 4096;
const MAX_ENTRY_NAME_BYTES: usize = 256;
const MAX_DIRECTORY_ENTRIES: usize = 100_000;
const ENTRY_FIXED_TAIL_BYTES: usize = 40;

#[derive(Debug, Clone, Serialize)]
pub struct Fingerprint {
    pub file_size: u64,
    pub mtime_ns: i64,
    pub digest: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DirectoryEntry {
    pub name: String,
    pub extension: String,
    pub frame_offset: u64,
    pub compressed_size: u64,
    pub decompressed_size: u64,
    pub trailing_values: [u64; 2],
}

impl DirectoryEntry {
    pub fn is_section(&self) -> bool {
        matches!(self.extension.as_str(), ".dat" | ".cmt")
    }
}

/// One named group of directory entries. FM27 stores the competition sections in
/// a group called `rgman`; the top-level entries describe the main sections.
#[derive(Debug, Clone)]
pub struct DirectoryGroup {
    pub version: u32,
    pub name: String,
    pub entries: Vec<DirectoryEntry>,
}

#[derive(Debug, Clone)]
pub struct SaveDirectory {
    pub save_name: String,
    pub reserved: u32,
    pub top_entries: Vec<DirectoryEntry>,
    pub groups: Vec<DirectoryGroup>,
    pub terminator: Vec<u8>,
}

impl SaveDirectory {
    pub fn all_entries(&self) -> impl Iterator<Item = &DirectoryEntry> {
        self.top_entries
            .iter()
            .chain(self.groups.iter().flat_map(|group| group.entries.iter()))
    }
}

#[derive(Debug, Clone)]
pub struct SaveFile {
    pub path: PathBuf,
    pub save_name: String,
    pub fingerprint: Fingerprint,
    pub trailer_offset: u64,
    pub directory: SaveDirectory,
    pub sections: HashMap<String, usize>,
    header: [u8; HEADER_SIZE as usize],
    trailer_header: [u8; TRAILER_HEADER_SIZE],
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveWriteReport {
    pub output: String,
    pub file_size: u64,
    pub sections_replaced: Vec<String>,
    pub compressed_delta: i64,
}

impl SaveFile {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut file = File::open(&path)?;
        let metadata = file.metadata()?;
        let file_size = metadata.len();
        let mtime_ns = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_nanos() as i64)
            .unwrap_or(0);

        let mut header = [0u8; HEADER_SIZE as usize];
        file.read_exact(&mut header).map_err(|_| {
            Error::Corrupt(format!("{} is too short to be an FM save", path.display()))
        })?;
        if !header.starts_with(FILE_MAGIC) {
            return Err(Error::NotFmSave(path));
        }

        let trailer_relative = u64::from_le_bytes(header[9..17].try_into().unwrap());
        let trailer_offset = TRAILER_POINTER_AT + trailer_relative;
        if trailer_offset < HEADER_SIZE || trailer_offset + TRAILER_HEADER_SIZE as u64 >= file_size
        {
            return Err(Error::corrupt("the save directory is missing"));
        }
        let trailer_len = file_size - trailer_offset;
        if trailer_len - TRAILER_HEADER_SIZE as u64 > MAX_TRAILER_COMPRESSED {
            return Err(Error::corrupt("the save directory is implausibly large"));
        }

        file.seek(SeekFrom::Start(trailer_offset))?;
        let mut trailer = vec![0u8; trailer_len as usize];
        file.read_exact(&mut trailer)?;
        if !trailer.starts_with(FILE_MAGIC) {
            return Err(Error::corrupt("the save directory is damaged"));
        }

        let mut digest = Sha256::new();
        digest.update(header);
        digest.update(&trailer);
        let digest = format!("{:x}", digest.finalize());

        let payload = decompress_frame(
            &trailer[TRAILER_HEADER_SIZE..],
            None,
            MAX_TRAILER_DECOMPRESSED,
            "the save directory",
        )?;
        let directory = parse_directory(&payload)?;
        validate_all_entries(&directory, trailer_offset)?;

        let mut sections = HashMap::new();
        for (index, entry) in directory.top_entries.iter().enumerate() {
            if entry.is_section() && sections.insert(entry.name.clone(), index).is_some() {
                return Err(Error::corrupt(format!(
                    "the save directory lists section {:?} twice",
                    entry.name
                )));
            }
        }

        let mut header_bytes = [0u8; HEADER_SIZE as usize];
        header_bytes.copy_from_slice(&header);
        let mut trailer_header_bytes = [0u8; TRAILER_HEADER_SIZE];
        trailer_header_bytes.copy_from_slice(&trailer[..TRAILER_HEADER_SIZE]);

        Ok(Self {
            path,
            save_name: directory.save_name.clone(),
            fingerprint: Fingerprint {
                file_size,
                mtime_ns,
                digest,
            },
            trailer_offset,
            directory,
            sections,
            header: header_bytes,
            trailer_header: trailer_header_bytes,
        })
    }

    pub fn section_entry(&self, name: &str) -> Result<&DirectoryEntry> {
        self.sections
            .get(name)
            .map(|index| &self.directory.top_entries[*index])
            .ok_or_else(|| Error::corrupt(format!("save has no {name:?} section")))
    }

    pub fn read_section(&self, name: &str) -> Result<Vec<u8>> {
        let entry = self.section_entry(name)?.clone();
        if entry.decompressed_size > DEFAULT_FRAME_CAP {
            return Err(Error::corrupt(format!(
                "section {name:?} is larger than the per-frame cap"
            )));
        }
        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(entry.frame_offset))?;
        let mut compressed = vec![0u8; entry.compressed_size as usize];
        file.read_exact(&mut compressed)?;
        decompress_frame(
            &compressed,
            Some(entry.decompressed_size as usize),
            DEFAULT_FRAME_CAP as usize,
            &format!("section {name:?}"),
        )
    }

    pub fn total_declared_bytes(&self) -> u64 {
        self.directory
            .top_entries
            .iter()
            .map(|entry| entry.decompressed_size)
            .sum()
    }

    pub fn section_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .directory
            .top_entries
            .iter()
            .filter(|entry| entry.is_section())
            .map(|entry| entry.name.as_str())
            .collect();
        names.sort_unstable();
        names
    }

    /// Rewrite this save with one or more top-level sections replaced.
    ///
    /// The source save is never modified: `output` must be a different path. Frames
    /// that are not replaced are copied byte-for-byte, including any unlisted frames
    /// in gaps, and every directory offset is updated for the new frame sizes.
    pub fn write_with_replacements(
        &self,
        replacements: &[(&str, &[u8])],
        output: impl AsRef<Path>,
        compression_level: i32,
    ) -> Result<SaveWriteReport> {
        let output = output.as_ref().to_path_buf();
        if same_file(&self.path, &output) {
            return Err(Error::Unsupported(
                "refusing to overwrite the source save; choose a different output path".to_owned(),
            ));
        }

        let mut replacement_map: HashMap<&str, &[u8]> = HashMap::new();
        for (name, data) in replacements {
            if replacement_map.insert(*name, *data).is_some() {
                return Err(Error::Unsupported(format!(
                    "section {name:?} was replaced more than once"
                )));
            }
            self.section_entry(name)?;
        }

        let file_metadata = std::fs::metadata(&self.path)?;
        let current_mtime_ns = file_metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_nanos() as i64)
            .unwrap_or(0);
        if file_metadata.len() != self.fingerprint.file_size
            || current_mtime_ns != self.fingerprint.mtime_ns
        {
            return Err(Error::Unsupported(
                "the source save changed on disk; close Football Manager or copy the save before editing"
                    .to_owned(),
            ));
        }
        let source = std::fs::read(&self.path)?;
        let mut digest = Sha256::new();
        digest.update(
            source
                .get(..HEADER_SIZE as usize)
                .ok_or_else(|| Error::corrupt("the source save is shorter than its header"))?,
        );
        digest.update(
            source
                .get(self.trailer_offset as usize..)
                .ok_or_else(|| Error::corrupt("the source save is shorter than its trailer"))?,
        );
        if format!("{:x}", digest.finalize()) != self.fingerprint.digest {
            return Err(Error::Unsupported(
                "the source save changed on disk; close Football Manager or copy the save before editing"
                    .to_owned(),
            ));
        }

        let mut compressed_replacements: HashMap<&str, Vec<u8>> = HashMap::new();
        let mut decompressed_sizes: HashMap<&str, u64> = HashMap::new();
        for (name, data) in &replacement_map {
            compressed_replacements.insert(*name, compress_frame(data, compression_level)?);
            decompressed_sizes.insert(*name, data.len() as u64);
        }

        let mut top_entries = self.directory.top_entries.clone();
        let mut groups = self.directory.groups.clone();
        let mut order: Vec<EntryRef> = top_entries
            .iter()
            .enumerate()
            .map(|(index, _)| EntryRef::Top(index))
            .chain(groups.iter().enumerate().flat_map(|(group, value)| {
                (0..value.entries.len()).map(move |index| EntryRef::Group(group, index))
            }))
            .collect();
        order.sort_by_key(|entry| entry_at(&top_entries, &groups, *entry).frame_offset);

        let mut output_bytes = Vec::with_capacity(source.len());
        output_bytes.extend_from_slice(&self.header);
        let mut source_cursor = HEADER_SIZE;
        let mut compressed_delta = 0i64;

        for entry_ref in order {
            let (old_offset, old_compressed_size, name) = {
                let entry = entry_at(&top_entries, &groups, entry_ref);
                (
                    entry.frame_offset,
                    entry.compressed_size,
                    entry.name.clone(),
                )
            };
            if old_offset < source_cursor {
                return Err(Error::corrupt(format!(
                    "directory entry {name:?} overlaps another frame"
                )));
            }
            if old_offset > source_cursor {
                let gap = source
                    .get(source_cursor as usize..old_offset as usize)
                    .ok_or_else(|| Error::corrupt("the source save ended inside a frame gap"))?;
                output_bytes.extend_from_slice(gap);
            }

            let new_offset = output_bytes.len() as u64;
            let old_end = old_offset
                .checked_add(old_compressed_size)
                .ok_or_else(|| Error::corrupt("a directory frame offset overflowed"))?;
            let replacement = replacement_map
                .get(name.as_str())
                .and_then(|_| compressed_replacements.get(name.as_str()))
                .zip(decompressed_sizes.get(name.as_str()));

            if let Some((compressed, decompressed)) = replacement {
                compressed_delta += compressed.len() as i64 - old_compressed_size as i64;
                output_bytes.extend_from_slice(compressed);
                let entry = entry_at_mut(&mut top_entries, &mut groups, entry_ref);
                entry.compressed_size = compressed.len() as u64;
                entry.decompressed_size = *decompressed;
            } else {
                let frame = source
                    .get(old_offset as usize..old_end as usize)
                    .ok_or_else(|| {
                        Error::corrupt("the source save ended inside a directory frame")
                    })?;
                output_bytes.extend_from_slice(frame);
            }

            entry_at_mut(&mut top_entries, &mut groups, entry_ref).frame_offset = new_offset;
            source_cursor = old_end;
        }

        let trailer_offset = self.trailer_offset;
        if source_cursor > trailer_offset {
            return Err(Error::corrupt("a directory frame runs past the trailer"));
        }
        if source_cursor < trailer_offset {
            let tail = source
                .get(source_cursor as usize..trailer_offset as usize)
                .ok_or_else(|| Error::corrupt("the source save ended before its trailer"))?;
            output_bytes.extend_from_slice(tail);
        }

        let directory = SaveDirectory {
            save_name: self.directory.save_name.clone(),
            reserved: self.directory.reserved,
            top_entries,
            groups,
            terminator: self.directory.terminator.clone(),
        };
        let payload = encode_directory(&directory)?;
        if payload.len() > MAX_TRAILER_DECOMPRESSED {
            return Err(Error::Unsupported(
                "the rebuilt save directory is larger than the trailer cap".to_owned(),
            ));
        }
        let compressed_directory = compress_frame(&payload, compression_level)?;
        let new_trailer_offset = output_bytes.len() as u64;
        output_bytes.extend_from_slice(&self.trailer_header);
        output_bytes.extend_from_slice(&compressed_directory);
        let pointer = new_trailer_offset
            .checked_sub(TRAILER_POINTER_AT)
            .ok_or_else(|| Error::corrupt("the rebuilt trailer offset underflowed"))?;
        output_bytes[9..17].copy_from_slice(&pointer.to_le_bytes());

        let parent = output
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        temp.write_all(&output_bytes)?;
        temp.as_file().sync_all()?;
        temp.persist(&output)
            .map_err(|error| Error::Io(error.error))?;

        let mut sections_replaced: Vec<String> = replacement_map
            .keys()
            .map(|name| (*name).to_owned())
            .collect();
        sections_replaced.sort_unstable();
        Ok(SaveWriteReport {
            output: output.display().to_string(),
            file_size: output_bytes.len() as u64,
            sections_replaced,
            compressed_delta,
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum EntryRef {
    Top(usize),
    Group(usize, usize),
}

fn entry_at<'a>(
    top_entries: &'a [DirectoryEntry],
    groups: &'a [DirectoryGroup],
    entry: EntryRef,
) -> &'a DirectoryEntry {
    match entry {
        EntryRef::Top(index) => &top_entries[index],
        EntryRef::Group(group, index) => &groups[group].entries[index],
    }
}

fn entry_at_mut<'a>(
    top_entries: &'a mut [DirectoryEntry],
    groups: &'a mut [DirectoryGroup],
    entry: EntryRef,
) -> &'a mut DirectoryEntry {
    match entry {
        EntryRef::Top(index) => &mut top_entries[index],
        EntryRef::Group(group, index) => &mut groups[group].entries[index],
    }
}

fn same_file(left: &Path, right: &Path) -> bool {
    fn identity(path: &Path) -> Option<PathBuf> {
        if let Ok(canonical) = path.canonicalize() {
            return Some(canonical);
        }
        let parent = path.parent()?.canonicalize().ok()?;
        Some(parent.join(path.file_name()?))
    }
    match (identity(left), identity(right)) {
        (Some(left), Some(right)) => left == right,
        _ => left == right,
    }
}

fn compress_frame(data: &[u8], level: i32) -> Result<Vec<u8>> {
    let mut encoder = zstd::stream::Encoder::new(Vec::new(), level)?;
    encoder.include_checksum(true)?;
    encoder.write_all(data)?;
    encoder.finish().map_err(Error::Io)
}

fn encode_directory(directory: &SaveDirectory) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    write_length_prefixed_string(&mut output, &directory.save_name, MAX_SAVE_NAME_BYTES)?;
    output.extend_from_slice(&directory.reserved.to_le_bytes());
    for entry in &directory.top_entries {
        encode_directory_entry(&mut output, entry)?;
    }
    for group in &directory.groups {
        output.extend_from_slice(&group.version.to_le_bytes());
        write_length_prefixed_string(&mut output, &group.name, MAX_ENTRY_NAME_BYTES)?;
        let count = u32::try_from(group.entries.len())
            .map_err(|_| Error::Unsupported("a directory group has too many entries".to_owned()))?;
        output.extend_from_slice(&count.to_le_bytes());
        for entry in &group.entries {
            encode_directory_entry(&mut output, entry)?;
        }
    }
    output.extend_from_slice(&directory.terminator);
    Ok(output)
}

fn encode_directory_entry(output: &mut Vec<u8>, entry: &DirectoryEntry) -> Result<()> {
    write_length_prefixed_string(output, &entry.name, MAX_ENTRY_NAME_BYTES)?;
    if entry.extension.len() != 4 || !entry.extension.starts_with('.') {
        return Err(Error::Unsupported(format!(
            "directory entry {:?} has an invalid extension",
            entry.name
        )));
    }
    output.extend_from_slice(&4u32.to_le_bytes());
    output.extend_from_slice(entry.extension.as_bytes());
    let relative_offset = entry.frame_offset.checked_sub(FRAME_BASE).ok_or_else(|| {
        Error::Unsupported(format!(
            "directory entry {:?} points before the frame area",
            entry.name
        ))
    })?;
    output.extend_from_slice(&relative_offset.to_le_bytes());
    output.extend_from_slice(&entry.compressed_size.to_le_bytes());
    output.extend_from_slice(&entry.decompressed_size.to_le_bytes());
    output.extend_from_slice(&entry.trailing_values[0].to_le_bytes());
    output.extend_from_slice(&entry.trailing_values[1].to_le_bytes());
    Ok(())
}

fn write_length_prefixed_string(output: &mut Vec<u8>, text: &str, max_len: usize) -> Result<()> {
    if text.len() > max_len {
        return Err(Error::Unsupported(format!(
            "directory string {text:?} is longer than {max_len} bytes"
        )));
    }
    let length = u32::try_from(text.len())
        .map_err(|_| Error::Unsupported("a directory string is too long".to_owned()))?;
    output.extend_from_slice(&length.to_le_bytes());
    output.extend_from_slice(text.as_bytes());
    Ok(())
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    bytes
        .get(offset..offset + 4)
        .map(|slice| u32::from_le_bytes(slice.try_into().unwrap()))
}

fn read_u64(bytes: &[u8], offset: usize) -> Option<u64> {
    bytes
        .get(offset..offset + 8)
        .map(|slice| u64::from_le_bytes(slice.try_into().unwrap()))
}

fn read_length_prefixed_string(
    bytes: &[u8],
    offset: usize,
    max_len: usize,
) -> Option<(String, usize)> {
    let len = read_u32(bytes, offset)? as usize;
    if len > max_len {
        return None;
    }
    let start = offset + 4;
    let end = start.checked_add(len)?;
    let text = std::str::from_utf8(bytes.get(start..end)?).ok()?;
    Some((text.to_owned(), end))
}

fn parse_directory(payload: &[u8]) -> Result<SaveDirectory> {
    let (save_name, mut cursor) = read_length_prefixed_string(payload, 0, MAX_SAVE_NAME_BYTES)
        .ok_or_else(|| Error::corrupt("the save directory name is unreadable"))?;
    let reserved = read_u32(payload, cursor)
        .ok_or_else(|| Error::corrupt("the save directory reserved word is unreadable"))?;
    cursor += 4;

    let mut top_entries = Vec::new();
    while let Some((entry, next)) = parse_directory_entry(payload, cursor) {
        top_entries.push(entry);
        cursor = next;
        if top_entries.len() > MAX_DIRECTORY_ENTRIES {
            return Err(Error::corrupt("the save directory has too many entries"));
        }
    }

    let mut groups = Vec::new();
    let terminator = loop {
        if cursor >= payload.len() {
            break Vec::new();
        }
        if payload[cursor..].iter().all(|byte| *byte == 0) {
            break payload[cursor..].to_vec();
        }
        let (group, next) = parse_directory_group(payload, cursor)?;
        groups.push(group);
        cursor = next;
    };

    if !top_entries.iter().any(DirectoryEntry::is_section)
        && !groups
            .iter()
            .flat_map(|group| group.entries.iter())
            .any(DirectoryEntry::is_section)
    {
        return Err(Error::corrupt("the save directory lists no sections"));
    }
    top_entries.sort_by_key(|entry| entry.frame_offset);
    Ok(SaveDirectory {
        save_name,
        reserved,
        top_entries,
        groups,
        terminator,
    })
}

fn parse_directory_group(payload: &[u8], cursor: usize) -> Result<(DirectoryGroup, usize)> {
    let version = read_u32(payload, cursor)
        .ok_or_else(|| Error::corrupt("a directory group header is truncated"))?;
    let mut cursor = cursor + 4;
    let name_len = read_u32(payload, cursor)
        .ok_or_else(|| Error::corrupt("a directory group name is truncated"))?
        as usize;
    if !(1..=MAX_ENTRY_NAME_BYTES).contains(&name_len) {
        return Err(Error::corrupt(
            "a directory group name has an invalid length",
        ));
    }
    let name_end = cursor
        .checked_add(4)
        .and_then(|at| at.checked_add(name_len))
        .ok_or_else(|| Error::corrupt("a directory group name runs past the directory"))?;
    let name_bytes = payload
        .get(cursor + 4..name_end)
        .ok_or_else(|| Error::corrupt("a directory group name is truncated"))?;
    if !name_bytes
        .iter()
        .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
    {
        return Err(Error::corrupt("a directory group name is not valid"));
    }
    let name = std::str::from_utf8(name_bytes)
        .map_err(|_| Error::corrupt("a directory group name is not UTF-8"))?
        .to_owned();
    cursor = name_end;

    let count = read_u32(payload, cursor)
        .ok_or_else(|| Error::corrupt("a directory group entry count is truncated"))?
        as usize;
    cursor += 4;
    if count > MAX_DIRECTORY_ENTRIES {
        return Err(Error::corrupt("a directory group has too many entries"));
    }

    let mut entries = Vec::new();
    for index in 0..count {
        let (entry, next) = parse_directory_entry(payload, cursor).ok_or_else(|| {
            Error::corrupt(format!(
                "directory group {name:?} entry {index} is unreadable"
            ))
        })?;
        entries.push(entry);
        cursor = next;
    }
    Ok((
        DirectoryGroup {
            version,
            name,
            entries,
        },
        cursor,
    ))
}

fn parse_directory_entry(payload: &[u8], cursor: usize) -> Option<(DirectoryEntry, usize)> {
    let name_len = read_u32(payload, cursor)? as usize;
    if !(1..=MAX_ENTRY_NAME_BYTES).contains(&name_len) {
        return None;
    }
    let extension_len_at = cursor.checked_add(4 + name_len)?;
    let tail_at = extension_len_at.checked_add(8)?;
    if tail_at.checked_add(ENTRY_FIXED_TAIL_BYTES)? > payload.len() {
        return None;
    }
    let name_bytes = payload.get(cursor + 4..extension_len_at)?;
    let extension_bytes = payload.get(extension_len_at + 4..tail_at)?;
    if read_u32(payload, extension_len_at)? != 4 {
        return None;
    }
    if !name_bytes
        .iter()
        .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
    {
        return None;
    }
    if extension_bytes.len() != 4
        || extension_bytes[0] != b'.'
        || !extension_bytes[1..].iter().all(u8::is_ascii_alphanumeric)
    {
        return None;
    }
    Some((
        DirectoryEntry {
            name: std::str::from_utf8(name_bytes).ok()?.to_owned(),
            extension: std::str::from_utf8(extension_bytes).ok()?.to_owned(),
            frame_offset: FRAME_BASE + read_u64(payload, tail_at)?,
            compressed_size: read_u64(payload, tail_at + 8)?,
            decompressed_size: read_u64(payload, tail_at + 16)?,
            trailing_values: [
                read_u64(payload, tail_at + 24)?,
                read_u64(payload, tail_at + 32)?,
            ],
        },
        tail_at + ENTRY_FIXED_TAIL_BYTES,
    ))
}

fn validate_all_entries(directory: &SaveDirectory, trailer_offset: u64) -> Result<()> {
    let mut entries: Vec<&DirectoryEntry> = directory.all_entries().collect();
    entries.sort_by_key(|entry| entry.frame_offset);
    let mut previous_end = HEADER_SIZE;
    let mut total = 0u64;
    for entry in entries {
        if entry.compressed_size == 0
            || entry.frame_offset < previous_end
            || entry.frame_offset + entry.compressed_size > trailer_offset
        {
            return Err(Error::corrupt(format!(
                "directory entry {}{} points outside the frame area",
                entry.name, entry.extension
            )));
        }
        if entry.decompressed_size > DEFAULT_FRAME_CAP {
            return Err(Error::corrupt(format!(
                "directory entry {}{} is larger than the per-frame cap",
                entry.name, entry.extension
            )));
        }
        total = total.saturating_add(entry.decompressed_size);
        previous_end = entry.frame_offset + entry.compressed_size;
    }
    if total > DEFAULT_TOTAL_CAP {
        return Err(Error::corrupt(
            "the save declares more data than the total decompressed cap",
        ));
    }
    Ok(())
}

fn decompress_frame(
    compressed: &[u8],
    expected_size: Option<usize>,
    cap: usize,
    what: &str,
) -> Result<Vec<u8>> {
    let output_limit = expected_size.map_or(cap, |size| size.min(cap));
    let decoded = zstd::bulk::decompress(compressed, output_limit)
        .map_err(|error| Error::corrupt(format!("{what} could not be decompressed: {error}")))?;
    if decoded.len() > output_limit {
        return Err(Error::corrupt(format!(
            "{what} decompresses to more than {output_limit} bytes"
        )));
    }
    if let Some(expected) = expected_size {
        if decoded.len() != expected {
            return Err(Error::corrupt(format!(
                "{what} decompressed to {} bytes, expected {expected}",
                decoded.len()
            )));
        }
    }
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const TRAILER_HEADER: [u8; TRAILER_HEADER_SIZE] =
        [0x02, 0x01, b'f', b'm', b'f', b'.', 0x08, 0x00, 0x00];

    fn build_save(top: &[(&str, &[u8])], group: &[(&str, &[u8])]) -> Vec<u8> {
        let mut header = [0u8; HEADER_SIZE as usize];
        header[..6].copy_from_slice(FILE_MAGIC);
        header[6..9].copy_from_slice(&[0x08, 0, 0]);
        header[17..21].copy_from_slice(&17u32.to_le_bytes());
        header[25] = 3;

        let mut frames = Vec::new();
        let mut top_entries = Vec::new();
        let mut group_entries = Vec::new();
        let mut append = |name: &str, data: &[u8], entries: &mut Vec<DirectoryEntry>| {
            let compressed = compress_frame(data, 3).unwrap();
            let frame_offset = HEADER_SIZE + frames.len() as u64;
            frames.extend_from_slice(&compressed);
            entries.push(DirectoryEntry {
                name: name.to_owned(),
                extension: ".dat".to_owned(),
                frame_offset,
                compressed_size: compressed.len() as u64,
                decompressed_size: data.len() as u64,
                trailing_values: [0xffff_fff1_886e_0900, 0xffff_fff1_886e_0900],
            });
        };
        for (name, data) in top {
            append(name, data, &mut top_entries);
        }
        for (name, data) in group {
            append(name, data, &mut group_entries);
        }

        let payload = encode_directory(&SaveDirectory {
            save_name: "test save".to_owned(),
            reserved: top.len() as u32,
            top_entries,
            groups: vec![DirectoryGroup {
                version: 1,
                name: "rgman".to_owned(),
                entries: group_entries,
            }],
            terminator: vec![0, 0, 0, 0],
        })
        .unwrap();
        let compressed_directory = compress_frame(&payload, 3).unwrap();
        let trailer_offset = HEADER_SIZE + frames.len() as u64;
        let mut output = Vec::new();
        output.extend_from_slice(&header);
        output.extend_from_slice(&frames);
        output.extend_from_slice(&TRAILER_HEADER);
        output.extend_from_slice(&compressed_directory);
        output[9..17].copy_from_slice(&(trailer_offset - TRAILER_POINTER_AT).to_le_bytes());
        output
    }

    #[test]
    fn reads_groups_and_rebuilds_after_a_replacement() {
        let directory = tempdir().unwrap();
        let source = directory.path().join("source.fm");
        let output = directory.path().join("edited.fm");
        std::fs::write(
            &source,
            build_save(
                &[
                    ("game_db", b"old game database"),
                    ("save_game_summary", b"summary"),
                    ("game_info", b"info"),
                ],
                &[
                    ("comp_1", b"competition one"),
                    ("comp_2", b"competition two"),
                ],
            ),
        )
        .unwrap();

        let save = SaveFile::open(&source).unwrap();
        assert_eq!(save.directory.top_entries.len(), 3);
        assert_eq!(save.directory.groups.len(), 1);
        assert_eq!(save.directory.groups[0].name, "rgman");
        assert_eq!(save.directory.groups[0].entries.len(), 2);
        assert_eq!(save.directory.terminator, vec![0, 0, 0, 0]);
        assert_eq!(save.read_section("game_db").unwrap(), b"old game database");
        assert_eq!(
            save.section_names(),
            vec!["game_db", "game_info", "save_game_summary"]
        );

        let replacement = b"new game database with a different length";
        let report = save
            .write_with_replacements(&[("game_db", replacement)], &output, 3)
            .unwrap();
        assert_eq!(report.sections_replaced, vec!["game_db"]);
        assert!(report.file_size > 0);

        let edited = SaveFile::open(&output).unwrap();
        assert_eq!(edited.read_section("game_db").unwrap(), replacement);
        assert_eq!(
            edited.read_section("save_game_summary").unwrap(),
            b"summary"
        );
        assert_eq!(edited.directory.groups[0].entries.len(), 2);
        assert_eq!(
            edited.directory.groups[0].entries[0].name,
            "comp_1".to_owned()
        );
        assert_eq!(
            edited.directory.groups[0].entries[0].trailing_values,
            [0xffff_fff1_886e_0900, 0xffff_fff1_886e_0900]
        );
        assert_eq!(
            std::fs::read(&source).unwrap(),
            build_save(
                &[
                    ("game_db", b"old game database"),
                    ("save_game_summary", b"summary"),
                    ("game_info", b"info"),
                ],
                &[
                    ("comp_1", b"competition one"),
                    ("comp_2", b"competition two")
                ],
            )
        );
    }

    #[test]
    fn a_noop_rebuild_keeps_every_section_readable() {
        let directory = tempdir().unwrap();
        let source = directory.path().join("source.fm");
        let output = directory.path().join("copy.fm");
        std::fs::write(
            &source,
            build_save(
                &[("game_db", b"database"), ("game_info", b"info")],
                &[("comp_1", b"one")],
            ),
        )
        .unwrap();

        let save = SaveFile::open(&source).unwrap();
        save.write_with_replacements(&[], &output, 3).unwrap();
        let copy = SaveFile::open(&output).unwrap();
        assert_eq!(copy.read_section("game_db").unwrap(), b"database");
        assert_eq!(copy.read_section("game_info").unwrap(), b"info");
        assert_eq!(copy.directory.groups[0].entries.len(), 1);
        assert_eq!(copy.directory.groups[0].entries[0].name, "comp_1");
    }

    #[test]
    fn refuses_to_overwrite_the_source() {
        let directory = tempdir().unwrap();
        let source = directory.path().join("source.fm");
        std::fs::write(
            &source,
            build_save(&[("game_db", b"database")], &[("comp_1", b"one")]),
        )
        .unwrap();

        let save = SaveFile::open(&source).unwrap();
        let error = save
            .write_with_replacements(&[("game_db", b"other")], &source, 3)
            .unwrap_err();
        assert!(error.to_string().contains("refusing to overwrite"));
    }
}
