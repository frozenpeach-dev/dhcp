//! Implements `DhcpOptions`, that represents
//! a set of DHCP options associated to a given
//! context. 
//!
//! It contains every existing valid DHCP option, 
//! but fields can be set to [`None`]

use std::{net::Ipv4Addr, collections::{VecDeque, HashSet}};

use byteorder::{BigEndian, ByteOrder};
use log::trace;
use serde::{Serialize, Deserialize};

/// `DhcpOptions` is used as an abstraction
/// of the available set of options used by DHCP requests,
/// defined in RFC 2132 ( <https://www.rfc-editor.org/rfc/rfc2132> )
/// and beyond.
///
/// Each field corresponds to an existing DHCP Option, and 
/// is wrapped around a Rust [`Option`]. An option
/// that has no reason to by set in a given context
/// will therefore be set to [`None`]
///
/// It implements the [`From`] trait for byte slices
/// to allow for an easy derivation from the underlying
/// byte representation of the options.
///
/// Likewise, bytes slices implement the [`From`] trait
/// for `DhcpOptions` to allow for an easy retrieval
/// of the underlying byte representation.
///
///
/// # Examples:
///
/// Given a byte representation of DHCP Options, you can derive 
/// the corresponding [`DhcpOptions`] :
/// 
/// ```
/// let option_bytes = [ 0x35, 0x01, 0x05, 0x36, 0x04, 0xc0,
///  0xa8, 0x00, 0xfe, 0x33, 0x04, 0x00, 0x00, 0xa8,
///  0xc0, 0x01, 0x04, 0xff, 0xff, 0xff, 0x00, 0x03,
///  0x04, 0xc0, 0xa8, 0x00, 0xfe, 0x06, 0x14, 0x01,
///  0x01, 0x01, 0x01, 0x08, 0x08, 0x08, 0x08, 0x08,
///  0x08, 0x04, 0x04, 0xd0, 0x43, 0xde, 0xde, 0xc0,
///  0xa8, 0x00, 0xfe, 0xff];
///  
///  let options = DhcpOptions::from(option_bytes);
///  ```
/// 
/// You can then easily access various defined options:
///
/// ```
/// assert!(options.message_type().unwrap() == 5);
/// 
/// assert!(options.router_option().unwrap().len() == 1);
/// assert!(options.router_options().unwrap().pop() == Ipv4Addr::new(192, 168, 0, 254));
/// ```
///
/// Or define new options:
///
/// ```
/// options.set_hostname(String::from("My PC"));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DhcpOptions {

    output_order: Option<VecDeque<u8>>,
    defined_options: HashSet<u8>,
    subnet_mask: Option<Ipv4Addr>,
    router_option: Option<Vec<Ipv4Addr>>,
    time_server: Option<Vec<Ipv4Addr>>,
    name_server: Option<Vec<Ipv4Addr>>,
    log_server: Option<Vec<Ipv4Addr>>,
    hostname: Option<String>,
    domain_name: Option<String>,
    broadcast_addr: Option<Ipv4Addr>,
    requested_ip: Option<Ipv4Addr>,
    lease_time: Option<u32>,
    message_type: Option<u8>,
    server_identifier: Option<u32>,
    parameter_request: Option<Vec<u8>>,
    renewal_time: Option<u32>,
    rebinding_time: Option<u32>,
    client_identifier: Option<Vec<u8>>,
    interface_mtu: Option<u16>,
    ntp_servers: Option<Vec<Ipv4Addr>>,
    wpad: Option<String>
    

}

fn _parse_string_type(bytes: &[u8]) -> Option<String> {
    Some(String::from_utf8(bytes.to_vec())
        .unwrap_or_default())
}

fn _parse_ipv4_type(bytes: &[u8]) -> Option<Ipv4Addr> {
    if bytes.len() < 4 {
        trace!("Invalid IPv4 address");
        return None;
    }
    Some(
        Ipv4Addr::from(
            BigEndian::read_u32(&bytes[..4])
        )
    )
}

fn _parse_ipv4_list_type(bytes: &[u8]) -> Option<Vec<Ipv4Addr>> {
    let mut ip_list = Vec::new();

    while !bytes.is_empty() {
        if bytes.len() < 4 {
            break;
        } 
        let ip_bytes = bytes.get(..4).unwrap();
        let ip = Ipv4Addr::from(BigEndian::read_u32(ip_bytes));
        ip_list.push(ip);
    }

    Some(ip_list)
}

impl From<&[u8]> for DhcpOptions {
    fn from(
        value: &[u8]
    ) -> Self {
        let mut data = value.to_vec();
        let mut options = DhcpOptions::new();

        while !data.is_empty() {
            let opt_code = data.remove(0); 
            if opt_code == 0 || opt_code == 255 {
                continue;
            }
            let len = data.remove(0) as usize;
            if data.len() < len{
                break;
            }
            match opt_code {
                1 => {
                    let raw_bytes: Vec<u8> = data.drain(..len).collect();
                    options.set_subnet_mask(
                        _parse_ipv4_type(&raw_bytes)
                    );
                },
                4 => {
                    let raw_bytes: Vec<u8> = data.drain(..len).collect();
                    options.set_time_server(
                        _parse_ipv4_list_type(&raw_bytes)
                    );
                },
                5 => {
                    let raw_bytes: Vec<u8> = data.drain(..len).collect();
                    options.set_name_server(
                        _parse_ipv4_list_type(&raw_bytes)
                    );
                },
                12 => {
                    let raw_bytes: Vec<u8> = data.drain(..len).collect();
                    options.set_hostname(
                        _parse_string_type(&raw_bytes)
                    );
                }
                15 => {
                    let raw_bytes: Vec<u8> = data.drain(..len).collect();
                    options.set_domain_name(
                        _parse_string_type(&raw_bytes)
                    );
                }
                26 => {
                    let raw_bytes: Vec<u8> = data.drain(..len).collect();
                    options.set_interface_mtu(
                        Some(BigEndian::read_u16(&raw_bytes))
                    );
                }
                28 => {
                    let raw_bytes: Vec<u8> = data.drain(..len).collect();
                    options.set_broadcast_addr(
                        _parse_ipv4_type(&raw_bytes)
                    );
                }
                42 => {
                    let raw_bytes: Vec<u8> = data.drain(..len).collect();
                    options.set_time_server(
                        _parse_ipv4_list_type(&raw_bytes)
                    );
                }
                50 => {
                    let raw_bytes: Vec<u8> = data.drain(..len).collect();
                    options.set_requested_ip(
                        _parse_ipv4_type(&raw_bytes)
                    );
                }
                51 => {
                    let requested_lease_time = data.drain(..len).as_slice().to_owned();
                    options.set_lease_time(Some(BigEndian::read_u32(requested_lease_time.as_slice())));
                }
                53 => {
                    let dhcp_code: u8 = data.first().unwrap().to_owned();
                    data.remove(0);
                    if dhcp_code > 9 {
                        trace!("Invalid DHCP Message type");
                        break;
                    }
                    options.set_message_type(Some(dhcp_code));
                }
                54 => {
                    let server_identifier = data.drain(..len).as_slice().to_owned();
                    options.set_server_identifier(Some(BigEndian::read_u32(server_identifier.as_slice())));
                }
                55 => {
                    let requested_codes: Vec<u8> = data.drain(..len).collect();
                    requested_codes.iter().map(|code| {
                        options.add_parameter_request(*code);
                    }).last();
                }
                58 => {
                    let renewal_time_value = data.drain(..len).as_slice().to_owned();
                    options.set_renewal_time(Some(BigEndian::read_u32(renewal_time_value.as_slice())));
                }
                59 => {
                    let rebinding_time = data.drain(..len).as_slice().to_owned();
                    options.set_rebinding_time(Some(BigEndian::read_u32(rebinding_time.as_slice())));
                }
                61 => {
                    let client_id: Vec<u8> = data.drain(..len).collect();
                    options.set_client_identifier(Some(client_id));
                }
                252 => {
                    let raw_bytes: Vec<u8> = data.drain(..len).collect();
                    options.set_wpad(
                        _parse_string_type(&raw_bytes)
                    );
                }

                _ => { data.drain(..len); } 
            }
        }
        options
    }
}

impl From<DhcpOptions> for Vec<u8> {
    fn from(options: DhcpOptions) -> Self {

        let mut buf = Vec::new();
        let mut in_output: Vec<u8> = Vec::new();
        
        // there is probably a better way than to clone
        if let Some(mut output_order) = options.clone().output_order {
            while !output_order.is_empty() {
                let next_option = output_order.pop_front().unwrap();
                in_output.push(next_option);
                _append_option(next_option, &options, &mut buf);
            }
        } 

        for i in options.defined_options
            .iter()
            .filter(|x| {!in_output.contains(x)} ) 
        {
            _append_option(*i, &options, &mut buf); 
        }
        buf.push(0xff);
        buf
    }

}

fn _format_string(str: &String) -> &[u8] {
    str.as_bytes()
}

fn _format_ipv4(addr: Ipv4Addr) -> [u8; 4] {
    u32::to_be_bytes(
        u32::from(addr)
    )
}

fn _format_ipv4_list(addrs: &Vec<Ipv4Addr>) -> Vec<u8> {

    let mut buf = Vec::new();
    addrs.into_iter().map(|x| {
        buf.extend_from_slice(
            &_format_ipv4(*x)
        );
    }).last();
    buf

}

fn _append_option(option_code: u8, options: &DhcpOptions, buffer: &mut Vec<u8>) {

    buffer.push(option_code);

    match option_code {
        1 => {
            let bytes = _format_ipv4(options.subnet_mask().unwrap()); 
            buffer.push(4);
            buffer.extend_from_slice(&bytes);
        }
        4 => {
            let mut bytes = _format_ipv4_list(options.time_server().unwrap());
            buffer.push(bytes.len() as u8);
            buffer.append(&mut bytes);
        }
        5 => {
            let mut bytes = _format_ipv4_list(options.name_server().unwrap());
            buffer.push(bytes.len() as u8);
            buffer.append(&mut bytes);
        }
        12 => {
            let bytes = _format_string(options.hostname().unwrap());
            buffer.push(bytes.len() as u8);
            buffer.extend_from_slice(bytes);
        }
        15 => {
            let bytes = _format_string(options.domain_name().unwrap());
            buffer.push(bytes.len() as u8);
            buffer.extend_from_slice(bytes);
        }
        26 => {
            let bytes = u16::to_be_bytes(options.interface_mtu().unwrap());
            buffer.push(2);
            buffer.extend_from_slice(&bytes);
        }
        28 => {
            let bytes = _format_ipv4(options.broadcast_addr().unwrap()); 
            buffer.push(2);
            buffer.extend_from_slice(&bytes);
        }
        42 => {
            let mut bytes = _format_ipv4_list(options.time_server().unwrap());
            buffer.push(bytes.len() as u8);
            buffer.append(&mut bytes);
        }
        50 => {
            let bytes = _format_ipv4(options.requested_ip().unwrap()); 
            buffer.push(2);
            buffer.extend_from_slice(&bytes);
        }
        51 => {
            let bytes = u32::to_be_bytes(options.lease_time().unwrap());
            buffer.push(4);
            buffer.extend_from_slice(&bytes);
        }
        53 => {
            buffer.push(1);
            buffer.push(options.message_type().unwrap());
        }
        54 => {
            let bytes = u32::to_be_bytes(options.server_identifier().unwrap());
            buffer.push(4);
            buffer.extend_from_slice(&bytes);
        }
        55 => {
            let bytes = options.parameter_request().unwrap();
            buffer.push(bytes.len() as u8);
            buffer.extend(bytes.iter());
        }
        58 => {
            let bytes = u32::to_be_bytes(options.renewal_time().unwrap());
            buffer.push(4);
            buffer.extend_from_slice(&bytes);
        }
        59 => {
            let bytes = u32::to_be_bytes(options.rebinding_time().unwrap());
            buffer.push(4);
            buffer.extend_from_slice(&bytes);
        }
        61 => {
            let bytes = options.client_identifier().unwrap();
            buffer.push(bytes.len() as u8);
            buffer.extend(bytes.iter());
        }
        252 => {
            let bytes = _format_string(options.wpad().unwrap());
            buffer.push(bytes.len() as u8);
            buffer.extend_from_slice(bytes);
        }
        _ => ()
    }

}

impl DhcpOptions {
    pub fn new(
        ) -> Self {
        Self { 
            output_order: None,
            defined_options: HashSet::new(),
            subnet_mask: None,
            router_option: None,
            time_server: None,
            name_server: None,
            log_server: None,
            hostname: None, 
            domain_name: None,
            broadcast_addr: None,
            requested_ip: None,
            lease_time: None,
            message_type: None,
            server_identifier: None,
            parameter_request: None,
            renewal_time: None,
            rebinding_time: None, 
            client_identifier: None,
            interface_mtu: None,
            ntp_servers: None,
            wpad: None,
        } 
    }    



    pub fn subnet_mask(
        &self
    ) -> Option<Ipv4Addr> {
        self.subnet_mask
    }

    pub fn set_subnet_mask(
        &mut self, subnet_mask: Option<Ipv4Addr>
    ) {
        self.defined_options.insert(1);
        self.subnet_mask = subnet_mask;
    }

    pub fn router_option(
        &self
    ) -> Option<&Vec<Ipv4Addr>> {
        self.router_option.as_ref()
    }

    pub fn set_router_option(
        &mut self, 
        router_option: Option<Vec<Ipv4Addr>>
    ) {
        self.defined_options.insert(3);
        self.router_option = router_option;
    }

    pub fn time_server(
        &self
    ) -> Option<&Vec<Ipv4Addr>> {
        self.time_server.as_ref()
    }

    pub fn add_time_server(
        &mut self, time_server: Ipv4Addr
    ) {
        self.defined_options.insert(4);
        match &mut self.time_server {
            Some(ref mut list) => list.push(time_server),
            None => {
                self.time_server = Some(vec![time_server]);
            }
        }
    }

    pub fn name_server(
        &self
    ) -> Option<&Vec<Ipv4Addr>> {
        self.name_server.as_ref()
    }

    pub fn add_name_server(
        &mut self, name_server: Ipv4Addr
    ) {
        match &mut self.name_server {
            Some(ref mut list) => list.push(name_server),
            None => {
                self.time_server = Some(vec![name_server]);
            }
        }
    }

    pub fn hostname(
        &self
    ) -> Option<&String> {
        self.hostname.as_ref()
    }

    pub fn set_hostname(
        &mut self, hostname: Option<String>
    ) {
        self.defined_options.insert(12);
        self.hostname = hostname;
    }

    pub fn domain_name(
        &self
    ) -> Option<&String> {
        self.domain_name.as_ref()
    }

    pub fn set_domain_name(
        &mut self, domain_name: Option<String>
    ) {
        self.defined_options.insert(15);
        self.domain_name = domain_name;
    }

    pub fn broadcast_addr(
        &self
    ) -> Option<Ipv4Addr> {
        self.broadcast_addr
    }

    pub fn set_broadcast_addr(
        &mut self, broadcast_addr: Option<Ipv4Addr>
    ) {
        self.defined_options.insert(28);
        self.broadcast_addr = broadcast_addr;
    }

    pub fn requested_ip(
        &self
    ) -> Option<Ipv4Addr> {
        self.requested_ip
    }

    pub fn set_requested_ip(
        &mut self, requested_ip: Option<Ipv4Addr>
    ) {
        self.defined_options.insert(50);
        self.requested_ip = requested_ip;
    }

    pub fn lease_time(
        &self
    ) -> Option<u32> {
        self.lease_time
    }

    pub fn set_lease_time(
        &mut self, lease_time: Option<u32>
    ) {
        self.defined_options.insert(51);
        self.lease_time = lease_time;
    }

    pub fn message_type(
        &self
    ) -> Option<u8> {
        self.message_type
    }

    pub fn set_message_type(
        &mut self, message_type: Option<u8>
    ) {
        self.defined_options.insert(53);
        self.message_type = message_type;
    }

    pub fn server_identifier(
        &self
    ) -> Option<u32> {
        self.server_identifier
    }

    pub fn set_server_identifier(
        &mut self, 
        server_identifier: Option<u32>
    ) {
        self.defined_options.insert(54);
        self.server_identifier = server_identifier;
    }

    pub fn parameter_request(
        &self
    ) -> Option<&Vec<u8>> {
        self.parameter_request.as_ref()
    }

    pub fn add_parameter_request(
        &mut self,
        parameter_request: u8
    ) {
        self.defined_options.insert(55);
        match &mut self.parameter_request {
            Some(list) => list.push(parameter_request),
            None => {
                self.parameter_request = Some(vec![parameter_request]);
            }
        }
    }

    pub fn renewal_time(
        &self
    ) -> Option<u32> {
        self.renewal_time
    }

    pub fn set_renewal_time(
        &mut self, 
        renewal_time: Option<u32>
    ) {
        self.defined_options.insert(58);
        self.renewal_time = renewal_time;
    }

    pub fn rebinding_time(
        &self
    ) -> Option<u32> {
        self.rebinding_time
    }

    pub fn set_rebinding_time(
        &mut self, rebinding_time: Option<u32>
    ) {
        self.defined_options.insert(59);
        self.rebinding_time = rebinding_time;
    }

    pub fn client_identifier(
        &self
    ) -> Option<&Vec<u8>> {
        self.client_identifier.as_ref()
    }

    pub fn set_client_identifier(
        &mut self, client_identifier: Option<Vec<u8>>
    ) {
        self.defined_options.insert(61);
        self.client_identifier = client_identifier;
    }

    pub fn log_server(
        &self
    ) -> Option<&Vec<Ipv4Addr>> {
        self.log_server.as_ref()
    }

    pub fn set_log_server(
        &mut self,
        log_server: Option<Vec<Ipv4Addr>>
    ) {
        self.defined_options.insert(7);
        self.log_server = log_server;
    }

    pub fn wpad(
        &self
    ) -> Option<&String> {
        self.wpad.as_ref()
    }

    pub fn set_wpad(
        &mut self, wpad: 
        Option<String>
    ) {
        self.wpad = wpad;
    }

    pub fn set_name_server(
        &mut self,
        name_server: Option<Vec<Ipv4Addr>>
    ) {
        self.defined_options.insert(5);
        self.name_server = name_server;
    }

    pub fn set_time_server(
        &mut self,
        time_server: Option<Vec<Ipv4Addr>>
    ) {
        self.defined_options.insert(4);
        self.time_server = time_server;
    }

    pub fn interface_mtu(
        &self
    ) -> Option<u16> {
        self.interface_mtu
    }

    pub fn set_interface_mtu(
        &mut self, 
        interface_mtu: Option<u16>
    ) {
        self.defined_options.insert(26);
        self.interface_mtu = interface_mtu;
    }

    pub fn ntp_servers(
        &self
    ) -> Option<&Vec<Ipv4Addr>> {
        self.ntp_servers.as_ref()
    }

    pub fn set_ntp_servers(
        &mut self,
        ntp_servers: Option<Vec<Ipv4Addr>>
    ) {
        self.defined_options.insert(42);
        self.ntp_servers = ntp_servers;
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    const OPTION_BYTES: [u8; 50] = [
        0x35, 0x01, 0x05, 0x36, 0x04, 0xc0,
        0xa8, 0x00, 0xfe, 0x33, 0x04, 0x00, 0x00, 0xa8,
        0xc0, 0x01, 0x04, 0xff, 0xff, 0xff, 0x00, 0x03,
        0x04, 0xc0, 0xa8, 0x00, 0xfe, 0x06, 0x14, 0x01,
        0x01, 0x01, 0x01, 0x08, 0x08, 0x08, 0x08, 0x08,
        0x08, 0x04, 0x04, 0xd0, 0x43, 0xde, 0xde, 0xc0,
        0xa8, 0x00, 0xfe, 0xff
    ];   

    #[test]
    fn options_from_bytes() {
        let options = DhcpOptions::from(OPTION_BYTES.as_slice());
        dbg!(options.clone());
        assert!(options.message_type().unwrap() == 5);
        assert!(options.subnet_mask().unwrap() == Ipv4Addr::new(255, 255, 255, 0));
        assert!(options.defined_options.contains(&53));
    }

    #[test]
    fn bytes_from_options() {
        let options = DhcpOptions::from(OPTION_BYTES.as_slice());
        let bytes = Vec::from(options);
        assert!(DhcpOptions::from(bytes.as_slice()) == DhcpOptions::from(OPTION_BYTES.as_slice()));
    }

}
