#![cfg_attr(not(feature = "std"), no_std)]

use io::Read;

pub mod io;

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

            self.device.read(position, read_size, raw_header.as_mut_slice());

            if &raw_header.signature[..5] != MAGIC {
                break;
            }

            
        }

        entries
    }

    // pub fn list_files
}
