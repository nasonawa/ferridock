use actix_web::{route, web::{self, Buf, Data}, HttpRequest, HttpResponse};
use oci_spec::image::{Descriptor, ImageIndex, ImageManifest, MediaType};
use qstring::QString;


use crate::{routes::apierror::{self, ApiError}, storage::{common::Tags, Storage}};


pub fn config(cfg: &mut web::ServiceConfig) {
    cfg
    .service(delete_manifest)
    .service(delete_blob)
    .service(get_tags)
    .service(get_referrers);
}

#[route("/{rep:.*}/manifests/{ref}",method="DELETE")]
async fn delete_manifest(info: web::Path<(String,String)>,store: web::Data<Storage>) -> apierror::Result<HttpResponse> {
    
    let (repo,digest) = info.into_inner();

    if !digest.starts_with("sha256:") {
       return  Ok(HttpResponse::MethodNotAllowed().finish());
    }
    
    let result =  store.delete_manifest(&repo, &digest).await;
    if result.is_err() {return  Err(ApiError::ContentNotFound { kind: MediaType::ImageManifest, mesg: "manifest is unknown".to_string() });}
    
    Ok(HttpResponse::Accepted().finish())
}

#[route("/{rep:.*}/blobs/{digest}",method="DELETE")]
async fn delete_blob(info: web::Path<(String,String)>,store: web::Data<Storage>) -> apierror::Result<HttpResponse> {
  
    let (repo,digest) = info.into_inner();
    let result =  store.delete_blob(&repo, &digest).await;
    if result.is_err() {return Err(ApiError::ContentNotFound { kind: MediaType::Other("Blob".to_string()), mesg: "blob is unknown".to_string() });}
  
    Ok(HttpResponse::Accepted().finish())
            
}

#[route("/{rep:.*}/tags/list",method="GET")]
async fn get_tags(req:HttpRequest,info: web::Path<String>,store: web::Data<Storage>) -> apierror::Result<HttpResponse> {
  
    let repo= info.into_inner();

    let qs = req.query_string();
    let q = QString::from(qs);

    
   let n = match q.get("n") {
        Some(n) => {
           let s =  n.to_string();
           s.parse::<usize>().unwrap()
        },
        None => 0,
    };
        
    let mut tag_list = store.get_tags(&repo).await?;
    if n!= 0&&n < tag_list.tags.len() {
        let new_tags = tag_list.tags.split_off(n+1);
       return   Ok(HttpResponse::Ok().json(Tags{name:tag_list.name,tags:new_tags}));
    }

    Ok(HttpResponse::Ok().json(tag_list))
    
}

//TODO:: add tag=? query
#[route("/{rep:.*}/referrers/{digest}",method="GET")]
async fn get_referrers(req: HttpRequest,info: web::Path<(String,String)>,store: web::Data<Storage>) -> apierror::Result<HttpResponse> {
  
    let (repo,digest) = info.into_inner();
   
    let qs = req.query_string();
    let q = QString::from(qs);

    
   let artifact_type = match q.get("artifactType") {
        Some(a) => a,
        None => "",
    };
    
    let mut index  =store.get_image_index(&repo).await?;
        
    let manifests = index.manifests();
    let mut new_manifests:Vec<Descriptor> = Vec::new();

    for m in manifests.iter() {

       let subject_digest = get_subject_digest(m, repo.clone(), store.clone()).await;

        if (!subject_digest.is_empty())&&subject_digest.eq(&digest){
            new_manifests.push(m.clone());
        }
    }
    
    if !artifact_type.is_empty(){
        for (i,nm) in new_manifests.clone().iter().enumerate() {
            match nm.artifact_type() {
                Some(a) => {
                    if !a.to_string().eq(artifact_type){
                        new_manifests.remove(i);
                    }
                },
                None => {},
            }
            }        
    }
    
    index.set_manifests(new_manifests);

    Ok(HttpResponse::Ok().content_type(MediaType::ImageIndex.to_string()).body(index.to_string().unwrap()))
            
}

async fn get_subject_digest(d: &Descriptor,repo: String, store: Data<Storage>) -> String {
   
    if d.media_type().eq(&MediaType::ImageManifest) {

    let  m_data = store.get_manifest(&repo, &d.digest().to_string()).await.unwrap();
    let manifest = ImageManifest::from_reader(m_data.reader()).expect(d.digest());
   
    return  match manifest.subject(){
        Some(s) =>s.digest().to_string(),
        None => String::new(),
        };
   
    }
    if d.media_type().eq(&MediaType::ImageIndex) {

        let  m_data = store.get_manifest(&repo, &d.digest().to_string()).await.unwrap();
        let manifest = ImageIndex::from_reader(m_data.reader()).expect(d.digest());
        
        return  match manifest.subject(){
            Some(s) =>s.digest().to_string(),
            None => String::new(),
            };
       
        }

    String::new()

}