use std::fs;

use serde::{Serialize, Deserialize};

use crate::leases::ip_subnet::Ipv4Subnet;

#[derive(Serialize, Deserialize, Debug)]
pub struct SubnetCfg {
    subnets: Vec<Ipv4Subnet>
}

pub fn load_subnet_cfg() -> Result<SubnetCfg, std::io::Error> {

    let cfg = fs::read_to_string("config/subnets.yml")?;
    Ok(serde_yaml::from_str(&cfg).unwrap())

}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::load_subnet_cfg;

    #[test]
    fn test_load_subnet_cfg() {
        let subnets = load_subnet_cfg();
        let subnet = subnets.unwrap()
            .subnets
            .pop().unwrap();

        assert!(subnet.network() == Ipv4Addr::new(192, 168, 0, 0));
        assert!(subnet.prefix() == 24);
    }

}
