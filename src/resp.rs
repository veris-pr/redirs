use crate::resp_result::{RESPError, RESPLength, RESPResult};
use std::fmt;

type RESPFn = fn(&[u8], &mut usize) -> RESPResult<RESP>;

#[derive(Debug, PartialEq)]
pub enum RESP {
    Array(Vec<RESP>),
    SimpleString(String),
    Null,
    BulkString(String),
}

impl fmt::Display for RESP {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let data = match self {
            Self::Array(data) => {
                let mut output = String::from("*");
                output.push_str(format!("{}\r\n", data.len()).as_str());
                for elem in data.iter() {
                    output.push_str(elem.to_string().as_str());
                }
                output
            }
            Self::SimpleString(data) => format!("+{}\r\n", data),
            Self::Null => "$-1\r\n".to_string(),
            Self::BulkString(data) => format!("${}\r\n{}\r\n", data.len(), data),
        };
        write!(f, "{}", data)
    }
}

fn resp_process_type(type_byte: char, buffer: &[u8], index: &mut usize) -> RESPResult<()> {
    if buffer[*index] != type_byte as u8 {
        return Err(RESPError::WrongType);
    }
    *index += 1;
    Ok(())
}

fn parser_router(buffer: &[u8], index: &mut usize) -> Option<RESPFn> {
    match buffer.get(*index) {
        Some(&b'+') => Some(resp_parse_simple_string),
        Some(&b'$') => Some(resp_parse_bulk_string),
        Some(&b'*') => Some(resp_parse_array),
        _ => None,
    }
}

pub fn bytes_to_resp(buffer: &[u8], index: &mut usize) -> RESPResult<RESP> {
    if let Some(parser_fn) = parser_router(buffer, index) {
        parser_fn(buffer, index)
    } else {
        Err(RESPError::Unknown)
    }
}

// utils
fn binary_extract_bytes(buffer: &[u8], index: &mut usize, length: usize) -> RESPResult<Vec<u8>> {
    let mut output = Vec::new();
    if *index + length > buffer.len() {
        return Err(RESPError::OutOfBounds(buffer.len()));
    }
    output.extend_from_slice(&buffer[*index..*index + length]);
    *index += length;
    Ok(output)
}

fn binary_extract_line(buffer: &[u8], index: &mut usize) -> RESPResult<Vec<u8>> {
    if *index >= buffer.len() {
        return Err(RESPError::OutOfBounds(*index));
    }

    let mut res = Vec::new();
    let mut prev: u8 = buffer[*index].clone();
    let mut curr: usize = *index;
    let mut found: bool = false;

    for &elem in buffer[*index..].iter() {
        curr += 1;
        if elem == b'\n' && prev == b'\r' {
            found = true;
            break;
        }
        prev = elem.clone();
    }
    if !found {
        *index = curr;
        return Err(RESPError::OutOfBounds(*index));
    }
    res.extend_from_slice(&buffer[*index..curr - 2]);
    *index = curr;
    Ok(res)
}

// Extracts bytes from the buffer until a `\r\n` is reached and converts them into a string.
fn binary_extract_line_as_string(buffer: &[u8], index: &mut usize) -> RESPResult<String> {
    // Extract all possible bytes updating the index.
    let line = binary_extract_line(buffer, index)?;
    // Convert the bytes to a UTF-8 String.
    Ok(String::from_utf8(line)?)
}

// simple string
fn resp_parse_simple_string(buffer: &[u8], index: &mut usize) -> RESPResult<RESP> {
    resp_process_type('+', buffer, index)?;
    let line: String = binary_extract_line_as_string(buffer, index)?;
    Ok(RESP::SimpleString(line))
}

// bulk string
fn resp_extract_length(buffer: &[u8], index: &mut usize) -> RESPResult<RESPLength> {
    let line = binary_extract_line_as_string(buffer, index)?;
    let len: RESPLength = line.parse()?;
    Ok(len)
}

fn resp_parse_bulk_string(buffer: &[u8], index: &mut usize) -> RESPResult<RESP> {
    resp_process_type('$', buffer, index)?;
    let len = resp_extract_length(buffer, index)?;
    if len == -1 {
        return Ok(RESP::Null);
    }
    if len < -1 {
        return Err(RESPError::IncorrectLength(len));
    }
    let bytes = binary_extract_bytes(buffer, index, len as usize)?;
    let data: String = String::from_utf8(bytes)?;
    *index += 2;
    Ok(RESP::BulkString(data))
}

// arrays
fn resp_parse_array(buffer: &[u8], index: &mut usize) -> RESPResult<RESP> {
    resp_process_type('*', buffer, index)?;
    let len = resp_extract_length(buffer, index)?;
    if len < 0 {
        return Err(RESPError::IncorrectLength(len));
    }
    let mut data = Vec::new();
    for _ in 0..len {
        match parser_router(buffer, index) {
            Some(parse_fn) => {
                let elem = parse_fn(buffer, index)?;
                data.push(elem);
            }
            None => return Err(RESPError::Unknown),
        }
    }
    Ok(RESP::Array(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_extract_line_empty_buffer() {
        let buffer = "".as_bytes();
        let mut index: usize = 0;

        match binary_extract_line(buffer, &mut index) {
            Err(RESPError::OutOfBounds(index)) => {
                assert_eq!(index, 0)
            }
            _ => panic!("Expected OutOfBounds error"),
        }
    }

    #[test]
    fn test_binary_extract_line_no_separator() {
        let buffer = "OK".as_bytes();
        let mut index: usize = 0;

        match binary_extract_line(buffer, &mut index) {
            Err(RESPError::OutOfBounds(index)) => {
                assert_eq!(index, 2)
            }
            _ => panic!("Expected OutOfBounds error"),
        }
    }

    #[test]
    fn test_binary_extract_line_oob() {
        let buffer = "OK\r\n".as_bytes();
        let mut index: usize = 5;

        match binary_extract_line(buffer, &mut index) {
            Err(RESPError::OutOfBounds(index)) => {
                assert_eq!(index, 5)
            }
            _ => panic!("Expected OutOfBounds error"),
        }
    }

    #[test]
    fn test_binary_extract_line() {
        let buffer = "OK\r\n".as_bytes();
        let mut index: usize = 0;
        let output = binary_extract_line(buffer, &mut index).unwrap();
        assert_eq!(output, "OK".as_bytes());
        assert_eq!(index, 4);
    }

    #[test]
    fn test_binary_extract_line_longer_string() {
        let buffer = "ECHO\r\n".as_bytes();
        let mut index: usize = 0;

        let output = binary_extract_line(buffer, &mut index).unwrap();

        assert_eq!(output, "ECHO".as_bytes());
        assert_eq!(index, 6);
    }

    #[test]
    fn test_binary_extract_line_incomplete_string_r() {
        let buffer = "OK\r".as_bytes();
        let mut index: usize = 0;
        match binary_extract_line(buffer, &mut index) {
            Err(RESPError::OutOfBounds(index)) => {
                assert_eq!(index, 3)
            }
            _ => panic!("Expected OutOfBounds error"),
        }
    }

    #[test]
    fn test_binary_extract_line_incomplete_string_n() {
        let buffer = "OK\n".as_bytes();
        let mut index: usize = 0;
        match binary_extract_line(buffer, &mut index) {
            Err(RESPError::OutOfBounds(index)) => {
                assert_eq!(index, 3)
            }
            _ => panic!("Expected OutOfBounds error"),
        }
    }

    #[test]
    fn test_binary_extract_line_to_string() {
        let buffer = "OK\r\n".as_bytes();
        let mut index: usize = 0;
        let output = binary_extract_line(buffer, &mut index).unwrap();
        let output_str = String::from_utf8(output).unwrap();
        assert_eq!(output_str, "OK");
        assert_eq!(index, 4);
    }

    #[test]
    fn test_binary_extract_line_to_string_as_string() {
        let buffer = "OK\r\n".as_bytes();
        let mut index: usize = 0;
        let output = binary_extract_line_as_string(buffer, &mut index).unwrap();
        assert_eq!(output, "OK");
        assert_eq!(index, 4);
    }

    #[test]
    fn test_binary_extract_line_to_string_invalid_utf8() {
        let buffer = [0xff, 0xfe, 0xfd, b'\r', b'\n'];
        let mut index: usize = 0;
        match binary_extract_line_as_string(&buffer, &mut index) {
            Err(RESPError::FromUtf8) => {}
            _ => panic!("Expected FromUtf8 error"),
        }
    }

    #[test]
    fn test_resp_type_parsing() {
        let buffer = b"+OK\r\n";
        let mut index: usize = 0;
        resp_process_type('+', buffer, &mut index).unwrap();
        assert_eq!(index, 1);
    }

    #[test]
    fn test_resp_type_parsing_wrong_type() {
        let buffer = b"-Error\r\n";
        let mut index: usize = 0;
        match resp_process_type('+', buffer, &mut index) {
            Err(RESPError::WrongType) => {}
            _ => panic!("Expected WrongType error"),
        }
    }

    #[test]
    fn test_resp_parse_simple_string() {
        let buffer = b"+OK\r\n";
        let mut index: usize = 0;

        let s = resp_parse_simple_string(buffer, &mut index).unwrap();
        assert_eq!(s, RESP::SimpleString(String::from("OK")));
        assert_eq!(index, 5);
    }

    #[test]
    fn test_parser_resp_to_simple_string() {
        let buffer = b"+OK\r\n";
        let mut index: usize = 0;

        let s = bytes_to_resp(buffer, &mut index).unwrap();
        assert_eq!(s, RESP::SimpleString(String::from("OK")));
        assert_eq!(index, 5);
    }

    #[test]
    fn test_parser_resp_unknown_type() {
        let buffer = b"-Error\r\n";
        let mut index: usize = 0;

        let result = bytes_to_resp(buffer, &mut index);
        assert!(matches!(result, Err(RESPError::Unknown)));
    }

    #[test]
    fn test_parse_bulk_string() {
        let buffer = "$2\r\nOK\r\n".as_bytes();
        let mut index: usize = 0;

        let output = resp_parse_bulk_string(buffer, &mut index).unwrap();

        assert_eq!(output, RESP::BulkString(String::from("OK")));
        assert_eq!(index, 8);
    }

    #[test]
    fn test_parse_bulk_string_empty() {
        let buffer = "$-1\r\n".as_bytes();
        let mut index: usize = 0;

        let output = resp_parse_bulk_string(buffer, &mut index).unwrap();

        assert_eq!(output, RESP::Null);
        assert_eq!(index, 5);
    }

    #[test]
    fn test_parse_bulk_string_unparsable_length() {
        let buffer = "$wrong\r\nOK\r\n".as_bytes();
        let mut index: usize = 0;

        let error = resp_parse_bulk_string(buffer, &mut index).unwrap_err();

        assert_eq!(error, RESPError::ParseInt);
        assert_eq!(index, 8);
    }

    #[test]
    fn test_parse_bulk_string_negative_length() {
        let buffer = "$-7\r\nOK\r\n".as_bytes();
        let mut index: usize = 0;

        let error = resp_parse_bulk_string(buffer, &mut index).unwrap_err();

        assert_eq!(error, RESPError::IncorrectLength(-7));
        assert_eq!(index, 5);
    }

    #[test]
    fn test_parse_bulk_string_data_too_short() {
        let buffer = "$7\r\nOK\r\n".as_bytes();
        let mut index: usize = 0;

        let error = resp_parse_bulk_string(buffer, &mut index).unwrap_err();

        assert_eq!(error, RESPError::OutOfBounds(8));
        assert_eq!(index, 4);
    }

    #[test]
    fn test_bytes_to_resp_bulk_string() {
        let buffer = "$2\r\nOK\r\n".as_bytes();
        let mut index: usize = 0;

        let output = bytes_to_resp(buffer, &mut index).unwrap();

        assert_eq!(output, RESP::BulkString(String::from("OK")));
        assert_eq!(index, 8);
    }
    // Array Tests
    #[test]
    fn test_parse_array() {
        let buffer = "*2\r\n+OK\r\n$5\r\nVALUE\r\n".as_bytes();
        let mut index: usize = 0;

        let output = resp_parse_array(buffer, &mut index).unwrap();

        assert_eq!(
            output,
            RESP::Array(vec![
                RESP::SimpleString(String::from("OK")),
                RESP::BulkString(String::from("VALUE"))
            ])
        );
        assert_eq!(index, 20);
    }

    #[test]
    fn test_parse_array_invalid_length() {
        let buffer = "*-1\r\n+OK\r\n$5\r\nVALUE\r\n".as_bytes();
        let mut index: usize = 0;

        let error = resp_parse_array(buffer, &mut index).unwrap_err();

        assert_eq!(error, RESPError::IncorrectLength(-1));
        assert_eq!(index, 5);
    }

    #[test]
    fn test_bytes_to_resp_array() {
        let buffer = "*2\r\n+OK\r\n$5\r\nVALUE\r\n".as_bytes();
        let mut index: usize = 0;

        let output = bytes_to_resp(buffer, &mut index).unwrap();

        assert_eq!(
            output,
            RESP::Array(vec![
                RESP::SimpleString(String::from("OK")),
                RESP::BulkString(String::from("VALUE"))
            ])
        );
        assert_eq!(index, 20);
    }
}
