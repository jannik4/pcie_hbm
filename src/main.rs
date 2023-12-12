use anyhow::{Context, Result};
use std::{
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom, Write},
};

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
    let path = format!("/dev/xdma0_h2c_{}", channel);
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .with_context(|| format!("Failed to open {} for writing.", path))?;

    file.seek(SeekFrom::Start(addr))
        .with_context(|| format!("Failed to seek to {} in {} for writing.", addr, path))?;
    file.write_all(buf)
        .with_context(|| format!("Failed to write to {} in {}.", addr, path))?;

    Ok(())
}

fn read(channel: u32, addr: u64, buf: &mut [u8]) -> Result<()> {
    let path = format!("/dev/xdma0_h2c_{}", channel);
    let mut file = OpenOptions::new()
        .read(true)
        .open(&path)
        .with_context(|| format!("Failed to open {} for reading.", path))?;

    file.seek(SeekFrom::Start(addr))
        .with_context(|| format!("Failed to seek to {} in {} for reading.", addr, path))?;
    file.read_exact(buf)
        .with_context(|| format!("Failed to read from {} in {}.", addr, path))?;

    Ok(())
}
