use std::fmt::Display;

use opendal::services::S3;
use serde::{Deserialize, Serialize};

#[derive(Serialize,Deserialize,Default,Debug)]
#[serde(default)]
pub struct AppConfig {
  pub server: Server,
  pub storage: Storage
}


#[derive(Serialize,Deserialize,Debug)]
#[serde(default)]
pub struct Server {
  pub address: String,
}

impl Default for Server {
    fn default() -> Self {
        Self { address: default_ip()}
    }
}

fn default_ip() -> String {
  String::from("0.0.0.0")
}

#[derive(Serialize,Deserialize,Default,Debug)]
#[serde(default)]
pub struct Storage {
  s3: S3Storage,
  local: Local
}

#[derive(Debug)]
pub enum StorageConfigError{
  S3Error(String)

}

impl Display for StorageConfigError{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f,"{}",self)
    }
}

impl Storage {

    pub fn create_s3_op(&self) -> Result<S3,StorageConfigError> {
      
      self.validate_s3_config()?;

        Ok(S3::default().access_key_id(&self.s3.access_key)
                    .secret_access_key(&self.s3.secret_key)
                    .endpoint(&self.s3.url)
                    .bucket(&self.s3.bucket)
                    .region(&self.s3.region))
    
    }
    fn validate_s3_config(&self) -> Result<(),StorageConfigError> {
    
      let e = ["url","access_key","secret_key","bucket"]
      .iter().zip([&self.s3.url,&self.s3.access_key,&self.s3.secret_key,&self.s3.bucket])
      .find(|(_,value)| value.trim().is_empty())
      .map_or(Ok(()), |(field,_)| Err(StorageConfigError::S3Error(format!("{} is not configured",field))));

      e

    }

    pub fn get_local(&self) -> String {

        if self.local.path.is_empty() {return String::from("/tmp/.armar");}

        return self.local.path.clone();

    }

}

#[derive(Serialize,Deserialize,Default,Debug)]
#[serde(default)]
pub struct S3Storage {
  url: String,
  access_key: String,
  secret_key: String,
  bucket: String,
  region:String,
  cache: String,
}

#[derive(Serialize,Deserialize,Default,Debug)]
struct Local {
  path: String
}