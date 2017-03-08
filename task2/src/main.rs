#![allow(dead_code)]

extern crate rand;
extern crate sha1;
extern crate hyper;

pub mod bencode;
pub mod torrent;
pub mod downloader;
pub mod storage;

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::env;

use torrent::Torrent;
use downloader::Downloader;
use downloader::tracker::HttpTracker;
use storage::memory::MemoryStorage;

fn main() {
    let args: Vec<String> = env::args().collect();
    let path = match args.into_iter().nth(1) {
        Some(arg) => arg,
        None => {
            println!("Usage: thing <torrent file>");
            return;
        }
    };

    println!("Torrent file: {}", path);
    
    let (torrent, info_hash) = read_torrent_file(path).unwrap();
    
    println!("Parsed file!");
    println!("Downloading: {:?}", torrent.info.root);
    
    let mut downloader: Downloader<MemoryStorage, HttpTracker> =
        Downloader::new(info_hash, torrent);

    downloader.run();
}

fn read_torrent_file<P: AsRef<Path>>(path: P) -> Option<(Torrent, [u8; 20])> {
    let mut file = File::open(path).expect("failed to open file");
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).expect("failed to read file");

    let bvalue = match bencode::decode(&contents) {
        Ok(x) => x,
        Err(e) => {
            println!("failed to parse file:\n  {:?}", e);
            return None;
        }
    };

    let (torrent, info_hash) = match torrent::from_bvalue(bvalue) {
        Ok(x) => x,
        Err(e) => {
            println!("failed to parse file:\n  {:?}", e);
            return None;
        }
    };

    Some((torrent, info_hash))
}