use std::{
    cell::RefCell,
    net::Ipv4Addr,
    rc::Rc,
    sync::{Arc, Mutex},
};

use fp_core::storage::data::RuntimeStorage;
use log::trace;

use crate::{
    allocators::{
        allocator::{AllocationDraft, Allocator},
        subnet_map::SubnetV4Map,
    },
    data::data::Data,
    leases::ip_subnet::Ipv4Subnet,
    packet::dhcp_packet::{DhcpMessage, DhcpV4Packet},
};

struct DynamicAllocator {
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

    fn allocate(&mut self, msg: DhcpMessage) -> Option<AllocationDraft> {
        let request = match msg {
            DhcpMessage::DhcpDiscover(packet) => packet,
            DhcpMessage::DhcpRequest(packet) => packet,
            _ => {
                return None;
            }
        };

        let subnet = self.get_client_subnet(&request)?;
        let mut subnet = subnet.borrow_mut();
        let options = subnet.options().clone();

        if let Some(req_ip) = request.options.requested_ip() {
            if subnet.is_free(req_ip) {
                subnet.force_allocate(req_ip).ok()?;
                self.subnet_map.update_subnet(subnet.clone());
                return Some(AllocationDraft::new(req_ip, options));
            }
        }

        let ip_addr = subnet.allocate().ok()?;
        self.subnet_map.update_subnet(subnet.clone());
        Some(AllocationDraft::new(ip_addr, options))
    }

    fn seal_allocation(&mut self, _draft: AllocationDraft) -> Result<(), ()> {
        todo!()
    }
}

impl DynamicAllocator {
    pub fn new(shared_storage: Arc<Mutex<RuntimeStorage<Data>>>) -> Self {
        Self {
            subnet_map: SubnetV4Map::new(shared_storage),
        }
    }

    fn get_client_subnet(&mut self, packet: &DhcpV4Packet) -> Option<Rc<RefCell<Ipv4Subnet>>> {
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

    pub fn register_subnet(&mut self, subnet: Rc<RefCell<Ipv4Subnet>>) -> Result<(), String> {
        self.subnet_map.insert_subnet(subnet).and(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fp_core::{
        core::packet::PacketType,
        storage::data::{DataPool, DbManager},
    };
    use std::time::{self, Duration};
    use tokio::time::sleep;
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

    #[tokio::test(flavor = "multi_thread")]
    async fn test_dynamic_allocator() {
        let db = DbManager::new(
            String::from("dhcp"),
            String::from("frozenpeach"),
            String::from("poney"),
            String::from("127.0.0.1:3333"),
        );

        let pool : DataPool<Data> = DataPool::new("Subnetv4".to_string(), "(type VARCHAR(255), id BIGINT, network_addr VARCHAR(255), prefix INT, options VARCHAR(3000), force_allocated VARCHAR(255), released VARCHAR(255))".to_string());
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

        println!("test_simple_allocation");
        test_simple_allocation(storage.clone());

        println!("test_double_allocation");
        test_double_allocation(storage.clone());

        sleep(Duration::from_secs(1)).await;
    }

    fn test_simple_allocation(storage: Arc<Mutex<RuntimeStorage<Data>>>) {
        let subnet = Rc::new(RefCell::new(Ipv4Subnet::new(
            Ipv4Addr::new(192, 168, 0, 0),
            24,
        )));
        let mut allocator = DynamicAllocator::new(storage.clone());
        allocator.register_subnet(subnet.clone()).unwrap();
        let packet = DhcpV4Packet::from_raw_bytes(&DHCP_PACKET);
        let dhcp_msg = DhcpMessage::DhcpDiscover(packet.clone());

        assert!(packet.options.requested_ip().unwrap() == Ipv4Addr::new(192, 168, 0, 17));

        let draft = allocator.allocate(dhcp_msg).unwrap();
        let sub = allocator
            .subnet_map
            .get_matching_subnet(Ipv4Addr::new(192, 168, 0, 17))
            .unwrap();
        let sub = sub.borrow();
        assert!(!sub.is_free(Ipv4Addr::new(192, 168, 0, 17)));
        assert!(draft.ip_addr() == Ipv4Addr::new(192, 168, 0, 17));
    }

    fn test_double_allocation(storage: Arc<Mutex<RuntimeStorage<Data>>>) {
        let subnet = Rc::new(RefCell::new(Ipv4Subnet::new(
            Ipv4Addr::new(192, 168, 0, 0),
            24,
        )));
        let allocator = Rc::new(RefCell::new(DynamicAllocator::new(storage.clone())));
        let mut allocator_mut = allocator.borrow_mut();
        allocator_mut.register_subnet(subnet.clone()).unwrap();
        let packet = DhcpV4Packet::from_raw_bytes(DHCP_PACKET.as_slice());
        let dhcp_msg = DhcpMessage::DhcpDiscover(packet.clone());

        assert!(packet.options.requested_ip().unwrap() == Ipv4Addr::new(192, 168, 0, 17));

        allocator_mut.allocate(dhcp_msg.clone()).unwrap();
        let draft2 = allocator_mut.allocate(dhcp_msg);
        let sub = allocator_mut
            .subnet_map
            .get_matching_subnet(Ipv4Addr::new(192, 168, 0, 17))
            .unwrap();
        let sub = sub.borrow();
        assert!(!sub.is_free(Ipv4Addr::new(192, 168, 0, 17)));
        assert!(!draft2.is_none());
        assert!(draft2.unwrap().ip_addr() != Ipv4Addr::new(192, 168, 0, 17))
    }
}
