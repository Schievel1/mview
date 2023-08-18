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
        // get some stats
        stats.message_count += 1;
        stats.chunk_count = 0;
        stats.message_len = buffer.len().as_u32();
        let chunkiter = buffer
            .chunks(chunksize)
            .take(1)
            .last()
            .context("Could not get size of chunk.")?;
        stats.hex_lines = chunkiter.chunks(HEX_LINE_SIZE).count();
        stats.bin_lines = chunkiter.chunks(BIN_LINE_SIZE).count();

        // when we read from stdin, return is pressed by the user after typing in a message.
        // In this case we need to get rid of the extra line
        if is_stdout && !first_run && args.cursor_jump && args.infile.is_empty() {
            execute!(io::stdout(), cursor::MoveUp(1))?;
        }

        for chunk in buffer.chunks(chunksize) {
            // get some stats
            stats.chunk_start = stats.chunk_count * (chunksize as u32);
            stats.chunk_count += 1;

            // in case we write to stdout, move the cursor back to the start
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
                    args,
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
        cursor::MoveUp(count_lines(args, stats, n_conf_lines)),
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
    writer: &mut dyn Write,
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
                    format_number(myslice[0..size_in_bits::<T>()].load_le::<T>(), format)
                ))
                .context("Could now write to writer")?;
        } else {
            writer
                .write_fmt(format_args!(
                    "{}\n",
                    format_number(myslice[0..size_in_bits::<T>()].load_be::<T>(), format)
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
    writer: &mut dyn Write,
    len: usize,     // number of typelen to jump ahead
    typelen: usize, // length of a gap part, 1 bit or 8 bit
) -> Result<usize> {
    // how much the bitpos was advanced (the legth of the gap in bits)
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
    args: &Args,
    conf_line: &str,
    chunk: &[u8],
    bitpos_in_chunk: &mut usize,
    writer: &mut dyn Write,
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
                    .write_fmt(format_args!(
                        "{}\n",
                        f32::from_bits(myslice[0..32].load_be::<u32>())
                    ))
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
                    .write_fmt(format_args!(
                        "{}\n",
                        f64::from_bits(myslice[0..64].load_be::<u64>())
                    ))
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
                    let c = [c_bits[*bitpos_in_chunk..*bitpos_in_chunk + size_in_bits::<u8>()].load::<u8>()];
                    let mut s = String::from_utf8_lossy(&c);
                    if args.filter_newlines {
                        s = s.chars().filter(|c| *c != '\n' ).collect();
                    }

                    writer
                        .write_fmt(format_args!(
                            "{}", s
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Display;
    // this is a wrapper funktion is case we change anything in the future
    fn format_write_line_output<T: Display>(expected: T) -> String {
        format!("Test: {}\n", expected)
    }
    fn make_dummy_args() -> Args {
        Args {
            infile: "nil".to_string(),
            outfile: "nil".to_string(),
            config: "nil".to_string(),
            pcap: false,
            chunksize: 0,
            offset: 0,
            bitoffset: 0,
            rawhex: false,
            rawbin: false,
            rawascii: false,
            pause: 0,
            little_endian: false,
            timestamp: false,
            read_head: 0,
            print_statistics: false,
            print_bitpos: false,
            cursor_jump: false,
            filter_newlines: false,
        }
    }

    #[test]
    fn test_write_gap_5bit() {
        let bitpos_in_chunk = 1;
        let chunk: [u8; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let c_bits = chunk.view_bits::<Msb0>();

        let mut output = Vec::new();
        assert_eq!(
            write_gap(&bitpos_in_chunk, c_bits, &mut output, 5, 1).unwrap(),
            5
        );
        assert_eq!(output, b"(gap of 5 bit)\n");
    }

    #[test]
    fn test_write_line_bool1_true() {
        let args = make_dummy_args();
        let conf_line = "Test:bool1";
        let chunk: [u8; 10] = [0b10101010, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut bitpos_in_chunk = 0;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, false).unwrap();
        assert_eq!(output, format_write_line_output("true").as_bytes());
    }
    #[test]
    fn test_write_line_bool1_false() {
        let args = make_dummy_args();
        let conf_line = "Test:bool1";
        let chunk: [u8; 10] = [0b10101010, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut bitpos_in_chunk = 1;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, false).unwrap();
        assert_eq!(output, format_write_line_output("false").as_bytes());
    }
    #[test]
    fn test_write_line_bool8_true() {
        let args = make_dummy_args();
        let conf_line = "Test:bool8";
        let chunk: [u8; 10] = [0b1111_0000, 0b0000_1111, 0b0000_1111, 3, 4, 5, 6, 7, 8, 9];
        let mut bitpos_in_chunk = 10;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, false).unwrap();
        assert_eq!(output, format_write_line_output("true").as_bytes());
    }
    #[test]
    fn test_write_line_bool8_false() {
        let args = make_dummy_args();
        let conf_line = "Test:bool8";
        let chunk: [u8; 10] = [0b1111_0000, 0b0000_1111, 0b0000_1111, 3, 4, 5, 6, 7, 8, 9];
        let mut bitpos_in_chunk = 4;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, false).unwrap();
        assert_eq!(output, format_write_line_output("false").as_bytes());
    }
    #[test]
    fn test_write_line_u8() {
        let args = make_dummy_args();
        let conf_line = "Test:u8";
        let chunk: [u8; 10] = [0b1111_0000, 0b0000_1111, 0b0000_1111, 3, 4, 5, 6, 7, 8, 9];
        let mut bitpos_in_chunk = 7;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, false).unwrap();
        // 0b00000111 = 7 in dec
        assert_eq!(output, format_write_line_output("7").as_bytes());
    }
    #[test]
    fn test_write_line_u8_hex() {
        let args = make_dummy_args();
        let conf_line = "Test:u8:h";
        let chunk: [u8; 10] = [0b1111_0000, 0b0000_1111, 0b0000_1111, 3, 4, 5, 6, 7, 8, 9];
        let mut bitpos_in_chunk = 7;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, false).unwrap();
        // 0b00000111 = 7 in dec
        assert_eq!(output, format_write_line_output("0x07").as_bytes());
    }
    #[test]
    fn test_write_line_u8_bin() {
        let args = make_dummy_args();
        let conf_line = "Test:u8:b";
        let chunk: [u8; 10] = [0b1111_0000, 0b0000_1111, 0b0000_1111, 3, 4, 5, 6, 7, 8, 9];
        let mut bitpos_in_chunk = 7;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, false).unwrap();
        // 0b00000111 = 7 in dec
        assert_eq!(output, format_write_line_output("00000111").as_bytes());
    }
    #[test]
    fn test_write_line_u16_be() {
        let args = make_dummy_args();
        let conf_line = "Test:u16";
        let chunk: [u8; 10] = [0b1111_0000, 0b0000_1111, 0b0000_1111, 3, 4, 5, 6, 7, 8, 9];
        let mut bitpos_in_chunk = 4;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, false).unwrap();
        // 0b0000_0000_1111_0000 = 240 in dec
        assert_eq!(output, format_write_line_output("240").as_bytes());
    }
    #[test]
    fn test_write_line_u16_le() {
        let args = make_dummy_args();
        let conf_line = "Test:u16";
        let chunk: [u8; 10] = [0b1111_0000, 0b0000_1111, 0b0000_1111, 3, 4, 5, 6, 7, 8, 9];
        let mut bitpos_in_chunk = 4;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, true).unwrap();
        // 0b1111_0000_0000_0000 = 61440 in dec
        assert_eq!(output, format_write_line_output("61440").as_bytes());
    }
    #[test]
    fn test_write_line_u32_be() {
        let args = make_dummy_args();
        let conf_line = "Test:u32";
        let chunk: [u8; 10] = [0xFF, 0x0F, 0xFF, 0x0F, 0xFF, 5, 6, 7, 8, 9];
        let mut bitpos_in_chunk = 4;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, false).unwrap();
        // 0xF0FF_F0FF = 4043305215 in dec
        assert_eq!(output, format_write_line_output("4043305215").as_bytes());
    }
    #[test]
    fn test_write_line_u32_le() {
        let args = make_dummy_args();
        let conf_line = "Test:u32";
        //		let chunk: [u8; 10] = [0xFF,0xAF,0xBF,0xCF,0xDF,5,6,7,8,9];
        let chunk: [u8; 10] = [
            0b0000_1111,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b1111_0000,
            5,
            6,
            7,
            8,
            9,
        ];
        let mut bitpos_in_chunk = 4;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, true).unwrap();
        // 0b0000_1111_1111_1111_1111_0000_1111_0000 = 268431600 in dec
        assert_eq!(output, format_write_line_output("268431600").as_bytes());
    }
    #[test]
    fn test_write_line_u64_be() {
        let args = make_dummy_args();
        let conf_line = "Test:u64";
        let chunk: [u8; 10] = [
            0b0000_1111,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b1111_0000,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b0000_1111,
            9,
        ];
        let mut bitpos_in_chunk = 4;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, false).unwrap();
        // 0b1111_0000 1111_0000 1111_1111 0000_1111 0000_0000 1111_0000 1111_1111 0000_0000 = 17361657003418648320 in dec
        assert_eq!(
            output,
            format_write_line_output("17361657003418648320").as_bytes()
        );
    }
    #[test]
    fn test_write_line_u64_le() {
        let args = make_dummy_args();
        let conf_line = "Test:u64";
        let chunk: [u8; 10] = [
            0b0000_1111,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b1111_0000,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b0000_1111,
            9,
        ];
        let mut bitpos_in_chunk = 4;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, true).unwrap();
        // 0b0000_0000 1111_1111 1111_0000 0000_0000 0000_1111 1111_1111 1111_0000 1111_0000 = 72040002120315120 in dec
        assert_eq!(
            output,
            format_write_line_output("72040002120315120").as_bytes()
        );
    }
    #[test]
    fn test_write_line_u128_be() {
        let args = make_dummy_args();
        let conf_line = "Test:u128";
        let chunk: [u8; 20] = [
            0b0000_1111,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b1111_0000,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b0000_1111,
            0b1111_1111,
            0b0000_1111,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b1111_0000,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b0000_1111,
            0b1111_1111,
        ];
        let mut bitpos_in_chunk = 4;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, false).unwrap();
        // 11110000111100001111111100001111000000001111000011111111000000001111111111110000111100001111000011111111000011110000000011110000
        // = 320266043437590883436287332191935856880 in dec
        assert_eq!(
            output,
            format_write_line_output("320266043437590883436287332191935856880").as_bytes()
        );
    }
    #[test]
    fn test_write_line_u128_le() {
        let args = make_dummy_args();
        let conf_line = "Test:u128";
        let chunk: [u8; 20] = [
            0b0000_1111,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b1111_0000,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b0000_1111,
            0b1111_1111,
            0b0000_1111,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b1111_0000,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b0000_1111,
            0b1111_1111,
        ];
        let mut bitpos_in_chunk = 4;
        // 11110000000000000000111111111111111100001111000011110000111111110000000011111111111100000000000000001111111111111111000011110000
        // = 319015043502272988035154135038543524080
        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, true).unwrap();
        assert_eq!(
            output,
            format_write_line_output("319015043502272988035154135038543524080").as_bytes()
        );
    }
    #[test]
    fn test_write_line_i8() {
        let args = make_dummy_args();
        let conf_line = "Test:i8";
        let chunk: [u8; 10] = [0b1111_0001, 0b0000_1111, 0b0000_1111, 3, 4, 5, 6, 7, 8, 9];
        let mut bitpos_in_chunk = 7;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, false).unwrap();
        // 0b10000111 = -241 in dec
        assert_eq!(output, format_write_line_output("-121").as_bytes());
    }
    #[test]
    fn test_write_line_i16_be() {
        let args = make_dummy_args();
        let conf_line = "Test:i16";
        let chunk: [u8; 10] = [0b1111_1000, 0b0000_1111, 0b0000_1111, 3, 4, 5, 6, 7, 8, 9];
        let mut bitpos_in_chunk = 4;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, false).unwrap();
        // 0b1000_0000_1111_0000 = -32528 in dec
        assert_eq!(output, format_write_line_output("-32528").as_bytes());
    }
    #[test]
    fn test_write_line_i16_le() {
        let args = make_dummy_args();
        let conf_line = "Test:i16";
        let chunk: [u8; 10] = [0b1111_0000, 0b0000_1111, 0b0000_1111, 3, 4, 5, 6, 7, 8, 9];
        let mut bitpos_in_chunk = 4;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, true).unwrap();
        // 0b1111_0000_0000_0000 = -4096 in dec
        assert_eq!(output, format_write_line_output("-4096").as_bytes());
    }
    #[test]
    fn test_write_line_i32_be() {
        let args = make_dummy_args();
        let conf_line = "Test:i32";
        let chunk: [u8; 10] = [
            0b0000_1111,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b1111_0000,
            5,
            6,
            7,
            8,
            9,
        ];
        let mut bitpos_in_chunk = 4;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, false).unwrap();
        // 11110000111100001111111100001111 = -252641521 in dec
        assert_eq!(output, format_write_line_output("-252641521").as_bytes());
    }
    #[test]
    fn test_write_line_i32_le() {
        let args = make_dummy_args();
        let conf_line = "Test:i32";
        let chunk: [u8; 10] = [
            0b0000_1111,
            0b0000_1111,
            0b0000_1111,
            0b1111_1000,
            0b1111_0000,
            5,
            6,
            7,
            8,
            9,
        ];
        let mut bitpos_in_chunk = 4;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, true).unwrap();
        // 0b1000_1111_1111_1111_1111_0000_1111_0000 = -1879052048 in dec
        assert_eq!(output, b"Test: -1879052048\n");
    }
    #[test]
    fn test_write_line_i64_be() {
        let args = make_dummy_args();
        let conf_line = "Test:i64";
        let chunk: [u8; 10] = [
            0b0000_1111,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b1111_0000,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b0000_1111,
            9,
        ];
        let mut bitpos_in_chunk = 4;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, false).unwrap();
        // 0b1111_0000 1111_0000 1111_1111 0000_1111 0000_0000 1111_0000 1111_1111 0000_0000 = -1085087070290903296 in dec
        assert_eq!(
            output,
            format_write_line_output("-1085087070290903296").as_bytes()
        );
    }
    #[test]
    fn test_write_line_i64_le() {
        let args = make_dummy_args();
        let conf_line = "Test:i64";
        let chunk: [u8; 10] = [
            0b0000_1111,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b1111_0000,
            0b0000_1111,
            0b0000_1111,
            0b1111_1000,
            0b0000_1111,
            9,
        ];
        let mut bitpos_in_chunk = 4;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, true).unwrap();
        // 0b1000_0000 1111_1111 1111_0000 0000_0000 0000_1111 1111_1111 1111_0000 1111_0000 = -9151332034734460688 in dec
        assert_eq!(
            output,
            format_write_line_output("-9151332034734460688").as_bytes()
        );
    }
    #[test]
    fn test_write_line_i128_be() {
        let args = make_dummy_args();
        let conf_line = "Test:i128";
        let chunk: [u8; 20] = [
            0b0000_1111,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b1111_0000,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b0000_1111,
            0b1111_1111,
            0b0000_1111,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b1111_0000,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b0000_1111,
            0b1111_1111,
        ];
        let mut bitpos_in_chunk = 4;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, false).unwrap();
        // 11110000111100001111111100001111000000001111000011111111000000001111111111110000111100001111000011111111000011110000000011110000
        // = -20016323483347580027087275239832354576 in dec
        assert_eq!(
            output,
            format_write_line_output("-20016323483347580027087275239832354576").as_bytes()
        );
    }
    #[test]
    fn test_write_line_i128_le() {
        let args = make_dummy_args();
        let conf_line = "Test:i128";
        let chunk: [u8; 20] = [
            0b0000_1111,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b1111_0000,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b0000_1111,
            0b1111_1111,
            0b0000_1111,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b1111_0000,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b0000_1111,
            0b1111_1111,
        ];
        let mut bitpos_in_chunk = 4;
        // 11110000000000000000111111111111111100001111000011110000111111110000000011111111111100000000000000001111111111111111000011110000
        // = 319015043502272988035154135038543524080
        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, true).unwrap();
        assert_eq!(
            output,
            format_write_line_output("-21267323418665475428220472393224687376").as_bytes()
        );
    }
    #[test]
    fn test_write_line_f32() {
        let args = make_dummy_args();
        let conf_line = "Test:f32";
        let chunk: [u8; 10] = [
            0b0000_1100,
            0b0000_0100,
            0b1100_1100,
            0b1100_1100,
            0b1101_0000,
            5,
            6,
            7,
            8,
            9,
        ];
        let mut bitpos_in_chunk = 4;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, false).unwrap();
        // 11000000010011001100110011001101 = -3.2
        assert_eq!(output, format_write_line_output("-3.2").as_bytes());
    }
    #[test]
    fn test_write_line_f64() {
        let args = make_dummy_args();
        let conf_line = "Test:f64";
        let chunk: [u8; 10] = [
            0b0000_1100,
            0b0000_0000,
            0b1001_1001,
            0b1001_1001,
            0b1001_1001,
            0b1001_1001,
            0b1001_1001,
            0b1001_1001,
            0b1001_1001,
            9,
        ];
        let mut bitpos_in_chunk = 4;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, false).unwrap();
        // 1100000000001001100110011001100110011001100110011001100110011010 = -3.1999999999999997
        assert_eq!(
            output,
            format_write_line_output("-3.1999999999999997").as_bytes()
        );
    }
    #[test]
    fn test_write_line_string() {
        let args = make_dummy_args();
        let conf_line = "Test:string:3";
        let chunk: [u8; 10] = [
            0b0000_1100,
            b'a',
            b'b',
            b'c',
            0b1001_1001,
            0b1001_1001,
            0b1001_1001,
            0b1001_1001,
            0b1001_1001,
            9,
        ];
        let mut bitpos_in_chunk = 8;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, false).unwrap();
        assert_eq!(output, format_write_line_output("abc").as_bytes());
    }
    #[test]
    fn test_write_line_uarb() {
        let args = make_dummy_args();
        let conf_line = "Test:uarb:9";
        let chunk: [u8; 10] = [
            0b0000_1100,
            0b1001_1001,
            0b1111_0000,
            0b1111_0000,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b0000_1111,
            0b1001_1001,
            0b1001_1001,
        ];
        let mut bitpos_in_chunk = 4;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, false).unwrap();
        // 110010011 = 403
        assert_eq!(output, format_write_line_output("403").as_bytes());
    }
    #[test]
    fn test_write_line_iarb() {
        let args = make_dummy_args();
        let conf_line = "Test:iarb:9";
        let chunk: [u8; 10] = [
            0b0000_1100,
            0b1001_1001,
            0b1111_0000,
            0b1111_0000,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b0000_1111,
            0b1001_1001,
            0b1001_1001,
        ];
        let mut bitpos_in_chunk = 4;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, false).unwrap();
        // 110010011 = -109
        assert_eq!(output, format_write_line_output("-109").as_bytes());
    }
    #[test]
    fn test_write_line_bytegap() {
        let args = make_dummy_args();
        let conf_line = "Test:bytegap:1";
        let chunk: [u8; 10] = [
            0b0000_1100,
            0b1001_1001,
            0b1111_0000,
            0b1111_0000,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b0000_1111,
            0b1001_1001,
            0b1001_1001,
        ];
        let mut bitpos_in_chunk = 4;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, false).unwrap();
        assert_eq!(
            output,
            format_write_line_output("(gap of 8 bit)").as_bytes()
        );
    }
    #[test]
    fn test_write_line_bitgap() {
        let args = make_dummy_args();
        let conf_line = "Test:bitgap:1";
        let chunk: [u8; 10] = [
            0b0000_1100,
            0b1001_1001,
            0b1111_0000,
            0b1111_0000,
            0b0000_1111,
            0b0000_1111,
            0b1111_0000,
            0b0000_1111,
            0b1001_1001,
            0b1001_1001,
        ];
        let mut bitpos_in_chunk = 4;

        let mut output = Vec::new();
        write_line(&args, conf_line, &chunk, &mut bitpos_in_chunk, &mut output, false).unwrap();
        assert_eq!(
            output,
            format_write_line_output("(gap of 1 bit)").as_bytes()
        );
    }
}
