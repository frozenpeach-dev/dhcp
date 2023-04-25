use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use chrono::{DateTime, Utc};
use crate::extract;
use fp_core::utils::data::{Storable, RuntimeStorage, DataPool};
use crate::data::data::{Data, LeaseData};
use crate::leases::lease::LeaseV4;
use crate::packet::dhcp_packet::DhcpV4Packet;
use tokio;
use mysql::params;

const ADDRESS : Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
const PENDING_LEASE_POOL_NAME : &str = "PendingLeases";
const LEASE_POOL_NAME : &str = "Leases";
const TRANSACTION_POOL_NAME : &str = "Transactions";

#[derive(Clone)]
enum TransactionState {
    Pending(String),
    Waiting(String),
    Requested(String),
}

pub struct TransactionManager {
    index : Arc<Mutex<HashMap<u32, u16>>>,
    storage : Arc<Mutex<RuntimeStorage<Data>>>
}

#[allow(dead_code)]
impl TransactionManager{
    /// Initializes the manager
    pub fn init(&self) {
        let storage = self.storage.clone();
        let storage = storage.lock().unwrap();
        let pending_lease_pool = DataPool::new(PENDING_LEASE_POOL_NAME.to_string(), "".to_string());
        let lease_pool = DataPool::new(LEASE_POOL_NAME.to_string(), "".to_string());
        let transaction_pool = DataPool::new(TRANSACTION_POOL_NAME.to_string(), "".to_string());
        storage.add_pool(pending_lease_pool);
        storage.add_pool(lease_pool);
        storage.add_pool(transaction_pool);
    }

    ///Initiate a transaction
    pub fn initiate_transaction(&self, transaction_id : u32) -> Result<(), String>{
        let index = self.index.clone();
        let mut index = index.lock().unwrap();
        match index.get(&transaction_id) {
            Some(_) => Err("Transaction already exists".to_string()),
            None => {
                let mut storage = self.storage.lock().unwrap();
                let mut transaction = Transaction::new(transaction_id, chrono::Utc::now());
                let address = storage.store(Data::Transaction(transaction.clone()), "transactions".to_string()).unwrap();
                transaction.set_lease_address(address);
                transaction.set_state(TransactionState::Pending("PENDING".to_string()));
                index.insert(transaction_id, transaction.address);
                return Ok(())
            }
        }
    }

    /// Aborts a [`Transaction`]
    pub fn abort(&mut self, transaction_id : u32) -> Result<(), String>{
        let index = self.index.clone();
        let mut index = index.lock().unwrap();
        if self.is_in(transaction_id) {
            self.delete_transaction(transaction_id)?;
            index.remove(&transaction_id);
            return Ok(())
        }else {
            return Err("Unexisting transaction".to_string());
        }  
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
    pub fn bind_lease(&self, xid : u32, lease : LeaseV4) -> Result<u16, String>{
        let storage = self.storage.clone();
        let mut storage = storage.lock().unwrap();
        let mut t = self.get_transaction(xid)?;
        let data_lease = Data::Lease(LeaseData::from(lease));
        let lease_address = storage.store(data_lease, PENDING_LEASE_POOL_NAME.to_string())?;
        t.bind(lease_address);
        Ok(lease_address)
    }

    /// Deletes a [`Transaction`] from the index and its [`LeaseV4`] from the storage
    fn delete_transaction(&self, transaction_id : u32) -> Result<(), String>{
        let t = self.get_transaction(transaction_id)?;
        //Lock storage and index
        let storage = self.storage.clone();
        let mut storage = storage.lock().unwrap();
        //Drop transaction from storage
        let transaction_address = t.address;
        storage.delete(transaction_address, TRANSACTION_POOL_NAME.to_string());
        //Drop correspoding lease from storage
        let lease_address = t.pending_lease_address; 
        storage.delete(lease_address, PENDING_LEASE_POOL_NAME.to_string()); //Not yet implemented in fp_core
        Ok(())
    }

    /// Gets [`Transaction`] from id
    pub fn get_transaction(&self, transaction_id : u32) -> Result<Transaction, String>{
        let storage = self.storage.clone();
        let storage = storage.lock().unwrap();
        let transaction_address = self.get_transaction_address(&transaction_id).ok_or_else(||"Error".to_string())?;
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

    fn update_transaction_state(&mut self, transaction_id : u32, state : TransactionState) -> Result<(), String>{
        let address = self.get_transaction_address(&transaction_id).ok_or_else(||"No address for given transaction".to_string())?;
        let storage = self.storage.clone();
        let mut storage = storage.lock().unwrap();
        storage.delete(address,TRANSACTION_POOL_NAME.to_string());
        let mut t = self.get_transaction(transaction_id)?;
        t.set_state(state);
        storage.store(Data::Transaction(t), PENDING_LEASE_POOL_NAME.to_string())?;
        Ok(())
    }

    ///Given an xid, returns the storage address of the bound lease
    fn get_transaction_lease_address(&self, transaction_id : u32) -> Result<u16, String> {
        let transaction = self.get_transaction(transaction_id)?;
        Ok(transaction.pending_lease_address)
    }

    /// Gets [`LeaseV4`] bound to the [`Transaction`] identified by the given id
    pub fn get_transaction_lease(&self, transaction_id : u32) -> Result<LeaseData, String>{
        let storage = self.storage.clone();
        let storage = storage.lock().unwrap();
        let transaction = self.get_transaction(transaction_id)?;
        let data = storage.get(transaction.pending_lease_address)?;
        let lease = extract!(data, Data::Lease).ok_or_else(||"No lease".to_string())?;
        Ok(lease)
    }

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
    pub fn handle_discover (&mut self, packet : &DhcpV4Packet) -> Result<(),String>{
        let xid = packet.xid;
        //Abort if DHCP Discover has an xid that is already bound to a transaction
        if self.is_in(xid) {
            return Err("DISCOVER trying to initiate an uncommited transaction, aborting.".to_string())
        }
        //Else initiate new transaction
        self.initiate_transaction(xid)
    }

    /// Handles an input packet if the packet is a DHCPREQUEST one
    pub fn handle_request (& mut self, packet : &DhcpV4Packet) -> Result<(), String> {
        let xid = packet.xid;
        match packet.options.server_identifier() {
            Some(address) => {
                if !address.is_unspecified() {
                    if address == ADDRESS {
                        self.update_transaction_state(xid, TransactionState::Requested("REQUESTED".to_string()))?;
                        return Ok(());
                    }
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
    pub fn handle_output(&mut self, packet : DhcpV4Packet) -> Result<(), String>{
        match packet.options.message_type() {
            Some(2) => self.handle_offer(packet),
            Some(5) => self.handle_ack(packet),
            Some(6) => self.handle_nack(packet),
            _ => Ok(())
        }
    }

    /// Handles an input packet if the packet is a DHCPACK one
    pub fn handle_ack(&mut self, packet : DhcpV4Packet) -> Result<(), String>{
        let xid = packet.xid;
        let t = self.get_transaction(xid)?;
        match t.state() {
            // If the transaction was requested and ACK is being sent, transaction has to be commited
            TransactionState::Requested(_e) => self.commit(xid),
            _ => Ok(())
        }
    }

    /// Handles an input packet if the packet is a DHCPNACK one
    pub fn handle_nack(&mut self, _packet : DhcpV4Packet) -> Result<(), String>{
        Ok(())
    }

    /// Handles an input packet if the packet is a DHCPOFFER one
    pub fn handle_offer(&mut self, packet : DhcpV4Packet) -> Result<(), String>{
        let xid = packet.xid;
        let mut t = self.get_transaction(xid)?;
        t.set_state(TransactionState::Waiting("WAITING".to_string()));
        Ok(())
    }

    pub fn new(storage : Arc<Mutex<RuntimeStorage<Data>>>) -> Self{
        Self { index: Arc::new(Mutex::new(HashMap::new())), storage: storage}
    }

    /// Given an Arc Mutex of a manager, starts polling with a given timeout
    pub fn watch(manager : Arc<Mutex<TransactionManager>>, timeout : Duration) {
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(timeout).await;
                let mut manager = manager.lock().unwrap();
                let index = manager.index.clone();
                let mut index = index.lock().unwrap();
                index.retain(|&k, &mut _address| {
                    let t = manager.get_transaction(k).unwrap();
                    if t.outdated() {
                        manager.abort(k);
                        return true
                    }else {
                        return false
                    }
                })
            }
        });
        
    }

}



impl Storable for Transaction{
    fn id(&self) -> u16 {
        self.xid as u16
    }

    fn insert_statement(&self, place : String) -> String {
        "INSERT INTO {} VALUE (:xid, :state)".to_string()
    }

    fn value(&self) -> mysql::params::Params {
        match &self.state {
            TransactionState::Pending(e)|TransactionState::Requested(e)|TransactionState::Waiting(e) => params! {"xid" => self.xid, "state" => e}
        }
    }

    fn set_uid(&mut self, uid : u16) {
        self.uid = uid;
    }
}

#[derive(Clone)]
pub struct Transaction {
    state : TransactionState,
    start : DateTime<Utc>,
    pending_lease_address : u16,
    address : u16,
    xid : u32,
    uid : u16
}

impl Transaction{
    pub fn new(transaction_id : u32, time : DateTime<Utc>) -> Self {
        todo!()
    }

    fn set_state(&mut self, state : TransactionState) {
        self.state = state;
         
    }

    pub fn abort(self) {
        drop(self);
    }

    pub fn set_lease_address(&mut self, address : u16) {
        self.pending_lease_address = address;
    }

    fn bind(&mut self, lease_address : u16) {
        self.pending_lease_address = lease_address;
    }

    pub fn outdated(&self) -> bool{
        (Utc::now() - self.start).is_zero()
    }

    fn state(&self) -> TransactionState{
        self.state.clone()
    }
}

#[macro_export]
macro_rules! extract {
    ($e:expr, $i:path) => {
        match $e {
            $i(value) => Some(value),
            _ => None,
        }
    };
}


#[cfg(test)]
mod test  {
    use std::sync::{Arc, Mutex};

    use fp_core::utils::data::{RuntimeStorage, DbManager};

    use super::TransactionManager;

    fn test() {
        let a = String::from("test");
        let db = DbManager::new(a.clone(), a.clone(), a.clone(), a.clone());
        let db = Arc::new(Mutex::new(db));
        let storage = RuntimeStorage::new(db);
        let storage = Arc::new(Mutex::new(storage));
        let manager = TransactionManager::new(storage);
        manager.initiate_transaction(65).unwrap();
    }
}