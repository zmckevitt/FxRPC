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

from plumbum.cmd import whoami, python3, cat, getent, whoami, cargo

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
# General build arguments
parser.add_argument("-v", "--verbose", action="store_true",
                    help="increase output verbosity")

parser.add_argument("-r", "--release", action="store_true",
                    help="Do a release build.")
parser.add_argument("--kfeatures", type=str, nargs='+', default=[],
                    help="Cargo features to enable (in the kernel).")
parser.add_argument("--no-kfeatures", action="store_true", default=False,
                    help="Disable default Cargo features (in the kernel).", required=False)
parser.add_argument("--ufeatures", type=str, nargs='+', default=[],
                    help="Cargo features to enable (in user-space, use module_name:feature_name syntax to specify module specific features, e.g. init:print-test).")
parser.add_argument('-m', '--mods', nargs='+', default=['init'],
                    help='User-space modules to be included in build & deployment', required=False)
parser.add_argument("--cmd", type=str,
                    help="Command line arguments passed to the kernel.")
parser.add_argument("--machine",
                    help='Which machine to run on (defaults to qemu)', required=False, default='qemu')

parser_tasks_mut = parser.add_mutually_exclusive_group(required=False)
parser_tasks_mut.add_argument("-n", "--norun", action="store_true", default=False,
                    help="Only build, don't run")
parser_tasks_mut.add_argument("-b", "--nobuild", action="store_true", default=False,
                    help="Only run, don't build")


# DCM Scheduler arguments
parser.add_argument("--dcm-path",
                    help='Path of DCM jar to use (defaults to latest release)', required=False, default=None)

# QEMU related arguments
parser.add_argument("--qemu-nodes", type=int,
                    help="How many NUMA nodes and sockets (for qemu).", required=False, default=None)
parser.add_argument("--qemu-cores", type=int,
                    help="How many cores (will get evenly divided among nodes).", default=1)
parser.add_argument("--qemu-memory", type=int,
                    help="How much total memory in MiB (will get evenly divided among nodes).", default=1024)
parser.add_argument("--qemu-pmem", type=int,
                    help="How much total peristent memory in MiB (will get evenly divided among nodes).", required=False, default=0)
parser.add_argument("--qemu-affinity", type=str,
                    help="Pin QEMU instance to dedicated host cores.", required=False, default=None)
parser.add_argument("--qemu-prealloc", action="store_true", default=False,
                    help="Pre-alloc memory for the guest", required=False)
parser.add_argument("--qemu-large-pages", action="store_true", default=False,
                    help="Use large-pages on the host for guest memory", required=False)
parser.add_argument("--qemu-settings", type=str,
                    help="Pass additional generic QEMU arguments.")
parser.add_argument("--qemu-monitor", action="store_true",
                    help="Launch the QEMU monitor (for qemu)")
parser.add_argument("--pvrdma", action="store_true",
                    help="Add para-virtual RDMA device (for qemu)", default=False)
parser.add_argument("-d", "--qemu-debug-cpu", action="store_true",
                    help="Debug CPU reset (for qemu)")
parser.add_argument('--nic', default='e1000', choices=["e1000", "virtio-net-pci", "vmxnet3"],
                    help='What NIC model to use for emulation', required=False)
parser.add_argument('--tap', default='tap0', choices=[f"tap{2*i}" for i in range(MAX_WORKERS)],
                    help='Which tap interface to use from the host', required=False)
parser.add_argument('--kgdb', action="store_true",
                    help="Use the GDB remote debugger to connect to the kernel")
parser.add_argument('--qemu-ivshmem',
                    type=str,
                    help="Enable the ivshmem device with the size in MiB.",
                    required=False,
                    default="")
parser.add_argument('--qemu-shmem-path',
                    type=str,
                    help="Provide shared memory file path.",
                    required=False,
                    default="")

# Baremetal argument
parser.add_argument('--configure-ipxe', action="store_true", default=False,
                    help='Execute pre-boot setup for bare-metal booting.', required=False)
parser.add_argument('--no-reboot', action="store_true", default=False,
                    help='Do not initiate a machine reboot.', required=False)

subparser = parser.add_subparsers(help='Advanced network configuration')

# Network setup parser
parser_net = subparser.add_parser('net', help='Network setup')
parser_net.add_argument('--workers', type=int, required=False, default=2,
                        help='Setup `n` workers connected to one controller.')

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

    # Need to find out how to set default=True in case workers are >0 in `args`
    if (not 'workers' in args) or ('workers' in args and args.workers <= 1):
        sudo[tunctl[['-t', args.tap, '-u', user, '-g', group]]]()
        sudo[ifconfig[args.tap, NETWORK_INFRA_IP]]()
        sudo[ip[['link', 'set', args.tap, 'up']]](retcode=(0, 1))
    else:
        assert args.workers <= MAX_WORKERS, "Too many workers, can't configure network"
        sudo[ip[['link', 'add', 'br0', 'type', 'bridge']]]()
        sudo[ip[['addr', 'add', NETWORK_INFRA_IP, 'brd', '+', 'dev', 'br0']]]()
        for _, ncfg in zip(range(0, args.workers), NETWORK_CONFIG):
            sudo[tunctl[['-t', ncfg, '-u', user, '-g', group]]]()
            sudo[ip[['link', 'set', ncfg, 'up']]](retcode=(0, 1))
            sudo[brctl[['addif', 'br0', ncfg]]]()
        sudo[ip[['link', 'set', 'br0', 'up']]](retcode=(0, 1))


BOOT_TIMEOUT = 60
IMG_FILE = "focal-server-cloudimg-amd64.img"
def qemu_run():
    cmd = "sudo qemu-system-x86_64 /users/zackm/focal-server-cloudimg-amd64.img -enable-kvm -nographic -netdev tap,id=nd0,script=no,ifname=tap0 -device e1000,netdev=nd0,mac=56:b4:44:e9:62:d0 -m 1024"

    print("Invoking QEMU with command: ", cmd)

    child = pexpect.spawn(cmd)

    # give guest time to boot
    child.expect("root@jammy:~# ", timeout=BOOT_TIMEOUT)

    # bring up ip address
    child.sendline("ip addr add 172.31.0.1/24 broadcast 172.31.0.255 dev ens3")
    child.expect("root@jammy:~# ")
    child.sendline("ip link set ens3 up")
    child.expect("root@jammy:~# ")

    # ensure ip address properly configured
    child.sendline("ifconfig")
    child.expect("root@jammy:~# ")
    output = child.before
    print("{}".format(output))

#
# Main routine of run.py
#
if __name__ == '__main__':
    "Execution pipeline for building and launching nrk"
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

    qemu_run()
