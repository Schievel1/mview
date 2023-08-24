use clap::{
    crate_authors, crate_description, crate_name, crate_version, Arg, ArgAction,
    Command,
};

pub fn get_styles() -> clap::builder::Styles {
    clap::builder::Styles::styled()
        .usage(
            anstyle::Style::new()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)))
                .bold(),
        )
        .header(
            anstyle::Style::new()
                .bold()
                .underline()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow))),
        )
        .literal(
            anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green))),
        )
}

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
    pub clear: bool,
    pub filter_newlines: bool,
}

impl Args {
    pub fn command() -> Command {
        Command::new(crate_name!())
            .styles(get_styles())
            .version(crate_version!())
            .author(crate_authors!("\n"))
            .about(crate_description!())
            .help_template(
                "{name} {version}
{author-with-newline}
{about-with-newline}
{usage-heading} {usage}

{all-args}{after-help}
            ",
            )
            .arg(Arg::new("infile")
                    .short('i')
                    .long("infile")
                    .help("Read from a file instead of stdin"),
            )
            .arg(
                Arg::new("outfile")
                    .short('w')
                    .long("outfile")
					.visible_short_alias('o')
                    .help("Write output to a file instead of stdout"),
            )
            .arg(
                Arg::new("config")
                    .short('c')
                    .long("config")

					.required(true)
                    .help("Definition of the datafields of a chunk"),
            )
            .arg(
                Arg::new("pcap")
                    .long("pcap")
                    .action(ArgAction::SetTrue)
                    .help("Read from a PCAP formatted file or data stream"),
            )
            .arg(
                Arg::new("chunksize (bytes)")
                    .short('s')
                    .long("chunksize")

                    .value_parser(clap::value_parser!(usize))
                    .help("Restart matching after n bytes")
				    .long_help("Restart matching after n bytes. If this argument \
                                is not given the size of a chunk is determined \
                                from the config. If chunksize is longer than a \
                                message, the chunk is filled with message data \
                                from the start until mview runs out of message \
                                data. Fields of the chunk that are left will \
                                not get a value. If chunksize is shorter than \
                                a message, the whole chunk will be filled with \
                                the message data, mview will print out that \
                                chunk and then start filling the next chunk \
                                with the remaining data from the message. \
                                For a graphical explaination of how mview \
                                handles message and chunk lengths see Readme."),
            )
            .arg(
                Arg::new("offset (bytes)")
                    .long("offset")
					.visible_alias("byteoffset")
                    .value_parser(clap::value_parser!(usize))
                    .help("offset in bytes at the start of a chunk before parsing starts"),
            )
            .arg(
                Arg::new("offset (bits)")
                    .long("bitoffset")
                    .value_parser(clap::value_parser!(usize))
                    .help("offset in bits at the start of a chunk before parsing starts (added to --offset <bytes>)"),
            )
            .arg(
                Arg::new("rawhex")
                    .short('r')
                    .long("rawhex")
					.visible_alias("raw")
                    .action(ArgAction::SetTrue)
                    .help("Print raw hexdump of the chunk at top of output"),
            )
            .arg(
                Arg::new("rawbin")
                    .long("rawbin")
                    .action(ArgAction::SetTrue)
                    .help("Print raw bindump of the chunk at top of output"),
            )
            .arg(
                Arg::new("rawascii")
                    .long("rawascii")
                    .action(ArgAction::SetTrue)
                    .help("Print raw ascii of the chunk at top of output"),
            )
            .arg(
                Arg::new("pause (ms)")
                    .long("pause")
                    .short('p')
                    .value_parser(clap::value_parser!(u64))
                    .help("Add a pause (in ms) between the output of chunks."),
            )
            .arg(
                Arg::new("little endian")
					.long("little-endian")
					.visible_aliases(["le", "littleendian"])
                    .action(ArgAction::SetTrue)
                    .help("Interpret integers as little endian (default is big endian)."),
            )
            .arg(
                Arg::new("timestamp")
                    .long("timestamp")
                    .short('t')
                    .action(ArgAction::SetTrue)
                    .help("Display timestamp of each chunk."),
            )
            .arg(
                Arg::new("head (bytes)")
                    .long("head")
                    .value_parser(clap::value_parser!(usize))
                    .help("Read only the first x bytes where x is the number given, print that as a message and then exit."),
            )
            .arg(
                Arg::new("print statistics")
                    .long("stats")
                    .action(ArgAction::SetTrue)
                    .help("Print statistics about messages received, message length and chunk number."),
            )
            .arg(
                Arg::new("print bitposition")
                    .long("bitpos")
                    .action(ArgAction::SetTrue)
                    .help("Print the current position inside a chunk. (For debugging purposes)"),
            )
            .arg(
                Arg::new("no cursor jumping")
                    .long("nojump")
                    .action(ArgAction::SetTrue)
                    .help("Print to stdout like printing to a file with option --outfile")
					.long_help("Print to stdout like printing to a file with \
                                option --outfile. Do not jump back the amount \
                                of lines printed before printing the next \
                                chunk when printing to stdout."),
            )
            .arg(
                Arg::new("clear")
                    .long("clear")
                    .action(ArgAction::SetTrue)
                    .help("Clear the terminal before each chunk is printed")
					.long_help("Clear the terminal before each chunk is \
                                printed. Counting lines then deleting them can \
                                be tricky and if it works depends on the \
                                terminal. Clearing the terminal almost \
                                always works."),
            )
            .arg(
                Arg::new("filter newlines")
                    .long("filter-newlines")
                    .action(ArgAction::SetTrue)
                    .help("Filter newline characters from string fields")
                    .long_help("Filter newline characters from string fields. \
                                Normally mview tries to not alter the data of \
                                a message and prints it 'as is'. However, this \
                                can result it a mess when strings in the \
                                message contain control characters like \\n. \
                                To avoid making a mess this argument lets \
                                mview filter the strings from newline \
                                characters"),
            )
    }
    pub fn parse() -> Self {
        let matches = Args::command().get_matches();

        let infile = matches.get_one::<String>("infile").cloned().unwrap_or_default();
        let outfile = matches.get_one::<String>("outfile").cloned().unwrap_or_default();
        let config = matches.get_one::<String>("config").cloned().unwrap_or_default();
        let pcap = matches.get_flag("pcap");
        let chunksize = matches
            .try_get_one::<usize>("chunksize (bytes)")
            .unwrap_or_default()
            .unwrap_or(&0);
        let offset = matches
            .try_get_one::<usize>("offset (bytes)")
            .unwrap_or_default()
            .unwrap_or(&0);
        let bitoffset = matches
            .try_get_one::<usize>("offset (bits)")
            .unwrap_or_default()
            .unwrap_or(&0);
        let rawhex = matches.get_flag("rawhex");
        let rawbin = matches.get_flag("rawbin");
        let rawascii = matches.get_flag("rawascii");
        let pause = matches
            .try_get_one::<u64>("pause (ms)")
            .unwrap_or_default()
            .unwrap_or(&0);
        let little_endian = matches.get_flag("little endian");
        let timestamp = matches.get_flag("timestamp");
        let read_head = matches
            .try_get_one::<usize>("head (bytes)")
            .unwrap_or_default()
            .unwrap_or(&0);
        let print_statistics = matches.get_flag("print statistics");
        let print_bitpos = matches.get_flag("print bitposition");
        let cursor_jump = !matches.get_flag("no cursor jumping");
        let clear = matches.get_flag("clear");
        let filter_newlines = matches.get_flag("filter newlines");
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
            clear,
            filter_newlines,
        }
    }
}
