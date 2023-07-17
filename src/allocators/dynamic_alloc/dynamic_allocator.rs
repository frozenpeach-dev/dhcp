use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};

use log::trace;

use crate::{
    allocators::{
        allocator::{AllocationDraft, Allocator},
        subnet_map::SubnetV4Map,
    },
    leases::ip_subnet::Ipv4Subnet,
    packet::dhcp_packet::DhcpV4Packet,
};

pub(crate) struct DynamicAllocator {
    subnet_map: SubnetV4Map,
}

impl Allocator for DynamicAllocator {
    /// Dynamically allocates an [`Ipv4Addr`] upon
    /// handling a `DhcpDiscover` request.
    ///
    /// If the `requested_ip` field in the [`DhcpOptions`]
    /// associated to the message is field, it first tries
    /// to allocate that IP.
    ///
    /// In case of failure, it then tries to allocate a
    /// random [`Ipv4Addr`] in the client's subnet.
    ///
    /// Returns an [`AllocationDraft`] if it successfully
    /// managed to reserve an address.
    ///
    /// # Examples:
    ///
    /// ```
    /// let subnet = Rc::new(RefCell::new(Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24)));
    /// let mut allocator = DynamicAllocator::new();
    /// allocator.register_subnet(subnet.clone());
    /// let draft = allocator.allocate(dhcp_msg);
    /// ```

    fn allocate(&mut self, request: &DhcpV4Packet) -> Option<AllocationDraft> {
        let subnet = self.get_client_subnet(request)?;
        let mut subnet = subnet.lock().unwrap();
        let options = subnet.options().clone();

        // TODO: Check if client has a prior allocation (expired or not) and use that ip

        let mut draft: AllocationDraft;

        match request.options.requested_ip() {
            Some(req_ip) if subnet.is_free(req_ip) => {
                subnet.force_allocate(req_ip).ok()?;
                draft = AllocationDraft::new(req_ip, options);
            }

            _ => {
                let ip_addr = subnet.allocate().ok()?;
                draft = AllocationDraft::new(ip_addr, options);
            }
        };

        if let Some(request_time) = request.options.lease_time() {
            if let Some(subnet_time) = subnet.options().lease_time() {
                if (request_time < 3 * subnet_time) & (request_time > subnet_time / 5) {
                    draft.options_mut().set_lease_time(Some(request_time));
                }
            }
        }

        Some(draft)
    }

    fn seal_allocation(&mut self, _draft: AllocationDraft) -> Result<(), ()> {
        todo!()
    }
}

impl DynamicAllocator {
    pub fn new() -> Self {
        Self {
            subnet_map: SubnetV4Map::new(),
        }
    }

    fn get_client_subnet(&mut self, packet: &DhcpV4Packet) -> Option<Arc<Mutex<Ipv4Subnet>>> {
        let bootp_relay_ip = packet.giaddr;

        // might require to be more specific and allocate an ip
        // on the exact same subnet the dhcp server is in
        if bootp_relay_ip == Ipv4Addr::new(0, 0, 0, 0) {
            if let Some(req_ip) = packet.options.requested_ip() {
                return self.subnet_map.get_matching_subnet(req_ip).or_else(|| {
                    trace!("DHCP Message received from an unknown subnet.");
                    None
                });
            };
            return None;
        };

        self.subnet_map
            .get_matching_subnet(bootp_relay_ip)
            .or_else(|| {
                trace!("DHCP Message received from an unknown subnet.");
                None
            })
    }

    pub fn register_subnet(&mut self, subnet: Arc<Mutex<Ipv4Subnet>>) {
        self.subnet_map.insert_subnet(subnet)
    }
}

#[cfg(test)]
mod tests {
    use fp_core::core::packet::PacketType;

    use super::*;
    const DHCP_PACKET: [u8; 304] = [
        0x01, 0x01, 0x06, 0x00, 0x5d, 0x14, 0xd3, 0x27, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x54, 0x3a,
        0xd6, 0x35, 0x76, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x63, 0x82, 0x53, 0x63,
        0x35, 0x01, 0x03, 0x3c, 0x0c, 0x75, 0x64, 0x68, 0x63, 0x70, 0x63, 0x31, 0x2e, 0x32, 0x31,
        0x2e, 0x31, 0x32, 0x04, 0xc0, 0xa8, 0x00, 0x11, 0x39, 0x02, 0x02, 0x40, 0x37, 0x08, 0x01,
        0x03, 0x1a, 0xfc, 0x2b, 0x2a, 0x06, 0x0c, 0x3d, 0x07, 0x01, 0x54, 0x3a, 0xd6, 0x35, 0x76,
        0x08, 0x0c, 0x07, 0x53, 0x61, 0x6d, 0x73, 0x75, 0x6e, 0x67, 0xff, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
    ];

    #[test]
    fn test_simple_allocation() {
        let subnet = Arc::new(Mutex::new(Ipv4Subnet::new(
            Ipv4Addr::new(192, 168, 0, 0),
            24,
        )));
        let mut allocator = DynamicAllocator::new();
        allocator.register_subnet(subnet.clone());
        let packet = DhcpV4Packet::from_raw_bytes(&DHCP_PACKET);

        assert!(packet.options.requested_ip().unwrap() == Ipv4Addr::new(192, 168, 0, 17));

        let draft = allocator.allocate(&packet.clone()).unwrap();
        let sub = subnet.lock().unwrap();
        assert!(!sub.is_free(Ipv4Addr::new(192, 168, 0, 17)));
        assert!(draft.ip_addr() == Ipv4Addr::new(192, 168, 0, 17));
    }
    #[test]
    fn test_double_allocation() {
        let subnet = Arc::new(Mutex::new(Ipv4Subnet::new(
            Ipv4Addr::new(192, 168, 0, 0),
            24,
        )));
        let allocator = Arc::new(Mutex::new(DynamicAllocator::new()));
        let mut allocator_mut = allocator.lock().unwrap();
        allocator_mut.register_subnet(subnet.clone());
        let packet = DhcpV4Packet::from_raw_bytes(DHCP_PACKET.as_slice());

        assert!(packet.options.requested_ip().unwrap() == Ipv4Addr::new(192, 168, 0, 17));

        allocator_mut.allocate(&packet.clone()).unwrap();
        let draft2 = allocator_mut.allocate(&packet);
        let sub = subnet.lock().unwrap();
        assert!(!sub.is_free(Ipv4Addr::new(192, 168, 0, 17)));
        assert!(draft2.is_some());
        assert!(draft2.unwrap().ip_addr() != Ipv4Addr::new(192, 168, 0, 17))
    }
}
