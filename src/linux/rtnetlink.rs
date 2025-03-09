// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use anyhow::{Context, Result};
use neli::{
    attr::Attribute,
    consts::nl::NlmF,
    consts::rtnl::{Arphrd, Ifla, RtAddrFamily, RtScope, RtTable, Rta, Rtm, Rtn, Rtprot},
    nl::NlPayload,
    rtnl::{Ifinfomsg, IfinfomsgBuilder, Rtmsg, RtmsgBuilder},
};

pub struct Link {
    pub name: String,
    pub rx: u64,
    pub tx: u64,
}

pub struct Route {
    pub gateway: std::net::IpAddr,
    pub oif: i32,
}

fn parse_get_link_response(resp: &Ifinfomsg) -> Option<Link> {
    let mut name = None;
    let mut rx = 0;
    let mut tx = 0;
    for attr in resp.rtattrs().iter() {
        match attr.rta_type() {
            Ifla::Ifname => {
                let s = attr.get_payload_as_with_len::<String>().unwrap();
                name = Some(s);
            }
            Ifla::Stats64 => {
                let payload = attr.payload().as_ref();
                // rtnl_link_stats64
                if payload.len() >= 32 {
                    rx = u64::from_ne_bytes(payload[16..24].try_into().unwrap());
                    tx = u64::from_ne_bytes(payload[24..32].try_into().unwrap());
                }
            }
            _ => (),
        }
    }

    if let Some(name) = name {
        Some(Link { name, rx, tx })
    } else {
        None
    }
}

fn parse_get_route_response(resp: &Rtmsg) -> Option<Route> {
    if *resp.rtm_dst_len() != 0 {
        return None;
    }

    let mut gateway = None;
    let mut oif = -1;
    for attr in resp.rtattrs().iter() {
        match attr.rta_type() {
            Rta::Gateway => {
                let payload = attr.rta_payload().as_ref();
                if let Ok(octets) = <&[u8; 4]>::try_from(payload) {
                    gateway = Some(std::net::IpAddr::from(*octets));
                } else if let Ok(segments) = <&[u8; 16]>::try_from(payload) {
                    gateway = Some(std::net::IpAddr::from(*segments));
                }
            }
            Rta::Oif => oif = attr.get_payload_as::<i32>().unwrap(),
            _ => (),
        }
    }

    if let Some(gateway) = gateway {
        Some(Route { gateway, oif })
    } else {
        None
    }
}

impl super::Linux {
    pub fn parse_links(&self) -> Result<Vec<Link>, Box<dyn std::error::Error>> {
        let req = IfinfomsgBuilder::default()
            .ifi_family(RtAddrFamily::Unspecified)
            .ifi_type(Arphrd::Netrom)
            .ifi_index(0)
            .build()?;
        let recv = self
            .rt_sock
            .send::<_, _, Rtm, Ifinfomsg>(Rtm::Getlink, NlmF::DUMP, NlPayload::Payload(req))
            .context("failed to send to rtnetlink")?;

        let mut ifaces = Vec::new();
        for nlmsg in recv {
            let nlmsg = nlmsg.context("got a rtnetlink error")?;
            if let Some(resp) = nlmsg.get_payload() {
                if let Some(link) = parse_get_link_response(resp) {
                    ifaces.push(link);
                }
            }
        }

        Ok(ifaces)
    }

    pub fn parse_routes(&self) -> Result<Vec<Route>, Box<dyn std::error::Error>> {
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
        let recv = self
            .rt_sock
            .send::<_, _, Rtm, Rtmsg>(Rtm::Getroute, NlmF::DUMP, NlPayload::Payload(req))
            .context("failed to send to rtnetlink")?;

        let mut routes = Vec::new();
        for nlmsg in recv {
            let nlmsg = nlmsg.context("got a rtnetlink error")?;
            if let Some(resp) = nlmsg.get_payload() {
                if let Some(route) = parse_get_route_response(resp) {
                    routes.push(route);
                }
            }
        }

        Ok(routes)
    }
}
