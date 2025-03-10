// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use crate::prometheus::Prom;
use anyhow::Result;
use std::{net, sync, time};

pub struct Ping {
    client_v4: surge_ping::Client,
    client_v6: surge_ping::Client,
    ident: surge_ping::PingIdentifier,
    payload: [u8; 56],
    notify: tokio::sync::Notify,

    hosts: sync::Mutex<Vec<net::SocketAddr>>,
    roundtrips: sync::Mutex<Option<Vec<Roundtrip>>>,
}

struct Roundtrip {
    host: net::SocketAddr,
    duration: time::Duration,
}

impl Ping {
    pub fn new() -> sync::Arc<Self> {
        let config_v4 = surge_ping::Config::builder().build();
        let client_v4 = surge_ping::Client::new(&config_v4).unwrap();

        let config_v6 = surge_ping::Config::builder()
            .kind(surge_ping::ICMP::V6)
            .build();
        let client_v6 = surge_ping::Client::new(&config_v6).unwrap();

        let notify = tokio::sync::Notify::new();

        let hosts = sync::Mutex::new(vec![
            net::SocketAddrV4::new(net::Ipv4Addr::LOCALHOST, 0).into(),
            net::SocketAddrV6::new(net::Ipv6Addr::LOCALHOST, 0, 0, 0).into(),
        ]);
        let roundtrips = sync::Mutex::new(None);

        let ping = Ping {
            client_v4,
            client_v6,
            ident: surge_ping::PingIdentifier(0),
            payload: [0; 56],
            notify,
            hosts,
            roundtrips,
        };
        let ping = sync::Arc::new(ping);

        let clone = ping.clone();
        tokio::task::spawn(async move {
            clone.task().await;
        });

        ping
    }

    pub fn collect(&self, _prom: &Prom) {
        if let Some(roundtrips) = self.roundtrips.lock().unwrap().take() {
            for roundtrip in roundtrips {
                println!("{:?} roundtrip: {:?}", roundtrip.host, roundtrip.duration);
            }
        }

        self.notify.notify_one();
    }

    async fn task(&self) {
        let mut seqno = 0;
        loop {
            self.notify.notified().await;
            *self.roundtrips.lock().unwrap() = self
                .parse_roundtrips(surge_ping::PingSequence(seqno))
                .await
                .ok();
            seqno += 1;
        }
    }

    async fn parse_roundtrips(&self, seqno: surge_ping::PingSequence) -> Result<Vec<Roundtrip>> {
        let hosts = self.hosts.lock().unwrap().clone();

        let mut pingers = Vec::new();
        for host in &hosts {
            let pinger = match host {
                net::SocketAddr::V4(_) => self.client_v4.pinger(host.ip(), self.ident),
                net::SocketAddr::V6(_) => self.client_v6.pinger(host.ip(), self.ident),
            }
            .await;
            pingers.push(pinger);
        }

        let mut futures = Vec::new();
        for pinger in &mut pingers {
            futures.push(pinger.ping(seqno, &self.payload));
        }

        let replies = futures::future::join_all(futures).await;

        let mut roundtrips = Vec::new();
        for (host, reply) in std::iter::zip(hosts, replies) {
            let duration = match reply {
                Ok((_, dur)) => dur,
                Err(_) => time::Duration::ZERO,
            };

            roundtrips.push(Roundtrip { host, duration })
        }

        Ok(roundtrips)
    }
}
