// Copyright 2025 Google LLC
// SPDX-License-Identifier: MIT

use anyhow::{Context, Result, anyhow};
use neli::{
    FromBytesWithInput, Size, ToBytes,
    attr::Attribute,
    consts::genl::NlAttrType,
    consts::nl::{NlType, NlmF},
    err::RouterError,
    genl::{AttrTypeBuilder, GenlAttrHandle, NlattrBuilder},
    nl::{NlPayload, Nlmsghdr},
    router::synchronous::NlRouterReceiverHandle,
    types::{Buffer, GenlBuffer},
};
use std::{io, net};

const NFNETLINK_V0: u8 = 0;
const NFNL_SUBSYS_NFTABLES: u8 = 10;

#[derive(Debug, FromBytesWithInput, Size, ToBytes)]
#[neli(from_bytes_bound = "T: NlAttrType")]
#[neli(to_bytes_bound = "T: NlAttrType")]
pub struct Nfgenmsg<T> {
    family: u8,
    version: u8,
    res_id: u16,
    #[neli(input = "input - 4")]
    attrs: GenlBuffer<T, Buffer>,
}

#[neli::neli_enum(serialized_type = "u16")]
enum NftMsg {
    Getset = ((NFNL_SUBSYS_NFTABLES as u16) << 8) | 10,
    Getsetelem = ((NFNL_SUBSYS_NFTABLES as u16) << 8) | 13,
}
impl NlType for NftMsg {}

#[neli::neli_enum(serialized_type = "u16")]
enum NftaList {
    Elem = 1,
}
impl NlAttrType for NftaList {}

#[neli::neli_enum(serialized_type = "u16")]
enum NftaSet {
    Table = 1,
    Name = 2,
    Flags = 3,
    KeyType = 4,
}
impl NlAttrType for NftaSet {}

#[neli::neli_enum(serialized_type = "u16")]
enum NftaSetElem {
    Key = 1,
    Expr = 7,
}
impl NlAttrType for NftaSetElem {}

#[neli::neli_enum(serialized_type = "u16")]
enum NftaSetElemList {
    Table = 1,
    Set = 2,
    Elements = 3,
}
impl NlAttrType for NftaSetElemList {}

#[neli::neli_enum(serialized_type = "u16")]
enum NftaData {
    Value = 1,
}
impl NlAttrType for NftaData {}

#[neli::neli_enum(serialized_type = "u16")]
enum NftaExpr {
    Name = 1,
    Data = 2,
}
impl NlAttrType for NftaExpr {}

#[neli::neli_enum(serialized_type = "u16")]
enum NftaCounter {
    Bytes = 1,
    Packets = 2,
}
impl NlAttrType for NftaCounter {}

pub(super) struct NftSet {
    pub family: u8,
    pub table: String,
    pub name: String,
}

fn parse_set(resp: &Nfgenmsg<NftaSet>) -> Option<NftSet> {
    let family = resp.family;

    let mut table = None;
    let mut name = None;
    let mut flags = None;
    let mut key_type = None;
    for attr in resp.attrs.iter() {
        match attr.nla_type().nla_type() {
            NftaSet::Table => {
                table = attr.get_payload_as_with_len::<String>().ok();
            }
            NftaSet::Name => {
                name = attr.get_payload_as_with_len::<String>().ok();
            }
            NftaSet::Flags => {
                flags = attr.get_payload_as::<u32>().map(u32::swap_bytes).ok();
            }
            NftaSet::KeyType => {
                key_type = attr.get_payload_as::<u32>().map(u32::swap_bytes).ok();
            }
            _ => (),
        }
    }

    const NFT_SET_ANONYMOUS: u32 = 1;
    if flags.is_none_or(|flags| flags & NFT_SET_ANONYMOUS > 0) {
        return None;
    }

    // defined by userspace nftables
    const TYPE_IPADDR: u32 = 7;
    const TYPE_IP6ADDR: u32 = 8;
    const TYPE_ETHERADDR: u32 = 9;
    match key_type {
        Some(TYPE_IPADDR | TYPE_IP6ADDR | TYPE_ETHERADDR) => (),
        _ => return None,
    }

    match (table, name) {
        (Some(table), Some(name)) => Some(NftSet {
            family,
            table,
            name,
        }),
        _ => None,
    }
}

pub(super) struct NftSetIter {
    recv: NlRouterReceiverHandle<NftMsg, Nfgenmsg<NftaSet>>,
}

impl Iterator for NftSetIter {
    type Item = Result<NftSet>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let nlmsg = match self.recv.next_typed::<NftMsg, Nfgenmsg<NftaSet>>() {
                Some(Ok(msg)) => msg,
                Some(Err(err)) => {
                    let err = if let RouterError::Nlmsgerr(err) = err {
                        let errno = -*err.error();
                        anyhow!(io::Error::from_raw_os_error(errno))
                    } else {
                        anyhow!(err)
                    };
                    return Some(Err(err).context("failed to recv set from nft"));
                }
                None => return None,
            };

            if let Some(set) = nlmsg.get_payload().and_then(parse_set) {
                return Some(Ok(set));
            }
        }
    }
}

fn parse_set_elem_expr_counter(counter: GenlAttrHandle<NftaCounter>) -> Option<(u64, u64)> {
    let mut bytes = None;
    let mut packets = None;
    for attr in counter.iter() {
        match attr.nla_type().nla_type() {
            NftaCounter::Bytes => {
                bytes = attr.get_payload_as::<u64>().map(u64::swap_bytes).ok();
            }
            NftaCounter::Packets => {
                packets = attr.get_payload_as::<u64>().map(u64::swap_bytes).ok();
            }
            _ => (),
        }
    }

    match (bytes, packets) {
        (Some(bytes), Some(packets)) => Some((bytes, packets)),
        _ => None,
    }
}

fn parse_set_elem_expr(expr: GenlAttrHandle<NftaExpr>) -> Option<(u64, u64)> {
    let mut name = None;
    let mut data = None;
    for attr in expr.iter() {
        match attr.nla_type().nla_type() {
            NftaExpr::Name => {
                name = attr.get_payload_as_with_len::<String>().ok();
            }
            NftaExpr::Data => {
                data = attr.get_attr_handle().ok();
            }
            _ => (),
        }
    }

    match (name, data) {
        (Some(name), Some(data)) if name == "counter" => parse_set_elem_expr_counter(data),
        _ => None,
    }
}

fn parse_set_elem_key(key: GenlAttrHandle<NftaData>) -> Option<String> {
    let mut value = None;
    for attr in key.iter() {
        if attr.nla_type().nla_type() == &NftaData::Value {
            value = Some(attr.payload().as_ref());
            break;
        }
    }

    value.and_then(|value| {
        if let Ok(octets) = <&[u8; 4]>::try_from(value) {
            Some(net::IpAddr::from(*octets).to_string())
        } else if let Ok(mac) = <&[u8; 6]>::try_from(value) {
            Some(format!(
                "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
            ))
        } else if let Ok(segments) = <&[u8; 16]>::try_from(value) {
            Some(net::IpAddr::from(*segments).to_string())
        } else {
            None
        }
    })
}

fn parse_set_elem(elem: GenlAttrHandle<NftaSetElem>) -> Option<NftSetCounter> {
    let mut addr = None;
    let mut counter = None;
    for attr in elem.iter() {
        match attr.nla_type().nla_type() {
            NftaSetElem::Key => {
                addr = attr.get_attr_handle().ok().and_then(parse_set_elem_key);
            }
            NftaSetElem::Expr => {
                counter = attr.get_attr_handle().ok().and_then(parse_set_elem_expr);
            }
            _ => (),
        }
    }

    match (addr, counter) {
        (Some(addr), Some((bytes, _))) => Some(NftSetCounter { addr, bytes }),
        _ => None,
    }
}

fn parse_set_elem_list(
    list: GenlAttrHandle<NftaList>,
    base_idx: usize,
) -> Option<(usize, NftSetCounter)> {
    let elems = list.get_attrs();

    let mut idx = base_idx;
    while idx < elems.len() {
        if let Some(counter) = elems[idx].get_attr_handle().ok().and_then(parse_set_elem) {
            return Some((idx, counter));
        }
        idx += 1;
    }

    None
}

pub(super) struct NftSetCounter {
    pub addr: String,
    pub bytes: u64,
}

pub(super) struct NftSetCounterIter {
    recv: NlRouterReceiverHandle<NftMsg, Nfgenmsg<NftaSetElemList>>,
    cur_nlmsg: Option<Nlmsghdr<NftMsg, Nfgenmsg<NftaSetElemList>>>,
    cur_attr: usize,
    cur_elem: usize,
}

impl Iterator for NftSetCounterIter {
    type Item = Result<NftSetCounter>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(resp) = self
                .cur_nlmsg
                .as_ref()
                .and_then(|nlmsg| nlmsg.get_payload())
            {
                let attrs = resp.attrs.as_ref();
                while self.cur_attr < attrs.len() {
                    let attr = &attrs[self.cur_attr];
                    if attr.nla_type().nla_type() == &NftaSetElemList::Elements {
                        if let Some((idx, counter)) = attr
                            .get_attr_handle::<NftaList>()
                            .ok()
                            .and_then(|list| parse_set_elem_list(list, self.cur_elem))
                        {
                            self.cur_elem = idx + 1;
                            return Some(Ok(counter));
                        }
                    }

                    self.cur_attr += 1;
                    self.cur_elem = 0;
                }
            }

            let nlmsg = match self.recv.next_typed::<NftMsg, Nfgenmsg<NftaSetElemList>>() {
                Some(Ok(msg)) => msg,
                Some(Err(err)) => {
                    return Some(Err(err).context("failed to recv set elem from nft"));
                }
                None => return None,
            };

            self.cur_nlmsg = Some(nlmsg);
            self.cur_attr = 0;
            self.cur_elem = 0;
        }
    }
}

impl super::Linux {
    pub(super) fn parse_nfnetlink(&self) -> Result<NftSetIter> {
        let req = Nfgenmsg::<NftaSet> {
            family: 0,
            version: NFNETLINK_V0,
            res_id: 0,
            attrs: Default::default(),
        };
        let recv = self
            .nf_sock
            .send(NftMsg::Getset, NlmF::DUMP, NlPayload::Payload(req))
            .context("failed to send to nft")?;

        Ok(NftSetIter { recv })
    }

    pub(super) fn parse_nft_set(&self, set: &NftSet) -> Result<NftSetCounterIter> {
        let attrs = [
            NlattrBuilder::default()
                .nla_type(
                    AttrTypeBuilder::default()
                        .nla_type(NftaSetElemList::Table)
                        .build()?,
                )
                .nla_payload(set.table.as_str())
                .build()?,
            NlattrBuilder::default()
                .nla_type(
                    AttrTypeBuilder::default()
                        .nla_type(NftaSetElemList::Set)
                        .build()?,
                )
                .nla_payload(set.name.as_str())
                .build()?,
        ];
        let req = Nfgenmsg::<NftaSetElemList> {
            family: set.family,
            version: NFNETLINK_V0,
            res_id: 0,
            attrs: GenlBuffer::from_iter(attrs),
        };
        let recv = self
            .nf_sock
            .send(NftMsg::Getsetelem, NlmF::DUMP, NlPayload::Payload(req))
            .context("failed to send to nft")?;

        Ok(NftSetCounterIter {
            recv,
            cur_nlmsg: None,
            cur_attr: 0,
            cur_elem: 0,
        })
    }
}
