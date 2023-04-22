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
            client_identifier: None 
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
        &mut self, parameter_request: u8
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
        &mut self, renewal_time: Option<u32>
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
                    if data.len() < 4 {
                        trace!("Invalid DHCP packet");
                        break;
                    }
                    let ip_bytes = BigEndian::read_u32(data.drain(..4).as_slice());
                    options.set_subnet_mask(Some(Ipv4Addr::from(ip_bytes))); 
                },
                4 => {
                    let mut raw_bytes: Vec<u8> = data.drain(..len).collect();

                    while !raw_bytes.is_empty() {
                        if raw_bytes.len() < 4 {
                            break;
                        } 
                        let timeserv_ip_bytes = raw_bytes.drain(..4).as_slice().to_owned();
                        let timeserv_ip = Ipv4Addr::from(BigEndian::read_u32(timeserv_ip_bytes.as_slice()));
                        options.add_time_server(timeserv_ip);
                    }
                },
                5 => {
                    let mut raw_bytes: Vec<u8> = data.drain(..len).collect();

                    while !raw_bytes.is_empty() {
                        if raw_bytes.len() < 4 {
                            break;
                        } 
                        // sus double as_slice
                        let nameserver_ip_bytes = raw_bytes.drain(..4).as_slice().to_owned();
                        let nameserver_ip = Ipv4Addr::from(BigEndian::read_u32(nameserver_ip_bytes.as_slice()));
                        options.add_name_server(nameserver_ip);
                    }
                },
                12 => {
                    
                    let raw_bytes: Vec<u8> = data.drain(..len).collect();

                    options.set_hostname(
                        Some(
                            String::from_utf8(raw_bytes)
                                .unwrap_or_default()
                        )
                    );

                }
                15 => {
                    let raw_bytes: Vec<u8> = data.drain(..len).collect();

                    options.set_domain_name(
                        Some(
                            String::from_utf8(raw_bytes)
                                .unwrap_or_default()
                        )
                    );
                }
                28 => {
                    let broadcast_ip_bytes = data.drain(..len).as_slice().to_owned();
                    let broadcast_ip = Ipv4Addr::from(BigEndian::read_u32(broadcast_ip_bytes.as_slice()));

                    options.set_broadcast_addr(Some(broadcast_ip));
                }
                50 => {
                    let requested_ip_bytes = data.drain(..len).as_slice().to_owned();
                    let requested_ip = Ipv4Addr::from(BigEndian::read_u32(requested_ip_bytes.as_slice()));
                    
                    options.set_requested_ip(Some(requested_ip));
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

                _ => { data.drain(..len); } 
            }
        }
        options
    }
}



