#![feature(plugin)]
#![plugin(docopt_macros)]

extern crate docopt;
extern crate bincode;
extern crate rustc_serialize;
extern crate rand;

use rustc_serialize::{Encodable, Decodable};
use std::net::{UdpSocket, ToSocketAddrs, SocketAddr};

docopt!(Args derive Debug, "
Usage:
    mesh [options]
    mesh [options] TARGET

Options:
    -h, --host HOST  Host to listen on. [default: 127.0.0.1]
    -p, --port PORT  Local port to bind to. [default: 0]

When run with TARGET, attempt to join the specified target mesh.
Otherwise, begin listening on the specified host and port.
",
    flag_host: String,
    flag_port: u16);

#[derive(RustcEncodable, RustcDecodable)]
enum Message {
    Join { seq: u32 },
}

impl Message {
    fn encode(&self) -> Vec<u8> {
        bincode::encode(self, bincode::SizeLimit::Infinite).unwrap()
    }
    fn decode(bytes: &[u8]) -> Message {
        bincode::decode::<Message>(bytes).unwrap()
    }
}

#[test]
fn join_message_is_recodable() {
    let m = Message::Join { seq: 100 };
    let bytes = m.encode();
    let m = Message::decode(&bytes);

    match Message::decode(&bytes) {
        Message::Join { seq: seq } => assert_eq!(seq, 100),
    }
}

fn send<A: ToSocketAddrs>(msg: &Message, target: &A, socket: &UdpSocket) {
    socket.send_to(&msg.encode(), target).ok();
}

fn join(seq: u32, joiner: &SocketAddr) {
    println!("Received a JOIN request {} from {}", seq, joiner);
}

// Listen on a UDP socket and call appropriate handlers for received messages.
fn dispatch_forever(socket: &UdpSocket) {
    loop {
        // TODO: establish MTU or just use large buffer
        let mut buf = [0;4096];
        let (amt, src) = socket.recv_from(&mut buf).unwrap();
        let buf = &buf[..amt];

        match Message::decode(&buf) {
            Message::Join {seq} => join(seq, &src)
        }
    }
}

fn main() {
    use std::net::UdpSocket;
    use rand::{thread_rng, Rng};

    let args: Args = Args::docopt().decode().unwrap_or_else(|e| e.exit());
    let (host, port) = (&args.flag_host[..], args.flag_port);
    let target = &args.arg_TARGET[..];

    let port = {
        if port == 0 { thread_rng().gen_range(1024, 32768) } else { port }
    };

    println!("Listening on {}:{}", host, port);
    let socket = UdpSocket::bind((host, port)).unwrap();

    // Send an initial JOIN if TARGET is given
    if target.len() > 0 {
        send(&Message::Join { seq: 1 }, &target, &socket);
    }

    dispatch_forever(&socket);
    drop(socket);
}
