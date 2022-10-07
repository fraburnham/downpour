use std::fs;

use downpour::decode;

fn main() {
    println!("{}", decode(&fs::read("debian-11.5.0-amd64-netinst.iso.torrent").unwrap()));
}
