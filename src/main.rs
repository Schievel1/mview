use core::time;
use crossterm::{
    cursor, execute,
    style::{self, Color, Stylize},
    terminal::{Clear, ClearType},
};
use mview::write::write_line;
use mview::{args::Args, chunksize_by_config, print_raws, size_in_bits, MAX_READ_SIZE};
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
    let is_stdout = outfile.is_empty();
    let mut reader: Box<dyn Read> = if !infile.is_empty() {
        Box::new(BufReader::new(File::open(infile)?))
    } else {
        Box::new(BufReader::new(io::stdin()))
    };
    let mut writer: Box<dyn Write> = if !is_stdout {
        Box::new(BufWriter::new(File::create(outfile)?))
    } else {
        Box::new(BufWriter::new(io::stdout()))
    };

    // read config
    let conf_file = File::open(config)?;
    let conf_reader = BufReader::new(conf_file);
    let config_lines: Vec<String> = conf_reader.lines().collect::<Result<_>>().unwrap();
    let config_lines: Vec<String> = config_lines.into_iter().filter(|l| !l.starts_with('#')).collect();
    let chunksize_from_config = chunksize_by_config(&config_lines); // bits!
    if chunksize < 1 {
        // bytes!
        // if the chunksize from arguments is invalid, get the config chunks
        chunksize = chunksize_from_config / 8;
    }
    if chunksize_from_config % size_in_bits::<u8>() > 0 {
        eprintln!("{}: Size of config is {} bytes and {} bits. The chunksize is {}.
this means that some bits will not be considered in the config as chunksize does not match the config size", style::style("WARNING").with(Color::Yellow).bold(), chunksize_from_config / 8, chunksize_from_config % 8, chunksize)
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
                    let mut stdout = io::stdout();
                    if is_stdout {
                        execute!(
                            stdout,
                            cursor::Hide,
                            )?;
                    }
                    print_raws(chunk, rawhex, rawbin, &mut writer);
                    let mut bitpos_in_chunk = 0 + bitoffset + offset * size_in_bits::<u8>();
                    for conf_line in config_lines.iter() {
                        write_line(conf_line, chunk, &mut bitpos_in_chunk, &mut writer)?;
                    }
                    thread::sleep(time::Duration::new(1, 0));
                    if is_stdout {
                        let mut extra_lines = 0;
                        if rawbin || rawhex { extra_lines += 1 };
                        if rawhex { extra_lines += 1 };
                        if rawbin { extra_lines += 1 };
                        execute!(
                            stdout,
                            cursor::MoveUp(config_lines.len() as u16 + extra_lines),
                            cursor::MoveToColumn(1),
                            Clear(ClearType::CurrentLine),
                            Clear(ClearType::FromCursorDown),
                            cursor::Show,
                            )?;
                    }
                    Ok(())
                });
        buffer.fill(0);
    }
    Ok(())
}
