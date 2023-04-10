use std::net::Ipv4Addr;

use chrono::{Duration, Utc, DateTime};

use crate::netutils::hw_addr::HardwareAddress;

pub struct LeaseV4 {

    addr: Ipv4Addr,
    t_begin: DateTime<Utc>,
    t_end: DateTime<Utc>,
    hw_addr: HardwareAddress,
    cid: HardwareAddress, 
    hostname: String

}

impl LeaseV4 {

    /// Create a new `LeaseV4` from given parameters
    ///
    /// # Examples:
    ///
    /// ```
    /// let lease = LeaseV4::new(
    ///     Ipv4Addr::new(192, 168, 0, 3),
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
        duration: Duration,
        hw_addr: HardwareAddress,
        cid: HardwareAddress,
        hostname: String
    ) -> Self {
        let t_begin = Utc::now();
        let t_end = t_begin + duration;
        Self { addr, t_begin, t_end, hw_addr, cid, hostname }
    }

    /// Returns the remaining [`Duration`] on the `LeaseV4`
    /// 
    /// Returns a [`Duration`] of zero if the lease already ended
    ///
    /// # Examples:
    ///
    /// ```
    /// let lease = LeaseV4::new(
    ///     Ipv4Addr::new(192, 168, 0, 3),
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
    /// let mut lease = LeaseV4::new(
    ///     Ipv4Addr::new(192, 168, 0, 3),
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
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_lease_creation() {
        let lease = LeaseV4::new(
            Ipv4Addr::new(192, 168, 0, 3),
            Duration::hours(8),
            HardwareAddress::broadcast(),
            HardwareAddress::broadcast(),
            String::from("test_lease"),
        );

        assert!(lease.hostname == "test_lease");
        assert!(lease.addr() == Ipv4Addr::new(192, 168, 0, 3));
    }

    #[test]
    fn test_lease_time_extend() {
        let mut lease = LeaseV4::new(
            Ipv4Addr::new(192, 168, 0, 3),
            Duration::hours(8),
            HardwareAddress::broadcast(),
            HardwareAddress::broadcast(),
            String::from("test_lease"),
        );

        assert!(lease.remaining() < Duration::hours(8));
        lease.extend(Duration::hours(2)).unwrap();
        assert!(lease.remaining() > Duration::hours(9));
    }

}
