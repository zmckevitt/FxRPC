pub const PATH_LEN: usize = 128;
pub const PAGE_LEN: usize = 8192;
// const WR_METADATA_SZ: usize =
//     std::mem::size_of::<i32>() - std::mem::size_of::<usize>() - std::mem::size_of::<i64>();

////////////////////////////// FS RPC Hdrs  //////////////////////////////

#[derive(Debug, Eq, PartialEq, PartialOrd, Clone, Copy)]
#[repr(u8)]
pub(crate) enum DRPC {
    /// Create a file
    Create = 0,
    /// Open a file
    Open = 1,
    /// Read from a file
    Read = 2,
    /// Read from a file from the given offset
    PRead = 3,
    /// Write to a file
    Write = 4,
    /// Write to a file
    PWrite = 5,
    /// Close an opened file.
    Close = 6,
    /// Get the information related to the file.
    GetInfo = 7,
    /// Remove the file
    Remove = 8,
    /// Write to a file without going into NR.
    WriteDirect = 9,
    /// Rename a file.
    FileRename = 10,
    /// Create a directory.
    MkDir = 11,
    /// Remove a directory.
    RmDir = 12,
}

pub fn pack_str<const output_size: usize>(input: &str) -> [u8; output_size] {
    let bytes = input.as_bytes();
    let mut output = [0; output_size];
    output[..input.len()].copy_from_slice(&bytes);
    output
}

pub struct OpenReq {
    pub path: [u8; PATH_LEN],
    pub flags: i32,
    pub mode: u32,
}

pub struct ReadReq {
    pub fd: i32,
    pub size: usize,
    pub offset: i64,
}

pub struct WriteReq {
    pub fd: i32,
    pub page: [u8; PAGE_LEN],
    pub size: usize,
    pub offset: i64,
}

pub struct CloseReq {
    pub fd: i32,
}

pub struct RemoveReq {
    pub path: [u8; PATH_LEN],
}
pub struct MkdirReq {
    pub path: [u8; PATH_LEN],
    pub mode: u32,
}
