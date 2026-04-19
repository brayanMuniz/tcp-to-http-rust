use std::net::TcpListener;

pub mod foo;
use foo::get_lines_channel;

// curl http://localhost:42069/coffee
// curl -X POST -H "Content-Type: application/json" -d '{"flavor":"dark mode"}' http://localhost:42069/coffee

// HTTP Format:
// CRLF: \r\n

// start-line CRLF
// *( field-line CRLF )
// CRLF
// [ message-body ]

fn main() {
    let listner = TcpListener::bind("127.0.0.1:42069");

    if let Ok(result) = listner {
        for stream in result.incoming() {
            if let Ok(r) = stream {
                let receiver = get_lines_channel(r);
                receiver.iter().for_each(|line| print!("{line}"));
            }
        }
    }
}
