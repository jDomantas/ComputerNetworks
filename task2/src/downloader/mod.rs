pub mod tracker;

use std::path::PathBuf;
use torrent::Torrent;
use storage::Storage;
use downloader::tracker::{Tracker, TrackerArgs};


pub struct Downloader<S: Storage, T: Tracker> {
	storage: S,
	tracker: T,
	id: String,
	info_hash: [u8; 20],
	port: u16,
}

impl<S: Storage, T: Tracker> Downloader<S, T> {
	pub fn new(path: PathBuf, info_hash: [u8; 20], torrent: Torrent) -> Downloader<S, T> {
		let id = generate_id();
		let port = 10; // TODO: actually get a port and listen
		let storage = S::new(path, torrent.info);
		let tracker = T::new(TrackerArgs {
			tracker_url: torrent.tracker_url,
			info_hash: info_hash.clone(),
			id: id.clone(),
			port: port,
		});
		Downloader {
			storage: storage,
			tracker: tracker,
			id: id,
			port: port,
			info_hash: info_hash,
		}
	}

	pub fn run(&mut self) {
		println!("Running downloader to death!");
		unimplemented!()
	}
}

fn generate_id() -> String {
	let mut id = "-DJ0001-".to_string();
	while id.len() < 20 {
		let digit: u8 = (::rand::random::<u64>() % 10) as u8;
		let ch = '0' as u8 + digit;
		id.push(ch as char);
	}
	id
}
