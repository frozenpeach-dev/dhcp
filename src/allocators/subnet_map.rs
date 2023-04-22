use std::{collections::BTreeMap, net::Ipv4Addr, cmp::Ordering, cell::RefCell, rc::Rc};

use crate::leases::ip_subnet::Ipv4Subnet;   

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
    prefix: u8
}

impl CidrSubnet {

    pub fn new(
        network_addr: u32,
        prefix: u8
    ) -> Self { 
        Self { 
            network_addr, 
            prefix } 
        }

    pub fn contains(&self,
        ip: Ipv4Addr
    ) -> bool {
        (self.network_addr <= u32::from(ip)) && (u32::from(self.broadcast()) >= u32::from(ip))
    }

    pub fn broadcast(
        &self
    ) -> Ipv4Addr {
        let wildcard_bits_count = 32 - self.prefix;
        let network_bytes = self.network_addr;

        let broadcast_bytes = network_bytes | ((2 << (wildcard_bits_count - 1)) - 1);

        Ipv4Addr::from(broadcast_bytes)
    }
}

impl PartialEq for CidrSubnet {
    fn eq(&self,
        other: &Self
    ) -> bool {
        (self.network_addr == other.network_addr) & (self.prefix == other.prefix)
    }
}

impl Eq for CidrSubnet {}

impl PartialOrd for CidrSubnet {
    fn partial_cmp(
        &self,
        other: &Self
    ) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CidrSubnet {
    fn cmp(
        &self,
        other: &Self
    ) -> std::cmp::Ordering {
        self.network_addr.cmp(&other.network_addr)
    }
}

pub struct SubnetV4Map {

    subnets: BTreeMap<CidrSubnet, Rc<RefCell<Ipv4Subnet>>>,

}

impl SubnetV4Map {

    pub fn new()
        -> Self {
        SubnetV4Map { subnets: BTreeMap::new() }
    }

    pub fn insert_subnet(
        &mut self,
        subnet: Rc<RefCell<Ipv4Subnet>>
    ) {
        self.subnets.insert(CidrSubnet::new(u32::from(subnet.borrow().network()), subnet.borrow().prefix()), subnet.clone());
    }

    pub fn get_subnet(&self, subnet: CidrSubnet) -> Option<&Rc<RefCell<Ipv4Subnet>>>{
        self.subnets.get(&subnet)
    }

    pub fn get_matching_subnet(
        &self,
        ip: Ipv4Addr
    ) -> Option<Rc<RefCell<Ipv4Subnet>>> {
        let available_subnets: Vec<&CidrSubnet> = self.subnets.keys().collect();
        let subnet = available_subnets.binary_search_by(|elem| {
            if elem.contains(ip) { 
                return Ordering::Equal; 
            }
            else if ip > elem.broadcast() {
                return Ordering::Less
            }
            Ordering::Greater
            
        });
        let subnet = available_subnets.get(subnet.unwrap())?;
        let subnet = *(*subnet);

        Some(self.subnets.get(&subnet)?.clone())
    }

}

#[cfg(test)]
mod tests {
    use std::{net::Ipv4Addr, cell::RefCell, rc::Rc};

    use crate::leases::ip_subnet::Ipv4Subnet;

    use super::SubnetV4Map;

    #[test]
    fn test_get_matching_subnet() {
        let mut map = SubnetV4Map::new();
        let subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
        let subnet2 = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 1, 0), 24);
        let subnet3 = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 3, 0), 24);
        map.insert_subnet(Rc::new(RefCell::new(subnet)));
        map.insert_subnet(Rc::new(RefCell::new(subnet2)));
        map.insert_subnet(Rc::new(RefCell::new(subnet3)));

        assert!(map.get_matching_subnet(Ipv4Addr::new(192, 168, 0, 5)).unwrap().borrow().network() == Ipv4Addr::new(192, 168, 0, 0));
        assert!(map.get_matching_subnet(Ipv4Addr::new(192, 168, 1, 5)).unwrap().borrow().network() == Ipv4Addr::new(192, 168, 1, 0));
        assert!(map.get_matching_subnet(Ipv4Addr::new(192, 168, 3, 5)).unwrap().borrow().network() == Ipv4Addr::new(192, 168, 3, 0));
    }

    #[bench]
    fn bench_subnet_insertion(b: &mut test::Bencher) {
        b.iter(|| {
            let mut map = SubnetV4Map::new();
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
        b.iter(|| {
            let mut map = SubnetV4Map::new();
            for j in 0..=255 {
                for i in 0..=255 {
                    let subnet = Ipv4Subnet::new(Ipv4Addr::new(192, j, i, 0), 24);
                    map.insert_subnet(Rc::new(RefCell::new(subnet)));
                }
            };

            for i in 0..=255 {
                let first_byte: u8 = rand::random();
                let last_byte: u8 = rand::random();
                assert!(map.get_matching_subnet(Ipv4Addr::new(192, i, first_byte, last_byte)).unwrap().borrow().network() == Ipv4Addr::new(192, i, first_byte, 0));
                
            }
        })
    }   

}
