pub mod http;

use downloader::{DownloaderId, PeerAddress};


pub struct TrackerArgs {
	pub tracker_url: String,
	pub info_hash: [u8; 20],
	pub id: DownloaderId,
	pub port: u16,
}

pub trait Tracker {
	fn new(args: TrackerArgs) -> Self where Self: Sized;
	fn update(&mut self, down: usize, up: usize, left: usize);
	fn peers<'a>(&'a self) -> Box<Iterator<Item=&'a PeerAddress> + 'a>;
}

pub fn create_tracker(args: TrackerArgs) -> Box<Tracker> {
	// TODO: check tracker url and return udp tracker client if needed
	// right now just always try to use http tracker client
	Box::new(http::HttpTracker::new(args))
}