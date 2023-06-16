use super::PAGE_SIZE;
use crate::fxmark::Bench;
use libc::*;
use std::cell::RefCell;
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};
use fxmark_grpc::*;

#[derive(Clone)]
pub struct DRBL {
    _path: &'static str,
    page: Vec<u8>,
    fds: RefCell<Vec<c_int>>,
}

unsafe impl Sync for DRBL {}

impl Default for DRBL {
    fn default() -> DRBL {
        let page = vec![0xb; PAGE_SIZE];
        let fd = vec![-1; 512];
        DRBL {
            // It doesn't work if trailing \0 isn't there in the filename.
            _path: "",
            page,
            fds: RefCell::new(fd),
        }
    }
}

impl Bench for DRBL {
    fn init(&self, cores: Vec<u64>, _open_files: usize) {
        // unsafe {
            for core in cores {
                let filename = format!("file{}.txt\0", core);

                let _a = grpc_remove(&filename).unwrap();
                let fd = grpc_open(&filename, O_CREAT | O_RDWR, S_IRWXU).unwrap();
                if fd == -1 {
                    panic!("Unable to create a file");
                }
                let len = self.page.len();
                if grpc_write(fd, &self.page, len).unwrap() != len as i32 {
                    panic!("Write failed");
                }
                self.fds.borrow_mut()[core as usize] = fd;
            }
        // }
    }

    fn run(&self, b: Arc<Barrier>, duration: u64, core: u64, _write_ratio: usize) -> Vec<usize> {
        let mut secs = duration as usize;
        let mut iops = Vec::with_capacity(secs);

        // unsafe {
            let fd = self.fds.borrow()[core as usize];
            if fd == -1 {
                panic!("Unable to open a file");
            }
            let mut page: Vec<u8> = vec![0; PAGE_SIZE];

            b.wait();
            while secs > 0 {
                let mut ops = 0;
                let start = Instant::now();
                let end_experiment = start + Duration::from_secs(1);
                while Instant::now() < end_experiment {
                    // pread for 128 times to reduce rdtsc overhead.
                    for _i in 0..128 {
                        if grpc_read(fd, &mut page, PAGE_SIZE, 0).unwrap()
                            != PAGE_SIZE as i32 
                        {
                            panic!("DRBL: pread() failed");
                        }
                        ops += 1;
                    }
                }
                iops.push(ops);
                secs -= 1;
            }

            let _ignore = grpc_close(fd);
            let filename = format!("file{}.txt\0", core);
            if grpc_remove(&filename).unwrap() != 0 {
                panic!(
                    "DRBL: Unable to remove file, errno: {}",
                    nix::errno::errno()
                );
            }
        // }

        iops.clone()
    }
}
