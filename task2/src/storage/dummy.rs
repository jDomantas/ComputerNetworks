use std::path::PathBuf;
use storage::Storage;
use torrent::TorrentInfo;


#[derive(Clone, Copy)]
pub struct DummyStorage(u64);

impl Storage for DummyStorage {
	fn new(_dir: PathBuf, info: TorrentInfo) -> Self {
		let total_length: u64 = info.files.iter()
			.map(|f| f.length)
			// sum is unstable :'(
			.fold(0, |a, b| a + b);
		DummyStorage(total_length)
	}

	fn contains_piece(&self, _index: u64) -> bool {
		false
	}

	fn get_piece(&mut self, _index: u64) -> Option<&[u8]> {
		None
	}

	fn store_block(&mut self, _index: u64, _offset: u64, _data: Vec<u8>) {

	}

	fn bytes_missing(&self) -> u64 {
		let DummyStorage(missing) = *self;
		missing
	}

	fn is_complete(&self) -> bool {
		self.bytes_missing() == 0
	}
}