use crossbeam::channel::bounded;
use crossterm::style::{self, Color, Stylize};
use mview::{args::Args, chunksize_by_config, read, size_in_bits, write};
use std::fs::File;
use std::io::{BufRead, BufReader, Result};
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

    let (write_tx, write_rx) = bounded(1024);

    // read config
    // let conf_file = File::open(config)?;
    // let conf_reader = BufReader::new(File::open(config)?);
    // let config_lines: Vec<String> = BufReader::new(File::open(config)?).lines().collect::<Result<_>>().unwrap();
    let config_lines = BufReader::new(File::open(config)?)
        .lines()
        .collect::<Result<Vec<String>>>()
        .unwrap()
        .into_iter()
        .filter(|l| !l.starts_with('#'))
        .collect();
    let chunksize_from_config = chunksize_by_config(&config_lines); // bits!
    if chunksize < 1 {
        // bytes!
        // if the chunksize from arguments is invalid, get the config chunks
        chunksize = chunksize_from_config / 8;
    }
    if chunksize_from_config % size_in_bits::<u8>() > 0 {
        eprintln!("{}: Size of config is {} bytes and {} bits. The chunksize is {}.
this means that some fields in the config will not be considered in the output as chunksize does not match sum of the fields sizes in config.", style::style("WARNING").with(Color::Yellow).bold(), chunksize_from_config / 8, chunksize_from_config % 8, chunksize)
    }

    let read_handle = thread::spawn(move || read::read_loop(&infile, write_tx));
    let write_handle = thread::spawn(move || {
        write::write_loop(
            &outfile,
            rawhex,
            rawbin,
            chunksize,
            offset,
            bitoffset,
            write_rx,
            &config_lines,
        )
    });
    let read_io_result = read_handle.join().unwrap();
    let write_io_result = write_handle.join().unwrap();
    read_io_result?;
    write_io_result?;
    Ok(())
}
