use std::{
    cell::RefCell,
    cmp::Ordering,
    collections::BTreeMap,
    net::Ipv4Addr,
    rc::Rc,
    sync::{Arc, Mutex},
};

use fp_core::storage::data::RuntimeStorage;
use log::debug;

use crate::{data::data::Data, extract, leases::ip_subnet::Ipv4Subnet};

/// Custom representation of a subnet defined
/// by its CIDR notation (network address + CIDR prefix)
///
/// Used for custom comparaison purposes. It supposes that
/// no subnet overlap each other while comparing. Therefore
/// it is only based on a simple comparaison of network address
///
/// A subnet will be greater than an other subnet if its
/// network address is bigger.
#[derive(Clone, Copy)]
pub struct CidrSubnet {
    network_addr: u32,
    prefix: u8,
}

impl CidrSubnet {
    pub fn new(network_addr: u32, prefix: u8) -> Self {
        Self {
            network_addr,
            prefix,
        }
    }

    pub fn contains(&self, ip: Ipv4Addr) -> bool {
        (self.network_addr <= u32::from(ip)) && (u32::from(self.broadcast()) >= u32::from(ip))
    }

    pub fn broadcast(&self) -> Ipv4Addr {
        let wildcard_bits_count = 32 - self.prefix;
        let network_bytes = self.network_addr;

        let broadcast_bytes = network_bytes | ((2 << (wildcard_bits_count - 1)) - 1);

        Ipv4Addr::from(broadcast_bytes)
    }
}

impl PartialEq for CidrSubnet {
    fn eq(&self, other: &Self) -> bool {
        (self.network_addr == other.network_addr) & (self.prefix == other.prefix)
    }
}

impl Eq for CidrSubnet {}

impl PartialOrd for CidrSubnet {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CidrSubnet {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.network_addr.cmp(&other.network_addr)
    }
}

pub struct SubnetV4Map {
    subnets: BTreeMap<CidrSubnet, u16>,
    storage: Arc<Mutex<RuntimeStorage<Data>>>,
}

impl SubnetV4Map {
    pub fn new(shared_storage: Arc<Mutex<RuntimeStorage<Data>>>) -> Self {
        SubnetV4Map {
            subnets: BTreeMap::new(),
            storage: shared_storage.clone(),
        }
    }

    pub fn update_subnet(&mut self, subnet : Ipv4Subnet) {
        let mut storage = self.storage.lock().unwrap();
        let cidr = CidrSubnet::new(u32::from(subnet.network()), subnet.prefix());
        let old_address = self.subnets.get(&cidr).unwrap();
        let data = Data::Ipv4Subnet(subnet);
        storage.delete(*old_address, "Subnetv4".to_string());
        let new_address = storage.store(data, "Subnetv4".to_string()).unwrap();
        *self.subnets.get_mut(&cidr).unwrap() = new_address;
    }

    pub fn insert_subnet(&mut self, subnet: Rc<RefCell<Ipv4Subnet>>) -> Result<u16, String> {
        let mut storage = self.storage.lock().unwrap();
        let data = subnet.borrow().clone();
        let storage_address = storage.store(
            Data::Ipv4Subnet(data),
            "Subnetv4".to_string(),
        )?;
        self.subnets.insert(
            CidrSubnet::new(
                u32::from(subnet.borrow().network()),
                subnet.borrow().prefix(),
            ),
            storage_address,
        );
        Ok(storage_address)
    }

    pub fn get_subnet(&self, subnet: CidrSubnet) -> Option<Rc<RefCell<Ipv4Subnet>>> {
        // Get address of subnet in storage
        let storage_address = self.subnets.get(&subnet)?;
        // Get subnet from storage
        let storage = self.storage.lock().unwrap();
        let res = storage.get(*storage_address);
        let data : Data;
        match res {
            Err(e) => return None,
            Ok(d) => data = d,
        }
        let subnet = extract!(data, Data::Ipv4Subnet)?;
        let subnet = Rc::new(RefCell::new(subnet));
        Some(subnet)
    }

    pub fn get_matching_subnet(&self, ip: Ipv4Addr) -> Option<Rc<RefCell<Ipv4Subnet>>> {
        let available_subnets: Vec<&CidrSubnet> = self.subnets.keys().collect();

        let subnet = available_subnets.binary_search_by(|elem| {
            if elem.contains(ip) {
                return Ordering::Equal;
            } else if ip > elem.broadcast() {
                return Ordering::Less;
            }
            Ordering::Greater
        });
        let subnet = available_subnets.get(subnet.unwrap())?;
        let subnet = *(*subnet);
        let storage = self.storage.lock().unwrap();
        let address = self.subnets.get(&subnet)?;
        let res = storage.get(*address);
        let data : Data;
        match res {
            Err(e) => return None,
            Ok(d) => data = d,
        }
        
        
        let subnet = extract!(data, Data::Ipv4Subnet)?;
        Some(Rc::new(RefCell::new(subnet)))
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, net::Ipv4Addr, rc::Rc, sync::{Arc, Mutex}};

    use fp_core::storage::data::{DbManager, RuntimeStorage, DataPool};

    use crate::{leases::ip_subnet::Ipv4Subnet, data::data::Data};

    use super::SubnetV4Map;

    #[test]
    fn test_get_matching_subnet() {
        // Initialize storage
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

        let mut map = SubnetV4Map::new(storage.clone());
        let subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
        let subnet2 = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 1, 0), 24);
        let subnet3 = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 3, 0), 24);
        map.insert_subnet(Rc::new(RefCell::new(subnet)));
        map.insert_subnet(Rc::new(RefCell::new(subnet2)));
        map.insert_subnet(Rc::new(RefCell::new(subnet3)));

        assert!(
            map.get_matching_subnet(Ipv4Addr::new(192, 168, 0, 5))
                .unwrap()
                .borrow()
                .network()
                == Ipv4Addr::new(192, 168, 0, 0)
        );
        assert!(
            map.get_matching_subnet(Ipv4Addr::new(192, 168, 1, 5))
                .unwrap()
                .borrow()
                .network()
                == Ipv4Addr::new(192, 168, 1, 0)
        );
        assert!(
            map.get_matching_subnet(Ipv4Addr::new(192, 168, 3, 5))
                .unwrap()
                .borrow()
                .network()
                == Ipv4Addr::new(192, 168, 3, 0)
        );
    }

    #[bench]
    fn bench_subnet_insertion(b: &mut test::Bencher) {
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

        b.iter(|| {
            let mut map = SubnetV4Map::new(storage.clone());
            for j in 0..255 {
                for i in 0..255 {
                    let subnet = Ipv4Subnet::new(Ipv4Addr::new(192, j, i, 0), 24);
                    map.insert_subnet(Rc::new(RefCell::new(subnet)));
                }
            }
        })
    }

    #[bench]
    fn bench_get_matching_subnet(b: &mut test::Bencher) {
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

        b.iter(|| {
            let mut map = SubnetV4Map::new(storage.clone());
            for j in 0..=255 {
                for i in 0..=255 {
                    let subnet = Ipv4Subnet::new(Ipv4Addr::new(192, j, i, 0), 24);
                    map.insert_subnet(Rc::new(RefCell::new(subnet)));
                }
            }

            for i in 0..=255 {
                let first_byte: u8 = rand::random();
                let last_byte: u8 = rand::random();
                assert!(
                    map.get_matching_subnet(Ipv4Addr::new(192, i, first_byte, last_byte))
                        .unwrap()
                        .borrow()
                        .network()
                        == Ipv4Addr::new(192, i, first_byte, 0)
                );
            }
        })
    }
}
