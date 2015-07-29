#![feature(plugin)]
#![plugin(docopt_macros)]

extern crate docopt;
extern crate rustc_serialize;
extern crate rand;

docopt!(Args derive Debug, "
Usage:
    mesh [options]
    mesh [options] TARGET MESSAGE

Options:
    -h, --host HOST  Host to listen on. [default: 127.0.0.1]
    -p, --port PORT  Local port to bind to. [default: 0]

When run with TARGET and MESSAGE, send specified message to a listening mesh.
Otherwise, begin listening on the specified host and port.
",
    flag_host: String,
    flag_port: u16);

fn main() {
    use std::net::UdpSocket;
    use rand::{thread_rng, Rng};

    let args: Args = Args::docopt().decode().unwrap_or_else(|e| e.exit());
    let (host, port) = (&args.flag_host[..], args.flag_port);
    let (target, message) = (&args.arg_TARGET[..], args.arg_MESSAGE);

    let port = {
        if port == 0 { thread_rng().gen_range(1024, 32768) } else { port }
    };

    println!("Listening on {}:{}", host, port);

    let socket = UdpSocket::bind((host, port)).unwrap();

    if target.len() > 0 {
        socket.send_to(message.as_bytes(), target).ok();
    }

    // Receive a response
    let mut buf = [0;4096];
    let (amt, src) = socket.recv_from(&mut buf).unwrap();
    let buf = &mut buf[..amt];
    {
        let s = std::str::from_utf8(buf).ok().unwrap();
        println!("Received: {}", s);
    }

    buf.reverse();
    socket.send_to(buf, &src).ok();

    drop(socket);
}
