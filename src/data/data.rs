use std::net::Ipv4Addr;
use chrono::{DateTime, Utc};
use fp_core::storage::data::Storable;
use derive_data::Storable;
use crate::{transactions::transaction::Transaction, leases::{lease::LeaseV4, ip_subnet::Ipv4Subnet}};
use mysql::{self, prelude::FromRow, params};
use std::str::FromStr;

#[derive(Clone, Storable)]
/// `Data` is an enum used by [`RuntimeStorage`] to store several types of data.
/// `Data` must derive [`Storable`] and implment [`FromRow`] and [`Clone`] 
/// in order to be stored in a [`RuntimeStorage`]
/// 
/// To store a special type of data, you will need to implement 
/// [`Storable`] and [`FromRow`] for this type as well.
/// 
/// # Examples
/// ```
/// pub struct MySpecialData {
///     
/// }
/// 
/// impl FromRow for MyData {
///     //Impl here
/// }
/// 
/// impl Storable for MyData {
///     //Impl here
/// }
/// 
/// #[derive(Storable, Clone)]
/// pub enum Data {
///     MySpecialData(MySpecialData)
/// }
/// let my_data = MySpecialData::new();
/// let data_to_store = Data::MySpecialData(my_data);
/// runtimestorage.store(data_to_store);
///```
pub enum Data{
    Transaction(Transaction),
    Lease(LeaseData),
    Ipv4Subnet(Ipv4Subnet),
    Null()
}

/// [`LeaseData`] is a struct created to make [`LeaseV4`] storable by 
/// storing only necessary fields of a lease.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LeaseData {
    expiration_time : DateTime<Utc>,
    address : Ipv4Addr,
    uid : u16,
    hostname : String
}

#[allow(dead_code)]
impl LeaseData {
    pub fn from(lease : LeaseV4) -> LeaseData{
        let expiration_time = lease.end();
        let address = lease.addr();
        LeaseData { expiration_time, address , uid : 0, hostname : "unknown".to_string()}
    }

    pub fn address(&self) -> Ipv4Addr {
        self.address
    }
}

impl FromRow for Data{
    fn from_row(row: mysql::Row) -> Self
        where
            Self: Sized, {
        let data_type : String = row.get(0).unwrap();
        match data_type.as_str() {
            "lease" => Data::Lease(LeaseData::from_row(row)),
            "transaction" => Data::Transaction(Transaction::from_row(row)),
            "subnet" => Data::Ipv4Subnet(Ipv4Subnet::from_row(row)),
            _ => Data::Null()
        }
    }

    fn from_row_opt(row: mysql::Row) -> Result<Self, mysql::FromRowError>
        where
            Self: Sized {
        Ok(Data::from_row(row))
    }
}

impl Storable for LeaseData {
    fn id(&self) -> u16 {
        self.uid
    }

    fn insert_statement(&self, place : String) -> String {
        format!("INSERT INTO {} VALUE (:type, :id, :name, :address, :expiration)", place)
    }

    fn set_uid(&mut self, uid : u16) {
        self.uid = uid
    }

    fn value(&self) -> params::Params {
        params! {"type" => "lease", "id" => self.uid,"name" => self.hostname.to_string(), "address" => self.address.to_string(), "expiration" => self.expiration_time.to_rfc2822()}
    }
}

impl FromRow for LeaseData {
    fn from_row(row: mysql::Row) -> Self
        where
            Self: Sized, {
        let uid : u16 = row.get(1).unwrap();
        let name : String = row.get(2).unwrap();
        let address : String = row.get(3).unwrap();
        let address = Ipv4Addr::from_str(&address).unwrap();
        let expiration : String = row.get(4).unwrap();
        let expiration: DateTime<Utc> = DateTime::parse_from_rfc2822(&expiration).unwrap().into();
        Self { expiration_time: expiration, address: address, uid: uid, hostname : name }

    }

    fn from_row_opt(row: mysql::Row) -> Result<Self, mysql::FromRowError>
        where
            Self: Sized {
        Ok(LeaseData::from_row(row))
        
    }
}

/// Extracts object from Data
#[macro_export]
macro_rules! extract {
($e:expr, $i:path) => {
    match $e {
        $i(value) => Some(value),
        _ => None,
    }
};
}