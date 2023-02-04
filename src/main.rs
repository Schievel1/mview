use bitvec::prelude::*;
use clap::{App, Arg};
use core::mem::size_of;
use core::time;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Result, Write};
use std::thread;

const CHUNK_SIZE: usize = 16 * 1024;

pub struct Args {
    infile: String,
    outfile: String,
    chunksize: usize,
}

impl Args {
    pub fn parse() -> Self {
        let matches = App::new("mview")
            .arg(Arg::with_name("infile").help("Read from a file instead of stdin"))
            .arg(
                Arg::with_name("outfile")
                    .short('o')
                    .long("outfile")
                    .takes_value(true)
                    .help("Write output to a file instead of stdout"),
            )
            .arg(
                Arg::with_name("chunksize")
                    .short('c')
                    .long("chunksize")
                    .takes_value(true)
                    .value_parser(clap::value_parser!(usize))
                    .help("flush stdout and restart matching after n bytes"),
            )
            .get_matches();
        let infile = matches.value_of("infile").unwrap_or_default().to_string();
        let outfile = matches.value_of("outfile").unwrap_or_default().to_string();
        let chunksize = matches
            .try_get_one::<usize>("chunksize")
            .unwrap_or_default()
            .unwrap();
        Self {
            infile,
            outfile,
            chunksize: *chunksize,
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let Args {
        infile,
        outfile,
        chunksize,
    } = args;
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

    let mut buffer = [0; CHUNK_SIZE];
    loop {
        let num_read = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(x) => x,
            Err(_) => break,
        };

        let config = vec![
            "Field0:bool1",
            "Field1:bool1",
            "Field2:bool1",
            "Field3:bool1",
            "Field4:bool1",
            "Field5:bool1",
            "Field6:bool1",
            "Field7:bool1",
            "Field8:u8",
            "Field9:u16",
            "Field10:i32",
        ];

        let s_buf = &buffer[..num_read];
        s_buf.chunks(chunksize).for_each(|c| {
            let c_bits = c.view_bits::<Msb0>();
            let mut bitpos_in_line = 0;
            for i in config.iter() {
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
                    "u8" => {
                        writer
                            .write_fmt(format_args!(
                                "{}\n",
                                c_bits[bitpos_in_line..bitpos_in_line + size_of::<u8>()]
                                    .load::<u8>()
                            ))
                            .unwrap();
                        bitpos_in_line += 8;
                    }
                    "i8" => {
                        writer
                            .write_fmt(format_args!(
                                "{}\n",
                                c_bits[bitpos_in_line..bitpos_in_line + size_of::<i8>()]
                                    .load::<i8>()
                            ))
                            .unwrap();
                        bitpos_in_line += 8;
                    }
                    "u16" => {
                        writer
                            .write_fmt(format_args!(
                                "{}\n",
                                c_bits[bitpos_in_line..bitpos_in_line + size_of::<u16>()]
                                    .load::<u16>()
                            ))
                            .unwrap();
                        bitpos_in_line += 16;
                    }
                    "i16" => {
                        writer
                            .write_fmt(format_args!(
                                "{}\n",
                                c_bits[bitpos_in_line..bitpos_in_line + size_of::<i16>()]
                                    .load::<i16>()
                            ))
                            .unwrap();
                        bitpos_in_line += 16;
                    }
                    "u32" => {
                        writer
                            .write_fmt(format_args!(
                                "{}\n",
                                c_bits[bitpos_in_line..bitpos_in_line + size_of::<u32>()]
                                    .load::<u32>()
                            ))
                            .unwrap();
                        bitpos_in_line += 32;
                    }
                    "i32" => {
                        writer
                            .write_fmt(format_args!(
                                "{}\n",
                                c_bits[bitpos_in_line..bitpos_in_line + size_of::<i32>()]
                                    .load::<i32>()
                            ))
                            .unwrap();
                        bitpos_in_line += 32;
                    }
                    _ => eprintln!("unknown type"),
                }
                writer.flush().unwrap();
            }

		thread::sleep(time::Duration::new(1, 0));
        std::process::Command::new("clear").status().unwrap();
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
