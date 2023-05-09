use std::{cell::RefCell, collections::HashMap, net::Ipv4Addr, rc::Rc, sync::{Arc, Mutex}};

use fp_core::storage::data::RuntimeStorage;

use crate::{
    allocators::{
        allocator::{AllocationDraft, Allocator},
        subnet_map::{CidrSubnet, SubnetV4Map},
    },
    leases::ip_subnet::Ipv4Subnet,
    netutils::hw_addr::HardwareAddress,
    packet::dhcp_packet::DhcpMessage, data::data::Data,
};

use super::static_allocation::StaticAllocation;

struct StaticAllocator {
    subnet_map: SubnetV4Map,
    registry: HashMap<HardwareAddress, StaticAllocation>,
}

impl Allocator for StaticAllocator {
    fn allocate(&mut self, msg: DhcpMessage) -> Option<AllocationDraft> {
        let request = match msg {
            DhcpMessage::DhcpDiscover(packet) => packet,
            DhcpMessage::DhcpRequest(packet) => packet,
            _ => {
                return None;
            }
        };

        // The following lines are an absurdity. Client identifier is by no mean of fixed length,
        // nor always correspond to a so called HardwareAddress.
        //
        // TO CHANGE ASAP
        if let Some(cid) = request.options.client_identifier() {
            let cidc = cid.clone();
            let client_id: &[u8; 16] = cidc[..16].try_into().unwrap();
            let record = self.registry.get(&HardwareAddress::new(*client_id))?;

            let ip_addr = record.options().requested_ip();

            if let Some(ip_addr) = ip_addr {
                let ip_addr = u32::from(ip_addr);
                return Some(AllocationDraft::new(
                    Ipv4Addr::from(ip_addr),
                    record.options().clone(),
                ));
            } else {
                return None;
            }
        };

        None
    }

    fn seal_allocation(&mut self, _draft: AllocationDraft) -> Result<(), ()> {
        Ok(())
    }
}

impl StaticAllocator {
    pub fn new(shared_storage : Arc<Mutex<RuntimeStorage<Data>>>) -> Self {
        Self {
            subnet_map: SubnetV4Map::new(shared_storage),
            registry: HashMap::new(),
        }
    }

    pub fn register_subnet(&mut self, subnet: Rc<RefCell<Ipv4Subnet>>) -> Result<(), String>{
        self.subnet_map.insert_subnet(subnet).and_then(|e| Ok(()))
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

        let mut subnet = subnet.borrow_mut();

        subnet.force_allocate(Ipv4Addr::from(requested_ip))?;
        self.registry.insert(alloc.cid(), alloc);
        self.subnet_map.update_subnet(subnet.clone());
        Ok(())
    }

    pub fn remove_static_allocation(&mut self, alloc: HardwareAddress) -> Result<(), ()> {
        let alloc = self.registry.get(&alloc).unwrap();

        let ip_addr = alloc.options().requested_ip().ok_or(())?;

        let subnet = self.subnet_map.get_matching_subnet(ip_addr).ok_or(())?;

        let mut subnet = subnet.borrow_mut();

        let ip_addr = u32::from(ip_addr);
        subnet.free_static_alloc(Ipv4Addr::from(ip_addr))?;
        self.registry.remove(&alloc.cid());
        self.subnet_map.update_subnet(subnet.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use fp_core::{core::packet::PacketType, storage::data::{DbManager, DataPool}};
    use crate::packet::{dhcp_options::DhcpOptions, dhcp_packet::DhcpV4Packet};
    use super::*;
    use std::time;
    #[tokio::test(flavor = "multi_thread")]
    async fn test_static_allocator(){
        let db = DbManager::new(
            String::from("dhcp"),
            String::from("frozenpeach"),
            String::from("poney"),
            String::from("127.0.0.1:3333"),
        );

        let pool : DataPool<Data> = DataPool::new("Subnetv4".to_string(), "(type VARCHAR(255), id BIGINT, network_addr VARCHAR(255), prefix INT, options VARBINARY(255), force_allocated VARCHAR(255), released VARCHAR(255))".to_string());
        let sto_db = Arc::new(Mutex::new(db));
        let storage: RuntimeStorage<Data> = RuntimeStorage::new(sto_db);
        storage.add_pool(pool);
        let storage = Arc::new(Mutex::new(storage));
        let sync = storage.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(time::Duration::from_millis(100)).await;
                sync.lock().unwrap().sync();
            }
        });
        
        println!("Static allocator creation");
        test_static_alloc_creation(storage.clone());

        println!("Static allocator removal");
        test_static_alloc_removal(storage.clone());

        println!("Static allocator allocation");
        test_static_allocate(storage.clone());

        println!("Static allocator options");
        test_static_allocate_options(storage.clone());


    }
    

    
    fn test_static_alloc_creation(storage : Arc<Mutex<RuntimeStorage<Data>>>) {
        let mut static_allocator = StaticAllocator::new(storage.clone());
        let subnet = Rc::new(RefCell::new(Ipv4Subnet::new(
            Ipv4Addr::new(192, 168, 0, 0),
            24,
        )));
        static_allocator.register_subnet(subnet.clone()).unwrap();
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
        let subnet = static_allocator.subnet_map.get_matching_subnet(Ipv4Addr::new(192, 168, 0, 17)).unwrap();
        assert!(!subnet.borrow().is_free(Ipv4Addr::new(192, 168, 0, 3)));
    }

    fn test_static_alloc_removal(storage : Arc<Mutex<RuntimeStorage<Data>>>) {
        let mut static_allocator = StaticAllocator::new(storage.clone());
        let subnet = Rc::new(RefCell::new(Ipv4Subnet::new(
            Ipv4Addr::new(192, 168, 0, 0),
            24,
        )));
        static_allocator.register_subnet(subnet.clone()).unwrap();
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
        let subnet = static_allocator.subnet_map.get_matching_subnet(Ipv4Addr::new(192, 168, 0, 17)).unwrap();
        assert!(subnet.borrow().is_free(Ipv4Addr::new(192, 168, 0, 3)));
    }

    fn test_static_allocate(storage : Arc<Mutex<RuntimeStorage<Data>>>) {
        let mut static_allocator = StaticAllocator::new(storage.clone());
        let subnet = Rc::new(RefCell::new(Ipv4Subnet::new(
            Ipv4Addr::new(192, 168, 0, 0),
            24,
        )));
        static_allocator.register_subnet(subnet).unwrap();
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
        let draft = static_allocator
            .allocate(DhcpMessage::DhcpDiscover(dhcp_packet))
            .unwrap();
        let subnet = static_allocator.subnet_map.get_matching_subnet(Ipv4Addr::new(192, 168, 0, 17)).unwrap();
        assert!(draft.ip_addr() == Ipv4Addr::new(192, 168, 0, 3));
    }

    fn test_static_allocate_options(storage : Arc<Mutex<RuntimeStorage<Data>>>) {
        let mut static_allocator = StaticAllocator::new(storage.clone());
        let subnet = Rc::new(RefCell::new(Ipv4Subnet::new(
            Ipv4Addr::new(192, 168, 0, 0),
            24,
        )));
        static_allocator.register_subnet(subnet).unwrap();
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
        let draft = static_allocator
            .allocate(DhcpMessage::DhcpDiscover(dhcp_packet))
            .unwrap();

        let log_server = draft.options().log_server().unwrap();
        assert!(log_server.len() == 1);
        assert!(*log_server.get(0).unwrap() == Ipv4Addr::new(10, 1, 1, 3));
    }
}
