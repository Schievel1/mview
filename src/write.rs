use crate::print_raws;
use crate::size_in_bits;
use bitvec::macros::internal::funty::{Fundamental, Integral};
use bitvec::prelude::*;
use core::time;
use crossbeam::channel::Receiver;
use crossterm::{
    cursor, execute,
    terminal::{Clear, ClearType},
};
use std::fs::File;
use std::io::{self, BufWriter, Result, Write};
use std::thread;

pub fn write_loop(
    outfile: &str,
    rawhex: bool,
    rawbin: bool,
    chunksize: usize,
    offset: usize,
    bitoffset: usize,
    write_rx: Receiver<Vec<u8>>,
    config_lines: &[String],
    pause: u64,
) -> Result<()> {
    // break what is read into chunks and apply config lines as masked to it
    let is_stdout = outfile.is_empty();
    let mut writer: Box<dyn Write> = if !is_stdout {
        Box::new(BufWriter::new(File::create(outfile)?))
    } else {
        Box::new(BufWriter::new(io::stdout()))
    };
    let mut first_run = true;
    loop {
        let buffer = write_rx.recv().unwrap();
        if buffer.is_empty() {
            break;
        }
        let _: Result<()> = buffer.chunks(chunksize).try_for_each(|chunk| {
            let mut stdout = io::stdout();
            if is_stdout {
                execute!(stdout, cursor::Hide,)?;
            }
            if is_stdout && !first_run {
                let mut extra_lines = 0;
                if rawbin || rawhex {
                    extra_lines += 1
                };
                if rawhex {
                    extra_lines += 1
                };
                if rawbin {
                    extra_lines += 1
                };
                execute!(
                    stdout,
                    cursor::MoveUp(config_lines.len() as u16 + extra_lines),
                    cursor::MoveToColumn(0),
                    Clear(ClearType::CurrentLine),
                    Clear(ClearType::FromCursorDown),
                    cursor::Show,
                )?;
            }
            print_raws(chunk, rawhex, rawbin, &mut writer);
            let mut bitpos_in_chunk = bitoffset + offset * size_in_bits::<u8>();
            for conf_line in config_lines.iter() {
                write_line(conf_line, chunk, &mut bitpos_in_chunk, &mut writer)?;
            }
            thread::sleep(time::Duration::from_millis(pause));
            first_run = false;
            Ok(())
        });
    }
    Ok(())
}

pub fn write_integer_data<T>(
    bitpos_in_chunk: &usize,
    c_bits: &BitSlice<u8, Msb0>,
    writer: &mut Box<dyn Write>,
) -> usize
where
    T: Integral,
{
    // returns the size of the written type in bits
    if *bitpos_in_chunk + size_in_bits::<T>() <= c_bits.len() {
        let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<T>()];
        myslice
            .copy_from_bitslice(&c_bits[*bitpos_in_chunk..*bitpos_in_chunk + size_in_bits::<T>()]);
        writer
            .write_fmt(format_args!(
                "{}\n",
                &myslice[0..size_in_bits::<T>()].load::<T>()
            ))
            .unwrap();
    } else {
        writer
            .write_all(b"values size is bigger than what is left of that data chunk\n")
            .unwrap();
    }
    size_in_bits::<T>()
}

fn write_gap(
    bitpos_in_chunk: &usize,
    c_bits: &BitSlice<u8, Msb0>,
    writer: &mut Box<dyn Write>,
    len: usize,
    typelen: usize,
) -> usize {
    if *bitpos_in_chunk + len * typelen <= c_bits.len() {
        writer
            .write_fmt(format_args!("(gap of {} bit)\n", len * typelen))
            .unwrap();
    } else {
        writer
            .write_all(b"values size is bigger than what is left of that data chunk\n")
            .unwrap();
    }
    typelen * len
}

pub fn write_line(
    conf_line: &str,
    c: &[u8],
    bitpos_in_chunk: &mut usize,
    writer: &mut Box<dyn Write>,
) -> Result<()> {
    let c_bits = c.view_bits::<Msb0>();
    let (fieldname, rest) = conf_line.split_once(':').unwrap();
    let (val_type, len) = match rest.split_once(':') {
        Some(s) => (s.0, s.1.parse().unwrap_or_default()),
        None => (rest, 0),
    };
    writer.write_fmt(format_args!("{}", fieldname)).unwrap();
    writer.write_all(b": ").unwrap();
    let val_type = val_type.to_lowercase(); // don't care about type
    match val_type.as_str() {
        "bool1" => {
            writer
                .write_fmt(format_args!("{}\n", c_bits[*bitpos_in_chunk]))
                .unwrap();
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
                    .unwrap();
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .unwrap();
            }
            *bitpos_in_chunk += size_in_bits::<u8>();
        }
        "u8" => *bitpos_in_chunk += write_integer_data::<u8>(bitpos_in_chunk, c_bits, writer),
        "u16" => *bitpos_in_chunk += write_integer_data::<u16>(bitpos_in_chunk, c_bits, writer),
        "u32" => *bitpos_in_chunk += write_integer_data::<u32>(bitpos_in_chunk, c_bits, writer),
        "u64" => *bitpos_in_chunk += write_integer_data::<u64>(bitpos_in_chunk, c_bits, writer),
        "u128" => *bitpos_in_chunk += write_integer_data::<u128>(bitpos_in_chunk, c_bits, writer),
        "i8" => *bitpos_in_chunk += write_integer_data::<i8>(bitpos_in_chunk, c_bits, writer),
        "i16" => *bitpos_in_chunk += write_integer_data::<i16>(bitpos_in_chunk, c_bits, writer),
        "i32" => *bitpos_in_chunk += write_integer_data::<i32>(bitpos_in_chunk, c_bits, writer),
        "i64" => *bitpos_in_chunk += write_integer_data::<i64>(bitpos_in_chunk, c_bits, writer),
        "i128" => *bitpos_in_chunk += write_integer_data::<i128>(bitpos_in_chunk, c_bits, writer),
        "f32" => {
            if *bitpos_in_chunk + size_in_bits::<f32>() <= c_bits.len() {
                let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<f32>()];
                myslice.copy_from_bitslice(
                    &c_bits[*bitpos_in_chunk..*bitpos_in_chunk + size_in_bits::<f32>()],
                );
                writer
                    .write_fmt(format_args!("{}\n", &myslice[0..32].load::<u32>().as_f32()))
                    .unwrap();
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .unwrap();
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
                    .unwrap();
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .unwrap();
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
                        .unwrap();
                    *bitpos_in_chunk += size_in_bits::<u8>();
                }
                writer.write_fmt(format_args!("\n")).unwrap();
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .unwrap();
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
                writer.write_fmt(format_args!("{}\n", target_int)).unwrap();
                *bitpos_in_chunk += len;
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .unwrap();
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
                writer.write_fmt(format_args!("{}\n", target_int)).unwrap();
                *bitpos_in_chunk += len;
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .unwrap();
            }
        }
        "bytegap" => {
            *bitpos_in_chunk += write_gap(bitpos_in_chunk, c_bits, writer, len, 8);
        }
        "bitgap" => {
            *bitpos_in_chunk += write_gap(bitpos_in_chunk, c_bits, writer, len, 1);
        }
        _ => eprintln!("unknown type"),
    }
    writer.flush().unwrap();
    Ok(())
}
