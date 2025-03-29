use no_std_io::{self, io::{Error, ErrorKind, Result}};
use std::{fs::File, io::{Read, Seek, SeekFrom}};

pub struct FileDevice(pub File);

impl no_std_io::io::Read for FileDevice {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.0
            .read(buf)
            .map_err(|_| Error::new(ErrorKind::Other, "Unknown error"))
    }
}

impl no_std_io::io::Seek for FileDevice {
    fn seek(&mut self, pos: no_std_io::io::SeekFrom) -> Result<u64> {
        // Convert `no_std_io`'s `SeekFrom` to `std::io`'s `SeekFrom`
        let pos = match pos {
            no_std_io::io::SeekFrom::Start(pos) => SeekFrom::Start(pos),
            no_std_io::io::SeekFrom::End(pos) => SeekFrom::End(pos),
            no_std_io::io::SeekFrom::Current(pos) => SeekFrom::Current(pos),
        };

        self.0.seek(pos).map_err(|_| Error::new(ErrorKind::Other, "Unknown error"))
    }
}

impl crate::Device for FileDevice {}
