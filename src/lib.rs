#![cfg_attr(not(feature = "std"), no_std)]

use no_std_io::io;

extern crate alloc;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
use core::mem::size_of;
use no_std_io::io::SeekFrom;

#[cfg(feature = "builtin_devices")]
pub mod file_device;

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

#[derive(Debug, Clone)]
pub struct Entity {
    pub name: String,
    pub size: usize,
    pub _type: Type,
    pub position: usize,
}

impl RawEntity {
    /// Helper function that exposes ISO header as an array off bytes
    pub const fn as_slice(&mut self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self as *const Self as *const u8, size_of::<Self>()) }
    }

    /// Helper function that exposes ISO header as a mutable array off bytes
    pub const fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self as *mut Self as *mut u8, size_of::<Self>()) }
    }
}

pub trait Device: io::Read + io::Seek {}

pub struct TarFS {
    device: Box<dyn Device>,
}

impl TarFS {
    pub fn from_device(mut device: impl Device + 'static) -> Option<TarFS> {
        let mut raw_header = unsafe { core::mem::zeroed::<RawEntity>() };
        //let read_size = size_of::<RawEntity>();

        let result = device.read(raw_header.as_mut_slice());

        if result.is_err() {
            return None;
        }

        if &raw_header.signature[..5] != MAGIC {
            return None;
        }

        Some(TarFS {
            device: Box::new(device),
        })
    }

    pub fn get_entries(&mut self) -> io::Result<Vec<(usize, RawEntity)>> {
        let mut entries = vec![];
        let mut position: usize = 0;

        loop {
            let mut raw_header = unsafe { core::mem::zeroed::<RawEntity>() };

            self.device.seek(SeekFrom::Start(position as _))?;
            self.device.read(raw_header.as_mut_slice())?;

            if &raw_header.signature[..5] != MAGIC {
                break;
            }

            let size_str: String = String::from_utf8_lossy(&raw_header.size).into_owned();
            let mut size_str = size_str.trim_end_matches('\0').trim_start_matches('0');

            if size_str.is_empty() {
                size_str = "0";
            }

            let mut size = usize::from_str_radix(size_str, 8).unwrap();

            // File content always aligned by 512 bytes
            if size % 512 != 0 {
                size += 512 - (size % 512);
            }

            entries.push((position, raw_header));

            // Add the aligned size of file and 512 bytes of entity header
            position += size + 512;
        }

        Ok(entries)
    }

    const fn raw_to_type(_type: u8) -> Option<Type> {
        match _type {
            TARFS_ELEM_TYPE_FILE => Some(Type::File),
            TARFS_ELEM_TYPE_HARD_LINK => Some(Type::HardLink),
            TARFS_ELEM_TYPE_SYMB_LINK => Some(Type::SymbLink),
            TARFS_ELEM_TYPE_CHR_DEV => Some(Type::ChrDev),
            TARFS_ELEM_TYPE_BLK_DEV => Some(Type::BlkDev),
            TARFS_ELEM_TYPE_DIR => Some(Type::Dir),
            _ => None,
        }
    }

    pub fn list(&mut self) -> io::Result<Vec<Entity>> {
        let raw_entries = self.get_entries()?;
        let mut entities: Vec<Entity> = vec![];

        for (position, i) in raw_entries {
            let name = String::from_utf8_lossy(&i.name)
                .trim_end_matches('\0')
                .to_string();

            // Trim leading zeroes and zero-chars
            let size_str = String::from_utf8_lossy(&i.size);
            let size_str = size_str.trim_end_matches('\0')
                .trim_start_matches('0');

            // From octal string to usize
            let size = if size_str.is_empty() {
                0
            } else {
                usize::from_str_radix(size_str, 8).unwrap()
            };

            entities.push(Entity {
                size,
                name,
                _type: Self::raw_to_type(i._type).unwrap(),
                position,
            });
        }

        Ok(entities)
    }

    pub fn list_by_path(&mut self, path: &str) -> io::Result<Vec<Entity>> {
        let entities = self.list()?;
        // Remove trailing slashes
        let cleaned_path = path.trim_end_matches('/').to_string();

        // Find directories (will always return zero or one element in Vec)
        let matching_directories: Vec<_> = entities
            .iter()
            .filter_map(|entry| {
                let cleaned_name = entry.name.clone();
                let cleaned_name = cleaned_name.trim_end_matches('/'); //.to_string();

                if entry._type == Type::Dir && cleaned_name == cleaned_path {
                    Some(entry.name.clone())
                } else {
                    None
                }
            })
            .collect();

        // Get first element
        let directory_full_name: Option<&String> = matching_directories.first();

        if directory_full_name.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Directory not found",
            ));
        }

        let directory_full_name: &String = directory_full_name.unwrap();

        // If entity name starts with directory name, it's a child in that directory
        let mut result = entities
            .iter()
            .filter_map(|entry| {
                if entry.name.starts_with(directory_full_name) {
                    Some(entry.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<Entity>>();

        // If there some files in it, remove first entry - it is directory itself.
        if !result.is_empty() {
            result.remove(0);
        }

        Ok(result)
    }

    pub fn list_by_path_shallow(&mut self, path: &str) -> io::Result<Vec<Entity>> {
        let entities = self.list()?;
        // Remove trailing slashes
        let cleaned_path = path.trim_end_matches('/').to_string();

        // Find directories (will always return zero or one element in Vec)
        let matching_directories: Vec<_> = entities
            .iter()
            .filter_map(|entry| {
                let cleaned_name = entry.name.clone().trim_end_matches('/').to_string();

                if entry._type == Type::Dir && cleaned_name == cleaned_path {
                    Some(entry.name.clone())
                } else {
                    None
                }
            })
            .collect();

        // Get first element
        let directory_full_name: Option<&String> = matching_directories.first();

        if directory_full_name.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Directory not found",
            ));
        }

        let directory_full_name: &String = directory_full_name.unwrap();
        let pathlen = directory_full_name.len();

        // If entity name starts with directory name, it's a child in that directory
        let mut result = entities
            .iter()
            .filter_map(|entry| {
                if entry.name.starts_with(directory_full_name) {
                    let remaining = &entry.name[pathlen..].trim_end_matches('/');
                    let slash_count = remaining.chars().filter(|&c| c == '/').count();

                    if slash_count == 0 {
                        Some(entry.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<Entity>>();

        // If there some files in it, remove first entry - it is directory itself.
        if !result.is_empty() {
            result.remove(0);
        }

        Ok(result)
    }

    pub fn find_file(&mut self, path: &str) -> io::Result<Entity> {
        match self.list()?.into_iter().find(|entry| entry.name == path) {
            Some(ent) => Ok(ent),
            None => Err(io::Error::new(io::ErrorKind::NotFound, "File not found")),
        }
    }

    pub fn read_file_by_entity(
        &mut self,
        entity: &Entity,
        position: usize,
        mut size: usize,
        output: &mut [u8],
    ) -> io::Result<usize> {
        let end_position = position + size;

        if end_position > entity.size {
            size = end_position - position;
        }

        self.device.seek(io::SeekFrom::Start(
            (entity.position + 512 + position) as u64,
        ))?;
        self.device.read(&mut output[..size])
    }

    pub fn read_file(
        &mut self,
        path: &str,
        position: usize,
        output: &mut [u8],
    ) -> io::Result<usize> {
        let entity = self.find_file(path)?;

        self.read_file_by_entity(&entity, position, output.len(), output)
    }

    pub fn read_entire_file(&mut self, path: &str) -> io::Result<Vec<u8>> {
        let entity = self.find_file(path)?;

        let mut output = vec![0u8; entity.size];

        let result = self.read_file_by_entity(&entity, 0, entity.size, &mut output);

        match result {
            Ok(_) => Ok(output),
            Err(e) => Err(e),
        }
    }

    pub fn read_to_string(&mut self, path: &str) -> io::Result<String> {
        self.read_entire_file(path)
            .map(|v| String::from_utf8_lossy(&v).into_owned())
    }
}
