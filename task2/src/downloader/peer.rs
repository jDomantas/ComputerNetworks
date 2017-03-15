use downloader::PeerAddress;
use downloader::connection;
use downloader::connection::{Connection, InMessage, HandshakeInfo};


pub enum Message {
	Request(usize, usize, usize),
	Piece(usize, usize, Vec<u8>),
}

pub struct Peer {
	connection: Box<Connection>,
	info: HandshakeInfo,
	peer: PeerAddress,
	peer_info: Option<HandshakeInfo>,
	piece_count: usize,
	have: Vec<u8>,
	self_choked: bool,
	self_interested: bool,
	peer_choked: bool,
	peer_interested: bool,
}

impl Peer {
	pub fn new(
			connection: Box<Connection>,
			peer: PeerAddress,
			piece_count: usize,
			info: HandshakeInfo) -> Peer {
		let bitfield_bytes = (piece_count + 7) / 8;
		Peer {
			connection: connection,
			info: info,
			peer: peer,
			peer_info: None,
			piece_count: piece_count,
			have: vec![0; bitfield_bytes],
			self_choked: true,
			peer_choked: true,
			self_interested: false,
			peer_interested: false,
		}
	}

	pub fn send(&mut self, msg: Message) {
		let msg = match msg {
			Message::Request(piece, off, len) =>
				connection::Message::Request(piece, off, len),
			Message::Piece(piece, off, data) =>
				connection::Message::Piece(piece, off, data),
		};
		self.connection.send(msg);
	}

	pub fn receive(&mut self) -> Option<Message> {
		loop {
			let incoming = match self.connection.receive() {
				Some(msg) => msg,
				None => return None,
			};
			match incoming {
				InMessage::Error(_) => {
					return None;
				}
				InMessage::Handshake(peer) => {
					if peer.info_hash != self.info.info_hash {
						debug!("Peer {:?} offered wrong torrent, disconnecting", self.peer);
						self.connection.close();
					} else if peer.id == self.info.id {
						debug!("Peer {:?} is me, disconnecting", self.peer);
						self.connection.close();
					} else {
						self.peer_info = Some(peer);
					}
				}
				InMessage::Normal(msg) => {
					let m = self.process_message(msg);
					if m.is_some() {
						return m;
					}
				}
			}
		}
	}

	pub fn is_alive(&self) -> bool {
		self.connection.is_alive()
	}

	pub fn disconnect(&mut self) {
		self.connection.close();
	}

	pub fn does_have(&self, piece: usize) -> bool {
		if piece > self.piece_count {
			false
		} else {
			let byte = piece / 8;
			let bit = 7 - piece % 8;
			self.have[byte] & (1 << bit) != 0
		}
	}

	fn store_bitfield(&mut self, bitfield: Vec<u8>) {
		if bitfield.len() != self.have.len() {
			debug!("Peer {:?} sent bad bitfield, length: {}, expected: {}",
				self.peer,
				bitfield.len(),
				self.have.len());
			self.connection.close();
			return;
		}

		let spare_bits = (8 - self.piece_count % 8) % 8;
		let last_byte = bitfield[bitfield.len() - 1];
		for i in 0..spare_bits {
			if last_byte & (1 << i) != 0 {
				debug!("Peer {:?} sent bad bitfield - some of spare bits are set", self.peer);
				debug!("Last byte: {}", last_byte);
				debug!("Piece count: {}", self.piece_count);
				self.connection.close();
				return;
			}
		}

		self.have = bitfield;
	}

	fn process_message(&mut self, msg: connection::Message) -> Option<Message> {
		match msg {
			connection::Message::Choke =>
				self.peer_choked = true,
			connection::Message::Unchoke =>
				self.peer_choked = false,
			connection::Message::Interested =>
				self.peer_interested = true,
			connection::Message::NotInterested =>
				self.peer_interested = false,
			connection::Message::Have(piece) => {
				if piece >= self.piece_count {
					debug!("Peer {:?} announced bad piece: {:?}, disconnecting",
						self.peer,
						piece);
					self.connection.close();
				} else {
					let byte = piece / 8;
					let bit = 7 - piece % 8;
					self.have[byte] |= 1 << bit;
				}
			}
			connection::Message::Bitfield(bits) => {
				self.store_bitfield(bits)
			}
			connection::Message::Request(piece, off, len) =>
				return Some(Message::Request(piece, off, len)),
			connection::Message::Piece(piece, off, data) =>
				return Some(Message::Piece(piece, off, data)),
			connection::Message::Cancel(_, _, _) => {
				// maybe some day this client will be
				// smart enough to make use of this.
			}
		}

		None
	}

	pub fn am_choked(&self) -> bool {
		self.self_choked
	}

	pub fn am_interested(&self) -> bool {
		self.self_interested
	}

	pub fn interested(&self) -> bool {
		self.peer_interested
	}
	
	pub fn choked(&self) -> bool {
		self.peer_choked
	}

	pub fn set_choking(&mut self, choked: bool) {
		if self.self_choked != choked {
			self.self_choked = choked;
			if choked {
				self.connection.send(connection::Message::Choke);
			} else {
				self.connection.send(connection::Message::Unchoke);
			}
		}
	}

	pub fn set_interested(&mut self, interested: bool) {
		if self.self_interested != interested {
			self.self_interested = interested;
			if interested {
				self.connection.send(connection::Message::Interested);
			} else {
				self.connection.send(connection::Message::NotInterested);
			}
		}
	}
}