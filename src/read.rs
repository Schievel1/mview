use crate::{args::Args, PcapTs, MAX_READ_SIZE};
use anyhow::{Context, Result};
use crossbeam::channel::Sender;
use pcap_parser::traits::PcapReaderIterator;
use pcap_parser::*;
use std::sync::{Arc, Mutex};
use std::{
    fs::File,
    io::{self, BufReader, Read},
};

const PCAP_MAGIC_US: u32 = 0xA1B2C3D4;
const PCAP_MAGIC_US_BE: u32 = 0xD4C3B2A1;
const PCAP_MAGIC_NS: u32 = 0xA1B23C4D;
const PCAP_MAGIC_NS_BE: u32 = 0x4D3CB2A1;

pub fn read_loop(
    args: &Args,
    write_tx: Sender<Vec<u8>>,
    pcap_ts: Arc<Mutex<PcapTs>>,
) -> Result<()> {
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
                    match block {
                        PcapBlockOwned::Legacy(ablock) => {
                            if write_tx.send(ablock.to_vec_raw()?).is_err() {
                                break;
                            }
                        }
                        PcapBlockOwned::LegacyHeader(fileheader) => match fileheader.magic_number {
                            PCAP_MAGIC_US | PCAP_MAGIC_US_BE => {
                                let mut pcap_ts_format = pcap_ts.lock().unwrap(); // we can use unwrap here,
                                                                                  // if the mutex can't not be aquired using lock() something went really wrong
                                *pcap_ts_format = PcapTs::Microsecs;
                            }
                            PCAP_MAGIC_NS | PCAP_MAGIC_NS_BE => {
                                let mut pcap_ts_format = pcap_ts.lock().unwrap();
                                *pcap_ts_format = PcapTs::Nanosecs;
                            }
                            _ => {}
                        },
                        PcapBlockOwned::NG(_) => {}
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
