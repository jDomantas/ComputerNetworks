#![allow(dead_code)]

extern crate rand;
extern crate sha1;
extern crate hyper;
#[macro_use]
extern crate log;

pub mod bencode;
pub mod torrent;
pub mod downloader;
pub mod storage;

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::env;
use log::{LogRecord, LogLevel, LogMetadata, SetLoggerError};

use torrent::Torrent;
use downloader::Downloader;
use storage::memory::MemoryStorage;
use storage::partial::PartialStorage;

fn main() {
    Logger::init().expect("Failed to initialize logger");

    let args: Vec<String> = env::args().collect();
    let path = match args.into_iter().nth(1) {
        Some(arg) => arg,
        None => {
            println!("Usage: thing <torrent file>");
            return;
        }
    };

    println!("Torrent file: {}", path);
    
    let (torrent, info_hash) = read_torrent_file(path.clone()).unwrap();
    
    println!("Parsed file!");
    println!("Downloading: {:?}", torrent.info.root);
    
    let mut downloader: Downloader<PartialStorage<MemoryStorage>> =
        Downloader::new(info_hash, torrent.clone());

    downloader.run();

    println!("splitting");
    split_to_files("./test.out", torrent);
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

fn split_to_files<P: AsRef<Path>>(source: P, torrent: Torrent) {
    let mut source = File::open(source).expect("failed to open source file");
    use std::io::prelude::*;
    let mut data = Vec::new();
    source.read_to_end(&mut data).expect("failed to read source");
    let mut start = 0_usize;
    for file in torrent.info.files.into_iter() {
        let mut dest = File::create(file.path.clone()).expect("failed to create file");
        let end = start + file.length as usize;
        println!("interval {} - {} goes to {:?}", start, end, file.path.clone());
        dest.write_all(&data[start..end]).expect("failed to write");
        start = end;
    }
    println!("wrote total {} bytes", start);
}

const LOGGING_LEVEL: LogLevel = LogLevel::Debug;
struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        metadata.level() <= LOGGING_LEVEL
    }

    fn log(&self, record: &LogRecord) {
        if self.enabled(record.metadata()) {
            println!("{} - {}", record.level(), record.args());
        }
    }
}

impl Logger {
    fn init() -> Result<(), SetLoggerError> {
        log::set_logger(|max_log_level| {
            max_log_level.set(LOGGING_LEVEL.to_log_level_filter());
            Box::new(Logger)
        })
    }
}