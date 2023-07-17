use std::{fs, net::Ipv4Addr};

use log::error;
use serde::{Deserialize, Serialize};

use crate::{leases::ip_subnet::Ipv4Subnet, packet::dhcp_options::DhcpOptions};

#[derive(Serialize, Deserialize, Debug)]
pub struct StaticAllocs {
    #[serde(skip)]
    pub only_static: bool,
    pub allocations: Vec<AllocCfg>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AllocCfg {
    pub ip_addr: Ipv4Addr,
    pub hw_addr: String,
    #[serde(skip)]
    pub options: DhcpOptions,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Ipv4SubnetCfg(Ipv4Subnet, StaticAllocs);

#[derive(Serialize, Deserialize, Debug)]
pub struct SubnetCfg {
    #[serde(rename = "defaults")]
    pub default_options: DhcpOptions,
    pub subnets: Vec<Ipv4SubnetCfg>,
}

pub fn load_subnet_cfg(path: &str) -> Result<SubnetCfg, std::io::Error> {
    let cfg = fs::read_to_string(path).expect("Fatal: failed to load subnets config file");
    serde_yaml::from_str(&cfg).map_err(|err| {
        let error = format!(
            "Fatal: failed to load subnets config file \n 
                Encountered the following error while trying to parse
                YAML file: {}",
            err
        );
        panic!("{}", error);
    })
}

pub fn save_subnet_cfg(path: &str, cfg: SubnetCfg) {
    let data = serde_yaml::to_string(&cfg)
        .map_err(|err| {
            let error = format!(
                "Fatal: failed to load subnets config file \n 
                Encountered the following error while trying to parse
                YAML file: {}",
                err
            );
            error!("{}", error);
        })
        .unwrap_or_default();
    fs::write(path, data)
        .map_err(|err| {
            let error = format!(
                "Fatal: failed to write subnets config file \n
                Encountered the following error while trying to write:
                {}",
                err
            );
            error!("{}", error);
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::load_subnet_cfg;

    #[test]
    fn test_load_subnet_cfg() {
        let subnets = load_subnet_cfg("tests/subnets.yml");
        let subnet = subnets.unwrap().subnets.pop().unwrap();

        assert_eq!(subnet.0.network(), Ipv4Addr::new(192, 168, 0, 0));
        assert_eq!(subnet.0.prefix(), 24);
    }
}
