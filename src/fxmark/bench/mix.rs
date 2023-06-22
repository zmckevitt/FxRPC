use super::PAGE_SIZE;
use crate::fxmark::Bench;
use libc::*;
use std::cell::RefCell;
use std::io::Error;
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};
use x86::random::rdrand16;

use fxmark_grpc::*;

#[derive(Clone)]
pub struct MIX {
    path: &'static str,
    page: Vec<u8>,
    open_files: RefCell<usize>,
    file_size: i64,
    fds: RefCell<Vec<c_int>>,
}

impl Default for MIX {
    fn default() -> MIX {
        let page = vec![0xb; PAGE_SIZE];
        let fd = vec![-1; 512];
        MIX {
            // It doesn't work if trailing \0 isn't there in the filename.
            path: "/mnt",
            page,
            open_files: RefCell::new(0),
            file_size: 256 * 1024 * 1024,
            fds: RefCell::new(fd),
        }
    }
}

impl Bench for MIX {
    fn init(&self, cores: Vec<u64>, open_files: usize) {
        *self.open_files.borrow_mut() = open_files;
        let mut temp: Vec<c_int> = Vec::with_capacity(open_files);
        // unsafe {
            for file in 0..open_files {
                let filename = format!("file{}.txt\0", file);
                let _a = grpc_remove(&filename).unwrap();
                let fd = grpc_open(&filename, O_CREAT | O_RDWR, S_IRWXU).unwrap();
                if fd == -1 {
                    panic!(
                        "Unable to create a file due to {:?}",
                        Error::last_os_error()
                    );
                }
                let mut size = 0;
                while size <= self.file_size {
                    if grpc_write(fd, &self.page, PAGE_SIZE).unwrap()
                        != PAGE_SIZE as i32
                    {
                        panic!("MIX: Write failed due to {:?}", Error::last_os_error());
                    }
                    size += PAGE_SIZE as i64;
                }

                let stat_size = {
                    // let mut info = std::mem::MaybeUninit::uninit();
                    grpc_fstat_size(fd).unwrap()
                    // info.assume_init()
                };
                assert_eq!(self.file_size + PAGE_SIZE as i64, stat_size);

                let _ignore = grpc_fsync(fd);
                temp.push(fd);
            }
        // }

        // Distribute the files among different cores.
        let mut iter = 0;
        for core in cores.iter() {
            let id = *core as usize;
            self.fds.borrow_mut()[id] = temp[iter % open_files];
            iter += 1;
        }
    }

    fn run(&self, b: Arc<Barrier>, duration: u64, core: u64, write_ratio: usize) -> Vec<usize> {
        let mut secs = duration as usize;
        let mut iops = Vec::with_capacity(secs);

        // unsafe {
            let fd = self.fds.borrow()[core as usize];
            if fd == -1 {
                panic!("Unable to open a file due to {:?}", Error::last_os_error());
            }
            let total_pages = self.file_size / PAGE_SIZE as i64;
            // let page: &mut [i8; PAGE_SIZE as usize] = &mut [0; PAGE_SIZE as usize];
            let mut page: Vec<u8> = vec![0; PAGE_SIZE];

            let mut random_num: u16 = 0;

            b.wait();
            while secs > 0 {
                let mut ops = 0;
                let start = Instant::now();
                let end_experiment = start + Duration::from_secs(1);
                while Instant::now() < end_experiment {
                    for _i in 0..128 {
                        unsafe {
                            rdrand16(&mut random_num);
                        }
                        let rand = random_num as i64 % total_pages;
                        let offset = rand * PAGE_SIZE as i64;
                        if random_num as usize % 100 < write_ratio {
                            if grpc_pwrite(fd, &page, PAGE_SIZE, offset).unwrap()
                                != PAGE_SIZE as i32
                            {
                                panic!("MIX: pwrite() failed {}", nix::errno::errno());
                            };
                        } else {
                            if grpc_pread(fd, &mut page, PAGE_SIZE, offset).unwrap()
                                != PAGE_SIZE as i32
                            {
                                panic!("MIX: pread() failed {}", nix::errno::errno());
                            };
                        }
                        ops += 1;
                    }
                }
                iops.push(ops);
                secs -= 1;
            }

            b.wait();

            let _ignore = grpc_fsync(fd);
            let _ignore = grpc_close(fd);

            for i in 0..*self.open_files.borrow() {
                let filename = format!("file{}.txt\0", i);
                let _a = grpc_remove(&filename);
            }
        // }

        iops.clone()
    }
}

unsafe impl Sync for MIX {}
