#!/usr/bin/python3

# Copyright © 2021 VMware, Inc. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR MIT

import argparse
import os
import sys
import signal
import shutil
import pexpect
import errno
import re
from time import sleep
from numa import info
from pathlib import Path

from plumbum import colors, local, SshMachine
from plumbum.commands import ProcessExecutionError

from plumbum.cmd import whoami, python3, cat, getent, whoami

BOOT_TIMEOUT = 180
EXP_TIMEOUT = 10000000
CSV_FILE = "fxmark_grpc_{}_benchmark.csv"
AFF_TIMEOUT = 120
HUGETLBFS_PATH = "/usr/lib/x86_64-linux-gnu/libhugetlbfs.so"

CSV_ROWS="benchmark,os,nthreads,servers,protocol,mem,queries,time,thpt"

# the version of the ubuntu distro to take
UBUNTU_VERSION="jammy"
HOSTNAME=UBUNTU_VERSION

def get_network_config(workers):
    """
    Returns a list of network configurations for the workers.
    """
    config = [{
        'tap': f'tap{2*i}',
        'mid': i,
        'mac': '56:b4:44:e9:62:d{:x}'.format(i),
        'ip' : f"172.31.0.1{i}"
    } for i in range(workers)]
    return config

MAX_WORKERS = 8
NETWORK_CONFIG = get_network_config(MAX_WORKERS)
NETWORK_INFRA_IP = '172.31.0.20'

#
# Command line argument parser
#
parser = argparse.ArgumentParser()

parser.add_argument("-c", "--cores", type=int, required=False, default=1,
                    help="Cores per memcached instance")
parser.add_argument("-i", "--image", required=False,
                    help="Specify disk image to use")
parser.add_argument("-q", "--queries", type=int, required=False, default=1,
                    help="Number of queries to execute")
parser.add_argument("-s", "--servers", type=int, required=True, default=1,
                    help="Number of memcached instances")
parser.add_argument("-n", "--offset", type=int, required=False, default=0,
                    help="Offset for numa host")
parser.add_argument("-m", "--memory", type=int, required=False, default=1024,
                    help="Amount of memory to give to each instance")
parser.add_argument("-l", "--loadbalancer", type=str, required=False, default="./loadbalancer",
                    help="The load balancer binary to use")
parser.add_argument("-k", "--kvstore", type=str, required=False, default="./memcached",
                    help="The memcached binary to use")
parser.add_argument("--nonuma", required=False, default=False, action="store_true",
                    help="Do not pin cores to numa node")
parser.add_argument("--numa", required=False, default=False, action="store_true",
                    help="Never used. Required so rust runner can pass alternate flag to --nonuma")
parser.add_argument("-o", "--out", type=str, required=False, default="./memcached_benchmark_sharded_linux.csv",
                    help="the output CSV file to be used")



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
    qemu.expect(f"root@{HOSTNAME}")

####################################################################################################
# Host Network Configuration
####################################################################################################


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
    for cfg in NETWORK_CONFIG:
        tap = cfg['tap']
        sudo[ip[['link', 'set', '{}'.format(tap), 'down']]](retcode=(0, 1))
        sudo[ip[['link', 'del', '{}'.format(tap)]]](retcode=(0, 1))

    assert args.servers <= MAX_WORKERS, "Too many workers, can't configure network"
    sudo[ip[['link', 'add', 'br0', 'type', 'bridge']]]()
    sudo[ip[['addr', 'add', f"{NETWORK_INFRA_IP}/24", 'brd', '+', 'dev', 'br0']]]()

    for idx in range(args.servers) :
        tap = NETWORK_CONFIG[idx]['tap']
        sudo[tunctl[['-t', tap, '-u', user, '-g', group]]]()
        sudo[ip[['link', 'set', tap, 'up']]](retcode=(0, 1))
        sudo[brctl[['addif', 'br0', tap]]]()

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

def start_memcached(args, node, affinity, disk_image):
    print(f" > starting memcached {node}")
    host_numa_nodes_list = query_host_numa()
    num_host_numa_nodes = len(host_numa_nodes_list)
    host_nodes = 0 if num_host_numa_nodes == 0 else \
        host_numa_nodes_list[(node+args.offset) % num_host_numa_nodes]

    # name of the server
    name=f"memcached{node}"

    # create the new disk image for the memcached node
    my_disk_image = Path(f"qemu_disk_image_{node}.img")

    mac = NETWORK_CONFIG[node]['mac']
    tap = NETWORK_CONFIG[node]['tap']
    ip = NETWORK_CONFIG[node]['ip']
    netif = "ens3"
    netif = "enp0s2"

    qemu_memory_size = 2*args.memory + 4096
    qemu_cmd = [
        "/usr/bin/env" , "qemu-system-x86_64",
        "-name", f"{name},debug-threads=on",
        "-enable-kvm", "-nographic",
        "-machine", "q35",
        # NUMA configuration
        "-numa node,memdev=nmem0,nodeid=0",
        # CPU configuration
        "-cpu", "host,migratable=no,+invtsc,+tsc,+x2apic,+fsgsbase",
        "-smp", f"{args.cores},sockets=1,maxcpus={args.cores}",
        "-numa", "cpu,node-id=0,socket-id=0",
        # memory
        "-m", f"{qemu_memory_size}M",
        "-object", f"memory-backend-memfd,id=nmem0,merge=off,dump=on,prealloc=off,size={qemu_memory_size}M,host-nodes={host_nodes},policy=bind,hugetlb=on,hugetlbsize=2M,share=on",
        # "-object", f"memory-backend-memfd,id=nmem0,merge=off,dump=on,prealloc=off,size={qemu_memory_size}M,host-nodes={host_nodes},policy=bind,share=on",
        # networking
        "-device", f"virtio-net,netdev=nd0,mac={mac}",
        "-netdev", f"tap,id=nd0,script=no,ifname={tap}",
        "-drive", f"file={str(my_disk_image)},if=virtio"
    ]

    qemu_cmd_str = " ".join(qemu_cmd)
    print("   + Invoking QEMU server with command: ", qemu_cmd_str)
    child = pexpect.spawn(qemu_cmd_str)
    child.logfile = open(f"qemulog_{node}.txt",'wb')
    try :
        child.expect("Booting from Hard Disk...", timeout=10)
    except pexpect.exceptions.EOF as e:
        print("before" + child.before.decode("utf-8"))
        print("after:" + child.after.decode("utf-8"))
        print(e)
        raise e


    print("   + setting affinity...")
    timeout = 0
    while True:
        if(timeout > AFF_TIMEOUT):
            print("Affinity timeout!")
            sys.exit()
        try:
            c = sudo[python3['./qemu_affinity.py',
                         '-k', affinity, '--', str(child.pid)]]()
            break
        except Exception as e:
            print(e)
            sleep(2)
            timeout += 2

    # while True:
    #     l = child.readline().decode('utf8')
    #     print(f"> {l}")
    #     if l.startswith("jammy login:"):
    #         break


    # give guest time to boot
    child.expect("root@jammy", timeout=BOOT_TIMEOUT)

    print("   + configuring IP...")

    do_cmd(child, f"ip address show")

    # bring up ip address
    do_cmd(child, f"ip address flush dev {netif}")
    do_cmd(child, f"ip route flush dev {netif}")
    do_cmd(child, f"ip address add {ip}/24 brd + dev {netif}")
    do_cmd(child, f"ip link set {netif} up")
    do_cmd(child, f"ip route add {NETWORK_INFRA_IP} dev  {netif}")
    do_cmd(child, f"ip route add default via {NETWORK_INFRA_IP} dev {netif}")
    do_cmd(child, f"ip address show dev {netif}")
    do_cmd(child, f"ip address show")

    # start memcached

    do_cmd(child, f"chmod +x memcached")

    cmd = f"./memcached --x-benchmark-no-run --disable-evictions --conn-limit=1024 --threads={args.cores} --x-benchmark-mem={2*args.memory} --memory-limit={2* args.memory+2048}"
    print(f"   + {cmd}")
    child.sendline(cmd)
    try:
        child.expect("INTERNAL BENCHMARK CONFIGURE")
        child.expect("INTERNAL BENCHMARK SKIPPING")
    except Exception as e:
        print(child.before)
        print(child.after)
        raise e
    print(f"   + memcached ready")

    return child

def spawn_load_balancer(args):
    servers = ",".join([f"tcp://{NETWORK_CONFIG[i]['ip']}:11211" for i in range(args.servers)])
    cmd = [
        args.loadbalancer,
        "--binary",
        f"--num-queries={args.queries}",
        f"--num-threads={args.cores}",
        f"--max-memory={args.memory}",
        f"--servers={servers}"
    ]
    cmd = " ".join(cmd)
    print(f" > spawning load balancer with `{cmd}`")
    return pexpect.spawn(cmd)

def qemu_run(args, affinity, nodes):
    log("Runing experiments: starting servers")
    servers = []
    for i in range(0, args.servers):
        server = start_memcached(args, nodes[i], affinity[i], args.image)
        servers.append(server)

    sleep(5)
    log("Runing experiments: starting load balancer")
    # here we need to run the memcached benchmark
    lb = spawn_load_balancer(args)

    counter = 0
    for i in range(0, args.cores):
        lb.expect("thread.(\d+) populating database with", timeout=30)
        counter = counter + 1
        print(f"   + thread {counter} of {args.cores} populating...")

    print(" > threads spawned, populating database")

    counter = 0
    ready = 0
    while ready != args.cores:
        expected = [
            "thread.(\d+) added (\d+) keys to (\d+) servers",
            "thread.(\d+) ready for benchmark"
        ]
        idx = lb.expect(expected, timeout=max(args.memory / 10, 30))
        if idx == 0:
            if counter <= 10:
                print(f"   + population progress: {(counter / 10) * 100}% populating...")
            counter = counter + 1
        elif idx == 1:
            ready = ready + 1
            print(f"   + thread {ready} of {args.cores} ready for benchmark...")

    print(" > waiting for threads to finish...")
    for i in range(0, args.cores):
        lb.expect("thread.(\d+) executed (\d+) queries.", timeout=max(args.queries / 1000, 60))
        print(f"   + thread {i+1} of {args.cores} done with queries...")

    print(" > benchmark done...")

    # > benchmark took 6 ms
    lb.expect("benchmark took (\d+) ms", timeout=30)
    line = lb.after.decode("utf-8")
    time = line.replace("benchmark took ", "").replace(" ms", "")

    # > benchmark took 16000 queries / second
    lb.expect("benchmark took (\d+) queries / second", timeout=30)
    line = lb.after.decode("utf-8")
    thpt = line.replace("benchmark took ", "").replace(" queries / second", "")

    # > benchmark executed 100 / 100 queries
    lb.expect("benchmark executed (\d+) / (\d+) queries", timeout=30)
    line = lb.after.decode("utf-8")
    queries = line.replace("benchmark executed ", "").replace(" queries", "").split(" / ")
    executed = queries[0]
    expected = queries[1]

    fail = "ok" if executed == expected else "not all queries executed"

    print(f"   + {thpt} queries per second -- {fail}")

    # lb.expect("benchmark took", timeout=-1)
    print("Terminating memcached instances...")
    # terminate the servers
    for s in servers :
        s.kill(signal.SIGKILL)
        s.wait()


    csv = Path(args.out)
    if not csv.exists():
        csv = open(csv, "w")
        csv.write(f"{CSV_ROWS}\n")
    else :
        csv = open(csv, "a")

    # benchmark,os,nthreads,protocol,mem,queries,time,thpt
    csv.write(f"memcached_sharded,linuxvm,{args.cores},{args.servers},tcp,{args.memory},{executed},{time},{thpt},{fail}\n")

    print("Done")

def setup(args):
    log("Setting up run")
    qemuimg = local["qemu-img"]

    abs_path = os.path.abspath(args.image)

    print (" > creating new template image")
    # create a copy of the disk image, using CoW
    disk_image_template = Path(os.path.abspath("my_disk_image_base.img"))
    disk_image_template.unlink()
    qemuimg("create", "-f", "qcow2", "-b", abs_path, "-F", "qcow2",  disk_image_template)


    print (" > installing memcached in the image")

    # mount it!
    mp = Path("disk-mount")
    if not mp.exists():
        mp.mkdir(parents=True, exist_ok=True)

    pidfile = Path("guestmount.pid")
    if pidfile.exists():
        pidfile.unlink()

    guestmount = local["guestmount"]
    guestmount("-a", str(disk_image_template), "--pid-file", "guestmount.pid", "--rw", "-i", str(mp))

    with open(pidfile, "r") as f:
        pid = f.readline()
    pid = int(pid.replace("\n", ''))

    memcached = Path(args.kvstore)
    dest = Path(mp / "root" / "memcached")
    shutil.copyfile(memcached, dest)

    guestunmount = local["guestunmount"]
    guestunmount(str(mp))

    os.kill(pid, 0)
    try:
        os.waitpid(pid, 0)
    except:
        pass

    sleep(5)

    print (" > creating the images for the memcached severs")
    for i in range(0, args.servers):
        my_disk_image = Path(f"qemu_disk_image_{i}.img")
        if my_disk_image.exists():
            my_disk_image.unlink()
        qemuimg("create", "-f", "qcow2", "-b", disk_image_template, "-F", "qcow2",  str(my_disk_image))

def cleanup():
    for i in range(0, args.servers):
        my_disk_image = Path(f"qemu_disk_image_{i}.img")
        my_disk_image.unlink()

def get_numa_mapping(args):
    numa = info.numa_hardware_info()['node_cpu_info']

    # Ensure we can map cores to clients
    tot_cores = 0
    for node in numa:
        tot_cores += len(numa[node])
    print("Total cores available: " + str(tot_cores))

    requested_cores = args.servers * args.cores

    assert tot_cores >= requested_cores, "Requesting more cores than available!"

    # initialize mapping
    mapping = {}
    for i in range(args.servers + 1):
        mapping[i] = []

    # allocate cores for server on first node
    for i in range(args.cores):
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
    while client < args.servers+1:
        try:
            # If current node has enough room for client, allocate it there
            if(args.cores <= len(numa[node])):
                mapping[client] = numa[node][0:args.cores]
                del numa[node][0:args.cores]
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
    "Execution pipeline for building and launching Memcached Linux VM"
    args = parser.parse_args()
    print("Invoking run.py with command: " + " ".join(sys.argv))

    # if args.csv is None:
    #     args.csv = CSV_FILE.format(args.transport)

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
