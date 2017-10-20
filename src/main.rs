extern crate tokio_postgres;
extern crate tokio_core;
extern crate futures;

use tokio_postgres::{Connection, TlsMode};
use tokio_core::reactor::Core;
use futures::{Future, Stream};

fn main() {
    let mut l = Core::new().unwrap();
    let handle = &l.handle();

    let done = Connection::connect(
        "postgres://postgres:111@172.17.0.2:5432",
        TlsMode::None,
        &handle,
    ).then(|c| c.unwrap().batch_execute("LISTEN test_notifications"))
        .map_err(|(e, _)| e)
        .and_then(|c| {
            c.notifications().for_each(|n| {
                let serve_one = futures::future::ok(println!("{:?}", n.payload));
                handle.spawn(serve_one);
                Ok(())
            })
        });

    l.run(done).unwrap();
}