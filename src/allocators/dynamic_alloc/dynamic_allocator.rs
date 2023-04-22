use std::{collections::HashMap, net::Ipv4Addr, cell::RefCell, rc::Rc};

use log::trace;

use crate::{leases::ip_subnet::Ipv4Subnet, allocators::{allocator::{Allocator, AllocationDraft}, subnet_map::SubnetV4Map}, packet::{dhcp_packet::{DhcpMessage, DhcpV4Packet} }};


struct DynamicAllocator<'a> {
    
    subnets: HashMap<usize, &'a mut Ipv4Subnet>,
    authoritative_on: HashMap<usize, &'a mut Ipv4Subnet>,
    subnet_map: SubnetV4Map, 
    count: usize
        
}

impl<'a> Allocator<'a> for DynamicAllocator<'a> {
    fn allocate(
        &'a mut self,
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

impl<'a> DynamicAllocator<'a> {

    pub fn new()
        -> Self {
        Self { 
            subnets: HashMap::new(), 
            authoritative_on: HashMap::new(), 
            subnet_map: SubnetV4Map::new(), 
            count: 1 
        }
    }

    fn get_client_subnet(
        &'a mut self,
        packet: &DhcpV4Packet
    ) -> Option<Rc<RefCell<Ipv4Subnet>>> {

        let bootp_relay_ip = packet.giaddr;

        // might require to be more specific and allocate an ip
        // on the exact same subnet the dhcp server is in
        if bootp_relay_ip == Ipv4Addr::new(0, 0, 0, 0) {
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

    #[test]
    fn test_client_subnet() {
    }

}
