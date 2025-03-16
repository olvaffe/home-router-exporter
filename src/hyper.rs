// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use crate::{collector, config};
use anyhow::{Context, Error, Result, anyhow};
use hyper::{Request, Response, body, header, server::conn::http1, service};
use log::{debug, error, info};
use std::{future, net, pin, sync};

#[derive(Clone)]
struct Svc {
    collector: sync::Arc<collector::Collector>,
    error_500: Response<http_body_util::Full<body::Bytes>>,
}

impl Svc {
    fn new(collector: collector::Collector) -> Result<Self> {
        let collector = sync::Arc::new(collector);
        let error_500 = Response::builder()
            .status(500)
            .body(http_body_util::Full::default())?;

        Ok(Svc {
            collector,
            error_500,
        })
    }

    fn handle_request(
        &self,
        req: Request<body::Incoming>,
    ) -> Result<Response<http_body_util::Full<body::Bytes>>> {
        match req.uri().path() {
            "/metrics" => {
                let buf = self.collector.collect();

                Response::builder()
                    .header(header::CONTENT_TYPE, collector::Collector::content_type())
                    .body(http_body_util::Full::from(buf))
            }
            _ => {
                debug!("incorrect uri {}", req.uri());
                Response::builder()
                    .status(404)
                    .body(http_body_util::Full::default())
            }
        }
        .or_else(|_| Ok(self.error_500.clone()))
    }

    async fn task(self, stream: tokio::net::TcpStream) {
        let io = hyper_util::rt::TokioIo::new(stream);
        let conn = http1::Builder::new().serve_connection(io, self);

        if let Err(err) = conn.await {
            error!("server connection error: {err:?}");
        }
    }
}

impl service::Service<Request<body::Incoming>> for Svc {
    type Response = Response<http_body_util::Full<body::Bytes>>;
    type Error = Error;
    type Future =
        pin::Pin<Box<dyn future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<body::Incoming>) -> Self::Future {
        let resp = self.handle_request(req);
        Box::pin(async { resp })
    }
}

pub async fn run(collector: collector::Collector) -> Result<()> {
    let svc = Svc::new(collector).context("failed to create service")?;

    let addr = &config::get().hyper_addr;
    let addr: net::SocketAddr = addr
        .parse()
        .map_err(|_| anyhow!("invalid listen address {addr}"))?;
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind to {addr:?}"))?;

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

        let clone = svc.clone();
        tokio::task::spawn(async move {
            clone.task(stream).await;
        });
    }
}
