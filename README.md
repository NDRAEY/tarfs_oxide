# TarFS

This is a no_std implementation of Tar archive format reader.

This crate's architecture allows to be usable in embedded systems like operating system kernels.

# Devices?

This crate uses "devices" as an universal interface to read data.

You can implement `Device` trait (that also needs `no_std_io::io::Read` and `no_std_io::io::Seek` to be implemented) for your structure and use it with `tarfs`.

See `src/file_device.rs` for approximate implementation.

# Usage

Add this crate by running this command:

```
cargo add tarfs
```

# Example

Here's a simple example to list all entries in archive:

```rust
    let fs = TarFS::from_device(FileDevice(File::open("archive.tar").unwrap()));

    if fs.is_none() {
        println!("Failed to open TAR file.");
        return;
    }

    let mut fs = fs.unwrap();

    let entries = fs.list().unwrap();

    for i in entries {
        println!("Entry `{}`; Size: `{}`", &i.name, i.size);
    }
```

Read text file to string:
```rust
    let lore: String = fs.read_to_string("/Ninjago Lore.txt")?;
```

Read binary file:
```rust
    let mut data = vec![0; 32];

    fs.read_file("/ScientificData.bin", /* position */ 0, /* size */ 32, &mut data)?;
```

Read API reference on [docs.rs](https://docs.rs/tarfs/latest/tarfs/).