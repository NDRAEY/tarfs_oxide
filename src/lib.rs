#![cfg_attr(not(feature = "std"), no_std)]

pub mod io;
use io::Read;

extern crate alloc;
use alloc::vec;
use alloc::boxed::Box;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::ToString;

pub const TARFS_ELEM_TYPE_FILE: u8 = 48;
pub const TARFS_ELEM_TYPE_HARD_LINK: u8 = 49;
pub const TARFS_ELEM_TYPE_SYMB_LINK: u8 = 50;
pub const TARFS_ELEM_TYPE_CHR_DEV: u8 = 51;
pub const TARFS_ELEM_TYPE_BLK_DEV: u8 = 52;
pub const TARFS_ELEM_TYPE_DIR: u8 = 53;
pub const TARFS_ELEM_TYPE_PIPE: u8 = 54;

pub const MAGIC: &[u8; 5] = b"ustar";

#[repr(C)]
pub struct RawEntity {
    pub(crate) name: [u8; 100],
    pub(crate) mode: [u8; 8],

    pub(crate) uid: [u8; 8],
    pub(crate) gid: [u8; 8],

    pub(crate) size: [u8; 12],
    pub(crate) addition_time: [u8; 12],

    pub(crate) checksum: [u8; 8],
    pub(crate) _type: u8,

    pub(crate) link: [u8; 100],
    pub(crate) signature: [u8; 6],
    pub(crate) version: [u8; 2],

    pub(crate) user: [u8; 32],
    /// Имя владельца
    pub(crate) group: [u8; 32],
    /// Имя группы
    pub(crate) device_nro_1: [u8; 8],
    /// Основной номер устройства
    pub(crate) device_nro_2: [u8; 8],
    /// Младший номер устройства
    pub(crate) prefix: [u8; 155],
}

#[derive(Debug, PartialEq, Clone)]
pub enum Type {
    File,
    HardLink,
    SymbLink,
    ChrDev,
    BlkDev,
    Dir,
}

#[derive(Debug)]
pub enum ListError {
    NotFound
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub name: String,
    pub size: usize,
    pub _type: Type,
    pub position: usize
}

impl RawEntity {
    /// Helper function that exposes ISO header as an array off bytes
    pub fn as_slice(&mut self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self as *const Self as *const u8, size_of::<Self>()) }
    }

    /// Helper function that exposes ISO header as a mutable array off bytes
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self as *mut Self as *mut u8, size_of::<Self>()) }
    }
}

pub struct TarFS {
    device: Box<dyn Read>,
}

impl TarFS {
    pub fn from_device(mut device: impl Read + 'static) -> Option<TarFS> {
        let mut raw_header = unsafe { core::mem::zeroed::<RawEntity>() };
        let read_size = size_of::<RawEntity>();

        device.read(0, read_size, raw_header.as_mut_slice());

        Some(TarFS {
            device: Box::new(device),
        })
    }

    pub fn get_entries(&mut self) -> Vec<(usize, RawEntity)> {
        let mut entries = vec![];
        let mut position: usize = 0;

        loop {
            let mut raw_header = unsafe { core::mem::zeroed::<RawEntity>() };
            let read_size = size_of::<RawEntity>();

            self.device
                .read(position, read_size, raw_header.as_mut_slice());

            if &raw_header.signature[..5] != MAGIC {
                break;
            }

            let size_str: String = String::from_utf8_lossy(&raw_header.size).into_owned();
            let mut size_str = size_str
                .trim_end_matches(|c| c == '\0')
                .trim_start_matches(|c| c == '0');

            if size_str.len() == 0 {
                size_str = "0";
            }

            let mut size = usize::from_str_radix(&size_str, 8).unwrap();

            // File content always aligned by 512 bytes
            if size % 512 != 0 {
                size += 512 - (size % 512);
            }

            entries.push((position, raw_header));

            // Add the aligned size of file and 512 bytes of entity header
            position += size + 512;
        }

        entries
    }

    fn raw_to_type(_type: u8) -> Option<Type> {
        match _type {
            TARFS_ELEM_TYPE_FILE => Some(Type::File),
            TARFS_ELEM_TYPE_HARD_LINK => Some(Type::HardLink),
            TARFS_ELEM_TYPE_SYMB_LINK => Some(Type::SymbLink),
            TARFS_ELEM_TYPE_CHR_DEV => Some(Type::ChrDev),
            TARFS_ELEM_TYPE_BLK_DEV => Some(Type::BlkDev),
            TARFS_ELEM_TYPE_DIR => Some(Type::Dir),
            _ => None
        }
    }

    pub fn list(&mut self) -> Vec<Entity> {
        let raw_entries = self.get_entries();
        let mut entities: Vec<Entity> = vec![];

        for (position, i) in raw_entries {
            let name = String::from_utf8_lossy(&i.name).into_owned();
            let name = name.trim_end_matches(|c| c == '\0').to_string();

            let size_str: String = String::from_utf8_lossy(&i.size).into_owned();
            
            // Trim zeroes and zero-chars
            let mut size_str = size_str
                .trim_end_matches(|c| c == '\0')
                .trim_start_matches(|c| c == '0');

            // If nothing remained, set to 0
            if size_str.len() == 0 {
                size_str = "0";
            }
            
            // From octal string to usize
            let size = usize::from_str_radix(&size_str, 8).unwrap();

            entities.push(Entity { size, name, _type: Self::raw_to_type(i._type).unwrap(), position });
        }

        entities
    }

    pub fn list_by_path(&mut self, path: &str) -> Result<Vec<Entity>, ListError> {
        let entities = self.list();
        // Remove trailing slashes
        let cleaned_path = path.trim_end_matches(|c| c == '/').to_string();

        // Find directories (will always return zero or one element in Vec)
        let matching_directories: Vec<_> = entities.iter().filter_map(|entry| {
            let cleaned_name = entry.name.clone().trim_end_matches(|c| c == '/').to_string();

            if entry._type == Type::Dir && cleaned_name == cleaned_path {
                Some(entry.name.clone())
            } else {
                None
            }
        }).collect();
        
        // Get first element
        let directory_full_name: Option<&String> = matching_directories.first();

        if directory_full_name.is_none() {
            return Err(ListError::NotFound);
        }

        let directory_full_name: &String = directory_full_name.unwrap();

        // If entity name starts with directory name, it's a child in that directory
        let mut result = entities.iter().filter_map(|entry| {
            if entry.name.starts_with(directory_full_name) {
                Some(entry.clone())
            } else {
                None
            }
        }).collect::<Vec<Entity>>();

        // If there some files in it, remove first entry - it is directory itself.
        if !result.is_empty() {
            result.remove(0);
        }

        Ok(result)
    }

    pub fn find_file(&mut self, path: &str) -> Option<Entity> {
        self.list().into_iter().find(|entry| entry.name == path)
    }

    pub fn read_file_by_entity(&mut self, entity: &Entity, position: usize, mut size: usize, output: &mut [u8]) -> usize {
        let end_position = position + size;

        if end_position > entity.size {
            size = end_position - position;
        }

        self.device.read(entity.position + 512 + position, size, output);

        size
    }

    pub fn read_file(&mut self, path: &str, position: usize, size: usize, output: &mut [u8]) -> Option<usize> {
        let entity = self.find_file(path)?;

        Some(self.read_file_by_entity(&entity, position, size, output))
    }

    pub fn read_entire_file(&mut self, path: &str) -> Option<Vec<u8>> {
        let entity = self.find_file(path)?;

        let mut output = vec![0u8; entity.size];

        self.read_file_by_entity(&entity, 0, entity.size, &mut output);

        Some(output)
    }

    pub fn read_to_string(&mut self, path: &str) -> Option<String> {
        self.read_entire_file(path).map(|v| String::from_utf8_lossy(&v).into_owned())
    }
}
