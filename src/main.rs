use bitvec::macros::internal::funty::Fundamental;
use bitvec::prelude::*;
use clap::{App, Arg};
use core::mem::size_of;
use core::time;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Result, Write, BufRead};
use std::thread;

const CHUNK_SIZE: usize = 16 * 1024;

pub struct Args {
    infile: String,
    outfile: String,
	config: String,
    chunksize: usize,
    offset: usize,
    bitoffset: usize,
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
        Self {
            infile,
            outfile,
			config,
            chunksize: *chunksize,
            offset: *offset,
            bitoffset: *bitoffset,
        }
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
            thread::sleep(time::Duration::new(1, 0));
            std::process::Command::new("clear").status().unwrap();
            let c_bits = c.view_bits::<Msb0>();
            let mut bitpos_in_line = 0 + bitoffset + offset * size_of::<u8>();
            for i in config_lines.iter() {
                let (fieldname, val_type) = i.split_once(':').unwrap();
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
                        if bitpos_in_line + size_of::<u8>() < c_bits.len() {
                            writer
                                .write_fmt(format_args!(
                                    "{}\n",
                                    (c_bits[bitpos_in_line..bitpos_in_line + size_of::<u8>()]
                                        .load::<u8>()
                                        > 0)
                                ))
                                .unwrap();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                        bitpos_in_line += size_of::<u8>();
                    }
                    "u8" => {
                        if bitpos_in_line + size_of::<u8>() < c_bits.len() {
                            writer
                                .write_fmt(format_args!(
                                    "{}\n",
                                    c_bits[bitpos_in_line..bitpos_in_line + size_of::<u8>()]
                                        .load::<u8>()
                                ))
                                .unwrap();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                        bitpos_in_line += size_of::<u8>();
                    }
                    "u16" => {
                        if bitpos_in_line + size_of::<u16>() < c_bits.len() {
                            writer
                                .write_fmt(format_args!(
                                    "{}\n",
                                    c_bits[bitpos_in_line..bitpos_in_line + size_of::<u16>()]
                                        .load::<u16>()
                                ))
                                .unwrap();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                        bitpos_in_line += size_of::<u16>();
                    }
                    "u32" => {
                        if bitpos_in_line + size_of::<u32>() < c_bits.len() {
                            writer
                                .write_fmt(format_args!(
                                    "{}\n",
                                    c_bits[bitpos_in_line..bitpos_in_line + size_of::<u32>()]
                                        .load::<u32>()
                                ))
                                .unwrap();
                            bitpos_in_line += size_of::<u32>();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                    }
                    "u64" => {
                        if bitpos_in_line + size_of::<u64>() < c_bits.len() {
                            writer
                                .write_fmt(format_args!(
                                    "{}\n",
                                    c_bits[bitpos_in_line..bitpos_in_line + size_of::<u64>()]
                                        .load::<u64>()
                                ))
                                .unwrap();
                            bitpos_in_line += size_of::<u64>();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                    }
                    "u128" => {
                        if bitpos_in_line + size_of::<u128>() < c_bits.len() {
                            writer
                                .write_fmt(format_args!(
                                    "{}\n",
                                    c_bits[bitpos_in_line..bitpos_in_line + size_of::<u128>()]
                                        .load::<u128>()
                                ))
                                .unwrap();
                            bitpos_in_line += size_of::<u128>();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                    }
                    "i8" => {
                        if bitpos_in_line + size_of::<u8>() < c_bits.len() {
                            writer
                                .write_fmt(format_args!(
                                    "{}\n",
                                    c_bits[bitpos_in_line..bitpos_in_line + size_of::<i8>()]
                                        .load::<i8>()
                                ))
                                .unwrap();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                        bitpos_in_line += size_of::<u8>();
                    }
                    "i16" => {
                        if bitpos_in_line + size_of::<i16>() < c_bits.len() {
                            writer
                                .write_fmt(format_args!(
                                    "{}\n",
                                    c_bits[bitpos_in_line..bitpos_in_line + size_of::<i16>()]
                                        .load::<i16>()
                                ))
                                .unwrap();
                            bitpos_in_line += size_of::<i16>();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                    }
                    "i32" => {
                        if bitpos_in_line + size_of::<i32>() < c_bits.len() {
                            writer
                                .write_fmt(format_args!(
                                    "{}\n",
                                    c_bits[bitpos_in_line..bitpos_in_line + size_of::<i32>()]
                                        .load::<i32>()
                                ))
                                .unwrap();
                            bitpos_in_line += size_of::<i32>();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                    }
                    "i64" => {
                        if bitpos_in_line + size_of::<i64>() < c_bits.len() {
                            writer
                                .write_fmt(format_args!(
                                    "{}\n",
                                    c_bits[bitpos_in_line..bitpos_in_line + size_of::<i64>()]
                                        .load::<i64>()
                                ))
                                .unwrap();
                            bitpos_in_line += size_of::<i64>();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                    }
                    "i128" => {
                        if bitpos_in_line + size_of::<i128>() < c_bits.len() {
                            writer
                                .write_fmt(format_args!(
                                    "{}\n",
                                    c_bits[bitpos_in_line..bitpos_in_line + size_of::<i128>()]
                                        .load::<i128>()
                                ))
                                .unwrap();
                            bitpos_in_line += size_of::<i128>();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                    }
                    "f32" => {
                        if bitpos_in_line + size_of::<f32>() < c_bits.len() {
                            writer
                                .write_fmt(format_args!(
                                    "{}\n",
                                    c_bits[bitpos_in_line..bitpos_in_line + size_of::<u32>()]
                                        .load::<u32>()
                                        .as_f32()
                                ))
                                .unwrap();
                            bitpos_in_line += size_of::<f32>();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
                    }
                    "f64" => {
                        if bitpos_in_line + size_of::<f64>() < c_bits.len() {
                            writer
                                .write_fmt(format_args!(
                                    "{}\n",
                                    c_bits[bitpos_in_line..bitpos_in_line + size_of::<f64>()]
                                        .load::<u64>()
                                        .as_f64()
                                ))
                                .unwrap();
                            bitpos_in_line += size_of::<f64>();
                        } else {
                            writer
                                .write_all(
                                    b"values size is bigger than what is left of that data chunk\n",
                                )
                                .unwrap();
                        }
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

#[cfg(test)]
mod tests {
    #[test]
    fn multiply_function() {
        let result = 5 * 5;
        assert_eq!(result, 25);
    }
}
