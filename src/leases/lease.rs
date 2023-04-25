use std::net::Ipv4Addr;

use chrono::{Duration, Utc, DateTime};

use crate::netutils::hw_addr::HardwareAddress;

use super::ip_subnet::Ipv4Subnet;

#[derive(Clone)]
pub struct LeaseV4<'a> {
    addr: Ipv4Addr,
    subnet: &'a Ipv4Subnet,
    t_begin: DateTime<Utc>,
    t_end: DateTime<Utc>,
    hw_addr: HardwareAddress,
    cid: HardwareAddress, 
    hostname: String

}

impl<'a> LeaseV4<'a> {

    /// Create a new `LeaseV4` from given parameters
    ///
    /// The [`Ipv4Addr`] must be allocated
    /// in the correct [`Ipv4Subnet`] before the 
    /// creation of the `LeaseV4`
    ///
    /// # Examples:
    ///
    /// ```
    /// let subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
    /// let lease = LeaseV4::new(
    ///     Ipv4Addr::new(192, 168, 0, 3),
    ///     &subnet,
    ///     Duration::hours(8),
    ///     HardwareAddress::broadcast(),
    ///     HardwareAddress::broadcast(),
    ///     String::from("test_lease"),
    /// );
    /// assert!(lease.hostname == "test_lease");
    /// assert!(lease.addr() == Ipv4Addr::new(192, 168, 0, 3));
    /// ```

    pub fn new(
        addr: Ipv4Addr,
        subnet: &'a Ipv4Subnet,
        duration: Duration,
        hw_addr: HardwareAddress,
        cid: HardwareAddress,
        hostname: String
    ) -> Result<Self, ()> {

        if !subnet.contains(addr) { return Err(()); };

        let t_begin = Utc::now();
        let t_end = t_begin + duration;
        Ok(Self { addr, subnet, t_begin, t_end, hw_addr, cid, hostname })
    }

    /// Returns the remaining [`Duration`] on the `LeaseV4`
    /// 
    /// Returns a [`Duration`] of zero if the lease already ended
    ///
    /// # Examples:
    ///
    /// ```
    /// let subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
    /// let lease = LeaseV4::new(
    ///     Ipv4Addr::new(192, 168, 0, 3),
    ///     subnet,
    ///     Duration::hours(8),
    ///     HardwareAddress::broadcast(),
    ///     HardwareAddress::broadcast(),
    ///     String::from("test_lease"),
    /// );
    /// assert!(lease.remaining() < Duration::hours(8));
    /// ```

    pub fn remaining(
        &self
    ) -> Duration {
        if self.t_end - Utc::now() < Duration::zero() { return Duration::zero(); };
        self.t_end - Utc::now()
    }

    /// Extends the remaining [`Duration`] on this `LeaseV4`
    ///
    /// Returns an error if the lease already expired.
    ///
    /// # Examples: 
    ///
    /// ```
    /// let subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
    /// let mut lease = LeaseV4::new(
    ///     Ipv4Addr::new(192, 168, 0, 3),
    ///     subnet,
    ///     Duration::hours(8),
    ///     HardwareAddress::broadcast(),
    ///     HardwareAddress::broadcast(),
    ///     String::from("test_lease"),
    /// );
    /// lease.extend(Duration::hours(2))
    /// assert!(lease.remaining() > Duration::hours(8));
    /// ```

    pub fn extend(
        &mut self,
        time_to_add: Duration
    ) -> Result<(), ()> {
        if Utc::now() > self.t_end { return Err(()); };
        self.t_end += time_to_add;

        Ok(())
    }

    pub fn hostname(
        &self
    ) -> &str {
        self.hostname.as_ref()
    }

    pub fn hostname_mut(
        &mut self
    ) -> &mut String {
        &mut self.hostname
    }

    pub fn addr(
        &self
    ) -> Ipv4Addr {
        self.addr
    }

    pub fn cid(
        &self
    ) -> HardwareAddress {
        self.cid
    }

    pub fn hw_addr(
        &self
    ) -> HardwareAddress {
        self.hw_addr
    }

    pub fn subnet(
        &self
    ) -> &Ipv4Subnet {
        self.subnet
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_lease_creation() {
        let subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
        let lease = LeaseV4::new(
            Ipv4Addr::new(192, 168, 0, 3),
            &subnet,
            Duration::hours(8),
            HardwareAddress::broadcast(),
            HardwareAddress::broadcast(),
            String::from("test_lease"),
        ).unwrap();

        assert!(lease.hostname == "test_lease");
        assert!(lease.addr() == Ipv4Addr::new(192, 168, 0, 3));
    }

    #[test]
    fn test_lease_time_extend() {
        let subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
        let mut lease = LeaseV4::new(
            Ipv4Addr::new(192, 168, 0, 3),
            &subnet,
            Duration::hours(8),
            HardwareAddress::broadcast(),
            HardwareAddress::broadcast(),
            String::from("test_lease"),
        ).unwrap();
        assert!(lease.remaining() < Duration::hours(8));
        lease.extend(Duration::hours(2)).unwrap();
        assert!(lease.remaining() > Duration::hours(9));
    }

}
