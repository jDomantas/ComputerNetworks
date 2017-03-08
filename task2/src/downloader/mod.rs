pub mod tracker;
pub mod connection;

use std::net::Ipv6Addr;
use torrent::Torrent;
use storage::{Storage, Block};
use self::tracker::{Tracker, TrackerArgs};
use self::connection::Connection;
use self::connection::Message;


const LISTEN_PORT: u16 = 6981;
const REQUEST_SIZE: usize = 0x4000; // 16 kb

pub struct Request {
	piece: usize,
	offset: usize,
	length: usize,
}

impl Request {
	pub fn new(piece: usize, offset: usize, length: usize) -> Request {
		Request {
			piece: piece,
			offset: offset,
			length: length,
		}
	}

	fn split_request(&self, max_length: usize) -> RequestSplitIter {
		RequestSplitIter {
			piece: self.piece,
			max_length: max_length,
			start: self.offset,
			end: self.offset + self.length,
		}
	}
}

struct RequestSplitIter {
	piece: usize,
	max_length: usize,
	start: usize,
	end: usize,
}

impl Iterator for RequestSplitIter {
	type Item = Request;
	fn next(&mut self) -> Option<Request> {
		if self.start >= self.end {
			None
		} else {
			let start = self.start;
			let len = ::std::cmp::min(self.end - start, self.max_length);
			self.start += self.max_length;
			Some(Request::new(self.piece, start, len))
		}
	}
}

#[derive(Clone)]
pub struct DownloaderId(pub [u8; 20]);

pub struct Downloader<S: Storage, T: Tracker> {
	storage: S,
	tracker: T,
	connections: Vec<(PeerInfo, Connection)>,
	known_peers: Vec<(Ipv6Addr, u16)>,
	downloaded: usize,
	uploaded: usize,
	id: DownloaderId,
	info_hash: [u8; 20],
	port: u16,
	part_count: usize,
}

impl<S: Storage, T: Tracker> Downloader<S, T> {
	pub fn new(info_hash: [u8; 20], torrent: Torrent) -> Downloader<S, T> {
		let id = generate_id();
		let port = LISTEN_PORT; // TODO: actually listen
		let piece_count = torrent.info.pieces.len();
		let storage = S::new(torrent.info);
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
			uploaded: 0,
			id: id,
			port: port,
			info_hash: info_hash,
			part_count: piece_count,
		}
	}

	pub fn run(&mut self) {
		println!("Running downloader");
		while self.storage.bytes_missing() > 0 {
			self.update_tracker();
			self.remove_dead_connections();
			self.check_for_new_peers();
			self.open_new_connections();
			self.process_messages();
			self.request_pieces();
			let sleep = ::std::time::Duration::from_millis(500);
			::std::thread::sleep(sleep);
		}
		println!("Download complete");
	}

	fn process_messages(&mut self) {
		for &mut (ref mut peer, ref mut con) in &mut self.connections {
			while let Some(msg) = con.receive() {
				match msg {
					Message::Dead => {
						break;
					}
					Message::Bitfield(bits) => {
						peer.got_parts(bits);
					}
					Message::Have(index) => {
						peer.got_part(index);
					}
					Message::Cancel(_, _, _) => {
						// because we only send pieces that were
						// available at the time of request,
						// cancel requests will be ignored
					}
					Message::Request(part, offset, length) => {
						match self.storage.get_piece(part as usize) {
							Some(piece) => {
								let off = offset as usize;
								let len = length as usize;
								if off + len > piece.len() {
									// client sent bad request
									// TODO: disconnect him
								} else {
									let data = &piece[off..(off + len)];
									con.send(Message::Piece(
										part,
										offset,
										data.to_vec()));
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
							Ok(()) => { }
							Err(_) => {
								// peer sent bad block
								// TODO: disconnect him
							}
						}
					}
				}
			}
		}
	}

	fn request_pieces(&mut self) {
		let requests = self.storage
			.requests()
			.flat_map(|r| r.split_request(REQUEST_SIZE))
			.take(20) // TODO: figure out how many
			.collect::<Vec<_>>();
		for r in requests {
			// TODO: actually request pieces
			println!("{} {} {}", r.piece, r.offset, r.length);
		}
	}

	fn update_tracker(&mut self) {
		let down = self.downloaded;
		let up = self.uploaded;
		let left = self.storage.bytes_missing();
		self.tracker.update_tracker(down, up, left);
	}

	fn remove_dead_connections(&mut self) {
		let before = self.connections.len();
		self.connections.retain(|&(_, ref con)| !con.is_dead());
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
					let info = PeerInfo::new(self.part_count);
					self.connections.push((info, con));
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

struct PeerInfo {
	part_mask: Vec<u8>,
	part_count: usize,
	has_parts: usize,
}

impl PeerInfo {
	fn new(part_count: usize) -> PeerInfo {
		let bytes = (part_count + 7) / 8;
		PeerInfo {
			part_mask: vec![0; bytes],
			part_count: part_count,
			has_parts: 0,
		}
	}

	fn got_part(&mut self, index: u32) {
		let index2 = index as usize;
		if index2 >= self.part_count {
			println!("Peer announced about bad part: {}", index2);
		} else if !self.does_have_part(index) {
			let byte = index2 / 8;
			// high bit in byte #0 corresponds to piece #0
			let bit = 7 - index2 % 8;
			self.part_mask[byte] |= 1 << bit;
			self.has_parts += 1;
			println!("Peer has {}/{} parts", self.has_parts, self.part_count);
		}
	}

	fn does_have_part(&mut self, index: u32) -> bool {
		let index = index as usize;
		if index >= self.part_count {
			false
		} else {
			let byte = index / 8;
			// high bit in byte #0 corresponds to piece #0
			let bit = 7 - index % 8;
			(self.part_mask[byte] & !(1 << bit)) != 0
		}
	}

	fn got_parts(&mut self, bitfield: Vec<u8>) {
		if bitfield.len() != self.part_mask.len() {
			println!("Peer sent bitfield of bad size: {}", bitfield.len());
		} else {
			let empty_bits = self.part_count % 8;
			let empty_bit_mask = ((1_u16 << empty_bits) - 1) as u8;
			let unused_bits = bitfield[bitfield.len() - 1] & empty_bit_mask;
			if unused_bits != 0 {
				println!("Peer sent bad bitfield - some of spare bits are set");
			} else {
				self.part_mask = bitfield;
			}

			let mut has = 0_usize;
			for i in 0..(self.part_count) {
				if self.does_have_part(i as u32) {
					has += 1;
				}
			}
			self.has_parts = has;
			println!("Peer has {}/{} parts", self.has_parts, self.part_count);
		}
	}
}
