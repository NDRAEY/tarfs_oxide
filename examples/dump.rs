use std::{fs::File, io::{Read, Seek, SeekFrom}};

use tarfs::TarFS;

struct FileDevice(File);

impl tarfs::io::Read for FileDevice {
    fn read(&mut self, position: usize, size: usize, buffer: &mut [u8]) -> Option<()> {
        // println!("Seek and read: 0x{:x}", position);

        if self.0.seek(SeekFrom::Start(position as u64)).is_err() {
            return None;
        }

        if self.0.read_exact(&mut buffer[..size]).is_ok() {
            Some(())
        } else {
            None
        }
    }
}

fn main() {
    let args = std::env::args();

    if args.len() <= 1 {
        eprintln!("Please provide the name of a file.");
        std::process::exit(1);
    }

    let filename = args.last().unwrap();

    let fs = TarFS::from_device(FileDevice(File::open(filename).unwrap()));

    if fs.is_none() {
        println!("Failed to open TAR file.");
        return;
    }

    let mut fs = fs.unwrap();
    
    let ents = fs.list();

    for i in ents {
        println!("{:40} - {:12} bytes", &i.name, i.size);
    }
}
