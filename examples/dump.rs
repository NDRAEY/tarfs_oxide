use no_std_io::{self, io::{Error, ErrorKind, Result}};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};
use tarfs::{file_device::FileDevice, TarFS};

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

    let ents = fs.list().unwrap();

    for i in ents {
        println!("{:40} - {:12} bytes", &i.name, i.size);
    }
}
