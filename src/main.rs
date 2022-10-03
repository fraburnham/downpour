use std::fs;

use downpour::parse;

fn main() {
    println!("{:?}", parse(&fs::read("debian-11.5.0-amd64-netinst.iso.torrent").unwrap()));
}
