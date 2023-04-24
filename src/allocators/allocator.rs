use std::net::Ipv4Addr;

use crate::{packet::{dhcp_options::DhcpOptions, dhcp_packet::DhcpMessage}};

#[derive(Debug)]
pub struct AllocationDraft {
    ip_addr: Ipv4Addr,
    options: DhcpOptions,
}

impl AllocationDraft {
    pub fn new(ip_addr: Ipv4Addr, options: DhcpOptions) -> Self { Self { ip_addr, options } }

    pub fn ip_addr(&self) -> Ipv4Addr {
        self.ip_addr
    }

    pub fn options_mut(&mut self) -> &mut DhcpOptions {
        &mut self.options
    }

    pub fn options(&self) -> &DhcpOptions {
        &self.options
    }
}

pub trait Allocator {
    fn allocate(&mut self, request: DhcpMessage) -> Option<AllocationDraft>;
    fn seal_allocation(&mut self, draft: AllocationDraft) -> Result<(), ()>;
}
