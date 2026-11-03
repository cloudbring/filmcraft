//! Compatibility adapter for the shared Compound File Binary reader.

use std::{collections::HashMap, fmt, sync::Arc};

use crate::{Error, Result, compare_names};

/// Directory entry object type (§2.6.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    Root,
    Storage,
    Stream,
    /// Unallocated or unrecognized directory entry.
    Unknown,
}

/// One reachable directory entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub kind: EntryKind,
    pub clsid: [u8; 16],
    pub state_bits: u32,
    pub created: u64,
    pub modified: u64,
    pub start: u32,
    pub size: u64,
    pub children: Vec<usize>,
}

/// A parsed compound file over borrowed bytes.
#[derive(Clone)]
pub struct CompoundFile<'a> {
    shared: Arc<craft_cfb::Cfb<'a>>,
    paths: Vec<String>,
    pub version: u16,
    pub header_clsid: [u8; 16],
    pub entries: Vec<Entry>,
}

impl fmt::Debug for CompoundFile<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompoundFile")
            .field("version", &self.version)
            .field("header_clsid", &self.header_clsid)
            .field("entries", &self.entries)
            .finish_non_exhaustive()
    }
}

fn map_error(error: craft_cfb::Error) -> Error {
    match error.0 {
        "truncated CFB header" | "missing CFB signature" | "unsupported byte order" => Error::NotCfb,
        message => Error::Invalid(message.to_string()),
    }
}

#[cfg(test)]
pub(crate) fn span(index: usize, skip: usize, unit: usize) -> Option<(usize, usize)> {
    let start = index.checked_add(skip)?.checked_mul(unit)?;
    Some((start, start.checked_add(unit)?))
}

impl<'a> CompoundFile<'a> {
    pub fn open(data: &'a [u8]) -> Result<Self> {
        let shared = Arc::new(craft_cfb::Cfb::open(data).map_err(map_error)?);
        let version = u16::from_le_bytes(data.get(26..28).and_then(|bytes| bytes.try_into().ok()).ok_or(Error::NotCfb)?);
        let header_clsid = data.get(8..24).and_then(|bytes| bytes.try_into().ok()).ok_or(Error::NotCfb)?;
        let mut entries = Vec::new();
        let mut paths = Vec::new();
        let mut indices = HashMap::new();
        for item in shared.entries() {
            let index = entries.len();
            let path = item.path().to_string();
            indices.insert(path.clone(), index);
            paths.push(path);
            entries.push(Entry {
                name: item.name().to_string(),
                kind: match item.kind() {
                    craft_cfb::EntryKind::Root => EntryKind::Root,
                    craft_cfb::EntryKind::Storage => EntryKind::Storage,
                    craft_cfb::EntryKind::Stream => EntryKind::Stream,
                },
                clsid: item.clsid(),
                state_bits: item.state_bits(),
                created: item.created(),
                modified: item.modified(),
                start: item.start_sector(),
                size: item.size(),
                children: Vec::new(),
            });
        }
        for (index, path) in paths.iter().enumerate().skip(1) {
            let parent = path.rsplit_once('/').map_or("", |(parent, _)| parent);
            let parent_index = indices.get(parent).copied().ok_or_else(|| Error::Invalid("missing parent storage".into()))?;
            let parent_entry: &mut Entry = entries.get_mut(parent_index).ok_or_else(|| Error::Invalid("missing parent storage".into()))?;
            parent_entry.children.push(index);
        }
        Ok(Self { shared, paths, version, header_clsid, entries })
    }

    pub fn root(&self) -> &Entry {
        &self.entries[0]
    }
    pub fn entry(&self, id: usize) -> Option<&Entry> {
        self.entries.get(id)
    }
    pub fn child(&self, parent: usize, name: &str) -> Option<usize> {
        self.entries.get(parent)?.children.iter().copied().find(|&id| self.entries.get(id).is_some_and(|entry| compare_names(&entry.name, name).is_eq()))
    }
    pub fn find(&self, path: &str) -> Result<usize> {
        let mut current = 0;
        for part in path.split('/').filter(|part| !part.is_empty()) {
            current = self.child(current, part).ok_or_else(|| Error::NotFound(path.to_string()))?;
        }
        Ok(current)
    }
    pub fn read(&self, id: usize) -> Result<Vec<u8>> {
        let entry = self.entries.get(id).ok_or_else(|| Error::NotFound(format!("entry {id}")))?;
        if entry.kind != EntryKind::Stream {
            return Err(Error::Invalid(format!("{:?} is not a stream", entry.name)));
        }
        let path = self.paths.get(id).ok_or_else(|| Error::NotFound(format!("entry {id}")))?;
        self.shared.read_stream(path).map_err(map_error)
    }
    pub fn read_path(&self, path: &str) -> Result<Vec<u8>> {
        self.read(self.find(path)?)
    }
    pub fn sector_size(&self) -> usize {
        if self.version == 3 { 512 } else { 4096 }
    }
}
