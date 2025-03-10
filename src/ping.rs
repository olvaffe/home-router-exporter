// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use crate::prometheus::Prom;
use anyhow::Result;
use std::{net, sync, time};

pub struct Ping {
    notify: sync::Arc<tokio::sync::Notify>,

    hosts: sync::Arc<sync::Mutex<Vec<net::IpAddr>>>,
}

struct Roundtrip {
    host: net::IpAddr,
    duration: time::Duration,
}

async fn test_ping(
    notify: sync::Arc<tokio::sync::Notify>,
    hosts: sync::Arc<sync::Mutex<Vec<net::IpAddr>>>,
) -> Result<()> {
    let config_v4 = surge_ping::Config::builder().build();
    let client_v4 = surge_ping::Client::new(&config_v4)?;

    let config_v6 = surge_ping::Config::builder()
        .kind(surge_ping::ICMP::V6)
        .build();
    let client_v6 = surge_ping::Client::new(&config_v6)?;

    let ident = surge_ping::PingIdentifier(0);
    let payload = [0u8; 56];
    let mut seqno = surge_ping::PingSequence(0);
    loop {
        notify.notified().await;

        let hosts = hosts.lock().unwrap().clone();

        let mut pingers = Vec::new();
        for host in &hosts {
            let pinger = match host {
                net::IpAddr::V4(_) => client_v4.pinger(*host, ident),
                net::IpAddr::V6(_) => client_v6.pinger(*host, ident),
            }
            .await;
            pingers.push(pinger);
        }

        let mut futures = Vec::new();
        for pinger in &mut pingers {
            futures.push(pinger.ping(seqno, &payload));
        }

        seqno.0 += 1;

        let replies = futures::future::join_all(futures).await;
        for (host, reply) in std::iter::zip(hosts, replies) {
            if let Ok((_, dur)) = reply {
                println!("{:?}: {:?}", host, dur);
            }
        }
    }
}

impl Ping {
    pub fn new() -> Self {
        let notify = sync::Arc::new(tokio::sync::Notify::new());
        let hosts = sync::Arc::new(sync::Mutex::new(vec![
            net::IpAddr::V4(net::Ipv4Addr::LOCALHOST),
            net::IpAddr::V6(net::Ipv6Addr::LOCALHOST),
        ]));
        tokio::task::spawn(test_ping(notify.clone(), hosts.clone()));
        Ping { notify, hosts }
    }

    pub fn collect(&self, _prom: &Prom) {
        self.notify.notify_one();
    }
}
