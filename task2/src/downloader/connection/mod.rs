pub mod bt;

use std::io;
use downloader::DownloaderId;


#[derive(Debug, Clone)]
pub struct HandshakeInfo {
	pub info_hash: [u8; 20],
	pub id: DownloaderId,
}

impl HandshakeInfo {
	pub fn new(info_hash: [u8; 20], id: DownloaderId) -> HandshakeInfo {
		HandshakeInfo {
			info_hash: info_hash,
			id: id,
		}
	}
}

pub enum Message {
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

#[derive(Debug)]
pub enum Error {
	IoError(io::Error),
	BadHandshake,
	NoHandshake,
	BadMessage,
	Closed,
}

pub enum InMessage {
	Error(Error),
	Handshake(HandshakeInfo),
	Normal(Message),
}

pub trait Connection {
	fn send(&mut self, msg: Message);
	fn receive(&mut self) -> Option<InMessage>;
	fn close(&mut self);
	fn is_alive(&self) -> bool;
}
