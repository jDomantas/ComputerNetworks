use std::net::{TcpStream, Ipv4Addr};
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;
use std::io;
use std::io::{Read, Write, ErrorKind};


pub enum Message {
	Have(u32),
	Bitfield(Vec<u8>),
	Request(u32, u32, u32),
	Piece(u32, u32, Vec<u8>),
	Cancel(u32, u32, u32),
	Dead,
}

pub struct Connection {
	sender: Sender<Message>,
	receiver: Receiver<Message>,
	thread: thread::JoinHandle<()>,
	dead: bool,
}

impl Connection {
	pub fn new(ip: u32, port: u16) -> Connection {
		let (send1, receive1) = mpsc::channel();
		let (send2, receive2) = mpsc::channel();
		let ip = Ipv4Addr::from(ip);
		let name = format!("{}:{}", ip, port);
		let thread = thread::spawn(move || {
			let mut con = match ConnectionInternal::new(send1, receive2, ip, port) {
				Ok(con) => con,
				Err(e) => {
					let description = e.description();
					println!("Connection to {} closed:\n  {}", name, description);
					return;
				}
			};
			match con.run() {
				Ok(()) => { }
				Err(e) => {
					let description = e.description();
					println!("Connection to {} died:\n  {}", name, description);
				}
			}
		});
		Connection {
			sender: send2,
			receiver: receive1,
			dead: false,
			thread: thread,
		}
	}

	pub fn is_dead(&self) -> bool {
		self.dead
	}

	pub fn send(&mut self, msg: Message) {
		// we don't care if the message is not sent
		let _ = self.sender.send(msg);
	}

	pub fn receive(&mut self) -> Option<Message> {
		if self.dead {
			Some(Message::Dead)
		} else {
			match self.receiver.try_recv() {
				Ok(Message::Dead) => {
					self.dead = true;
					Some(Message::Dead)
				}
				Ok(msg) => {
					Some(msg)
				}
				Err(TryRecvError::Empty) => {
					None
				}
				Err(TryRecvError::Disconnected) => {
					self.dead = true;
					Some(Message::Dead)
				}
			}
		}
	}
}

#[derive(Debug)]
pub enum ConnectionError {
	FailedToConnect(io::Error),
	FailedToInit(io::Error),
	MalformedMessage,
	BadMessageType(u8),
	BadMessageLength(u32),
	ConnectionClosed,
	ReadError(io::Error),
	WriteError(io::Error),
}

pub type Result<T> = ::std::result::Result<T, ConnectionError>;

enum RawMessage {
	KeepAlive,
	Choke,
	Unchoke,
	Interested,
	NotInterested,
	Have(u32),
	Bitfield(Vec<u8>),
	Request(u32, u32, u32),
	Piece(u32, u32, Vec<u8>),
	Cancel(u32, u32, u32),
}

struct ConnectionInternal {
	sender: Sender<Message>,
	receiver: Receiver<Message>,
	stream: TcpStream,
	peer_name: String,
	next_message: Vec<u8>,
	interested: bool,
	chocked: bool,
}

impl ConnectionInternal {
	fn new(send: Sender<Message>, recv: Receiver<Message>, ip: Ipv4Addr, port: u16)
			-> Result<ConnectionInternal> {

		let name = format!("{}:{}", ip, port);
		println!("Connecting to peer {}", name);
		let stream = try!(TcpStream::connect((ip, port))
			.map_err(ConnectionError::FailedToConnect));
		println!("Connected to {}", name);
		Ok(ConnectionInternal {
			sender: send,
			receiver: recv,
			stream: stream,
			peer_name: name,
			next_message: Vec::new(),
			interested: false,
			chocked: true,
		})
	}

	fn send(&mut self, msg: Message) {
		// we don't care if the message is not sent
		let _ = self.sender.send(msg);
	}

	fn receive(&mut self) -> Option<Message> {
		match self.receiver.try_recv() {
			Ok(msg) => Some(msg),
			Err(TryRecvError::Empty) => None,
			Err(TryRecvError::Disconnected) => Some(Message::Dead),
		}
	}

	fn next_message_length(&self) -> Option<u32> {
		if self.next_message.len() >= 4 {
			Some(u32_from_bytes(&self.next_message[0..4]))
		} else {
			None
		}
	}

	fn decode_raw_message(&self, slice: &[u8]) -> Result<RawMessage> {
		if slice.len() == 0 {
			return Ok(RawMessage::KeepAlive);
		}
		match slice[0] {
			0 => {
				if slice.len() == 1 { 
					Ok(RawMessage::Choke)
				} else {
					Err(ConnectionError::MalformedMessage)
				}
			}
			1 => {
				if slice.len() == 1 { 
					Ok(RawMessage::Unchoke)
				} else {
					Err(ConnectionError::MalformedMessage)
				}
			}
			2 => {
				if slice.len() == 1 { 
					Ok(RawMessage::Interested)
				} else {
					Err(ConnectionError::MalformedMessage)
				}
			}
			3 => {
				if slice.len() == 1 { 
					Ok(RawMessage::NotInterested)
				} else {
					Err(ConnectionError::MalformedMessage)
				}
			}
			4 => {
				if slice.len() == 5 {
					Ok(RawMessage::Have(u32_from_bytes(&slice[1..5])))
				} else {
					Err(ConnectionError::MalformedMessage)
				}
			}
			5 => {
				Ok(RawMessage::Bitfield(slice[1..].to_vec()))
			}
			6 => {
				if slice.len() == 13 {
					let piece = u32_from_bytes(&slice[1..5]);
					let offset = u32_from_bytes(&slice[5..9]);
					let length = u32_from_bytes(&slice[9..13]);
					Ok(RawMessage::Request(piece, offset, length))
				} else {
					Err(ConnectionError::MalformedMessage)
				}
			}
			7 => {
				if slice.len() >= 9 {
					let piece = u32_from_bytes(&slice[1..5]);
					let offset = u32_from_bytes(&slice[5..9]);
					let data = slice[9..].to_vec();
					Ok(RawMessage::Piece(piece, offset, data))
				} else {
					Err(ConnectionError::MalformedMessage)
				}
			}
			8 => {
				if slice.len() == 13 {
					let piece = u32_from_bytes(&slice[1..5]);
					let offset = u32_from_bytes(&slice[5..9]);
					let length = u32_from_bytes(&slice[9..13]);
					Ok(RawMessage::Cancel(piece, offset, length))
				} else {
					Err(ConnectionError::MalformedMessage)
				}
			}
			x => {
				Err(ConnectionError::BadMessageType(x))
			}
		}
	}

	fn get_raw_message(&mut self) -> Result<Option<RawMessage>> {
		match self.stream.read_to_end(&mut self.next_message) {
			Ok(_) => {
				// connection was closed?
				return Err(ConnectionError::ConnectionClosed);
			}
			Err(ref e) if
				e.kind() == ErrorKind::TimedOut ||
				e.kind() == ErrorKind::WouldBlock => {
				// read timed out, everything is fine
			}
			Err(e) => {
				return Err(ConnectionError::ReadError(e));
			}
		}
		
		let len = match self.next_message_length() {
			Some(len) if len < (1 << 20) => len as usize,
			Some(len) => return Err(ConnectionError::BadMessageLength(len)),
			None => return Ok(None),
		};

		if self.next_message.len() >= len + 4 {
			let message = {
				let message_data = &self.next_message[4..(4 + len)];
				try!(self.decode_raw_message(message_data))
			};
			// remove message that was just decoded
			for _ in self.next_message.drain(0..(len + 4)) { }
			Ok(Some(message))
		} else {
			Ok(None)
		}
	}

	fn write_bytes(&mut self, bytes: &[u8]) -> Result<()> {
		match self.stream.write_all(bytes) {
			Ok(_) => Ok(()),
			Err(e) => Err(ConnectionError::WriteError(e)),
		}
	}

	fn write_message(&mut self, msg: RawMessage) -> Result<()> {
		match msg {
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
				let index = bytes_from_u32(index);
				self.write_bytes(&bytes_from_u32(5))
					.and_then(|_| self.write_bytes(&[4]))
					.and_then(|_| self.write_bytes(&index))
			}
			RawMessage::Bitfield(bits) => {
				let len = bits.len() + 1;
				self.write_bytes(&bytes_from_u32(len as u32))
					.and_then(|_| self.write_bytes(&[5]))
					.and_then(|_| self.write_bytes(&bits))
			}
			RawMessage::Request(piece, offset, len) => {
				let piece = bytes_from_u32(piece);
				let offset = bytes_from_u32(offset);
				let len = bytes_from_u32(len);
				self.write_bytes(&bytes_from_u32(13))
					.and_then(|_| self.write_bytes(&[6]))
					.and_then(|_| self.write_bytes(&piece))
					.and_then(|_| self.write_bytes(&offset))
					.and_then(|_| self.write_bytes(&len))
			}
			RawMessage::Piece(piece, offset, bytes) => {
				let piece = bytes_from_u32(piece);
				let offset = bytes_from_u32(offset);
				let len = bytes.len() + 9;
				self.write_bytes(&bytes_from_u32(len as u32))
					.and_then(|_| self.write_bytes(&[7]))
					.and_then(|_| self.write_bytes(&piece))
					.and_then(|_| self.write_bytes(&offset))
					.and_then(|_| self.write_bytes(&bytes))
			}
			RawMessage::Cancel(piece, offset, len) => {
				let piece = bytes_from_u32(piece);
				let offset = bytes_from_u32(offset);
				let len = bytes_from_u32(len);
				self.write_bytes(&bytes_from_u32(13))
					.and_then(|_| self.write_bytes(&[8]))
					.and_then(|_| self.write_bytes(&piece))
					.and_then(|_| self.write_bytes(&offset))
					.and_then(|_| self.write_bytes(&len))
			}
		}
	}

	fn run(&mut self) -> Result<()> {
		let read_timeout = Duration::from_millis(100);
		try!(self.stream.set_read_timeout(Some(read_timeout))
			.map_err(ConnectionError::FailedToInit));

		loop {
			match try!(self.get_raw_message()) {
				Some(RawMessage::KeepAlive) => {
					println!("Got KeepAlive from {}", self.peer_name);
				}
				Some(RawMessage::Choke) => {
					println!("{} is chocked", self.peer_name);
					self.chocked = true;
				}
				Some(RawMessage::Unchoke) => {
					println!("{} is not chocked", self.peer_name);
					self.chocked = false;
				}
				Some(RawMessage::Interested) => {
					println!("{} is interested", self.peer_name);
					self.interested = true;
				}
				Some(RawMessage::NotInterested) => {
					println!("{} is not interested", self.peer_name);
					self.interested = false;
				}
				Some(RawMessage::Have(piece)) => {
					println!("{} has piece #{}", self.peer_name, piece);
					self.send(Message::Have(piece));
				}
				Some(RawMessage::Bitfield(bits)) => {
					println!("{} sent bitfield (length: {})", self.peer_name, bits.len() * 8);
					self.send(Message::Bitfield(bits));
				}
				Some(RawMessage::Request(piece, offset, len)) => {
					println!("{} wants piece #{}, off: {}, len: {}", self.peer_name, piece, offset, len);
					self.send(Message::Request(piece, offset, len));
				}
				Some(RawMessage::Piece(piece, offset, bytes)) => {
					println!("{} sent piece #{}, off: {}, len: {}", self.peer_name, piece, offset, bytes.len());
					self.send(Message::Piece(piece, offset, bytes));
				}
				Some(RawMessage::Cancel(piece, offset, len)) => {
					println!("{} canceled request for piece #{}, off: {}, len: {}", self.peer_name, piece, offset, len);
					self.send(Message::Cancel(piece, offset, len));
				}
				None => { }
			}

			match self.receive() {
				Some(Message::Dead) => {
					return Err(ConnectionError::ConnectionClosed);
				}
				Some(Message::Have(index)) => {
					try!(self.write_message(RawMessage::Have(index)));
				}
				Some(Message::Bitfield(field)) => {
					try!(self.write_message(RawMessage::Bitfield(field)));
				}
				Some(Message::Piece(index, offset, bits)) => {
					try!(self.write_message(RawMessage::Piece(index, offset, bits)));
				}
				Some(Message::Request(index, offset, len)) => {
					try!(self.write_message(RawMessage::Request(index, offset, len)));
				}
				Some(Message::Cancel(index, offset, len)) => {
					try!(self.write_message(RawMessage::Cancel(index, offset, len)));
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

impl ConnectionError {
	pub fn description(&self) -> String {
		match self {
			&ConnectionError::BadMessageLength(len) =>
				format!("bad message length: {}", len),
			&ConnectionError::BadMessageType(typ) =>
				format!("bad message type: {}", typ),
			&ConnectionError::ConnectionClosed =>
				"connection closed".to_string(),
			&ConnectionError::FailedToConnect(_) =>
				"failed to connect".to_string(),
			&ConnectionError::FailedToInit(_) =>
				"failed to initialize".to_string(),
			&ConnectionError::MalformedMessage =>
				"received malformed message".to_string(),
			&ConnectionError::ReadError(_) =>
				"read error".to_string(),
			&ConnectionError::WriteError(_) =>
				"write error".to_string(),
		}
	}
}
