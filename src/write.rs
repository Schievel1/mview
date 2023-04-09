use crate::{
    args::Args, chunksize_by_config, count_lines, format_number, parse_config_line,
    print_additional, print_bitpos, read_config, size_in_bits, Format, BIN_LINE_SIZE,
    HEX_LINE_SIZE,
};
use anyhow::{Context, Result};
use bitvec::{
    macros::internal::funty::{Fundamental, Integral},
    prelude::*,
};
use core::time;
use crossbeam::channel::Receiver;
use crossterm::style::{self, Color, Stylize};
use crossterm::{
    cursor, execute,
    terminal::{Clear, ClearType},
};
use std::{
    fs::File,
    io::{self, BufWriter, Write},
    thread,
};

#[derive(Default)]
pub struct Stats {
    pub message_count: u32,
    pub message_len: u32,
    pub chunk_count: u32,
    pub chunk_start: u32,
    pub hex_lines: usize,
    pub bin_lines: usize,
}
pub fn write_loop(args: &Args, write_rx: Receiver<Vec<u8>>) -> Result<()> {
    // break what is read into chunks and apply config lines as masked to it
    let is_stdout = args.outfile.is_empty();
    let mut writer: Box<dyn Write> = if !is_stdout {
        Box::new(BufWriter::new(File::create(&args.outfile)?))
    } else {
        Box::new(BufWriter::new(io::stdout()))
    };
    let mut first_run = true;
    let mut stats: Stats = Default::default();
    let config_lines = read_config(&args.config)?;
    let chunksize_from_config = chunksize_by_config(&config_lines)?; // bits!
    let mut chunksize = args.chunksize;
    if chunksize < 1 {
        // bytes!
        // if the chunksize from arguments is invalid, get the config chunks
        chunksize = chunksize_from_config / 8;
    }
    if chunksize_from_config % size_in_bits::<u8>() > 0 {
        eprintln!("{}: Size of config is {} bytes and {} bits. The chunksize is {} bytes.
this means that some fields in the config will not be considered in the output because chunksize does not match sum of the fields sizes in config.", style::style("WARNING").with(Color::Yellow).bold(), chunksize_from_config / 8, chunksize_from_config % 8, chunksize)
    }
    loop {
        let buffer = write_rx
            .recv()
            .context("Error recieving data from read thread.")?;
        if buffer.is_empty() {
            break;
        }
        stats.message_count += 1;
        stats.message_len = buffer.len().as_u32();
        let chunkiter = buffer
            .chunks(chunksize)
            .take(1)
            .last()
            .context("Could not get size of chunk.")?;
        stats.hex_lines = chunkiter.chunks(HEX_LINE_SIZE).count();
        stats.bin_lines = chunkiter.chunks(BIN_LINE_SIZE).count();
        for chunk in buffer.chunks(chunksize) {
            stats.chunk_start = stats.chunk_count * (chunksize as u32);
            stats.chunk_count += 1;
            if is_stdout && !first_run && args.cursor_jump {
                move_cursor(args, config_lines.len(), &stats)?;
            }
            print_additional(args, &stats, &mut writer, chunk, chunksize)?;
            let mut bitpos_in_chunk = args.bitoffset + args.offset * size_in_bits::<u8>();
            // strategy: for every config line we call write_line().
            // write_line() will parse the config line, then get the size of the data type it
            // found it that line from the chunk, print it out and advance bitpos_in_chunk accordingly
            for conf_line in config_lines.iter() {
                if args.print_bitpos {
                    print_bitpos(&mut writer, bitpos_in_chunk)?;
                }
                write_line(
                    conf_line,
                    chunk,
                    &mut bitpos_in_chunk,
                    &mut writer,
                    args.little_endian,
                )?;
            }
            // print an empty line at the end of every chunk
            writer
                .write_all(b"\n")
                .context("Could now write to writer")?;
            thread::sleep(time::Duration::from_millis(args.pause));
            first_run = false;
        }
    }
    Ok(())
}

pub fn move_cursor(args: &Args, n_conf_lines: usize, stats: &Stats) -> Result<()> {
    let mut stdout = io::stdout();
    execute!(
        stdout,
        cursor::MoveUp(count_lines(args, &stats, n_conf_lines,)),
        cursor::MoveToColumn(0),
        // the following is necessary because writing in the terminal with a newline?
        cursor::MoveDown(1),
        Clear(ClearType::FromCursorDown),
        cursor::MoveUp(1),
    )?;
    Ok(())
}

pub fn write_integer_data<T>(
    bitpos_in_chunk: &usize,
    c_bits: &BitSlice<u8, Msb0>,
    writer: &mut Box<dyn Write>,
    format: Format,
    little_endian: bool,
) -> Result<usize>
where
    T: Integral,
{
    // returns the size of the written type in bits
    if *bitpos_in_chunk + size_in_bits::<T>() <= c_bits.len() {
        let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<T>()];
        myslice
            .copy_from_bitslice(&c_bits[*bitpos_in_chunk..*bitpos_in_chunk + size_in_bits::<T>()]);
        if little_endian {
            writer
                .write_fmt(format_args!(
                    "{}\n",
                    format_number(&myslice[0..size_in_bits::<T>()].load_le::<T>(), format)
                ))
                .context("Could now write to writer")?;
        } else {
            writer
                .write_fmt(format_args!(
                    "{}\n",
                    format_number(&myslice[0..size_in_bits::<T>()].load_be::<T>(), format)
                ))
                .context("Could now write to writer")?;
        }
    } else {
        writer
            .write_all(b"values size is bigger than what is left of that data chunk\n")
            .context("Could now write to writer")?;
    }
    Ok(size_in_bits::<T>())
}

fn write_gap(
    bitpos_in_chunk: &usize,
    c_bits: &BitSlice<u8, Msb0>,
    writer: &mut Box<dyn Write>,
    len: usize,
    typelen: usize,
) -> Result<usize> {
    if *bitpos_in_chunk + len * typelen <= c_bits.len() {
        writer
            .write_fmt(format_args!("(gap of {} bit)\n", len * typelen))
            .context("Could now write to writer")?;
    } else {
        writer
            .write_all(b"values size is bigger than what is left of that data chunk\n")
            .context("Could now write to writer")?;
    }
    Ok(typelen * len)
}

pub fn write_line(
    conf_line: &str,
    chunk: &[u8],
    bitpos_in_chunk: &mut usize,
    writer: &mut Box<dyn Write>,
    little_endian: bool,
) -> Result<()> {
    let c_bits = chunk.view_bits::<Msb0>();
    let (fieldname, val_type, form, len) = parse_config_line(conf_line)?;
    writer
        .write_fmt(format_args!("{}", fieldname))
        .context("Could now write to writer")?;
    writer
        .write_all(b": ")
        .context("Could now write to writer")?;
    let val_type = val_type.to_lowercase(); // don't care about case fo the letters
    match val_type.as_str() {
        "bool1" => {
            writer
                .write_fmt(format_args!("{}\n", c_bits[*bitpos_in_chunk]))
                .context("Could now write to writer")?;
            *bitpos_in_chunk += 1;
        }
        "bool8" => {
            if *bitpos_in_chunk + size_in_bits::<u8>() <= c_bits.len() {
                let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<u8>()];
                myslice.copy_from_bitslice(
                    &c_bits[*bitpos_in_chunk..*bitpos_in_chunk + size_in_bits::<u8>()],
                );
                writer
                    .write_fmt(format_args!("{}\n", myslice[0..8].load::<u8>() > 0))
                    .context("Could now write to writer")?;
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .context("Could now write to writer")?;
            }
            *bitpos_in_chunk += size_in_bits::<u8>();
        }
        "u8" => {
            *bitpos_in_chunk +=
                write_integer_data::<u8>(bitpos_in_chunk, c_bits, writer, form, little_endian)?;
        }
        "u16" => {
            *bitpos_in_chunk +=
                write_integer_data::<u16>(bitpos_in_chunk, c_bits, writer, form, little_endian)?;
        }
        "u32" => {
            *bitpos_in_chunk +=
                write_integer_data::<u32>(bitpos_in_chunk, c_bits, writer, form, little_endian)?;
        }
        "u64" => {
            *bitpos_in_chunk +=
                write_integer_data::<u64>(bitpos_in_chunk, c_bits, writer, form, little_endian)?;
        }
        "u128" => {
            *bitpos_in_chunk +=
                write_integer_data::<u128>(bitpos_in_chunk, c_bits, writer, form, little_endian)?;
        }
        "i8" => {
            *bitpos_in_chunk +=
                write_integer_data::<i8>(bitpos_in_chunk, c_bits, writer, form, little_endian)?;
        }
        "i16" => {
            *bitpos_in_chunk +=
                write_integer_data::<i16>(bitpos_in_chunk, c_bits, writer, form, little_endian)?;
        }
        "i32" => {
            *bitpos_in_chunk +=
                write_integer_data::<i32>(bitpos_in_chunk, c_bits, writer, form, little_endian)?;
        }
        "i64" => {
            *bitpos_in_chunk +=
                write_integer_data::<i64>(bitpos_in_chunk, c_bits, writer, form, little_endian)?;
        }
        "i128" => {
            *bitpos_in_chunk +=
                write_integer_data::<i128>(bitpos_in_chunk, c_bits, writer, form, little_endian)?;
        }
        "f32" => {
            if *bitpos_in_chunk + size_in_bits::<f32>() <= c_bits.len() {
                let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<f32>()];
                myslice.copy_from_bitslice(
                    &c_bits[*bitpos_in_chunk..*bitpos_in_chunk + size_in_bits::<f32>()],
                );
                writer
                    .write_fmt(format_args!("{}\n", &myslice[0..32].load::<u32>().as_f32()))
                    .context("Could now write to writer")?;
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .context("Could now write to writer")?;
            }
            *bitpos_in_chunk += size_in_bits::<f32>();
        }
        "f64" => {
            if *bitpos_in_chunk + size_in_bits::<f64>() <= c_bits.len() {
                let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<f64>()];
                myslice.copy_from_bitslice(
                    &c_bits[*bitpos_in_chunk..*bitpos_in_chunk + size_in_bits::<f64>()],
                );
                writer
                    .write_fmt(format_args!("{}\n", &myslice[0..64].load::<u64>().as_f64()))
                    .context("Could now write to writer")?;
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .context("Could now write to writer")?;
            }
            *bitpos_in_chunk += size_in_bits::<f64>();
        }
        "string" => {
            if *bitpos_in_chunk + len * size_in_bits::<u8>() <= c_bits.len() {
                for _i in 0..len {
                    writer
                        .write_fmt(format_args!(
                            "{}",
                            c_bits[*bitpos_in_chunk..*bitpos_in_chunk + size_in_bits::<u8>()]
                                .load::<u8>() as char
                        ))
                        .context("Could now write to writer")?;
                    *bitpos_in_chunk += size_in_bits::<u8>();
                }
                writer
                    .write_fmt(format_args!("\n"))
                    .context("Could now write to writer")?;
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .context("Could now write to writer")?;
            }
        }
        "iarb" => {
            if *bitpos_in_chunk + len <= c_bits.len() {
                let target_int;
                let negative = c_bits[*bitpos_in_chunk + len - 1];
                let mut target_slice: [u8; 16] = [0; 16];
                let int_bits = target_slice.view_bits_mut::<Lsb0>();
                for i in 0..len {
                    int_bits.set(i, c_bits[*bitpos_in_chunk + i]); // copy the payload over
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
                writer
                    .write_fmt(format_args!("{}\n", target_int))
                    .context("Could now write to writer")?;
                *bitpos_in_chunk += len;
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .context("Could now write to writer")?;
            }
        }
        "uarb" => {
            if *bitpos_in_chunk + len <= c_bits.len() {
                let mut target_slice: [u8; 16] = [0; 16];
                let int_bits = target_slice.view_bits_mut::<Lsb0>();
                for i in 0..len {
                    int_bits.set(i, c_bits[*bitpos_in_chunk + i]); // copy the payload over
                }
                let target_int = int_bits.load::<i128>();
                writer
                    .write_fmt(format_args!("{}\n", target_int))
                    .context("Could now write to writer")?;

                *bitpos_in_chunk += len;
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .context("Could now write to writer")?;
            }
        }
        "bytegap" => {
            *bitpos_in_chunk += write_gap(bitpos_in_chunk, c_bits, writer, len, 8)?;
        }
        "bitgap" => {
            *bitpos_in_chunk += write_gap(bitpos_in_chunk, c_bits, writer, len, 1)?;
        }
        _ => eprintln!("unknown type"),
    }
    writer.flush().context("Could now write to writer")?;

    Ok(())
}
