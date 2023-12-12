use anyhow::{Context, Result};
use bytesize::ByteSize;
use std::{
    fmt,
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom, Write},
};

fn main() -> Result<()> {
    // Parse args
    let args = Args::from_env();

    // Print args
    println!("{}", args);

    // Prepare data
    let buf_write = (0..args.size.0)
        .map(|v| (v % 256) as u8)
        .collect::<Vec<_>>();

    // Write
    write(0, 0, &buf_write)?;
    println!("Write was successful.");

    // Read
    let mut buf_read = vec![0; args.size.0 as usize];
    read(0, 0, &mut buf_read)?;
    assert_eq!(buf_write, buf_read);
    println!("Read was successful.");

    Ok(())
}

#[derive(Debug)]
struct Args {
    channel: u32,
    addr: u64,
    size: ByteSize,
}

impl Args {
    fn from_env() -> Self {
        let default = Self::default();
        let mut args = pico_args::Arguments::from_env();
        Self {
            channel: args
                .value_from_str(["-c", "--channel"])
                .unwrap_or(default.channel),
            addr: args
                .value_from_str(["-a", "--addr"])
                .unwrap_or(default.addr),
            size: args
                .value_from_str(["-s", "--size"])
                .unwrap_or(default.size),
        }
    }
}

impl Default for Args {
    fn default() -> Self {
        Self {
            channel: 0,
            addr: 0,
            size: ByteSize::kib(1),
        }
    }
}

impl fmt::Display for Args {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Channel: {}", self.channel)?;
        writeln!(f, "Address: 0x{:x}", self.addr)?;
        writeln!(f, "Size: {}", self.size)?;
        Ok(())
    }
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
    let path = format!("/dev/xdma0_c2h_{}", channel);
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
