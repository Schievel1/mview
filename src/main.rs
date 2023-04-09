use anyhow::Result;
use crossbeam::channel::bounded;
use mview::{args::Args, read, write};
use std::thread;

fn main() -> Result<()> {
    // get args
    let args = Args::parse();

    let (write_tx, write_rx) = bounded(1024);

    thread::scope(|s| {
        let read_handle = s.spawn(|| read::read_loop(&args, write_tx));
        let write_handle = s.spawn(|| write::write_loop(&args, write_rx));
        let read_io_result = read_handle.join().expect("Unable to join read thread");
        let write_io_result = write_handle.join().expect("Unable to join write thread");
        read_io_result.expect("Error during read thread");
        write_io_result.expect("Error during write thread");
    });
    Ok(())
}
