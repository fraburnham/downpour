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
struct ElementParsed {
    element: Element,
    end_offset: usize,
}

#[derive(Debug)]
#[derive(PartialEq)]
enum ParseResult {
    Ok(ElementParsed),
    Err(&'static str), // TODO: make errors types and attach more useful info to them
    None,
}

#[derive(Debug)]
#[derive(PartialEq)]
pub enum ParsedDocument {
    Ok(Vec<Element>),
    Err(&'static str),
}

const COLON: &u8 = &":".as_bytes()[0];
const MINUS: &u8 = &"-".as_bytes()[0];
const D: &u8 = &"d".as_bytes()[0];
const E: &u8 = &"e".as_bytes()[0];
const I: &u8 = &"i".as_bytes()[0];
const L: &u8 = &"l".as_bytes()[0];

fn parse_ascii_integer(data: &[u8]) -> Result<i64, &'static str> { // figure out how to let the caller specify the return type?
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

    // there is a failure case when there is nothing to parse, but this doesn't account for only having a `-`...
    if data.len() == 0 {
	return Err("Nothing to parse");
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

fn parse_bytestring(data: &[u8]) -> ParseResult {
    // wtf is this position syntax?
    match data.iter().position(|d_var| d_var == COLON) {
	Some(size_end) => {
	    match parse_ascii_integer(&data[0..size_end]) {
		Ok(length) => {
		    let parse_start: usize = size_end + 1;
		    let parse_end: usize = parse_start + (length as usize);
		    let mut element: Vec<u8> = Vec::new();
		    element.extend_from_slice(&data[parse_start..parse_end]);

		    ParseResult::Ok(ElementParsed{
			element: Element::ByteString(element),
			end_offset: parse_end,
		    })
		},

		Err(_) => ParseResult::Err("Can't parse size integer for string"),
	    }
	},
	None => ParseResult::Err("Can't parse bytestring from data: missing ':'"),
    }
}

fn parse_integer(data: &[u8]) -> ParseResult {
    if &data[0] != I { // this looks stupid it can't be right...
	return ParseResult::Err("Can't parse integer: missing leading 'i'");
    }

    match data.iter().position(|d_var| d_var == E) {
	Some(integer_end) => {
	    match parse_ascii_integer(&data[1..integer_end]) {
		Ok(element) => {
		    ParseResult::Ok(ElementParsed{
			element: Element::Integer(element),
			end_offset: integer_end + 1,
		    })
		},

		Err(_) => ParseResult::Err("Can't parse integer")
	    }
	},
	    
	None => ParseResult::Err("Can't parse integer: missing end 'e'"),
    }
}

fn parse_list(data: &[u8]) -> ParseResult {
    let mut offset = 0;
    let mut ret: Vec<Element> = Vec::new();

    if &data[0] != L {
	return ParseResult::Err("Can't parse list: missing leading 'l'");
    }
    offset += 1; // trim leading l

    while offset < data.len() {
	match &data[offset] { // if I use a byte array this can be array index?
	    E => return ParseResult::Ok(ElementParsed{ 
		element: Element::List(ret),
		end_offset: offset + 1, // trim trailling 'e'
	    }), 
	    _ => {
		match dispatch(&data[offset..]) {
		    ParseResult::Ok(result) => {
			ret.push(result.element);
			offset += result.end_offset;
		    },

		    ParseResult::Err(e) => return ParseResult::Err(e),
		
		    ParseResult::None => return ParseResult::Err("Can't parse list: dispatch parse 'None'"),
		}
	    },
	}
    }

    return ParseResult::Err("Can't parse list: ran out of chars before trailing 'e'");
}

fn parse_dict(data: &[u8]) -> ParseResult {
    let mut offset = 0;
    let mut ret: Vec<DictEntry> = Vec::new();

    if &data[0] != D {
	return ParseResult::Err("Can't parse list: missing leading 'd'");
    }
    offset += 1;

    while offset < data.len() {
	match &data[offset] {
	    E => return ParseResult::Ok(ElementParsed{
		element: Element::Dict(ret),
		end_offset: offset + 1, // trim trailing 'e'
	    }),
	    _ => {
		match parse_bytestring(&data[offset..]) {
		    ParseResult::Ok(parse_key) => {
			match parse_key.element {
			    Element::ByteString(key) => {
				match dispatch(&data[offset+parse_key.end_offset..]) {
				    ParseResult::Ok(result) => {
					// need to convert the bytestring to a real string
					ret.push(DictEntry{ key: key, value: result.element });
					offset += result.end_offset + parse_key.end_offset
				    },

				    ParseResult::Err(e) => return ParseResult::Err(e),

				    ParseResult::None => return ParseResult::Err("Can't parse list: dispatch parse 'None'"),
				}
			    },

			    _ => return ParseResult::Err("Can't parse dict key: got non bytestring element")
			}
		    },

		    _ => return ParseResult::Err("Can't parse dict key!")
		}
	    },
	}
    }

    return ParseResult::Err("Can't parse dict: ran out of chars");
}

fn dispatch(data: &[u8]) -> ParseResult {
    if data.len() <= 0 {
	return ParseResult::None;
    }

    match &data[0] {
	0x30 ..= 0x39 => parse_bytestring(data),
	I => parse_integer(data),
	L => parse_list(data),
	D => parse_dict(data),
	_ => ParseResult::Err("Unable to continue parsing: can't determine where to dispatch"),
    }
}

pub fn parse(data: &[u8]) -> ParsedDocument {
    let mut offset = 0;
    let mut ret: Vec<Element> = Vec::new();

    while offset < data.len() {
	match dispatch(&data[offset..]) {
	    ParseResult::Ok(result) => {
		ret.push(result.element);
		offset += result.end_offset;
	    },

	    ParseResult::Err(e) => return ParsedDocument::Err(e),

	    ParseResult::None => return ParsedDocument::Err("Unexpected end of input"), // make a test that can cause this state...
	}
    }

    return ParsedDocument::Ok(ret);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ascii_integer_happy_path() {
	assert_eq!(parse_ascii_integer("-134".as_bytes()).unwrap(), -134);
	assert_eq!(parse_ascii_integer("134".as_bytes()).unwrap(), 134);
	assert_eq!(parse_ascii_integer("0".as_bytes()).unwrap(), 0);
	assert_eq!(parse_ascii_integer("12345678".as_bytes()).unwrap(), 12345678);
	assert_eq!(parse_ascii_integer("-12345678".as_bytes()).unwrap(), -12345678);
    }
    
    #[test]
    fn parse_string_happy_path() {
	let input = "0:".as_bytes();
	let result = parse_bytestring(input);
	assert_eq!(result, ParseResult::Ok(ElementParsed{
	    element: Element::ByteString(vec![]),
	    end_offset: input.len(),
	}));

	let input = "8:announce".as_bytes();
	let result = parse_bytestring(input);
	assert_eq!(result, ParseResult::Ok(ElementParsed{
	    element: Element::ByteString("announce".as_bytes().to_vec()),
	    end_offset: input.len(),
	}));

	let input = "41:http://bttracker.debian.org:6969/announce".as_bytes();
	let result = parse_bytestring(input);
	assert_eq!(result, ParseResult::Ok(ElementParsed{
	    element: Element::ByteString("http://bttracker.debian.org:6969/announce".as_bytes().to_vec()),
	    end_offset: input.len(),
	}));

	let input = "8:announce41:http://bttracker.debian.org:6969/announce7:comment".as_bytes();
	let result = parse_bytestring(input);
	assert_eq!(result, ParseResult::Ok(ElementParsed{
	    element: Element::ByteString("announce".as_bytes().to_vec()),
	    end_offset: input.len() - "41:http://bttracker.debian.org:6969/announce7:comment".len(),
	}));
    }

    #[test]
    fn parse_string_missing_colon() {
	// TODO! How to assert return type?
    }

    #[test]
    fn parse_string_fail_to_parse_size() {
	// TODO!
    }

    #[test]
    fn parse_integer_happy_path() {
	let input = "i10e".as_bytes();
	let result = parse_integer(input);
	assert_eq!(result, ParseResult::Ok(ElementParsed{
	    element: Element::Integer(10),
	    end_offset: input.len(),
	}));

	let input = "i-10e".as_bytes();
	let result = parse_integer(input);
	assert_eq!(result, ParseResult::Ok(ElementParsed{
	    element: Element::Integer(-10),
	    end_offset: input.len(),
	}));
    }

    #[test]
    fn parse_integer_missing_leading_i() {
    }

    #[test]
    fn parse_integer_fail_to_parse_int() {
    }

    #[test]
    fn parse_integer_missing_ending_e() {
    }

    #[test]
    fn parse_list_happy_path() {
	let input = "le".as_bytes();
	let result = parse_list(input);
	assert_eq!(
	    result,
	    ParseResult::Ok(ElementParsed{
		element: Element::List(vec![]),
		end_offset: input.len(),
	    })
	);

	let input = "li10ei1ee".as_bytes();
	let result = parse_list(input);
	assert_eq!(
	    result,
	    ParseResult::Ok(ElementParsed{
		element: Element::List(vec![
		    Element::Integer(10),
		    Element::Integer(1)
		]),
		end_offset: input.len(),
	    })
	);

	let input = "li10ei1ee1:a".as_bytes();
	let result = parse_list(input);
	assert_eq!(
	    result,
	    ParseResult::Ok(ElementParsed{
		element: Element::List(vec![
		    Element::Integer(10),
		    Element::Integer(1)
		]),
		end_offset: input.len() - "1:a".len()
	    })
	);

	let input = "li10ei1el1:bee1:a".as_bytes();
	let result = parse_list(input);
	assert_eq!(
	    result,
	    ParseResult::Ok(ElementParsed{
		element: Element::List(vec![
		    Element::Integer(10),
		    Element::Integer(1),
		    Element::List(vec![
			Element::ByteString("b".as_bytes().to_vec()),
		    ]),
		]),
		end_offset: input.len() - "1:a".len(),
	    })
	);
    }

    #[test]
    fn parse_dict_happy_path() {
	let input = "de".as_bytes();
	let result = parse_dict(input);
	assert_eq!(
	    result,
	    ParseResult::Ok(ElementParsed{
		element: Element::Dict(vec![]),
		end_offset: input.len(),
	    })
	);

	let input = "d1:ai10ee".as_bytes();
	let result = parse_dict(input);
	assert_eq!(
	    result,
	    ParseResult::Ok(ElementParsed{
		element: Element::Dict(vec![
		    DictEntry{ key: "a".as_bytes().to_vec(), value: Element::Integer(10) }
		]),
		end_offset: input.len(),
	    })
	);

	let input = "d4:listli10ei1el1:beee1:a".as_bytes();
	let result = parse_dict(input);
	assert_eq!(
	    result,
	    ParseResult::Ok(ElementParsed{
		element: Element::Dict(vec![
		    DictEntry{
			key: "list".as_bytes().to_vec(),
			value: Element::List(vec![
			    Element::Integer(10),
			    Element::Integer(1),
			    Element::List(vec![
				Element::ByteString("b".as_bytes().to_vec()),
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
	let input = "8:announce".as_bytes();
	let result: ParseResult = dispatch(input);
	assert_eq!(result, ParseResult::Ok(ElementParsed{
	    element: Element::ByteString("announce".as_bytes().to_vec()),
	    end_offset: input.len(),
	}));

	let input = "i-18e".as_bytes();
	let result: ParseResult = dispatch(input);
	assert_eq!(result, ParseResult::Ok(ElementParsed{
	    element: Element::Integer(-18),
	    end_offset: input.len(),
	}));
    }

    #[test]
    fn parse_happy_path() {
	assert_eq!(
	    parse("8:announce".as_bytes()),
	    ParsedDocument::Ok(vec![
		Element::ByteString("announce".as_bytes().to_vec())
	    ])
	);

	assert_eq!(
	    parse("i-18e".as_bytes()),
	    ParsedDocument::Ok(vec![
		Element::Integer(-18)
	    ])
	);

	assert_eq!(
	    parse("8:announce41:http://bttracker.debian.org:6969/announce7:comment35:\"Debian CD from cdimage.debian.org\"10:created by".as_bytes()),
	    ParsedDocument::Ok(vec![
		Element::ByteString("announce".as_bytes().to_vec()),
		Element::ByteString("http://bttracker.debian.org:6969/announce".as_bytes().to_vec()),
		Element::ByteString("comment".as_bytes().to_vec()),
		Element::ByteString("\"Debian CD from cdimage.debian.org\"".as_bytes().to_vec()),
		Element::ByteString("created by".as_bytes().to_vec()),
	    ])
	);

	assert_eq!(
	    parse("li10ei1el1:bee1:a".as_bytes()),
	    ParsedDocument::Ok(vec![
		Element::List(vec![
		    Element::Integer(10),
		    Element::Integer(1),
		    Element::List(vec![
			Element::ByteString("b".as_bytes().to_vec()),
		    ]),
		]),
		Element::ByteString("a".as_bytes().to_vec()),
	    ])
	);

	assert_eq!(
	    parse("d8:announce41:http://bttracker.debian.org:6969/announce7:comment35:\"Debian CD from cdimage.debian.org\"10:created by13:mktorrent 1.113:creation datei1662813552ee".as_bytes()),
	    ParsedDocument::Ok(vec![
		Element::Dict(vec![
		    DictEntry{
			key: "announce".as_bytes().to_vec(),
			value: Element::ByteString("http://bttracker.debian.org:6969/announce".as_bytes().to_vec())
		    },
		    DictEntry{
			key: "comment".as_bytes().to_vec(),
			value: Element::ByteString("\"Debian CD from cdimage.debian.org\"".as_bytes().to_vec())
		    },
		    DictEntry{
			key: "created by".as_bytes().to_vec(),
			value: Element::ByteString("mktorrent 1.1".as_bytes().to_vec())
		    },
		    DictEntry{
			key: "creation date".as_bytes().to_vec(),
			value: Element::Integer(1662813552)
		    },
		]),
	    ])
	);
    }
}
