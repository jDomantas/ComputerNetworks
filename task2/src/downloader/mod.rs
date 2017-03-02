pub mod tracker;
pub mod connection;

use std::path::PathBuf;
use torrent::Torrent;
use storage::Storage;
use self::tracker::{Tracker, TrackerArgs};
use self::connection::Connection;


pub struct Downloader<S: Storage, T: Tracker> {
	storage: S,
	tracker: T,
	connections: Vec<Connection>,
	known_peers: Vec<(u32, u16)>,
	downloaded: u64,
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
			connections: Vec::new(),
			known_peers: Vec::new(),
			downloaded: 0,
			id: id,
			port: port,
			info_hash: info_hash,
		}
	}

	pub fn run(&mut self) {
		println!("Running downloader");
		while self.storage.bytes_missing() > 0 {
			self.update_tracker();
			self.remove_dead_connections();
			self.check_for_new_peers();
			self.open_new_connections();
			// TODO: process messages
			let sleep = ::std::time::Duration::from_secs(5);
			::std::thread::sleep(sleep);
		}
		println!("Download complete");
	}

	fn update_tracker(&mut self) {
		let down = self.downloaded;
		let up = 0; // :(
		let left = self.storage.bytes_missing();
		self.tracker.update_tracker(down, up, left);
	}

	fn remove_dead_connections(&mut self) {
		self.connections.retain(|con| !con.is_dead());
	}

	fn open_new_connections(&mut self) {
		while self.connections.len() == 0 {
			match self.pick_peer() {
				Some((ip, port)) => {
					let con = Connection::new(ip, port);
					self.connections.push(con);
				}
				None => {
					// no known peers, don't try to loop here
					break;
				}
			}
		}
	}

	fn pick_peer(&self) -> Option<(u32, u16)> {
		if self.known_peers.len() == 0 {
			None
		} else {
			let index = ::rand::random::<usize>() % self.known_peers.len();
			Some(self.known_peers[index])
		}
	}

	fn check_for_new_peers(&mut self) {
		match self.tracker.latest_response() {
			Some(response) => {
				self.known_peers = response.peers;
			}
			None => { }
		}
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
