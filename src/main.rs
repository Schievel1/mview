#![feature(array_windows)]

use clap::{App, Arg};
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Result, Write};

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

		let s_buf = &buffer[..num_read];
		s_buf.chunks(chunksize).for_each(|c| {
        	writer.write_all(c).unwrap();
			writer.flush().unwrap();
			println!();
		});
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
