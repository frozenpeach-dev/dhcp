use std::{net::Ipv4Addr, cell::RefCell, rc::Rc};

use log::trace;

use crate::{leases::ip_subnet::Ipv4Subnet, allocators::{allocator::{Allocator, AllocationDraft}, subnet_map::SubnetV4Map}, packet::{dhcp_packet::{DhcpMessage, DhcpV4Packet} }};


struct DynamicAllocator {
    
    subnet_map: SubnetV4Map, 
        
}

impl Allocator for DynamicAllocator {
    fn allocate(
        & mut self,
        msg: DhcpMessage
    ) -> Option<AllocationDraft> {
        let request = match msg {
            DhcpMessage::DhcpDiscover(packet) => packet,
            DhcpMessage::DhcpRequest(packet) => packet,
            _ => { return None; },
        };

        let subnet = self.get_client_subnet(&request)?;
        let mut subnet = subnet.borrow_mut();
        let options = subnet.options().clone();

        if let Some(req_ip) = request.options.requested_ip() {
            if subnet.is_free(req_ip) {
                subnet.force_allocate(req_ip).ok()?;
                return Some(AllocationDraft::new(req_ip, options));
            } 
        }

        let ip_addr = subnet.allocate().ok()?;
        drop(subnet);
        Some(AllocationDraft::new(ip_addr, options))
    }


    fn seal_allocation(&mut self, _draft: AllocationDraft) -> Result<(), ()> {
        todo!()
    }
}

impl DynamicAllocator {

    pub fn new()
        -> Self {
        Self { 
            subnet_map: SubnetV4Map::new(), 
        }
    }

    fn get_client_subnet(
        & mut self,
        packet: &DhcpV4Packet
    ) -> Option<Rc<RefCell<Ipv4Subnet>>> {

        let bootp_relay_ip = packet.giaddr;

        // might require to be more specific and allocate an ip
        // on the exact same subnet the dhcp server is in
        if bootp_relay_ip == Ipv4Addr::new(0, 0, 0, 0) {
            if let Some(req_ip) = packet.options.requested_ip() {
                return self.subnet_map
                    .get_matching_subnet(req_ip)
                    .or_else(|| {
                        trace!("DHCP Message received from an unknown subnet.");
                        return None;
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

    pub fn register_subnet(
        &mut self,
        subnet: Rc<RefCell<Ipv4Subnet>>
    ) {
        self.subnet_map.insert_subnet(subnet) 
    }

}

#[cfg(test)]
mod tests {
    use fp_core::core::packet::PacketType;

    use super::*;
    const DHCP_PACKET: [u8; 304]  = [
         0x01, 0x01, 0x06, 0x00, 0x5d, 0x14, 0xd3, 0x27, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 
         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x54, 0x3a, 0xd6, 0x35,
         0x76, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
         0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x63, 0x82, 0x53, 0x63,
         0x35, 0x01, 0x03, 0x3c, 0x0c, 0x75, 0x64, 0x68, 0x63, 0x70, 0x63, 0x31, 0x2e, 0x32, 0x31, 0x2e,
         0x31, 0x32, 0x04, 0xc0, 0xa8, 0x00, 0x11, 0x39, 0x02, 0x02, 0x40, 0x37, 0x08, 0x01, 0x03, 0x1a,
         0xfc, 0x2b, 0x2a, 0x06, 0x0c, 0x3d, 0x07, 0x01, 0x54, 0x3a, 0xd6, 0x35, 0x76, 0x08, 0x0c, 0x07,
         0x53, 0x61, 0x6d, 0x73, 0x75, 0x6e, 0x67, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    #[test]
    fn test_simple_allocation() {
   
        let subnet = Rc::new(RefCell::new(Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24)));
        let mut allocator = DynamicAllocator::new();
        allocator.register_subnet(subnet.clone());
        let packet = DhcpV4Packet::from_raw_bytes(&DHCP_PACKET);
        let dhcp_msg = DhcpMessage::DhcpDiscover(packet.clone());

        assert!(packet.options.requested_ip().unwrap() == Ipv4Addr::new(192, 168, 0, 17));

        let draft = allocator.allocate(dhcp_msg).unwrap();
        let sub = subnet.borrow();
        assert!(!sub.is_free(Ipv4Addr::new(192, 168, 0, 17)));
        assert!(draft.ip_addr() == Ipv4Addr::new(192, 168, 0, 17));
         
    }
    #[test]
    fn test_double_allocation() {
        let subnet = Rc::new(RefCell::new(Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24)));
        let allocator = Rc::new(RefCell::new(DynamicAllocator::new()));
        let mut allocator_mut = allocator.borrow_mut();
        allocator_mut.register_subnet(subnet.clone());
        let packet = DhcpV4Packet::from_raw_bytes(DHCP_PACKET.as_slice());
        let dhcp_msg = DhcpMessage::DhcpDiscover(packet.clone());

        assert!(packet.options.requested_ip().unwrap() == Ipv4Addr::new(192, 168, 0, 17));

        allocator_mut.allocate(dhcp_msg.clone()).unwrap();
        let draft2 = allocator_mut.allocate(dhcp_msg);
        let sub = subnet.borrow();
        assert!(!sub.is_free(Ipv4Addr::new(192, 168, 0, 17)));
        assert!(!draft2.is_none());
        assert!(draft2.unwrap().ip_addr() != Ipv4Addr::new(192, 168, 0, 17))
 
    }

}
