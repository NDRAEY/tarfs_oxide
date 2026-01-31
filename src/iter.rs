use no_std_io::io::{self, SeekFrom};
use zerocopy::FromZeros;
use zerocopy::IntoBytes;

use crate::{Entity, MAGIC, RawEntity, TarFS};

use alloc::string::String;
use alloc::string::ToString;

pub struct EntryIter<'fs> {
    pub(crate) fs: &'fs mut TarFS,
    pub(crate) position: usize,
}

impl<'fs> EntryIter<'fs> {
    pub fn new(fs: &'fs mut TarFS) -> Self {
        Self {
            fs,
            position: 0
        }
    }
}

impl Iterator for EntryIter<'_> {
    type Item = io::Result<(usize, RawEntity)>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut raw_header = RawEntity::new_zeroed();

        if let Err(e) = self.fs.device.seek(SeekFrom::Start(self.position as _)) {
            return Some(Err(e));
        }

        if let Err(e) = self.fs.device.read(raw_header.as_mut_bytes()) {
            return Some(Err(e));
        }

        if &raw_header.signature[..5] != MAGIC {
            return None;
        }

        let size_str = String::from_utf8_lossy(&raw_header.size);
        let mut size_str = size_str.trim_end_matches('\0').trim_start_matches('0');

        if size_str.is_empty() {
            size_str = "0";
        }

        let mut size = usize::from_str_radix(size_str, 8).unwrap();

        // File content always aligned by 512 bytes
        if size % 512 != 0 {
            size += 512 - (size % 512);
        }

        let element = Ok((self.position, raw_header));

        // Add the aligned size of file and 512 bytes of entity header
        self.position += size + 512;

        Some(element)
    }
}

pub struct EntityIter<'fs> {
    entries_iter: EntryIter<'fs>
}

impl<'fs> EntityIter<'fs> {
    pub fn new(fs: &'fs mut TarFS) -> Self {
        Self {
            entries_iter: fs.get_entries(),
        }
    }
}

impl Iterator for EntityIter<'_> {
    type Item = io::Result<Entity>;

    fn next(&mut self) -> Option<Self::Item> {
        let entry = self.entries_iter.next()?;

        let (position, i) = match entry {
            Err(e) => return Some(Err(e)),
            Ok(x) => x
        };

        let name = String::from_utf8_lossy(&i.name)
            .trim_end_matches('\0')
            .to_string();

        // Trim leading zeroes and zero-chars
        let size_str = String::from_utf8_lossy(&i.size);
        let size_str = size_str.trim_end_matches('\0').trim_start_matches('0');

        // From octal string to usize
        let size = if size_str.is_empty() {
            0
        } else {
            usize::from_str_radix(size_str, 8).unwrap()
        };

        Some(Ok(Entity {
            size,
            name,
            _type: TarFS::raw_to_type(i._type).unwrap(),
            position,
        }))
    }
}
