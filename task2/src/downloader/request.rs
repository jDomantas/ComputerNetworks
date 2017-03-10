#[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Hash, Clone)]
pub struct Request {
	pub piece: usize,
	pub offset: usize,
	pub length: usize,
}

impl Request {
	pub fn new(piece: usize, offset: usize, length: usize) -> Request {
		Request {
			piece: piece,
			offset: offset,
			length: length,
		}
	}

	pub fn split_request(&self, max_length: usize) -> RequestSplitIter {
		RequestSplitIter {
			piece: self.piece,
			max_length: max_length,
			start: self.offset,
			end: self.offset + self.length,
		}
	}

	pub fn intersects(&self, other: &Request) -> bool {
		self.piece == other.piece
		&& self.offset < other.offset + other.length
		&& other.offset < self.offset + self.length
	}
}

pub struct RequestSplitIter {
	piece: usize,
	max_length: usize,
	start: usize,
	end: usize,
}

impl Iterator for RequestSplitIter {
	type Item = Request;
	fn next(&mut self) -> Option<Request> {
		if self.start >= self.end {
			None
		} else {
			let start = self.start;
			let len = ::std::cmp::min(self.end - start, self.max_length);
			self.start += self.max_length;
			Some(Request::new(self.piece, start, len))
		}
	}
}
