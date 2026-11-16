# verify-app drive 20261112-132339-fast-1004183
scope: fast; features: registry
checkout: /mnt/data/Projects/goglbe/waggledance @ 601f92d
binary: /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance
binary sha256: 5f4255cac6eaf27efc9dfd046ad08cdcaf0c506726a85dcd58ed9a5b4dd4f3e1  mtime: 2026-11-11T23:57:32+07:00
binary version: waggledance 0.5.5

doctor: ok pid=1004224 version=0.5.5 home=/mnt/data/Projects/goglbe/waggledance/.bee/tmp/verify-app/20261112-132339-fast-1004183/home
== registry
### 01 registry-list-empty  (exit 0)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance list 
No projects registered.
```
### 02 registry-register  (exit 0)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance register /mnt/data/Projects/goglbe/waggledance/.bee/tmp/verify-app/20261112-132339-fast-1004183/repo --name verify-scratch --json 
{"project_id":"repo","name":"verify-scratch","root_path":"/mnt/data/Projects/goglbe/waggledance/.bee/tmp/verify-app/20261112-132339-fast-1004183/repo"}
```
### 03 registry-list  (exit 0)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance list 
repo                 /mnt/data/Projects/goglbe/waggledance/.bee/tmp/verify-app/20261112-132339-fast-1004183/repo
```
### 04 registry-projects-json  (exit 0)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance projects 
{"projects":[{"id":"repo","name":"verify-scratch","root_path":"/mnt/data/Projects/goglbe/waggledance/.bee/tmp/verify-app/20261112-132339-fast-1004183/repo"}]}
```
### 05 registry-status  (exit 0)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance status 
running: pid 1004224
projects: 1
```
### 06 registry-unregister  (exit 0)
```
$ env -i HOME=$SCRATCH_HOME PATH=/usr/bin:/bin LANG=C.UTF-8 TERM=dumb RUST_LOG=info /mnt/data/Projects/goglbe/waggledance/target/fast/waggledance unregister repo 
Unregistered 'repo'.
```
doctor: ok pid=1004224 version=0.5.5 home=/mnt/data/Projects/goglbe/waggledance/.bee/tmp/verify-app/20261112-132339-fast-1004183/home
