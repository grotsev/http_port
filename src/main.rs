extern crate tokio_postgres;
extern crate tokio_core;
extern crate futures;
extern crate futures_cpupool;
extern crate r2d2;
extern crate r2d2_postgres;
extern crate hyper;
extern crate serde;
extern crate serde_json;
extern crate toml;
#[macro_use]
extern crate serde_derive;

use std::fs::File;
use std::env;
use std::io;
use std::io::prelude::*;
use std::error::Error;

use tokio_postgres::Connection;
use tokio_core::reactor::Core;
use futures::{Future, Stream};
use hyper::Client;
use serde_json::Value;
use futures_cpupool::CpuPool;
use r2d2_postgres::PostgresConnectionManager;

static VERSION: &'static str = "0.0.1";

#[derive(Debug, Deserialize)]
struct Config {
    db_uri : String,
    db_pool : usize,
    db_channel : String
}

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

fn help() {
    println!(r##"
Usage: http_port FILENAME
    http_port {version} / REST API request from Postgres

Available options:
  -h,--help                Show this help text
  FILENAME                 Path to configuration file

Example Config File:
  db-uri = "postgres://user:pass@localhost:5432/dbname"
  db-pool = 10
  db-channel = http_port
"##, version=VERSION);
}

fn process(config: Config) -> io::Result<()> {
    let mut l = Core::new().unwrap();
    let handle = l.handle();
    let client = Client::new(&handle);
    let thread_pool = CpuPool::new(config.db_pool);

    let db_config = r2d2::Config::default();
    let db_manager = PostgresConnectionManager::new(config.db_uri.clone(), r2d2_postgres::TlsMode::None).unwrap();
    let db_pool = r2d2::Pool::new(db_config, db_manager).unwrap();

    let done = Connection::connect(config.db_uri.clone(), tokio_postgres::TlsMode::None, &handle)
        .then(|c| c.unwrap().batch_execute(&format!("listen {}", &config.db_channel)))
        .map_err(|(e, _)| e)
        .and_then(|c| {
            c.notifications().for_each(|n| {
                let request: Request = serde_json::from_str(&n.payload).unwrap();
                let url = request.url.parse().unwrap();
                let thread_pool = thread_pool.clone();
                let db = db_pool.clone();

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
                                thread_pool.spawn_fn(move || {
                                    let conn = db.get().map_err(|e| {
                                        io::Error::new(io::ErrorKind::Other, format!("timeout: {}", e))
                                    }).unwrap();

                                    conn.execute(&request.callback, &[&s]).unwrap();
                                    Ok(())
                                })
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
    return Ok(());
}

fn real_main() -> io::Result<()> {
    let mut args = env::args();
    let name = args.nth(1).ok_or_else(|| {
        help();
        io::Error::new(io::ErrorKind::Other, "Unexpected arguments length")
    })?;
    let mut f = File::open(&name)?;
    let mut input = String::new();
    let input = f.read_to_string(&mut input).map(|_| input)?;
    let config = toml::from_str(&input)
        .map_err(|error| io::Error::new(io::ErrorKind::Other, error.description()) )?;
    process(config)
}

fn main() {
    real_main().unwrap_or_else(|description| {
        println!("{}", description);
        std::process::exit(1)
    })
}