use std::path::PathBuf;
use std::collections::HashMap;
use storage::Storage;
use torrent::TorrentInfo;


pub struct MemoryStorage {
	pieces: HashMap<usize, Vec<u8>>,
	piece_size: usize,
	piece_hashes: Vec<[u8; 20]>,
}

impl Storage for MemoryStorage {
	fn new(_dir: PathBuf, info: TorrentInfo) -> Self {
		MemoryStorage {
			pieces: HashMap::new(),
			piece_size: info.piece_length as usize,
			piece_hashes: info.pieces,
		}
	}

	fn contains_piece(&self, index: usize) -> bool {
		self.pieces.get(&index).is_some()
	}

	fn get_piece(&mut self, index: usize) -> Option<&[u8]> {
		self.pieces.get(&index).map(|x| x as &[u8])
	}

	fn store_block(&mut self, index: usize, offset: usize, data: Vec<u8>) {
		if index >= self.piece_hashes.len() {
			println!("Got bad piece, index: {}", index);
		} else if offset != 0 || data.len() != self.piece_size {
			println!("Got partial piece: off: {}, len: {}", offset, data.len());
			println!("Won't store :(");
		} else if !super::is_piece_valid(&data, &self.piece_hashes[index]) {
			println!("Piece is corrupt");
		} else {
			println!("Storing piece #{}", index);
			self.pieces.insert(index, data);
		}
	}

	fn bytes_missing(&self) -> usize {
		let pieces_missing = self.pieces.len() - self.piece_hashes.len();
		pieces_missing * self.piece_size
	}
}
