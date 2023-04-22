use std::{net::Ipv4Addr, collections::HashMap};

use crate::packet::dhcp_options::DhcpOptions;


/// `Ipv4Subnet` provides an abstraction layer over 
/// IP v4 subnets, to help manage such subnets.
pub struct Ipv4Subnet {

    network_addr: Ipv4Addr,
    alloc_ptr: u32,
    released: Vec<Ipv4Addr>,
    force_allocated: HashMap<Ipv4Addr, usize>,
    prefix: u8,
    options: DhcpOptions,

}

impl Ipv4Subnet {

    /// Creates a new `Ipv4Subnet` from a given
    /// network address and a CIDR prefix (0-32)
    ///
    /// # Examples:
    ///
    /// ```
    /// let subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
    /// ```

    pub fn new(network_addr: Ipv4Addr, prefix: u8) -> Self {
        Self { network_addr, alloc_ptr: 1, released: Vec::new(), force_allocated: HashMap::new(), prefix, options: DhcpOptions::new()}
    }

    /// Returns the network address corresponding to the
    /// `Ipv4Subnet`
    ///
    /// # Examples: 
    ///
    /// ```
    /// let subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
    /// assert!(subnet.network() == Ipv4Addr::new(192, 168, 0, 0));
    /// ```
    pub fn network(&self) -> Ipv4Addr {
        self.network_addr
    }

    /// Returns the broadcast address corresponding to the
    /// `Ipv4Subnet`
    ///
    /// # Examples: 
    ///
    /// ```
    /// let subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
    /// assert!(subnet.broadcast() == Ipv4Addr::new(192, 168, 0, 255));
    /// ```

    pub fn broadcast(&self) -> Ipv4Addr {
        let wildcard_bits_count = 32 - self.prefix;
        let network_bytes = u32::from(self.network_addr);

        let broadcast_bytes = network_bytes | ((2 << (wildcard_bits_count - 1)) - 1);

        Ipv4Addr::from(broadcast_bytes)
    }

    /// Returns the number of IP addresses belonging to this
    /// `Ipv4Subnet`.
    ///
    /// # Examples : 
    ///
    /// ```
    /// let subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
    /// assert!(subnet.count() == 256);
    /// ```
    pub fn count(&self) -> u32 {
        let wc_bytes_count = 32 - self.prefix;

        2 << (wc_bytes_count - 1)
    }

    /// Returns the number of IP addresses in this `Ipv4Subnet`
    /// that have been allocated.
    ///
    /// # Examples:
    ///
    /// ```
    /// let mut subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
    /// subnet.allocate();
    /// assert!(subnet.allocated_count() == 1);
    /// ```

    pub fn allocated_count(&self) -> u32 {
        self.alloc_ptr - 1 - self.released.len() as u32
    }

    /// Check if a given [`Ipv4Addr`] belongs to 
    /// this `Ipv4Subnet`.
    ///
    /// # Examples: 
    ///
    /// ```
    /// let subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
    /// assert!(subnet.contains(Ipv4Addr::new(192, 168, 0, 3)));
    /// ```

    pub fn contains(&self, ip: Ipv4Addr) -> bool {
        (u32::from(self.network_addr) <= u32::from(ip)) && (u32::from(self.broadcast()) >= u32::from(ip))
    }

    /// Check if a given [`Ipv4Addr`] has been allocated
    /// in that subnet. Returns false if it is yet to be 
    /// allocated, or if it does not belong to this
    /// `Ipv4Subnet`.
    ///
    /// # Examples :
    ///
    /// ```
    /// let mut subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
    /// subnet.allocate();
    /// assert!(!subnet.is_free(Ipv4Addr::new(192, 168, 0, 1)));
    /// assert!(subnet.is_free(Ipv4Addr::new(192, 168, 0, 3)));
    /// assert!(!subnet.is_free(Ipv4Addr::new(192, 168, 1, 0)));
    /// ```

    pub fn is_free(&self, ip: Ipv4Addr) -> bool {
        let cnt_from_nw = u32::from(ip) - u32::from(self.network_addr);
        self.contains(ip) & ((cnt_from_nw >= self.alloc_ptr) | (self.released.contains(&ip))) & !self.force_allocated.contains_key(&ip)
    }

    /// De-allocate a given [`Ipv4Addr`].
    /// Returns an error if it does not belong to this `Ipv4Subnet`,
    /// or if it has not been allocated yet.
    ///
    /// # Examples: 
    ///
    /// ```
    /// let mut subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24); 
    /// subnet.allocate();
    /// assert!(subnet.free(Ipv4Addr::new(192, 168, 0, 1)).is_ok());
    /// assert!(!subnet.free(Ipv4Addr::new(192, 168, 0, 2)).is_ok());
    /// assert!(!subnet.free(Ipv4Addr::new(192, 168, 1, 0)).is_ok());
    /// ```

    pub fn free(&mut self, ip: Ipv4Addr) -> Result<(), ()> {
        if !self.contains(ip) { return Err(()); };
        if self.is_free(ip) {
            return Err(());
        }; 

        self.released.push(ip);
        Ok(())
    }

    /// Remove and de-allocate a previously introduced static
    /// allocation.
    ///
    /// Returns an error if the given [`Ipv4Addr`] was
    /// not statically allocated.
    ///
    /// # Examples:
    ///
    /// ```
    /// let mut subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
    /// subnet.force_allocate(Ipv4Addr::new(192, 168, 0, 10));
    /// assert!(subnet.free_static_alloc(Ipv4Addr::new(192, 168, 0, 10)).is_ok());
    /// ```

    pub fn free_static_alloc(&mut self, ip: Ipv4Addr) -> Result<(), ()> {
        if !self.force_allocated.contains_key(&ip) { return Err(()); };
        self.force_allocated.remove(&ip);

        Ok(())
    }

    /// Allocate an [`Ipv4Addr`] in that `Ipv4Subnet`.
    ///
    /// If any allocated IPs that were previously allocated
    /// have been freed, they are chosen first. Otherwise,
    /// the next never-allocated IP is returned.
    ///
    /// Returns an error if there are no more IP addresses
    /// available.
    ///
    /// # Examples:
    /// 
    /// ```
    /// let mut subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24); 
    /// subnet.allocate();
    /// assert(!subnet.is_free(Ipv4Addr::new(192, 168, 0, 1)));
    /// ```

    pub fn allocate(&mut self) -> Result<Ipv4Addr, ()> {
        if (self.alloc_ptr > self.count()) & self.released.is_empty() {
            return Err(());
        };

        if !self.released.is_empty() {
            return Ok(self.released.pop().unwrap());
        };

        let will_allocate = u32::from(self.network_addr) + self.alloc_ptr;
        self.alloc_ptr += 1;

        Ok(Ipv4Addr::from(will_allocate))
    }

    /// Performs a static allocation on the given [`Ipv4Addr`].
    ///
    /// Returns an error is the given [`Ipv4Addr`] is currently
    /// allocated.
    ///
    /// # Examples:
    ///
    /// ```
    /// let mut subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
    /// subnet.force_allocate(Ipv4Addr::new(192, 168, 0, 5));
    /// assert!(!subnet.is_free(Ipv4Addr::new(192, 168, 0, 5)));
    /// ```

    pub fn force_allocate(&mut self, ip: Ipv4Addr) -> Result<(), ()> {
        if !self.is_free(ip) { return Err(()); };

        self.force_allocated.insert(ip, 1);

        Ok(())
    }

    pub fn prefix(&self) -> u8 {
        self.prefix
    }

    pub fn options(&self) -> &DhcpOptions {
        &self.options
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::Ipv4Subnet;


    #[test]
    fn test_broadcast_addr() {
        let subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
        assert!(subnet.broadcast() == Ipv4Addr::new(192, 168, 0, 255));
    }

    #[test]
    fn test_subnet_contains() {
        let subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
        assert!(subnet.contains(Ipv4Addr::new(192, 168, 0, 3)));
        assert!(!subnet.contains(Ipv4Addr::new(192, 168, 1, 0)));
    }
    
    #[test]
    fn test_subnet_count() {
        let subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
        assert!(subnet.count() == 256);
    }

    #[test]
    fn test_allocated_count() {
        let mut subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
        subnet.allocate().unwrap();
        dbg!(subnet.allocated_count());
        assert!(subnet.allocated_count() == 1);
    }

    #[test]
    fn test_subnet_is_free() {
        let mut subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
        subnet.allocate().unwrap();
        subnet.force_allocate(Ipv4Addr::new(192, 168, 0, 5)).unwrap();
        assert!(!subnet.is_free(Ipv4Addr::new(192, 168, 0 ,5)));
        assert!(!subnet.is_free(Ipv4Addr::new(192, 168, 0, 1)));
        assert!(subnet.is_free(Ipv4Addr::new(192, 168, 0, 2)));
        assert!(!subnet.is_free(Ipv4Addr::new(192, 168, 1, 0)));
    }

    #[test]
    fn test_subnet_free() {
        let mut subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24); 
        subnet.allocate().unwrap();
        assert!(subnet.free(Ipv4Addr::new(192, 168, 0, 1)).is_ok());
        assert!(!subnet.free(Ipv4Addr::new(192, 168, 1, 0)).is_ok());
        assert!(!subnet.free(Ipv4Addr::new(192, 168, 0, 2)).is_ok());
    }

    #[test]
    fn test_static_free() {
        let mut subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24); 

        subnet.force_allocate(Ipv4Addr::new(192, 168, 0, 5)).unwrap();
        assert!(subnet.free_static_alloc(Ipv4Addr::new(192, 168, 0, 5)).is_ok());
        assert!(subnet.force_allocate(Ipv4Addr::new(192, 168, 0, 5)).is_ok());
    }

    #[test]
    fn test_subnet_allocation() {
        let mut subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24); 
        let first_ip = subnet.allocate().unwrap();

        assert!(first_ip == Ipv4Addr::new(192, 168, 0, 1));
        subnet.allocate().unwrap();
        assert!(subnet.free(Ipv4Addr::new(192, 168, 0, 1)).is_ok());
        let last = subnet.allocate().unwrap();
        assert!(last == Ipv4Addr::new(192, 168, 0, 1));
    }

    #[test]
    fn test_static_allocation() {
        let mut subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24); 

        subnet.force_allocate(Ipv4Addr::new(192, 168, 0, 5)).unwrap();
        assert!(!subnet.is_free(Ipv4Addr::new(192, 168, 0 ,5)));
        assert!(!subnet.force_allocate(Ipv4Addr::new(192, 168, 0, 5)).is_ok());
    }

}
