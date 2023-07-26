#!/usr/bin/python3

# Copyright © 2021 VMware, Inc. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

import argparse
import os
import sys
import pathlib
import shutil
import subprocess
import prctl
import signal
import toml
import pexpect
import plumbum
import re
import errno
from time import sleep
import tempfile

from plumbum import colors, local, SshMachine
from plumbum.commands import ProcessExecutionError

from plumbum.cmd import whoami, python3, cat, getent, whoami

BOOT_TIMEOUT = 60
EXP_TIMEOUT = 10000000
CSV_FILE = "fxmark_grpc_benchmark.csv" 

def get_network_config(workers):
    """
    Returns a list of network configurations for the workers.
    """
    config = {}
    for i in range(workers):
        config['tap{}'.format(2*i)] = {
            'mid': i,
            'mac': '56:b4:44:e9:62:d{:x}'.format(i),
        }
    return config

MAX_WORKERS = 16
NETWORK_CONFIG = get_network_config(MAX_WORKERS)
NETWORK_INFRA_IP = '172.31.0.20/24'

#
# Command line argument parser
#
parser = argparse.ArgumentParser()

parser.add_argument("--image", required=True, help="Specify disk image to use")
parser.add_argument("--scores", type=int, required=True, default=1, help="Cores for server")
parser.add_argument("--clients", type=int, required=True, default=1, help="Setup n clients")
parser.add_argument("--cores", type=int, required=True, default=1, help="Cores per client")
parser.add_argument("--wratio", nargs="+", required=True, help="Specify write ratio for mix benchmarks")
parser.add_argument("--openf", nargs="+", required=True, help="Specify number of open files for mix benchmarks")
parser.add_argument("--duration", type=int, required=True, default=10, help="Experiment duration")

subparser = parser.add_subparsers(help='Advanced network configuration')

# Network setup parser
parser_net = subparser.add_parser('net', help='Network setup')

parser_net_mut = parser_net.add_mutually_exclusive_group(required=False)
parser_net_mut.add_argument("--network-only", action="store_true", default=False,
                            help="Setup network only.")
parser_net_mut.add_argument("--no-network-setup", action="store_true", default=False,
                            help="Setup network.")

def log(msg):
    print(colors.bold | ">>>", end=" "),
    print(colors.bold.reset & colors.info | msg)

def configure_network(args):
    """
    Configure the host network stack to allow host/cross VM communication.
    """
    from plumbum.cmd import sudo, tunctl, ifconfig, ip, brctl

    user = (whoami)().strip()
    group = (local['id']['-gn'])().strip()

    # TODO: Could probably avoid 'sudo' here by doing
    # sudo setcap cap_net_admin .../run.py
    # in the setup.sh script

    # Remove any existing interfaces
    sudo[ip[['link', 'set', 'br0', 'down']]](retcode=(0, 1))
    sudo[brctl[['delbr', 'br0']]](retcode=(0, 1))
    for tap in NETWORK_CONFIG:
        sudo[ip[['link', 'set', '{}'.format(tap), 'down']]](retcode=(0, 1))
        sudo[ip[['link', 'del', '{}'.format(tap)]]](retcode=(0, 1))

    assert args.clients <= MAX_WORKERS, "Too many workers, can't configure network"
    sudo[ip[['link', 'add', 'br0', 'type', 'bridge']]]()
    sudo[ip[['addr', 'add', NETWORK_INFRA_IP, 'brd', '+', 'dev', 'br0']]]()
    for _, ncfg in zip(range(0, args.clients + 1), NETWORK_CONFIG):
        sudo[tunctl[['-t', ncfg, '-u', user, '-g', group]]]()
        sudo[ip[['link', 'set', ncfg, 'up']]](retcode=(0, 1))
        sudo[brctl[['addif', 'br0', ncfg]]]()
    sudo[ip[['link', 'set', 'br0', 'up']]](retcode=(0, 1))



def start_server(args):
    cmd = "sudo qemu-system-x86_64 /tmp/disk.img" \
        + " -enable-kvm -nographic" \
        + " -netdev tap,id=nd0,script=no,ifname=tap0" \
        + " -device e1000,netdev=nd0,mac=56:b4:44:e9:62:d0" \
        + " -m 1024 -smp " + str(args.scores) \
        + " -cpu host,migratable=no,+invtsc,+tsc,+x2apic,+fsgsbase"

    print("Invoking QEMU server with command: ", cmd)

    child = pexpect.spawn(cmd)

    # give guest time to boot
    child.expect("root@jammy:~# ", timeout=BOOT_TIMEOUT)

    # bring up ip address
    child.sendline("ip addr add 172.31.0.1/24 broadcast 172.31.0.255 dev ens3")
    child.expect("root@jammy:~# ")
    child.sendline("ip link set ens3 up")
    child.expect("root@jammy:~# ")

    child.sendline("./fxmark_grpc --mode emu_server --port 8080")
    child.expect("Starting server on port 8080")
    child.expect("root@jammy:~# ", timeout=EXP_TIMEOUT)

def start_client(cid, args):
    cmd = "sudo qemu-system-x86_64 /tmp/disk" + str(cid) + ".img" \
        + " -enable-kvm -nographic" \
        + " -netdev tap,id=nd0,script=no,ifname=tap" + str(cid*2) \
        + " -device e1000,netdev=nd0,mac=56:b4:44:e9:62:d" + str(cid) \
        + " -m 1024 -smp " + str(args.cores) \
        + " -cpu host,migratable=no,+invtsc,+tsc,+x2apic,+fsgsbase"

    print("Invoking QEMU client with command: ", cmd)

    child = pexpect.spawn(cmd)

    # give guest time to boot
    child.expect("root@jammy:~# ", timeout=BOOT_TIMEOUT)

    # bring up ip address
    child.sendline("ip addr add 172.31.0." + str(cid*2) + "/24 broadcast 172.31.0.255 dev ens3")
    child.expect("root@jammy:~# ")
    child.sendline("ip link set ens3 up")
    child.expect("root@jammy:~# ")
  
    wratios = ""
    for ratio in args.wratio:
        wratios += ratio + " "
    openfs = ""
    for f in args.openf:
        openfs += f + " "
 
    child.sendline("./fxmark_grpc --mode emu_client --wratio " + wratios + "--openf " + openfs + "--duration " + str(args.duration))
    child.expect_exact("thread_id,benchmark,ncores,write_ratio,open_files,duration_total,duration,operations")
    child.expect("root@jammy:~# ", timeout=EXP_TIMEOUT)

    output = child.before
    f = open(CSV_FILE, "a")
    f.write(output.decode().replace('\r', ''))
    f.close()

def qemu_run(args):
    s_pid = os.fork()
    if s_pid == 0:
        start_server(args)
    else:
        print("Spawning server with pid: " + str(s_pid))
        sleep(5)
        children = []
        for i in range(0, args.clients):
            c_pid = os.fork()
            if(c_pid == 0):
                start_client(i+1, args)
                sys.exit()
            else:
                print("Spawning child with pid: " + str(c_pid))
                children.append(c_pid)

        # wait for clients to finish
        n = len(children)
        while(n > 0):
            pid = os.wait()
            print("Child with pid " + str(pid) + " has finished.")
            n -= 1

        # terminate the server
        os.kill(s_pid, signal.SIGTERM)

def setup(args):
    # create image for server
    cmd = "qemu-img create -f qcow2 -b " + args.image + " -F qcow2 /tmp/disk.img"
    os.system(cmd)
    for i in range(0, args.clients):
        cmd = "qemu-img create -f qcow2 -b " + args.image + " -F qcow2 /tmp/disk" + str(i + 1) + ".img"
        os.system(cmd)

def cleanup():
    os.system("rm /tmp/disk*.img")

#
# Main routine of run.py
#
if __name__ == '__main__':
    "Execution pipeline for building and launching Fxmark gRPC"
    args = parser.parse_args()

    # Setup network
    if not ('no_network_setup' in args and args.no_network_setup):
        configure_network(args)

    if 'network_only' in args and args.network_only:
        sys.exit(0)

    # print(NETWORK_CONFIG)

    user = whoami().strip()
    kvm_members = getent['group', 'kvm']().strip().split(":")[-1].split(',')
    if not user in kvm_members and not args.norun:
        print("Your user ({}) is not in the kvm group.".format(user))
        print("Add yourself to the group with `sudo adduser {} kvm`".format(user))
        print("You'll likely have to restart for changes to take effect,")
        print("or run `sudo chmod +666 /dev/kvm` if you don't care about")
        print("kvm access restriction on the machine.")
        sys.exit(errno.EACCES)

    try:
        from plumbum.cmd import sudo
        r = sudo['-n']['true']()
    except ProcessExecutionError as e:
        if e.retcode == 1:
            print("`sudo` is asking for a password, but for testing to work, `sudo` should not prompt for a password.")
            print("Add the line `{} ALL=(ALL) NOPASSWD: ALL` with the `sudo visudo` command to fix this.".format(user))
            sys.exit(errno.EINVAL)
        else:
            raise e
    try:
        os.remove(CSV_FILE)
    except:
        pass
 
    f = open(CSV_FILE, "a")
    f.write("thread_id,benchmark,ncores,write_ratio,open_files,duration_total,duration,operations")
    f.close()

    setup(args)
    qemu_run(args)
    cleanup()
