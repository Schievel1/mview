use std::io::Write;
use std::mem::size_of;

pub mod args;
pub mod write;

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
