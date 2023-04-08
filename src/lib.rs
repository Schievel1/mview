use std::fmt::Binary;
use std::fmt::{Debug, Display};
use std::io::Write;
use std::mem::size_of;

pub mod args;
pub mod read;
pub mod write;

pub const MAX_READ_SIZE: usize = 16 * 1024;
pub const BYTE_TO_BIT: usize = 8;

pub enum Format {
    Norm,
    Hex,
    Bin,
}
impl Format {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "n" | "normal" | "norm" => Self::Norm,
            "h" | "hex" | "hexadecimal" => Self::Hex,
            "b" | "bin" | "binary" => Self::Bin,
            _ => Self::Norm,
        }
    }
}

fn format_number<T: Display + Debug + Binary>(num: T, format: Format) -> String {
    match format {
        Format::Norm => format!("{}", num),
        Format::Hex => format!("0x{:02X?}", num),
        Format::Bin => format!("{:08b}", num),
    }
}

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

pub fn print_timestamp(writer: &mut Box<dyn Write>) {
    writer
        .write_fmt(format_args!("{}\n", chrono::offset::Local::now()))
        .unwrap()
}

pub fn print_bitpos(writer: &mut Box<dyn Write>, bitpos: usize) {
    writer
        .write_fmt(format_args!(
            "byte {}, bit {}\n",
            bitpos / BYTE_TO_BIT,
            bitpos % BYTE_TO_BIT
        ))
        .unwrap()
}

pub fn print_statistics(
    writer: &mut Box<dyn Write>,
    message_count: u32,
    message_len: u32,
    chunk_count: u32,
) {
    writer
        .write_fmt(format_args!(
            "Message no: {}\nMessage length: {} bytes\nCurrent chunk in this message: {}\n",
            message_count, message_len, chunk_count
        ))
        .unwrap();
}

pub fn parse_config_line(conf_line: &str) -> (&str, &str, Format, usize) {
    let (fieldname, rest) = conf_line.split_once(':').unwrap();
    let (val_type, rest) = match rest.split_once(':') {
        Some(s) => (s.0, s.1),
        None => (rest, "0"),
    };
    // at this point, rest could be a letter (to print in hex or binary)
    // or a number (for stringlength)
    let form = Format::from_str(&rest);
    let mut len = 0;
    if let Ok(n) = rest.parse() {
        len = n;
    }
    (fieldname, val_type, form, len)
}

// calculate the size of a chunk using the config, returns bits!
pub fn chunksize_by_config(config_lines: &[String]) -> usize {
    let mut bitlength = 0;
    for conf_line in config_lines.iter() {
        let (_, val_type, _, len) = parse_config_line(conf_line);
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

#[cfg(test)]
mod tests {
    use crate::chunksize_by_config;

    #[test]
    fn test_chunksize_by_config_bool1() {
        let config = "Field:bool1";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), 1);
    }
    #[test]
    fn test_chunksize_by_config_bool8() {
        let config = "Field:bool8";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), 8);
    }
    #[test]
    fn test_chunksize_by_config_u8() {
        let config = "Field:u8";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), 8);
    }
    #[test]
    fn test_chunksize_by_config_i8() {
        let config = "Field:i8";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), 8);
    }
    #[test]
    fn test_chunksize_by_config_u16() {
        let config = "Field:u16";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), 16);
    }
    #[test]
    fn test_chunksize_by_config_i16() {
        let config = "Field:i16";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), 16);
    }
    #[test]
    fn test_chunksize_by_config_u32() {
        let config = "Field:u32";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), 32);
    }
    #[test]
    fn test_chunksize_by_config_i32() {
        let config = "Field:i32";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), 32);
    }
    #[test]
    fn test_chunksize_by_config_f32() {
        let config = "Field:f32";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), 32);
    }
    #[test]
    fn test_chunksize_by_config_u64() {
        let config = "Field:u64";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), 64);
    }
    #[test]
    fn test_chunksize_by_config_i64() {
        let config = "Field:i64";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), 64);
    }
    #[test]
    fn test_chunksize_by_config_f64() {
        let config = "Field:f64";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), 64);
    }
    #[test]
    fn test_chunksize_by_config_u128() {
        let config = "Field:u128";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), 128);
    }
    #[test]
    fn test_chunksize_by_config_i128() {
        let config = "Field:i128";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), 128);
    }
    #[test]
    fn test_chunksize_by_config_string4() {
        let config = "Field:string:4";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), (4 * 8));
    }
    #[test]
    fn test_chunksize_by_config_string8() {
        let config = "Field:string:8";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), (8 * 8));
    }
    #[test]
    fn test_chunksize_by_config_stringlong() {
        let config = "Field:string:32000";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), (32000 * 8));
    }
    #[test]
    fn test_chunksize_by_config_bytegap4() {
        let config = "Field:bytegap:4";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), (4 * 8));
    }
    #[test]
    fn test_chunksize_by_config_bytegap8() {
        let config = "Field:bytegap:8";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), (8 * 8));
    }
    #[test]
    fn test_chunksize_by_config_bytegaplong() {
        let config = "Field:bytegap:32000";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), (32000 * 8));
    }
    #[test]
    fn test_chunksize_by_config_bitgap4() {
        let config = "Field:bitgap:4";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), 4);
    }
    #[test]
    fn test_chunksize_by_config_bitgap8() {
        let config = "Field:bitgap:8";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), 8);
    }
    #[test]
    fn test_chunksize_by_config_bitgaplong() {
        let config = "Field:bitgap:32000";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), 32000);
    }
    #[test]
    fn test_chunksize_by_config_iarb() {
        let config = "Field:iarb:7";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), 7);
    }
    #[test]
    fn test_chunksize_by_config_uarb() {
        let config = "Field:uarb:7";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), 7);
    }
    #[test]
    fn test_chunksize_by_config_case_insensitive() {
        let config = "Field:bOoL8";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), 8);
    }
    #[test]
    fn test_chunksize_by_config_sum() {
        let config = "Field0(bool1):bool1
Field1(bool1):bool1
Field2(bool1):bool1
Field3(bool1):bool1
Field4(bool1):bool1
Field5(bool1):bool1
Field6(bool1):bool1
Field7(bool1):bool1
Field8(bool8):bool8
Two_Bytegap(bytegap2):bytegap:2
Four_Bitgap(bitgap4):bitgap:4
Field9(u8):u8
Field10(u16):u16
Field11(i32):i32
Field12(String4):String:4
Field13(iarb7):iarb:7
Field14(uarb4):uarb:4"; // should sum up to 135 bits
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines), 135);
    }
}
