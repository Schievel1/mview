use crate::size_in_bits;
use bitvec::macros::internal::funty::{Fundamental, Integral};
use bitvec::prelude::*;
use std::io::{Result, Write};

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
    conf_line: &String,
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
                let target_int;
                let mut target_slice: [u8; 16] = [0; 16];
                let int_bits = target_slice.view_bits_mut::<Lsb0>();
                for i in 0..len {
                    int_bits.set(i, c_bits[*bitpos_in_chunk + i]); // copy the payload over
                }
                target_int = int_bits.load::<i128>();
                writer.write_fmt(format_args!("{}\n", target_int)).unwrap();
                *bitpos_in_chunk += len;
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .unwrap();
            }
        }
        "bytegap" => {
            *bitpos_in_chunk += write_gap(&bitpos_in_chunk, c_bits, writer, len, 8);
        }
        "bitgap" => {
            *bitpos_in_chunk += write_gap(&bitpos_in_chunk, c_bits, writer, len, 1);
        }
        _ => eprintln!("unknown type"),
    }
    writer.flush().unwrap();
    Ok(())
}
