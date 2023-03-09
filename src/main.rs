use bitvec::macros::internal::funty::Fundamental;
use bitvec::prelude::*;
use clap::{App, Arg};
use core::mem::size_of;
use core::time;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Result, Write};
use std::thread;

const CHUNK_SIZE: usize = 16 * 1024;
const BYTE_TO_BIT: usize = 8;

pub struct Args {
    infile: String,
    outfile: String,
    config: String,
    chunksize: usize,
    offset: usize,
    bitoffset: usize,
    rawhex: bool,
    rawbin: bool,
}

impl Args {
    pub fn parse() -> Self {
        let matches = App::new("mview")
            .arg(Arg::with_name("infile").help("Read from a file instead of stdin"))
            .arg(
                Arg::with_name("outfile")
                    .short('w')
                    .long("outfile")
                    .takes_value(true)
                    .help("Write output to a file instead of stdout"),
            )
            .arg(
                Arg::with_name("config")
                    .short('c')
                    .long("config")
                    .takes_value(true)
					.required(true)
                    .help("Definition of the datafields of a chunk"),
            )
            .arg(
                Arg::with_name("chunksize")
                    .short('b')
                    .long("chunksize")
                    .takes_value(true)
                    .value_parser(clap::value_parser!(usize))
                    .help("flush stdout and restart matching after n bytes"),
            )
            .arg(
                Arg::with_name("offset (bytes)")
                    .short('o')
                    .long("offset")
                    .takes_value(true)
                    .value_parser(clap::value_parser!(usize))
                    .help("offset in bytes at the start of a chunk before parsing starts"),
            )
            .arg(
                Arg::with_name("offset (bits)")
                    .short('p')
                    .long("bitoffset")
                    .takes_value(true)
                    .value_parser(clap::value_parser!(usize))
                    .help("offset in bits at the start of a chunk before parsing starts (added to --offset <bytes>)"),
            )
            .arg(
                Arg::with_name("rawhex")
                    .short('r')
                    .long("rawhex")
                    .takes_value(false)
                    .help("Print raw hexdump of the chunk at top of output"),
            )
            .arg(
                Arg::with_name("rawbin")
                    .long("rawbin")
                    .takes_value(false)
                    .help("Print raw bindump of the chunk at top of output"),
            )
            .get_matches();
        let infile = matches.value_of("infile").unwrap_or_default().to_string();
        let outfile = matches.value_of("outfile").unwrap_or_default().to_string();
        let config = matches.value_of("config").unwrap_or_default().to_string();
        let chunksize = matches
            .try_get_one::<usize>("chunksize")
            .unwrap_or_default()
            .unwrap();
        let offset = matches
            .try_get_one::<usize>("offset")
            .unwrap_or_default()
            .unwrap_or(&0);
        let bitoffset = matches
            .try_get_one::<usize>("bitoffset")
            .unwrap_or_default()
            .unwrap_or(&0);
        let rawhex = matches.is_present("rawhex");
        let rawbin = matches.is_present("rawbin");
        Self {
            infile,
            outfile,
            config,
            chunksize: *chunksize,
            offset: *offset,
            bitoffset: *bitoffset,
            rawhex,
            rawbin,
        }
    }
}

fn size_in_bits<T>() -> usize {
    size_of::<T>() * BYTE_TO_BIT
}

fn print_raws(c: &[u8], rawhex: bool, rawbin: bool, writer: &mut Box<dyn Write>) {
    if rawhex {
        writer.write_fmt(format_args!("{:02X?}\n", c)).unwrap();
    }
    if rawbin {
        for b in c {
            writer.write_fmt(format_args!("{:08b} ", b)).unwrap();
        }
        writer.write_all(b"\n\n").unwrap();
    }
}

fn main() -> Result<()> {
    // get args
    let args = Args::parse();
    let Args {
        infile,
        outfile,
        config,
        chunksize,
        offset,
        bitoffset,
        rawhex,
        rawbin,
    } = args;

    // create writer and reader
    let mut reader: Box<dyn Read> = if !infile.is_empty() {
        Box::new(BufReader::new(File::open(infile)?))
    } else {
        Box::new(BufReader::new(io::stdin()))
    };
    let mut writer: Box<dyn Write> = if !outfile.is_empty() {
        Box::new(BufWriter::new(File::create(outfile)?))
    } else {
        Box::new(BufWriter::new(io::stdout()))
    };

    // read config
    let conf_file = File::open(config)?;
    let conf_reader = BufReader::new(conf_file);
    let mut config_lines = vec![];
    for line in conf_reader.lines() {
        config_lines.push(line?);
    }

    let mut buffer = [0; CHUNK_SIZE];
    loop {
        let num_read = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(x) => x,
            Err(_) => break,
        };

        let s_buf = &buffer[..num_read];
        s_buf.chunks(chunksize).for_each(|c| {
            thread::sleep(time::Duration::new(1, 0)); // this is only here for debugging
            std::process::Command::new("clear").status().unwrap();
            print_raws(c, rawhex, rawbin, &mut writer);
            let c_bits = c.view_bits::<Msb0>();
            let mut bitpos_in_line = 0 + bitoffset + offset * size_in_bits::<u8>();
            for i in config_lines.iter() {
                if i.starts_with('#') {
                    // # is the symbol to comment out a config line
                    continue;
                }
                let (fieldname, rest) = i.split_once(':').unwrap();
                let (val_type, len) = match rest.split_once(':') {
                    Some(s) => (s.0, s.1.parse().unwrap_or_default()),
                    None => (rest, 0),
                };
                // let len: usize = len.parse().unwrap_or_default();
                writer.write_fmt(format_args!("{}", fieldname)).unwrap();
                writer.write_all(b": ").unwrap();
                match val_type {
                    "bool1" => {
                        writer
                            .write_fmt(format_args!("{}\n", c_bits[bitpos_in_line]))
                            .unwrap();
                        bitpos_in_line += 1;
                    }
                    "bool8" => {
                        if bitpos_in_line + size_in_bits::<u8>() <= c_bits.len() {
                            let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<u8>()];
                            myslice.copy_from_bitslice(
                                &c_bits[bitpos_in_line..bitpos_in_line + size_in_bits::<u8>()],
                            );
                            writer
                                .write_fmt(format_args!("{}\n", &myslice[0..8].load::<u8>()))
                                .unwrap();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                        bitpos_in_line += size_in_bits::<u8>();
                    }
                    "u8" => {
                        if bitpos_in_line + size_in_bits::<u8>() <= c_bits.len() {
                            let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<u8>()];
                            myslice.copy_from_bitslice(
                                &c_bits[bitpos_in_line..bitpos_in_line + size_in_bits::<u8>()],
                            );
                            writer
                                .write_fmt(format_args!("{}\n", &myslice[0..8].load::<u8>()))
                                .unwrap();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                        bitpos_in_line += size_in_bits::<u8>();
                    }
                    "u16" => {
                        if bitpos_in_line + size_in_bits::<u16>() <= c_bits.len() {
                            let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<u16>()];
                            myslice.copy_from_bitslice(
                                &c_bits[bitpos_in_line..bitpos_in_line + size_in_bits::<u16>()],
                            );
                            writer
                                .write_fmt(format_args!("{}\n", &myslice[0..16].load::<u16>()))
                                .unwrap();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                        bitpos_in_line += size_in_bits::<u16>();
                    }
                    "u32" => {
                        if bitpos_in_line + size_in_bits::<u32>() <= c_bits.len() {
                            let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<u32>()];
                            myslice.copy_from_bitslice(
                                &c_bits[bitpos_in_line..bitpos_in_line + size_in_bits::<u32>()],
                            );
                            writer
                                .write_fmt(format_args!("{}\n", &myslice[0..32].load::<u32>()))
                                .unwrap();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                        bitpos_in_line += size_in_bits::<u32>();
                    }
                    "u64" => {
                        if bitpos_in_line + size_in_bits::<u64>() <= c_bits.len() {
                            let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<u64>()];
                            myslice.copy_from_bitslice(
                                &c_bits[bitpos_in_line..bitpos_in_line + size_in_bits::<u64>()],
                            );
                            writer
                                .write_fmt(format_args!("{}\n", &myslice[0..64].load::<u64>()))
                                .unwrap();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                        bitpos_in_line += size_in_bits::<u64>();
                    }
                    "u128" => {
                        if bitpos_in_line + size_in_bits::<u128>() <= c_bits.len() {
                            let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<u128>()];
                            myslice.copy_from_bitslice(
                                &c_bits[bitpos_in_line..bitpos_in_line + size_in_bits::<u128>()],
                            );
                            writer
                                .write_fmt(format_args!("{}\n", &myslice[0..128].load::<u128>()))
                                .unwrap();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                        bitpos_in_line += size_in_bits::<u128>();
                    }
                    "i8" => {
                        if bitpos_in_line + size_in_bits::<u8>() <= c_bits.len() {
                            let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<i8>()];
                            myslice.copy_from_bitslice(
                                &c_bits[bitpos_in_line..bitpos_in_line + size_in_bits::<i8>()],
                            );
                            writer
                                .write_fmt(format_args!("{}\n", &myslice[0..8].load::<i8>()))
                                .unwrap();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                        bitpos_in_line += size_in_bits::<i8>();
                    }
                    "i16" => {
                        if bitpos_in_line + size_in_bits::<i16>() <= c_bits.len() {
                            let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<i16>()];
                            myslice.copy_from_bitslice(
                                &c_bits[bitpos_in_line..bitpos_in_line + size_in_bits::<i16>()],
                            );
                            writer
                                .write_fmt(format_args!("{}\n", &myslice[0..16].load::<i16>()))
                                .unwrap();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                        bitpos_in_line += size_in_bits::<i16>();
                    }
                    "i32" => {
                        if bitpos_in_line + size_in_bits::<i32>() <= c_bits.len() {
                            let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<i32>()];
                            myslice.copy_from_bitslice(
                                &c_bits[bitpos_in_line..bitpos_in_line + size_in_bits::<i32>()],
                            );
                            writer
                                .write_fmt(format_args!("{}\n", &myslice[0..32].load::<i32>()))
                                .unwrap();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                        bitpos_in_line += size_in_bits::<i32>();
                    }
                    "i64" => {
                        if bitpos_in_line + size_in_bits::<i64>() <= c_bits.len() {
                            let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<i64>()];
                            myslice.copy_from_bitslice(
                                &c_bits[bitpos_in_line..bitpos_in_line + size_in_bits::<i64>()],
                            );
                            writer
                                .write_fmt(format_args!("{}\n", &myslice[0..64].load::<i64>()))
                                .unwrap();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                        bitpos_in_line += size_in_bits::<i64>();
                    }
                    "i128" => {
                        if bitpos_in_line + size_in_bits::<i128>() <= c_bits.len() {
                            let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<i128>()];
                            myslice.copy_from_bitslice(
                                &c_bits[bitpos_in_line..bitpos_in_line + size_in_bits::<i128>()],
                            );
                            writer
                                .write_fmt(format_args!("{}\n", &myslice[0..128].load::<i128>()))
                                .unwrap();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                    }
                    "f32" => {
                        if bitpos_in_line + size_in_bits::<f32>() <= c_bits.len() {
                            let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<f32>()];
                            myslice.copy_from_bitslice(
                                &c_bits[bitpos_in_line..bitpos_in_line + size_in_bits::<f32>()],
                            );
                            writer
                                .write_fmt(format_args!(
                                    "{}\n",
                                    &myslice[0..32].load::<u32>().as_f32()
                                ))
                                .unwrap();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                        bitpos_in_line += size_in_bits::<f32>();
                    }
                    "f64" => {
                        if bitpos_in_line + size_in_bits::<f64>() <= c_bits.len() {
                            let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<f64>()];
                            myslice.copy_from_bitslice(
                                &c_bits[bitpos_in_line..bitpos_in_line + size_in_bits::<f64>()],
                            );
                            writer
                                .write_fmt(format_args!(
                                    "{}\n",
                                    &myslice[0..64].load::<u64>().as_f64()
                                ))
                                .unwrap();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                        bitpos_in_line += size_in_bits::<f64>();
                    }
                    "string" | "String" => {
                        if bitpos_in_line + len * size_in_bits::<u8>() <= c_bits.len() {
                            for _i in 0..len {
                                writer
                                    .write_fmt(format_args!(
                                        "{}",
                                        c_bits
                                            [bitpos_in_line..bitpos_in_line + size_in_bits::<u8>()]
                                            .load::<u8>()
                                            as char
                                    ))
                                    .unwrap();
                                bitpos_in_line += size_in_bits::<u8>();
                            }
                            writer.write_fmt(format_args!("\n")).unwrap();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                    }
                    "iarb" => {
                        if bitpos_in_line + len <= c_bits.len() {
                            let target_int;
                            let negative = c_bits[bitpos_in_line + len - 1];
                            let mut target_slice: [u8; 16] = [0; 16];
                            let int_bits = target_slice.view_bits_mut::<Lsb0>();
                            for i in 0..len {
                                int_bits.set(i, c_bits[bitpos_in_line + i]); // copy the payload over
                            }
                            if negative {
                                // integer is negative, do the twos complement
                                for i in len..int_bits.len() {
                                    int_bits.set(i, !int_bits[i]); // flip all bits from the sign bit to end
                                }
                                target_int = int_bits.load::<i128>(); // add 1
                            } else {
                                target_int = int_bits.load::<i128>();
                            }
                            writer.write_fmt(format_args!("{}\n", target_int)).unwrap();
                            bitpos_in_line += len;
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                    }
                    "uarb" => {
                        if bitpos_in_line + len <= c_bits.len() {
                            let target_int;
                            let mut target_slice: [u8; 16] = [0; 16];
                            let int_bits = target_slice.view_bits_mut::<Lsb0>();
                            for i in 0..len {
                                int_bits.set(i, c_bits[bitpos_in_line + i]); // copy the payload over
                            }
                            target_int = int_bits.load::<i128>();
                            writer.write_fmt(format_args!("{}\n", target_int)).unwrap();
                            bitpos_in_line += len;
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                    }
                    "bytegap" => {
                        if bitpos_in_line + len * size_in_bits::<u8>() <= c_bits.len() {
                            writer
                                .write_fmt(format_args!("(gap of {} byte)\n", len))
                                .unwrap();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                        bitpos_in_line += len * size_in_bits::<u8>();
                    }
                    "bitgap" => {
                        if bitpos_in_line + len <= c_bits.len() {
                            writer
                                .write_fmt(format_args!("(gap of {} bit)\n", len))
                                .unwrap();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                        bitpos_in_line += len;
                    }
                    _ => eprintln!("unknown type"),
                }
                writer.flush().unwrap();
            }
        });
        buffer.fill(0);
    }
    Ok(())
}
