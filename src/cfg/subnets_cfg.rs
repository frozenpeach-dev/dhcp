use std::fs;

use log::error;
use serde::{Serialize, Deserialize};


use crate::{leases::ip_subnet::Ipv4Subnet, packet::dhcp_options::DhcpOptions};

#[derive(Serialize, Deserialize, Debug)]
pub struct SubnetCfg {
    #[serde(rename = "defaults")]
    default_options: DhcpOptions,
    subnets: Vec<Ipv4Subnet>
}


pub fn load_subnet_cfg() -> Result<SubnetCfg, std::io::Error> {

    let cfg = fs::read_to_string("config/subnets.yml")
        .expect("Fatal: failed to load subnets config file");
    serde_yaml::from_str(&cfg).map_err(|err| {
        let error = format!("Fatal: failed to load subnets config file \n 
                Encountered the following error while trying to parse
                YAML file: {}", err);
        panic!("{}", error);
    }) 
}

pub fn save_subnet_cfg(cfg: SubnetCfg) {

    let data = serde_yaml::to_string(&cfg).map_err(|err| {
        let error = format!("Fatal: failed to load subnets config file \n 
                Encountered the following error while trying to parse
                YAML file: {}", err);
        error!("{}", error);
    }).unwrap_or_default();
    fs::write("config/subnets.yml", data)
        .map_err(|err| {
            let error = format!("Fatal: failed to write subnets config file \n
                Encountered the following error while trying to write:
                {}", err);
            error!("{}", error);
    }).ok();

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
