use actix_web::{http::Method, route, web::{self}, HttpMessage, HttpRequest, HttpResponse};
use oci_spec::image::MediaType;


use crate::{routes::apierror::{self, ApiError}, storage::{error::StorageError, Storage}};

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg
    .service(pull_manifest)
    .service(pull_blob);
}


#[route("/{rep:.*}/manifests/{ref}",method="GET",method="HEAD")]
async fn pull_manifest(req:HttpRequest,info: web::Path<(String,String)>,store: web::Data<Storage>) -> apierror::Result<HttpResponse>{
    
    let content_type = req.content_type();

    if !content_type.eq(MediaType::ImageManifest.to_string().as_str()) && !content_type.is_empty(){
        let er_resp = format!("the given format is not accepted for manifest {}",content_type);
       return Err(ApiError::InvalidManifestFormat(er_resp));
    }
   
    let (repo,tag) = info.into_inner();
    
    match store.get_manifest(&repo, &tag).await {
    Ok(file) => {
        if req.method().eq(&Method::GET) {
        Ok(HttpResponse::Ok().content_type(MediaType::ImageManifest.to_string()).body(file))
        }else {
            Ok(HttpResponse::Ok().content_type(MediaType::ImageManifest.to_string()).finish())
        }
    },
    Err(e) => {
        match e {
            StorageError::ContenNotFound => {
             Err(ApiError::ContentNotFound { kind: MediaType::ImageManifest, mesg: "manifest is unknown".to_string() })
            }
            _=>{Err(ApiError::Storage(e))}
        }
    },
    }
        
}

#[route("/{rep:.*}/blobs/{digest}",method="GET",method="HEAD")]
async fn pull_blob(req:HttpRequest,info: web::Path<(String,String)>,store: web::Data<Storage>) -> apierror::Result<HttpResponse>{
  
    let (repo,digest) = info.into_inner();
    match store.get_blobs(&repo, &digest).await {
        Ok(file) => {
            if req.method().eq(&Method::GET) {
                Ok(HttpResponse::Ok().body(file))
                
            }else {
                Ok(HttpResponse::Ok().finish())
            }
        },
        Err(e) => {
            match e {
                StorageError::ContenNotFound => {
                    Err(ApiError::ContentNotFound { kind: MediaType::Other("Blob".to_string()), mesg: "blob is unknown".to_string() })
                }
                _=>{Err(ApiError::Storage(e))}
            }
        }
        }
            

}
