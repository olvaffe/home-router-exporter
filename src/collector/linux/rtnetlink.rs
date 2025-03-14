// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use anyhow::{Context, Result};
use neli::{
    attr::Attribute,
    consts::nl::NlmF,
    consts::rtnl::{Arphrd, Iff, Ifla, RtAddrFamily, RtScope, RtTable, Rta, Rtm, Rtn, Rtprot},
    nl::NlPayload,
    router::synchronous::NlRouterReceiverHandle,
    rtnl::{Ifinfomsg, IfinfomsgBuilder, Rtmsg, RtmsgBuilder},
};
use std::net;

pub(super) struct Link {
    pub name: String,
    pub admin_up: bool,
    pub operstate: u8,
    pub rx: u64,
    pub tx: u64,
}

fn parse_get_link_response(resp: &Ifinfomsg) -> Option<Link> {
    let admin_up = resp.ifi_flags().contains(Iff::UP);

    let mut name = None;
    let mut operstate = None;
    let mut stats64 = None;
    for attr in resp.rtattrs().iter() {
        match attr.rta_type() {
            Ifla::Ifname => {
                name = attr.get_payload_as_with_len::<String>().ok();
            }
            Ifla::Operstate => {
                operstate = attr.get_payload_as::<u8>().ok();
            }
            Ifla::Stats64 => {
                stats64 = Some(attr.payload().as_ref());
            }
            _ => (),
        }
    }

    let operstate = operstate.unwrap_or(0);
    let mut rx = 0;
    let mut tx = 0;
    if let Some(stats64) = stats64 {
        // struct rtnl_link_stats64
        if stats64.len() >= 32 {
            rx = u64::from_ne_bytes(stats64[16..24].try_into().unwrap());
            tx = u64::from_ne_bytes(stats64[24..32].try_into().unwrap());
        }
    }

    name.map(|name| Link {
        name,
        admin_up,
        operstate,
        rx,
        tx,
    })
}

pub(super) struct LinkIter {
    recv: NlRouterReceiverHandle<Rtm, Ifinfomsg>,
}

impl Iterator for LinkIter {
    type Item = Result<Link>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let nlmsg = match self.recv.next_typed::<Rtm, Ifinfomsg>() {
                Some(Ok(msg)) => msg,
                Some(Err(err)) => return Some(Err(err).context("failed to recv from rtnetlink")),
                None => return None,
            };

            if let Some(link) = nlmsg.get_payload().and_then(parse_get_link_response) {
                return Some(Ok(link));
            }
        }
    }
}

fn parse_get_route_response(resp: &Rtmsg) -> Option<net::SocketAddr> {
    // skip if not default route
    if *resp.rtm_dst_len() != 0 {
        return None;
    }

    let mut gateway = None;
    let mut oif = None;
    for attr in resp.rtattrs().iter() {
        match attr.rta_type() {
            Rta::Gateway => gateway = Some(attr.rta_payload().as_ref()),
            Rta::Oif => oif = attr.get_payload_as::<u32>().ok(),
            _ => (),
        }
    }

    gateway
        .and_then(|gateway| {
            if let Ok(octets) = <&[u8; 4]>::try_from(gateway) {
                Some(net::IpAddr::from(*octets))
            } else if let Ok(segments) = <&[u8; 16]>::try_from(gateway) {
                Some(net::IpAddr::from(*segments))
            } else {
                None
            }
        })
        .map(|ip| {
            if let net::IpAddr::V6(v6) = ip {
                if v6.is_unicast_link_local() {
                    let oif = oif.unwrap_or(0);
                    return net::SocketAddrV6::new(v6, 0, 0, oif).into();
                }
            }

            net::SocketAddr::new(ip, 0)
        })
}

pub(super) struct RouteIter {
    recv: NlRouterReceiverHandle<Rtm, Rtmsg>,
}

impl Iterator for RouteIter {
    type Item = Result<net::SocketAddr>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let nlmsg = match self.recv.next_typed::<Rtm, Rtmsg>() {
                Some(Ok(msg)) => msg,
                Some(Err(err)) => return Some(Err(err).context("failed to recv from rtnetlink")),
                None => return None,
            };

            if let Some(route) = nlmsg.get_payload().and_then(parse_get_route_response) {
                return Some(Ok(route));
            }
        }
    }
}

impl super::Linux {
    pub(super) fn parse_links(&self) -> Result<LinkIter> {
        let req = IfinfomsgBuilder::default()
            .ifi_family(RtAddrFamily::Unspecified)
            .ifi_type(Arphrd::Netrom)
            .ifi_index(0)
            .build()?;
        let recv: NlRouterReceiverHandle<Rtm, Ifinfomsg> = self
            .rt_sock
            .send(Rtm::Getlink, NlmF::DUMP, NlPayload::Payload(req))
            .context("failed to send to rtnetlink")?;

        Ok(LinkIter { recv })
    }

    pub(super) fn parse_routes(&self) -> Result<RouteIter> {
        let req = RtmsgBuilder::default()
            .rtm_family(RtAddrFamily::Unspecified)
            .rtm_dst_len(0)
            .rtm_src_len(0)
            .rtm_tos(0)
            .rtm_table(RtTable::Main)
            .rtm_protocol(Rtprot::Unspec)
            .rtm_scope(RtScope::Universe)
            .rtm_type(Rtn::Unspec)
            .build()?;
        let recv: NlRouterReceiverHandle<Rtm, Rtmsg> = self
            .rt_sock
            .send(Rtm::Getroute, NlmF::DUMP, NlPayload::Payload(req))
            .context("failed to send to rtnetlink")?;

        Ok(RouteIter { recv })
    }
}
