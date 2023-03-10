use crate::size_in_bits;
use bitvec::macros::internal::funty::Fundamental;
use bitvec::prelude::*;
use std::io::{Result, Write};

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
    match val_type {
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
        "u8" => {
            if *bitpos_in_chunk + size_in_bits::<u8>() <= c_bits.len() {
                let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<u8>()];
                myslice.copy_from_bitslice(
                    &c_bits[*bitpos_in_chunk..*bitpos_in_chunk + size_in_bits::<u8>()],
                );
                writer
                    .write_fmt(format_args!("{}\n", &myslice[0..8].load::<u8>()))
                    .unwrap();
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .unwrap();
            }
            *bitpos_in_chunk += size_in_bits::<u8>();
        }
        "u16" => {
            if *bitpos_in_chunk + size_in_bits::<u16>() <= c_bits.len() {
                let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<u16>()];
                myslice.copy_from_bitslice(
                    &c_bits[*bitpos_in_chunk..*bitpos_in_chunk + size_in_bits::<u16>()],
                );
                writer
                    .write_fmt(format_args!("{}\n", &myslice[0..16].load::<u16>()))
                    .unwrap();
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .unwrap();
            }
            *bitpos_in_chunk += size_in_bits::<u16>();
        }
        "u32" => {
            if *bitpos_in_chunk + size_in_bits::<u32>() <= c_bits.len() {
                let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<u32>()];
                myslice.copy_from_bitslice(
                    &c_bits[*bitpos_in_chunk..*bitpos_in_chunk + size_in_bits::<u32>()],
                );
                writer
                    .write_fmt(format_args!("{}\n", &myslice[0..32].load::<u32>()))
                    .unwrap();
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .unwrap();
            }
            *bitpos_in_chunk += size_in_bits::<u32>();
        }
        "u64" => {
            if *bitpos_in_chunk + size_in_bits::<u64>() <= c_bits.len() {
                let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<u64>()];
                myslice.copy_from_bitslice(
                    &c_bits[*bitpos_in_chunk..*bitpos_in_chunk + size_in_bits::<u64>()],
                );
                writer
                    .write_fmt(format_args!("{}\n", &myslice[0..64].load::<u64>()))
                    .unwrap();
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .unwrap();
            }
            *bitpos_in_chunk += size_in_bits::<u64>();
        }
        "u128" => {
            if *bitpos_in_chunk + size_in_bits::<u128>() <= c_bits.len() {
                let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<u128>()];
                myslice.copy_from_bitslice(
                    &c_bits[*bitpos_in_chunk..*bitpos_in_chunk + size_in_bits::<u128>()],
                );
                writer
                    .write_fmt(format_args!("{}\n", &myslice[0..128].load::<u128>()))
                    .unwrap();
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .unwrap();
            }
            *bitpos_in_chunk += size_in_bits::<u128>();
        }
        "i8" => {
            if *bitpos_in_chunk + size_in_bits::<u8>() <= c_bits.len() {
                let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<i8>()];
                myslice.copy_from_bitslice(
                    &c_bits[*bitpos_in_chunk..*bitpos_in_chunk + size_in_bits::<i8>()],
                );
                writer
                    .write_fmt(format_args!("{}\n", &myslice[0..8].load::<i8>()))
                    .unwrap();
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .unwrap();
            }
            *bitpos_in_chunk += size_in_bits::<i8>();
        }
        "i16" => {
            if *bitpos_in_chunk + size_in_bits::<i16>() <= c_bits.len() {
                let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<i16>()];
                myslice.copy_from_bitslice(
                    &c_bits[*bitpos_in_chunk..*bitpos_in_chunk + size_in_bits::<i16>()],
                );
                writer
                    .write_fmt(format_args!("{}\n", &myslice[0..16].load::<i16>()))
                    .unwrap();
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .unwrap();
            }
            *bitpos_in_chunk += size_in_bits::<i16>();
        }
        "i32" => {
            if *bitpos_in_chunk + size_in_bits::<i32>() <= c_bits.len() {
                let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<i32>()];
                myslice.copy_from_bitslice(
                    &c_bits[*bitpos_in_chunk..*bitpos_in_chunk + size_in_bits::<i32>()],
                );
                writer
                    .write_fmt(format_args!("{}\n", &myslice[0..32].load::<i32>()))
                    .unwrap();
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .unwrap();
            }
            *bitpos_in_chunk += size_in_bits::<i32>();
        }
        "i64" => {
            if *bitpos_in_chunk + size_in_bits::<i64>() <= c_bits.len() {
                let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<i64>()];
                myslice.copy_from_bitslice(
                    &c_bits[*bitpos_in_chunk..*bitpos_in_chunk + size_in_bits::<i64>()],
                );
                writer
                    .write_fmt(format_args!("{}\n", &myslice[0..64].load::<i64>()))
                    .unwrap();
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .unwrap();
            }
            *bitpos_in_chunk += size_in_bits::<i64>();
        }
        "i128" => {
            if *bitpos_in_chunk + size_in_bits::<i128>() <= c_bits.len() {
                let mut myslice = bitvec![u8, Msb0; 0; size_in_bits::<i128>()];
                myslice.copy_from_bitslice(
                    &c_bits[*bitpos_in_chunk..*bitpos_in_chunk + size_in_bits::<i128>()],
                );
                writer
                    .write_fmt(format_args!("{}\n", &myslice[0..128].load::<i128>()))
                    .unwrap();
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .unwrap();
            }
        }
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
        "string" | "String" => {
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
            if *bitpos_in_chunk + len * size_in_bits::<u8>() <= c_bits.len() {
                writer
                    .write_fmt(format_args!("(gap of {} byte)\n", len))
                    .unwrap();
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .unwrap();
            }
            *bitpos_in_chunk += len * size_in_bits::<u8>();
        }
        "bitgap" => {
            if *bitpos_in_chunk + len <= c_bits.len() {
                writer
                    .write_fmt(format_args!("(gap of {} bit)\n", len))
                    .unwrap();
            } else {
                writer
                    .write_all(b"values size is bigger than what is left of that data chunk\n")
                    .unwrap();
            }
            *bitpos_in_chunk += len;
        }
        _ => eprintln!("unknown type"),
    }
    writer.flush().unwrap();
    Ok(())
}
