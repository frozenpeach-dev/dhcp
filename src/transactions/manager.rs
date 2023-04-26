use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};
use itertools::Itertools;
use crate::extract;
use fp_core::utils::data::{Storable, RuntimeStorage, DataPool};
use crate::data::data::{Data, LeaseData};
use crate::leases::lease::LeaseV4;
use crate::packet::dhcp_packet::DhcpV4Packet;

use super::transaction::{Transaction, TransactionState};

const ADDRESS : Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
const PENDING_LEASE_POOL_NAME : &str = "PendingLeases";
const LEASE_POOL_NAME : &str = "Leases";
const TRANSACTION_POOL_NAME : &str = "Transactions";


pub struct TransactionManager {
    index : Arc<Mutex<HashMap<u32, u16>>>,
    storage : Arc<Mutex<RuntimeStorage<Data>>>
}

/// [`TransactionManager`] is the service that deals with the fact that lease are not
/// either free or allocated but can also be in an intermediate state "pending".
/// This leads to the fact that packets going through the hooks must be monitored
/// in order to decide in which state a lease is living.
/// 
/// To do so, the [`TransactionManager`] introduces a new struct [`Transaction`]
/// that can live in several states defined by [`TransactionState`]. The interest 
/// lies on the fact that one can bind a [`LeaseV4`] to a [`Transaction`] with the
/// method called [`bind_lease`].
/// 
/// Once the lease bound, the manager will handle every input and output packet to
/// decide if a transaction must be commited or aborted. Binding a lease to a 
/// transaction leads to the fact that both lease and bound transaction will 
/// have the same lifecycle. Thus committing or aborting a transaction will have the
/// same effet on the lease.
/// 
/// To initiate a [`TransactionManager`], you will need a [`RuntimeStorage`] enclosed 
/// in an Arc Mutex.
/// 
/// # Examples
/// ```
/// // Create your manager
/// let my_storage = RuntimeStorage::new();
/// let my_shared_storage = Arc::new(Mutex::new(my_storage));
/// let my_manager = TransactionManager::new(my_shared_storage);
/// 
/// //Init your manager
/// my_manager.init();
/// 
/// //Start the monitoring on your manager
/// let my_manager = Arc::new(Mutex::new(my_manager));
/// let my_monitor = my_manager.clone();
/// 
/// tokio::spawn(async move {
/// loop {
///     tokio::time::sleep(time::Duration::from_secs(1)).await;
///     let mut syncer = transaction_syncer.lock().unwrap();
///     syncer.watchout().unwrap();
/// }
/// });
/// 
/// ```
#[allow(dead_code)]
impl TransactionManager{
    /// Initializes the manager
    pub fn init(&self) {
        let storage = self.storage.clone();
        let storage = storage.lock().unwrap();
        let pending_lease_pool = DataPool::new(PENDING_LEASE_POOL_NAME.to_string(), "(type VARCHAR(255), id BIGINT, name VARCHAR(255), address VARCHAR(255), expiration VARCHAR(255))".to_string());
        let lease_pool = DataPool::new(LEASE_POOL_NAME.to_string(), "(type VARCHAR(255), id BIGINT, name VARCHAR(255), address VARCHAR(255), expiration VARCHAR(255))".to_string());
        let transaction_pool = DataPool::new(TRANSACTION_POOL_NAME.to_string(), "(type VARCHAR(255), id BIGINT, identifier BIGINT, time VARCHAR(255), lease_address BIGINT, state VARCHAR(255))".to_string());
        storage.add_pool(pending_lease_pool);
        storage.add_pool(lease_pool);
        storage.add_pool(transaction_pool);
    }

    ///Initiate a [`Transaction`] with a given identifier (usually the xid of your packet)
    pub fn initiate_transaction(&self, transaction_id : u32) -> Result<(), String>{
        let index = self.index.clone();
        let mut index = index.lock().unwrap();
        match index.get(&transaction_id) {
            Some(_) => Err("Transaction already exists".to_string()),
            None => {
                let mut storage = self.storage.lock().unwrap();
                let mut transaction = Transaction::init_new(transaction_id, chrono::Utc::now());
                transaction.set_state(TransactionState::Pending("PENDING".to_string()));
                let address = storage.store(Data::Transaction(transaction.clone()), TRANSACTION_POOL_NAME.to_string())?;
                index.insert(transaction_id, address);
                return Ok(())
            }
        }
    }

    /// Aborts a [`Transaction`]
    pub fn abort(&mut self, transaction_id : u32) -> Result<u16, String>{
        self.delete_transaction(transaction_id)?;
        Ok(0)
    }
    
    /// Commits a [`Transaction`]
    /// Commiting a [`Transaction`] includes :
    /// - Moving bound [`LeaseV4`] from pending [`DataPool`] to running [`DataPool`]
    /// - Closing [`Transaction`]
    pub fn commit (&mut self, transaction_id : u32) -> Result<(), String> {
        let lease = self.get_transaction_lease(transaction_id)?;
        self.delete_transaction(transaction_id)?;
        let storage = self.storage.clone();
        let mut storage = storage.lock().unwrap();
        match storage.store(Data::Lease(lease), LEASE_POOL_NAME.to_string()) {
            Err(err) => Err(err),
            _ => Ok(())
        }
    }

    /// Given a [`LeaseV4`] and an xid, binds xid's transaction to that lease, so that the state of the lease will be 
    /// autommatically either commited ot aborted depnding on the incoming events.
    pub fn bind_lease(&mut self, xid : u32, lease : LeaseV4) -> Result<u16, String>{
        let t = self.get_transaction(xid)?;
        if t.pending_lease_address == 0 {
            let xid = t.xid;
            self.update_transaction_state(xid, TransactionState::Bound("BOUND".to_string()))?;
            let mut t = self.get_transaction(xid)?;
            let storage = self.storage.clone();
            let mut storage = storage.lock().unwrap();
            let data_lease = Data::Lease(LeaseData::from(lease));
            let lease_address = storage.store(data_lease, PENDING_LEASE_POOL_NAME.to_string())?;
            t.bind(lease_address);
            storage.delete(t.id(), TRANSACTION_POOL_NAME.to_string());
            let address = storage.store(Data::Transaction(t), TRANSACTION_POOL_NAME.to_string())?;
            let index = self.index.clone();
            let mut index = index.lock().unwrap();
            index.remove(&xid);
            index.insert(xid, address);
            Ok(lease_address)
        }else {
            Err("Lease already bound to this transaction".to_string())
        }
        
    }

    /// Deletes a [`Transaction`] from the index and its [`LeaseV4`] from the storage
    fn delete_transaction(&self, transaction_id : u32) -> Result<(), String>{
        let t = self.get_transaction(transaction_id)?;
        let transaction_address = t.uid;
        let lease_address = t.pending_lease_address; //Not yet implemented in fp_core

        let storage = self.storage.clone();
        let mut storage = storage.lock().unwrap();

        let index = self.index.clone();
        let mut index = index.lock().unwrap();
        index.remove(&transaction_id);
        //Drops transaction from storage
        storage.delete(transaction_address, TRANSACTION_POOL_NAME.to_string());
        //Drops lease from storage
        storage.delete(lease_address, PENDING_LEASE_POOL_NAME.to_string()); 
        Ok(())
    }

    /// Gets [`Transaction`] from identifier
    pub fn get_transaction(&self, transaction_id : u32) -> Result<Transaction, String>{
        let transaction_address = self.get_transaction_address(&transaction_id).ok_or_else(||"Error".to_string())?;
        let storage = self.storage.clone();
        let storage = storage.lock().unwrap();
        let data = storage.get(transaction_address)?;
        let transaction = extract!(data, Data::Transaction).ok_or_else(|| "Error".to_string())?;
        Ok(transaction)
    }

    /// Given a [`LeaseV4`] and an xid, changes the lease of the coresponding transaction
    pub fn update_transaction_lease(&mut self, transaction_id : u32, new_lease : LeaseV4) -> Result<(), String>{
        self.delete_transaction(transaction_id)?;
        self.initiate_transaction(transaction_id)?;
        self.bind_lease(transaction_id, new_lease)?;
        Ok(())
    }

    /// Updates the state of a [`Transaction`] given its id
    fn update_transaction_state(&mut self, transaction_id : u32, state : TransactionState) -> Result<(), String>{
        let address = self.get_transaction_address(&transaction_id).ok_or_else(||"No address for given transaction".to_string())?;
        let mut t = self.get_transaction(transaction_id)?;
        t.set_state(state);
        let storage = self.storage.clone();
        let mut storage = storage.lock().unwrap();
        storage.delete(address,TRANSACTION_POOL_NAME.to_string());
        let new_address = storage.store(Data::Transaction(t), TRANSACTION_POOL_NAME.to_string())?;
        let index = self.index.clone();
        let mut index = index.lock().unwrap();
        index.remove(&transaction_id);
        index.insert(transaction_id, new_address);

        Ok(())
    }

    ///Given an xid, returns the storage address of the bound lease
    fn get_transaction_lease_address(&self, transaction_id : u32) -> Result<u16, String> {
        let transaction = self.get_transaction(transaction_id)?;
        Ok(transaction.pending_lease_address)
    }

    /// Gets [`LeaseV4`] bound to the [`Transaction`] identified by the given id
    pub fn get_transaction_lease(&self, transaction_id : u32) -> Result<LeaseData, String>{
        let transaction = self.get_transaction(transaction_id)?;
        let storage = self.storage.clone();
        let storage = storage.lock().unwrap();
        let data = storage.get(transaction.pending_lease_address)?;
        let lease = extract!(data, Data::Lease).ok_or_else(||"No lease".to_string())?;
        Ok(lease)
    }

    /// Returns true if the transaction exists
    pub fn is_in(&self, xid : u32) -> bool {
        let index = self.index.clone();
        let index = index.lock().unwrap();
        index.get(&xid).is_some()
    }

    /// Returns the storage address of a transaction given its id
    pub fn get_transaction_address(&self, xid : &u32) -> Option<u16>{
        let index = self.index.clone();
        let index = index.lock().unwrap();
        index.get(xid).and_then(|t|Some(*t))
    }

    /// Handle an input packet
    pub fn handle_input(&mut self, packet : &DhcpV4Packet) -> Result<(), String> {
        match packet.options.message_type() {
            Some(1) => self.handle_discover(packet),
            Some(3) => self.handle_request(packet),
            _ => Ok(())
        }
    }

    /// Handle an input packet if the packet is a DHCPREQUEST one
    fn handle_discover(&mut self, packet : &DhcpV4Packet) -> Result<(),String>{
        let xid = packet.xid;
        //Abort if DHCP Discover has an xid that is already bound to a transaction
        if self.is_in(xid) {
            return Err("DISCOVER trying to initiate an uncommited transaction, aborting.".to_string())
        }
        //Else initiate new transaction
        self.initiate_transaction(xid)
    }

    /// Handles an input packet if the packet is a DHCPREQUEST one
    fn handle_request(& mut self, packet : &DhcpV4Packet) -> Result<(), String> {
        let xid = packet.xid;
        let t = self.get_transaction(xid)?;
        match packet.options.server_identifier() {
            Some(address) => {
                if !address.is_unspecified() {
                    if address == ADDRESS {
                        match t.state {
                            TransactionState::Waiting(_) => {
                                self.update_transaction_state(xid, TransactionState::Requested("REQUESTED".to_string()))?;
                                return Ok(());
                            }
                            _ => return Err("Trying to request a lease never awaited".to_string())
                        }
                    }
                    // Client chose another server
                    else {
                        self.abort(xid)?;
                        return Ok(())
                    }
                }
            }
            _ => return Err("Unvalid Server Identifier".to_string())
        }
        // If server identifier is unspecified  
        Ok(())
    }

    /// Handles an output [`DhcpV4Packet`]
    pub fn handle_output(&mut self, packet : &DhcpV4Packet) -> Result<(), String>{
        match packet.options.message_type() {
            Some(2) => self.handle_offer(packet),
            Some(5) => self.handle_ack(packet),
            Some(6) => self.handle_nack(packet),
            _ => Ok(())
        }
    }

    /// Handles an output packet if the packet is a DHCPACK one
    fn handle_ack(&mut self, packet : &DhcpV4Packet) -> Result<(), String>{
        let xid = packet.xid;
        let t = self.get_transaction(xid)?;
        match t.state() {
            // If the transaction was requested and ACK is being sent, transaction has to be commited
            TransactionState::Requested(_e) => self.commit(xid),
            _ => Ok(())
        }
    }

    /// Handles an output packet if the packet is a DHCPNACK one
    fn handle_nack(&mut self, _packet : &DhcpV4Packet) -> Result<(), String>{
        Ok(())
    }

    /// Handles an output packet if the packet is a DHCPOFFER one
    fn handle_offer(&mut self, packet :&DhcpV4Packet) -> Result<(), String>{
        let xid = packet.xid;
        let t = self.get_transaction(xid)?;
        match t.state {
            TransactionState::Bound(_e) => self.update_transaction_state(xid, TransactionState::Waiting("WAITING".to_string()))?,
            _ => return Err("Trying to offer but no lease has been bound".to_string())
        };
        
        Ok(())
    }

    /// Creates new [`TransactionManager`] from a shared [`RuntimeStorage`]
    pub fn new(storage : Arc<Mutex<RuntimeStorage<Data>>>) -> Self{
        Self { index: Arc::new(Mutex::new(HashMap::new())), storage}
    }

    /// Drop [`Transaction`] that have timed out
    pub fn watchout(&mut self) -> Result<(), String>{
        let index = self.index.clone();
        let index_list = index.clone();
        let keys : Vec<u32>;
        {   
            let index_list = index_list.lock().unwrap();
            keys = index_list.keys().collect_vec().into_iter().map(|k| *k).collect_vec();
        }
        let outdated_transactions = keys.into_iter().filter(|key|{
            let t = *key;
            match self.get_transaction(t){
                Ok(t) =>{ 
                    t.outdated()
                },
                _ => false
            }
        }).collect_vec();
        
        for transaction_id in outdated_transactions {
            println!("Aborting {}", transaction_id);
            self.abort(transaction_id)?;
        };
        Ok(())
    }

}



#[cfg(test)]
mod test {
    use chrono::Duration;
    use fp_core::core::packet::PacketType;
    use fp_core::utils::data::{DbManager, RuntimeStorage};
    use mysql::params;
    use tokio;
    use std::assert_matches::assert_matches;
    use std::net::Ipv4Addr;
    use std::sync::{Arc, Mutex};
    use std::thread::sleep;
    use std::time;
    use crate::extract;
    use crate::leases::ip_subnet::Ipv4Subnet;
    use crate::leases::lease::LeaseV4;
    use crate::netutils::hw_addr::HardwareAddress;
    use crate::packet::dhcp_packet::DhcpV4Packet;
    use crate::transactions::manager::{TransactionManager, Transaction, TransactionState, ADDRESS};
    use crate::data::data::{Data, LeaseData};
    
    const DHCP_REQUEST : [u8; 300] = [
        0x01, 0x01, 0x06, 0x00, 0xaa, 0xed,
        0x4e, 0xea, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf8, 0x4d, 0x89, 0x82, 0x44, 0x2a, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x63, 0x82, 0x53, 0x63, 0x35, 0x01, 0x03, 0x37, 0x0c, 0x01,
        0x79, 0x03, 0x06, 0x0f, 0x6c, 0x72, 0x77, 0xfc, 0x5f, 0x2c, 0x2e, 0x39, 0x02, 0x05, 0xdc, 0x3d,
        0x07, 0x01, 0xf8, 0x4d, 0x89, 0x82, 0x44, 0x2a, 0x33, 0x04, 0x00, 0x76, 0xa7, 0x00, 0x0c, 0x0c,
        0x4d, 0x42, 0x50, 0x2d, 0x64, 0x65, 0x2d, 0x53, 0x61, 0x63, 0x68, 0x61, 0xff, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00
    ];

    const DHCP_OFFER : [u8; 300] = [
        0x01, 0x01, 0x06, 0x00, 0xaa, 0xed,
        0x4e, 0xea, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf8, 0x4d, 0x89, 0x82, 0x44, 0x2a, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x63, 0x82, 0x53, 0x63, 0x35, 0x01, 0x02, 0x37, 0x0c, 0x01,
        0x79, 0x03, 0x06, 0x0f, 0x6c, 0x72, 0x77, 0xfc, 0x5f, 0x2c, 0x2e, 0x39, 0x02, 0x05, 0xdc, 0x3d,
        0x07, 0x01, 0xf8, 0x4d, 0x89, 0x82, 0x44, 0x2a, 0x33, 0x04, 0x00, 0x76, 0xa7, 0x00, 0x0c, 0x0c,
        0x4d, 0x42, 0x50, 0x2d, 0x64, 0x65, 0x2d, 0x53, 0x61, 0x63, 0x68, 0x61, 0xff, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00
    ];

    const DHCP_DISCOVER : [u8; 300] = [
        0x01, 0x01, 0x06, 0x00, 0xaa, 0xed,
        0x4e, 0xea, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf8, 0x4d, 0x89, 0x82, 0x44, 0x2a, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x63, 0x82, 0x53, 0x63, 0x35, 0x01, 0x01, 0x37, 0x0c, 0x01,
        0x79, 0x03, 0x06, 0x0f, 0x6c, 0x72, 0x77, 0xfc, 0x5f, 0x2c, 0x2e, 0x39, 0x02, 0x05, 0xdc, 0x3d,
        0x07, 0x01, 0xf8, 0x4d, 0x89, 0x82, 0x44, 0x2a, 0x33, 0x04, 0x00, 0x76, 0xa7, 0x00, 0x0c, 0x0c,
        0x4d, 0x42, 0x50, 0x2d, 0x64, 0x65, 0x2d, 0x53, 0x61, 0x63, 0x68, 0x61, 0xff, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00
    ];

    const DHCP_ACK : [u8; 300] = [
        0x01, 0x01, 0x06, 0x00, 0xaa, 0xed,
        0x4e, 0xea, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf8, 0x4d, 0x89, 0x82, 0x44, 0x2a, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x63, 0x82, 0x53, 0x63, 0x35, 0x01, 0x05, 0x37, 0x0c, 0x01,
        0x79, 0x03, 0x06, 0x0f, 0x6c, 0x72, 0x77, 0xfc, 0x5f, 0x2c, 0x2e, 0x39, 0x02, 0x05, 0xdc, 0x3d,
        0x07, 0x01, 0xf8, 0x4d, 0x89, 0x82, 0x44, 0x2a, 0x33, 0x04, 0x00, 0x76, 0xa7, 0x00, 0x0c, 0x0c,
        0x4d, 0x42, 0x50, 0x2d, 0x64, 0x65, 0x2d, 0x53, 0x61, 0x63, 0x68, 0x61, 0xff, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00
    ];
    
    #[tokio::test(flavor = "multi_thread")]
    async fn test_init(){
        let db = DbManager::new(String::from("dhcp"), String::from("frozenpeach"), String::from("poney"), String::from("127.0.0.1:3333"));
        let sto_db = Arc::new(Mutex::new(db));
        let test_db = sto_db.clone();
        let storage: RuntimeStorage<Data> = RuntimeStorage::new(sto_db);
        let storage = Arc::new(Mutex::new(storage));
        let sync = storage.clone();
        let transaction_manager = TransactionManager::new(storage);
        let manager = Arc::new(Mutex::new(transaction_manager));
        let transaction_manager = manager.clone();
        { 
            let transaction_manager = transaction_manager.lock().unwrap();
            transaction_manager.init();
        }
        let manager_handler = manager.clone();
        let transaction_syncer = manager.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(time::Duration::from_millis(100)).await;
                let mut syncer = transaction_syncer.lock().unwrap();
                syncer.watchout().unwrap();
            }
        });
        
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(time::Duration::from_millis(100)).await;
                sync.lock().unwrap().sync();
            }
        });
        discover_process(manager_handler, test_db, true);
        
        tokio::time::sleep(time::Duration::from_secs(7)).await;
    }

    fn discover_process(manager : Arc<Mutex<TransactionManager>>, test_db : Arc<Mutex<DbManager>>, success : bool){
        let subnet = Ipv4Subnet::new(Ipv4Addr::new(192, 168, 0, 0), 24);
        let lease = LeaseV4::new(
            Ipv4Addr::new(192, 168, 0, 3),
            &subnet,
            Duration::hours(8),
            HardwareAddress::broadcast(),
            HardwareAddress::broadcast(),
            String::from("test_lease"),
        ).unwrap();

        let packet_offer = DhcpV4Packet::from_raw_bytes(DHCP_OFFER.as_slice());
        let mut packet_request = DhcpV4Packet::from_raw_bytes(DHCP_REQUEST.as_slice());
        let mut packet_ack = DhcpV4Packet::from_raw_bytes(DHCP_ACK.as_slice());
        let expected_lease_address : u16;
        
        //Test discover handling
        println!("Testing discover handling");
        let packet_discover = DhcpV4Packet::from_raw_bytes(DHCP_DISCOVER.as_slice());
        assert_eq!(packet_discover.options.message_type(), Some(1));
        {
            {
            let mut manager = manager.lock().unwrap();
            manager.handle_input(&packet_discover).unwrap();
            }
            let xid = packet_discover.xid;
            sleep(time::Duration::from_secs(1));
            let test_db = test_db.lock().unwrap();
            let t : Vec<Transaction> = test_db.exec_and_return("SELECT * FROM Transactions where identifier = :identifier".to_string(), params! {"identifier" => xid}).unwrap();
            let transaction = t.get(0).unwrap();
            assert_eq!(t.len(), 1);
            assert_eq!(transaction.xid, xid);
            assert_matches!(transaction.state, TransactionState::Pending(_));
        }
    
        //Test lease binding
        println!("Testing lease binding");
        {
            {
            let mut manager = manager.lock().unwrap();
            manager.bind_lease(packet_discover.xid, lease.clone()).unwrap();
            }
            let xid = packet_discover.xid;
            sleep(time::Duration::from_secs(1));
            let test_db = test_db.lock().unwrap();
            let t : Vec<Transaction> = test_db.exec_and_return("SELECT * FROM Transactions where identifier = :identifier".to_string(), params! {"identifier" => xid}).unwrap();
            let transaction = t.get(0).unwrap();
            let state = transaction.state.clone();
            assert_matches!(state, TransactionState::Bound(_));
            let state = extract!(state, TransactionState::Bound).unwrap();
            assert_eq!(state, "BOUND".to_string());

            let l : Vec<LeaseData> = test_db.exec_and_return("SELECT * FROM PendingLeases where id = :id".to_string(), params! {"id" => transaction.pending_lease_address}).unwrap();
            let l = l.get(0).unwrap();
            let lease = LeaseData::from(lease.clone());
            assert_eq!(lease.address(), l.address());

            expected_lease_address = transaction.pending_lease_address;
            //Note that expiration differ. To be fixed.
        }

        //Test offer handling
        println!("Testing offer handling");
        {
            {
            let mut manager = manager.lock().unwrap();
            manager.handle_output(&packet_offer).unwrap();
            }
            let xid = packet_discover.xid;
            sleep(time::Duration::from_secs(1));
            let test_db = test_db.lock().unwrap();
            let t : Vec<Transaction> = test_db.exec_and_return("SELECT * FROM Transactions where identifier = :identifier".to_string(), params! {"identifier" => xid}).unwrap();
            let transaction = t.get(0).unwrap();
            let state = transaction.state.clone();
            assert_matches!(state, TransactionState::Waiting(_));

            let l : Vec<LeaseData> = test_db.exec_and_return("SELECT * FROM PendingLeases where id = :id".to_string(), params! {"id" => transaction.pending_lease_address}).unwrap();
            let l = l.get(0).unwrap();
            let lease = LeaseData::from(lease.clone());
            assert_eq!(lease.address(), l.address());
            //Note that expiration differ. To be fixed.
        }
        
        if success {
            println!();
            println!("--- Switching to lease request ---");
            println!();
            //Test valid request handling
            println!("Testing request handling");
            {
                {
                    let mut manager = manager.lock().unwrap();
                    packet_request.options.set_server_identifier(Some(ADDRESS));
                    manager.handle_input(&packet_request).unwrap();
                }
                let xid = packet_discover.xid;
                sleep(time::Duration::from_secs(1));
                let test_db = test_db.lock().unwrap();
                let t : Vec<Transaction> = test_db.exec_and_return("SELECT * FROM Transactions where identifier = :identifier".to_string(), params! {"identifier" => xid}).unwrap();
                let transaction = t.get(0).unwrap();
                let state = transaction.state.clone();
                assert_matches!(state, TransactionState::Requested(_));

                let l : Vec<LeaseData> = test_db.exec_and_return("SELECT * FROM PendingLeases where id = :id".to_string(), params! {"id" => transaction.pending_lease_address}).unwrap();
                let l = l.get(0).unwrap();
                let lease = LeaseData::from(lease.clone());
                assert_eq!(lease.address(), l.address());
                //Note that expiration differ. To be fixed.
            }

            //Test ack handling
            println!("Testing ack handling");
            {
                {
                    let mut manager = manager.lock().unwrap();
                    packet_ack.options.set_server_identifier(Some(ADDRESS));
                    manager.handle_output(&packet_ack).unwrap();
                }
                let xid = packet_discover.xid;
                sleep(time::Duration::from_secs(1));
                let test_db = test_db.lock().unwrap();
                let t : Vec<Transaction> = test_db.exec_and_return("SELECT * FROM Transactions where identifier = :identifier".to_string(), params! {"identifier" => xid}).unwrap();
                assert_eq!(t.len(), 0);

                let l : Vec<LeaseData> = test_db.exec_and_return("SELECT * FROM PendingLeases where id = :id".to_string(), params! {"id" => expected_lease_address}).unwrap();
                assert_eq!(l.len(), 0);

                let l : Vec<LeaseData> = test_db.exec_and_return("SELECT * FROM Leases where address = :address".to_string(), params! {"address" => lease.addr().to_string()}).unwrap();
                let l = l.get(0).unwrap();
                let lease = LeaseData::from(lease);
                assert_eq!(lease.address(), l.address());
            }
            println!();
            println!("--- Reinitializing ---");
            println!();
            discover_process(manager, test_db, false);

        } else {
            println!();
            println!("--- Switching to lease disengagement ---");
            println!();
            //Test unvalid request handling (ie other server)
            println!("Testing unvalid request handling");
            {
                {
                    let xid = packet_discover.xid;
                    let mut manager = manager.lock().unwrap();
                    manager.update_transaction_state(xid, TransactionState::Waiting("WAITING".to_string())).unwrap();
                    packet_request.options.set_server_identifier(Some(Ipv4Addr::BROADCAST));
                    manager.handle_input(&packet_request).unwrap();
                }
                let xid = packet_discover.xid;
                sleep(time::Duration::from_secs(1));
                let test_db = test_db.lock().unwrap();
                let t : Vec<Transaction> = test_db.exec_and_return("SELECT * FROM Transactions where identifier = :identifier".to_string(), params! {"identifier" => xid}).unwrap();
                assert_eq!(t.len(), 0);

                let l : Vec<LeaseData> = test_db.exec_and_return("SELECT * FROM PendingLeases where id = :id".to_string(), params! {"id" => expected_lease_address}).unwrap();
                assert_eq!(l.len(), 0);
                //Note that expiration differ. To be fixed.
            }

            println!("Discovery process successfull")
        }
    }
}

