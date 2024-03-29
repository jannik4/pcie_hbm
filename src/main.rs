use anyhow::{Context, Result};
use humansize::{ISizeFormatter, SizeFormatter, BINARY};
use parse_size::parse_size;
use std::{
    fmt,
    fs::{File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    thread,
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
    let duration = write(&args, &buf_write, args.warmup)?;
    println!(
        "Write was successful ({:?} @ {}/s).",
        duration,
        ISizeFormatter::new(args.size as f64 / duration.as_secs_f64(), BINARY),
    );

    // Read
    let mut buf_read = vec![0; args.size as usize];
    let duration = read(&args, &mut buf_read, args.warmup)?;
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
    channel: Option<u32>,
    addr: u64,
    size: u64,
    chunk_size: Option<u64>,
    warmup: bool,
}

impl Args {
    fn from_env() -> Result<Self> {
        let default = Self::default();
        let mut args = pico_args::Arguments::from_env();
        Ok(Self {
            channel: args
                .opt_value_from_str(["-c", "--channel"])?
                .or(default.channel),
            addr: args
                .opt_value_from_fn(["-a", "--addr"], parse_num)?
                .unwrap_or(default.addr),
            size: args
                .opt_value_from_fn(["-s", "--size"], |s| parse_size(s))?
                .unwrap_or(default.size),
            chunk_size: args
                .opt_value_from_fn(["-k", "--chunk-size"], |s| parse_size(s))?
                .or(default.chunk_size),
            warmup: args.contains(["-w", "--warmup"]),
        })
    }
}

impl Default for Args {
    fn default() -> Self {
        Self {
            channel: None,
            addr: 0,
            size: 1024,
            chunk_size: None,
            warmup: false,
        }
    }
}

impl fmt::Display for Args {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.channel {
            Some(channel) => writeln!(f, "Channel: {}", channel)?,
            None => writeln!(f, "Channel: all")?,
        }
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

struct PcieWriter {
    channel: u32,
    file: File,
}

impl PcieWriter {
    fn new(channel: u32) -> Result<Self> {
        let path = format!("/dev/xdma0_h2c_{}", channel);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .with_context(|| format!("Failed to open {} for writing.", path))?;
        Ok(Self { channel, file })
    }

    fn write(&mut self, addr: u64, buf: &[u8], chunk_size: Option<u64>) -> Result<()> {
        self.file.seek(SeekFrom::Start(addr)).with_context(|| {
            format!(
                "Failed to seek to {} in channel {} for writing.",
                addr, self.channel
            )
        })?;

        let res = match chunk_size {
            Some(chunk_size) => write_all_chunked(&mut self.file, buf, chunk_size as usize),
            None => self.file.write_all(buf),
        };
        res.with_context(|| format!("Failed to write to {} in channel {}.", addr, self.channel))?;

        Ok(())
    }

    fn write_parallel(
        worker: &mut [Self],
        addr: u64,
        buf: &[u8],
        chunk_size: Option<u64>,
    ) -> Result<()> {
        thread::scope(|s| {
            let buf_chunk_size = usize::div_ceil(buf.len(), worker.len());
            let threads = worker
                .iter_mut()
                .zip(buf.chunks(buf_chunk_size))
                .map(|(writer, buf)| s.spawn(|| writer.write(addr, buf, chunk_size)))
                .collect::<Vec<_>>();

            for thread in threads {
                thread.join().unwrap()?;
            }

            Ok(())
        })
    }
}

fn write(args: &Args, buf: &[u8], warmup: bool) -> Result<Duration> {
    match args.channel {
        Some(channel) => {
            let mut writer = PcieWriter::new(channel)?;

            if warmup {
                do_warmup(|| writer.write(args.addr, buf, args.chunk_size))?;
            }

            let start = Instant::now();
            writer.write(args.addr, buf, args.chunk_size)?;
            let duration = start.elapsed();

            Ok(duration)
        }
        None => {
            let mut worker = (0..4).map(PcieWriter::new).collect::<Result<Vec<_>>>()?;

            if warmup {
                do_warmup(|| {
                    PcieWriter::write_parallel(&mut worker, args.addr, buf, args.chunk_size)
                })?;
            }

            let start = Instant::now();
            PcieWriter::write_parallel(&mut worker, args.addr, buf, args.chunk_size)?;
            let duration = start.elapsed();

            Ok(duration)
        }
    }
}

struct PcieReader {
    channel: u32,
    file: File,
}

impl PcieReader {
    fn new(channel: u32) -> Result<Self> {
        let path = format!("/dev/xdma0_c2h_{}", channel);
        let file = OpenOptions::new()
            .read(true)
            .open(&path)
            .with_context(|| format!("Failed to open {} for reading.", path))?;
        Ok(Self { channel, file })
    }

    fn read(&mut self, addr: u64, buf: &mut [u8], chunk_size: Option<u64>) -> Result<()> {
        self.file.seek(SeekFrom::Start(addr)).with_context(|| {
            format!(
                "Failed to seek to {} in channel {} for reading.",
                addr, self.channel
            )
        })?;

        let res = match chunk_size {
            Some(chunk_size) => read_exact_chunked(&mut self.file, buf, chunk_size as usize),
            None => self.file.read_exact(buf),
        };
        res.with_context(|| format!("Failed to read from {} in channel {}.", addr, self.channel))?;

        Ok(())
    }

    fn read_parallel(
        worker: &mut [Self],
        addr: u64,
        buf: &mut [u8],
        chunk_size: Option<u64>,
    ) -> Result<()> {
        thread::scope(|s| {
            let buf_chunk_size = usize::div_ceil(buf.len(), worker.len());
            let threads = worker
                .iter_mut()
                .zip(buf.chunks_mut(buf_chunk_size))
                .map(|(reader, buf)| s.spawn(|| reader.read(addr, buf, chunk_size)))
                .collect::<Vec<_>>();

            for thread in threads {
                thread.join().unwrap()?;
            }

            Ok(())
        })
    }
}

fn read(args: &Args, buf: &mut [u8], warmup: bool) -> Result<Duration> {
    match args.channel {
        Some(channel) => {
            let mut reader = PcieReader::new(channel)?;

            if warmup {
                do_warmup(|| reader.read(args.addr, buf, args.chunk_size))?;
            }

            let start = Instant::now();
            reader.read(args.addr, buf, args.chunk_size)?;
            let duration = start.elapsed();

            Ok(duration)
        }
        None => {
            let mut worker = (0..4).map(PcieReader::new).collect::<Result<Vec<_>>>()?;

            if warmup {
                do_warmup(|| {
                    PcieReader::read_parallel(&mut worker, args.addr, buf, args.chunk_size)
                })?;
            }

            let start = Instant::now();
            PcieReader::read_parallel(&mut worker, args.addr, buf, args.chunk_size)?;
            let duration = start.elapsed();

            Ok(duration)
        }
    }
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

fn do_warmup(mut f: impl FnMut() -> Result<()>) -> Result<()> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(1) {
        f()?;
    }
    Ok(())
}
