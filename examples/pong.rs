/// An example demonstrating how to send and recieve a custom ping/pong frame.
extern crate ws;
extern crate env_logger;
extern crate time;

use std::str::from_utf8;

use ws::{listen, CloseCode, OpCode, Sender, Frame, Handler, Handshake, Message, Result, Error, ErrorKind};
use ws::util::{Token, Timeout};

const PING: Token = Token(1);
const EXPIRE: Token = Token(2);

fn main () {

    // Setup logging
    env_logger::init().unwrap();

    // Run the WebSocket
    listen("127.0.0.1:3012", |out| {
        Server {
            out: out,
            timeout: None,
        }
    }).unwrap();

}

// Server WebSocket handler
struct Server {
    out: Sender,
    timeout: Option<Timeout>,
}

impl Handler for Server {

    fn on_open(&mut self, _: Handshake) -> Result<()> {
        // schedule a timeout to send a ping every 5 seconds
        try!(self.out.timeout(5_000, PING));
        // schedule a timeout to close the connection if there is no activity for 30 seconds
        self.out.timeout(30_000, EXPIRE)
    }

    fn on_message(&mut self, msg: Message) -> Result<()> {
        println!("Server got message '{}'. ", msg);
        self.out.send(msg)
    }

    fn on_close(&mut self, code: CloseCode, reason: &str) {
        println!("WebSocket closing for ({:?}) {}", code, reason);
        println!("Shutting down server after first connection closes.");
        self.out.shutdown().unwrap();
    }

    fn on_error(&mut self, err: Error) {
        // Shutdown on any error
        println!("Shutting down server for error: {}", err);
        self.out.shutdown().unwrap();
    }

    fn on_timeout(&mut self, event: Token) -> Result<()> {
        match event {
            // PING timeout has occured, send a ping and reschedule
            PING => {
                try!(self.out.ping(time::precise_time_ns().to_string().into()));
                self.out.timeout(5_000, PING)
            }
            // EXPIRE timeout has occured, this means that the connection is inactive, let's close
            EXPIRE => self.out.close(CloseCode::Away),
            // No other timeouts are possible
            _ => Err(Error::new(ErrorKind::Internal, "Invalid timeout token encountered!")),
        }
    }

    fn on_new_timeout(&mut self, event: Token, timeout: Timeout) -> Result<()> {
        // If the new scheduled timeout is an EXPIRE timeout,
        // we cancel the old timeout and replace.
        if event == EXPIRE {
            if let Some(t) = self.timeout.take() {
                try!(self.out.cancel(t))
            }
            self.timeout = Some(timeout)
        }

        Ok(())
    }

    fn on_frame(&mut self, frame: Frame) -> Result<Option<Frame>> {
        // If the frame is a pong, print the latency.
        // The pong should contain data from out ping, but it isn't guaranteed to.
        if frame.opcode() == OpCode::Pong {
            if let Ok(pong) = try!(from_utf8(frame.payload())).parse::<u64>() {
                let now = time::precise_time_ns();
                println!("Latency is {:.3}ms.", (now - pong) as f64 / 1_000_000f64);
            } else {
                println!("Received bad pong.");
            }
        }

        // Some activity has occured, so reset the expiration
        try!(self.out.timeout(30_000, EXPIRE));

        // Run default frame validation
        DefaultHandler.on_frame(frame)
    }
}

// For accessing the default handler implementation
struct DefaultHandler;

impl Handler for DefaultHandler {}

