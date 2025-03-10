// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use crate::prometheus::Prom;
use anyhow::Result;
use std::{net, sync};

pub struct Ping {
    notify: sync::Arc<tokio::sync::Notify>,
}

async fn test_ping(notify: sync::Arc<tokio::sync::Notify>) -> Result<()> {
    let host_v4 = net::IpAddr::V4(net::Ipv4Addr::LOCALHOST);
    let config_v4 = surge_ping::Config::builder().build();
    let client_v4 = surge_ping::Client::new(&config_v4)?;
    let mut pinger_v4 = client_v4
        .pinger(host_v4, surge_ping::PingIdentifier(0))
        .await;

    let host_v6 = net::IpAddr::V6(net::Ipv6Addr::LOCALHOST);
    let config_v6 = surge_ping::Config::builder()
        .kind(surge_ping::ICMP::V6)
        .build();
    let client_v6 = surge_ping::Client::new(&config_v6)?;
    let mut pinger_v6 = client_v6
        .pinger(host_v6, surge_ping::PingIdentifier(0))
        .await;

    let payload = [0u8; 56];
    let mut seqno = surge_ping::PingSequence(0);
    loop {
        notify.notified().await;

        let (reply_v4, reply_v6) = tokio::join!(
            pinger_v4.ping(seqno, &payload),
            pinger_v6.ping(seqno, &payload)
        );
        seqno.0 += 1;

        if let Ok((_, dur)) = reply_v4 {
            println!("{:?}: {:?}", pinger_v4.host, dur);
        }
        if let Ok((_, dur)) = reply_v6 {
            println!("{:?}: {:?}", pinger_v6.host, dur);
        }
    }
}

impl Ping {
    pub fn new() -> Self {
        let notify = sync::Arc::new(tokio::sync::Notify::new());
        tokio::task::spawn(test_ping(notify.clone()));
        Ping { notify }
    }

    pub fn collect(&self, _prom: &Prom) {
        self.notify.notify_one();
    }
}
