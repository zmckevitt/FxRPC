# Memcached Benchmark

## Dependencies

```
# sudo apt-get install cloud-image-utils libguestfs-tools
```

## Running

```
# python3 ../tools/create_disk_image.py
# cargo run -- --image ubuntu-server-cloudimg-amd64.img --memory 4069 --queries 10000000
```
