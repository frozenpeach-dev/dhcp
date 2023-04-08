use std::{collections::HashMap, net::Ipv4Addr};

use byteorder::{LittleEndian, ByteOrder, BigEndian};

use crate::{leases::ip_subnet::Ipv4Subnet, netutils::hw_addr::HardwareAddress, packet::dhcp_options::DhcpOption, allocators::allocator::{Allocator, AllocationDraft}};

use super::static_allocation::StaticAllocation;

struct StaticAllocator<'a> {
    
    subnets: HashMap<usize, &'a mut Ipv4Subnet>,
    authoritative_on: HashMap<usize, &'a mut Ipv4Subnet>,
    subnet_map: HashMap<(u32, u32), usize>,
    registry: HashMap<HardwareAddress, (usize, StaticAllocation)>,
    count: usize

}

impl<'a> Allocator for StaticAllocator<'a> {
    fn allocate(
        &mut self, 
        cid: HardwareAddress
    ) -> Option<AllocationDraft> {
        
        let record = match self.registry.get(&cid) {
            Some(record) => record,
            None => { return None; }
        }; 

        let ip_addr = record.1
            .options()
            .options
            .get(&u8::from(DhcpOption::RequestedIP(vec![])))?;

        if let DhcpOption::RequestedIP(ip_addr) = ip_addr {
            let ip_addr = BigEndian::read_u32(&ip_addr); 
            Some(AllocationDraft::new(Ipv4Addr::from(ip_addr), record.1.options().clone()))
        } else {
            None
        }

    }

    fn seal_allocation(
        &mut self, 
        draft: AllocationDraft
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
            .options
            .get(&u8::from(DhcpOption::SubnetMask(vec![])))
            .ok_or(())?;
        let requested_ip = alloc.options()
            .options
            .get(&u8::from(DhcpOption::RequestedIP(vec![])))
            .ok_or(())?;

        if let (DhcpOption::RequestedIP(requested_ip), DhcpOption::SubnetMask(subnet_mask)) = (requested_ip, subnet_mask) {
            // Wrong size for ipv4 addr
            if !((requested_ip.len() == 4) | (subnet_mask.len() == 4)) { return Err(()); }

            let requested_ip = BigEndian::read_u32(&requested_ip); 
            let subnet_mask = BigEndian::read_u32(&subnet_mask);

            let prefix = subnet_mask.count_ones();
            let network_ip = requested_ip & subnet_mask;

            let internal_subnet_id = self.subnet_map.get(&(network_ip, prefix)).unwrap(); 
            
            let subnet: &mut Ipv4Subnet = self.subnets
                .get_mut(internal_subnet_id)
                .or_else(|| { self.authoritative_on.get_mut(internal_subnet_id) }).unwrap();

            subnet.force_allocate(Ipv4Addr::from(requested_ip))?;
            self.registry.insert(alloc.cid(), (*internal_subnet_id, alloc));
            return Ok(());
        }; 
        Err(())
    }

    pub fn remove_static_allocation(
        &mut self,
        alloc: HardwareAddress
    ) -> Result<(), ()> {
        
        let (internal_subnet_id, alloc) = self.registry.get(&alloc).unwrap();

        let ip_addr = alloc.options().options.get(&u8::from(DhcpOption::RequestedIP(vec![]))).ok_or(())?;

        if let DhcpOption::RequestedIP(ip_addr) = ip_addr {

            let subnet: &mut Ipv4Subnet = self.subnets
                .get_mut(internal_subnet_id).or_else(|| { self.authoritative_on.get_mut(internal_subnet_id) }).unwrap();

            if ip_addr.len() == 4 {
                let ip_addr = BigEndian::read_u32(ip_addr);
                subnet.free_static_alloc(Ipv4Addr::from(ip_addr))?;
                self.registry.remove(&alloc.cid());
                return Ok(())
            };
        } 

        Err(())
    }

}

#[cfg(test)]
mod tests {
    use crate::packet::dhcp_options::DhcpOptions;

    use super::*;


    #[test]
    fn test_static_alloc_creation() {
        let mut static_allocator = StaticAllocator::new();
        let mut subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
        static_allocator.register_subnet(&mut subnet);
        let mut options = DhcpOptions::new();
        options.add(DhcpOption::RequestedIP(vec![192, 168, 0, 3]));
        options.add(DhcpOption::SubnetMask(vec![255, 255, 255, 0]));
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
        options.add(DhcpOption::RequestedIP(vec![192, 168, 0, 3]));
        options.add(DhcpOption::SubnetMask(vec![255, 255, 255, 0]));
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
        options.add(DhcpOption::RequestedIP(vec![192, 168, 0, 3]));
        options.add(DhcpOption::SubnetMask(vec![255, 255, 255, 0]));
        static_allocator.register_static_allocation(
            StaticAllocation::new(
                HardwareAddress::broadcast(), 
                options
        )).unwrap();

        let draft = static_allocator.allocate(HardwareAddress::broadcast()).unwrap();

        assert!(draft.ip_addr() == Ipv4Addr::new(192, 168, 0, 3));
    }

    #[test]
    fn test_static_allocate_options() {
        let mut static_allocator = StaticAllocator::new();
        let mut subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
        static_allocator.register_subnet(&mut subnet);
        let mut options = DhcpOptions::new();
        options.add(DhcpOption::RequestedIP(vec![192, 168, 0, 3]));
        options.add(DhcpOption::SubnetMask(vec![255, 255, 255, 0]));
        options.add(DhcpOption::LogServer(vec![0xf, 0xe, 3]));
        static_allocator.register_static_allocation(
            StaticAllocation::new(
                HardwareAddress::broadcast(), 
                options
        )).unwrap();

        let draft = static_allocator.allocate(HardwareAddress::broadcast()).unwrap();
        let log_server = draft.options().options.get(&u8::from(DhcpOption::LogServer(vec![]))).unwrap();
        if let DhcpOption::LogServer(log_server) = log_server {
            assert!(log_server == &vec![0xf, 0xe, 3]);
        }
    }

} 
