// use std::sync::{Arc, Mutex};

// use fp_core::{hooks::{self, hook_registry::{Hook, HookRegistry}, typemap::TypeMap, flags::HookFlag}, core::{packet::PacketContext, errors::HookError}};

// use crate::{packet::dhcp_packet::DhcpV4Packet, transactions::manager::TransactionManager};

// fn init_transaction_manager_hook(mut registry : HookRegistry<DhcpV4Packet, DhcpV4Packet>) -> HookRegistry<DhcpV4Packet, DhcpV4Packet>{
//     let input_handler = Box::new(|type_map : Arc<Mutex<TypeMap>>, context : &mut PacketContext<DhcpV4Packet, DhcpV4Packet>|{
//         let mut input_manager = type_map.lock().unwrap();
//         let transaction_manager = input_manager.get_mut::<Arc<Mutex<TransactionManager>>>().unwrap();
//         let input = context.get_input();
//         let transaction_manager = transaction_manager.clone();
//         let mut transaction_handler = transaction_manager.lock().unwrap();
//         let result = transaction_handler.handle_input(input);
//         match result {
//             Err(e) => Ok(-1),
//             _ => Ok(0)
//         }
//     });

//     let output_handler = Box::new(|type_map : Arc<Mutex<TypeMap>>, context : &mut PacketContext<DhcpV4Packet, DhcpV4Packet>|{
//         let mut input_manager = type_map.lock().unwrap();
//         let transaction_manager = input_manager.get_mut::<Arc<Mutex<TransactionManager>>>().unwrap();
//         let output = context.get_output();
//         let transaction_manager = transaction_manager.clone();
//         let mut transaction_handler = transaction_manager.lock().unwrap();
//         let result = transaction_handler.handle_output(output);
//         match result {
//             Err(e) => Ok(-1),
//             _ => Ok(0)
//         }
//     });

//     let flags = vec![];
//     let input_hook = Hook::new("TransactionInput".to_string(),input_handler,flags);
//     let flags = vec![];
//     let mut output_hook = Hook::new("TransactionOutput".to_string(), output_handler, flags);
//     output_hook.must(input_hook.id());
//     registry.register_hook(fp_core::core::state::PacketState::Received, output_hook);
//     registry
    
// }