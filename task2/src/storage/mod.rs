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

pub trait Storage {
	fn new(dir: PathBuf, info: TorrentInfo) -> Self;
	fn get_piece(&mut self, index: usize) -> Option<&[u8]>;
	fn store_block(&mut self, block: Block) -> Result<(), BadBlock>;
	fn bytes_missing(&self) -> usize;

	fn is_complete(&self) -> bool {
		self.bytes_missing() == 0
	}
}

pub fn is_piece_valid(piece: &[u8], expected_hash: &[u8; 20]) -> bool {
	let mut hasher = ::sha1::Sha1::new();
	hasher.update(piece);
	&hasher.digest().bytes() == expected_hash
}
