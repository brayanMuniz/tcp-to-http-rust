use std::io::Read;
use std::sync::mpsc::{self, Receiver};
use std::thread;

pub fn get_lines_channel<R: Read + Send + 'static>(mut reader: R) -> Receiver<String> {
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
