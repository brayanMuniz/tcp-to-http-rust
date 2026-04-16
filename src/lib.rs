use std::io::{Cursor, Read};
use std::sync::mpsc::{self, Receiver};
use std::thread;

#[derive(Debug)]
struct Request {
    request_line: RequestLine,
}

#[derive(Debug)]
struct RequestLine {
    http_version: String,
    request_target: String,
    method: String,
}

#[derive(Debug)]
enum ReaderError {
    NoStartLine,
    NoRequestTarget,
    NoMethod,
    MalformedRequest,
}

// GET / HTTP/1.1\r\n
fn request_from_reader<R: Read + Send + 'static>(reader: R) -> Result<Request, ReaderError> {
    let receiver = get_lines_channel(reader);

    // RequestLine fields
    let method;
    let request_target;
    let http_version;

    let request_line = receiver.iter().next();
    let r = {
        if let Some(p) = request_line {
            let mut request_parts = p.split_whitespace();

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

            RequestLine {
                http_version,
                request_target,
                method,
            }
        } else {
            return Err(ReaderError::NoStartLine);
        }
    };

    Ok(Request {
        request_line: RequestLine {
            http_version: r.http_version,
            request_target: r.request_target,
            method: r.method,
        },
    })
}

fn get_lines_channel<R: Read + Send + 'static>(mut reader: R) -> Receiver<String> {
    let (tx, rx) = mpsc::channel();
    let mut buffer = [0u8; 8]; // 8 bytes long
    let mut output = String::new();

    thread::spawn(move || {
        loop {
            let r = reader.read(&mut buffer);
            let n;
            match r {
                Ok(read) => {
                    if read == 0 {
                        return;
                    }
                    n = read
                }
                Err(_) => return,
            }

            let s = std::str::from_utf8(&buffer[..n]);
            match s {
                Ok(str) => {
                    if str.contains("\r\n") {
                        let result = str.find("\r\n");
                        match result {
                            Some(idx) => {
                                let left = str[..idx].to_string();
                                let _ = tx.send(format!("{output}{left}").to_string());

                                // Reset output for next one
                                output = str[idx..].to_string();
                            }
                            None => return,
                        }
                    } else {
                        output.push_str(str);
                    }
                }
                Err(_) => return,
            }
        }
    });

    rx
}

#[test]
fn read_single_line() {
    let single_request = "GET / HTTP/1.1\r\n".to_string();
    let receiver = get_lines_channel(Cursor::new(single_request));

    let val = receiver.iter().next();
    if let Some(val) = val {
        assert_eq!("GET / HTTP/1.1", val);
    } else {
        assert!(false);
    }
}

#[test]
fn good_request_line() {
    let good_request =
        "GET / HTTP/1.1\r\nHost: localhost:42069\r\nUser-Agent: curl/7.81.0\r\nAccept: */*\r\n\r\n"
            .to_string();
    let result = request_from_reader(Cursor::new(good_request));
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
    let result = request_from_reader(Cursor::new("GET /coffee HTTP/1.1\r\nHost: localhost:42069\r\nUser-Agent: curl/7.81.0\r\nAccept: */*\r\n\r\n".to_string() ));
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
    let result = request_from_reader(Cursor::new(bad_request));
    match result {
        Ok(_) => {
            assert!(false)
        }
        Err(_) => assert!(true),
    }
}
