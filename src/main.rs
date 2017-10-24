extern crate tokio_postgres;
extern crate tokio_core;
extern crate futures;
extern crate async_http_client;

use std::io;
use tokio_postgres::{Connection, TlsMode};
use tokio_core::reactor::Core;
use futures::{Future, Stream};
use async_http_client::prelude::*;
use async_http_client::{HttpRequest, HttpCodec};

fn main() {
    let mut l = Core::new().unwrap();
    let handle = l.handle();

    let done = Connection::connect(
        "postgres://postgres:111@172.17.0.2:5432",
        TlsMode::None,
        &handle,
    ).then(|c| c.unwrap().batch_execute("LISTEN test_notifications"))
        .map_err(|(e, _)| e)
        .and_then(|c| {
            c.notifications().for_each(|n| {
                let req = HttpRequest::get("http://example.com").unwrap();
                let serve_one = TcpStream::connect(&(req.addr().unwrap()), &handle)
                    .and_then(|connection| {
                        let framed = connection.framed(HttpCodec::new());
                        req.send(framed)
                    })
                    .then(|r| match r {
                        Ok((resp, _fram)) => futures::future::ok(println!("{}", resp.unwrap())),
                        Err(e) => futures::future::ok(println!("{}", e)),
                    });
                handle.spawn(serve_one);
                Ok(())
            })
        });

    l.run(done).unwrap();
}

fn other(desc: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Other, desc)
}