pub mod dummy;
pub mod memory;

use std::path::PathBuf;
use torrent::TorrentInfo;


pub struct BadBlock;

pub struct Block {
	pub piece: usize,
	pub offset: usize,
	pub data: Vec<u8>,
}

impl Block {
	pub fn new(piece: usize, offset: usize, data: Vec<u8>) -> Block {
		Block {
			piece: piece,
			offset: offset,
			data: data,
		}
	}
}

pub struct Request {
	pub piece: usize,
	pub offset: usize,
	pub size: usize,
}

impl Request {
	pub fn new(piece: usize, offset: usize, size: usize) -> Request {
		Request {
			piece: piece,
			offset: offset,
			size: size,
		}
	}
}

pub trait Storage {
	fn new(dir: PathBuf, info: TorrentInfo) -> Self;
	fn get_piece(&mut self, index: usize) -> Option<&[u8]>;
	fn store_block(&mut self, block: Block) -> Result<(), BadBlock>;
	fn bytes_missing(&self) -> usize;
	fn create_request(&self) -> Option<Request>;

	fn is_complete(&self) -> bool {
		self.bytes_missing() == 0
	}
}
