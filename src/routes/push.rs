use actix_web::{http::header::{self, HeaderValue}, route, web::{self, Bytes}, HttpMessage, HttpRequest, HttpResponse};
use qstring::QString;

use crate::{routes::apierror::{self, ApiError}, storage::{error::StorageError, Storage}};

 pub fn config(cfg: &mut web::ServiceConfig) {
    cfg
        .service(push_manifest)
        .service(create_blob_uploads)
        .service(update_blob)
        .service(update_blob_chunks)
        .service(get_stale_blob_upload);
        
}

#[route("/{rep:.*}/manifests/{ref}",method="PUT")]
 async fn push_manifest(req: HttpRequest,info: web::Path<(String,String)>,store: web::Data<Storage>,file: Bytes) -> apierror::Result<HttpResponse> {
   
    let (repo,reff) = info.into_inner();
   
    let content_len = file.len();
   
    let media_type = req.content_type();

   let (digest,subject) = store.write_manifest(&repo,&reff,file,content_len,&media_type.to_string()).await?;
    
    Ok(HttpResponse::Created()
                .append_header(("Location",format!("/{}/manifests/{}",repo,digest)))
                .append_header(("OCI-Subject",subject)).finish())
}

#[route("/{rep:.*}/blobs/uploads/{uuid}",method="GET")]
async fn get_stale_blob_upload(info: web::Path<(String,String)>,store: web::Data<Storage>) -> apierror::Result<HttpResponse> {
    
    let (repo,uuid) = info.into_inner();
    
    let n = match store.get_blob_upload(&repo, &uuid).await {
    Ok(n) => n,
    Err(e) => match e {
        StorageError::ContenNotFound => {return  Err(ApiError::BlobUploadUnknown);},
        e => {return Err(ApiError::Storage(e));}
    }};

    let location: String = format!("/v2/{repo}/blobs/uploads/{uuid}");
     
    Ok(HttpResponse::NoContent()
    .insert_header(("location",location))
    .insert_header(("Range",format!("0-{}",n-1)))
    .finish())
    

}
//TODO: add the cross mount blob
#[route("/{rep:.*}/blobs/uploads/",method="POST")]
 async fn create_blob_uploads(req:HttpRequest,info: web::Path<String>,store: web::Data<Storage>,file: Bytes) -> apierror::Result<HttpResponse>{

    let repo = info.into_inner();
    let data = file.to_vec();
    let qs = req.query_string();
    let q = QString::from(qs);

    let (digest,ok) = match q.get("digest") {
        Some(digest) =>(digest,true),
        None => ("",false),
    };

    let (_,is_mount) = match q.get("mount") {
        Some(mount) =>(mount,true),
        None => ("",false),
    };
    

    let uuid =  store.new_blob_upload(&repo).await?;
    let location: String = format!("/v2/{repo}/blobs/uploads/{uuid}");

    if is_mount {
        return Ok(HttpResponse::Accepted().insert_header(("location",location)).finish());
    }

    
    if ok && !data.is_empty(){
       
       store.update_blob_upload(&repo, &uuid, 0, data).await.unwrap();
       let _ = store.delete_blob_upload(&repo, &digest.to_string(), &uuid).await?;
  
       return Ok(HttpResponse::Created().insert_header(("location",format!("/v2/{repo}/blobs/{digest}"))).finish());
    
    }


    Ok(HttpResponse::Accepted().insert_header(("location",location)).finish())
}


#[route("/{rep:.*}/blobs/uploads/{uuid}",method="PUT")]
 async fn update_blob(req:HttpRequest,info: web::Path<(String,String)>,store: web::Data<Storage>,file: Bytes) -> apierror::Result<HttpResponse> {
    
    let (repo,uuid) = info.into_inner();
    let data = file.to_vec();
    
    let qs = req.query_string();
    let q = QString::from(qs);
    
    let (digest,ok) = match q.get("digest") {
        Some(digest) =>(digest,true),
        None => ("",false),
    };

    if ok  {
        if  !data.is_empty() {
            store.streamed_blob_upload(&repo, &uuid, data).await?;
        }       
       let _ = store.delete_blob_upload(&repo, &digest.to_string(), &uuid).await?;
       let location = format!("/v2/{repo}/blobs/{digest}");
       return Ok(HttpResponse::Created().insert_header(("location",location)).finish());
    }

    let _ = store.delete_blob_upload(&repo, &digest.to_string(), &uuid);
    Ok(HttpResponse::Created().finish())
}

#[route("/{rep:.*}/blobs/uploads/{uuid}",method="PATCH")]
 async fn update_blob_chunks(req:HttpRequest,info: web::Path<(String,String)>,store: web::Data<Storage>,file: Bytes) -> apierror::Result<HttpResponse> {
    
    let (repo,uuid) = info.into_inner();
    let data = file.to_vec();
    let len = data.len();
    let location: String = format!("/v2/{repo}/blobs/uploads/{uuid}");
    

    /*TODO: logic for content range header blob upload */
    let ((from,_),ok):((usize,usize),bool)= match req.headers().get(header::CONTENT_RANGE){
         Some(r) =>{
            (parse_range(r),true)
         } ,
         None => ((0,0),false),
     };
        
     let mut n=0;

    if ok {
        store.update_blob_upload(&repo, &uuid, from as u64, data).await?;
        n = len-1;
        }
        else {
        store.streamed_blob_upload(&repo, &uuid, data).await?;    
        }

    Ok(HttpResponse::Accepted()
    .insert_header(("location",location))
    .insert_header(("Range",format!("0-{}",n)))
    .finish())

}


fn parse_range(header: &HeaderValue) -> (usize,usize){
    let s = header.to_str().unwrap();

    let a:Vec<&str> = s.split("-").collect();

    let from: usize = a[0].parse().unwrap();
    let to: usize = a[1].parse().unwrap();
    (from,to)
}
