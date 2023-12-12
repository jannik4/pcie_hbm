use anyhow::{Context, Result};
use humansize::{SizeFormatter, BINARY};
use parse_size::parse_size;
use std::{
    fmt,
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom, Write},
};

fn main() -> Result<()> {
    // Parse args
    let args = Args::from_env().context("failed to parse args")?;

    // Print args
    println!("{}", args);

    // Prepare data
    let buf_write = (0..args.size).map(|v| (v % 256) as u8).collect::<Vec<_>>();

    // Write
    write(0, args.addr, &buf_write)?;
    println!("Write was successful.");

    // Read
    let mut buf_read = vec![0; args.size as usize];
    read(0, args.addr, &mut buf_read)?;
    assert_eq!(buf_write, buf_read);
    println!("Read was successful.");

    Ok(())
}

#[derive(Debug)]
struct Args {
    channel: u32,
    addr: u64,
    size: u64,
}

impl Args {
    fn from_env() -> Result<Self> {
        let default = Self::default();
        let mut args = pico_args::Arguments::from_env();
        Ok(Self {
            channel: args
                .opt_value_from_str(["-c", "--channel"])?
                .unwrap_or(default.channel),
            addr: args
                .opt_value_from_str(["-a", "--addr"])?
                .unwrap_or(default.addr),
            size: args
                .opt_value_from_fn(["-s", "--size"], |s| parse_size(s))?
                .unwrap_or(default.size),
        })
    }
}

impl Default for Args {
    fn default() -> Self {
        Self {
            channel: 0,
            addr: 0,
            size: 1024,
        }
    }
}

impl fmt::Display for Args {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Channel: {}", self.channel)?;
        writeln!(
            f,
            "Address: 0x{:x} - 0x{:x}",
            self.addr,
            self.addr + self.size - 1
        )?;
        writeln!(
            f,
            "Size: {} ({})",
            SizeFormatter::new(self.size, BINARY),
            self.size
        )?;
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
