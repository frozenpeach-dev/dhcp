use std::sync::atomic::AtomicBool;

use fp_core::{netio::{udp_input::UdpInput, udp_output::UdpOutput}, core::state_switcher::StateSwitcher, hooks::hook_registry::{self, HookRegistry}};
use packet::dhcp_packet::DhcpV4Packet;

mod packet;
mod leases;
mod netutils;
mod allocators;

fn main() {

    //let (switch, server) = serve();
    
    //server.start();

}

// fn serve() -> (StateSwitcher<DhcpV4Packet, DhcpV4Packet>, Arc<AtomicBool>){
// 
//     // probably not going to work, should rather share a common socket wrapped inside of an arc
//     let input = Box::new(UdpInput::start("0.0.0.0:68"));
//     let output = UdpOutput::start("0.0.0.0:68");
// 
//     let switch = Arc::new(AtomicBool::new(true));
//         
//     let hook_registry: HookRegistry<DhcpV4Packet, DhcpV4Packet> = HookRegistry::new();
// 
//     let state_switcher = StateSwitcher::new(input, output, hook_registry, switch.clone()); 
// 
//     state_switcher
// 
// }
