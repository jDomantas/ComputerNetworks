use std::collections::BTreeMap;

#[derive(Debug, Eq, PartialEq)]
pub enum BValue {
	Int(i64),
	Str(String),
	List(Vec<BValue>),
	Dict(BTreeMap<String, BValue>),
}

macro_rules! bdict {
	( $( $k:expr => $v:expr ),* ) => {{
		let mut m: ::std::collections::BTreeMap<String, BValue> =
			::std::collections::BTreeMap::new();
		$(
			m.insert($k, $v);
		)*
		BValue::Dict(m)
	}};
}

macro_rules! blist {
	( $( $i:expr ),* ) => {{
		let mut v: ::std::vec::Vec<BValue> = ::std::vec::Vec::new();
		$(
			v.push($i);
		)*
		BValue::List(v)
	}};
}

#[derive(Debug, Eq, PartialEq)]
pub enum DecodeError {
	UTF8DecodeError,
	BadFormat,
	NumberTooLarge,
	EarlyEndOfInput,
}

pub type DecodeResult<T> = Result<T, DecodeError>;

pub fn encode(value: &BValue) -> Vec<u8> {
	let mut encoder = Encoder::new();
	encoder.write(value);
	encoder.get_output()
}

pub fn decode(input: &[u8]) -> DecodeResult<BValue> {
	let mut decoder = Decoder::new(input);
	decoder.read()
}

struct Encoder {
	result: Vec<u8>,
}

impl Encoder {
	fn new() -> Encoder {
		Encoder {
			result: Vec::new(),
		}
	}

	fn get_output(self) -> Vec<u8> {
		self.result
	}

	fn write(&mut self, value: &BValue) {
		match value {
			&BValue::Int(i) => self.write_int(i),
			&BValue::Str(ref s) => self.write_string(s),
			&BValue::List(ref l) => self.write_list(l),
			&BValue::Dict(ref d) => self.write_dict(d),
		}
	}

	fn write_byte(&mut self, byte: u8) {
		self.result.push(byte);
	}

	fn write_bytes(&mut self, bytes: &[u8]) {
		self.result.extend_from_slice(bytes);
	}

	fn write_char(&mut self, ch: char) {
		self.write_byte(ch as u8);
	}

	fn write_raw_number(&mut self, i: u64) {
		if i >= 10 {
			self.write_raw_number(i / 10);
		}
		let ch = (i % 10) as u8 + '0' as u8;
		self.write_byte(ch);
	}

	fn write_int(&mut self, i: i64) {
		self.write_char('i');
		if i < 0 {
			self.write_char('-');
			self.write_raw_number(-i as u64);
		} else {
			self.write_raw_number(i as u64);
		}
		self.write_char('e');
	}

	fn write_string(&mut self, s: &str) {
		let bytes = s.as_bytes();
		let length = bytes.len();
		self.write_raw_number(length as u64);
		self.write_char(':');
		self.write_bytes(bytes);
	}

	fn write_list(&mut self, list: &[BValue]) {
		self.write_char('l');
		for item in list {
			self.write(item);
		}
		self.write_char('e');
	}

	fn write_dict(&mut self, dict: &BTreeMap<String, BValue>) {
		self.write_char('d');
		for (key, value) in dict {
			self.write_string(key);
			self.write(value);
		}
		self.write_char('e');
	}
}

struct Decoder<'a> {
	input: &'a [u8],
	position: usize,
}

impl<'a> Decoder<'a> {
	fn new(input: &[u8]) -> Decoder {
		Decoder {
			input: input,
			position: 0,
		}
	}

	fn peek(&self) -> Option<u8> {
		self.input.get(self.position).map(|&x| x)
	}

	fn peek_char(&self) -> Option<char> {
		self.peek().map(|x| x as char)
	}

	fn advance(&mut self) {
		self.position += 1;
	}

	fn match_byte(&mut self, byte: u8) -> bool {
		match self.peek() {
			Some(b) => {
				if b == byte {
					self.advance();
					true
				} else {
					false
				}
			}
			None => false,
		}
	}

	fn match_char(&mut self, ch: char) -> bool {
		self.match_byte(ch as u8)
	}

	fn read(&mut self) -> DecodeResult<BValue> {
		match self.peek_char() {
			Some('i') => self.read_int(),
			Some('l') => self.read_list(),
			Some('d') => self.read_dict(),
			// can we really get a '0' in here?
			Some('0' ... '9') => self.read_string().map(BValue::Str),
			Some(_) => Err(DecodeError::BadFormat),
			None => Err(DecodeError::EarlyEndOfInput),
		}
	}

	fn read_raw_number(&mut self, ends_with: u8) -> DecodeResult<u64> {
		let mut value = 0_u64;
		let mut bytes_consumed = 0_usize;
		loop {
			match self.peek() {
				Some(byte) if byte == ends_with => {
					if bytes_consumed > 0 {
						self.advance();
						return Ok(value);
					} else {
						return Err(DecodeError::BadFormat);
					}
				}
				// 48 is ASCII code for '0', and 57 is ASCII code for '9'
				Some(digit @ 48 ... 57) => {
					self.advance();
					bytes_consumed += 1;
					let digit = digit - ('0' as u8);
					let next_value = value
						.checked_mul(10)
						.and_then(|x| x.checked_add(digit as u64));
					match next_value {
						Some(x) => value = x,
						None => return Err(DecodeError::NumberTooLarge),
					}
				}
				_ => return Err(DecodeError::BadFormat),
			}
		}
	}

	fn read_int(&mut self) -> DecodeResult<BValue> {
		self.advance();
		let negative = self.match_char('-');
		let num = try!(self.read_raw_number('e' as u8));
		if negative {
			// this will disallow getting i64::MIN :/
			if num > ::std::i64::MAX as u64 {
				Err(DecodeError::NumberTooLarge)
			} else {
				Ok(BValue::Int(num as i64 * -1))
			}
		} else {
			if num > ::std::i64::MAX as u64 {
				Err(DecodeError::NumberTooLarge)
			} else {
				Ok(BValue::Int(num as i64))
			}
		}
	}

	fn read_bytes(&mut self, amount: usize) -> DecodeResult<&[u8]> {
		let new_position = self.position + amount;
		if new_position <= self.input.len() {
			let slice = &self.input[self.position..new_position];
			self.position = new_position;
			Ok(slice)
		} else {
			Err(DecodeError::EarlyEndOfInput)
		}
	}

	fn read_string(&mut self) -> DecodeResult<String> {
		let byte_count = try!(self.read_raw_number(':' as u8));
		let bytes = try!(self.read_bytes(byte_count as usize)).to_vec();
		String::from_utf8(bytes)
			.map_err(|_| DecodeError::UTF8DecodeError)
	}

	fn read_list(&mut self) -> DecodeResult<BValue> {
		self.advance();
		let mut items = Vec::new();
		while !self.match_char('e') {
			items.push(try!(self.read()));
		}
		Ok(BValue::List(items))
	}

	fn read_dict(&mut self) -> DecodeResult<BValue> {
		self.advance();
		let mut dict = ::std::collections::BTreeMap::new();
		while !self.match_char('e') {
			let key = try!(self.read_string());
			let value = try!(self.read());
			dict.insert(key, value);
		}
		Ok(BValue::Dict(dict))
	}
}


#[cfg(test)]
mod test {
	use super::BValue;

	pub fn bstr(literal: &'static str) -> BValue {
		BValue::Str(literal.to_owned())
	}

	mod encoding {
		use super::*;
		use super::super::*;

		fn check_encode(value: BValue, expected: &[u8]) {
			let encoded = encode(&value);
			assert_eq!(encoded.as_slice(), expected);
		}

		#[test]
		fn number() {
			check_encode(BValue::Int(7438465982), b"i7438465982e");
		}
		
		#[test]
		fn negative_number() {
			check_encode(BValue::Int(-507), b"i-507e");
		}
		
		#[test]
		fn zero() {
			check_encode(BValue::Int(0), b"i0e");
		}

		#[test]
		fn string() {
			check_encode(bstr("Hello!"), b"6:Hello!");
		}
		
		#[test]
		fn list() {
			check_encode(
				blist![BValue::Int(42), bstr("abc")],
				b"li42e3:abce");
		}

		#[test]
		fn dict() {
			check_encode(
				bdict!(
					"baz".to_owned() => BValue::Int(-9),
					"foo".to_owned() => bstr("bar")
				),
				b"d3:bazi-9e3:foo3:bare");
		}

		#[test]
		fn nested() {
			check_encode(
				bdict![
					"baz".to_owned() => BValue::Int(-9),
					"foo".to_owned() => bstr("bar"),
					"nest!".to_owned() => bdict!(
						"baaaar".to_owned() => bdict!("?".to_owned() => bstr("!")),
						"fooooo".to_owned() => blist![BValue::Int(123456789)]
					)
				],
				b"d3:bazi-9e3:foo3:bar5:nest!d6:baaaard1:?1:!e6:foooooli123456789eeee");
		}
	}

	mod decoding {
		use super::*;
		use super::super::*;

		fn check_decode(expected: BValue, input: &[u8]) {
			let decoded = decode(input);
			assert_eq!(decoded, Ok(expected));
		}

		#[test]
		fn number() {
			check_decode(BValue::Int(7438465982), b"i7438465982e");
		}
		
		#[test]
		fn negative_number() {
			check_decode(BValue::Int(-507), b"i-507e");
		}
		
		#[test]
		fn zero() {
			check_decode(BValue::Int(0), b"i0e");
		}

		#[test]
		fn string() {
			check_decode(bstr("Hello!"), b"6:Hello!");
		}
		
		#[test]
		fn list() {
			check_decode(
				blist![BValue::Int(42), bstr("abc")],
				b"li42e3:abce");
		}

		#[test]
		fn dict() {
			check_decode(
				bdict![
					"baz".to_owned() => BValue::Int(-9),
					"foo".to_owned() => bstr("bar")
				],
				b"d3:bazi-9e3:foo3:bare");
		}

		#[test]
		fn nested() {
			check_decode(
				bdict![
					"baz".to_owned() => BValue::Int(-9),
					"foo".to_owned() => bstr("bar"),
					"nest!".to_owned() => bdict!(
						"baaaar".to_owned() => bdict!("?".to_owned() => bstr("!")),
						"fooooo".to_owned() => blist![BValue::Int(123456789)]
					)
				],
				b"d3:bazi-9e3:foo3:bar5:nest!d6:baaaard1:?1:!e6:foooooli123456789eeee");
		}
	}
}
