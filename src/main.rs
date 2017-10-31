extern crate tokio_postgres;
extern crate tokio_core;
extern crate futures;
extern crate hyper;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

use std::io;
use futures::sync::mpsc;
use tokio_postgres::{Connection, TlsMode};
use tokio_core::reactor::Core;
use futures::{Future, Stream, Sink};
use hyper::Client;
use serde_json::Value;

#[derive(Debug, Deserialize)]
enum Method {
    GET,
    POST,
}

#[derive(Deserialize)]
struct Request {
    method: Method,
    url: String,
    callback: String,
}

#[derive(Debug, Serialize)]
struct Response {
    status: u16,
    body: Value,
}

fn main() {
    let mut l = Core::new().unwrap();
    let handle = l.handle();
    let client = Client::new(&handle);
    let (tx, rx) = mpsc::channel(8);

    let back = rx.for_each(|(callback, result)| {
        println!("Response: {}\n{}", callback, result);
        Ok(())
    });
    handle.spawn(back);

    let done = Connection::connect(
        "postgres://postgres:111@172.17.0.2:5432",
        TlsMode::None,
        &handle,
    ).then(|c| c.unwrap().batch_execute("LISTEN test_notifications"))
        .map_err(|(e, _)| e)
        .and_then(|c| {
            c.notifications().for_each(|n| {
                let request: Request = serde_json::from_str(&n.payload).unwrap();
                let url = request.url.parse().unwrap();
                let tx = tx.clone();
                let serve_one = client
                    .get(url)
                    .and_then(|res| {
                        let mut response = Response {
                            status: res.status().into(),
                            body: Value::Null,
                        };
                        println!("Response: {}", res.status());

                        res.body()
                            .concat2()
                            .and_then(move |body| {
                                response.body = serde_json::from_slice(&body).unwrap();
                                let s = serde_json::to_string(&response).unwrap();
                                println!("Response: {}", s);
                                tx.send((request.callback, s))
                                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
                                    .map_err(From::from)
                                    .map(|_| ())
                            })
                            .map_err(From::from)
                    })
                    .map_err(|e| {
                        println!("Response: {}", e);
                    });
                handle.spawn(serve_one);
                Ok(())
            })
        });

    l.run(done).unwrap();
}