use crate::{packet::dhcp_options::DhcpOptions, netutils::hw_addr::HardwareAddress};


pub struct StaticAllocation {
    cid: HardwareAddress,
    options: DhcpOptions
}

impl StaticAllocation {
    pub fn new(cid: HardwareAddress, options: DhcpOptions) -> Self { Self { cid, options } }

    pub fn cid(&self) -> HardwareAddress {
        self.cid
    }

    pub fn options(&self) -> &DhcpOptions {
        &self.options
    }
}
