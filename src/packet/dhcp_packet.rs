use std::{net::Ipv4Addr, time::Duration};

use chrono::{NaiveTime};
use fp_core::core::packet::PacketType;
use itertools::Itertools;

use crate::netutils::hw_addr::HardwareAddress;

use super::dhcp_options::DhcpOptions;

#[derive(Clone, Debug)]
pub struct DhcpV4Packet {
    pub op: u8,
    pub htype : u8,
    pub hlen : u8,
    pub hops : u8,
    pub xid : u32,
    pub secs : Duration,
    pub flags : [u8; 2],
    pub ciaddr : Ipv4Addr,
    pub yiaddr : Ipv4Addr,
    pub siaddr : Ipv4Addr,
    pub giaddr : Ipv4Addr,
    pub chadd : HardwareAddress,
    pub sname : [u8; 64],
    pub file : [u8; 128],
    pub options : DhcpOptions
}

#[derive(Clone)]
pub enum DhcpMessage {
    DhcpDiscover(DhcpV4Packet),
    DhcpRequest(DhcpV4Packet),
    DhcpDecline(DhcpV4Packet),
    DhcpRelease(DhcpV4Packet),
    DhcpInform(DhcpV4Packet),
}

impl PacketType for DhcpV4Packet {
    fn to_raw_bytes(&self) -> &[u8] {
        todo!()
    }

    fn empty() -> Self {
        todo!()
    }

    fn from_raw_bytes(raw : &[u8]) -> Self {
        let mut raw = raw.to_vec();
        let op = raw.remove(0);
        let htype = raw.remove(0);
        let hlen = raw.remove(0);
        let hops = raw.remove(0);
        let next:[u8; 4] = raw.drain(0..4).as_slice().to_owned().try_into().unwrap();
        let xid = u32::from_le_bytes(next);
        let next: [u8; 2] = raw.drain(0..2).as_slice().to_owned().try_into().unwrap();
        // let secs = NaiveTime::from_hms_opt(0, 0, u16::from_le_bytes(next) as u32).unwrap();
        let secs = Duration::from_secs(u16::from_le_bytes(next) as u64);
        
        let flags = raw.drain(0..2).as_slice().to_owned().try_into().unwrap();
        let (a, b, c, d) = raw.drain(0..4).collect_tuple().unwrap();

        let ciaddr = Ipv4Addr::new(a, b, c, d);
        let (a, b, c, d) = raw.drain(0..4).collect_tuple().unwrap();

        let yiaddr = Ipv4Addr::new(a, b, c, d);
        let (a, b, c, d) = raw.drain(0..4).collect_tuple().unwrap();

        let siaddr = Ipv4Addr::new(a, b, c, d);
        let (a, b, c, d) = raw.drain(0..4).collect_tuple().unwrap();

        let giaddr = Ipv4Addr::new(a, b, c, d);
        let next: [u8; 16] = raw.drain(0..16).as_slice().to_owned().try_into().unwrap();
        let chadd = HardwareAddress::new(next);
        let sname: [u8; 64] = raw.drain(0..64).as_slice().try_into().unwrap();
        let file: [u8; 128] = raw.drain(0..128).as_slice().to_vec().try_into().unwrap();
        let _magic_cookie = raw.drain(0..4).as_slice().to_vec();
        let options = DhcpOptions::from(raw.as_slice()); 
        Self { op, htype, hlen, hops, xid, secs, flags, ciaddr, yiaddr, siaddr, giaddr, chadd, sname, file, options }

    }
}
