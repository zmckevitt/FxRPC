# Disk Image Configuration

## Image Download
The disk imageused for the qemu emulation is based on Ubuntu Server 20.04. To configure the disk image, first download a prebuilt cloud image:

```wget https://cloud-images.ubuntu.com/focal/current/focal-server-cloudimg-amd64.img```

## Initialize image

As the downloaded image is a prebuilt cloud image, we need to configure a username and password by creating initialization files:

User-data:
```
cat << EOF > user-data
#cloud-config
password: password
chpasswd:
  expire: False

EOF
```

Meta-data:
```
cat << EOF > meta-data
instance-id: someid/somehostname
local-hostname: jammy

EOF
```

Vendor-data:
```
touch vendor-data
```

After creating each file, run a python http server in another shell in the same directory as the initialization files:
```
python3 -m http.server --directory .
```

Once the initialization files are created and the http server is running, run a qemu instance with the following options to configure the image:
```
qemu-system-x86_64                                            \
    -net nic                                                    \
    -net user                                                   \
    -machine accel=kvm:tcg                                      \
    -cpu host                                                   \
    -m 512                                                      \
    -nographic                                                  \
    -hda focal-server-cloudimg-amd64.img         \
    -smbios type=1,serial=ds='nocloud-net;s=http://10.0.2.2:8000/'
```

This will boot into the Ubuntu Server 20.04 image where you can enter the username ```ubuntu``` and password ```password```

## Enable autologin as root

Once logged into the image, configure auto login as root:

```
sudo mkdir /etc/systemd/system/serial-getty@ttyS0.service.d
sudo vim /etc/systemd/system/serial-getty@ttyS0.service.d/override.conf
```

When editing ```/etc/systemd/system/serial-getty@ttyS0.service.d/override.conf```, paste in the following lines:
```
[Service]
ExecStart=
ExecStart=/sbin/agetty --autologin root -8 --keep-baud 115200,38400,9600 ttyS0 $TERM
```

Lastly, ```sudo reboot now``` to ensure that you can automatically boot into root without a login prompt.

## Running fxmark_grpc binary in guest

To run the ```fxmark_grpc``` program in the guest, first compile it elsewhere (preferrably on a host with Ubuntu 20.04), and scp the binary to the root directory from the guest:

```scp user@host:/path/to/fxmark_grpc/proc/target/release/fxmark_grpc /root```

Note: in order to compile on the guest image, one must clone the repository and follow the instructions in ```README.md``` to build the binary from scratch. This will require extending (resizing) the disk image.

After the binary is installed on the guest, install necessary dependencies:

```
sudo apt update
sudo apt install hwloc
```

Verify that the binary works by trying to run it:
```
/root/fxmark_grpc
```

## Enabling guest to use TAP interface

Once the disk image is configured, the last thing to do is override any prior network configuration, as we will be using our own qemu configuration. To do so, remove existing network plans:
```
rm /etc/netplan/50-cloud-init.yaml
```

Note: without this file, you can no longer use userspace networking, so ensure that all network dependent steps are complete prior to this step!
