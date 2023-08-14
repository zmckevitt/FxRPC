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
from numa import info

from plumbum import colors, local, SshMachine
from plumbum.commands import ProcessExecutionError

from plumbum.cmd import whoami, python3, cat, getent, whoami

BOOT_TIMEOUT = 60
EXP_TIMEOUT = 10000000
CSV_FILE = "fxmark_grpc_benchmark.csv" 
AFF_TIMEOUT = 120

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

parser.add_argument("-i", "--image", required=True, help="Specify disk image to use")
parser.add_argument("-s", "--scores", type=int, required=True, default=1, help="Cores for server")
parser.add_argument("-nc", "--clients", type=int, required=True, default=1, help="Setup n clients")
parser.add_argument("-c", "--ccores", type=int, required=True, default=1, help="Cores per client")
parser.add_argument("-w", "--wratio", nargs="+", required=True, help="Specify write ratio for mix benchmarks")
parser.add_argument("-o", "--openf", nargs="+", required=True, help="Specify number of open files for mix benchmarks")
parser.add_argument("-d", "--duration", type=int, required=True, default=10, help="Experiment duration")
parser.add_argument("-f", "--csv", type=str, required=False, default="fxmark_grpc_benchmarks.csv", help="CSV file")
parser.add_argument("-n", "--offset", type=int, required=False, default=0, help="Offset for numa host")
parser.add_argument("-m", "--memory", type=int, required=False, default=1024, help="Amount of memory to give to each instance")

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

def numa_nodes_to_list(file):
        nodes = []
        good_nodes = cat[file]().split(',')
        for node_range in good_nodes:
            if "-" in node_range:
                nlow, nmax = node_range.split('-')
                for i in range(int(nlow), int(nmax)+1):
                    nodes.append(i)
            else:
                nodes.append(int(node_range.strip()))
        return nodes

def query_host_numa():
    mem_nodes = numa_nodes_to_list(
        "/sys/devices/system/node/has_memory")
    cpu_nodes = numa_nodes_to_list("/sys/devices/system/node/has_cpu")

    # Now return the intersection of the two
    return list(sorted(set(mem_nodes).intersection(set(cpu_nodes))))

def start_server(args, node, affinity):
    host_numa_nodes_list = query_host_numa()
    num_host_numa_nodes = len(host_numa_nodes_list)
    host_nodes = 0 if num_host_numa_nodes == 0 else host_numa_nodes_list[(node+args.offset) % num_host_numa_nodes]
    cmd = "/usr/bin/env qemu-system-x86_64 /tmp/disk.img" \
        + " -enable-kvm -nographic" \
        + " -netdev tap,id=nd0,script=no,ifname=tap0" \
        + " -device e1000,netdev=nd0,mac=56:b4:44:e9:62:d0" \
        + " -cpu host,migratable=no,+invtsc,+tsc,+x2apic,+fsgsbase" \
        + " -name server,debug-threads=on" \
        + " -object memory-backend-memfd,id=nmem0,merge=off,dump=on,prealloc=off,size=" + str(args.memory) + "M" \
        + ",host-nodes=" + str(host_nodes) \
        + ",policy=bind,hugetlb=on,hugetlbsize=2M,share=on" \
        + " -numa node,memdev=nmem0,nodeid=0" \
        + " -numa cpu,node-id=0,socket-id=0" \
        + " -smp " + str(args.scores) + ",sockets=1,maxcpus=" + str(args.scores) + " -m " + str(args.memory) + "M"
        # + " -m 1024 -smp " + str(args.scores) \

    print("Invoking QEMU server with command: ", cmd)

    child = pexpect.spawn(cmd)
   
    timeout = 0 
    while True:

        if(timeout > AFF_TIMEOUT):
            print("Affinity timeout!")
            sys.exit()

        try:
            sudo[python3['./qemu_affinity.py', 
                         '-k', affinity, '--', str(child.pid)]]()
            break
        except:
            sleep(2)
            timeout += 2

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

def start_client(cid, args, node, affinity):
    host_numa_nodes_list = query_host_numa()
    num_host_numa_nodes = len(host_numa_nodes_list)
    host_nodes = 0 if num_host_numa_nodes == 0 else host_numa_nodes_list[(node+args.offset) % num_host_numa_nodes]
    cmd = "/usr/bin/env qemu-system-x86_64 /tmp/disk" + str(cid) + ".img" \
        + " -enable-kvm -nographic" \
        + " -netdev tap,id=nd0,script=no,ifname=tap" + str(cid*2) \
        + " -device e1000,netdev=nd0,mac=56:b4:44:e9:62:d" + str(cid) \
        + " -cpu host,migratable=no,+invtsc,+tsc,+x2apic,+fsgsbase" \
        + " -name client" + str(cid) + ",debug-threads=on" \
        + " -smp " + str(args.ccores) + ",sockets=1,maxcpus=" + str(args.ccores) + " -m " + str(args.memory) + "M" \
        + " -object memory-backend-memfd,id=nmem0,merge=off,dump=on,prealloc=off,size=" + str(args.memory) + "M" \
        + ",host-nodes=" + str(host_nodes) \
        + ",policy=bind,hugetlb=on,hugetlbsize=2M,share=on" \
        + " -numa node,memdev=nmem0,nodeid=0" \
        + " -numa cpu,node-id=0,socket-id=0"

    print("Invoking QEMU client with command: ", cmd)

    child = pexpect.spawn(cmd)

    timeout = 0
    while True:
        
        if(timeout > AFF_TIMEOUT):
            print("Affinity timeout!")
            sys.exit()

        try:
            sudo[python3['./qemu_affinity.py', 
                         '-k', affinity, '--', str(child.pid)]]()
            break
        except:
            sleep(2)
            timeout += 2

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
 
    child.sendline("./fxmark_grpc --mode emu_client --wratio " + wratios + "--openf " + openfs + "--duration " + str(args.duration) + " --cid " + str(cid-1) + " --nclients " + str(args.clients) + " --ccores " + str(args.ccores))
    child.expect_exact("thread_id,benchmark,ncores,write_ratio,open_files,duration_total,duration,operations,client_id,client_cores,nclients")
    child.expect("root@jammy:~# ", timeout=EXP_TIMEOUT)

    output = child.before
    f = open(args.csv, "a")
    f.write(output.decode().replace('\r', ''))
    f.close()

def qemu_run(args, affinity, nodes):
    s_pid = os.fork()
    if s_pid == 0:
        start_server(args, 0, affinity[0])
    else:
        print("Spawning server with pid: " + str(s_pid))
        sleep(5)
        children = []
        for i in range(0, args.clients):
            c_pid = os.fork()
            if(c_pid == 0):
                start_client(i+1, args, nodes[i+1], affinity[i+1])
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
    abs_path = os.path.abspath(args.image)
    # create image for server
    cmd = "qemu-img create -f qcow2 -b " + abs_path + " -F qcow2 /tmp/disk.img"
    os.system(cmd)
    for i in range(0, args.clients):
        cmd = "qemu-img create -f qcow2 -b " + abs_path + " -F qcow2 /tmp/disk" + str(i + 1) + ".img"
        os.system(cmd)

def cleanup():
    os.system("rm /tmp/disk*.img")

def get_numa_mapping(args):
    numa = info.numa_hardware_info()['node_cpu_info']
   
    # Ensure we can map cores to clients 
    tot_cores = 0
    for node in numa:
        tot_cores += len(numa[node])
    print("Total cores available: " + str(tot_cores))
    
    requested_cores = args.scores + (args.clients * args.ccores)

    assert tot_cores >= requested_cores, "Requesting more cores than available!"

    # initialize mapping
    mapping = {}
    for i in range(args.clients + 1):
        mapping[i] = []

    # allocate cores for server on first node
    for i in range(args.scores):
        try:
            mapping[0].append(numa[0][0])
            del numa[0][0] 
        except:
            print("Unable to allocate cores for server!")
            sys.exit(1)

    # Determine which process is on which node
    nodes = {}
    nodes[0] = 0

    node = 1 % len(numa)
    client = 1
    while client < args.clients+1:
        try:
            # If current node has enough room for client, allocate it there
            if(args.ccores <= len(numa[node])):
                mapping[client] = numa[node][0:args.ccores]
                del numa[node][0:args.ccores]
                nodes[client] = node
                client += 1
                node = node + 1 % len(numa)
            else:
                node = node + 1 % len(numa)
        except:
            print("Cannot pin client topology to host!")
            sys.exit(1)

    return (mapping,nodes)

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
    affinity,nodes = get_numa_mapping(args)
    print("Detected affinity: ", affinity)
    setup(args)
    qemu_run(args, affinity, nodes)
    cleanup()