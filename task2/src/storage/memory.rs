use std::path::PathBuf;
use storage::*;
use torrent::{TorrentInfo, File};


const REQUEST_SIZE: usize = 0x4000; // 16 kb

struct Piece {
	index: usize,
	size: usize,
	data: Vec<u8>,
	hash: [u8; 20],
}

impl Piece {
	fn is_complete(&self) -> bool {
		self.size == self.data.len()
	}

	fn is_correct(&self) -> bool {
		if !self.is_complete() {
			return true;
		}
		let mut hasher = ::sha1::Sha1::new();
		hasher.update(&self.data);
		hasher.digest().bytes() == self.hash
	}

	fn validate(&mut self) {
		if !self.is_correct() {
			self.data.clear();
		}
	}

	fn fill_request(&self) -> Option<Request> {
		let missing_size = self.size - self.data.len();
		let offset = self.data.len();
		if missing_size > REQUEST_SIZE {
			Some(Request::new(self.index, offset, REQUEST_SIZE))
		} else if missing_size > 0 {
			Some(Request::new(self.index, offset, missing_size))
		} else {
			None
		}
	}
}

pub struct MemoryStorage {
	pieces: Vec<Piece>,
	files: Vec<File>,
	dir: PathBuf,
}


impl Storage for MemoryStorage {
	fn new(dir: PathBuf, info: TorrentInfo) -> Self {
		let mut size = info.files.iter()
			.map(|ref f| f.length as usize)
			.fold(0, |a, b| a + b);
		let mut pieces = Vec::new();
		for hash in info.pieces {
			if size == 0 {
				panic!("Cannot divide to pieces");
			}
			let s = if size > info.piece_length as usize {
				info.piece_length as usize
			} else {
				size
			};
			size -= s;
			let index = pieces.len();
			pieces.push(Piece {
				index: index,
				size: s,
				data: Vec::new(),
				hash: hash,
			});
		}
		if size != 0 {
			panic!("Cannot divide to pieces");
		}
		MemoryStorage {
			pieces: pieces,
			files: info.files,
			dir: dir,
		}
	}

	fn get_piece(&mut self, index: usize) -> Option<&[u8]> {
		self.pieces.get(index).and_then(|ref piece| {
			if piece.data.len() == piece.size {
				Some(piece.data.as_slice())
			} else {
				None
			}
		})
	}

	fn store_block(&mut self, block: Block) -> Result<(), BadBlock> {
		self.pieces.get_mut(block.piece).ok_or(BadBlock).and_then(|ref mut piece| {
			let old_end = piece.data.len();
			let new_end = block.offset + piece.data.len();
			if new_end > piece.size {
				Err(BadBlock)
			} else {
				if new_end > old_end && block.offset <= old_end {
					let skip = block.offset - old_end;
					for &byte in &block.data[skip..] {
						piece.data.push(byte);
					}
					piece.validate();
				}
				Ok(())
			}
		})
	}

	fn create_request(&self) -> Option<Request> {
		self.pieces.iter()
			.map(Piece::fill_request)
			.fold(None, |a, b| a.or(b))
	}

	fn bytes_missing(&self) -> usize {
		self.pieces.iter()
			.map(|ref piece| piece.size - piece.data.len())
			.fold(0, |a, b| a + b)
	}
}
