use clap::{App, Arg};

pub struct Args {
    pub infile: String,
    pub outfile: String,
    pub config: String,
    pub pcap: bool,
    pub chunksize: usize,
    pub offset: usize,
    pub bitoffset: usize,
    pub rawhex: bool,
    pub rawbin: bool,
    pub rawascii: bool,
    pub pause: u64,
    pub little_endian: bool,
    pub timestamp: bool,
    pub read_head: usize,
    pub print_statistics: bool,
    pub print_bitpos: bool,
    pub cursor_jump: bool,
    pub filter_newlines: bool,
}

impl Args {
    pub fn parse() -> Self {
        let matches = App::new("mview")
            .arg(Arg::with_name("infile")
                    .short('i')
                    .long("infile")
                    .takes_value(true)
                    .help("Read from a file instead of stdin"),
            )
            .arg(
                Arg::with_name("outfile")
                    .short('w')
                    .long("outfile")
                    .takes_value(true)
                    .help("Write output to a file instead of stdout"),
            )
            .arg(
                Arg::with_name("config")
                    .short('c')
                    .long("config")
                    .takes_value(true)
					.required(true)
                    .help("Definition of the datafields of a chunk"),
            )
            .arg(
                Arg::with_name("pcap")
                    .long("pcap")
                    .takes_value(false)
                    .help("Read from a PCAP formatted file or data stream"),
            )
            .arg(
                Arg::with_name("chunksize")
                    .short('b')
                    .long("chunksize")
                    .takes_value(true)
                    .value_parser(clap::value_parser!(usize))
                    .help("flush stdout and restart matching after n bytes"),
            )
            .arg(
                Arg::with_name("offset (bytes)")
                    .short('o')
                    .long("offset")
                    .takes_value(true)
                    .value_parser(clap::value_parser!(usize))
                    .help("offset in bytes at the start of a chunk before parsing starts"),
            )
            .arg(
                Arg::with_name("offset (bits)")
                    .short('s')
                    .long("bitoffset")
                    .takes_value(true)
                    .value_parser(clap::value_parser!(usize))
                    .help("offset in bits at the start of a chunk before parsing starts (added to --offset <bytes>)"),
            )
            .arg(
                Arg::with_name("rawhex")
                    .short('r')
                    .long("rawhex")
                    .takes_value(false)
                    .help("Print raw hexdump of the chunk at top of output"),
            )
            .arg(
                Arg::with_name("rawbin")
                    .long("rawbin")
                    .takes_value(false)
                    .help("Print raw bindump of the chunk at top of output"),
            )
            .arg(
                Arg::with_name("rawascii")
                    .long("rawascii")
                    .takes_value(false)
                    .help("Print raw ascii of the chunk at top of output"),
            )
            .arg(
                Arg::with_name("pause")
                    .long("pause")
                    .short('p')
                    .takes_value(true)
                    .value_parser(clap::value_parser!(u64))
                    .help("Add a pause (in ms) between the output of chunks."),
            )
            .arg(
                Arg::with_name("little endian")
                    .long("le")
                    .takes_value(false)
                    .help("Interpret integers as little endian (default is big endian)."),
            )
            .arg(
                Arg::with_name("timestamp")
                    .long("timestamp")
                    .short('t')
                    .takes_value(false)
                    .help("Display timestamp of each chunk."),
            )
            .arg(
                Arg::with_name("read head")
                    .short('h')
                    .long("head")
                    .takes_value(true)
                    .value_parser(clap::value_parser!(usize))
                    .help("Read only the first x bytes where is the number given and then exit."),
            )
            .arg(
                Arg::with_name("print statistics")
                    .long("stats")
                    .takes_value(false)
                    .help("Print statistics about messages received, message length and chunk number."),
            )
            .arg(
                Arg::with_name("print bitposition")
                    .long("bitpos")
                    .takes_value(false)
                    .help("Print the current position inside a chunk. (For debugging purposes)"),
            )
            .arg(
                Arg::with_name("no cursor jumping")
                    .long("--nojump")
                    .takes_value(false)
                    .help("Print to stdout like printing to a file with option --outfile"),
            )
            .arg(
                Arg::with_name("filter newlines")
                    .long("--filter-newlines")
                    .takes_value(false)
                    .help("Filter newline characters from string fields"),
            )
            .get_matches();
        let infile = matches.value_of("infile").unwrap_or_default().to_string();
        let outfile = matches.value_of("outfile").unwrap_or_default().to_string();
        let config = matches.value_of("config").unwrap_or_default().to_string();
        let pcap = matches.is_present("pcap");
        let chunksize = matches
            .try_get_one::<usize>("chunksize")
            .unwrap_or_default()
            .unwrap_or(&0);
        let offset = matches
            .try_get_one::<usize>("offset")
            .unwrap_or_default()
            .unwrap_or(&0);
        let bitoffset = matches
            .try_get_one::<usize>("bitoffset")
            .unwrap_or_default()
            .unwrap_or(&0);
        let rawhex = matches.is_present("rawhex");
        let rawbin = matches.is_present("rawbin");
        let rawascii = matches.is_present("rawascii");
        let pause = matches
            .try_get_one::<u64>("pause")
            .unwrap_or_default()
            .unwrap_or(&0);
        let little_endian = matches.is_present("little endian");
        let timestamp = matches.is_present("timestamp");
        let read_head = matches
            .try_get_one::<usize>("read head")
            .unwrap_or_default()
            .unwrap_or(&0);
        let print_statistics = matches.is_present("print statistics");
        let print_bitpos = matches.is_present("print bitposition");
        let cursor_jump = !matches.is_present("no cursor jumping");
        let filter_newlines = matches.is_present("filter newlines");
        Self {
            infile,
            outfile,
            config,
            pcap,
            chunksize: *chunksize,
            offset: *offset,
            bitoffset: *bitoffset,
            rawhex,
            rawbin,
            rawascii,
            pause: *pause,
            little_endian,
            timestamp,
            read_head: *read_head,
            print_statistics,
            print_bitpos,
            cursor_jump,
            filter_newlines,
        }
    }
}
