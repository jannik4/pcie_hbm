use std::{
    error::Error as StdError,
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom, Write},
};

type Error = Box<dyn StdError>;
type Result<T, E = Error> = std::result::Result<T, E>;

fn main() -> Result<()> {
    println!("Hello, world!");

    // Prepare data
    let size = 1024;
    let buf_write = (0..size).map(|v| (v % 256) as u8).collect::<Vec<_>>();

    // Write
    write(0, 0, &buf_write)?;

    // Read
    let mut buf_read = vec![0; size];
    read(0, 0, &mut buf_read)?;
    assert_eq!(buf_write, buf_read);

    Ok(())
}

fn write(channel: u32, addr: u64, buf: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(format!("/dev/xdma0_h2c_{}", channel))?;

    file.seek(SeekFrom::Start(addr))?;
    file.write_all(buf)?;

    Ok(())
}

fn read(channel: u32, addr: u64, buf: &mut [u8]) -> Result<()> {
    let mut file = OpenOptions::new()
        .read(true)
        .open(format!("/dev/xdma0_h2c_{}", channel))?;

    file.seek(SeekFrom::Start(addr))?;
    file.read_exact(buf)?;

    Ok(())
}
