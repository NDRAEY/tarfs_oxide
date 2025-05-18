use std::fs::File;
use tarfs::{file_device::FileDevice, TarFS};

fn main() {
    let filename = match std::env::args().skip(1).next() {
        Some(f) => f,
        None => {
            eprintln!("Please provide the name of a file.");
            std::process::exit(1);    
        },
    };

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
