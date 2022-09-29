#[derive(Debug)]
#[derive(PartialEq)]
pub struct DictEntry {
    key: String,
    value: Element,
}

#[derive(Debug)]
#[derive(PartialEq)]
pub enum Element {
    ByteString(String),
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

fn parse_bytestring(data: &str) -> ParseResult {
    match data.find(":") {
	Some(size_end) => {
	    match data[0..size_end].parse::<usize>() {
		Ok(length) => {
		    let parse_start = size_end + 1;
		    let parse_end = parse_start + length;
		    let element = data[parse_start..parse_end].to_string();

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

fn parse_integer(data: &str) -> ParseResult {
    if &data[0..1] != "i" { // this looks stupid it can't be right...
	return ParseResult::Err("Can't parse integer: missing leading 'i'");
    }

    match data.find("e") {
	Some(integer_end) => {
	    match data[1..integer_end].parse::<i64>() {
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

fn parse_list(data: &str) -> ParseResult {
    let mut offset = 0;
    let mut ret: Vec<Element> = Vec::new();

    if &data[0..1] != "l" {
	return ParseResult::Err("Can't parse list: missing leading 'l'");
    }
    offset += 1; // trim leading l

    while offset < data.len() {
	match &data[offset..offset+1] {
	    "e" => return ParseResult::Ok(ElementParsed{ 
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

fn parse_dict(data: &str) -> ParseResult {
    let mut offset = 0;
    let mut ret: Vec<DictEntry> = Vec::new();

    if &data[0..1] != "d" {
	return ParseResult::Err("Can't parse list: missing leading 'd'");
    }
    offset += 1;

    while offset < data.len() {
	match &data[offset..offset+1] {
	    "e" => return ParseResult::Ok(ElementParsed{
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

fn dispatch(data: &str) -> ParseResult {
    if data.len() <= 0 {
	return ParseResult::None;
    }

    match &data[0..1] {
	"0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => parse_bytestring(data),
	"i" => parse_integer(data),
	"l" => parse_list(data),
	"d" => parse_dict(data),
	_ => ParseResult::Err("Unable to continue parsing: can't determine where to dispatch"),
    }
}

pub fn parse(data: &str) -> ParsedDocument {
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
    fn parse_string_happy_path() {
	let input = "0:";
	let result = parse_bytestring(input);
	assert_eq!(result, ParseResult::Ok(ElementParsed{
	    element: Element::ByteString("".to_string()),
	    end_offset: input.len(),
	}));

	let input = "8:announce";
	let result = parse_bytestring(input);
	assert_eq!(result, ParseResult::Ok(ElementParsed{
	    element: Element::ByteString("announce".to_string()),
	    end_offset: input.len(),
	}));

	let input = "41:http://bttracker.debian.org:6969/announce";
	let result = parse_bytestring(input);
	assert_eq!(result, ParseResult::Ok(ElementParsed{
	    element: Element::ByteString("http://bttracker.debian.org:6969/announce".to_string()),
	    end_offset: input.len(),
	}));

	let input = "8:announce41:http://bttracker.debian.org:6969/announce7:comment";
	let result = parse_bytestring(input);
	assert_eq!(result, ParseResult::Ok(ElementParsed{
	    element: Element::ByteString("announce".to_string()),
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
	let input = "i10e";
	let result = parse_integer(input);
	assert_eq!(result, ParseResult::Ok(ElementParsed{
	    element: Element::Integer(10),
	    end_offset: input.len(),
	}));

	let input = "i-10e";
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
	let input = "le";
	let result = parse_list(input);
	assert_eq!(
	    result,
	    ParseResult::Ok(ElementParsed{
		element: Element::List(vec![]),
		end_offset: input.len(),
	    })
	);

	let input = "li10ei1ee";
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

	let input = "li10ei1ee1:a";
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

	let input = "li10ei1el1:bee1:a";
	let result = parse_list(input);
	assert_eq!(
	    result,
	    ParseResult::Ok(ElementParsed{
		element: Element::List(vec![
		    Element::Integer(10),
		    Element::Integer(1),
		    Element::List(vec![
			Element::ByteString("b".to_string()),
		    ]),
		]),
		end_offset: input.len() - "1:a".len(),
	    })
	);
    }

    #[test]
    fn parse_dict_happy_path() {
	let input = "de";
	let result = parse_dict(input);
	assert_eq!(
	    result,
	    ParseResult::Ok(ElementParsed{
		element: Element::Dict(vec![]),
		end_offset: input.len(),
	    })
	);

	let input = "d1:ai10ee";
	let result = parse_dict(input);
	assert_eq!(
	    result,
	    ParseResult::Ok(ElementParsed{
		element: Element::Dict(vec![
		    DictEntry{ key: "a".to_string(), value: Element::Integer(10) }
		]),
		end_offset: input.len(),
	    })
	);

	let input = "d4:listli10ei1el1:beee1:a";
	let result = parse_dict(input);
	assert_eq!(
	    result,
	    ParseResult::Ok(ElementParsed{
		element: Element::Dict(vec![
		    DictEntry{
			key: "list".to_string(),
			value: Element::List(vec![
			    Element::Integer(10),
			    Element::Integer(1),
			    Element::List(vec![
				Element::ByteString("b".to_string()),
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
	let input = "8:announce";
	let result: ParseResult = dispatch(input);
	assert_eq!(result, ParseResult::Ok(ElementParsed{
	    element: Element::ByteString("announce".to_string()),
	    end_offset: input.len(),
	}));

	let input = "i-18e";
	let result: ParseResult = dispatch(input);
	assert_eq!(result, ParseResult::Ok(ElementParsed{
	    element: Element::Integer(-18),
	    end_offset: input.len(),
	}));
    }

    #[test]
    fn parse_happy_path() {
	assert_eq!(
	    parse("8:announce"),
	    ParsedDocument::Ok(vec![
		Element::ByteString("announce".to_string())
	    ])
	);

	assert_eq!(
	    parse("i-18e"),
	    ParsedDocument::Ok(vec![
		Element::Integer(-18)
	    ])
	);

	assert_eq!(
	    parse("8:announce41:http://bttracker.debian.org:6969/announce7:comment35:\"Debian CD from cdimage.debian.org\"10:created by"),
	    ParsedDocument::Ok(vec![
		Element::ByteString("announce".to_string()),
		Element::ByteString("http://bttracker.debian.org:6969/announce".to_string()),
		Element::ByteString("comment".to_string()),
		Element::ByteString("\"Debian CD from cdimage.debian.org\"".to_string()),
		Element::ByteString("created by".to_string()),
	    ])
	);

	assert_eq!(
	    parse("li10ei1el1:bee1:a"),
	    ParsedDocument::Ok(vec![
		Element::List(vec![
		    Element::Integer(10),
		    Element::Integer(1),
		    Element::List(vec![
			Element::ByteString("b".to_string()),
		    ]),
		]),
		Element::ByteString("a".to_string()),
	    ])
	);

	assert_eq!(
	    parse("d8:announce41:http://bttracker.debian.org:6969/announce7:comment35:\"Debian CD from cdimage.debian.org\"10:created by13:mktorrent 1.113:creation datei1662813552ee"),
	    ParsedDocument::Ok(vec![
		Element::Dict(vec![
		    DictEntry{
			key: "announce".to_string(),
			value: Element::ByteString("http://bttracker.debian.org:6969/announce".to_string())
		    },
		    DictEntry{
			key: "comment".to_string(),
			value: Element::ByteString("\"Debian CD from cdimage.debian.org\"".to_string())
		    },
		    DictEntry{
			key: "created by".to_string(),
			value: Element::ByteString("mktorrent 1.1".to_string())
		    },
		    DictEntry{
			key: "creation date".to_string(),
			value: Element::Integer(1662813552)
		    },
		]),
	    ])
	);
    }
}
