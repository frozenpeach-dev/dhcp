use std::sync::{RwLock, Arc, Mutex};

use fp_core::{hooks::{typemap::TypeMap, hook_registry::HookClosure}, core::{packet::PacketContext, errors::HookError}};

use crate::{packet::dhcp_packet::DhcpV4Packet, allocators::allocator::Allocator, cfg::{main_cfg::DhcpCfg, subnets_cfg::SubnetCfg}};
use crate::allocators::static_alloc::static_allocator::StaticAllocator;

pub fn allocator_hook() {

    let static_alloc_hook = HookClosure(
        Box::new(| services: Arc<Mutex<TypeMap>>, context: &mut PacketContext<DhcpV4Packet, DhcpV4Packet> | {

            let input = context.get_input();

            if let Some(msg_type) = input.options.message_type() {

                match msg_type {

                    1 => {

                        let services = services.lock().unwrap();
                        let _stat_allocator = services.get::<Arc<Mutex<StaticAllocator>>>().unwrap().clone().unwrap();
                        let _subnet_cfg: Arc<RwLock<SubnetCfg>> = services.get().unwrap().clone();
                        drop(services);

                        match _stat_allocator.allocate(input) {

                            Some(draft) => {

                                let output = context.get_mut_output();
                                output.options = *draft.options();
                                output.yiaddr = draft.ip_addr();

                                if output.options.lease_time().is_none() {

                                    output.options
                                        .set_lease_time(
                                            _subnet_cfg.read().unwrap()
                                            .default_options.lease_time()
                                        );

                                }

                                // TODO: Iterator over DHCP options to set every requested default
                                // option for which we have a default

                            }

                        };

                        Ok(0)

                    }


                }


            } else {
                return Err(HookError::new("DHCP message received without type"));
            }



        })
    );

}
