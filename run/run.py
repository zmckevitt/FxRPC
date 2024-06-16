#!/usr/bin/python3

# Copyright © 2021 VMware, Inc. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

import argparse
import os
import sys
import signal
import pexpect
import errno
from time import sleep
from numa import info

from plumbum import colors, local, SshMachine
from plumbum.commands import ProcessExecutionError

from plumbum.cmd import whoami, python3, cat, getent, whoami

BOOT_TIMEOUT = 60
EXP_TIMEOUT = 10000000
CSV_FILE = "fxrpc_{}_{}_benchmark.csv" 
AFF_TIMEOUT = 120
HUGETLBFS_PATH = "/usr/lib/x86_64-linux-gnu/libhugetlbfs.so"

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

parser.add_argument("-t", "--transport", required=True, 
                    help="Specify transport method")
parser.add_argument("--rpc", required=True, 
                    help="Specify rpc library (grpc or drpc)")
parser.add_argument("-i", "--image", required=False, 
                    help="Specify disk image to use")
parser.add_argument("-s", "--scores", type=int, required=True, default=1, 
                    help="Cores for server")
parser.add_argument("-nc", "--clients", type=int, required=True, default=1, 
                    help="Setup n clients")
parser.add_argument("-c", "--ccores", type=int, required=True, default=1, 
                    help="Cores per client")
parser.add_argument("-w", "--wratio", nargs="+", required=True, 
                    help="Specify write ratio for mix benchmarks")
parser.add_argument("-o", "--openf", nargs="+", required=True, 
                    help="Specify number of open files for mix benchmarks")
parser.add_argument("-d", "--duration", type=int, required=True, default=10, 
                    help="Experiment duration")
parser.add_argument("-f", "--csv", type=str, required=False, default=None, 
                    help="CSV file")
parser.add_argument("-n", "--offset", type=int, required=False, default=0, 
                    help="Offset for numa host")
parser.add_argument("-m", "--memory", type=int, required=False, default=1024, 
                    help="Amount of memory to give to each instance")
parser.add_argument("--nonuma", required=False, default=False, action="store_true", 
                    help="Do not pin cores to numa node")
parser.add_argument("--numa", required=False, default=False, action="store_true", 
                    help="Never used. Required so rust runner can pass alternate flag to --nonuma")

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

def start_server_tcp(args, node, affinity):
    host_numa_nodes_list = query_host_numa()
    num_host_numa_nodes = len(host_numa_nodes_list)
    host_nodes = 0 if num_host_numa_nodes == 0 else \
        host_numa_nodes_list[(node+args.offset) % num_host_numa_nodes]
    cmd = "/usr/bin/env qemu-system-x86_64 /tmp/disk.img" \
        + " -enable-kvm -nographic" \
        + " -netdev tap,id=nd0,script=no,ifname=tap0" \
        + " -device e1000,netdev=nd0,mac=56:b4:44:e9:62:d0" \
        + " -cpu host,migratable=no,+invtsc,+tsc,+x2apic,+fsgsbase" \
        + " -name server,debug-threads=on" \
        + " -object memory-backend-memfd,id=nmem0,merge=off,dump=on,prealloc=off,size=" \
            + str(args.memory) + "M" \
        + ",host-nodes=" + str(host_nodes) \
        + ",policy=bind,hugetlb=on,hugetlbsize=2M,share=on" \
        + " -numa node,memdev=nmem0,nodeid=0" \
        + " -numa cpu,node-id=0,socket-id=0" \
        + " -smp " + str(args.scores) + ",sockets=1,maxcpus=" + str(args.scores) + \
            " -m " + str(args.memory) + "M"
        # + " -m 1024 -smp " + str(args.scores) \

    print("Invoking QEMU server with command: ", cmd)

    child = pexpect.spawn(cmd)
   
    timeout = 0 
    while True:

        if(timeout > AFF_TIMEOUT):
            print("Server affinity timeout!")
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
    cmd = "ip addr add 172.31.0.1/24 broadcast 172.31.0.255 dev ens3"
    print("Setup IP in server emulated environment: " + cmd)
    child.sendline(cmd)
    child.expect("root@jammy:~# ")
    cmd = "ip link set ens3 up"
    print("Setup IP in server emulated environment: " + cmd)
    child.sendline(cmd)
    child.expect("root@jammy:~# ")

    cmd = "./fxrpc --mode server --transport tcpremote --rpc " + args.rpc + " --port 8080"
    print("Invoking TCP server in emulated environment with command: ", cmd)
    child.sendline(cmd)
    child.expect("Starting server on port 8080")
    child.expect("root@jammy:~# ", timeout=EXP_TIMEOUT)

def start_client_tcp(cid, args, node, affinity):
    host_numa_nodes_list = query_host_numa()
    num_host_numa_nodes = len(host_numa_nodes_list)
    host_nodes = 0 if num_host_numa_nodes == 0 else \
        host_numa_nodes_list[(node+args.offset) % num_host_numa_nodes]
    cmd = "/usr/bin/env qemu-system-x86_64 /tmp/disk" + str(cid) + ".img" \
        + " -enable-kvm -nographic" \
        + " -netdev tap,id=nd0,script=no,ifname=tap" + str(cid*2) \
        + " -device e1000,netdev=nd0,mac=56:b4:44:e9:62:d" + str(cid) \
        + " -cpu host,migratable=no,+invtsc,+tsc,+x2apic,+fsgsbase" \
        + " -name client" + str(cid) + ",debug-threads=on" \
        + " -smp " + str(args.ccores) + ",sockets=1,maxcpus=" + str(args.ccores) \
            + " -m " + str(args.memory) + "M" \
        + " -object memory-backend-memfd,id=nmem0,merge=off,dump=on,prealloc=off,size=" \
              + str(args.memory) + "M" \
        + ",host-nodes=" + str(host_nodes) \
        + ",policy=bind,hugetlb=on,hugetlbsize=2M,share=on" \
        + " -numa node,memdev=nmem0,nodeid=0" \
        + " -numa cpu,node-id=0,socket-id=0"

    print("Invoking QEMU client with command: ", cmd)

    child = pexpect.spawn(cmd)

    timeout = 0
    while True:
        
        if(timeout > AFF_TIMEOUT):
            print("Client affinity timeout!")
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
    cmd = "ip addr add 172.31.0." + str(cid*2) + "/24 broadcast 172.31.0.255 dev ens3"
    print("Setting up network on client: " + cmd)
    child.sendline(cmd)
    child.expect("root@jammy:~# ")
    cmd = "ip link set ens3 up"
    print("Setting up network on client: " + cmd)
    child.sendline(cmd)
    child.expect("root@jammy:~# ")
  
    wratios = ""
    for ratio in args.wratio:
        wratios += ratio + " "
    openfs = ""
    for f in args.openf:
        openfs += f + " "
 
    cmd = "./fxrpc --mode client --transport tcpremote --rpc " + args.rpc + " --wratio " + wratios + "--openf " + openfs + \
        "--duration " + str(args.duration) + " --cid " + str(cid-1) + \
        " --nclients " + str(args.clients) + " --ccores " + str(args.ccores)
    print("Invoking TCP client in emulated environment with command: " + cmd)
    child.sendline(cmd)
    child.expect_exact("thread_id,benchmark,ncores,write_ratio,open_files,duration_total," \
                       "duration,operations,client_id,client_cores,nclients,rpctype")
    child.expect("root@jammy:~# ", timeout=EXP_TIMEOUT)

    output = child.before.decode().replace('\r', '')
    if(output[0] == '\n'):
        output = output[1:]
    f = open(args.csv, "a")
    f.write(output)
    f.close()

def start_server_uds(args):
    cmd = "../prog/target/release/fxrpc --mode server --transport uds --rpc " + args.rpc 
    if(not args.nonuma):
        cmd = "numactl --membind=0 --cpunodebind=0 " + cmd
        print("Invoking UDS server with command: ", cmd)
        child = pexpect.run(cmd, timeout=EXP_TIMEOUT, env =
                          {'LD_PRELOAD': HUGETLBFS_PATH, 
                           'HUGETLB_MORECORE': 'yes'})
    else:
        print("Invoking UDS server with command: ", cmd)
        child = pexpect.run(cmd, timeout=EXP_TIMEOUT)


def start_client_uds(cid, args):
    wratios = ""
    for ratio in args.wratio:
        wratios += ratio + " "
    openfs = ""
    for f in args.openf:
        openfs += f + " "
    cmd = "../prog/target/release/fxrpc --mode client --transport uds --rpc " + args.rpc + " --wratio " + wratios + \
        "--openf " + openfs + "--duration " + str(args.duration) + " --cid " + str(cid-1) + \
        " --nclients " + str(args.clients) + " --ccores " + str(args.ccores)
    if(not args.nonuma):
        cmd = "numactl --membind=" + str(cid) + " --cpunodebind=" + str(cid) + " " + cmd
        print("Invoking UDS client with command: ", cmd)
        child = pexpect.run(cmd, timeout=EXP_TIMEOUT, env =
                          {'LD_PRELOAD': HUGETLBFS_PATH, 
                           'HUGETLB_MORECORE': 'yes'})
    else:
        print("Invoking UDS client with command: ", cmd)
        child = pexpect.run(cmd, timeout=EXP_TIMEOUT)

    output = child
    f = open(args.csv, "a")
    f.write(output.decode().replace('\r', ''))
    f.close()

def qemu_run(args, affinity, nodes):
    s_pid = os.fork()
    if s_pid == 0:
        if(args.transport == "tcp"):
            start_server_tcp(args, 0, affinity[0])
        if(args.transport == "uds"):
            start_server_uds(args)
    else:
        print("Spawning server with pid: " + str(s_pid))
        sleep(5)
        children = []
        for i in range(0, args.clients):
            c_pid = os.fork()
            if(c_pid == 0):
                if(args.transport == "tcp"):
                    start_client_tcp(i+1, args, nodes[i+1], affinity[i+1])
                    sys.exit()
                if(args.transport == "uds"):
                    start_client_uds(i+1, args)
                    sys.exit() 
            else:
                print("Spawning child with pid: " + str(c_pid))
                children.append(c_pid)

        # wait for clients to finish
        n = len(children)
        while(n > 0):
            pid = os.wait()
            if(pid == s_pid):
                print("Unexpected server failure! Exiting...")
                sys.exit(1)
            print("Child with pid " + str(pid) + " has finished.")
            n -= 1

        # terminate the server
        os.kill(s_pid, signal.SIGTERM)

def setup(args):
    abs_path = os.path.abspath(args.image)
    # create image for server
    cmd = "qemu-img create -f qcow2 -b " + abs_path + " -F qcow2 /tmp/disk.img"
    print("Configuring qemu image with: " + cmd)
    os.system(cmd)
    for i in range(0, args.clients):
        cmd = "qemu-img create -f qcow2 -b " + abs_path + " -F qcow2 /tmp/disk" + str(i + 1) + ".img"
        print("Configuring qemu image with: " + cmd)
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
    print("Invoking run.py with command: " + " ".join(sys.argv))

    if args.csv is None:
        args.csv = CSV_FILE.format(args.transport, args.rpc)

    # print(NETWORK_CONFIG)
    if args.transport == "tcp":
        # Setup network
        if not ('no_network_setup' in args and args.no_network_setup):
            configure_network(args)

        if 'network_only' in args and args.network_only:
            sys.exit(0)

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
                print("`sudo` is asking for a password, but for testing to work, `sudo` " \
                      "should not prompt for a password.")
                print("Add the line `{} ALL=(ALL) NOPASSWD: ALL` with the `sudo visudo` " \
                      "command to fix this.".format(user))
                sys.exit(errno.EINVAL)
            else:
                raise e
        affinity,nodes = get_numa_mapping(args)
        print("Detected affinity: ", affinity)
        setup(args)
        qemu_run(args, affinity, nodes)
        cleanup()
    if args.transport == "uds":
        if not args.nonuma:
            if not os.path.isfile(HUGETLBFS_PATH):
                print("ERROR: " + HUGETLBFS_PATH + " is not present. " \
                        "Please change path or install hugetlbfs")
                sys.exit(errno.EINVAL)

        qemu_run(args, [], [])
