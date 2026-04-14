use std::fs::File;
use std::io::Read;
use std::sync::mpsc::{self, Receiver};
use std::thread;

// messages.txt:
// Do you have what it takes to be an engineer at TheStartup™?
// Are you willing to work 80 hours a week in hopes that your 0.001% equity is worth something?
// Can you say "synergy" and "democratize" with a straight face?
// Are you prepared to eat top ramen at your desk 3 meals a day?
// end

fn main() {
    let file_attempt = File::open("./messages.txt");
    let f;

    match file_attempt {
        Ok(result) => f = result,
        Err(_) => return,
    }

    let receiver = get_lines_channel(f);
    receiver.iter().for_each(|line| print!("{line}"));
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
                    if str.contains("\n") {
                        let result = str.find('\n');
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
