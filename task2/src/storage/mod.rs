pub mod dummy;


use std::path::PathBuf;
use torrent::TorrentInfo;

pub trait Storage {
	fn new(dir: PathBuf, info: TorrentInfo) -> Self;
	fn contains_piece(&self, index: u64) -> bool;
	fn get_piece(&mut self, index: u64) -> Option<&[u8]>;
	fn store_block(&mut self, index: u64, offset: u64, data: Vec<u8>);
	fn bytes_missing(&self) -> u64;

	fn is_complete(&self) -> bool {
		self.bytes_missing() == 0
	}
}
