//use std::collections::{HashMap, Entry};
use std::collections::hash_map::{HashMap, Entry};
use storage::*;
use downloader::request::Request;
use torrent::TorrentInfo;


const MAX_PARTIAL_PIECES: usize = 100;

struct Segment {
	start: usize,
	end: usize,
	data: Vec<u8>,
}

impl Segment {
	fn from_block(block: Block) -> Segment {
		Segment::new(block.offset, block.data)
	}

	fn new(offset: usize, data: Vec<u8>) -> Segment {
		Segment {
			start: offset,
			end: offset + data.len(),
			data: data,
		}
	}

	fn intersects(&self, other: &Segment) -> bool {
		self.start <= other.end && other.start <= self.end
	}

	fn merge(self, other: Segment) -> Segment {
		// can only be called if intersects other segment
		assert!(self.intersects(&other));

		let (mut first, second) = if self.start < other.start {
			(self, other)
		} else {
			(other, self)
		};

		if first.end < second.end {
			let extra_bytes = second.end - first.end;
			let data = &second.data[(second.data.len() - extra_bytes)..];
			first.data.extend_from_slice(data);
			first.end = second.end;
		}
		first
	}
}

struct PartialPiece {
	piece: usize,
	length: usize,
	segments: Vec<Segment>,
}

impl PartialPiece {
	fn new(piece: usize, length: usize) -> PartialPiece {
		PartialPiece {
			piece: piece,
			length: length,
			segments: Vec::new(),
		}
	}

	fn add_segment(&mut self, segment: Segment) -> Result<usize, BadBlock> {
		if segment.end > self.length {
			return Err(BadBlock);
		}

		let (start, end) = self.intersecting(&segment);
		let mut removed = 0;
		let mut new_segment = segment;
		for seg in self.segments.drain(start..end) {
			removed += seg.data.len();
			new_segment = new_segment.merge(seg);
		}

		let added = new_segment.data.len() - removed;
		self.segments.insert(start, new_segment);
		Ok(added)
	}

	fn intersecting(&self, segment: &Segment) -> (usize, usize) {
		// first segment that is not completely before given one
		let start = self.segments.iter().enumerate()
			.filter_map(|(index, seg)| {
				if seg.end >= segment.start {
					Some(index)
				} else {
					None
				}
			})
			.next()
			.unwrap_or(self.segments.len());
		
		// first segment that is completely after given one
		let end = self.segments.iter().enumerate()
			.filter_map(|(index, seg)| {
				if seg.start > segment.end {
					Some(index)
				} else {
					None
				}
			})
			.next()
			.unwrap_or(self.segments.len());

		(start, end)
	}

	fn bytes_stored(&self) -> usize {
		self.segments.iter().fold(0, |acc, ref seg| acc + seg.data.len())
	}

	fn bytes_missing(&self) -> usize {
		self.length - self.bytes_stored()
	}

	fn is_complete(&self) -> bool {
		self.bytes_missing() == 0
	}

	fn requests<'a>(&'a self) -> Box<Iterator<Item=Request> + 'a> {
		let last_end = self.segments.iter().last().map(|s| s.end).unwrap_or(0);
		let last_request = Request::new(self.piece, last_end, self.length - last_end);
		let mut start = 0;
		Box::new(self.segments.iter()
			.map(move |ref seg| {
				let request = Request::new(self.piece, start, seg.start - start);
				start = seg.end;
				request
			})
			.chain(Some(last_request).into_iter()))
	}
}

struct PieceRequestIter<'a> {
	piece: &'a PartialPiece,

}

pub struct PartialStorage<S: Storage> {
	partial_pieces: HashMap<usize, PartialPiece>,
	backed_storage: S,
	pieces: usize,
	piece_size: usize,
	last_piece_size: usize,
}

impl<S: Storage> Storage for PartialStorage<S> {
	fn new(info: TorrentInfo) -> Self {
		let size = info.files.iter()
			.map(|ref f| f.length as usize)
			.fold(0, |a, b| a + b);
		let last_piece_size = size - (info.pieces.len() - 1) * (info.piece_length as usize);
		let pieces = info.pieces.len();
		let piece_size = info.piece_length as usize;
		let backed = S::new(info);
		PartialStorage {
			partial_pieces: HashMap::new(),
			backed_storage: backed,
			pieces: pieces,
			piece_size: piece_size,
			last_piece_size: last_piece_size,
		}
	}

	fn get_piece(&mut self, index: usize) -> Option<&[u8]> {
		self.backed_storage.get_piece(index)
	}

	fn store_block(&mut self, block: Block) -> Result<usize, BadBlock> {
		if block.piece >= self.pieces {
			return Err(BadBlock);
		}

		self.receiving_piece(block.piece);

		match self.partial_pieces.entry(block.piece) {
			Entry::Occupied(mut entry) => {
				let segment = Segment::from_block(block);
				let added = try!(entry.get_mut().add_segment(segment));
				if entry.get().is_complete() {
					let (index, piece) = entry.remove_entry();
					assert!(piece.segments.len() == 1);
					let segment = piece.segments.into_iter().next().unwrap();
					let block = Block::new(index, segment.start, segment.data);
					self.backed_storage.store_block(block)
						.ok().expect("backed storage refused complete block");
				}
				Ok(added)
			}
			Entry::Vacant(_) => {
				Ok(0)
			}
		}
	}

	fn bytes_missing(&self) -> usize {
		// TODO: this is not strictly correct, because backed storage
		// might have some parts of pieces we store as partial,
		// but maybe it is good enough.

		let from_partials = self.partial_pieces
			.values()
			.fold(0, |acc, ref piece| acc + piece.bytes_missing());
	
		self.backed_storage.bytes_missing() + from_partials
	}

	fn requests<'a>(&'a self) -> Box<Iterator<Item=Request> + 'a> {
		let requests = self.partial_pieces.values()
			.flat_map(PartialPiece::requests)
			.chain(self.backed_storage.requests());
		Box::new(requests)
	}

	fn is_complete(&self) -> bool {
		self.bytes_missing() == 0
	}
}

impl<S: Storage> PartialStorage<S> {
	fn receiving_piece(&mut self, piece: usize) {
		if self.partial_pieces.len() < MAX_PARTIAL_PIECES &&
			!self.partial_pieces.contains_key(&piece) &&
			!self.backed_storage.has_piece(piece) {

			let piece_length = if piece == self.pieces - 1 {
				self.last_piece_size
			} else {
				self.piece_size
			};
			let new_piece = PartialPiece::new(piece, piece_length);
			self.partial_pieces.insert(piece, new_piece);
		}
	}
}
