use crate::packet::dhcp_packet::DhcpV4Packet;
use crate::{
    allocators::{
        allocator::{AllocationDraft, Allocator},
        subnet_map::{CidrSubnet, SubnetV4Map},
    },
    leases::ip_subnet::Ipv4Subnet,
    netutils::hw_addr::HardwareAddress,
    packet::dhcp_packet::DhcpMessage,
};
use std::sync::{Arc, Mutex};
use std::{cell::RefCell, collections::HashMap, net::Ipv4Addr, rc::Rc};

use super::static_allocation::StaticAllocation;

pub(crate) struct StaticAllocator {
    subnet_map: SubnetV4Map,
    registry: HashMap<HardwareAddress, StaticAllocation>,
}

impl Allocator for StaticAllocator {
    fn allocate(&mut self, request: &DhcpV4Packet) -> Option<AllocationDraft> {
        // The following lines are an absurdity. Client identifier is by no mean of fixed length,
        // nor always correspond to a so called HardwareAddress.
        //
        // TO CHANGE ASAP
        if let Some(cid) = request.options.client_identifier() {
            let cidc = cid.clone();
            let client_id: &[u8; 16] = cidc[..16].try_into().unwrap();
            let record = self.registry.get(&HardwareAddress::new(*client_id))?;

            let ip_addr = record.options().requested_ip();

            return if let Some(ip_addr) = ip_addr {
                let ip_addr = u32::from(ip_addr);
                Some(AllocationDraft::new(
                    Ipv4Addr::from(ip_addr),
                    record.options().clone(),
                ))
            } else {
                None
            };
        };

        None
    }

    fn seal_allocation(&mut self, _draft: AllocationDraft) -> Result<(), ()> {
        Ok(())
    }
}

impl StaticAllocator {
    pub fn new() -> Self {
        Self {
            subnet_map: SubnetV4Map::new(),
            registry: HashMap::new(),
        }
    }

    pub fn register_subnet(&mut self, subnet: Arc<Mutex<Ipv4Subnet>>) {
        self.subnet_map.insert_subnet(subnet)
    }

    pub fn register_static_allocation(&mut self, alloc: StaticAllocation) -> Result<(), ()> {
        let subnet_mask = alloc.options().subnet_mask().ok_or(())?;
        let requested_ip = alloc.options().requested_ip().ok_or(())?;

        let requested_ip = u32::from(requested_ip);
        let subnet_mask = u32::from(subnet_mask);

        let prefix = subnet_mask.count_ones();
        let network_ip = requested_ip & subnet_mask;
        let cidr = CidrSubnet::new(network_ip, prefix as u8);

        let subnet = self.subnet_map.get_subnet(cidr).ok_or(())?;

        let mut subnet = subnet.lock().unwrap();

        subnet.force_allocate(Ipv4Addr::from(requested_ip))?;
        self.registry.insert(alloc.cid(), alloc);
        Ok(())
    }

    pub fn remove_static_allocation(&mut self, alloc: HardwareAddress) -> Result<(), ()> {
        let alloc = self.registry.get(&alloc).unwrap();

        let ip_addr = alloc.options().requested_ip().ok_or(())?;

        let subnet = self.subnet_map.get_matching_subnet(ip_addr).ok_or(())?;

        let mut subnet = subnet.lock().unwrap();

        let ip_addr = u32::from(ip_addr);
        subnet.free_static_alloc(Ipv4Addr::from(ip_addr))?;
        self.registry.remove(&alloc.cid());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use fp_core::core::packet::PacketType;
    use std::sync::Arc;

    use crate::packet::{dhcp_options::DhcpOptions, dhcp_packet::DhcpV4Packet};

    use super::*;

    #[test]
    fn test_static_alloc_creation() {
        let mut static_allocator = StaticAllocator::new();
        let subnet = Arc::new(Mutex::new(Ipv4Subnet::new(
            Ipv4Addr::new(192, 168, 0, 0),
            24,
        )));
        static_allocator.register_subnet(subnet.clone());
        let mut options = DhcpOptions::new();
        options.set_requested_ip(Some(Ipv4Addr::new(192, 168, 0, 3)));
        options.set_subnet_mask(Some(Ipv4Addr::new(255, 255, 255, 0)));
        static_allocator
            .register_static_allocation(StaticAllocation::new(
                HardwareAddress::broadcast(),
                Ipv4Addr::new(192, 168, 0, 3),
                options,
            ))
            .unwrap();

        assert!(!subnet
            .lock()
            .unwrap()
            .is_free(Ipv4Addr::new(192, 168, 0, 3)));
    }

    #[test]
    fn test_static_alloc_removal() {
        let mut static_allocator = StaticAllocator::new();
        let subnet = Arc::new(Mutex::new(Ipv4Subnet::new(
            Ipv4Addr::new(192, 168, 0, 0),
            24,
        )));
        static_allocator.register_subnet(subnet.clone());
        let mut options = DhcpOptions::new();
        options.set_requested_ip(Some(Ipv4Addr::new(192, 168, 0, 3)));
        options.set_subnet_mask(Some(Ipv4Addr::new(255, 255, 255, 0)));
        static_allocator
            .register_static_allocation(StaticAllocation::new(
                HardwareAddress::broadcast(),
                Ipv4Addr::new(192, 168, 0, 3),
                options,
            ))
            .unwrap();

        static_allocator
            .remove_static_allocation(HardwareAddress::broadcast())
            .unwrap();
        assert!(subnet
            .try_lock()
            .unwrap()
            .is_free(Ipv4Addr::new(192, 168, 0, 3)));
    }

    #[test]
    fn test_static_allocate() {
        let mut static_allocator = StaticAllocator::new();
        let subnet = Arc::new(Mutex::new(Ipv4Subnet::new(
            Ipv4Addr::new(192, 168, 0, 0),
            24,
        )));
        static_allocator.register_subnet(subnet);
        let mut options = DhcpOptions::new();
        options.set_requested_ip(Some(Ipv4Addr::new(192, 168, 0, 3)));
        options.set_subnet_mask(Some(Ipv4Addr::new(255, 255, 255, 0)));
        static_allocator
            .register_static_allocation(StaticAllocation::new(
                HardwareAddress::broadcast(),
                Ipv4Addr::new(192, 168, 0, 3),
                options,
            ))
            .unwrap();

        let mut buf = vec![0u8; 240];
        let mut option: Vec<u8> = vec![
            61, 16, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        buf.append(&mut option);
        let dhcp_packet = DhcpV4Packet::from_raw_bytes(buf.as_slice());
        let draft = static_allocator.allocate(&dhcp_packet).unwrap();

        assert_eq!(draft.ip_addr(), Ipv4Addr::new(192, 168, 0, 3));
    }

    #[test]
    fn test_static_allocate_options() {
        let mut static_allocator = StaticAllocator::new();
        let subnet = Arc::new(Mutex::new(Ipv4Subnet::new(
            Ipv4Addr::new(192, 168, 0, 0),
            24,
        )));
        static_allocator.register_subnet(subnet);
        let mut options = DhcpOptions::new();
        options.set_requested_ip(Some(Ipv4Addr::new(192, 168, 0, 3)));
        options.set_subnet_mask(Some(Ipv4Addr::new(255, 255, 255, 0)));
        options.set_log_server(Some(vec![Ipv4Addr::new(10, 1, 1, 3)]));
        static_allocator
            .register_static_allocation(StaticAllocation::new(
                HardwareAddress::broadcast(),
                Ipv4Addr::new(192, 168, 0, 3),
                options,
            ))
            .unwrap();

        let mut buf = vec![0u8; 240];
        let mut option: Vec<u8> = vec![
            61, 16, 0xf, 0xf, 0xf, 0xf, 0xf, 0xf, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        buf.append(&mut option);
        let dhcp_packet = DhcpV4Packet::from_raw_bytes(buf.as_slice());
        let draft = static_allocator.allocate(&dhcp_packet).unwrap();

        let log_server = draft.options().log_server().unwrap();
        assert_eq!(log_server.len(), 1);
        assert_eq!(*log_server.get(0).unwrap(), Ipv4Addr::new(10, 1, 1, 3));
    }
}
