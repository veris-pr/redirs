use rand::{
    distr::{Alphanumeric, SampleString},
    rng,
};

#[derive(Debug, PartialEq, Clone)]
pub struct MasterConfig {
    pub host: String,
    pub port: u16,
}

pub struct ReplicationConfig {
    pub master: Option<MasterConfig>,
    pub replid: String,
    pub repl_offset: usize,
}

impl ReplicationConfig {
    pub fn new_master() -> Self {
        ReplicationConfig {
            master: None,
            replid: Alphanumeric.sample_string(&mut rng(), 40),
            repl_offset: 0,
        }
    }

    pub fn new_replica(master_host: String, master_port: u16) -> Self {
        ReplicationConfig {
            master: Some(MasterConfig {
                host: master_host,
                port: master_port,
            }),
            replid: Alphanumeric.sample_string(&mut rng(), 40),
            repl_offset: 0,
        }
    }

    pub fn info(&self) -> ReplicationInfo {
        ReplicationInfo {
            role: match &self.master {
                Some(_) => Role::Replica,
                None => Role::Master,
            },
            replid: self.replid.clone(),
            repl_offset: self.repl_offset,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Role {
    Master,
    Replica,
}

#[derive(Debug, PartialEq)]
pub struct ReplicationInfo {
    pub role: Role,
    pub replid: String,
    pub repl_offset: usize,
}
