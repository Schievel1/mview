use core::time;
use mview::write::write_line;
use mview::{args::Args, print_raws, size_in_bits, MAX_READ_SIZE};
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Result, Write};
use std::thread;

fn main() -> Result<()> {
    // get args
    let args = Args::parse();
    let Args {
        infile,
        outfile,
        config,
        mut chunksize,
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
    let config_lines: Vec<String> = conf_reader.lines().collect::<Result<_>>().unwrap();
    let chunksize_from_config = chunksize_by_config(&config_lines); // bits!
    if chunksize < 1 { // bytes!
        // if the chunksize from arguments is invalid, get the config chunks
        chunksize =  chunksize_from_config / 8;
    }
    if chunksize_from_config % size_in_bits::<u8>() > 0 {
        eprintln!("WARNING: Size of config is {} bytes and {} bits. The chunksize is {}.
this means that some bits will not be considered in the config as chunksize does not match the config size", chunksize_from_config / 8, chunksize_from_config % 8, chunksize)
    }
    thread::sleep(time::Duration::new(1, 0));
    let mut buffer = [0; MAX_READ_SIZE];
    loop {
        // read input
        let num_read = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(x) => x,
            Err(_) => break,
        };
        // break what is read into chunks and apply config lines as masked to it
        let _write_result: Result<()> =
            (&buffer[..num_read])
                .chunks(chunksize)
                .try_for_each(|chunk| {
                    thread::sleep(time::Duration::new(1, 0)); // this is only here for debugging
                    std::process::Command::new("clear").status().unwrap();
                    print_raws(chunk, rawhex, rawbin, &mut writer);
                    let mut bitpos_in_chunk = 0 + bitoffset + offset * size_in_bits::<u8>();
                    for conf_line in config_lines.iter() {
                        if conf_line.starts_with('#') {
                            // # is the symbol to comment out a config line
                            continue;
                        }
                        write_line(conf_line, chunk, &mut bitpos_in_chunk, &mut writer)?;
                    }
                    Ok(())
                });
        buffer.fill(0);
    }
    Ok(())
}

