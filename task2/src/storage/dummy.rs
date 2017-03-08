use std::path::PathBuf;
use storage::*;
use torrent::TorrentInfo;


#[derive(Clone, Copy)]
pub struct DummyStorage(usize);

impl Storage for DummyStorage {
	fn new(_dir: PathBuf, info: TorrentInfo) -> Self {
		let total_length: usize = info.files.iter()
			.map(|f| f.length as usize)
			// sum is unstable :'(
			.fold(0, |a, b| a + b);
		DummyStorage(total_length)
	}

	fn get_piece(&mut self, _index: usize) -> Option<&[u8]> {
		None
	}

	fn store_block(&mut self, _block: Block) -> Result<(), BadBlock> {
		Ok(())
	}

	fn create_request(&self) -> Option<Request> {
		None
	}

	fn bytes_missing(&self) -> usize {
		let DummyStorage(missing) = *self;
		missing
	}

	fn is_complete(&self) -> bool {
		self.bytes_missing() == 0
	}
}