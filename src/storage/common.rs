
use serde::{Deserialize, Serialize};


#[derive(Deserialize,Serialize,Default,Debug)]
pub struct Tags {
   pub name: String,
   pub tags: Vec<String>
}


