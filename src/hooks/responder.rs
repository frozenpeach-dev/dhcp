use std::sync::{Arc, Mutex};

use fp_core::{
    core::{errors::HookError, packet::PacketContext},
    hooks::{hook_registry::HookClosure, typemap::TypeMap},
};
use log::trace;

use crate::allocators::allocator::Allocator;
use crate::allocators::dynamic_alloc::dynamic_allocator::DynamicAllocator;
use crate::allocators::static_alloc::static_allocator::StaticAllocator;
use crate::{cfg::main_cfg::NetworkCfg, packet::dhcp_packet::DhcpV4Packet};

pub fn responder_hook() -> HookClosure<DhcpV4Packet, DhcpV4Packet> {
    HookClosure(Box::new(
        |services: Arc<Mutex<TypeMap>>, context: &mut PacketContext<DhcpV4Packet, DhcpV4Packet>| {
            match context.get_input().options.message_type().unwrap() {
                1 => _handle_dhcp_discover(services.clone(), context),
                3 => _handle_dhcp_request(services.clone(), context),
                _ => Ok(1),
            }?;
            _fill_dhcp_header(services.clone(), context)?;
            Ok(0)
        },
    ))
}

fn _fill_dhcp_header(
    services: Arc<Mutex<TypeMap>>,
    context: &mut PacketContext<DhcpV4Packet, DhcpV4Packet>,
) -> Result<(), HookError> {
    let input = context.get_input().clone();
    let output = context.get_mut_output();
    let cfg = services.lock().unwrap();
    let net_cfg = cfg
        .get::<Arc<Mutex<NetworkCfg>>>()
        .ok_or(HookError::new("Failed to retrieve network configuration."))?
        .lock()
        .unwrap();

    output.options.set_server_identifier(net_cfg.ipv4());
    drop(net_cfg);
    drop(cfg);
    output.op = 2;
    output.htype = 1;
    output.hlen = 6;
    output.hops = 0;
    output.xid = input.xid;
    output.giaddr = input.giaddr;
    output.chadd = input.chadd;
    output
        .options
        .set_client_identifier(input.options.client_identifier().cloned());

    Ok(())
}

fn _handle_dhcp_discover(
    services: Arc<Mutex<TypeMap>>,
    context: &mut PacketContext<DhcpV4Packet, DhcpV4Packet>,
) -> Result<isize, HookError> {
    let input = context.get_input().clone();
    let output = context.get_mut_output();

    let services_unlocked = services.lock().unwrap();
    let mut static_allocator = services_unlocked
        .get::<Arc<Mutex<StaticAllocator>>>()
        .unwrap()
        .lock()
        .unwrap();
    if let Some(draft) = static_allocator.allocate(&input) {
        output.yiaddr = draft.ip_addr();
        output.options = draft.options().clone();
        output.options.set_message_type(Some(2));
        return Ok(0);
    }
    drop(static_allocator);
    let mut dynamic_allocator = services_unlocked
        .get::<Arc<Mutex<DynamicAllocator>>>()
        .unwrap()
        .lock()
        .unwrap();
    if let Some(draft) = dynamic_allocator.allocate(&input) {
        output.yiaddr = draft.ip_addr();
        output.options = draft.options().clone();
        output.options.set_message_type(Some(2));
        return Ok(0);
    }

    trace!("Unable to allocate IP address, discarding request");

    Ok(0)
}

fn _handle_dhcp_request(
    services: Arc<Mutex<TypeMap>>,
    context: &mut PacketContext<DhcpV4Packet, DhcpV4Packet>,
) -> Result<isize, HookError> {
    Ok(0)
}

#[cfg(test)]
mod tests {
    use std::{
        net::Ipv4Addr,
        sync::{Arc, Mutex},
    };

    use fp_core::{
        core::packet::{PacketContext, PacketType},
        hooks::hook_registry::{Hook, HookClosure, HookRegistry},
    };

    use crate::allocators::static_alloc::static_allocation::StaticAllocation;
    use crate::allocators::static_alloc::static_allocator::StaticAllocator;
    use crate::leases::ip_subnet::Ipv4Subnet;
    use crate::netutils::hw_addr::HardwareAddress;
    use crate::packet::dhcp_options::DhcpOptions;
    use crate::{cfg::main_cfg::load_main_cfg, packet::dhcp_packet::DhcpV4Packet};

    use super::responder_hook;

    const INPUT_DHCP_DISCOVER: [u8; 308] = [
        0x01, 0x01, 0x06, 0x00, 0xab, 0xcd, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x48, 0x55,
        0x19, 0xc8, 0x57, 0x3d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x63, 0x82, 0x53, 0x63,
        0x35, 0x01, 0x01, 0x39, 0x02, 0x05, 0xdc, 0x0c, 0x0a, 0x45, 0x53, 0x50, 0x5f, 0x43, 0x38,
        0x35, 0x37, 0x33, 0x44, 0x37, 0x0c, 0x01, 0x03, 0x1c, 0x06, 0x0f, 0x2c, 0x2e, 0x2f, 0x1f,
        0x21, 0x79, 0x2b, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    #[test]
    fn test_dhcp_discover_responder_dynamic_alloc() {}

    #[test]
    fn test_dhcp_discover_responder_static_alloc() {
        let mut input_packet = DhcpV4Packet::from_raw_bytes(&INPUT_DHCP_DISCOVER);
        input_packet
            .options
            .set_client_identifier(Some(input_packet.chadd.raw.to_vec()));
        let net_cfg = load_main_cfg("tests/main.yml")
            .unwrap()
            .network_cfg()
            .clone();
        let mut static_allocator = StaticAllocator::new();
        let subnet = Arc::new(Mutex::new(Ipv4Subnet::new(
            Ipv4Addr::new(192, 168, 0, 0),
            24,
        )));
        static_allocator.register_subnet(subnet.clone());
        let mut options = DhcpOptions::new();
        options.set_requested_ip(Some(Ipv4Addr::new(192, 168, 0, 3)));
        options.set_subnet_mask(Some(Ipv4Addr::new(255, 255, 255, 0)));
        static_allocator
            .register_static_allocation(StaticAllocation::new(
                input_packet.chadd,
                Ipv4Addr::new(192, 168, 0, 3),
                options,
            ))
            .unwrap();
        let mut registry: HookRegistry<DhcpV4Packet, DhcpV4Packet> = HookRegistry::new();
        registry.register_service(Mutex::new(net_cfg));
        registry.register_service(Mutex::new(static_allocator));
        let mut context: PacketContext<DhcpV4Packet, DhcpV4Packet> =
            PacketContext::from(input_packet);
        registry.register_hook(
            fp_core::core::state::PacketState::Received,
            Hook::new(String::from("responder_hook"), responder_hook(), Vec::new()),
        );
        registry.run_hooks(&mut context).unwrap();

        let output = context.get_output();
        dbg!(output.clone());
        assert_eq!(output.op, 2);
        assert_eq!(output.xid, context.get_input().xid);
        assert_eq!(
            output.options.server_identifier(),
            Some(Ipv4Addr::new(127, 0, 0, 1))
        );
        assert_eq!(output.options.message_type(), Some(2));
    }
}
