// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use anyhow::{Context, Result, anyhow};
use neli::{
    attr::Attribute,
    consts::nl::NlmF,
    genl::{Genlmsghdr, GenlmsghdrBuilder, NoUserHeader},
    nl::NlPayload,
    router::synchronous::NlRouterReceiverHandle,
};

type Ethtoolmsghdr = Genlmsghdr<EthtoolMessage, EthtoolLinkModes>;
type EthtoolmsghdrBuilder = GenlmsghdrBuilder<EthtoolMessage, EthtoolLinkModes, NoUserHeader>;
type EthtoolReceiverHandle = NlRouterReceiverHandle<u16, Ethtoolmsghdr>;

#[neli::neli_enum(serialized_type = "u8")]
enum EthtoolMessage {
    LinkModesGet = 4,
}
impl neli::consts::genl::Cmd for EthtoolMessage {}

#[neli::neli_enum(serialized_type = "u16")]
enum EthtoolLinkModes {
    Header = 1,
    Speed = 5,
}
impl neli::consts::genl::NlAttrType for EthtoolLinkModes {}

#[neli::neli_enum(serialized_type = "u16")]
enum EthtoolHeader {
    DevName = 2,
}
impl neli::consts::genl::NlAttrType for EthtoolHeader {}

pub struct EthtoolSpeed {
    pub name: String,
    pub speed: i32,
}

fn parse_link_modes_get_response(resp: &Ethtoolmsghdr) -> Result<EthtoolSpeed> {
    let mut name = None;
    let mut speed = -1;

    for attr in resp.attrs().iter() {
        match attr.nla_type().nla_type() {
            EthtoolLinkModes::Header => {
                let nested_handle = attr.get_attr_handle::<EthtoolHeader>().unwrap();
                for nested in nested_handle.iter() {
                    match nested.nla_type().nla_type() {
                        EthtoolHeader::DevName => {
                            let n = nested.get_payload_as_with_len::<String>().unwrap();
                            name = Some(n);
                        }
                        _ => (),
                    }
                }
            }
            EthtoolLinkModes::Speed => {
                speed = attr.get_payload_as::<i32>().unwrap();
            }
            _ => (),
        }
    }

    if name.is_some() && speed > 0 {
        Ok(EthtoolSpeed {
            name: name.unwrap(),
            speed,
        })
    } else {
        Err(anyhow!(""))
    }
}

impl super::Linux {
    pub fn parse_ethtool(&self) -> Result<Vec<EthtoolSpeed>> {
        let req = EthtoolmsghdrBuilder::default()
            .cmd(EthtoolMessage::LinkModesGet)
            .version(1)
            .build()?;
        let recv: EthtoolReceiverHandle = self
            .genl_sock
            .send(self.ethtool_id, NlmF::DUMP, NlPayload::Payload(req))
            .context("failed to send to ethtool")?;

        let mut ifaces = Vec::new();
        for genlmsg in recv {
            let genlmsg = genlmsg.context("got an ethtool error")?;
            let resp = match genlmsg.nl_payload() {
                NlPayload::Payload(resp) => resp,
                _ => continue,
            };

            match parse_link_modes_get_response(resp) {
                Ok(speed) => ifaces.push(speed),
                _ => (),
            }
        }

        Ok(ifaces)
    }
}
