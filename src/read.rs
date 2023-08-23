use crate::{args::Args, MAX_READ_SIZE};
use anyhow::{Context, Result};
use crossbeam::channel::Sender;
use pcap_parser::traits::PcapReaderIterator;
use pcap_parser::*;
use std::{
    fs::File,
    io::{self, BufReader, Read},
};

pub fn read_loop(args: &Args, write_tx: Sender<Vec<u8>>) -> Result<()> {
    if args.pcap {
        let reader: Box<dyn Read> = if !args.infile.is_empty() {
            Box::new(BufReader::new(File::open(&args.infile)?))
        } else {
            Box::new(BufReader::new(io::stdin()))
        };
        let mut pcapreader =
            LegacyPcapReader::new(MAX_READ_SIZE, reader).context("Error creating PCAP reader.")?;
        loop {
            // read input
            match pcapreader.next() {
                Ok((offset, block)) => {
                    if let PcapBlockOwned::Legacy(ablock) = block {
                        if write_tx.send(Vec::from(ablock.data)).is_err() {
                            break;
                        }
                    }
                    pcapreader.consume(offset);
                }
                Err(PcapError::Eof) => {
                    if args.infile.is_empty() {
                        continue;
                    } else {
                        break;
                    }
                }
                Err(PcapError::Incomplete) => {
                    pcapreader.refill().unwrap();
                }
                Err(e) => panic!("error while reading: {:?}", e),
            }
        }
    } else {
        let mut reader: Box<dyn Read> = if !args.infile.is_empty() {
            Box::new(BufReader::new(File::open(&args.infile)?))
        } else {
            Box::new(BufReader::new(io::stdin()))
        };
        let mut buffer = [0; MAX_READ_SIZE];
        loop {
            let num_read = match reader.read(&mut buffer) {
                Ok(0) => {
                    if args.infile.is_empty() {
                        continue;
                    } else {
                        break;
                    }
                }
                Ok(x) => x,
                Err(_) => break,
            };
            if args.read_head > 0 {
                if write_tx.send(Vec::from(&buffer[..args.read_head])).is_err() {
                    break;
                }
                break;
            } else {
                if write_tx.send(Vec::from(&buffer[..num_read])).is_err() {
                    break;
                }
            }
        }
    }

    let _ = write_tx.send(Vec::new());
    Ok(())
}
