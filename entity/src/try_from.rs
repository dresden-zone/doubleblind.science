use hickory_proto::rr::{rdata, Name, RData};

use crate::error::EntityError;
use crate::{record_a, record_aaaa, record_cname, record_mx, record_ns, record_txt};

fn parse_name(provided: &str, origin: &Name) -> Result<Name, EntityError> {
    let name = match provided.strip_suffix('@') {
        None => Name::from_ascii(provided)?,
        Some(stripped) => {
            if stripped.is_empty() {
                origin.clone()
            } else {
                Name::from_ascii(stripped)?.append_domain(origin)?
            }
        }
    };

    Ok(name)
}

pub trait IntoRecord {
    fn into_record(self, origin: &Name) -> Result<RData, EntityError>;
}

impl IntoRecord for record_a::Model {
    fn into_record(self, _origin: &Name) -> Result<RData, EntityError> {
        Ok(RData::A(rdata::A(self.address.parse()?)))
    }
}

impl IntoRecord for record_aaaa::Model {
    fn into_record(self, _origin: &Name) -> Result<RData, EntityError> {
        Ok(RData::AAAA(rdata::AAAA(self.address.parse()?)))
    }
}

impl IntoRecord for record_cname::Model {
    fn into_record(self, origin: &Name) -> Result<RData, EntityError> {
        Ok(RData::CNAME(rdata::CNAME(parse_name(
            &self.target,
            origin,
        )?)))
    }
}

impl IntoRecord for record_mx::Model {
    fn into_record(self, origin: &Name) -> Result<RData, EntityError> {
        Ok(RData::MX(rdata::MX::new(
            self.preference as u16,
            parse_name(&self.exchange, origin)?,
        )))
    }
}

impl IntoRecord for record_ns::Model {
    fn into_record(self, origin: &Name) -> Result<RData, EntityError> {
        Ok(RData::NS(rdata::NS(parse_name(&self.target, origin)?)))
    }
}

impl IntoRecord for record_txt::Model {
    fn into_record(self, _origin: &Name) -> Result<RData, EntityError> {
        Ok(RData::TXT(rdata::TXT::new(vec![self.content])))
    }
}
