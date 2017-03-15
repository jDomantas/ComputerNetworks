pub mod tracker;
pub mod connection;
pub mod request;
pub mod peer;

use std::io;
use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr, ToSocketAddrs};
use std::time::{Duration, Instant};
use torrent::Torrent;
use storage::{Storage, Block};
use downloader::tracker::{Tracker, TrackerArgs};
use downloader::connection::HandshakeInfo;
use downloader::peer::{Peer, Message};


const LISTEN_PORT: u16 = 6981;
const REQUEST_SIZE: usize = 0x4000; // 16 kb

#[derive(Clone)]
pub struct PeerAddress {
	pub ip: Ipv6Addr,
	pub port: u16,
}

impl fmt::Debug for PeerAddress {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
		write!(fmt, "{:?}:{}", self.ip, self.port)
	}
}

impl PeerAddress {
	pub fn new(ip: Ipv6Addr, port: u16) -> PeerAddress {
		PeerAddress {
			ip: ip,
			port: port,
		}
	}
}

impl ToSocketAddrs for PeerAddress {
	type Iter = <(Ipv6Addr, u16) as ToSocketAddrs>::Iter;
	fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
		let ip = self.ip.to_ipv4().expect("failed to convert ipv6 address to ipv4");
		<(Ipv4Addr, u16) as ToSocketAddrs>::to_socket_addrs(&(ip, self.port))
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloaderId(pub [u8; 20]);

pub struct Downloader<S: Storage> {
	storage: S,
	tracker: Box<Tracker>,
	peers: Vec<Peer>,
	downloaded: usize,
	uploaded: usize,
	info: HandshakeInfo,
	listen_port: u16,
	piece_count: usize,
	last_request_time: Instant,
}

impl<S: Storage> Downloader<S> {
	pub fn new(info_hash: [u8; 20], torrent: Torrent) -> Downloader<S> {
		let info = HandshakeInfo::new(info_hash, generate_id());
		let piece_count = torrent.info.pieces.len();
		let storage = S::new(torrent.info);
		let tracker = tracker::create_tracker(TrackerArgs {
			tracker_url: torrent.tracker_url,
			id: info.id.clone(),
			info_hash: info.info_hash.clone(),
			port: LISTEN_PORT,
		});
		Downloader {
			storage: storage,
			tracker: tracker,
			peers: Vec::new(),
			downloaded: 0,
			uploaded: 0,
			info: info,
			listen_port: LISTEN_PORT, // TODO: actually listen
			piece_count: piece_count,
			last_request_time: Instant::now(),
		}
	}

	pub fn run(&mut self) {
		info!("Running downloader");
		while !self.storage.is_complete() {
			self.update_tracker();
			self.remove_dead_connections();
			self.open_new_connections();
			self.process_messages();
			self.request_pieces();
			::std::thread::sleep(Duration::from_millis(500));
		}
		info!("Download complete");
	}

	fn process_messages(&mut self) {
		// TODO: too much nesting, refactor
		for peer in &mut self.peers {
			while let Some(msg) = peer.receive() {
				match msg {
					Message::Request(part, offset, length) => {
						match self.storage.get_piece(part as usize) {
							Some(piece) => {
								let off = offset as usize;
								let len = length as usize;
								if off + len > piece.len() || len > REQUEST_SIZE {
									// client sent bad request
									peer.disconnect();
								} else {
									let data = &piece[off..(off + len)];
									peer.send(Message::Piece(
										part,
										offset,
										data.to_vec()));
									self.uploaded += len;
								}
							}
							None => {
								// we don't have the piece :(
							}
						}
					}
					Message::Piece(part, offset, payload) => {
						let block = Block::new(part as usize, offset as usize, payload);
						match self.storage.store_block(block) {
							Ok(new_bytes) => {
								self.downloaded += new_bytes;
							}
							Err(_) => {
								// peer sent bad block
								peer.disconnect();
							}
						}
					}
				}
			}
		}
	}

	fn request_pieces(&mut self) {
		let now = Instant::now();
		let passed = now - self.last_request_time;
		if passed < Duration::from_secs(5) {
			return;
		}
		self.last_request_time = Instant::now();

		let requests = self.storage
			.requests()
			// because we only store prefixes of pieces,
			// take at most one request from each piece
			.filter_map(|r| r.split_request(REQUEST_SIZE).next())
			// TODO: figure out how many
			.take(40)
			.collect::<Vec<_>>();

		for r in requests {
			if let Some(peer) = self.pick_peer_for_request(r.piece) {
				peer.send(Message::Request(r.piece, r.offset, r.length));
			}
		}
	}

	fn pick_peer_for_request(&mut self, piece: usize) -> Option<&mut Peer> {
		if self.peers.len() == 0 {
			return None;
		}
		let range = (0..(self.peers.len())).into_iter().cycle();
		let start_with = ::rand::random::<usize>() % self.peers.len();
		let range = range.skip(start_with).take(self.peers.len());
		for i in range {
			if self.peers[i].does_have(piece) {
				return Some(&mut self.peers[i]);
			}
		}
		None
	}

	fn update_tracker(&mut self) {
		let down = self.downloaded;
		let up = self.uploaded;
		let left = self.storage.bytes_missing();
		self.tracker.update(down, up, left);
	}

	fn remove_dead_connections(&mut self) {
		self.peers.retain(|ref peer| peer.is_alive());
	}

	fn open_new_connections(&mut self) {
		while self.peers.len() < 8 {
			match self.pick_peer() {
				Some(address) => {
					let connection = connection::bt::BtConnection::new(self.info.clone(), address.clone());
					let mut peer = Peer::new(
						Box::new(connection),
						address,
						self.piece_count,
						self.info.clone());
					// TODO: properly maintain and change state
					// this is for debugging only
					peer.set_choking(false);
					peer.set_interested(true);
					self.peers.push(peer);
				}
				None => {
					// no known peers, don't try to loop here
					break;
				}
			}
		}
	}

	fn pick_peer(&self) -> Option<PeerAddress> {
		let count = self.tracker.peers().count();
		if count == 0 {
			None
		} else {
			let index = ::rand::random::<usize>() % count;
			self.tracker.peers().nth(index).cloned()
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
