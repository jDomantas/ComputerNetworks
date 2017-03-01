pub struct Torrent {
	pub tracker_url: String,
	pub info: TorrentInfo,
}

pub struct TorrentInfo {
	pub name: String,
	pub piece_length: u64
	pub pieces: Vec<,
}