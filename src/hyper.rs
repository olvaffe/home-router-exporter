// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use crate::prometheus::Prom;
use anyhow::{Context, Result};
use hyper::{Request, Response, body::Bytes};
use std::{future, net, pin, sync};

#[derive(Clone)]
struct Svc {
    prom: sync::Arc<Prom>,
}

impl hyper::service::Service<Request<hyper::body::Incoming>> for Svc {
    type Response = Response<http_body_util::Full<Bytes>>;
    type Error = hyper::Error;
    type Future =
        pin::Pin<Box<dyn future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<hyper::body::Incoming>) -> Self::Future {
        let resp = match req.uri().path() {
            "/metrics" => {
                self.prom.collect();
                let buf = self.prom.encode();

                Response::builder()
                    .header(hyper::header::CONTENT_TYPE, self.prom.format_type())
                    .body(http_body_util::Full::new(Bytes::from(buf)))
                    .unwrap()
            }
            _ => Response::builder()
                .status(404)
                .body(http_body_util::Full::new(Bytes::new()))
                .unwrap(),
        };

        Box::pin(async { Ok(resp) })
    }
}

#[tokio::main]
pub async fn run(addr: net::SocketAddr, prom: Prom) -> Result<()> {
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind to {:?}", addr))?;

    let svc = Svc {
        prom: sync::Arc::new(prom),
    };

    loop {
        let stream = match listener.accept().await {
            Ok((stream, _)) => stream,
            Err(e) => {
                println!("failed to accept connection: {e:?}");
                continue;
            }
        };
        let io = hyper_util::rt::TokioIo::new(stream);
        let svc = svc.clone();

        let future = async move {
            if let Err(err) = hyper::server::conn::http1::Builder::new()
                .serve_connection(io, svc)
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        };

        tokio::task::spawn(future);
    }
}
