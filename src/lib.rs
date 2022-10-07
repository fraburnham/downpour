use base64;

#[derive(Debug)]
#[derive(PartialEq)]
pub enum DecodeErrorType {
    DispatchFailed,
    InvalidByteStringSize,
    InvalidByteStringData,
    InvalidDictKey,
    InvalidIntegerValue,
    MissingDelimiter, // :
    MissingEndDelimiter, // e
    MissingStartDelimiter, // l,d,i
    NothingToDecode,
}

#[derive(Debug)]
#[derive(PartialEq)]
pub struct DecodeError {
    offset: usize, // each error catcher needs to adjust the offset based on the relative value they see
    msg: &'static str,
    error_type: DecodeErrorType,
}

#[derive(Debug)]
#[derive(PartialEq)]
pub struct DictEntry {
    key: Vec<u8>,
    value: Element,
}

#[derive(Debug)]
#[derive(PartialEq)]
pub enum Element {
    // lifecycles could make using a &[u8] possible here
    ByteString(Vec<u8>), // ascii or byte
    Integer(i64),
    List(Vec<Element>),
    Dict(Vec<DictEntry>),
}

#[derive(Debug)]
#[derive(PartialEq)]
struct ElementDecoded {
    element: Element,
    end_offset: usize,
}

#[derive(Debug)]
#[derive(PartialEq)]
enum DecodeResult {
    Ok(ElementDecoded),
    Err(DecodeError)
}

#[derive(Debug)]
#[derive(PartialEq)]
pub enum DecodedDocument {
    Ok(Vec<Element>),
    Err(DecodeError),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
	write!(f, "ERROR! offset: {}, error type: {:?}\nmessage:\n{}", self.offset, self.error_type, self.msg)
    }
}

impl std::fmt::Display for DictEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
	// dict keys are printable ascii or an error
	write!(f, "\"{}\": {}", String::from_utf8(self.key.to_vec()).unwrap(), self.value)
    }
}

impl std::fmt::Display for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
	match self {
	    Element::ByteString(s) => {
		write!(f, "\"{}\"",
		       match String::from_utf8(s.to_vec()) {
			   Ok(s) => s.replace("\"", "\\\""),
			   Err(_) => format!("{}", base64::encode(s)),
		       }
		)
	    },
	    Element::Integer(i) => write!(f, "{}", i),
	    Element::List(list) => {
		// TODO: De-dup! Is this a macro?
		let mut formatted = String::new();

		formatted.push('[');
		for entry in list {
		    formatted.push_str(format!("{}, ", entry).as_str()); // is this cast dumb?
		}
		formatted.pop(); formatted.pop(); // remove trailing `, `
		formatted.push(']');

		write!(f, "{}", formatted)
	    },
	    Element::Dict(dict) => {
		let mut formatted = String::new();

		formatted.push('{');
		for entry in dict {
		    formatted.push_str(format!("{}, ", entry).as_str()); // is this cast dumb?
		}
		formatted.pop(); formatted.pop(); // remove trailing `, `
		formatted.push('}');

		write!(f, "{}", formatted)
	    },
	}
    }
}

impl std::fmt::Display for DecodedDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
	match self {
	    DecodedDocument::Ok(elements) => {
		let mut res: std::fmt::Result = write!(f, "");
		
		for el in elements {
		    res = write!(f, "{}", el);
		}

		res
	    },
	    DecodedDocument::Err(e) => {
		write!(f, "{}", e)
	    }
	}
    }
}

const COLON: &u8 = &b':';
const MINUS: &u8 = &b'-';
const D: &u8 = &b'd';
const E: &u8 = &b'e';
const I: &u8 = &b'i';
const L: &u8 = &b'l';

fn decode_ascii_integer(data: &[u8]) -> Result<i64, DecodeError> {
    let mut negative = false;
    let mut val: i64 = 0;
    let mut iter = data.iter();

    if let Some(n) = iter.next() {
	if n == MINUS {
	    // consume the byte
	    negative = true;
	} else {
	    // reset the iterator
	    iter = data.iter();
	}
    }

    // there is a failure case when there is nothing to decode, but this doesn't account for only having a `-`...
    if data.len() == 0 {
	return Err(DecodeError{
	    msg: "Nothing to decode",
	    offset: 0,
	    error_type: DecodeErrorType::NothingToDecode,
	});
    }

    for (index, n) in iter.rev().enumerate() {
	// guard for index greater than u32?
	// more general guard against integer overflow?
	val += ((n - 0x30) as i64) * 10i64.pow(index.try_into().unwrap());
    }

    if negative {
	val = -val;
    }
    
    return Ok(val);
}

fn decode_bytestring(data: &[u8]) -> DecodeResult {
    // wtf is this position syntax?
    match data.iter().position(|d_var| d_var == COLON) {
	Some(size_end) => {
	    match decode_ascii_integer(&data[0..size_end]) {
		Ok(length) => {
		    let decode_start: usize = size_end + 1;
		    let decode_end: usize = decode_start + (length as usize);
		    let mut element: Vec<u8> = Vec::new();
		    element.extend_from_slice(&data[decode_start..decode_end]);

		    DecodeResult::Ok(ElementDecoded{
			element: Element::ByteString(element),
			end_offset: decode_end,
		    })
		},

		Err(e) => DecodeResult::Err(DecodeError{
		    msg: "Can't decode size integer for string",
		    offset: e.offset,
		    error_type: DecodeErrorType::InvalidByteStringSize,
		}),
	    }
	},
	None => DecodeResult::Err(DecodeError{
	    msg: "Can't decode bytestring from data: missing ':'",
	    offset: 0,
	    error_type: DecodeErrorType::MissingDelimiter,
	}),
    }
}

fn decode_integer(data: &[u8]) -> DecodeResult {
    if &data[0] != I {
	return DecodeResult::Err(DecodeError{
	    msg: "Can't decode integer: missing leading 'i'",
	    offset: 0,
	    error_type: DecodeErrorType::MissingStartDelimiter,
	});
    }

    match data.iter().position(|d_var| d_var == E) {
	Some(integer_end) => {
	    match decode_ascii_integer(&data[1..integer_end]) {
		Ok(element) => {
		    DecodeResult::Ok(ElementDecoded{
			element: Element::Integer(element),
			end_offset: integer_end + 1,
		    })
		},

		Err(_) => DecodeResult::Err(DecodeError{
		    msg: "Can't decode integer",
		    offset: 0,
		    error_type: DecodeErrorType::InvalidIntegerValue,
		})
	    }
	},
	    
	None => DecodeResult::Err(DecodeError{
	    msg: "Can't decode integer: missing end 'e'",
	    offset: 0,
	    error_type: DecodeErrorType::MissingEndDelimiter,
	}),
    }
}

fn decode_list(data: &[u8]) -> DecodeResult {
    let mut offset = 0;
    let mut ret: Vec<Element> = Vec::new();

    if &data[0] != L {
	return DecodeResult::Err(DecodeError{
	    msg: "Can't decode list: missing leading 'l'",
	    offset: offset,
	    error_type: DecodeErrorType::MissingStartDelimiter,
	});
    }
    offset += 1; // trim leading l

    while offset < data.len() {
	match &data[offset] {
	    E => return DecodeResult::Ok(ElementDecoded{ 
		element: Element::List(ret),
		end_offset: offset + 1, // trim trailling 'e'
	    }), 
	    _ => {
		match dispatch(&data[offset..]) {
		    DecodeResult::Ok(result) => {
			ret.push(result.element);
			offset += result.end_offset;
		    },

		    DecodeResult::Err(e) => return DecodeResult::Err(DecodeError{
			msg: e.msg,
			offset: e.offset + offset,
			error_type: e.error_type,
		    }),
		}
	    },
	}
    }

    return DecodeResult::Err(DecodeError{
	msg: "Can't decode list: ran out of chars before trailing 'e'",
	offset: offset,
	error_type: DecodeErrorType::MissingEndDelimiter,
    });
}

fn decode_dict(data: &[u8]) -> DecodeResult {
    let mut offset = 0;
    let mut ret: Vec<DictEntry> = Vec::new();

    if &data[0] != D {
	return DecodeResult::Err(DecodeError{
	    msg: "Can't decode list: missing leading 'd'",
	    offset: offset,
	    error_type: DecodeErrorType::MissingStartDelimiter,
	});
    }
    offset += 1;

    while offset < data.len() {
	match &data[offset] {
	    E => return DecodeResult::Ok(ElementDecoded{
		element: Element::Dict(ret),
		end_offset: offset + 1, // trim trailing 'e'
	    }),
	    _ => {
		match decode_bytestring(&data[offset..]) {
		    DecodeResult::Ok(decode_key) => {
			match decode_key.element {
			    Element::ByteString(key) => {
				match dispatch(&data[offset+decode_key.end_offset..]) {
				    DecodeResult::Ok(result) => {
					ret.push(DictEntry{ key: key, value: result.element });
					offset += result.end_offset + decode_key.end_offset
				    },

				    DecodeResult::Err(e) => return DecodeResult::Err(DecodeError{
					msg: e.msg,
					offset: e.offset + offset,
					error_type: e.error_type,
				    }),
				}
			    },

			    _ => return DecodeResult::Err(DecodeError{
				msg: "Can't decode dict key: got non bytestring element",
				offset: offset,
				error_type: DecodeErrorType::InvalidDictKey,
			    })
			}
		    },

		    DecodeResult::Err(e) => {
			return DecodeResult::Err(DecodeError{
			    msg: e.msg,
			    offset: e.offset + offset,
			    error_type: e.error_type,
			})
		    },
		}
	    },
	}
    }

    return DecodeResult::Err(DecodeError{
	msg: "Can't decode dict: ran out of chars",
	offset: offset,
	error_type: DecodeErrorType::MissingEndDelimiter,
    });
}

fn dispatch(data: &[u8]) -> DecodeResult {
    match &data[0] {
	0x30 ..= 0x39 => decode_bytestring(data), // 0 - 9 in ascii
	I => decode_integer(data),
	L => decode_list(data),
	D => decode_dict(data),
	_ => DecodeResult::Err(DecodeError{
	    msg: "Unable to continue parsing: can't determine where to dispatch",
	    offset: 0,
	    error_type: DecodeErrorType::DispatchFailed,
	}),
    }
}

pub fn decode(data: &[u8]) -> DecodedDocument {
    let mut offset = 0;
    let mut ret: Vec<Element> = Vec::new();

    while offset < data.len() {
	match dispatch(&data[offset..]) {
	    DecodeResult::Ok(result) => {
		ret.push(result.element);
		offset += result.end_offset;
	    },

	    DecodeResult::Err(e) => return DecodedDocument::Err(DecodeError{
		msg: e.msg,
		offset: e.offset + offset,
		error_type: e.error_type,
	    }),
	}
    }

    return DecodedDocument::Ok(ret);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_ascii_integer_happy_path() {
	assert_eq!(decode_ascii_integer(b"-134").unwrap(), -134);
	assert_eq!(decode_ascii_integer(b"134").unwrap(), 134);
	assert_eq!(decode_ascii_integer(b"0").unwrap(), 0);
	assert_eq!(decode_ascii_integer(b"12345678").unwrap(), 12345678);
	assert_eq!(decode_ascii_integer(b"-12345678").unwrap(), -12345678);
    }
    
    #[test]
    fn decode_string_happy_path() {
	let input = b"0:";
	let result = decode_bytestring(input);
	assert_eq!(result, DecodeResult::Ok(ElementDecoded{
	    element: Element::ByteString(vec![]),
	    end_offset: input.len(),
	}));

	let input = b"8:announce";
	let result = decode_bytestring(input);
	assert_eq!(result, DecodeResult::Ok(ElementDecoded{
	    element: Element::ByteString("announce".as_bytes().to_vec()),
	    end_offset: input.len(),
	}));

	let input = b"41:http://bttracker.debian.org:6969/announce";
	let result = decode_bytestring(input);
	assert_eq!(result, DecodeResult::Ok(ElementDecoded{
	    element: Element::ByteString("http://bttracker.debian.org:6969/announce".as_bytes().to_vec()),
	    end_offset: input.len(),
	}));

	let input = b"8:announce41:http://bttracker.debian.org:6969/announce7:comment";
	let result = decode_bytestring(input);
	assert_eq!(result, DecodeResult::Ok(ElementDecoded{
	    element: Element::ByteString("announce".as_bytes().to_vec()),
	    end_offset: input.len() - "41:http://bttracker.debian.org:6969/announce7:comment".len(),
	}));
    }

    #[test]
    fn decode_integer_happy_path() {
	let input = b"i10e";
	let result = decode_integer(input);
	assert_eq!(result, DecodeResult::Ok(ElementDecoded{
	    element: Element::Integer(10),
	    end_offset: input.len(),
	}));

	let input = b"i-10e";
	let result = decode_integer(input);
	assert_eq!(result, DecodeResult::Ok(ElementDecoded{
	    element: Element::Integer(-10),
	    end_offset: input.len(),
	}));
    }

    #[test]
    fn decode_list_happy_path() {
	let input = b"le";
	let result = decode_list(input);
	assert_eq!(
	    result,
	    DecodeResult::Ok(ElementDecoded{
		element: Element::List(vec![]),
		end_offset: input.len(),
	    })
	);

	let input = b"li10ei1ee";
	let result = decode_list(input);
	assert_eq!(
	    result,
	    DecodeResult::Ok(ElementDecoded{
		element: Element::List(vec![
		    Element::Integer(10),
		    Element::Integer(1)
		]),
		end_offset: input.len(),
	    })
	);

	let input = b"li10ei1ee1:a";
	let result = decode_list(input);
	assert_eq!(
	    result,
	    DecodeResult::Ok(ElementDecoded{
		element: Element::List(vec![
		    Element::Integer(10),
		    Element::Integer(1)
		]),
		end_offset: input.len() - "1:a".len()
	    })
	);

	let input = b"li10ei1el1:bee1:a";
	let result = decode_list(input);
	assert_eq!(
	    result,
	    DecodeResult::Ok(ElementDecoded{
		element: Element::List(vec![
		    Element::Integer(10),
		    Element::Integer(1),
		    Element::List(vec![
			Element::ByteString(b"b".to_vec()),
		    ]),
		]),
		end_offset: input.len() - "1:a".len(),
	    })
	);
    }

    #[test]
    fn decode_dict_happy_path() {
	let input = b"de";
	let result = decode_dict(input);
	assert_eq!(
	    result,
	    DecodeResult::Ok(ElementDecoded{
		element: Element::Dict(vec![]),
		end_offset: input.len(),
	    })
	);

	let input = b"d1:ai10ee";
	let result = decode_dict(input);
	assert_eq!(
	    result,
	    DecodeResult::Ok(ElementDecoded{
		element: Element::Dict(vec![
		    DictEntry{ key: b"a".to_vec(), value: Element::Integer(10) }
		]),
		end_offset: input.len(),
	    })
	);

	let input = b"d4:listli10ei1el1:beee1:a";
	let result = decode_dict(input);
	assert_eq!(
	    result,
	    DecodeResult::Ok(ElementDecoded{
		element: Element::Dict(vec![
		    DictEntry{
			key: b"list".to_vec(),
			value: Element::List(vec![
			    Element::Integer(10),
			    Element::Integer(1),
			    Element::List(vec![
				Element::ByteString(b"b".to_vec()),
			    ]),
			])
		    }
		]),
		end_offset: input.len() - "1:a".len(),
	    })
	);
    }
    
    #[test]
    fn dispatch_happy_path() {
	let input = b"8:announce";
	let result: DecodeResult = dispatch(input);
	assert_eq!(result, DecodeResult::Ok(ElementDecoded{
	    element: Element::ByteString(b"announce".to_vec()),
	    end_offset: input.len(),
	}));

	let input = b"i-18e";
	let result: DecodeResult = dispatch(input);
	assert_eq!(result, DecodeResult::Ok(ElementDecoded{
	    element: Element::Integer(-18),
	    end_offset: input.len(),
	}));
    }

    #[test]
    fn decode_happy_path() {
	assert_eq!(
	    decode(b"8:announce"),
	    DecodedDocument::Ok(vec![
		Element::ByteString("announce".as_bytes().to_vec())
	    ])
	);

	assert_eq!(
	    decode(b"i-18e"),
	    DecodedDocument::Ok(vec![
		Element::Integer(-18)
	    ])
	);

	assert_eq!(
	    decode(b"8:announce41:http://bttracker.debian.org:6969/announce7:comment35:\"Debian CD from cdimage.debian.org\"10:created by"),
	    DecodedDocument::Ok(vec![
		Element::ByteString(b"announce".to_vec()),
		Element::ByteString(b"http://bttracker.debian.org:6969/announce".to_vec()),
		Element::ByteString(b"comment".to_vec()),
		Element::ByteString(b"\"Debian CD from cdimage.debian.org\"".to_vec()),
		Element::ByteString(b"created by".to_vec()),
	    ])
	);

	assert_eq!(
	    decode(b"li10ei1el1:bee1:a"),
	    DecodedDocument::Ok(vec![
		Element::List(vec![
		    Element::Integer(10),
		    Element::Integer(1),
		    Element::List(vec![
			Element::ByteString(b"b".to_vec()),
		    ]),
		]),
		Element::ByteString(b"a".to_vec()),
	    ])
	);

	assert_eq!(
	    decode(b"d8:announce41:http://bttracker.debian.org:6969/announce7:comment35:\"Debian CD from cdimage.debian.org\"10:created by13:mktorrent 1.113:creation datei1662813552ee"),
	    DecodedDocument::Ok(vec![
		Element::Dict(vec![
		    DictEntry{
			key: b"announce".to_vec(),
			value: Element::ByteString(b"http://bttracker.debian.org:6969/announce".to_vec())
		    },
		    DictEntry{
			key: b"comment".to_vec(),
			value: Element::ByteString(b"\"Debian CD from cdimage.debian.org\"".to_vec())
		    },
		    DictEntry{
			key: b"created by".to_vec(),
			value: Element::ByteString(b"mktorrent 1.1".to_vec())
		    },
		    DictEntry{
			key: b"creation date".to_vec(),
			value: Element::Integer(1662813552)
		    },
		]),
	    ])
	);
    }
}
