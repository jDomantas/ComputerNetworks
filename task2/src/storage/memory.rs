use std::path::PathBuf;
use storage::*;
use torrent::{TorrentInfo, File};


struct Piece {
	size: usize,
	data: Vec<u8>,
	hash: [u8; 20],
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
			pieces.push(Piece {
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
				}
				Ok(())
			}
		})
	}

	fn bytes_missing(&self) -> usize {
		self.pieces.iter()
			.map(|ref piece| piece.size - piece.data.len())
			.fold(0, |a, b| a + b)
	}
}
