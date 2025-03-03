use std::{collections::HashMap, path::{Path, PathBuf}};

use actix_web::web::{Buf, Bytes};
use oci_spec::image::{Descriptor, ImageIndex, ImageIndexBuilder, ImageManifest, MediaType};
use opendal::{Buffer, Operator};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::{common::Tags, error::StorageError};
use crate::storage::error::Result;

pub struct Storage {
    primary: Operator,
    cache: Operator
}

pub fn new(primary: Operator,cache: Operator) -> Storage {Storage{primary,cache}}

impl Storage {
    
pub async fn get_manifest(&self,repo:&String,tag:&String) -> Result<Vec<u8>>{
            if tag.starts_with("sha256:") {
               let data =  self.get_blobs(repo, tag).await?;
               return  Ok(data);

            }else {
                let img_index = self.get_image_index(repo).await?;
                for  m in img_index.manifests().into_iter(){

                    let a = m.annotations().as_ref();
                    if a.is_some() {
                        let a = a.unwrap();   
                        let ta = a.get("org.opencontainers.image.ref.name");
                        
                        if ta.is_some() && ta.unwrap().eq(tag){
                            let data = self.get_blobs(repo, m.digest()).await?;
                            return  Ok(data);
                        }
                    }
                   
                }
            }
           Err(StorageError::ContenNotFound)
    }

pub async fn get_blobs(&self,repo:&String,digest:&String) -> Result<Vec<u8>> {
    
    let blob_path = Self::create_blob_path(repo, digest);
    let p = blob_path.to_str().unwrap();
    let d = self.primary.read(p).await?;

    return  Ok(d.to_vec());
 }

pub async fn get_tags(&self,repo:&String) -> Result<Tags>{
    
    let tag_path = Path::new("repo").join(repo).join("tags.json");
    
    match  self.cache.read(&tag_path.to_str().unwrap()).await {
        Ok(data) => {
            
            let tags:Tags = serde_json::from_reader(data.reader())?;
            return Ok(tags);   
        },
        Err(_) => {

        let t = Tags{name: repo.to_string(),tags:Vec::new()};
        let data = serde_json::to_vec(&t)?;
        self.cache.write(&tag_path.to_str().unwrap(), data).await?;

        return  Ok(t);
        },
    }
}

pub async fn update_tags(&self,repo:&String,tag:Tags)-> Result<()>{

    let tag_path = Path::new("repo").join(repo).join("tags.json");
    let data = serde_json::to_vec(&tag)?;

    self.cache.write(&tag_path.to_str().unwrap(), data).await?;

    Ok(())

}
pub async fn write_manifest(&self,repo:&String,tag:&String,data: Bytes,size: usize,media_type: &String) -> Result<(String,String)> {

    let digest = Self::digest_from_content(&data);
    let mut tagannotaions = HashMap::<String,String>::new();
    let mut tags = self.get_tags(repo).await?;
   

    match tag.starts_with("sha256:") {
        true => {
            let path = Self::create_blob_path(repo, tag);
            self.primary.write(&path.to_str().unwrap(), data.to_vec()).await?;
        },
        false => {
            tagannotaions.insert("org.opencontainers.image.ref.name".to_owned(), tag.clone());
            tags.tags.push(tag.to_string());
            self.update_tags(repo, tags).await?;
            
            let path = Self::create_blob_path(repo, &digest);
            self.primary.write(&path.to_str().unwrap(), data.to_vec()).await?;


        },
    };
    let mut img_index = self.get_image_index(repo).await.unwrap();

    let mut descriptors = img_index.manifests().to_owned();
    let mut subject_digest = String::new();

    let mut descriptor: Descriptor = Descriptor::new(MediaType::from(media_type.as_str()),size as i64 , digest.clone());

   if String::from(MediaType::ImageManifest).eq(media_type) {
    let m = ImageManifest::from_reader(data.clone().reader())?;
     subject_digest = match m.subject() {
        Some(s) =>s.digest().to_string(),
        None => "".to_string(),
    };
    descriptor.set_annotations(m.annotations().clone());
   }else if String::from(MediaType::ImageIndex).eq(media_type){
    let i= ImageIndex::from_reader(data.reader())?; 
    subject_digest = match i.subject() {
        Some(s) =>s.digest().to_string(),
        None => "".to_string(),
    };
    descriptor.set_annotations(i.annotations().clone());

   }

   let mut an: HashMap<String, String> ;
   if descriptor.annotations().is_some() {
    an = descriptor.annotations().to_owned().unwrap();
    an.extend(tagannotaions.into_iter());
   }else {
    an = tagannotaions;
   }

   descriptor.set_annotations(Some(an));
   descriptors.push(descriptor);
   img_index.set_manifests(descriptors);
    self.update_image_index(repo, img_index).await?;

    return Ok((digest,subject_digest));

}

pub async fn new_blob_upload(&self,repo:&String) -> Result<String> {
    let upload_uuid = Uuid::new_v4();

    let f_path = Path::new("repo").join(repo).join(".cache").join(upload_uuid.to_string());

    self.cache.write(&f_path.to_str().unwrap(),Buffer::new()).await?;

    Ok(upload_uuid.to_string())
}

pub async fn update_blob_upload(&self,repo:&String,location:&String,from:u64,data: Vec<u8>) -> Result<()> {

    let f_path =Path::new("repo").join(repo)
                              .join(".cache")
                              .join(location); 
    let meta = self.cache.stat(&f_path.to_str().unwrap()).await?;
    if meta.content_length() != from {
        return Err(StorageError::RangeIsNotStatisfied);
    }
    self.cache.write_with(&f_path.to_str().unwrap(), data).append(true).await?;
    Ok(())
}

pub async fn get_blob_upload(&self,repo:&String,location:&String) -> Result<usize> {
    
    let f_path =Path::new("repo").join(repo)
                              .join(".cache")
                              .join(location); 
    
   let meta = self.cache.stat(&f_path.to_str().unwrap()).await?;
   let  n =  meta.content_length();
   Ok(n as usize)
}

pub async fn streamed_blob_upload(&self,repo:&String,location:&String,data: Vec<u8>) -> Result<()> {
    let f_path =Path::new("repo").join(repo)
                              .join(".cache")
                              .join(location); 
    
   self.cache.write_with(&f_path.to_str().unwrap(), data).append(true).await?;
   
   Ok(())
}
pub async fn delete_blob_upload(&self,repo:&String,digest:&String,location:&String) -> Result<()>{
    let cached_blob = Path::new("repo").join(repo).join(".cache").join(location);

    let data = self.cache.read(&cached_blob.to_str().unwrap()).await?;
    
    let blob_path = Self::create_blob_path(repo, digest);
   
    self.primary.write(&blob_path.to_str().unwrap(), data).await?;
    self.cache.delete(&cached_blob.to_str().unwrap()).await?;

    Ok(())
}

pub async fn delete_manifest(&self,repo:&String,digest:&String) -> Result<()>{

    let mut index = self.get_image_index(repo).await.unwrap();
    let mut new_manifests = index.manifests().clone();
    let mut repo_tags = self.get_tags(repo).await?;
   
    for m in new_manifests.clone().iter() {
        let an = m.annotations().clone().or(Some(HashMap::new())).unwrap();
        match an.contains_key("org.opencontainers.image.ref.name")  {
            true => {
               let tag =  an.get("org.opencontainers.image.ref.name").unwrap();                              
                repo_tags.tags.retain(|t: &String| {
                    !t.eq(tag)
                });
            },
            false =>{},
        };
    }     

    new_manifests.retain(| d| {
        !d.digest().eq(digest)
    });

    index.set_manifests(new_manifests);
   
    self.update_tags(repo, repo_tags).await?;
    self.update_image_index(repo, index).await.unwrap();

    let blob_path = Self::create_blob_path(repo, digest);

    self.primary.delete(&blob_path.to_str().unwrap()).await?;

    Ok(())
}

pub async fn delete_blob(&self,repo:&String,digest:&String) -> Result<()>{

    let blob_path = Self::create_blob_path(repo, digest);
    self.primary.delete(&blob_path.to_str().unwrap()).await?;

    Ok(())

}   

pub async fn get_image_index(&self,repo:&String) -> Result<ImageIndex> {
  
  let index_path =  Path::new("repo").join(repo).join("index.json");

    match self.primary.read(&index_path.to_str().unwrap()).await {
        Ok(data) =>{
            let index = ImageIndex::from_reader(data.reader()).expect("error in reading index");
            return Ok(index);
        },
        Err(_) => {
            let index = ImageIndexBuilder::default()
            .schema_version(2 as u32)
            .media_type("application/vnd.oci.image.index.v1+json")
            .manifests(Vec::new())
            .build()?;
           let data =  index.to_string()?;   
           self.primary.write(&index_path.to_str().unwrap(), data.into_bytes().to_vec()).await?;
           return  Ok(index);
        },
    }  

}


async fn  update_image_index(&self,repo:&String,index:ImageIndex) -> Result<()>{
    
    let index_path =  Path::new("repo").join(repo).join("index.json");

    let data =  index.to_string()?;
    self.primary.write_with(&index_path.to_str().unwrap(), data.into_bytes().to_vec()).await?;

    Ok(())
}

fn create_blob_path(repo:&String,digest:&String) -> PathBuf {

    Path::new("repo").join(repo).join("blobs").join(digest)
}


fn digest_from_content(data: &Bytes) -> String{
        
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();

    format!("sha256:{:x}",hash)
}

}