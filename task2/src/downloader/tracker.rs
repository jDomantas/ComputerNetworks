use ::std::io::Read;
use ::hyper::client::{Client, IntoUrl};
use ::hyper::status::StatusCode;
use ::hyper::Url;
use ::std::time::{Instant, Duration};
use ::std::net::{Ipv4Addr, Ipv6Addr};
use bencode::*;
use downloader::DownloaderId;


pub trait Tracker {
	fn new(args: TrackerArgs) -> Self;
	fn update_tracker(&mut self, down: usize, up: usize, left: usize);
	fn latest_response(&mut self) -> Option<Response>;
}

pub struct TrackerArgs {
	pub tracker_url: String,
	pub info_hash: [u8; 20],
	pub id: DownloaderId,
	pub port: u16,
}

pub struct Response {
	pub query_interval: u64,
	pub peers: Vec<(Ipv6Addr, u16)>,
}

pub struct HttpTracker {
	client: Client,
	args: TrackerArgs,
	sent_started: bool,
	latest_response: Option<Response>,
	no_requests_before: Instant,
}

impl Tracker for HttpTracker {
	fn new(args: TrackerArgs) -> Self {
		let client = Client::new();
		HttpTracker {
			client: client,
			args: args,
			sent_started: false,
			latest_response: None,
			no_requests_before: Instant::now(),
		}
	}

	fn update_tracker(&mut self, down: usize, up: usize, left: usize) {
		if self.can_send_request() {
			let retry_interval = Duration::new(10, 0);
			self.no_requests_before = Instant::now() + retry_interval;
		} else {
			return;
		}
		let url = self.build_request(down, up, left);
		let response = self.client.get(url).send();
		match response {
			Ok(response) => {
				if response.status == StatusCode::Ok {
					self.sent_started = true;
					self.process_tracker_response(response);
				} else {
					println!("Tracker response status: {}", response.status);
				}
			}
			Err(error) => {
				println!("Tracker request failed.\n  {}", error);
			}
		}
	}

	fn latest_response(&mut self) -> Option<Response> {
		self.latest_response.take()
	}
}

impl HttpTracker {
	fn can_send_request(&self) -> bool {
		Instant::now() >= self.no_requests_before
	}

	fn process_tracker_response(&mut self, mut response: ::hyper::client::Response) {
		let mut body = Vec::new();
		match response.read_to_end(&mut body) {
			Ok(_) => {}
			Err(e) => {
				println!("Failed to read tracker response body.\n  {}", e);
				return;
			}
		}
		let bvalue = match decode(&body) {
			Ok(value) => value,
			Err(e) => {
				println!("Tracker response is malformed.\n  {:?}", e);
				return;
			}
		};
		let decoded = match decode_response(bvalue) {
			Ok(response) => response,
			Err(e) => {
				println!("Tracker response is malformed.\n  {}", e);
				return;
			}
		};
		println!("Got peers: {}", decoded.peers.len());
		for peer in decoded.peers.iter() {
			println!("  Ip: {}, port: {}", peer.0, peer.1);
		}
		self.store_response(decoded);
	}

	fn store_response(&mut self, response: Response) {
		let to_next_request = Duration::new(response.query_interval, 0);
		self.no_requests_before = Instant::now() + to_next_request;
		self.latest_response = Some(response);
	}

	fn build_request(&mut self, down: usize, up: usize, left: usize) -> Url {
		fn nibble_to_char(nibble: u8) -> char {
			if nibble < 10 {
				('0' as u8 + nibble) as char
			} else {
				('A' as u8 + nibble - 10) as char
			}
		}
		let mut url = self.args.tracker_url.clone();
		url.push_str("?info_hash=");
		for byte in &self.args.info_hash {
			url.push('%');
			url.push(nibble_to_char(byte >> 4));
			url.push(nibble_to_char(byte & 0xF));
		}
		push_url_arg(&mut url, "peer_id", "");
		for &byte in &self.args.id.0 {
			url.push(byte as char);
		}
		push_url_arg(&mut url, "port", &self.args.port.to_string());
		push_url_arg(&mut url, "uploaded", &up.to_string());
		push_url_arg(&mut url, "downloaded", &down.to_string());
		push_url_arg(&mut url, "left", &left.to_string());
		push_url_arg(&mut url, "compact", "1");
		if !self.sent_started {
			push_url_arg(&mut url, "event", "started");
		}
		// please?
		url.into_url().unwrap()
	}
}

fn push_url_arg(url: &mut String, name: &str, value: &str) {
	url.push('&');
	url.push_str(name);
	url.push('=');
	url.push_str(value);
}

fn decode_response(value: BValue) -> Result<Response, &'static str> {
	let mut dict = try!(value.get_dict().ok_or("not a dict"));
	
	let interval = try!(dict
		.remove(&b"interval"[..])
		.and_then(|x| x.get_int())
		.ok_or("missing interval")
		.and_then(|x| if x >= 0 {
			Ok(x as u64)
		} else {
			Err("negative interval")
		}));

	let peers = try!(dict
		.remove(&b"peers"[..])
		.ok_or("missing peers")
		.and_then(decode_peers));

	Ok(Response {
		query_interval: interval,
		peers: peers,
	})
}

fn decode_peers(value: BValue) -> Result<Vec<(Ipv6Addr, u16)>, &'static str> {
	match value {
		BValue::List(list) => {
			let mut peers = Vec::new();
			for peer in list {
				peers.push(try!(decode_peer(peer)));
			}
			Ok(peers)
		}
		BValue::Str(s) => {
			if s.len() % 6 != 0 {
				return Err("bad packed peer list string length");
			}
			let peer_count = s.len() / 6;
			let mut peers = Vec::new();
			for i in 0..peer_count {
				let ip1 = s[i * 6 + 0] as u32;
				let ip2 = s[i * 6 + 1] as u32;
				let ip3 = s[i * 6 + 2] as u32;
				let ip4 = s[i * 6 + 3] as u32;
				let port1 = s[i * 6 + 4] as u16;
				let port2 = s[i * 6 + 5] as u16;
				let ip = (ip1 << 24) | (ip2 << 16) | (ip3 << 8) | ip4;
				let ip = Ipv4Addr::from(ip).to_ipv6_mapped();
				let port = (port1 << 8) | port2;
				peers.push((ip, port));
			}
			Ok(peers)
		}
		_ => {
			Err("bad peer list format")
		}
	}
}

fn decode_peer(value: BValue) -> Result<(Ipv6Addr, u16), &'static str> {
	let mut dict = try!(value.get_dict().ok_or("bad peer"));

	let ip = try!(dict
		.remove(&b"ip"[..])
		.and_then(BValue::get_string)
		.ok_or("missing peer ip")
		.and_then(|ip| {
			use std::str::FromStr;
			let ip_string = String::from_utf8_lossy(&ip);
			let ipv4: Option<Ipv4Addr> = Ipv4Addr::from_str(&ip_string).ok();
			let ipv6: Option<Ipv6Addr> = Ipv6Addr::from_str(&ip_string).ok();
			ipv4.map(|x| x.to_ipv6_mapped()).or(ipv6).ok_or("bad peer ip")
		}));

	let port = try!(dict
		.remove(&b"port"[..])
		.and_then(|x| x.get_int())
		.ok_or("missing peer port")
		.and_then(|port| if port >= 0 && port < ::std::u16::MAX as i64 {
			Ok(port as u16)
		} else {
			Err("bad peer port")
		}));

	Ok((ip, port))
}
