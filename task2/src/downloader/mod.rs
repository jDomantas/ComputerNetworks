use std::path::PathBuf;
use torrent::Torrent;
use storage::Storage;

pub struct Downloader<S: Storage> {
	tracker_url: String,
	storage: S,
	id: String,
	info_hash: [u8; 20],
	port: u16,
}

impl<S: Storage> Downloader<S> {
	pub fn new(path: PathBuf, info_hash: [u8; 20], torrent: Torrent) -> Downloader<S> {
		let storage = S::new(path, torrent.info);
		Downloader {
			tracker_url: torrent.tracker_url,
			storage: storage,
			id: generate_id(),
			port: 10, // TODO: actually get a port and listen
			info_hash: info_hash,
		}
	}

	pub fn run(&mut self) {
		println!("Running downloader to death!");
		let tracker_request = self.create_tracker_request();
		println!("tracker request:\n{}", tracker_request);
		unimplemented!()
	}

	fn create_tracker_request(&self) -> String {
		let mut url = self.tracker_url.clone();
		url.push_str("?info_hash=");
		for byte in &self.info_hash {
			url.push('%');
			url.push(nibble_to_char(byte >> 4));
			url.push(nibble_to_char(byte & 0xF));
		}
		url.push_str("&peer_id=");
		url.push_str(&self.id);
		url.push_str("&port=");
		url.push_str(&self.port.to_string());
		url.push_str("&uploaded=0&downloaded=0&left=");
		url.push_str(&self.storage.bytes_missing().to_string());
		url.push_str("&compact=1");
		url
	}
}

fn nibble_to_char(nibble: u8) -> char {
	if nibble <= 10 {
		('0' as u8 + nibble) as char
	} else {
		('A' as u8 + nibble - 10) as char
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
