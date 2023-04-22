use std::net::Ipv4Addr;

use byteorder::{BigEndian, ByteOrder};
use log::trace;


#[derive(Clone, Debug)]
pub struct DhcpOptions {
    
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

impl DhcpOptions {
    pub fn new(
        ) -> Self {
        Self { subnet_mask: None,
            router_option: None,
            time_server: None,
            name_server: None,
            log_server: None,
            hostname: None, domain_name: None,
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
        self.message_type = message_type;
    }

    pub fn server_identifier(
        &self
    ) -> Option<u32> {
        self.server_identifier
    }

    pub fn set_server_identifier(&mut self, server_identifier: Option<u32>) {
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
        self.name_server = name_server;
    }

    pub fn set_time_server(
        &mut self,
        time_server: Option<Vec<Ipv4Addr>>
    ) {
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
        self.ntp_servers = ntp_servers;
    }
}



