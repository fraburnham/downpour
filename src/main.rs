use clap::{Arg, ArgMatches, Command, arg};
use downpour::decode;
use std::fs;
use std::io::{Read, Stdin};

fn cmd() -> Command {
    Command::new("downpour")
	.about("A tool for interacting with BitTorrent trackers, DHT and peers")
	.subcommand_required(true)
	.subcommand(
	    Command::new("torrent")
		.about("Get details from torrent files")
		.arg_required_else_help(true)
		.arg(
		    arg!(<FILE>)
			.help("The file to parse")
		),
	)
	.subcommand(
	    Command::new("bencoding")
		.about("Work with bencoded data from stdin")
		.args([
		    Arg::new("decode")
			.help("Decode bencoded data to json"),
		    Arg::new("encode")
			.help("TODO! Encode json to bencoding"),
		    ]),
	)
}

fn torrent(args: &ArgMatches) {
    let file_path = args.get_one::<String>("FILE").expect("required");
    println!("{}", decode(&fs::read(file_path).unwrap()));
}

fn bencoding(_args: &ArgMatches) {
    let mut buf: Vec<u8> = Vec::new();
    let mut stdin: Stdin = std::io::stdin();

    match stdin.read_to_end(&mut buf) {
	Ok(_) => {
	    println!("{}", decode(&buf));
	},

	_ => println!("Failed to read STDIN!"),
    }
    
}

fn main() {
    let args = cmd().get_matches();

    match args.subcommand() {
	Some(("torrent", args)) => torrent(args),
	Some(("bencoding", args)) => bencoding(args),
	// todo tracker stuff to make those requests easier!
	_ => unreachable!(),
    }
}
