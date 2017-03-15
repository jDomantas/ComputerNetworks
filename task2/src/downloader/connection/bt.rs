use std::io;
use std::io::{Read, Write, ErrorKind};
use std::net::TcpStream;
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;
use downloader::{DownloaderId, PeerAddress};
use downloader::connection::*;


pub struct BtConnection {
	sender: Sender<OutMessage>,
	receiver: Receiver<InMessage>,
	thread: thread::JoinHandle<()>,
	alive: bool,
}

impl BtConnection {
	pub fn new(handshake: HandshakeInfo, peer: PeerAddress) -> BtConnection {
		let (send1, recv1) = mpsc::channel();
		let (send2, recv2) = mpsc::channel();
		let send = send1.clone();
		let thread = thread::spawn(move || {
			let result = Internal::new(handshake, peer.clone(), send1, recv2)
				.map_err(Error::IoError)
				.and_then(|mut con| con.run());
			match result {
				Ok(()) => {
					debug!("Connection to {:?} closed.", peer);
				}
				Err(e) => {
					debug!("Connection to {:?} closed: {:?}", peer, e);
					// also push that error to channel
					let _ = send.send(InMessage::Error(e));
				}
			}
		});

		BtConnection {
			sender: send2,
			receiver: recv1,
			thread: thread,
			alive: true,
		}
	}
}

impl Connection for BtConnection {
	fn send(&mut self, msg: Message) {
		// we don't care if the message is not sent
		let _ = self.sender.send(OutMessage::Normal(msg));
	}

	fn receive(&mut self) -> Option<InMessage> {
		match self.receiver.try_recv() {
			Ok(e @ InMessage::Error(_)) => {
				self.alive = false;
				Some(e)
			}
			Ok(msg) => {
				Some(msg)
			}
			Err(TryRecvError::Empty) => {
				None
			}
			Err(TryRecvError::Disconnected) => {
				Some(InMessage::Error(Error::Closed))
			}
		}
	}

	fn close(&mut self) {
		let _ = self.sender.send(OutMessage::Close);
		self.alive = false;
		// do I really need to join the thread, or can I just let it run loose?
		// does not work right now anyways - self is only borrowed
		// let _ = self.thread.join();
	}

	fn is_alive(&self) -> bool {
		self.alive
	}
}

enum OutMessage {
	Normal(Message),
	Close,
}

enum RawMessage {
	KeepAlive,
	Choke,
	Unchoke,
	Interested,
	NotInterested,
	Have(usize),
	Bitfield(Vec<u8>),
	Request(usize, usize, usize),
	Piece(usize, usize, Vec<u8>),
	Cancel(usize, usize, usize),
}

impl RawMessage {
	fn from_message(msg: Message) -> RawMessage {
		match msg {
			Message::Choke => RawMessage::Choke,
			Message::Unchoke => RawMessage::Unchoke,
			Message::Interested => RawMessage::Interested,
			Message::NotInterested => RawMessage::NotInterested,
			Message::Have(piece) => RawMessage::Have(piece),
			Message::Bitfield(bits) => RawMessage::Bitfield(bits),
			Message::Request(piece, off, len) => RawMessage::Request(piece, off, len),
			Message::Piece(piece, off, data) => RawMessage::Piece(piece, off, data),
			Message::Cancel(piece, off, len) => RawMessage::Cancel(piece, off, len),
		}
	}
}

struct Internal {
	sender: Sender<InMessage>,
	receiver: Receiver<OutMessage>,
	handshake: HandshakeInfo,
	stream: TcpStream,
	recv_buffer: Vec<u8>,
	peer: String,
}

impl Internal {
	fn new(
			handshake: HandshakeInfo,
			peer: PeerAddress,
			send: Sender<InMessage>,
			recv: Receiver<OutMessage>) -> Result<Internal, io::Error> {
		debug!("Connecting to peer: {:?}", peer);
		let socket = try!(TcpStream::connect(peer.clone()));
		debug!("Connected to peer: {:?}", peer);
		Ok(Internal {
			sender: send,
			receiver: recv,
			handshake: handshake,
			stream: socket,
			recv_buffer: Vec::new(),
			peer: format!("{:?}", peer),
		})
	}

	fn send(&mut self, msg: InMessage) {
		// we don't care if the message is not sent
		let _ = self.sender.send(msg);
	}

	fn receive(&mut self) -> Option<OutMessage> {
		match self.receiver.try_recv() {
			Ok(msg) => Some(msg),
			Err(TryRecvError::Empty) => None,
			Err(TryRecvError::Disconnected) => Some(OutMessage::Close),
		}
	}

	fn next_message_length(&self) -> Option<u32> {
		if self.recv_buffer.len() >= 4 {
			Some(u32_from_bytes(&self.recv_buffer[0..4]))
		} else {
			None
		}
	}
	
	fn decode_raw_message(&self, slice: &[u8]) -> Result<RawMessage, Error> {
		if slice.len() == 0 {
			return Ok(RawMessage::KeepAlive);
		}
		match slice[0] {
			0 => {
				if slice.len() == 1 { 
					Ok(RawMessage::Choke)
				} else {
					Err(Error::BadMessage)
				}
			}
			1 => {
				if slice.len() == 1 { 
					Ok(RawMessage::Unchoke)
				} else {
					Err(Error::BadMessage)
				}
			}
			2 => {
				if slice.len() == 1 { 
					Ok(RawMessage::Interested)
				} else {
					Err(Error::BadMessage)
				}
			}
			3 => {
				if slice.len() == 1 { 
					Ok(RawMessage::NotInterested)
				} else {
					Err(Error::BadMessage)
				}
			}
			4 => {
				if slice.len() == 5 {
					Ok(RawMessage::Have(usize_from_bytes(&slice[1..5])))
				} else {
					Err(Error::BadMessage)
				}
			}
			5 => {
				Ok(RawMessage::Bitfield(slice[1..].to_vec()))
			}
			6 => {
				if slice.len() == 13 {
					let piece = usize_from_bytes(&slice[1..5]);
					let offset = usize_from_bytes(&slice[5..9]);
					let length = usize_from_bytes(&slice[9..13]);
					Ok(RawMessage::Request(piece, offset, length))
				} else {
					Err(Error::BadMessage)
				}
			}
			7 => {
				if slice.len() >= 9 {
					let piece = usize_from_bytes(&slice[1..5]);
					let offset = usize_from_bytes(&slice[5..9]);
					let data = slice[9..].to_vec();
					Ok(RawMessage::Piece(piece, offset, data))
				} else {
					Err(Error::BadMessage)
				}
			}
			8 => {
				if slice.len() == 13 {
					let piece = usize_from_bytes(&slice[1..5]);
					let offset = usize_from_bytes(&slice[5..9]);
					let length = usize_from_bytes(&slice[9..13]);
					Ok(RawMessage::Cancel(piece, offset, length))
				} else {
					Err(Error::BadMessage)
				}
			}
			x => {
				debug!("Received bad message from {}: type is {}", self.peer, x);
				Err(Error::BadMessage)
			}
		}
	}

	fn receive_bytes(&mut self) -> Result<(), Error> {
		match self.stream.read_to_end(&mut self.recv_buffer) {
			Ok(_) => {
				// connection was closed
				Err(Error::Closed)
			}
			Err(ref e) if
				e.kind() == ErrorKind::TimedOut ||
				e.kind() == ErrorKind::WouldBlock => {
				// read timed out, everything is fine
				Ok(())
			}
			Err(e) => {
				debug!("Error while reading from {}: {:?}", self.peer, e);
				Err(Error::IoError(e))
			}
		}
	}

	fn remove_bytes(&mut self, count: usize) {
		// drain leading bytes, after dropping iterator vector
		// will push following bytes to front 
		for _ in self.recv_buffer.drain(0..count) { }
	}

	fn get_raw_message(&mut self) -> Result<Option<RawMessage>, Error> {
		let len = match self.next_message_length() {
			Some(len) if len < (1 << 20) => {
				len as usize
			}
			Some(len) => {
				debug!("Peer {} send too long message: {}", self.peer, len);
				return Err(Error::BadMessage);
			}
			None => {
				return Ok(None);
			}
		};

		if self.recv_buffer.len() >= len + 4 {
			let message = {
				let message_data = &self.recv_buffer[4..(4 + len)];
				try!(self.decode_raw_message(message_data))
			};
			// remove message that was just decoded
			self.remove_bytes(len + 4);
			Ok(Some(message))
		} else {
			Ok(None)
		}
	}

	fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
		match self.stream.write_all(bytes) {
			Ok(_) => Ok(()),
			Err(e) => Err(Error::IoError(e)),
		}
	}

	fn write_message(&mut self, msg: RawMessage) -> Result<(), Error> {
		let write_result = match msg {
			RawMessage::KeepAlive => {
				self.write_bytes(&[0, 0, 0, 0])
			}
			RawMessage::Choke => {
				self.write_bytes(&[0, 0, 0, 1, 0])
			}
			RawMessage::Unchoke => {
				self.write_bytes(&[0, 0, 0, 1, 1])
			}
			RawMessage::Interested => {
				self.write_bytes(&[0, 0, 0, 1, 2])
			}
			RawMessage::NotInterested => {
				self.write_bytes(&[0, 0, 0, 1, 3])
			}
			RawMessage::Have(index) => {
				let index = bytes_from_usize(index);
				self.write_bytes(&bytes_from_u32(5))
					.and_then(|_| self.write_bytes(&[4]))
					.and_then(|_| self.write_bytes(&index))
			}
			RawMessage::Bitfield(bits) => {
				let len = bits.len() + 1;
				self.write_bytes(&bytes_from_usize(len))
					.and_then(|_| self.write_bytes(&[5]))
					.and_then(|_| self.write_bytes(&bits))
			}
			RawMessage::Request(piece, offset, len) => {
				let piece = bytes_from_usize(piece);
				let offset = bytes_from_usize(offset);
				let len = bytes_from_usize(len);
				self.write_bytes(&bytes_from_u32(13))
					.and_then(|_| self.write_bytes(&[6]))
					.and_then(|_| self.write_bytes(&piece))
					.and_then(|_| self.write_bytes(&offset))
					.and_then(|_| self.write_bytes(&len))
			}
			RawMessage::Piece(piece, offset, bytes) => {
				let piece = bytes_from_usize(piece);
				let offset = bytes_from_usize(offset);
				let len = bytes.len() + 9;
				self.write_bytes(&bytes_from_usize(len))
					.and_then(|_| self.write_bytes(&[7]))
					.and_then(|_| self.write_bytes(&piece))
					.and_then(|_| self.write_bytes(&offset))
					.and_then(|_| self.write_bytes(&bytes))
			}
			RawMessage::Cancel(piece, offset, len) => {
				let piece = bytes_from_usize(piece);
				let offset = bytes_from_usize(offset);
				let len = bytes_from_usize(len);
				self.write_bytes(&bytes_from_u32(13))
					.and_then(|_| self.write_bytes(&[8]))
					.and_then(|_| self.write_bytes(&piece))
					.and_then(|_| self.write_bytes(&offset))
					.and_then(|_| self.write_bytes(&len))
			}
		};
		try!(write_result);
		self.stream.flush().map_err(Error::IoError)
	}

	fn send_handshake(&mut self) -> Result<(), Error> {
		let mut handshake = [0_u8; 68];
		for i in 0..8_usize {
			handshake[i + 20] = 0;
		}
		for i in 0..20_usize {
			handshake[i] = b"\x13BitTorrent protocol"[i];
			handshake[i + 28] = self.handshake.info_hash[i];
			handshake[i + 48] = self.handshake.id.0[i];
		}
		try!(self.write_bytes(&handshake));
		try!(self.stream.flush().map_err(Error::IoError));
		debug!("Sent handshake to {}", self.peer);
		Ok(())
	}

	fn check_handshake(&mut self) -> Result<Option<HandshakeInfo>, Error> {
		if self.recv_buffer.len() < 68 {
			Ok(None)
		} else {
			if &self.recv_buffer[0..20] == b"\x13BitTorrent protocol" {
				let mut hash = [0; 20];
				let mut id = DownloaderId([0; 20]);
				for i in 0..20 { hash[i] = self.recv_buffer[28 + i]; }
				for i in 0..20 { id.0[i] = self.recv_buffer[48 + i]; }
				self.remove_bytes(68);
				debug!("Completed handshake with {}", self.peer);
				Ok(Some(HandshakeInfo::new(hash, id)))
			} else {
				Err(Error::BadHandshake)
			}
		}
	}

	fn run(&mut self) -> Result<(), Error> {
		let read_timeout = Duration::from_millis(100);
		try!(self.stream.set_read_timeout(Some(read_timeout))
			.map_err(Error::IoError));

		try!(self.send_handshake());

		let mut checks = 0;
		loop {
			try!(self.receive_bytes());
			match try!(self.check_handshake()) {
				Some(handshake) => {
					self.send(InMessage::Handshake(handshake));
					break;
				}
				None => {
					checks += 1;
					if checks > 20 {
						return Err(Error::NoHandshake);
					}
				}
			}
			thread::sleep(Duration::from_millis(1000));
		}

		loop {
			try!(self.receive_bytes());
			match try!(self.get_raw_message()) {
				Some(RawMessage::KeepAlive) => {
					debug!("Got KeepAlive from {}", self.peer);
					// TODO: check for timeouts & shit
					// also send my own keepalives
				}
				Some(RawMessage::Choke) => {
					debug!("Got Choke from {}", self.peer);
					self.send(InMessage::Normal(Message::Choke));
				}
				Some(RawMessage::Unchoke) => {
					debug!("Got Unchoke from {}", self.peer);
					self.send(InMessage::Normal(Message::Unchoke));
				}
				Some(RawMessage::Interested) => {
					debug!("Got Interested from {}", self.peer);
					self.send(InMessage::Normal(Message::Interested));
				}
				Some(RawMessage::NotInterested) => {
					debug!("Got NotInterested from {}", self.peer);
					self.send(InMessage::Normal(Message::NotInterested));
				}
				Some(RawMessage::Have(piece)) => {
					debug!("Got Have({}) from {}", piece, self.peer);
					self.send(InMessage::Normal(Message::Have(piece)));
				}
				Some(RawMessage::Bitfield(bits)) => {
					debug!("Got Bitfield({} bits) from {}", bits.len() * 8, self.peer);
					self.send(InMessage::Normal(Message::Bitfield(bits)));
				}
				Some(RawMessage::Request(piece, offset, len)) => {
					debug!("Got Request({}, {}, {}) from {}", piece, offset, len, self.peer);
					self.send(InMessage::Normal(Message::Request(piece, offset, len)));
				}
				Some(RawMessage::Piece(piece, offset, bytes)) => {
					debug!("Got Piece({}, {}, {} bytes) from {}", piece, offset, bytes.len(), self.peer);
					self.send(InMessage::Normal(Message::Piece(piece, offset, bytes)));
				}
				Some(RawMessage::Cancel(piece, offset, len)) => {
					debug!("Got Cancel({}, {}, {}) from {}", piece, offset, len, self.peer);
					self.send(InMessage::Normal(Message::Cancel(piece, offset, len)));
				}
				None => { }
			}

			match self.receive() {
				Some(OutMessage::Close) => {
					return Err(Error::Closed);
				}
				Some(OutMessage::Normal(msg)) => {
					let raw = RawMessage::from_message(msg);
					try!(self.write_message(raw));
				}
				None => { }
			}
		}
	}
}

fn u32_from_bytes(slice: &[u8]) -> u32 {
	let b1 = slice[0] as u32;
	let b2 = slice[1] as u32;
	let b3 = slice[2] as u32;
	let b4 = slice[3] as u32;
	(b1 << 24) | (b2 << 16) | (b3 << 8) | b4
}

fn bytes_from_u32(num: u32) -> [u8; 4] {
	let b1 = ((num >> 24) & 0xFF) as u8;
	let b2 = ((num >> 16) & 0xFF) as u8;
	let b3 = ((num >> 8) & 0xFF) as u8;
	let b4 = (num & 0xFF) as u8;
	[b1, b2, b3, b4]
}

fn usize_from_bytes(slice: &[u8]) -> usize {
	u32_from_bytes(slice) as usize
}

fn bytes_from_usize(num: usize) -> [u8; 4] {
	if num > ::std::u32::MAX as usize {
		// not very nice, but not checking it is also not too good
		panic!("Encoding too large usize value: {}", num);
	}
	bytes_from_u32(num as u32)
}
