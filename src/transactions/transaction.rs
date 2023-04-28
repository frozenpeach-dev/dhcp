use chrono::{DateTime, Utc};
use fp_core::storage::data::Storable;
use mysql::{params, prelude::FromRow};

#[derive(Clone, Debug)]
pub enum TransactionState {
    Pending(String),
    Waiting(String),
    Requested(String),
    Undefined(String),
    Bound(String)
}

//Make Transactions storable
impl Storable for Transaction{
    fn id(&self) -> u16 {
        self.uid
    }

    fn insert_statement(&self, place : String) -> String {
        format!("INSERT INTO {} VALUE (:type, :id, :identifier, :time, :lease_address, :state)", place)
    }

    fn value(&self) -> mysql::params::Params {
        let state : String;
        match &self.state {
            TransactionState::Pending(e)|TransactionState::Requested(e)|TransactionState::Waiting(e)|TransactionState::Undefined(e)|TransactionState::Bound(e) => state = e.to_string()
        }
        params! {"type" => "transaction", "id" => self.uid, "identifier" => self.xid, "time" => self.start.to_rfc2822(), "lease_address" => self.pending_lease_address, "state" => state}
    }

    fn set_uid(&mut self, uid : u16) {
        self.uid = uid;
    }
}

#[derive(Clone, Debug)]
pub struct Transaction {
    pub state : TransactionState,
    pub start : DateTime<Utc>,
    pub pending_lease_address : u16,
    pub uid : u16,
    pub xid : u32,
}

#[allow(dead_code)]
impl Transaction{
    pub fn init_new(transaction_id : u32, time : DateTime<Utc>) -> Self {
        Self { state: TransactionState::Undefined("UNDEFINED".to_string()), start: time, pending_lease_address: 0, uid: 0, xid: transaction_id }
    }

    pub fn new(state : TransactionState, start : DateTime<Utc>, pending_lease_address : u16, uid : u16, xid : u32) -> Self{
        Self {
            state,
            start,
            pending_lease_address,
            uid,
            xid
        }
    }

    pub fn set_state(&mut self, state : TransactionState) {
        self.state = state;
    }

    pub fn abort(self) {
        drop(self);
    }

    pub fn set_lease_address(&mut self, address : u16) {
        self.pending_lease_address = address;
    }

    pub fn bind(&mut self, lease_address : u16) {
        self.pending_lease_address = lease_address;
    }

    pub fn outdated(&self) -> bool{
        (Utc::now() - self.start) > chrono::Duration::seconds(30)
    }

    pub fn state(&self) -> TransactionState{
        self.state.clone()
    }
}

impl FromRow for Transaction {
    fn from_row(row: mysql::Row) -> Self
        where
            Self: Sized, {

        let uid : u16 = row.get(1).unwrap();
        let identifier : u32 = row.get(2).unwrap();
        let time : String = row.get(3).unwrap();
        let time: DateTime<Utc> = DateTime::parse_from_rfc2822(&time).unwrap().into();
        let lease_address: u16 = row.get(4).unwrap();
        let state : String = row.get(5).unwrap();
        let a : TransactionState;
        match state.as_str() {
            "UNDEFINED" => a = TransactionState::Undefined("UNDEFINED".to_string()),
            "PENDING" => a = TransactionState::Pending("PENDING".to_string()),
            "REQUESTED" => a = TransactionState::Requested("REQUESTED".to_string()),
            "WAITING" => a = TransactionState::Waiting("WAITING".to_string()),
            "BOUND" => a = TransactionState::Bound("BOUND".to_string()),
            _ => a = TransactionState::Undefined("UNDEFINED".to_string())
        }
        Self::new(a, time, lease_address, uid, identifier)
    }

    fn from_row_opt(row: mysql::Row) -> Result<Self, mysql::FromRowError>
        where
            Self: Sized {
        Ok(Transaction::from_row(row))
    }
}
