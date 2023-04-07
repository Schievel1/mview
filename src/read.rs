use crate::MAX_READ_SIZE;
use crossbeam::channel::Sender;
use std::fs::File;
use std::io::{self, BufReader, Read, Result};

pub fn read_loop(infile: &str, write_tx: Sender<Vec<u8>>, read_head: usize) -> Result<()> {
    let mut reader: Box<dyn Read> = if !infile.is_empty() {
        Box::new(BufReader::new(File::open(infile)?))
    } else {
        Box::new(BufReader::new(io::stdin()))
    };

    let mut buffer = [0; MAX_READ_SIZE];
    loop {
        // read input
        let num_read = match reader.read(&mut buffer) {
            Ok(0) => continue,
            Ok(x) => x,
            Err(_) => break,
        };
        if read_head > 0 {
            if write_tx.send(Vec::from(&buffer[..read_head])).is_err() {
                break;
            }
            break;
        } else {
            if write_tx.send(Vec::from(&buffer[..num_read])).is_err() {
                break;
            }
        }
        buffer.fill(0);
    }
    let _ = write_tx.send(Vec::new());
    Ok(())
}
