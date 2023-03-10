use std::io::Write;
use std::mem::size_of;

pub mod args;
pub mod write;
pub mod read;

pub const MAX_READ_SIZE: usize = 16 * 1024;
pub const BYTE_TO_BIT: usize = 8;

pub fn size_in_bits<T>() -> usize {
    size_of::<T>() * BYTE_TO_BIT
}

pub fn print_raws(c: &[u8], rawhex: bool, rawbin: bool, writer: &mut Box<dyn Write>) {
    if rawhex {
        writer.write_fmt(format_args!("{:02X?}\n", c)).unwrap();
    }
    if rawbin {
        for b in c {
            writer.write_fmt(format_args!("{:08b} ", b)).unwrap();
        }
        writer.write_all(b"\n\n").unwrap();
    }
}

pub fn chunksize_by_config(config_lines: &Vec<String>) -> usize {
    let mut bitlength = 0;
    for conf_line in config_lines.iter() {
        let (_, rest) = conf_line.split_once(':').unwrap();
        let (val_type, len) = match rest.split_once(':') {
            Some(s) => (s.0, s.1.parse().unwrap_or_default()),
            None => (rest, 0),
        };

        let val_type = val_type.to_lowercase(); // don't care about type
        match val_type.as_str() {
            "bool1" => bitlength += 1,
            "bool8" | "u8" | "i8" => bitlength += size_in_bits::<u8>(),
            "u16" | "i16" => bitlength += size_in_bits::<u16>(),
            "u32" | "i32" | "f32" => bitlength += size_in_bits::<u32>(),
            "u64" | "i64" | "f64" => bitlength += size_in_bits::<u64>(),
            "u128" | "i128" => bitlength += size_in_bits::<u128>(),
            "string" | "bytegap" => bitlength += len * size_in_bits::<u8>(),
            "iarb" | "uarb" => bitlength += len,
            "bitgap" => bitlength += len,
            _ => eprintln!("unknown type"),
        }
    }
    bitlength
}
