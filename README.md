# ferridock

Ferridock is a lightweight, bare-minimum OCI container registry built from the ground up in Rust. Designed for simplicity and compliance, it supports storing OCI container images (exluding Docker images) and has successfully passed all OCI Distribution Conformance Tests.

## Overview

Ferridock leverages Rust’s performance and safety to provide a minimal yet functional container registry. It uses:
- **OpenDAL**: A versatile storage backend abstraction crate, enabling flexible and efficient image storage.
- **Actix**: A powerful Rust framework for building the REST API server, ensuring a robust and responsive interface.

This project focuses on core functionality—storing and serving OCI container images only. at moment images can be stored locally or cloud storage with S3.

---
## Building and Running

Ferridock is easy to build and run using Rust’s `cargo` tool. Follow these steps:

1. **Prerequisites**  
   Ensure you have Rust and Cargo installed. You can install them via [rustup](https://rustup.rs/).

2. **Clone the Repository**  
   ```bash
   git clone https://github.com/nasonawa/ferridock.git
   cd ferridock
   ```

3. **Build the Project**  
   Compile Ferridock with:
   ```bash
   cargo build --release
   ```
   This generates an optimized binary in the `target/release/` directory.

4. **Run the Registry**  
   Start the registry with:
   ```bash
   cargo run --release /path/config.yaml
   ```
   By default, Ferridock will use the local filesystem as the storage backend. To configure S3 or tweak settings, refer to the configuration documentation (TBD).

    ```yaml
    server:
    address: 127.0.0.1
    storage:
    s3:
        url: 
        access_key: s3-access-key
        secret_key: s3-secret-key
        bucket: bucket-name
        cache: path/
        region: us-east-1
    local:
    path: path
    ```

## Pushing and Pulling Images

To push image use below podman command. 

   ```bash
   podman push -f oci --tls-verify=false [IMAGE]:[ID] docker://localhost:8080/[IMAGE]:[TAG]
   ```

To pull the image use below podman command. 

   ```bash
   podman pull -tls-verify=false docker://localhost:8080/[IMAGE]:[TAG]
   ```
## Feedback

Feel free to clone and fork the repository and give it try-any feedback releated to the code is welcomed. 