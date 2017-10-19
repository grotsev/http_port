extern crate tokio_postgres;
extern crate tokio_core;
extern crate futures;

use tokio_postgres::{Connection, TlsMode};
use tokio_core::reactor::Core;
use futures::{Future, Stream};

fn main() {
    let mut l = Core::new().unwrap();
    let handle = l.handle();

    let done = Connection::connect(
        "postgres://postgres:111@172.17.0.2:5432",
        TlsMode::None,
        &handle,
    ).then(|c| c.unwrap().batch_execute("LISTEN test_notifications"))
        .and_then(|c1| {
            Connection::connect(
                "postgres://postgres:111@172.17.0.2:5432",
                TlsMode::None,
                &handle,
            ).then(|c2| {
                c2.unwrap()
                    .batch_execute("NOTIFY test_notifications, 'foo'")
                    .map(|_| c1)
            })
        })
        .and_then(|c| {
            c.notifications().into_future().map_err(
                |(e, n)| (e, n.into_inner()),
            )
        })
        .map(|(n, _)| {
            let n = n.unwrap();
            assert_eq!(n.channel, "test_notifications");
            assert_eq!(n.payload, "foo");
        });

    l.run(done).unwrap();
}