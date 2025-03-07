// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use std::net::SocketAddr;

use hyper::{Request, Response, body::Bytes};

#[derive(Clone)]
struct Svc {
    prom: std::sync::Arc<crate::prometheus::Prom>,
}

impl hyper::service::Service<Request<hyper::body::Incoming>> for Svc {
    type Response = Response<http_body_util::Full<Bytes>>;
    type Error = hyper::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn call(&self, req: Request<hyper::body::Incoming>) -> Self::Future {
        let resp = match req.uri().path() {
            "/metrics" => {
                self.prom.update();
                let buf = self.prom.gather();

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
pub async fn run(
    prom: crate::prometheus::Prom,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    let svc = Svc {
        prom: std::sync::Arc::new(prom),
    };

    loop {
        let (stream, _) = listener.accept().await?;
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
