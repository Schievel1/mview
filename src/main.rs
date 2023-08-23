use anyhow::Result;
use crossbeam::channel::bounded;
use mview::{args::Args, read, write, PcapTs};
use std::sync::{Arc, Mutex};
use std::thread;

fn main() -> Result<()> {
    // get args
    let args = Args::parse();
    let (write_tx, write_rx) = bounded(1024);

    // the mutex for wether the timestamp in the PCAP file is in
    // nanoseconds or microseconds
    let pcap_ts = Arc::new(Mutex::new(PcapTs::Microsecs));

    thread::scope(|s| {
        let pcap_ts_read = Arc::clone(&pcap_ts);
        let pcap_ts_write = Arc::clone(&pcap_ts);
        let read_handle = s.spawn(|| read::read_loop(&args, write_tx, pcap_ts_read));
        let write_handle = s.spawn(|| write::write_loop(&args, write_rx, pcap_ts_write));
        let read_io_result = read_handle.join().expect("Unable to join read thread");
        let write_io_result = write_handle.join().expect("Unable to join write thread");
        read_io_result.expect("Error during read thread");
        write_io_result.expect("Error during write thread");
    });
    Ok(())
}
