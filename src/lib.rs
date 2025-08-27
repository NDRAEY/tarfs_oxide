#![cfg_attr(not(feature = "std"), no_std)]

use no_std_io::io;

extern crate alloc;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
use core::mem::size_of;

#[cfg(feature = "builtin_devices")]
pub mod file_device;

pub mod iter;
use iter::EntryIter;

use crate::iter::EntityIter;

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

    pub fn get_entries<'a>(&'a mut self) -> EntryIter<'a> {
        EntryIter {
            fs: self,
            position: 0,
        }
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

    pub fn list<'a>(&'a mut self) -> EntityIter<'a> {
        EntityIter::new(self)
    }

    pub fn list_by_path(&mut self, path: &str) -> io::Result<Vec<Entity>> {
        let mut entities = self.list();
        // Remove trailing slashes
        let cleaned_path = path.trim_end_matches('/').to_string();

        // Find directories (will always return zero or one element in Vec)
        let directory_full_name = entities
            .find(|entry| {
                let entry = entry.as_ref().unwrap();
                let cleaned_name = entry.name.trim_end_matches('/');

                entry._type == Type::Dir && cleaned_name == cleaned_path
            })
            .and_then(|x| x.map(|a| a.name.clone()).ok());

        let directory_full_name = match directory_full_name {
            Some(x) => x,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Directory not found",
                ))
            }
        };

        // If entity name starts with directory name, it's a child in that directory
        let mut result = entities
            .filter_map(|entry| {
                let entry = entry.unwrap();

                if entry.name.starts_with(&directory_full_name) {
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
        let mut entities = self.list();
        // Remove trailing slashes
        let cleaned_path = path.trim_end_matches('/').to_string();

        // Find directory
        let directory_full_name = entities
            .find(|entry| {
                let entry = entry.as_ref().unwrap();

                let cleaned_name = entry.name.trim_end_matches('/');

                entry._type == Type::Dir && cleaned_name == cleaned_path
            })
            .and_then(|x| x.map(|a| a.name.clone()).ok());

        let directory_full_name = match directory_full_name {
            Some(x) => x,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Directory not found",
                ))
            }
        };

        let pathlen = directory_full_name.len();

        // If entity name starts with directory name, it's a child in that directory
        let mut result = entities
            .filter_map(|entry| {
                let entry = entry.unwrap();

                if entry.name.starts_with(&directory_full_name) {
                    let remaining = entry.name[pathlen..].trim_end_matches('/');
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
        match self
            .list()
            .into_iter()
            .find(|entry| entry.as_ref().map(|x| x.name == path).unwrap_or_default())
        {
            Some(ent) => {
                ent
            },
            None => Err(io::Error::new(io::ErrorKind::NotFound, "File not found")),
        }
    }

    pub fn read_file_by_entity(
        &mut self,
        entity: &Entity,
        position: usize,
        output: &mut [u8],
    ) -> io::Result<usize> {
        self.device.seek(io::SeekFrom::Start(
            (entity.position + 512 + position) as u64,
        ))?;
        self.device.read(output)
    }

    pub fn read_file(
        &mut self,
        path: &str,
        position: usize,
        output: &mut [u8],
    ) -> io::Result<usize> {
        let entity = self.find_file(path)?;

        self.read_file_by_entity(&entity, position, output)
    }

    pub fn read_entire_file(&mut self, path: &str) -> io::Result<Vec<u8>> {
        let entity = self.find_file(path)?;

        let mut output = vec![0u8; entity.size];

        let result = self.read_file_by_entity(&entity, 0, &mut output);

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
