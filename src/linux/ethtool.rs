// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use anyhow::{Context, Result};
use neli::{
    attr::Attribute,
    consts::nl::NlmF,
    genl::{GenlAttrHandle, Genlmsghdr, GenlmsghdrBuilder, NoUserHeader},
    nl::NlPayload,
    router::synchronous::NlRouterReceiverHandle,
};

#[neli::neli_enum(serialized_type = "u8")]
enum EthtoolMsg {
    LinkModesGet = 4,
}
impl neli::consts::genl::Cmd for EthtoolMsg {}

#[neli::neli_enum(serialized_type = "u16")]
enum EthtoolAttrLinkModes {
    Header = 1,
    Speed = 5,
}
impl neli::consts::genl::NlAttrType for EthtoolAttrLinkModes {}

#[neli::neli_enum(serialized_type = "u16")]
enum EthtoolAttrHeader {
    DevName = 2,
}
impl neli::consts::genl::NlAttrType for EthtoolAttrHeader {}

type Ethtoolmsghdr = Genlmsghdr<EthtoolMsg, EthtoolAttrLinkModes>;
type EthtoolmsghdrBuilder = GenlmsghdrBuilder<EthtoolMsg, EthtoolAttrLinkModes, NoUserHeader>;
type EthtoolReceiverHandle = NlRouterReceiverHandle<u16, Ethtoolmsghdr>;

pub(super) struct LinkSpeed {
    pub name: String,
    pub speed: i32,
}

fn parse_header_attrs(header: GenlAttrHandle<EthtoolAttrHeader>) -> Option<String> {
    for attr in header.iter() {
        match attr.nla_type().nla_type() {
            EthtoolAttrHeader::DevName => {
                let name = attr.get_payload_as_with_len::<String>().unwrap();
                return Some(name);
            }
            _ => (),
        }
    }

    None
}

fn parse_link_modes_get_response(resp: &Ethtoolmsghdr) -> Option<LinkSpeed> {
    let mut name = None;
    let mut speed = -1;
    for attr in resp.attrs().iter() {
        match attr.nla_type().nla_type() {
            EthtoolAttrLinkModes::Header => {
                let header = attr.get_attr_handle::<EthtoolAttrHeader>().unwrap();
                name = parse_header_attrs(header);
            }
            EthtoolAttrLinkModes::Speed => {
                speed = attr.get_payload_as::<i32>().unwrap();
            }
            _ => (),
        }
    }

    if name.is_some() && speed > 0 {
        Some(LinkSpeed {
            name: name.unwrap(),
            speed,
        })
    } else {
        None
    }
}

impl super::Linux {
    pub(super) fn parse_ethtool(&self) -> Result<Vec<LinkSpeed>> {
        let req = EthtoolmsghdrBuilder::default()
            .cmd(EthtoolMsg::LinkModesGet)
            .version(1)
            .build()?;
        let recv: EthtoolReceiverHandle = self
            .genl_sock
            .send(self.ethtool_id, NlmF::DUMP, NlPayload::Payload(req))
            .context("failed to send to ethtool")?;

        let mut ifaces = Vec::new();
        for genlmsg in recv {
            let genlmsg = genlmsg.context("got an ethtool error")?;
            if let Some(resp) = genlmsg.get_payload() {
                if let Some(speed) = parse_link_modes_get_response(resp) {
                    ifaces.push(speed);
                }
            }
        }

        Ok(ifaces)
    }
}
