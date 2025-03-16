// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use crate::{collector, config};
use anyhow::{Context, Result, anyhow};
use hyper::{Request, Response, body::Bytes};
use log::{debug, error, info};
use std::{future, net, pin, sync};

#[derive(Clone)]
struct Svc {
    collector: sync::Arc<collector::Collector>,

    error_500: Response<http_body_util::Full<Bytes>>,
}

impl hyper::service::Service<Request<hyper::body::Incoming>> for Svc {
    type Response = Response<http_body_util::Full<Bytes>>;
    type Error = hyper::Error;
    type Future =
        pin::Pin<Box<dyn future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<hyper::body::Incoming>) -> Self::Future {
        let resp = match req.uri().path() {
            "/metrics" => {
                let buf = self.collector.collect();

                Response::builder()
                    .header(
                        hyper::header::CONTENT_TYPE,
                        collector::Collector::content_type(),
                    )
                    .body(http_body_util::Full::from(buf))
            }
            _ => {
                debug!("incorrect uri {}", req.uri());
                Response::builder()
                    .status(404)
                    .body(http_body_util::Full::default())
            }
        }
        .unwrap_or_else(|_| self.error_500.clone());

        Box::pin(async { Ok(resp) })
    }
}

async fn serve_connection(stream: tokio::net::TcpStream, svc: Svc) {
    let io = hyper_util::rt::TokioIo::new(stream);

    let http = hyper::server::conn::http1::Builder::new();
    let conn = http.serve_connection(io, svc);

    if let Err(err) = conn.await {
        error!("server connection error: {err:?}");
    }
}

pub async fn run(collector: collector::Collector) -> Result<()> {
    let addr = &config::get().hyper_addr;
    let addr: net::SocketAddr = addr
        .parse()
        .map_err(|_| anyhow!("invalid listen address {addr}"))?;
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind to {addr:?}"))?;

    let svc = Svc {
        collector: sync::Arc::new(collector),
        error_500: Response::builder()
            .status(500)
            .body(http_body_util::Full::default())?,
    };

    info!("listening on {addr:?}");

    loop {
        let stream = match listener.accept().await {
            Ok((stream, client_addr)) => {
                debug!("new connection from {client_addr:?}");
                stream
            }
            Err(err) => {
                error!("failed to accept connection: {err:?}");
                continue;
            }
        };

        tokio::task::spawn(serve_connection(stream, svc.clone()));
    }
}
