use mac_address::MacAddress;
use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct HardwareAddress {
    #[serde(skip)]
    pub is_mac_address: bool,
    pub raw : [u8; 16]
}

impl HardwareAddress {
    
    pub fn broadcast() -> Self {
        Self::new([0xf,0xf,0xf,0xf,0xf,0xf,0,0,0,0,0,0,0,0,0,0])
    }

    pub fn new(mut raw : [u8; 16]) -> Self{
        let mut i =0;
        raw.reverse();
        while (*raw.get(i).unwrap() == 0) && (i < 9) {
            i+=1
        }
        raw.reverse();
        let mut addr = MacAddress::new([0; 6]);
        let mut is_mac_address = false;
        if i == 9 {
            let bytes : [u8;6] = raw[0..6].try_into().unwrap();
            addr = MacAddress::new(bytes);
            is_mac_address = true;
        }
        Self { is_mac_address, raw: (raw) }

    }
}
