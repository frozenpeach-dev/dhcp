use std::{fs, net::{Ipv4Addr, IpAddr}};

use pnet::{datalink::NetworkInterface, util::MacAddr};
use serde::{Serialize, Deserialize, Serializer, Deserializer, de};

#[derive(Serialize, Deserialize, Debug)]
pub struct DhcpCfg {
    #[serde(rename = "network")]
    network_cfg: NetworkCfg
}

impl DhcpCfg {
    pub fn network_cfg(&self) -> &NetworkCfg {
        &self.network_cfg
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkCfg {
    #[serde(serialize_with = "_serialize_net_interface")]
    #[serde(deserialize_with = "_deserialize_net_interface")]
    interface: NetworkInterface,
}

impl NetworkCfg {

    /// Returns the [`MacAddr`] corresponding to
    /// the network interface defined in the config.
    ///
    pub fn mac(&self) -> Option<MacAddr> {
        self.interface.mac
    }

    /// Returns an [`Ipv4Addr`] that represents
    /// the subnet corresponding to the 
    /// network interface defined in the config
    /// file.
    ///
    /// # Examples: 
    ///
    /// ```
    /// let cfg = load_main_cfg("tests/main.yml").unwrap();
    /// assert!(cfg.network_cfg.mask().unwrap() == Ipv4Addr::new(255, 0, 0, 0))
    /// ```
    pub fn mask(&self) -> Option<Ipv4Addr> {
        let ip = self.interface.ips
            .iter()
            .filter(|x| {
                x.is_ipv4()
            })
            .next();

        if let Some(IpAddr::V4(ip)) = ip.map(|x| x.mask())
        {
            return Some(ip);
        } else {
           return None; 
        }
    } 

    /// Returns an [`Ipv4Addr`] that represents
    /// the subnet corresponding to the 
    /// network interface defined in the config
    /// file.
    ///
    /// # Examples: 
    ///
    /// ```
    /// let cfg = load_main_cfg("tests/main.yml").unwrap();
    /// assert!(cfg.network_cfg.ipv4().unwrap() == Ipv4Addr::new(127, 0, 0, 1))
    /// ```
    pub fn ipv4(&self) -> Option<Ipv4Addr> {
        let ip = self.interface.ips
            .iter()
            .filter(|x| {
                x.is_ipv4()
            })
            .next();

        if let Some(IpAddr::V4(ip)) = ip.map(|x| x.ip())
        {
            return Some(ip);
        } else {
           return None; 
        }

    }

}

fn _serialize_net_interface<S>(x: &NetworkInterface, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer
{
    s.serialize_str(&x.name)
}

fn _deserialize_net_interface<'de, D>(de: D) -> Result<NetworkInterface, D::Error>
where
    D: Deserializer<'de>
{
    let if_name: &str = de::Deserialize::deserialize(de)?;

    let interface = pnet::datalink::interfaces()
        .into_iter()
        .filter(|iface| iface.name == if_name)
        .next()
        .or_else(|| {
            panic!("Fatal: failed to load main config file \n
                Encountered the following error: \n
                Failed to bind network interface {}", if_name);
        })
        .unwrap();   

    Ok(interface) 
}

pub fn load_main_cfg(path: &str) -> Result<DhcpCfg, std::io::Error> {

    let cfg = fs::read_to_string(path)
        .expect("Fatal: failed to load main config file");
    serde_yaml::from_str(&cfg).map_err(|err| {
        panic!("Fatal: failed to load main config file \n 
                Encountered the following error while trying to parse
                YAML file: {}", err);
    })

}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_load_main_cfg() {
        let cfg = load_main_cfg("tests/main.yml").unwrap();
        assert!(cfg.network_cfg.interface.name == "lo0");
    }

    #[test]
    fn test_load_iface_ipv4() {
        let cfg = load_main_cfg("tests/main.yml").unwrap();
        assert!(cfg.network_cfg.ipv4().unwrap() == Ipv4Addr::new(127, 0, 0, 1));
        assert!(cfg.network_cfg.mask().unwrap() == Ipv4Addr::new(255, 0, 0, 0))
    }


}
