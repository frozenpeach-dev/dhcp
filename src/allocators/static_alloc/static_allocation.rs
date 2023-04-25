use std::net::Ipv4Addr;
use crate::{packet::dhcp_options::DhcpOptions, netutils::hw_addr::HardwareAddress};


pub struct StaticAllocation {
    cid: HardwareAddress,
    ip_addr: Ipv4Addr,
    options: DhcpOptions
}

impl StaticAllocation {
    pub fn new(cid: HardwareAddress, ip_addr: Ipv4Addr, options: DhcpOptions) -> Self { Self { cid, ip_addr, options } }

    pub fn cid(&self) -> HardwareAddress {
        self.cid
    }

    pub fn options(&self) -> &DhcpOptions {
        &self.options
    }
}
