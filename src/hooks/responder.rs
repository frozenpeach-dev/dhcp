use std::sync::{Mutex, Arc};

use fp_core::{hooks::{hook_registry::HookClosure, typemap::TypeMap}, core::{packet::PacketContext, errors::HookError}};

use crate::packet::dhcp_packet::DhcpV4Packet;


pub fn responder_hook() {

    let responder_hook = HookClosure(Box::new(
        |services: Arc<Mutex<TypeMap>>, context: &mut PacketContext<DhcpV4Packet, DhcpV4Packet>| {

            match context.get_input().options.message_type().unwrap() {
                1 => {
                    _handle_dhcp_discover(services, context) 
                }
                3 => {
                    _handle_dhcp_request(services, context)
                }
            }

        }
    ));

}

fn _handle_dhcp_discover (
    services: Arc<Mutex<TypeMap>>,
    context: &mut PacketContext<DhcpV4Packet, DhcpV4Packet>
) -> Result<isize, HookError> {

    let output = context.get_mut_output();

    output.op = 2;
    output.htype = 1;
    output.options.set_server_identifier(None);

}

fn _handle_dhcp_request (
    services: Arc<Mutex<TypeMap>>,
    context: &mut PacketContext<DhcpV4Packet, DhcpV4Packet>
) -> Result<isize, HookError> {

}
