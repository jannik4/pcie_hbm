use anyhow::{Context, Result};
use humansize::{ISizeFormatter, SizeFormatter, BINARY};
use parse_size::parse_size;
use std::{
    fmt,
    fs::OpenOptions,
    io::{self, Read, Seek, SeekFrom, Write},
    time::{Duration, Instant},
};

fn main() -> Result<()> {
    // Parse args
    let args = Args::from_env().context("failed to parse args")?;

    // Print args
    println!("{}", args);

    // Prepare data
    let buf_write = (0..args.size).map(|v| (v % 256) as u8).collect::<Vec<_>>();

    // Write
    let duration = write(0, args.addr, &buf_write, args.chunk_size)?;
    println!(
        "Write was successful ({:?} @ {}/s).",
        duration,
        ISizeFormatter::new(args.size as f64 / duration.as_secs_f64(), BINARY),
    );

    // Read
    let mut buf_read = vec![0; args.size as usize];
    let duration = read(0, args.addr, &mut buf_read, args.chunk_size)?;
    assert_eq!(buf_write, buf_read);
    println!(
        "Read was successful ({:?} @ {}/s).",
        duration,
        ISizeFormatter::new(args.size as f64 / duration.as_secs_f64(), BINARY),
    );

    Ok(())
}

#[derive(Debug)]
struct Args {
    channel: u32,
    addr: u64,
    size: u64,
    chunk_size: Option<u64>,
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
                .opt_value_from_fn(["-a", "--addr"], parse_num)?
                .unwrap_or(default.addr),
            size: args
                .opt_value_from_fn(["-s", "--size"], |s| parse_size(s))?
                .unwrap_or(default.size),
            chunk_size: args.opt_value_from_fn(["-k", "--chunk-size"], |s| parse_size(s))?,
        })
    }
}

impl Default for Args {
    fn default() -> Self {
        Self {
            channel: 0,
            addr: 0,
            size: 1024,
            chunk_size: None,
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
            "Size: {} ({} bytes)",
            SizeFormatter::new(self.size, BINARY),
            self.size
        )?;
        Ok(())
    }
}

fn parse_num(s: &str) -> Result<u64, std::num::ParseIntError> {
    if let Some(s) = s.strip_prefix("0x") {
        u64::from_str_radix(s, 16)
    } else if let Some(s) = s.strip_prefix("0b") {
        u64::from_str_radix(s, 2)
    } else {
        s.parse::<u64>()
    }
}

fn write(channel: u32, addr: u64, buf: &[u8], chunk_size: Option<u64>) -> Result<Duration> {
    let path = format!("/dev/xdma0_h2c_{}", channel);
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .with_context(|| format!("Failed to open {} for writing.", path))?;

    let start = Instant::now();

    file.seek(SeekFrom::Start(addr))
        .with_context(|| format!("Failed to seek to {} in {} for writing.", addr, path))?;

    let res = match chunk_size {
        Some(chunk_size) => write_all_chunked(&mut file, buf, chunk_size as usize),
        None => file.write_all(buf),
    };
    res.with_context(|| format!("Failed to write to {} in {}.", addr, path))?;

    let duration = start.elapsed();

    Ok(duration)
}

fn read(channel: u32, addr: u64, buf: &mut [u8], chunk_size: Option<u64>) -> Result<Duration> {
    let path = format!("/dev/xdma0_c2h_{}", channel);
    let mut file = OpenOptions::new()
        .read(true)
        .open(&path)
        .with_context(|| format!("Failed to open {} for reading.", path))?;

    let start = Instant::now();

    file.seek(SeekFrom::Start(addr))
        .with_context(|| format!("Failed to seek to {} in {} for reading.", addr, path))?;

    let res = match chunk_size {
        Some(chunk_size) => read_exact_chunked(&mut file, buf, chunk_size as usize),
        None => file.read_exact(buf),
    };
    res.with_context(|| format!("Failed to read from {} in {}.", addr, path))?;

    let duration = start.elapsed();

    Ok(duration)
}

fn write_all_chunked(writer: &mut impl Write, mut buf: &[u8], chunk_size: usize) -> io::Result<()> {
    while !buf.is_empty() {
        match writer.write(&buf[..usize::min(chunk_size, buf.len())]) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "failed to write whole buffer",
                ));
            }
            Ok(n) => {
                buf = &buf[n..];
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

fn read_exact_chunked(
    this: &mut impl Read,
    mut buf: &mut [u8],
    chunk_size: usize,
) -> io::Result<()> {
    while !buf.is_empty() {
        let n = usize::min(chunk_size, buf.len());
        match this.read(&mut buf[..n]) {
            Ok(0) => break,
            Ok(n) => {
                let tmp = buf;
                buf = &mut tmp[n..];
            }
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    if !buf.is_empty() {
        Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "failed to fill whole buffer",
        ))
    } else {
        Ok(())
    }
}
