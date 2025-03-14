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

pub const ETHTOOL_GENL_NAME: &str = "ethtool";
const ETHTOOL_GENL_VERSION: u8 = 1;

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
        if attr.nla_type().nla_type() == &EthtoolAttrHeader::DevName {
            return attr.get_payload_as_with_len::<String>().ok();
        }
    }

    None
}

fn parse_link_modes_get_response(resp: &Ethtoolmsghdr) -> Option<LinkSpeed> {
    let mut name = None;
    let mut speed = None;
    for attr in resp.attrs().iter() {
        match attr.nla_type().nla_type() {
            EthtoolAttrLinkModes::Header => {
                name = attr
                    .get_attr_handle::<EthtoolAttrHeader>()
                    .ok()
                    .and_then(parse_header_attrs);
            }
            EthtoolAttrLinkModes::Speed => {
                speed = attr.get_payload_as::<i32>().ok();
            }
            _ => (),
        }
    }

    match (name, speed) {
        (Some(name), Some(speed)) if speed > 0 => Some(LinkSpeed { name, speed }),
        _ => None,
    }
}

pub(super) struct EthtoolIter {
    recv: EthtoolReceiverHandle,
}

impl Iterator for EthtoolIter {
    type Item = Result<LinkSpeed>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let genlmsg = match self.recv.next_typed::<u16, Ethtoolmsghdr>() {
                Some(Ok(msg)) => msg,
                Some(Err(err)) => return Some(Err(err).context("failed to recv from ethtool")),
                None => return None,
            };

            if let Some(speed) = genlmsg
                .get_payload()
                .and_then(parse_link_modes_get_response)
            {
                return Some(Ok(speed));
            }
        }
    }
}

impl super::Linux {
    pub(super) fn parse_ethtool(&self) -> Result<EthtoolIter> {
        let req = EthtoolmsghdrBuilder::default()
            .cmd(EthtoolMsg::LinkModesGet)
            .version(ETHTOOL_GENL_VERSION)
            .build()?;
        let recv: EthtoolReceiverHandle = self
            .genl_sock
            .send(self.ethtool_id, NlmF::DUMP, NlPayload::Payload(req))
            .context("failed to send to ethtool")?;

        Ok(EthtoolIter { recv })
    }
}
