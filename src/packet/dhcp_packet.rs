use std::net::Ipv4Addr;

use chrono::NaiveTime;
use fp_core::core::packet::PacketType;
use itertools::Itertools;

use crate::netutils::hw_addr::HardwareAddress;

use super::dhcp_options::DhcpOptions;

#[derive(Clone)]
pub struct DhcpV4Packet {
    op: u8,
    htype : u8,
    hlen : u8,
    hops : u8,
    xid : u32,
    secs : NaiveTime,
    flags : [u8; 2],
    ciaddr : Ipv4Addr,
    yiaddr : Ipv4Addr,
    siaddr : Ipv4Addr,
    giaddr : Ipv4Addr,
    chadd : HardwareAddress,
    sname : [u8; 64],
    file : [u8; 128],
    options : DhcpOptions
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
        let secs = NaiveTime::from_hms_opt(0, 0, u16::from_le_bytes(next) as u32).unwrap();

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
        let options = DhcpOptions::from(raw); 
        Self { op, htype, hlen, hops, xid, secs, flags, ciaddr, yiaddr, siaddr, giaddr, chadd, sname, file, options }

    }
}
