use std::{collections::HashMap, net::Ipv4Addr};



use crate::{leases::ip_subnet::Ipv4Subnet, netutils::hw_addr::HardwareAddress, packet::{dhcp_packet::DhcpMessage}, allocators::allocator::{Allocator, AllocationDraft}};

use super::static_allocation::StaticAllocation;

struct StaticAllocator<'a> {
    
    subnets: HashMap<usize, &'a mut Ipv4Subnet>,
    authoritative_on: HashMap<usize, &'a mut Ipv4Subnet>,
    subnet_map: HashMap<(u32, u32), usize>,
    registry: HashMap<HardwareAddress, (usize, StaticAllocation)>,
    count: usize

}

impl<'a> Allocator<'a> for StaticAllocator<'a> {
    fn allocate(
        &mut self, 
        msg: DhcpMessage
    ) -> Option<AllocationDraft> 
    {

        let request = match msg {
            DhcpMessage::DhcpDiscover(packet) => packet,
            DhcpMessage::DhcpRequest(packet) => packet,
            _ => { return None; },
        };


        // The following lines are an absurdity. Client identifier is by no mean of fixed length,
        // nor always correspond to a so called HardwareAddress. 
        //
        // TO CHANGE ASAP
        if let Some(cid) = request.options.client_identifier() {
            let cidc = cid.clone();
            let client_id: &[u8; 16] = cidc[..16].try_into().unwrap();
            let record = self.registry.get(&HardwareAddress::new(*client_id))?; 

            let ip_addr = record.1
                .options()
                .requested_ip();

            if let Some(ip_addr) = ip_addr {
                let ip_addr = u32::from(ip_addr); 
                return Some(AllocationDraft::new(Ipv4Addr::from(ip_addr), record.1.options().clone()))
            } else {
                return None
            }
        };

        None
        
    }

    fn seal_allocation(
        &mut self, 
        _draft: AllocationDraft
    ) -> Result<(), ()> {
        Ok(())
    }
}

impl<'a> StaticAllocator<'a> {

    pub fn new()
        -> Self {
        Self { 
            subnets: HashMap::new(), 
            subnet_map: HashMap::new(),
            authoritative_on: HashMap::new(), 
            registry: HashMap::new(),
            count: 0
        }
    }

    pub fn register_subnet(
        &mut self,
        subnet: &'a mut Ipv4Subnet
    ) {
        self.subnet_map.insert((subnet.network().into(), subnet.prefix() as u32), self.count);
        self.subnets.insert(self.count, subnet);
        self.count += 1;
    }

    pub fn register_authoritative_subnet(
        &mut self,
        subnet: &'a mut Ipv4Subnet
    ) {
        self.subnet_map.insert((subnet.network().into(), subnet.prefix() as u32), self.count);
        self.authoritative_on.insert(self.count, subnet);
        self.count += 1;
    }

    pub fn register_static_allocation(
        &mut self,
        alloc: StaticAllocation
    ) -> Result<(), ()> {
        let subnet_mask = alloc.options()
            .subnet_mask()
            .ok_or(())?;
        let requested_ip = alloc.options()
            .requested_ip()
            .ok_or(())?;

            let requested_ip = u32::from(requested_ip); 
            let subnet_mask = u32::from(subnet_mask);

            let prefix = subnet_mask.count_ones();
            let network_ip = requested_ip & subnet_mask;

            let internal_subnet_id = self.subnet_map.get(&(network_ip, prefix)).unwrap(); 
            
            let subnet: &mut Ipv4Subnet = self.subnets
                .get_mut(internal_subnet_id)
                .or_else(|| { self.authoritative_on.get_mut(internal_subnet_id) }).unwrap();

            subnet.force_allocate(Ipv4Addr::from(requested_ip))?;
            self.registry.insert(alloc.cid(), (*internal_subnet_id, alloc));
            Ok(())
    }

    pub fn remove_static_allocation(
        &mut self,
        alloc: HardwareAddress
    ) -> Result<(), ()> {
        
        let (internal_subnet_id, alloc) = self.registry.get(&alloc).unwrap();

        let ip_addr = alloc.options().requested_ip().ok_or(())?;

        let subnet: &mut Ipv4Subnet = self.subnets
                .get_mut(internal_subnet_id).or_else(|| { self.authoritative_on.get_mut(internal_subnet_id) }).unwrap();

        let ip_addr = u32::from(ip_addr);
        subnet.free_static_alloc(Ipv4Addr::from(ip_addr))?;
        self.registry.remove(&alloc.cid());
        Ok(())

    }

}

#[cfg(test)]
mod tests {
    use fp_core::core::packet::PacketType;

    use crate::packet::{dhcp_options::DhcpOptions, dhcp_packet::DhcpV4Packet};

    use super::*;


    #[test]
    fn test_static_alloc_creation() {
        let mut static_allocator = StaticAllocator::new();
        let mut subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
        static_allocator.register_subnet(&mut subnet);
        let mut options = DhcpOptions::new();
        options.set_requested_ip(Some(Ipv4Addr::new(192, 168, 0, 3)));
        options.set_subnet_mask(Some(Ipv4Addr::new(255, 255, 255, 0)));
        static_allocator.register_static_allocation(
            StaticAllocation::new(
                HardwareAddress::broadcast(), 
                options
        )).unwrap();

        assert!(!subnet.is_free(Ipv4Addr::new(192, 168, 0, 3)));
    }

    #[test]
    fn test_static_alloc_removal() {
        let mut static_allocator = StaticAllocator::new();
        let mut subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
        static_allocator.register_subnet(&mut subnet);
        let mut options = DhcpOptions::new();
        options.set_requested_ip(Some(Ipv4Addr::new(192, 168, 0, 3)));
        options.set_subnet_mask(Some(Ipv4Addr::new(255, 255, 255, 0)));
        static_allocator.register_static_allocation(
            StaticAllocation::new(
                HardwareAddress::broadcast(), 
                options
        )).unwrap();

        static_allocator.remove_static_allocation(HardwareAddress::broadcast()).unwrap();
        assert!(subnet.is_free(Ipv4Addr::new(192, 168, 0, 3)));
    }

    #[test]
    fn test_static_allocate() {
        let mut static_allocator = StaticAllocator::new();
        let mut subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
        static_allocator.register_subnet(&mut subnet);
        let mut options = DhcpOptions::new();
        options.set_requested_ip(Some(Ipv4Addr::new(192, 168, 0, 3)));
        options.set_subnet_mask(Some(Ipv4Addr::new(255, 255, 255, 0)));
        static_allocator.register_static_allocation(
            StaticAllocation::new(
                HardwareAddress::broadcast(), 
                options
        )).unwrap();

        let mut buf = vec![0u8; 240];
        let mut option: Vec<u8> = vec![61, 16, 0xf,0xf,0xf,0xf,0xf,0xf,0,0,0,0,0,0,0,0,0,0];
        buf.append(&mut option);
        let dhcp_packet = DhcpV4Packet::from_raw_bytes(buf.as_slice());
        let draft = static_allocator.allocate(DhcpMessage::DhcpDiscover(dhcp_packet)).unwrap();

        assert!(draft.ip_addr() == Ipv4Addr::new(192, 168, 0, 3));
    }

    #[test]
    fn test_static_allocate_options() {
        let mut static_allocator = StaticAllocator::new();
        let mut subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
        static_allocator.register_subnet(&mut subnet);
        let mut options = DhcpOptions::new();
        options.set_requested_ip(Some(Ipv4Addr::new(192, 168, 0, 3)));
        options.set_subnet_mask(Some(Ipv4Addr::new(255, 255, 255, 0)));
        options.set_log_server(Some(vec![Ipv4Addr::new(10, 1, 1, 3)]));
        static_allocator.register_static_allocation(
            StaticAllocation::new(
                HardwareAddress::broadcast(), 
                options
        )).unwrap();

        let mut buf = vec![0u8; 240];
        let mut option: Vec<u8> = vec![61, 16, 0xf,0xf,0xf,0xf,0xf,0xf,0,0,0,0,0,0,0,0,0,0];
        buf.append(&mut option);
        let dhcp_packet = DhcpV4Packet::from_raw_bytes(buf.as_slice());
        let draft = static_allocator.allocate(DhcpMessage::DhcpDiscover(dhcp_packet)).unwrap();

        let log_server = draft.options().log_server().unwrap();
        assert!(log_server.len() == 1);
        assert!(*log_server.get(0).unwrap() == Ipv4Addr::new(10, 1, 1, 3));
    }

} 
