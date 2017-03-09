use storage::*;
use downloader::Request;
use torrent::{TorrentInfo, File};


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
			debug!("Hash mismatch, deleting piece #{}", self.index);
			self.data.clear();
		}
	}

	fn create_fill_request(&self) -> Option<Request> {
		let missing_size = self.size - self.data.len();
		let offset = self.data.len();
		if missing_size > 0 {
			Some(Request::new(self.index, offset, missing_size))
		} else {
			None
		}
	}
}

pub struct MemoryStorage {
	pieces: Vec<Piece>,
	files: Vec<File>,
	pieces_complete: usize,
}

impl Storage for MemoryStorage {
	fn new(info: TorrentInfo) -> Self {
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
			pieces_complete: 0,
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

	fn store_block(&mut self, block: Block) -> Result<usize, BadBlock> {
		let mut completed_piece = false;

		let res = self.pieces.get_mut(block.piece)
			.ok_or(BadBlock)
			.and_then(|ref mut piece| {
			let old_end = piece.data.len();
			let new_end = block.offset + block.data.len();
			if new_end > piece.size {
				Err(BadBlock)
			} else {
				if new_end > old_end && block.offset <= old_end {
					let skip = block.offset - old_end;
					for &byte in &block.data[skip..] {
						piece.data.push(byte);
					}
					piece.validate();
					if piece.is_complete() {
						completed_piece = true;
					}
					Ok(block.data.len() - skip)
				} else {
					Ok(0)
				}
			}
		});
		if completed_piece {
			self.pieces_complete += 1;
			info!("Downloaded piece #{} (completed: {}/{})",
				block.piece,
				self.pieces_complete,
				self.pieces.len());
		}
		if self.is_complete() {
			self.dump_to_file();
		}
		res
	}

	fn requests<'a>(&'a self) -> Box<Iterator<Item=Request> + 'a> {
		Box::new(self.pieces.iter().filter_map(|x| x.create_fill_request()))
	}

	fn bytes_missing(&self) -> usize {
		self.pieces.iter()
			.map(|ref piece| piece.size - piece.data.len())
			.fold(0, |a, b| a + b)
	}
}

impl MemoryStorage {
	fn dump_to_file(&self) {
		use std::io::prelude::*;
		let mut file = ::std::fs::File::create("./test.out").expect("Failed to create file");
		let mut total_size = 0_usize;
		for piece in &self.pieces {
			total_size += piece.size;
			file.write_all(&piece.data).expect("failed to write to file");
		}
		println!("Wrote to file, total size: {}", total_size);
	}
}
