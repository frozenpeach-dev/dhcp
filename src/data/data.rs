use fp_core::utils::data::Storable;
use derive_data::Storable;
use crate::{transactions::dhcp_transactions::{Transaction}, leases::lease::LeaseV4};
use mysql::{self, prelude::FromRow, params};

#[derive(Clone)]
pub enum Data{
    Transaction(Transaction),
    Lease(LeaseData)
}

#[derive(Clone)]
pub struct LeaseData {

}

impl LeaseData {
    pub fn from(lease : LeaseV4) -> LeaseData{
        todo!()
    }
}


impl Storable for Data{
    fn id(&self) -> u16 {
        todo!()
    }
    fn insert_statement(&self, place : String) -> String {
        todo!()
    }
    fn set_uid(&mut self, uid : u16) {
        todo!()
    }
    fn value(&self) -> params::Params {
        todo!()
    }
}

impl FromRow for Data{
    fn from_row(row: mysql::Row) -> Self
        where
            Self: Sized, {
        todo!()
    }

    fn from_row_opt(row: mysql::Row) -> Result<Self, mysql::FromRowError>
        where
            Self: Sized {
        todo!()
    }
}

impl<'a> Storable for LeaseV4<'a> {
    fn id(&self) -> u16 {
        todo!()
    }
    fn insert_statement(&self, place : String) -> String {
        todo!()
    }
    fn set_uid(&mut self, uid : u16) {
        todo!()
    }
    fn value(&self) -> params::Params {
        todo!()
    }
}