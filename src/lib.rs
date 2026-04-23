use std::collections::HashMap;
use std::io::{Cursor, Error, Read};
use std::sync::mpsc::{self, Receiver};
use std::thread;

#[derive(Debug)]
struct Request {
    request_line: RequestLine,
    // headers: Headers,
}

#[derive(Debug)]
struct RequestLine {
    method: String,
    request_target: String,
    http_version: String,
}

impl RequestLine {
    fn parse(line: &str) -> Result<RequestLine, ReaderError> {
        let method;
        let request_target;
        let http_version;

        let mut request_parts = line.split_whitespace();

        method = request_parts
            .next()
            .ok_or(ReaderError::NoMethod)?
            .to_string();

        request_target = request_parts
            .next()
            .ok_or(ReaderError::NoRequestTarget)?
            .to_string();

        http_version = request_parts
            .next()
            .and_then(|s| s.split_once('/')) // HTTP/1.1 => (HTTP/, 1.1)
            .ok_or(ReaderError::MalformedRequest)?
            .1
            .to_string();

        Ok(RequestLine {
            http_version,
            request_target,
            method,
        })
    }
}

#[derive(Debug)]
enum ReaderError {
    NoStartLine,
    NoRequestTarget,
    NoMethod,
    MalformedRequest,
    NoHeadersLine,
    MalformedHeader,
}

#[derive(Debug)]
struct Headers {
    headers: HashMap<String, String>,
}

impl Headers {
    fn new() -> Headers {
        Headers {
            headers: HashMap::new(),
        }
    }

    fn get_headers(self) -> HashMap<String, String> {
        return self.headers;
    }

    fn parse(&mut self, line: String) -> Result<(), ReaderError> {
        let (key, val) = line.split_once(":").ok_or(ReaderError::MalformedHeader)?;

        if key.contains(" ") {
            return Err(ReaderError::MalformedHeader);
        }

        self.headers
            .entry(key.trim().to_string())
            .or_insert(val.trim().to_string());

        Ok(())
    }
}

// GET / HTTP/1.1\r\n
fn request_from_reader<R: Read + Send + 'static>(reader: R) -> Result<Request, ReaderError> {
    let receiver = get_lines_channel(reader);

    let first_line = receiver.iter().next().ok_or(ReaderError::NoStartLine)?;
    let request_line = RequestLine::parse(&first_line)?;

    Ok(Request {
        request_line: RequestLine {
            http_version: request_line.http_version,
            request_target: request_line.request_target,
            method: request_line.method,
        },
    })
}

// Returns a lines channel seperated by \r\n
pub fn get_lines_channel<R: Read + Send + 'static>(mut reader: R) -> Receiver<String> {
    let (tx, rx) = mpsc::channel();
    let mut buffer = [0u8; 8]; // 8 bytes long
    let mut output: Vec<u8> = Vec::new();

    thread::spawn(move || {
        loop {
            let n = match reader.read(&mut buffer) {
                Ok(amount_read) => amount_read,
                Err(_) => return, // Error
            };

            output.extend_from_slice(&buffer[..n]);

            let is_new_line = output.windows(2).position(|w| w == b"\r\n");
            match is_new_line {
                Some(idx) => {
                    let line_bytes = output[..idx].to_vec();
                    output.drain(..idx + 2); // reset output, +2 for \r\n

                    if let Ok(line_string) = String::from_utf8(line_bytes) {
                        let _ = tx.send(line_string);
                    }
                }
                None => continue,
            }
        }
    });

    rx
}

mod tests {
    use std::hash::Hash;

    use crate::*;

    #[derive(Debug)]
    struct TestReader {
        data: Cursor<Vec<u8>>,
        number_bytes_to_read: u64,
    }

    impl TestReader {
        fn new(data: String, number_bytes_to_read: u64) -> TestReader {
            TestReader {
                data: Cursor::new(data.into_bytes()),
                number_bytes_to_read,
            }
        }
    }

    impl Read for TestReader {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            // by_ref is used to keep compiler happy
            self.data.by_ref().take(self.number_bytes_to_read).read(buf)
        }
    }

    #[test]
    fn read_single_request_line() {
        let single_request = "GET / HTTP/1.1\r\n".to_string();
        let reader = TestReader::new(single_request, 2);
        let receiver = get_lines_channel(reader);

        let val = receiver.iter().next();
        if let Some(val) = val {
            assert_eq!("GET / HTTP/1.1", val);
        } else {
            assert!(false);
        }
    }

    #[test]
    fn valid_single_header() {
        let headers_line = "Host: localhost:42069\r\n\r\n".to_string();
        let reader = TestReader::new(headers_line, 2);
        let receiver = get_lines_channel(reader);

        let val = receiver.iter().next();

        if let Some(line) = val {
            let mut n = Headers::new();
            match n.parse(line) {
                Ok(()) => {
                    let mut tmp = HashMap::new();
                    tmp.entry("Host".to_string())
                        .or_insert("localhost:42069".to_string());
                    assert_eq!(tmp, n.get_headers());
                }
                Err(_) => assert!(false),
            }
        }
    }

    #[test]
    fn invalid_header() {
        let headers_line = "       Host : localhost:42069       \r\n\r\n".to_string();
        let reader = TestReader::new(headers_line, 2);
        let receiver = get_lines_channel(reader);

        let val = receiver.iter().next();

        if let Some(line) = val {
            let mut n = Headers::new();
            match n.parse(line) {
                Ok(()) => {
                    assert!(false);
                }
                Err(_) => assert!(true),
            }
        }
    }

    #[test]
    fn good_request_line() {
        let good_request =
        "GET / HTTP/1.1\r\nHost: localhost:42069\r\nUser-Agent: curl/7.81.0\r\nAccept: */*\r\n\r\n"
            .to_string();
        let reader = TestReader::new(good_request, 4);
        let result = request_from_reader(reader);
        match result {
            Ok(request) => {
                println!("{request:?}");
                assert_eq!("GET", request.request_line.method);
                assert_eq!("/", request.request_line.request_target);
                assert_eq!("1.1", request.request_line.http_version);
            }
            Err(_) => assert!(false),
        }
    }

    #[test]
    fn good_request_line_with_path() {
        let str = "GET /coffee HTTP/1.1\r\nHost: localhost:42069\r\nUser-Agent: curl/7.81.0\r\nAccept: */*\r\n\r\n".to_string();
        let reader = TestReader::new(str, 5);
        let result = request_from_reader(reader);
        match result {
            Ok(request) => {
                assert_eq!("GET", request.request_line.method);
                assert_eq!("/coffee", request.request_line.request_target);
                assert_eq!("1.1", request.request_line.http_version);
            }
            Err(_) => assert!(false),
        }
    }

    #[test]
    fn invalid_number_parts_in_request_line() {
        let bad_request =
        "/coffee HTTP/1.1\r\nHost: localhost:42069\r\nUser-Agent: curl/7.81.0\r\nAccept: */*\r\n\r\n".to_string();
        let reader = TestReader::new(bad_request, 7);
        let result = request_from_reader(reader);
        match result {
            Ok(_) => {
                assert!(false)
            }
            Err(_) => assert!(true),
        }
    }
}
