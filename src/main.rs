extern crate tokio;
extern crate futures;
extern crate chrono;

use tokio::prelude::*;
use chrono::prelude::*;
use tokio::net::tcp::TcpListener;
use tokio::codec::{Framed, LinesCodec};
use futures::future::{lazy, ok};
use std::net::SocketAddr;
use std::str::FromStr;
use std::collections::VecDeque;

struct ClientState {
    offset: Option<FixedOffset>
}

fn process_request(mut state: ClientState, request: String) -> (ClientState, String) {
    let mut parts = request.split(" ").collect::<VecDeque<&str>>();

    println!("parts count: {}", parts.len());

    match parts.pop_front() {
        None => (state, "ERROR no command specific".to_string()),
        Some(command) => match command {
            "set_offset" => match parts.pop_front() {
                None => (state, "ERROR set_offset expects offset value".to_string()),
                Some(offset_raw) => match i32::from_str(offset_raw) {
                    Ok(offset) => {
                        state.offset = FixedOffset::east_opt(offset);
                        (state, "OK offset configured".to_string())
                    },
                    Err(_) => (state, "ERROR invalid offset".to_string()),
                },
            },
            "get_time" => {
		let utc: DateTime<Utc> = Utc::now();

                match state.offset.as_ref() {
                    None => (state, format!("OK {}", utc.to_rfc3339())),
                    Some(offset) => {
                        let with_offset = utc.with_timezone(offset);
                        (state, format!("OK {}", with_offset.to_rfc3339()))
                    },
                }
            },
            s => (state, format!("ERROR unknown command '{}'", s)),
        },
    }
}

fn main() {
    tokio::run(lazy(|| {
        let address = "127.0.0.1:3456".parse().unwrap();
        let listener = TcpListener::bind(&address).unwrap();
        let work = listener.incoming()
            .map_err(|e| println!("listener: caught error: {:?}", e))
            .for_each(|client| {
                let framed = Framed::new(client, LinesCodec::new());
                let (client_tx, client_rx) = framed.split();

                let initial_state = ClientState { offset: None };

                let work2 = client_rx
                    .fold((initial_state, client_tx), |(state, tx), msg| {
                        let (state, response) = process_request(state, msg);

                        tx.send(response)
                            .map(move |tx| (state, tx))
                    })
                    .map(|_| ())
                    .map_err(|_| ());

                tokio::spawn(work2);
                ok(())
            });

        tokio::spawn(work);
        ok(())
    }));
}
