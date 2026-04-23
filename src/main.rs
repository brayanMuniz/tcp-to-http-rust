use std::net::TcpListener;

use tcp_to_http_rust::get_lines_channel;

// curl http://localhost:42069/coffee
// curl -X POST -H "Content-Type: application/json" -d '{"flavor":"dark mode"}' http://localhost:42069/coffee

// HTTP Format:
// CRLF: \r\n

// start-line CRLF
// *( field-line CRLF )
// CRLF
// [ message-body ]

// This is just for my reference
// -- Scroll the documentation window [b]ack / [f]orward
// ['<C-b>'] = cmp.mapping.scroll_docs(-4),
// ['<C-f>'] = cmp.mapping.scroll_docs(4),

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
