use libc::*;
use log::debug;
use rpc::rpc::*;
use rpc::server::{RPCHandler, Server};
use rpc::transport::stdtcp::*;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use abomonation::{decode, encode};

use crate::fxrpc::drpc::fileops::*;
use crate::fxrpc::FS_PATH;

////////////////////////////////// SERVER //////////////////////////////////

fn construct_ret(
    hdr: &mut RPCHeader,
    mut payload: &mut [u8],
    result: i32,
    size: usize,
    page: Vec<u8>,
) {
    let response = Response {
        result: result,
        size: size,
        page: page,
    };

    let mut bytes = Vec::new();
    unsafe { encode(&response, &mut bytes) }.expect("Failed to encode response");

    payload[0..bytes.len()].copy_from_slice(&bytes);

    hdr.msg_id = 0;
    hdr.msg_type = 0;
    hdr.msg_len = (bytes.len() * std::mem::size_of::<u8>()) as MsgLen;
}

fn handle_open(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    let msg_id: MsgId = hdr.msg_id;
    let msg_type: RPCType = hdr.msg_type;
    let msg_len: usize = hdr.msg_len as usize;

    let (path, flags, modes) = match unsafe { decode::<OpenReq>(payload) } {
        Some((req, _)) => (req.path.clone(), req.flags, req.mode),
        None => panic!("Cannot decode open request!"),
    };

    let path = std::str::from_utf8(&path).unwrap();

    debug!(
        "Open request - path: {:?}, flags: {:?}, modes: {:?}",
        path, flags, modes
    );

    let file_path = format!("{}{}{}", FS_PATH, path, char::from(0));
    let fd;
    unsafe {
        fd = open(file_path.as_ptr() as *const i8, flags, modes);
    }

    construct_ret(hdr, payload, fd, 0, vec![]);
    Ok(())
}

fn handle_read(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    let msg_id: MsgId = hdr.msg_id;
    let msg_type: RPCType = hdr.msg_type;
    let msg_len: MsgLen = hdr.msg_len;

    let (fd, size, offset) = match unsafe { decode::<ReadReq>(payload) } {
        Some((req, _)) => (req.fd, req.size, req.offset),
        None => panic!("Cannot decode read request!"),
    };

    debug!(
        "Read request - fd: {:?}, size: {:?}, offset: {:?}",
        fd, size, offset
    );

    let res;
    let page: Vec<u8> = vec![0; size];
    unsafe {
        res = read(fd, page.as_ptr() as *mut c_void, size);
    }

    construct_ret(hdr, payload, res as i32, size, page.to_vec());
    Ok(())
}

fn handle_pread(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    let msg_id: MsgId = hdr.msg_id;
    let msg_type: RPCType = hdr.msg_type;
    let msg_len: MsgLen = hdr.msg_len;

    let (fd, size, offset) = match unsafe { decode::<ReadReq>(payload) } {
        Some((req, _)) => (req.fd, req.size, req.offset),
        None => panic!("Cannot decode pread request!"),
    };

    debug!(
        "PRead request - fd: {:?}, size: {:?}, offset: {:?}",
        fd, size, offset
    );

    let res;
    let page: Vec<u8> = vec![0; size];
    unsafe {
        res = pread(fd, page.as_ptr() as *mut c_void, size, offset);
    }

    construct_ret(hdr, payload, res as i32, size, page.to_vec());
    Ok(())
}

fn handle_write(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    let msg_id: MsgId = hdr.msg_id;
    let msg_type: RPCType = hdr.msg_type;
    let msg_len: MsgLen = hdr.msg_len;

    let (fd, page, size, offset) = match unsafe { decode::<WriteReq>(payload) } {
        Some((req, _)) => (req.fd, req.page.clone(), req.size, req.offset),
        None => panic!("Cannot decode write request!"),
    };

    debug!(
        "Write request - fd: {:?}, page: {:?}, size: {:?}, offset: {:?}",
        fd, page, size, offset
    );

    let res;
    unsafe {
        res = write(fd, page.as_ptr() as *const c_void, size);
    }

    construct_ret(hdr, payload, res as i32, 0, vec![]);
    Ok(())
}

fn handle_pwrite(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    let msg_id: MsgId = hdr.msg_id;
    let msg_type: RPCType = hdr.msg_type;
    let msg_len: MsgLen = hdr.msg_len;

    let (fd, page, size, offset) = match unsafe { decode::<WriteReq>(payload) } {
        Some((req, _)) => (req.fd, req.page.clone(), req.size, req.offset),
        None => panic!("Cannot decode pwrite request!"),
    };

    debug!(
        "PWrite request - fd: {:?}, page: {:?}, size: {:?}, offset: {:?}",
        fd, page, size, offset
    );

    let res;
    unsafe {
        res = pwrite(fd, page.as_ptr() as *const c_void, size, offset);
    }

    construct_ret(hdr, payload, res as i32, 0, vec![]);
    Ok(())
}

fn handle_close(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    let msg_id: MsgId = hdr.msg_id;
    let msg_type: RPCType = hdr.msg_type;
    let msg_len: MsgLen = hdr.msg_len;

    let fd = match unsafe { decode::<CloseReq>(payload) } {
        Some((req, _)) => req.fd,
        None => panic!("Cannot decode close request!"),
    };

    debug!("Close request - fd: {:?}", fd);

    let res;
    unsafe {
        res = close(fd);
    }

    construct_ret(hdr, payload, fd, 0, vec![]);
    Ok(())
}

fn handle_remove(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    let msg_id: MsgId = hdr.msg_id;
    let msg_type: RPCType = hdr.msg_type;
    let msg_len: MsgLen = hdr.msg_len;

    let path = match unsafe { decode::<RemoveReq>(payload) } {
        Some((req, _)) => req.path.clone(),
        None => panic!("Cannot decode remove request!"),
    };

    let path = std::str::from_utf8(&path).unwrap();

    debug!("Remove request - path: {:?}", path);

    let file_path = format!("{}{}{}", FS_PATH, path, char::from(0));
    let fd;
    unsafe {
        fd = remove(file_path.as_ptr() as *const i8);
    }

    construct_ret(hdr, payload, fd, 0, vec![]);
    Ok(())
}

fn handle_fsync(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    let msg_id: MsgId = hdr.msg_id;
    let msg_type: RPCType = hdr.msg_type;
    let msg_len: MsgLen = hdr.msg_len;
    debug!(
        "Request with ID {:?}, type {:?}, len {:?}",
        msg_id, msg_type, msg_len
    );
    construct_ret(hdr, payload, 0, 0, vec![]);
    Ok(())
}

fn handle_mkdir(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    let msg_id: MsgId = hdr.msg_id;
    let msg_type: RPCType = hdr.msg_type;
    let msg_len: MsgLen = hdr.msg_len;

    let (path, modes) = match unsafe { decode::<MkdirReq>(payload) } {
        Some((req, _)) => (req.path.clone(), req.mode),
        None => panic!("Cannot decode mkdir request!"),
    };

    let path = std::str::from_utf8(&path).unwrap();

    debug!("Mkdir request - path: {:?}, modes: {:?}", path, modes);

    let dir_path = format!("{}{}{}", FS_PATH, path, char::from(0));
    let res;
    unsafe {
        res = mkdir(dir_path.as_ptr() as *const i8, modes.try_into().unwrap());
    }

    construct_ret(hdr, payload, res, 0, vec![]);
    Ok(())
}

fn handle_rmdir(hdr: &mut RPCHeader, payload: &mut [u8]) -> Result<(), RPCError> {
    let msg_id: MsgId = hdr.msg_id;
    let msg_type: RPCType = hdr.msg_type;
    let msg_len: MsgLen = hdr.msg_len;
    debug!(
        "Request with ID {:?}, type {:?}, len {:?}",
        msg_id, msg_type, msg_len
    );
    construct_ret(hdr, payload, 0, 0, vec![]);
    Ok(())
}

const OPEN_HANDLER: RPCHandler = handle_open;
const READ_HANDLER: RPCHandler = handle_read;
const PREAD_HANDLER: RPCHandler = handle_pread;
const WRITE_HANDLER: RPCHandler = handle_write;
const PWRITE_HANDLER: RPCHandler = handle_pwrite;
const CLOSE_HANDLER: RPCHandler = handle_close;
const REMOVE_HANDLER: RPCHandler = handle_remove;
const FSYNC_HANDLER: RPCHandler = handle_fsync;
const MKDIR_HANDLER: RPCHandler = handle_mkdir;
const RMDIR_HANDLER: RPCHandler = handle_rmdir;

fn register_rpcs(server: &mut Server) {
    server
        .register(DRPC::Open as RPCType, &OPEN_HANDLER)
        .unwrap();
    server
        .register(DRPC::Read as RPCType, &READ_HANDLER)
        .unwrap();
    server
        .register(DRPC::PRead as RPCType, &PREAD_HANDLER)
        .unwrap();
    server
        .register(DRPC::Write as RPCType, &WRITE_HANDLER)
        .unwrap();
    server
        .register(DRPC::PWrite as RPCType, &PWRITE_HANDLER)
        .unwrap();
    server
        .register(DRPC::Close as RPCType, &CLOSE_HANDLER)
        .unwrap();
    server
        .register(DRPC::Remove as RPCType, &REMOVE_HANDLER)
        .unwrap();
    server
        .register(DRPC::MkDir as RPCType, &MKDIR_HANDLER)
        .unwrap();
    server
        .register(DRPC::RmDir as RPCType, &RMDIR_HANDLER)
        .unwrap();
}

fn server_from_stream(stream: TcpStream) {
    let transport = StdTCP {
        stream: Arc::new(Mutex::new(stream)),
    };
    let mut server = Server::new(Box::new(transport));
    register_rpcs(&mut server);
    // I dont think we need this, client registration in DiNOS
    // is usually for allocation of kernel resources (shmem and dcm)
    // server.add_client(&CLIENT_REGISTRAR);
    server.run_server();
}

pub fn start_drpc_server_tcp(bind_addr: &str, port: u16) {
    // TODO: bind to addr/port specified in parameters
    let listener = TcpListener::bind("127.0.0.1:8080").expect("Failed to create TCP transport");

    for stream in listener.incoming() {
        std::thread::spawn(move || server_from_stream(stream.unwrap()));
    }
}

pub fn start_drpc_server_uds() {}
