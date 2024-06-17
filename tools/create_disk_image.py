#!/usr/bin/python3

# Copyright © 2021 VMware, Inc. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

import signal
import pexpect
from pathlib import Path
from time import sleep

from plumbum import colors, local
from plumbum.cmd import whoami, whoami

# the version of the ubuntu distro to take
UBUNTU_VERSION="jammy"

# packages to be installed
UBUNTU_PACKAGES=["libevent-dev", "libgomp1", "net-tools"]

# The URL of the Ubuntu Cloud Image
DISK_FILE_URL=f"https://cloud-images.ubuntu.com/{UBUNTU_VERSION}/current/{UBUNTU_VERSION}-server-cloudimg-amd64.img"
# the name of the downloaded image
DOWNLOAD_DISK_FILE_NAME=f"{UBUNTU_VERSION}-server-cloudimg-amd64.img"
# the created disk image
DISK_FILE_NAME="ubuntu-server-cloudimg-amd64.img"
# name of the user-data disk with clout-init configuration
USER_DATA_FILE="user-data.iso"
# directory holdingt the cloud-init config files
CONF_DIR="config"

# username and password to be set
USERNAME="ubuntu"
PASSWORD="password"
HOSTNAME=UBUNTU_VERSION

IP_ADDRESS_VM="172.31.0.2"
IP_ADDRESS_BRIDGE="172.31.0.1"
MAC_ADDRESS="56:b4:44:e9:62:d0"

####################################################################################################
# Logging
####################################################################################################

# some logging functionality
def log(msg):
    print(colors.bold | ">>>", end=" "),
    print(colors.bold.reset & colors.info | msg)

####################################################################################################
# Utils to run commands etc
####################################################################################################

def consume_output(qemu, print=False):
    output = ""
    while True:
        try:
            output += qemu.read_nonblocking(size=100, timeout=0.5)
        except Exception:
            break
    if print:
        for l in output.splitlines():
            print(f" > {l}")

# execute a command and wait for the
def do_cmd(qemu, cmd):
    qemu.sendline(f"{cmd}")
    #qemu.expect(f"root@{HOSTNAME}", timeout=30000)
    qemu.expect(f"root@{HOSTNAME}")

# spawns a new Qemu instance
def spawn_qemu(disk, config, conf: bool):

    qemu_cmd = [
        "/usr/bin/env" , "qemu-system-x86_64",
        "-name", f"cloud-init,debug-threads=on",
        "-enable-kvm", "-nographic",
        "-machine", "q35",
        # CPU configuration
        "-cpu", "host,migratable=no,+invtsc,+tsc,+x2apic,+fsgsbase",
        "-smp", f"2,sockets=1,maxcpus=2",
        # memory
        "-m", f"1024M",
        # networking
        "-device", f"virtio-net,netdev=nd0,mac={MAC_ADDRESS}",
        "-netdev", f"tap,id=nd0,script=no,ifname=tap",
        "-drive", f"file={str(disk)},if=virtio"
    ]

    if config != None:
        qemu_cmd.extend(["-drive", f"file={str(config)},if=virtio"])

    qemu_cmd_str = " ".join(qemu_cmd)
    print(f" > spawning qemu with command: `{qemu_cmd_str}`")
    with open("qemucmd.txt", "w") as f:
        f.write(qemu_cmd_str + "\n")

    qemu = pexpect.spawn(qemu_cmd_str)
    qemu.logfile = open('qemulog.txt','wb')
    return qemu

# wait for login prompt to appear
def wait_for_login(qemu) :
    qemu.expect(f"Ubuntu 22.04.4 LTS {UBUNTU_VERSION} ttyS0", timeout=360)

# wait for the prompt to appear
def wait_for_prompt(qemu) :
    qemu.expect(f"ubuntu@{HOSTNAME}", timeout=30)

# perform login
def login_as(qemu, username, password) :
    print(f" > Logging in as {username}")
    qemu.sendline(f"{username}")
    qemu.expect("Password:")
    qemu.sendline(f"{password}")
    print(" > Check Login Success")
    wait_for_prompt(qemu)
    print(" > Logged in as {username}")


####################################################################################################
# Host Network Configuration
####################################################################################################

# clenup networking setup
def reset_networking():
    from plumbum.cmd import sudo, ip, brctl, iptables

    print(" > reset networking")

    # delete the bridge
    sudo[ip[['link', 'set', 'br0', 'down']]](retcode=(0, 1))
    sudo[brctl[['delbr', 'br0']]](retcode=(0, 1))

    # delete the TAP interface
    sudo[ip[['link', 'set', 'tap', 'down']]](retcode=(0, 1))
    sudo[ip[['link', 'del', 'tap']]](retcode=(0, 1))

    # delete the IP tables forwarding rules
    sudo[iptables[["-F", "FORWARD"]]](retcode=(0, 1))

# setup networking so the VM can reach the internet to install packages
def setup_networking():
    """
    Configure the host network stack to allow host/cross VM communication.
    """
    from plumbum.cmd import sudo, tunctl, ip, brctl, iptables, sysctl

    from plumbum.machines import LocalCommand

    # hack to make the ! below owrk
    quote_level = LocalCommand.QUOTE_LEVEL
    LocalCommand.QUOTE_LEVEL = 3

    # remove existing interface
    reset_networking()

    print(" > setting up networking")

    user = (whoami)().strip()
    group = (local['id']['-gn'])().strip()

    # create the new bridge
    sudo[ip[['link', 'add', 'br0', 'type', 'bridge']]]()
    sudo[ip[['addr', 'add', f"{IP_ADDRESS_BRIDGE}/24", 'brd', '+', 'dev', 'br0']]]()

    # create the TAP interface, add to bridge
    sudo[tunctl[['-t', "tap", '-u', user, '-g', group]]]()
    sudo[ip[['link', 'set', "tap", 'up']]](retcode=(0, 1))
    sudo[brctl[['addif', 'br0', "tap"]]]()

    # set the bridge online
    sudo[ip[['link', 'set', 'br0', 'up']]](retcode=(0, 1))

    # set the IP table rules for fowarding
    sudo[iptables[[ "-I",  "FORWARD",  "-i",  "br0",  "-j", "ACCEPT" ]]]()
    sudo[iptables[[ "-I",  "FORWARD",  "-o",  "br0",  "-m",  "state", "--state", "RELATED,ESTABLISHED", "-j", "ACCEPT"]]]()
    sudo[iptables[[ "-t",  "nat",  "-A",  "POSTROUTING", '!', "-o", "br0", "--source",  f"{IP_ADDRESS_VM}/24", "-j", "MASQUERADE"]]]()

    # enable IP forwarding
    sudo[sysctl[[ "-w", "net.ipv4.ip_forward=1"]]]()

    # restore the quote level
    LocalCommand.QUOTE_LEVEL = quote_level

####################################################################################################
# Downloading Image
####################################################################################################

# downloads the cloud image and prepares the writable disk
def download_image():
    from plumbum.cmd import wget
    img = Path(DOWNLOAD_DISK_FILE_NAME)
    if img.exists():
        log("Download image: (use cached)")
    else :
        log(f"Download image: downloading from {DISK_FILE_URL}")
        wget(DISK_FILE_URL, "-O", DOWNLOAD_DISK_FILE_NAME)

    img = Path(DISK_FILE_NAME)
    img.unlink(missing_ok=True)

    print(" > create qcow2 image..")

    qemuimg = local["qemu-img"]
    qemuimg("create", "-f", "qcow2", "-b", DOWNLOAD_DISK_FILE_NAME, "-F", "qcow2",  DISK_FILE_NAME)
    return Path(DISK_FILE_NAME)


####################################################################################################
# Run Cloud Init
####################################################################################################

# prepare the image configuration
def prepare_cloud_init_config():
    log(f"Prepare cloud-init configuration")
    confdir = Path(CONF_DIR)
    confdir.mkdir(parents = True, exist_ok = True)

    print(f" > write configuration to {CONF_DIR}")

    # write the user data
    with open(confdir / "user-data", "w") as ud:
        ud.write("#cloud-config\n")
        ud.write(f"password: {PASSWORD}\n")
        ud.write("chpasswd: { expire: False }\n")
        ud.write("ssh_pwauth: True\n")

    with open(confdir / "network-config.yaml", "w") as nd:
        nd.write( "network:\n")
        nd.write( "  version: 2\n")
        nd.write( "  ethernets:\n")
        nd.write( "    ens3:\n")
        nd.write( "      match:\n")
        nd.write(f"        macaddress: '{MAC_ADDRESS}'\n")
        nd.write(f"      dhcp4: false\n")
        nd.write(f"      set-name: ens3\n")
        nd.write(f"      dhcp6: false\n")
        nd.write( "      addresses:\n")
        nd.write(f"        - {IP_ADDRESS_VM}/24\n")
        nd.write(f"      gateway4: {IP_ADDRESS_BRIDGE}\n")
        nd.write( "      nameservers:\n")
        nd.write( "        addresses: [8.8.8.8]\n")

    # write the meta-data
    with open(confdir / "meta-data", "w") as md:
        md.write("instance-id: someid/somehostname\n")
        md.write("local-hostname: jammy\n\n")
    # write the meta-data
    with open(confdir / "vendor-data", "w") as vd:
        vd.write("# vendor data\n")
        vd.write("\n")

    print(f" > create the user-data file: {USER_DATA_FILE}")
    userdata = local["cloud-localds"]
    userdata(USER_DATA_FILE, "--dsmode", "local",
            f"--network-config", str(confdir / "network-config.yaml"),
             "--hostname", HOSTNAME, str(confdir / "user-data"))

    return Path(USER_DATA_FILE)

# enable autologin for the root user, expects qemu instance in root shell
def enable_root_autologin(qemu):
    print(" > enabling root autologin")
    # see https://ostechnix.com/ubuntu-automatic-login/
    do_cmd(qemu, "mkdir /etc/systemd/system/serial-getty@ttyS0.service.d")
    do_cmd(qemu, "echo '[Service]' > /etc/systemd/system/serial-getty@ttyS0.service.d/override.conf")
    do_cmd(qemu, "echo 'ExecStart=' >> /etc/systemd/system/serial-getty@ttyS0.service.d/override.conf")
    do_cmd(qemu, "echo 'ExecStart=/sbin/agetty --noissue --autologin root %I $TERM' >> /etc/systemd/system/serial-getty@ttyS0.service.d/override.conf")
    do_cmd(qemu, "echo 'Type=idle' >> /etc/systemd/system/serial-getty@ttyS0.service.d/override.conf")
    do_cmd(qemu, "cat /etc/systemd/system/serial-getty@ttyS0.service.d/override.conf")

    print(" > rebooting...")
    qemu.sendline("reboot now\n")

    qemu.expect(f"{HOSTNAME} login: root \(automatic login\)", timeout=360)
    print(" > successfully logged in as root!")
    qemu.sendline("\n\n")
    qemu.expect(f"root@{HOSTNAME}")

# disable the network check during boot
def disable_network_check(qemu):
    print(" > disabling network check")
    do_cmd(qemu, "systemctl mask systemd-networkd-wait-online.service")

# configures the IP of the virtual machine
def configure_ip(qemu):
    netif = "ens3"
    # bring up ip address
    do_cmd(qemu, f"ip address flush dev {netif}")
    do_cmd(qemu, f"ip route flush dev {netif}")
    do_cmd(qemu, f"ip address add {IP_ADDRESS_VM}/24 brd + dev {netif}")
    do_cmd(qemu, f"ip link set {netif} up")
    do_cmd(qemu, f"ip route add {IP_ADDRESS_BRIDGE} dev  {netif}")
    do_cmd(qemu, f"ip route add default via {IP_ADDRESS_BRIDGE} dev {netif}")
    do_cmd(qemu, f"ip address show dev {netif}")

    # setting dns server
    do_cmd(qemu, f"resolvectl dns ens3 8.8.8.8")

def install_packages(qemu, packages):
    packages = " ".join(packages)
    print(f" > Installing packages: {packages}" )
    do_cmd(qemu, f"apt install -y {packages}")

def install_fxrpc(qemu):
    print(f" > Installing fxrpc")

    do_cmd(qemu, f"cd /root")

    qemu.sendline(f"wget https://github.com/zmckevitt/fxmark_grpc/raw/dinos-rpc/run/bin/fxrpc")
    qemu.expect(f"root@{HOSTNAME}", timeout=60)
    
    do_cmd(qemu, f"chmod +x fxrpc")
    do_cmd(qemu, f"apt update")
    do_cmd(qemu, f"apt install -y hwloc")

# configures the cloud image based on the configuration, returns qemu instance in root shell
def run_cloud_init(disk, config):
    log(f"Configuring Image...")
    setup_networking()

    qemu = spawn_qemu(disk, config, True)
    wait_for_login(qemu)

    # login prompt
    qemu.expect("jammy login", timeout=30)
    login_as(qemu, USERNAME, PASSWORD)

    
    print(" > Switching to root")
    do_cmd(qemu, f"sudo su")
    consume_output(qemu, False)

    # remove the networking configuration, as we will do that later
    do_cmd(qemu, f"mv /etc/netplan/50-cloud-init.yaml /root")

    # disable the networking check at boot
    disable_network_check(qemu)

    # configure the IP
    configure_ip(qemu)

    # installing packets
    install_packages(qemu, UBUNTU_PACKAGES)
   
    # install fxrpc 
    install_fxrpc(qemu)

    # reset the networking configuration again
    reset_networking()

    # enabel the root autologin
    enable_root_autologin(qemu)

    return qemu

def mount_disk(disk, mountpoint):
    mp = Path(mountpoint)
    mp.mkdir(parent=True, exist_ok=True)

    pidfile = Path("guestmount.pid")
    if pidfile.exists():
        pidfile.unlink()

    guestmount = local["guestmount"]
    guestmount("-a", str(disk), "--pid-file", "guestmount.pid", "--rw", "-i", str(mp))
    return pidfile

def umount_disk(pidfile, mountpoint):
    with open(pidfile, "r") as pf:
        pid = pf.read()

    guestunmount = local["guestunmount"]
    guestunmount(str(mountpoint))


    kill = local("kill")
    for _ in range(0, 10):
        kill("-0", pid)
        sleep(1)

disk = download_image()
config = prepare_cloud_init_config()
qemu = run_cloud_init(disk, config)

# terminate qemu
qemu.kill(sig=signal.SIGKILL)
