use anyhow::Result;
use anyhow::Context;
use std::fmt::Binary;
use std::fmt::{Debug, Display};
use std::io::Write;
use std::mem::size_of;

pub mod args;
pub mod read;
pub mod write;

pub const MAX_READ_SIZE: usize = 16 * 1024;
pub const BYTE_TO_BIT: usize = 8;

#[derive(Debug, PartialEq)]
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

pub fn print_raw_hex(writer: &mut Box<dyn Write>, chunk: &[u8]) -> Result<()> {
    for line in chunk.chunks(16) {
        writer
            .write_fmt(format_args!("{:02X?}\n", line))
            .context("Could now write to writer")?;
    }
    Ok(())
}

pub fn print_raw_bin(writer: &mut Box<dyn Write>, chunk: &[u8]) -> Result<()> {
    for line in chunk.chunks(8) {
        for byte in line {
            writer
                .write_fmt(format_args!("{:08b} ", byte))
                .context("Could now write to writer")?;
        }
        writer.write_all(b"\n").context("Could now write to writer")?;
    }
    Ok(())
}

pub fn print_timestamp(writer: &mut Box<dyn Write>) -> Result<()> {
    writer
        .write_fmt(format_args!("{}\n", chrono::offset::Local::now()))
        .context("Could now write to writer")?;
    Ok(())
}

pub fn print_bitpos(writer: &mut Box<dyn Write>, bitpos: usize) -> Result<()> {
    writer
        .write_fmt(format_args!(
            "byte {}, bit {}\n",
            bitpos / BYTE_TO_BIT,
            bitpos % BYTE_TO_BIT
        ))
        .context("Could now write to writer")?;
    Ok(())
}

pub fn count_lines(
    rawbin: bool,
    rawhex: bool,
    timestamp: bool,
    statistics: bool,
    bitpos: bool,
    config_lines: usize,
    chunk: &[u8],
) -> u16 {
    let mut extra_lines: u16 = config_lines as u16;
    // count lines and reset cursor position ofter first run
    if rawbin {
        extra_lines += chunk.chunks(8).count() as u16;
    };
    if rawhex {
        extra_lines += chunk.chunks(16).count() as u16;
    };
    if timestamp {
        extra_lines += 1;
    };
    if statistics {
        extra_lines += 3;
    };
    if bitpos {
        extra_lines += config_lines as u16
    };
    if rawbin || rawhex || timestamp || statistics {
        extra_lines += 1;
    }
    extra_lines
}

pub fn print_additional(
    writer: &mut Box<dyn Write>,
    rawbin: bool,
    rawhex: bool,
    timestamp: bool,
    statistics: bool,
    chunk: &[u8],
    message_count: u32,
    message_len: u32,
    chunk_count: u32,
) -> Result<()> {
    if timestamp {
        print_timestamp(writer)?;
    }
    if statistics {
        print_statistics(writer, message_count, message_len, chunk_count)?;
    }
    if rawhex {
        print_raw_hex(writer, chunk)?;
    }
    if rawbin {
        print_raw_bin(writer, chunk)?;
    }
    if rawbin || rawhex || timestamp || statistics {
        writer.write_all(b"\n").context("Could now write to writer")?;
    }
    Ok(())
}

pub fn print_statistics(
    writer: &mut Box<dyn Write>,
    message_count: u32,
    message_len: u32,
    chunk_count: u32,
) -> Result<()> {
    writer
        .write_fmt(format_args!(
            "Message no: {}\nMessage length: {} bytes\nCurrent chunk in this message: {}\n",
            message_count, message_len, chunk_count
        ))
        .context("Could now write to writer")?;
    Ok(())
}

pub fn parse_config_line(conf_line: &str) -> Result<(&str, &str, Format, usize)> {
    let (fieldname, rest) = conf_line.split_once(':').context("Syntax error in config, could not find : in line.")?;
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
    Ok((fieldname, val_type, form, len))
}

// calculate the size of a chunk using the config, returns bits!
pub fn chunksize_by_config(config_lines: &[String]) -> Result<usize> {
    let mut bitlength = 0;
    for conf_line in config_lines.iter() {
        let (_, val_type, _, len) = parse_config_line(conf_line)?;
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
    Ok(bitlength)
}

#[cfg(test)]
mod tests {
    use crate::{chunksize_by_config, count_lines, parse_config_line, size_in_bits, Format};

    #[test]
    fn test_chunksize_by_config_bool1() {
        let config = "Field:bool1";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), 1);
    }
    #[test]
    fn test_chunksize_by_config_bool8() {
        let config = "Field:bool8";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), 8);
    }
    #[test]
    fn test_chunksize_by_config_u8() {
        let config = "Field:u8";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), 8);
    }
    #[test]
    fn test_chunksize_by_config_i8() {
        let config = "Field:i8";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), 8);
    }
    #[test]
    fn test_chunksize_by_config_u16() {
        let config = "Field:u16";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), 16);
    }
    #[test]
    fn test_chunksize_by_config_i16() {
        let config = "Field:i16";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), 16);
    }
    #[test]
    fn test_chunksize_by_config_u32() {
        let config = "Field:u32";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), 32);
    }
    #[test]
    fn test_chunksize_by_config_i32() {
        let config = "Field:i32";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), 32);
    }
    #[test]
    fn test_chunksize_by_config_f32() {
        let config = "Field:f32";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), 32);
    }
    #[test]
    fn test_chunksize_by_config_u64() {
        let config = "Field:u64";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), 64);
    }
    #[test]
    fn test_chunksize_by_config_i64() {
        let config = "Field:i64";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), 64);
    }
    #[test]
    fn test_chunksize_by_config_f64() {
        let config = "Field:f64";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), 64);
    }
    #[test]
    fn test_chunksize_by_config_u128() {
        let config = "Field:u128";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), 128);
    }
    #[test]
    fn test_chunksize_by_config_i128() {
        let config = "Field:i128";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), 128);
    }
    #[test]
    fn test_chunksize_by_config_string4() {
        let config = "Field:string:4";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), (4 * 8));
    }
    #[test]
    fn test_chunksize_by_config_string8() {
        let config = "Field:string:8";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), (8 * 8));
    }
    #[test]
    fn test_chunksize_by_config_stringlong() {
        let config = "Field:string:32000";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), (32000 * 8));
    }
    #[test]
    fn test_chunksize_by_config_bytegap4() {
        let config = "Field:bytegap:4";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), (4 * 8));
    }
    #[test]
    fn test_chunksize_by_config_bytegap8() {
        let config = "Field:bytegap:8";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), (8 * 8));
    }
    #[test]
    fn test_chunksize_by_config_bytegaplong() {
        let config = "Field:bytegap:32000";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), (32000 * 8));
    }
    #[test]
    fn test_chunksize_by_config_bitgap4() {
        let config = "Field:bitgap:4";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), 4);
    }
    #[test]
    fn test_chunksize_by_config_bitgap8() {
        let config = "Field:bitgap:8";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), 8);
    }
    #[test]
    fn test_chunksize_by_config_bitgaplong() {
        let config = "Field:bitgap:32000";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), 32000);
    }
    #[test]
    fn test_chunksize_by_config_iarb() {
        let config = "Field:iarb:7";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), 7);
    }
    #[test]
    fn test_chunksize_by_config_uarb() {
        let config = "Field:uarb:7";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), 7);
    }
    #[test]
    fn test_chunksize_by_config_case_insensitive() {
        let config = "Field:bOoL8";
        let config_lines: Vec<String> = config.lines().map(|s| s.to_owned()).collect();
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), 8);
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
        assert_eq!(chunksize_by_config(&config_lines).unwrap(), 135);
    }

    #[test]
    fn test_size_in_bits() {
        assert_eq!(size_in_bits::<u16>(), 16);
    }

    #[test]
    fn test_count_lines_rawbin() {
        // we divide the chunk into 8 byte wide lines, so therefore
        // this must be 25 / 8 = 3 plus 1 for the last lines
        // plus 1 for the free lines beneath the additional info
        // plus 2 because the config lines are always counted
        let chunk: [u8; 25] = [0xFF; 25];
        let conf_lines = 2;
        assert_eq!(
            count_lines(true, false, false, false, false, conf_lines, &chunk),
            7
        );
    }
    #[test]
    fn test_count_lines_rawhex() {
        // we divide the chunk into 16 byte wide lines, so therefore
        // this must be 58 / 16 = 3 plus 1 for the last lines
        // plus 1 for the free lines beneath the additional info
        // plus 2 because the config lines are always counted
        let chunk: [u8; 58] = [0xFF; 58];
        let conf_lines = 2;
        assert_eq!(
            count_lines(false, true, false, false, false, conf_lines, &chunk),
            7
        );
    }
    #[test]
    fn test_count_lines_bitpos() {
        // plus 2 because the config lines are always counted
        let chunk: [u8; 2] = [0xFF, 25];
        let conf_lines = 2;
        assert_eq!(
            count_lines(false, false, false, false, true, conf_lines, &chunk),
            4
        );
    }
    #[test]
    fn test_count_lines_other() {
        // plus 2 because the config lines are always counted
        let chunk: [u8; 2] = [0xFF, 25];
        let conf_lines = 2;
        assert_eq!(
            count_lines(false, false, true, false, false, conf_lines, &chunk),
            4
        );
        assert_eq!(
            count_lines(false, false, true, true, false, conf_lines, &chunk),
            7
        );
    }

    #[test]
    fn test_parse_config_line_simple() {
        let conf_line = "Testfield:u8";
        let parsed_line = parse_config_line(conf_line).unwrap();
        assert_eq!(parsed_line.0, "Testfield");
        assert_eq!(parsed_line.1, "u8");
        assert_eq!(parsed_line.2, Format::Norm);
    }
    #[test]
    fn test_parse_config_line_len() {
        let conf_line = "Testfield:STriNg:4";
        let parsed_line = parse_config_line(conf_line).unwrap();
        assert_eq!(parsed_line.0, "Testfield");
        assert_eq!(parsed_line.1, "STriNg");
        assert_eq!(parsed_line.3, 4);
    }
    #[test]
    fn test_parse_config_line_h() {
        let conf_line = "Testfield:u16:h";
        let parsed_line = parse_config_line(conf_line).unwrap();
        assert_eq!(parsed_line.0, "Testfield");
        assert_eq!(parsed_line.1, "u16");
        assert_eq!(parsed_line.2, Format::Hex);
    }
    #[test]
    fn test_parse_config_line_hex() {
        let conf_line = "Testfield:u16:hex";
        let parsed_line = parse_config_line(conf_line).unwrap();
        assert_eq!(parsed_line.0, "Testfield");
        assert_eq!(parsed_line.1, "u16");
        assert_eq!(parsed_line.2, Format::Hex);
    }
    #[test]
    fn test_parse_config_line_hexadecimal() {
        let conf_line = "Testfield:u16:hexadecimal";
        let parsed_line = parse_config_line(conf_line).unwrap();
        assert_eq!(parsed_line.0, "Testfield");
        assert_eq!(parsed_line.1, "u16");
        assert_eq!(parsed_line.2, Format::Hex);
    }
    #[test]
    fn test_parse_config_line_b() {
        let conf_line = "Testfield:u16:b";
        let parsed_line = parse_config_line(conf_line).unwrap();
        assert_eq!(parsed_line.0, "Testfield");
        assert_eq!(parsed_line.1, "u16");
        assert_eq!(parsed_line.2, Format::Bin);
    }
    #[test]
    fn test_parse_config_line_binary() {
        let conf_line = "Testfield:u16:binary";
        let parsed_line = parse_config_line(conf_line).unwrap();
        assert_eq!(parsed_line.0, "Testfield");
        assert_eq!(parsed_line.1, "u16");
        assert_eq!(parsed_line.2, Format::Bin);
    }
    #[test]
    fn test_parse_config_line_unknown_format() {
        // if the format is unknown, Format::Norm should be returned
        let conf_line = "Testfield:u16:dunno";
        let parsed_line = parse_config_line(conf_line).unwrap();
        assert_eq!(parsed_line.0, "Testfield");
        assert_eq!(parsed_line.1, "u16");
        assert_eq!(parsed_line.2, Format::Norm);
    }
}
