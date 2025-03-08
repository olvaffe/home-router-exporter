// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use neli::{
    attr::Attribute,
    consts::{
        nl::NlmF,
        rtnl::{Arphrd, Ifla, RtAddrFamily, RtScope, RtTable, Rta, Rtm, Rtn, Rtprot},
        socket::NlFamily,
    },
    nl::NlPayload,
    router::synchronous::NlRouter,
    rtnl::{Ifinfomsg, IfinfomsgBuilder, Rtmsg, RtmsgBuilder},
    utils::Groups,
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

pub fn parse_links() -> Result<Vec<Link>, Box<dyn std::error::Error>> {
    let mut ifaces = Vec::new();

    let (sock, _) = NlRouter::connect(NlFamily::Route, None, Groups::empty())?;
    sock.enable_ext_ack(true)?;
    sock.enable_strict_checking(true)?;

    let req = IfinfomsgBuilder::default()
        .ifi_family(RtAddrFamily::Unspecified)
        .ifi_type(Arphrd::Netrom)
        .ifi_index(0)
        .build()?;

    let recv = sock.send::<_, _, Rtm, Ifinfomsg>(
        Rtm::Getlink,
        NlmF::REQUEST | NlmF::DUMP,
        NlPayload::Payload(req),
    )?;

    for resp in recv {
        if let Some(payload) = resp?.get_payload() {
            let mut name = None;
            let mut rx = 0;
            let mut tx = 0;

            let attr_handle = payload.rtattrs().get_attr_handle();
            for attr in attr_handle.iter() {
                match attr.rta_type() {
                    Ifla::Ifname => {
                        let n = attr.get_payload_as_with_len::<String>().unwrap();
                        name = Some(n);
                    }
                    Ifla::Stats64 => {
                        let n = attr.payload().as_ref();
                        if n.len() >= 32 {
                            rx = u64::from_le_bytes(n[16..24].try_into()?);
                            tx = u64::from_le_bytes(n[24..32].try_into()?);
                        }
                    }
                    _ => (),
                }
            }

            if name.is_some() {
                ifaces.push(Link {
                    name: name.unwrap(),
                    rx,
                    tx,
                });
            }
        }
    }

    Ok(ifaces)
}

pub fn parse_routes() -> Result<Vec<Route>, Box<dyn std::error::Error>> {
    let mut routes = Vec::new();

    let (sock, _) = NlRouter::connect(NlFamily::Route, None, Groups::empty())?;
    sock.enable_ext_ack(true)?;
    sock.enable_strict_checking(true)?;

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

    let recv = sock.send::<_, _, Rtm, Rtmsg>(
        Rtm::Getroute,
        NlmF::REQUEST | NlmF::DUMP,
        NlPayload::Payload(req),
    )?;

    for resp in recv {
        if let Some(payload) = resp?.get_payload() {
            if *payload.rtm_dst_len() != 0 {
                continue;
            }

            let mut gateway = None;
            let mut oif = -1;
            for attr in payload.rtattrs().iter() {
                match attr.rta_type() {
                    Rta::Gateway => {
                        let p = attr.rta_payload().as_ref();
                        if let Ok(octets) = <&[u8; 4]>::try_from(p) {
                            gateway = Some(std::net::IpAddr::from(*octets));
                        } else if let Ok(segments) = <&[u8; 16]>::try_from(p) {
                            gateway = Some(std::net::IpAddr::from(*segments));
                        }
                    }
                    Rta::Oif => oif = attr.get_payload_as::<i32>().unwrap(),
                    _ => (),
                }
            }

            if let Some(gateway) = gateway {
                routes.push(Route { gateway, oif });
            }
        }
    }

    Ok(routes)
}
