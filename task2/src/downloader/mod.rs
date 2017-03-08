pub mod tracker;
pub mod connection;

use std::path::PathBuf;
use std::net::Ipv6Addr;
use torrent::Torrent;
use storage::Storage;
use self::tracker::{Tracker, TrackerArgs};
use self::connection::Connection;


const LISTEN_PORT: u16 = 6981;

#[derive(Clone)]
pub struct DownloaderId(pub [u8; 20]);

pub struct Downloader<S: Storage, T: Tracker> {
	storage: S,
	tracker: T,
	connections: Vec<Connection>,
	known_peers: Vec<(Ipv6Addr, u16)>,
	downloaded: u64,
	id: DownloaderId,
	info_hash: [u8; 20],
	port: u16,
}

impl<S: Storage, T: Tracker> Downloader<S, T> {
	pub fn new(path: PathBuf, info_hash: [u8; 20], torrent: Torrent) -> Downloader<S, T> {
		let id = generate_id();
		let port = LISTEN_PORT; // TODO: actually get a port and listen
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
		let before = self.connections.len();
		self.connections.retain(|con| !con.is_dead());
		let after = self.connections.len();
		let removed = before - after;
		if removed > 0 {
			println!("Removed {} dead connections", removed);
		}
	}

	fn open_new_connections(&mut self) {
		while self.connections.len() == 0 {
			match self.pick_peer() {
				Some((ip, port)) => {
					let con = Connection::new(
						self.id.clone(),
						self.info_hash.clone(),
						ip,
						port);
					self.connections.push(con);
				}
				None => {
					// no known peers, don't try to loop here
					break;
				}
			}
		}
	}

	fn pick_peer(&self) -> Option<(Ipv6Addr, u16)> {
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

fn generate_id() -> DownloaderId {
	let mut id: [u8; 20] = *b"-dj0001-????????????";
	for i in 8..20 {
		let digit: u8 = (::rand::random::<u64>() % 62) as u8;
		let ch = if digit < 10 {
			'0' as u8 + digit
		} else if digit < 10 + 26 {
			'a' as u8 + (digit - 10)
		} else {
			'A' as u8 + (digit - 10 - 26)
		};
		id[i] = ch;
	}
	DownloaderId(id)
}
