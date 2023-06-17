use super::PAGE_SIZE;
use crate::fxmark::Bench;
use libc::*;
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};
use fxmark_grpc::*;

#[derive(Clone)]
pub struct DRBH {
    path: &'static str,
    page: Vec<u8>,
}

impl Default for DRBH {
    fn default() -> DRBH {
        let page = vec![0xb; PAGE_SIZE];
        DRBH {
            // It doesn't work if trailing \0 isn't there in the filename.
            path: "file.txt\0",
            page,
        }
    }
}

impl Bench for DRBH {
    fn init(&self, _cores: Vec<u64>, _open_files: usize) {
        // unsafe {
            let _a = grpc_remove(self.path).unwrap();
            let fd = grpc_open(self.path, O_CREAT | O_RDWR, S_IRWXU).unwrap();
            if fd == -1 {
                panic!("Unable to create a file");
            }
            let len = self.page.len();
            if grpc_write(fd, &self.page, len).unwrap() != len as i32 {
                panic!("Write failed");
            };

            let _ignore = grpc_fsync(fd);
            let _ignore = grpc_close(fd);
        // }
    }

    fn run(&self, b: Arc<Barrier>, duration: u64, _core: u64, _write_ratio: usize) -> Vec<usize> {
        let mut secs = duration as usize;
        let mut iops = Vec::with_capacity(secs);

        // unsafe {
            let fd = grpc_open(self.path, O_CREAT | O_RDWR, S_IRWXU).unwrap();
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
                    for _i in 0..128 {
                        // Might need to modify the proto to have 4 params 
                        // if pread(fd, page.as_ptr(), PAGE_SIZE, 0)
                        if grpc_pread(fd, &mut page, PAGE_SIZE, 0).unwrap()
                            != PAGE_SIZE as i32
                        {
                            panic!("DRBH: pread() failed");
                        };
                        ops += 1;
                    }
                }
                iops.push(ops);
                secs -= 1;
            }

            let _ignore = grpc_fsync(fd);
            let _ignore = grpc_close(fd);
        // }

        iops.clone()
    }
}
