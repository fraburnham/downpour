#[derive(Debug)]
#[derive(PartialEq)]
enum Element {
    ByteString(String),
    Integer(i64),
    List(Vec<Element>),
    Dict(Vec<(String, Box<Element>)>), // what is box doing here?
}

#[derive(Debug)]
struct ReadResult {
    element: Element,
    end_offset: usize,
}

// TODO: make errors types and attach more useful info to them
fn read_bytestring(data: &str) -> Result<ReadResult, &'static str> {
    match data.find(":") {
	Some(size_end) => {
	    match data[0..size_end].parse::<usize>() {
		Ok(length) => {
		    let read_start = size_end + 1;
		    let read_end = read_start + length;
		    let element = data[read_start..read_end].to_string();

		    Ok(ReadResult{
			element: Element::ByteString(element),
			end_offset: read_end,
		    })
		},

		Err(_) => Err("Can't parse size integer for string"),
	    }
	},
	None => Err("Can't parse bytestring from data: missing ':'"),
    }
}

fn read_integer(data: &str) -> Result<ReadResult, &'static str> {
    if &data[0..1] != "i" { // this looks stupid it can't be right...
	return Err("Can't parse integer: missing leading 'i'");
    }

    match data.find("e") {
	Some(integer_end) => {
	    match data[1..integer_end].parse::<i64>() {
		Ok(element) => {
		    Ok(ReadResult{
			element: Element::Integer(element),
			end_offset: integer_end + 1,
		    })
		},

		Err(_) => Err("Can't parse integer")
	    }
	},
	    
	None => Err("Can't parse integer: missing end 'e'"),
    }
}

fn read_list(data: &str) -> Result<ReadResult, &'static str> {
    let mut offset = 0;
    let mut ret: Vec<Element> = Vec::new();

    if &data[0..1] != "l" {
	return Err("Can't parse list: missing leading 'l'");
    }
    offset += 1; // trim leading l

    while offset < data.len() {
	match &data[offset..offset+1] {
	    "e" => return Ok(ReadResult{ 
		element: Element::List(ret),
		end_offset: offset + 1, // trim trailling 'e'
	    }), 
	    _ => {
		match dispatch(&data[offset..]) {
		    Some(result) => {
			match result {
			    Ok(result) => {
				ret.push(result.element);
				offset += result.end_offset;
			    },

			    Err(e) => return Err(e)
			}
		    },

		    None => return Err("Can't parse list: dispatch read 'None'")
		}
	    },
	}
    }

    return Err("Can't parse list: ran out of chars before trailing 'e'");
}

fn read_dict(data: &str) -> Result<ReadResult, &'static str> {
    let mut offset = 0;
    let mut ret: Vec<(String, Box<Element>)> = Vec::new();

    if &data[0..1] != "d" {
	return Err("Can't parse list: missing leading 'd'");
    }
    offset += 1;

    while offset < data.len() {
	match &data[offset..offset+1] {
	    "e" => return Ok(ReadResult{
		element: Element::Dict(ret),
		end_offset: offset + 1, // trim trailing 'e'
	    }),
	    _ => {
		// has to be possible to flatten this out...
		// maybe re-use from parse?
		// a type would probably help flatten out the matches, combine the Some and Ok
		// into ReadResult? <Ok | Err | None>
		match read_bytestring(&data[offset..]) {
		    Ok(read_key) => {
			match read_key.element {
			    Element::ByteString(key) => {
				match dispatch(&data[read_key.end_offset+1..]) {
				    Some(result) => {
					match result {
					    Ok(result) => {
						ret.push((key, Box::new(result.element)));
						offset += result.end_offset + read_key.end_offset
					    },

					    Err(e) => return Err(e)
					}
				    },

				    None => return Err("Can't parse list: dispatch read 'None'")
				}
			    },
			    _ => return Err("Can't read dict key: got non bytestring element")
			}
		    },

		    Err(_) => return Err("Can't read dict key!")
		}
	    },
	}
    }

    return Err("Can't parse dict: ran out of chars");
}

fn dispatch(data: &str) -> Option<Result<ReadResult, &'static str>> {
    if data.len() <= 0 {
	return None;
    }

    match &data[0..1] {
	"0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => Some(read_bytestring(data)),
	"i" => Some(read_integer(data)),
	"l" => Some(read_list(data)),
	"d" => Some(read_dict(data)),
	_ => Some(Err("Unable to continue parsing")),
    }
}

fn parse(data: &str) -> Result<Vec<Element>, &'static str> {
    let mut offset = 0;
    let mut ret: Vec<Element> = Vec::new();

    while let Some(result) = dispatch(&data[offset..]) {
	match result {
	    Ok(result) => {
		ret.push(result.element);
		offset += result.end_offset;
	    },

	    Err(e) => return Err(e)
	}
    }

    return Ok(ret);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_string_happy_path() {
	let input = "0:";
	let result = read_bytestring(input).unwrap();
	assert_eq!(result.element, Element::ByteString("".to_string()));
	assert_eq!(&input[result.end_offset..], "");

	let input = "8:announce";
	let result = read_bytestring(input).unwrap();
	assert_eq!(result.element, Element::ByteString("announce".to_string()));
	assert_eq!(&input[result.end_offset..], "");

	let input = "41:http://bttracker.debian.org:6969/announce";
	let result = read_bytestring(input).unwrap();
	assert_eq!(result.element, Element::ByteString("http://bttracker.debian.org:6969/announce".to_string()));
	assert_eq!(&input[result.end_offset..], "");

	let input = "8:announce41:http://bttracker.debian.org:6969/announce7:comment";
	let result = read_bytestring(input).unwrap();
	assert_eq!(result.element, Element::ByteString("announce".to_string()));
	assert_eq!(&input[result.end_offset..], "41:http://bttracker.debian.org:6969/announce7:comment");
    }

    #[test]
    fn read_string_missing_colon() {
	// TODO! How to assert return type?
    }

    #[test]
    fn read_string_fail_to_parse_size() {
	// TODO!
    }

    #[test]
    fn read_integer_happy_path() {
	let input = "i10e";
	let result = read_integer(input).unwrap();
	assert_eq!(result.element, Element::Integer(10));
	assert_eq!(&input[result.end_offset..], "");

	let input = "i-10e";
	let result = read_integer(input).unwrap();
	assert_eq!(result.element, Element::Integer(-10));
	assert_eq!(&input[result.end_offset..], "");
    }

    #[test]
    fn read_integer_missing_leading_i() {
    }

    #[test]
    fn read_integer_fail_to_parse_int() {
    }

    #[test]
    fn read_integer_missing_ending_e() {
    }

    #[test]
    fn read_list_happy_path() {
	let input = "le";
	let result = read_list(input).unwrap();
	assert_eq!(
	    result.element,
	    Element::List(vec![])
	);
	assert_eq!(&input[result.end_offset..], "");

	let input = "li10ei1ee";
	let result = read_list(input).unwrap();
	assert_eq!(
	    result.element,
	    Element::List(vec![
		Element::Integer(10),
		Element::Integer(1)
	    ])
	);
	assert_eq!(&input[result.end_offset..], "");

	let input = "li10ei1ee1:a";
	let result = read_list(input).unwrap();
	assert_eq!(
	    result.element,
	    Element::List(vec![
		Element::Integer(10),
		Element::Integer(1)
	    ])
	);
	assert_eq!(&input[result.end_offset..], "1:a");

	let input = "li10ei1el1:bee1:a";
	let result = read_list(input).unwrap();
	assert_eq!(
	    result.element,
	    Element::List(vec![
		Element::Integer(10),
		Element::Integer(1),
		Element::List(vec![
		    Element::ByteString("b".to_string()),
		]),
	    ])
	);
	assert_eq!(&input[result.end_offset..], "1:a");
    }

    #[test]
    fn read_dict_happy_path() {
	let input = "de";
	let result = read_dict(input).unwrap();
	assert_eq!(
	    result.element,
	    Element::Dict(vec![])
	);
	assert_eq!(&input[result.end_offset..], "");

	let input = "d1:ai10ee";
	let result = read_dict(input).unwrap();
	assert_eq!(
	    result.element,
	    Element::Dict(vec![
		("a".to_string(), Box::new(Element::Integer(10)))
	    ])
	);
	assert_eq!(&input[result.end_offset..], "");

	let input = "d4:listli10ei1el1:beee1:a";
	let result = read_dict(input).unwrap();
	assert_eq!(
	    result.element,
	    Element::Dict(vec![
		(
		    "list".to_string(),
		    Box::new(
			Element::List(vec![
			    Element::Integer(10),
			    Element::Integer(1),
			    Element::List(vec![
				Element::ByteString("b".to_string()),
			    ]),
			])
		    )
		),
	    ])
	);
	assert_eq!(&input[result.end_offset..], "1:a");
    }
    
    #[test]
    fn dispatch_happy_path() {
	let result: ReadResult = dispatch("8:announce").unwrap().unwrap();
	assert!(matches!(result.element, Element::ByteString{ .. }));

	let result: ReadResult = dispatch("i-18e").unwrap().unwrap();
	assert!(matches!(result.element, Element::Integer{ .. }));
    }

    #[test]
    fn parse_happy_path() {
	let result: &Element = &parse("8:announce").unwrap()[0];
	assert_eq!(result, &Element::ByteString("announce".to_string()));

	let result: &Element = &parse("i-18e").unwrap()[0];
	assert_eq!(result, &Element::Integer(-18));

	let result: Vec<Element> = parse(
	    "8:announce41:http://bttracker.debian.org:6969/announce7:comment35:\"Debian CD from cdimage.debian.org\"10:created by"
	).unwrap();
	assert_eq!(
	    result,
	    vec![
		Element::ByteString("announce".to_string()),
		Element::ByteString("http://bttracker.debian.org:6969/announce".to_string()),
		Element::ByteString("comment".to_string()),
		Element::ByteString("\"Debian CD from cdimage.debian.org\"".to_string()),
		Element::ByteString("created by".to_string()),
	    ]
	);

	let result: Vec<Element> = parse("li10ei1el1:bee1:a").unwrap();
	assert_eq!(
	    result,
	    vec![
		Element::List(vec![
		    Element::Integer(10),
		    Element::Integer(1),
		    Element::List(vec![
			Element::ByteString("b".to_string()),
		    ]),
		]),
		Element::ByteString("a".to_string()),
	    ]
	);
    }
}
