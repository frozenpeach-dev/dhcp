use std::sync::{Arc, Mutex, RwLock};

use fp_core::{
    core::{errors::HookError, packet::PacketContext},
    hooks::{hook_registry::HookClosure, typemap::TypeMap},
};

use crate::allocators::static_alloc::static_allocator::StaticAllocator;
use crate::{
    allocators::allocator::Allocator,
    cfg::{main_cfg::DhcpCfg, subnets_cfg::SubnetCfg},
    packet::dhcp_packet::DhcpV4Packet,
};

pub fn allocator_hook() {
    let static_alloc_hook = HookClosure(Box::new(
        |services: Arc<Mutex<TypeMap>>, context: &mut PacketContext<DhcpV4Packet, DhcpV4Packet>| {
            let input = context.get_input();

            if let Some(msg_type) = input.options.message_type() {
                match msg_type {
                    1 => {
                        let services = services.lock().unwrap();
                        let mut _stat_allocator = services
                            .get::<Arc<Mutex<StaticAllocator>>>()
                            .unwrap()
                            .lock()
                            .unwrap();
                        let _subnet_cfg: Arc<Mutex<SubnetCfg>> =
                            services.get::<Arc<Mutex<SubnetCfg>>>().unwrap().clone();

                        match _stat_allocator.allocate(input) {
                            Some(draft) => {
                                let output = context.get_mut_output();
                                output.options = draft.options().clone();
                                output.yiaddr = draft.ip_addr();

                                if output.options.lease_time().is_none() {
                                    output.options.set_lease_time(
                                        _subnet_cfg.lock().unwrap().default_options.lease_time(),
                                    );
                                }

                                // TODO: Iterator over DHCP options to set every requested default
                                // option for which we have a default
                            }

                            None => {}
                        };

                        Ok(0)
                    }
                    _ => Ok(-1),
                }
            } else {
                return Err(HookError::new("DHCP message received without type"));
            }
        },
    ));
}
