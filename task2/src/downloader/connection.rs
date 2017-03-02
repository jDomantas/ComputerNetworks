use std::net::{TcpStream, Ipv4Addr};
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;
use std::io::{Read, Write, ErrorKind};


pub enum Message {
	Have(u32),
	Bitfield(Vec<u8>),
	Request(u32, u32, u32),
	Piece(u32, u32, Vec<u8>),
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
		let thread = thread::spawn(move || {
			let mut con = match ConnectionInternal::new(send1, receive2, ip, port) {
				Ok(con) => con,
				Err(e) => {
					println!("Peer connection died: {}", e);
					return;
				}
			};
			match con.run() {
				Ok(()) => { }
				Err(e) => {
					println!("Peer connection {} died: {}", con.peer_name, e);
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
}

impl ConnectionInternal {
	fn new(send: Sender<Message>, recv: Receiver<Message>, ip: u32, port: u16)
			-> Result<ConnectionInternal, String> {

		let ip = Ipv4Addr::from(ip);
		let name = format!("{}:{}", ip, port);
		println!("Connecting to peer {}", name);
		let stream = try!(TcpStream::connect((ip, port)).map_err(|_|
			format!("Failed to connect to {}", name)));
		println!("Connected to {}", name);
		Ok(ConnectionInternal {
			sender: send,
			receiver: recv,
			stream: stream,
			peer_name: name,
			next_message: Vec::new(),
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

	fn decode_raw_message(&self, slice: &[u8]) -> Result<RawMessage, String> {
		if slice.len() == 0 {
			return Ok(RawMessage::KeepAlive);
		}
		match slice[0] {
			0 => {
				if slice.len() == 1 { 
					Ok(RawMessage::Choke)
				} else {
					Err("Malformed CHOKE message".to_string())
				}
			}
			1 => {
				if slice.len() == 1 { 
					Ok(RawMessage::Unchoke)
				} else {
					Err("Malformed UNCHOKE message".to_string())
				}
			}
			2 => {
				if slice.len() == 1 { 
					Ok(RawMessage::Interested)
				} else {
					Err("Malformed INTERESTED message".to_string())
				}
			}
			3 => {
				if slice.len() == 1 { 
					Ok(RawMessage::NotInterested)
				} else {
					Err("Malformed NOTINTERESTED message".to_string())
				}
			}
			4 => {
				if slice.len() == 5 {
					Ok(RawMessage::Have(u32_from_bytes(&slice[1..5])))
				} else {
					Err("Malformed HAVE message".to_string())
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
					Err("Malformed REQUEST message".to_string())
				}
			}
			7 => {
				if slice.len() >= 9 {
					let piece = u32_from_bytes(&slice[1..5]);
					let offset = u32_from_bytes(&slice[5..9]);
					let data = slice[9..].to_vec();
					Ok(RawMessage::Piece(piece, offset, data))
				} else {
					Err("Malformed PIECE message".to_string())
				}
			}
			8 => {
				if slice.len() == 13 {
					let piece = u32_from_bytes(&slice[1..5]);
					let offset = u32_from_bytes(&slice[5..9]);
					let length = u32_from_bytes(&slice[9..13]);
					Ok(RawMessage::Cancel(piece, offset, length))
				} else {
					Err("Malformed CANCEL message".to_string())
				}
			}
			x => {
				Err(format!("Invalid message type: {}", x))
			}
		}
	}

	fn get_raw_message(&mut self) -> Result<Option<RawMessage>, String> {
		match self.stream.read_to_end(&mut self.next_message) {
			Ok(_) => {
				// connection was closed?
				return Err("Connection closed".to_string());
			}
			Err(ref e) if
				e.kind() == ErrorKind::TimedOut ||
				e.kind() == ErrorKind::WouldBlock => {
				// read timed out, everything is fine
			}
			Err(e) => {
				return Err(format!("Connection error: {}", e));
			}
		}
		
		let len = match self.next_message_length() {
			Some(len) if len < (1 << 20) => len as usize,
			Some(len) => return Err(format!("Message too large: {}", len)),
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

	fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), String> {
		match self.stream.write_all(bytes) {
			Ok(_) => Ok(()),
			Err(_) => Err("Write failed".to_string()),
		}
	}

	fn write_message(&mut self, msg: RawMessage) -> Result<(), String> {
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

	fn run(&mut self) -> Result<(), String> {
		let read_timeout = Duration::from_millis(100);
		try!(self.stream.set_read_timeout(Some(read_timeout)).map_err(|_|
			"Failed to set stream to nonblocking".to_string()));

		loop {
			match try!(self.get_raw_message()) {
				Some(_) => {
					unimplemented!()
				}
				None => { }
			}

			match self.receive() {
				Some(Message::Dead) => {
					return Err("Killed".to_string())
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
