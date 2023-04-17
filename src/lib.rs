use anyhow::{Context, Result};
use args::Args;
use std::fmt::{Binary, Debug, Display, UpperHex};
use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
    mem::size_of,
};
use write::Stats;

pub mod args;
pub mod read;
pub mod write;

pub const MAX_READ_SIZE: usize = 16 * 1024;
pub const BYTE_TO_BIT: usize = 8;
pub const HEX_LINE_SIZE: usize = 16; // how many bytes are printed in a line with --rawhex
pub const BIN_LINE_SIZE: usize = 8; // how many bytes are printed in a line with --rawhex

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

fn format_number<T: Display + Debug + Binary + UpperHex>(num: T, format: Format) -> String {
    match format {
        Format::Norm => format!("{}", num),
        Format::Hex => format!("0x{:02X}", num),
        Format::Bin => format!("{:08b}", num),
    }
}

pub fn read_config(config_path: &str) -> Result<Vec<String>> {
    Ok(BufReader::new(File::open(config_path)?)
        .lines()
        .flatten()
        .filter(|l| !l.starts_with('#'))
        .collect())
}

pub fn size_in_bits<T>() -> usize {
    size_of::<T>() * BYTE_TO_BIT
}

pub fn print_raw_hex(writer: &mut dyn Write, chunk: &[u8], hex_lines: usize) -> Result<()> {
    for line in chunk.chunks(HEX_LINE_SIZE) {
        writer
            .write_fmt(format_args!("{:02X?}\n", line))
            .context("Could now write to writer")?;
    }
    // last chunk has less bytes, so print empty lines
    if chunk.chunks(HEX_LINE_SIZE).count() < hex_lines {
        let missing = hex_lines - chunk.chunks(HEX_LINE_SIZE).count();
        for _ in 0..missing {
            writer
                .write_all(b"\n")
                .context("Could now write to writer")?;
        }
    }
    Ok(())
}

pub fn print_raw_bin(writer: &mut dyn Write, chunk: &[u8], bin_lines: usize) -> Result<()> {
    for line in chunk.chunks(BIN_LINE_SIZE) {
        for byte in line {
            writer
                .write_fmt(format_args!("{:08b} ", byte))
                .context("Could now write to writer")?;
        }
        writer
            .write_all(b"\n")
            .context("Could now write to writer")?;
    }
    // last chunk has less bytes, so print empty lines
    if chunk.chunks(BIN_LINE_SIZE).count() < bin_lines {
        let missing = bin_lines - chunk.chunks(BIN_LINE_SIZE).count();
        for _ in 0..missing {
            writer
                .write_all(b"\n")
                .context("Could now write to writer")?;
        }
    }
    Ok(())
}

pub fn print_raw_ascii(writer: &mut dyn Write, chunk: &[u8], hex_lines: usize) -> Result<()> {
    for line in chunk.chunks(HEX_LINE_SIZE) {
        writer
            .write_fmt(format_args!("[ {} ]\n", String::from_utf8_lossy(line).chars().filter(|c| *c != '\n').collect::<String>()))
            .context("Could now write to writer")?;
    }
    // last chunk has less bytes, so print empty lines
    if chunk.chunks(HEX_LINE_SIZE).count() < hex_lines {
        let missing = hex_lines - chunk.chunks(HEX_LINE_SIZE).count();
        for _ in 0..missing {
            writer
                .write_all(b"\n")
                .context("Could now write to writer")?;
        }
    }
    Ok(())
}

pub fn print_timestamp(writer: &mut dyn Write) -> Result<()> {
    writer
        .write_fmt(format_args!("{}\n", chrono::offset::Local::now()))
        .context("Could now write to writer")?;
    Ok(())
}

pub fn print_bitpos(writer: &mut dyn Write, bitpos: usize) -> Result<()> {
    writer
        .write_fmt(format_args!(
            "byte {}, bit {}\n",
            bitpos / BYTE_TO_BIT,
            bitpos % BYTE_TO_BIT
        ))
        .context("Could now write to writer")?;
    Ok(())
}

pub fn count_lines(args: &Args, stats: &Stats, n_conf_lines: usize) -> u16 {
    let mut extra_lines: u16 = n_conf_lines as u16 + 1; // plus 1 for the last free line after a chunk
                                                        // count lines and reset cursor position ofter first run
    if args.rawhex {
        extra_lines += stats.hex_lines as u16;
    };
    if args.rawbin {
        extra_lines += stats.bin_lines as u16;
    };
    if args.rawascii {
        extra_lines += stats.hex_lines as u16;
    };
    if args.timestamp {
        extra_lines += 1;
    };
    if args.print_statistics {
        extra_lines += 5;
    };
    if args.print_bitpos {
        extra_lines += n_conf_lines as u16
    };
    if args.rawbin || args.rawhex || args.timestamp || args.print_statistics || args.rawascii {
        extra_lines += 1;
    }
    extra_lines
}

pub fn print_additional(
    args: &Args,
    stats: &Stats,
    writer: &mut dyn Write,
    chunk: &[u8],
    chunksize: usize,
) -> Result<()> {
    if args.timestamp {
        print_timestamp(writer)?;
    }
    if args.print_statistics {
        print_statistics(stats, writer, chunksize)?;
    }
    if args.rawhex {
        print_raw_hex(writer, chunk, stats.hex_lines)?;
    }
    if args.rawbin {
        print_raw_bin(writer, chunk, stats.bin_lines)?;
    }
    if args.rawascii {
        print_raw_ascii(writer, chunk, stats.hex_lines)?;
    }
    if args.rawbin || args.rawhex || args.timestamp || args.print_statistics {
        writer
            .write_all(b"\n")
            .context("Could now write to writer")?;
    }
    Ok(())
}

pub fn print_statistics(stats: &Stats, writer: &mut dyn Write, chunksize: usize) -> Result<()> {
    writer
        .write_fmt(format_args!(
            "Message no: {}\nMessage length: {} bytes\nChunk length: {} bytes\nCurrent chunk in this message: {}\nChunk starts at byte {} of message\n",
            stats.message_count, stats.message_len, chunksize, stats.chunk_count, stats.chunk_start
        ))
        .context("Could now write to writer")?;
    Ok(())
}

pub fn parse_config_line(conf_line: &str) -> Result<(&str, &str, Format, usize)> {
    // discard comments and whitespaces
    let line = match conf_line.split_once('#') {
        Some(s) => s.0,
        None => conf_line,
    };
    // split off any remaining whitespace
    let line = match line.split_once(' ') {
        Some(s) => s.0,
        None => line,
    };
    let (fieldname, rest) = line
        .split_once(':')
        .context("Syntax error in config, could not find : in line.")?;
    let (val_type, rest) = match rest.split_once(':') {
        Some(s) => (s.0, s.1),
        None => (rest, "0"),
    };
    // at this point, rest could be a letter (to print in hex or binary)
    // or a number (for stringlength)
    let form = Format::from_str(rest);
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
    use chrono::{Duration, Local, TimeZone};

    use super::*;

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
        // this must be 25 / 8 = 3
        // plus 1 for the last line
        // plus 1 for the free line beneath the additional info
        // plus 2 because the config lines are always counted
        let args = Args {
            infile: "nil".to_string(),
            outfile: "nil".to_string(),
            config: "nil".to_string(),
            chunksize: 0,
            offset: 0,
            bitoffset: 0,
            rawhex: false,
            rawbin: true,
            rawascii: false,
            pause: 0,
            little_endian: false,
            timestamp: false,
            read_head: 0,
            print_statistics: false,
            print_bitpos: false,
            cursor_jump: false,
        };
        let stats = Stats {
            message_count: 0,
            message_len: 0,
            chunk_count: 0,
            chunk_start: 0,
            hex_lines: 0,
            bin_lines: 3,
        };
        assert_eq!(count_lines(&args, &stats, 2), 7);
    }
    #[test]
    fn test_count_lines_rawhex() {
        // we divide the chunk into 16 byte wide lines, so therefore
        // this must be 58 / 16 = 3
        // plus 1 for the last line
        // plus 1 for the free line beneath the additional info
        // plus 2 because the config lines are always counted
        let args = Args {
            infile: "nil".to_string(),
            outfile: "nil".to_string(),
            config: "nil".to_string(),
            chunksize: 0,
            offset: 0,
            bitoffset: 0,
            rawhex: true,
            rawbin: false,
            rawascii: false,
            pause: 0,
            little_endian: false,
            timestamp: false,
            read_head: 0,
            print_statistics: false,
            print_bitpos: false,
            cursor_jump: false,
        };
        let stats = Stats {
            message_count: 0,
            message_len: 0,
            chunk_count: 0,
            chunk_start: 0,
            hex_lines: 3,
            bin_lines: 0,
        };
        assert_eq!(count_lines(&args, &stats, 2), 7);
    }
    #[test]
    fn test_count_lines_rawascii() {
        // we divide the chunk into 16 byte wide lines, so therefore
        // this must be 58 / 16 = 3
        // plus 1 for the last line
        // plus 1 for the free line beneath the additional info
        // plus 2 because the config lines are always counted
        let args = Args {
            infile: "nil".to_string(),
            outfile: "nil".to_string(),
            config: "nil".to_string(),
            chunksize: 0,
            offset: 0,
            bitoffset: 0,
            rawhex: false,
            rawbin: false,
            rawascii: true,
            pause: 0,
            little_endian: false,
            timestamp: false,
            read_head: 0,
            print_statistics: false,
            print_bitpos: false,
            cursor_jump: false,
        };
        let stats = Stats {
            message_count: 0,
            message_len: 0,
            chunk_count: 0,
            chunk_start: 0,
            hex_lines: 3,
            bin_lines: 0,
        };
        assert_eq!(count_lines(&args, &stats, 2), 7);
    }
    #[test]
    fn test_count_lines_bitpos() {
        // 2 for every bitpos above the config lines output
        // plus 2 because the config lines are always counted
        // plus 1 for the free line beneath the additional info
        // plus 1 for the last line
        let args = Args {
            infile: "nil".to_string(),
            outfile: "nil".to_string(),
            config: "nil".to_string(),
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
            print_bitpos: true,
            cursor_jump: false,
        };
        let stats = Stats {
            message_count: 0,
            message_len: 0,
            chunk_count: 0,
            chunk_start: 0,
            hex_lines: 99,
            bin_lines: 99,
        };
        assert_eq!(count_lines(&args, &stats, 2), 5);
    }
    #[test]
    fn test_count_lines_stats() {
        // 5 from the stats
        // plus 2 because the config lines are always counted
        // plus 1 for the free line beneath the additional info
        // plus 1 for the last line
        let args = Args {
            infile: "nil".to_string(),
            outfile: "nil".to_string(),
            config: "nil".to_string(),
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
            print_statistics: true,
            print_bitpos: false,
            cursor_jump: false,
        };
        let stats = Stats {
            message_count: 0,
            message_len: 0,
            chunk_count: 0,
            chunk_start: 0,
            hex_lines: 0,
            bin_lines: 3,
        };
        assert_eq!(count_lines(&args, &stats, 2), 9);
    }
    #[test]
    fn test_count_lines_timestamp() {
        // 1 from the timestamp
        // plus 2 because the config lines are always counted
        // plus 1 for the free line beneath the additional info
        // plus 1 for the last line
        let args = Args {
            infile: "nil".to_string(),
            outfile: "nil".to_string(),
            config: "nil".to_string(),
            chunksize: 0,
            offset: 0,
            bitoffset: 0,
            rawhex: false,
            rawbin: false,
            rawascii: false,
            pause: 0,
            little_endian: false,
            timestamp: true,
            read_head: 0,
            print_statistics: false,
            print_bitpos: false,
            cursor_jump: false,
        };
        let stats = Stats {
            message_count: 0,
            message_len: 0,
            chunk_count: 0,
            chunk_start: 0,
            hex_lines: 0,
            bin_lines: 3,
        };
        assert_eq!(count_lines(&args, &stats, 2), 5);
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
    #[test]
    fn test_print_raw_hex() {
        let chunk: [u8; 20] = [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x01, 0x02, 0x03, 0x04,
            0x05, 0x06, 0x07, 0x08, 0x09, 0x0A,
        ];

        let mut output = Vec::new();
        let hex_lines = 2;
        print_raw_hex(&mut output, &chunk, hex_lines).unwrap();
        assert_eq!(
            output,
            b"[01, 02, 03, 04, 05, 06, 07, 08, 09, 0A, 01, 02, 03, 04, 05, 06]\n[07, 08, 09, 0A]\n"
        );
    }
    #[test]
    fn test_print_raw_bin() {
        let chunk: [u8; 10] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A];

        let mut output = Vec::new();
        let bin_lines = 2;
        print_raw_bin(&mut output, &chunk, bin_lines).unwrap();
        assert_eq!(output, b"00000001 00000010 00000011 00000100 00000101 00000110 00000111 00001000 \n00001001 00001010 \n");
    }
    #[test]
    fn test_print_timestamp() {
        let mut output = Vec::new();
        print_timestamp(&mut output).unwrap();
        let output = String::from_utf8(output).unwrap();
        let dt = Local
            .datetime_from_str(&output, "%Y-%m-%d %H:%M:%S%.f %:z\n")
            .unwrap();
        assert!(chrono::offset::Local::now() - dt < Duration::seconds(10));
    }
}
